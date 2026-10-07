#!/usr/bin/env bash
#
# check-js-listeners.sh
#
# Keeps a JS event listener from being added in a shape that nothing ties to the scope that owns
# it. cargo, clippy and `cargo test` cannot see it, and it looks like a harmless one-liner inside
# a `document::eval` string.
#
# The class (dev-docs/backlog.md rows 149 and 159): a listener added from `document::eval` (or from
# a shipped or inline script) belongs to the PAGE, not to the Dioxus scope that evaluated the script.
# Dropping the component drops the Rust task that listens to the eval; the `addEventListener`
# registration stays on `window`/`document` for the rest of the visit, and a script that is re-run
# (an effect, a second visit to the route) adds another one on top. Found in the form fixture (6
# adds, 0 removes: two of them on `document`, one more pair per visit), the sidebar, `pointer.rs`,
# the focus trap and `scroll_lock.rs`; for a non-passive `wheel`/`touchmove` listener on `window`
# it also made every wheel tick on the page wait for the main thread after the first modal had
# opened once. Fixed by construction, not per instance: the one sanctioned way is
# `primitives/src/js_listener.rs` (`JsListeners` / `use_js_listeners`): the script registers through
# `listen(target, type, handler, options)`, the removal is DERIVED (undone when the value drops, i.e.
# when the component unmounts, the effect re-runs or the lock releases), and wheel/touch listeners
# are passive unless the call says otherwise. This script keeps a raw registration from coming
# back. In `primitives/src` and `preview/src` (`*.rs`, `*.js`, `*.ts`) and `preview/index.html` it
# fails on every `addEventListener(` that
#
#   1. is not in primitives/src/js_listener.rs (the one implementation), AND
#   2. has no `listener-ok: <reason>` marker on the same line or on one of the two lines above it
#      (a Rust/JS/HTML comment; inside an eval string write a JS comment on the line above the
#      call, in a `\`-joined one-line string use the allowlist below), AND
#   3. is not an already-correct HAND-PAIRED registration in a file of PAIRED_BY_HAND below: the
#      same file removes the same (event, handler) with `removeEventListener`, from a teardown
#      the author wired up by hand.
#
# PAIRED_BY_HAND is a ratchet, not a licence: it lists the files that had a correct hand-written
# add/remove pair before the helper existed and have not been moved onto it yet (each migration
# is mechanical: `x.addEventListener(t, h, o)` -> `listen(x, t, h, o)`, delete the
# `await dioxus.recv()` and the matching removes, wrap in `use_js_listeners`). The list only shrinks;
# a file that no longer needs its entry is reported (a note, not a failure) so it gets deleted. A
# NEW file, or a registration with no matching remove in a listed file, fails.
#
# Also checked: `scripts/check-blocking-scroll-listeners.sh` sees `listen(.., 'wheel' | 'touchstart'
# | 'touchmove', .., { passive: false })` too, so the helper's passive default cannot be turned off
# without a `blocking-ok:` reason.
#
# NOT covered, said plainly: a listener added through another API (`web_sys`, an `on*` property, a
# vendored script), `MutationObserver` / `ResizeObserver` / `IntersectionObserver` (the same class,
# a different spelling; a grep cannot tell a bounded one from a perpetual one), and the generated
# bundle `primitives/src/js/focus-trap.js` (its source `primitives/src/ts/focus-trap.ts` is scanned).
# `playwright/oracle/tier2-html/listener-inventory.spec.ts` is the runtime backstop: it asks Chromium
# (`DOMDebugger.getEventListeners`) which listeners `window` and `document` hold after every overlay
# and route has been opened and closed again.
#
# Usage: scripts/check-js-listeners.sh [file[=repo/relative/name] ...]
#   No arguments scans the tree; explicit files are scanned as given (used to prove the gate red
#   against old code: `git show HEAD:primitives/src/pointer.rs > /tmp/old.rs;
#   scripts/check-js-listeners.sh /tmp/old.rs=primitives/src/pointer.rs` scans the old text as if it
#   were that file, so the allowlists apply to it exactly as they would in the tree).
#   Exit 0: clean. Exit 1: offending file:line on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Explicit files are resolved against the caller's directory, before moving to the repo root.
args=()
for a in "$@"; do
  f="${a%%=*}"; as=""
  [ "$f" != "$a" ] && as="=${a#*=}"
  args+=("$(cd "$(dirname "$f")" && pwd)/$(basename "$f")$as")
done
cd "$repo_root"

checker="$(mktemp -t check-js-listeners.XXXXXX.py)"
trap 'rm -f "$checker"' EXIT

cat >"$checker" <<'PYEOF'
import re
import sys
from pathlib import Path

HELPER = "primitives/src/js_listener.rs"

# Generated, not source: primitives/build.rs bundles primitives/src/ts/focus-trap.ts into it.
GENERATED = ("primitives/src/js/",)

# file (repo-relative) -> why a hand-written add/remove pair is still acceptable there. Only
# shrinks: migrate the file onto `use_js_listeners`, then delete its line.
PAIRED_BY_HAND = {
    "primitives/src/carousel.rs": "scroller/container/window listeners with their own teardown script, owned by the carousel bridges",
    "primitives/src/collapsible.rs": "`beforematch` on the collapsible's own element, removed after the recv-sentinel",
    "primitives/src/context_menu.rs": "window wheel/touchmove while a context menu is open, removed in the effect's cleanup",
    "primitives/src/drag_and_drop_list.rs": "document drag listeners, removed in the drag's own teardown",
    "primitives/src/drawer.rs": "window pointer listeners of one drag, removed when the drag ends",
    "primitives/src/input_otp.rs": "input events, removed after the recv-sentinel",
    "primitives/src/menu_sub.rs": "document `focusin`, removed after the recv-sentinel",
    "primitives/src/message_scroller.js": "viewport/document listeners installed by attach() and removed by the function it returns",
    "primitives/src/navigation_menu.rs": "document `pointerdown`, removed after the recv-sentinel",
    "primitives/src/top_layer.rs": "`toggle` and window scroll/resize trackers, removed after the recv-sentinel",
    "primitives/src/virtual_list.rs": "container scroll + window resize bridge, removed after the recv-sentinel",
}

# Registrations that are NOT hand-paired but still legitimate, in files whose owner has not
# added the marker yet. Same ratchet: delete the line when the marker lands.
PENDING_MARKER: dict[str, str] = {}

MARKER = "listener-ok:"
ADD_RE = re.compile(r"addEventListener\s*\(")
REMOVE_RE = re.compile(r"removeEventListener\s*\(")
IDENT_RE = re.compile(r"^[A-Za-z_$][\w$.]*$")


def norm(arg):
    return re.sub(r"\s+", "", arg.replace('"', "'"))


def split_args(text, open_pos, limit=2):
    """First `limit` top-level arguments of the call whose '(' is at open_pos, robust to the call
    being cut off by the end of an inline string (a Rust test that quotes the call)."""
    args, cur, depth, quote = [], [], 0, None
    i = open_pos + 1
    while i < len(text):
        c = text[i]
        if c == "\n" and depth == 0:
            # an unbalanced call at the end of a line: stop (a call quoted inside a Rust test string);
            # what is left is only usable as a leading identifier
            tail = re.match(r"\s*[A-Za-z_$][\w$.]*", "".join(cur))
            args.append(tail.group(0) if tail else "".join(cur))
            return args
        if quote:
            cur.append(c)
            if c == "\\":
                i += 1
                if i < len(text):
                    cur.append(text[i])
            elif c == quote:
                quote = None
        elif c in "'\"`":
            quote = c
            cur.append(c)
        elif c in "([{":
            depth += 1
            cur.append(c)
        elif c in ")]}":
            if depth == 0:
                args.append("".join(cur))
                return args
            depth -= 1
            cur.append(c)
        elif c == "," and depth == 0:
            args.append("".join(cur))
            cur = []
            if len(args) == limit:
                return args
        else:
            cur.append(c)
        i += 1
    args.append("".join(cur))
    return args


def is_comment(line):
    s = line.lstrip()
    return s.startswith("//") or s.startswith("*") or s.startswith("/*") or s.startswith("<!--")


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def scan(rel, text):
    """Returns (violations, hand_paired_sites) for one file."""
    lines = text.split("\n")
    removes = set()
    for m in REMOVE_RE.finditer(text):
        a = split_args(text, m.end() - 1)
        if len(a) >= 2:
            removes.add((norm(a[0]), norm(a[1])))

    bad, hand_paired = [], 0
    for m in ADD_RE.finditer(text):
        ln = line_of(text, m.start())
        if is_comment(lines[ln - 1]):
            continue
        window = lines[max(0, ln - 3):ln]
        if any(MARKER in l for l in window):
            continue
        a = split_args(text, m.end() - 1)
        paired = len(a) >= 2 and IDENT_RE.match(norm(a[1]) or "") is not None and (norm(a[0]), norm(a[1])) in removes
        if rel in PAIRED_BY_HAND and paired:
            hand_paired += 1
            continue
        if rel in PENDING_MARKER:
            continue
        if paired:
            why = ("hand-paired `addEventListener`/`removeEventListener` -- move it onto `use_js_listeners` "
                   "(the file is not in PAIRED_BY_HAND, which only shrinks)")
        else:
            why = ("raw `addEventListener` with no matching `removeEventListener` -- the listener lives as long as "
                   "the page, whatever scope ran the script")
        bad.append((ln, why))
    return bad, hand_paired


def main():
    root = Path(".")
    args = sys.argv[1:]
    names = {}
    if args:
        paths = []
        for a in args:
            src, _, as_name = a.partition("=")
            paths.append(Path(src))
            if as_name:
                names[src] = as_name
    else:
        paths = []
        for base in ("primitives/src", "preview/src"):
            for ext in ("*.rs", "*.js", "*.ts"):
                paths.extend(sorted((root / base).rglob(ext)))
        idx = root / "preview/index.html"
        if idx.is_file():
            paths.append(idx)

    bad, notes = [], []
    seen_hand_paired = {}
    for p in paths:
        rel = names.get(str(p), str(p)).replace("\\", "/")
        if rel == HELPER or rel.startswith(GENERATED):
            continue
        try:
            text = p.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        file_bad, hand_paired = scan(rel, text)
        seen_hand_paired[rel] = hand_paired
        for ln, why in file_bad:
            bad.append((rel, ln, why))

    if not args:
        for rel in PAIRED_BY_HAND:
            if seen_hand_paired.get(rel, 0) == 0:
                notes.append(f"note: {rel} no longer needs its PAIRED_BY_HAND entry -- delete it from scripts/check-js-listeners.sh")
        for rel in PENDING_MARKER:
            p = Path(rel)
            if p.is_file() and any(
                MARKER in l for l in p.read_text(encoding="utf-8").split("\n")
            ):
                notes.append(f"note: {rel} now carries a listener-ok marker -- delete its PENDING_MARKER entry from scripts/check-js-listeners.sh")

    for n in notes:
        print(f"check-js-listeners: {n}", file=sys.stderr)

    if bad:
        print("check-js-listeners: listener(s) not tied to a scope found:", file=sys.stderr)
        for rel, ln, why in sorted(bad):
            print(f"{rel}:{ln}: {why}", file=sys.stderr)
        print(
            "  Fix: register through primitives' `JsListeners` / `use_js_listeners` "
            "(`listen(target, type, handler, options)` inside the script -- its removal is derived and wheel/touch "
            "are passive by default), or, for a listener that genuinely lives as long as the page, add "
            "`listener-ok: <reason>` on the same line or the line above (dev-docs/backlog.md rows 149/159)",
            file=sys.stderr,
        )
        sys.exit(1)

    print("check-js-listeners: OK -- every addEventListener is in the helper, marked listener-ok, or an "
          "already-paired registration in a PAIRED_BY_HAND file.")


if __name__ == "__main__":
    main()
PYEOF

python3 "$checker" "${args[@]+"${args[@]}"}"
