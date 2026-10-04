#!/usr/bin/env bash
#
# check-blocking-scroll-listeners.sh
#
# Keeps a blocking (non-passive) wheel / touch listener from being added to
# the page by accident. cargo, clippy and `cargo test` cannot see it, and it
# looks like a harmless one-line event handler.
#
# Why it matters: Dioxus registers every bubbling rsx event handler ONCE, on
# the app root (`#main`), with no options (dioxus-interpreter-js
# `createListener`: `this.root.addEventListener(event_name, this.handler)`),
# so it can never be passive. One `onwheel:` / `ontouchstart:` /
# `ontouchmove:` anywhere in the app therefore makes the WHOLE page's wheel /
# touch input non-passive: before the compositor may scroll, every wheel tick
# is dispatched to the main thread and waited for (Chromium
# `DidHandleInputEventSentToMain` 40/40 ticks vs 0; wheel latency max 388 ms vs
# 143 ms with a busy main thread, dev-docs/research/scroll-jank-2026-10-04.md,
# Cause 2b). On a touch device a blocking `touchstart` additionally delays every
# scroll start.
#
# The instances this was written against, all removed: `Input`'s `onwheel`
# shim (set on every `<input>` even when the prop was `None`), and
# `ontouchstart: prevent_default` on the slider thumb, the resizable handle
# and the color-area thumb (their `onmousedown` twin already stops the focus
# steal; drags are pointer-driven with `touch-action: none`). Removing them is only
# safe while the touched element's EFFECTIVE `touch-action` is `none` (the
# intersection down the ancestor chain): the slider thumb and the color area get it
# from their non-focusable containers, but the resizable handle is itself focusable,
# so the docs app's `touch-action: manipulation !important` catch-all beat its own
# `none` and Chromium cancelled the drag (`pointercancel` after ~20px) until
# `preview/assets/main.css` exempted `.dx-resizable-handle[role="separator"]`.
# `playwright/oracle/tier2-html/scroll-main-thread.spec.ts` drives all three with real touch.
#
# What it flags, in `primitives/src` and `preview/src` (`*.rs`, `*.js`) and
# `preview/index.html`:
#
#   1. An rsx event ATTRIBUTE `onwheel: <closure|ident>`, `onmousewheel:`,
#      `ontouchstart:`, `ontouchmove:` (a prop DECLARATION such as
#      `onwheel: Option<EventHandler<WheelEvent>>` is a type, not a value, and
#      is fine). To forward an optional handler, build the listener only when
#      the caller passed one (`preview/src/components/input/component.rs`).
#   2. A raw `addEventListener('wheel' | 'mousewheel' | 'touchstart' |
#      'touchmove' | 'DOMMouseScroll', ...)` whose options do not say
#      `passive: true` (a bare `true`/`{capture: true}` is non-passive for
#      wheel and touch on elements, and the "root-level passive" default does
#      not apply to a capturing listener on `window`).
#
# Escape hatches, both with a reason:
#   - a Rust line:   `ontouchstart: move |e| { .. }, // blocking-ok: <reason>`
#     (same line or the line above);
#   - a whole file in ALLOWLIST below (needed for JS inside a Rust string,
#     where a `//` marker would comment out the rest of a `\`-joined line).
#
# Not covered, said plainly: handlers installed through some other API
# (`web_sys`, a vendored script), and a listener that is only non-passive for
# a moment (the allowlist is for those). `playwright/oracle/tier2-html/
# scroll-main-thread.spec.ts` is the runtime backstop: it asks Chromium
# (`DOMDebugger.getEventListeners`) for `#main`'s listeners on `/`.
#
# Usage: scripts/check-blocking-scroll-listeners.sh [file ...]
#   No arguments scans the tree; explicit files are scanned as given (used to
#   prove the gate red against old code).
# Exit 0: clean. Exit 1: offending file:line on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

checker="$(mktemp -t check-blocking-scroll-listeners.XXXXXX.py)"
trap 'rm -f "$checker"' EXIT

cat >"$checker" <<'PYEOF'
import re
import sys
from pathlib import Path

# file (repo-relative) -> why its non-passive wheel/touch listeners are legitimate.
ALLOWLIST = {
    "primitives/src/context_menu.rs": (
        "installs non-passive window wheel/touchmove listeners in an effect only while a "
        "context menu is OPEN and removes them in that effect's cleanup -- blocking the page "
        "scroll behind an open menu is its job"
    ),
    "primitives/src/scroll_lock.rs": (
        "KNOWN GAP, not a clean pass: the modal scroll lock needs non-passive window wheel/"
        "touchmove while a lock is held, but `ensure_scroll_block_listeners_installed` installs "
        "them PERMANENTLY at the first lock (a `__dxScrollLocked` flag then gates them), so after "
        "the first dialog/popover/dropdown opens, every wheel/touch on the page waits for the main "
        "thread for the rest of the session. The fix is install-on-lock / remove-on-unlock; "
        "tracked in dev-docs/research/scroll-jank-2026-10-04.md (follow-ups)"
    ),
}

ATTR_RE = re.compile(
    r"(?<![A-Za-z0-9_.])(onwheel|onmousewheel|ontouchstart|ontouchmove)\s*:\s*(?=move\b|\||[a-z_])"
)
LISTENER_RE = re.compile(
    r"addEventListener\s*\(\s*(['\"`])(wheel|mousewheel|touchstart|touchmove|DOMMouseScroll)\1"
)
MARKER = "blocking-ok:"


def matching_paren(text, open_pos):
    depth = 0
    for i in range(open_pos, len(text)):
        c = text[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
    return len(text)


def line_of(text, pos):
    return text.count("\n", 0, pos) + 1


def strip_line_comment(line):
    # Good enough for this tree: a `//` inside a string literal on the same line as a real
    # handler attribute does not occur (checked), and prose comments must not trip the gate.
    return line.split("//", 1)[0]


def scan(path_str, text, allowlisted):
    out = []
    lines = text.split("\n")

    for idx, line in enumerate(lines):
        code = strip_line_comment(line)
        m = ATTR_RE.search(code)
        if not m:
            continue
        prev = lines[idx - 1] if idx else ""
        if MARKER in line or MARKER in prev:
            continue
        out.append((idx + 1, f"unconditional `{m.group(1)}:` handler -- Dioxus attaches it to `#main` as a "
                              f"non-passive listener, so the whole page's wheel/touch waits for the main thread"))

    if not allowlisted:
        for m in LISTENER_RE.finditer(text):
            open_paren = text.index("(", m.start())
            close = matching_paren(text, open_paren)
            span = text[open_paren:close + 1]
            if re.search(r"passive\s*:\s*true", span):
                continue
            ln = line_of(text, m.start())
            if MARKER in lines[ln - 1] or (ln > 1 and MARKER in lines[ln - 2]):
                continue
            out.append((ln, f"`addEventListener('{m.group(2)}', ...)` without `passive: true` -- a blocking "
                            f"{m.group(2)} listener makes the compositor wait for the main thread"))
    return out


def main():
    root = Path(".")
    args = sys.argv[1:]
    if args:
        paths = [Path(a) for a in args]
    else:
        paths = []
        for base in ("primitives/src", "preview/src"):
            for ext in ("*.rs", "*.js"):
                paths.extend(sorted((root / base).rglob(ext)))
        idx = root / "preview/index.html"
        if idx.is_file():
            paths.append(idx)

    bad = []
    for p in paths:
        try:
            text = p.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        rel = str(p).replace("\\", "/")
        allow = rel in ALLOWLIST
        for ln, why in scan(rel, text, allow):
            bad.append((rel, ln, why))

    if bad:
        print("check-blocking-scroll-listeners: blocking wheel/touch listener(s) found:", file=sys.stderr)
        for rel, ln, why in sorted(bad):
            print(f"{rel}:{ln}: {why}", file=sys.stderr)
        print(
            "  Fix: drop the handler (touch-action / pointer events already cover drags), forward an optional "
            "handler only when Some, or add `// blocking-ok: <reason>` "
            "(dev-docs/research/scroll-jank-2026-10-04.md, Cause 2b)",
            file=sys.stderr,
        )
        sys.exit(1)

    print("check-blocking-scroll-listeners: OK -- no unconditional wheel/touch handler or blocking "
          "addEventListener outside the allowlist.")


if __name__ == "__main__":
    main()
PYEOF

python3 "$checker" "$@"
