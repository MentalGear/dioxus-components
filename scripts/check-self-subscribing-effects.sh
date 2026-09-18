#!/usr/bin/env bash
#
# check-self-subscribing-effects.sh
#
# Enforces a Dioxus reactivity invariant that, like the hook-inside-hook
# shape check-hooks-in-closures.sh guards against, is easy to write by
# accident and invisible to `cargo check`/clippy/`cargo test`: inside a
# `use_effect(move || { ... })` body, reading a signal with tracked syntax
# (`x()` or `x.read()`) and then writing that same signal (`x.set(...)`,
# `x.write()`, `*x.write()`) in the same run subscribes the effect to a
# value it just changed itself. Dioxus reschedules the effect whenever a
# signal it read changes, so this can re-trigger the effect from its own
# write -- forever, if nothing else in that same run breaks the cycle.
#
# The 2026-09-18 incident this guards against (dev-docs/backlog.md row
# 73): `DrawerContent`'s drag-continuation `use_effect` read
# `raw_offset()` with tracked syntax and then wrote a new value to that
# same signal via `.set()` on every pointermove while dragging -- nothing
# on that path ever broke the cycle, so the effect re-invoked itself
# synchronously from the first `pointerdown`, wedging the tab's whole main
# thread (not just the component -- a `page.evaluate` in the same tab hung
# too). Fixed (commit b7037cb, cherry-picked from 896e603; the original
# defect shipped in cdb073b) by reading `raw_offset` through `.peek()`
# instead of tracked syntax for both of the effect's self-referential
# reads, matching the read-through-`.peek()`-write-through-`.set()`
# discipline `lib.rs`'s `use_animated_open` already documents for its own
# generation counter. Proven below: this script fails against a scratch
# copy of cdb073b's drawer.rs (on `raw_offset`, plus two more identifiers
# that turn out to be pre-existing and bounded -- see "Known HEAD
# findings" below) and no longer flags `raw_offset` against HEAD's fixed
# version.
#
# What this catches, per `use_effect(move || { ... })` closure body: any
# identifier read with tracked syntax -- a bare `x()` call (empty
# parens; a call WITH arguments is an ordinary function call, not a
# signal read) or `x.read()` -- that is ALSO written in the same closure
# via `x.set(`, `x.write()`, or `*x.write()`. `.peek()` reads never count
# (that is the fix's own idiom: read the current value without
# subscribing), and neither does any read syntax other than the two named
# above -- e.g. `.cloned()`/`.take()` on a `CopyValue` is not a tracked
# Signal read at all, and is deliberately not flagged.
#
# Two categories are excluded by construction, not by per-name allowlist,
# because they are provably not the mechanism this script guards against
# (verified by reading dioxus-signals 0.7.9's own source, not assumed):
#
#   1. `CopyValue<T>` identifiers. `copy_value.rs` implements `Readable`
#      for `CopyValue` with `fn subscribers(&self) -> Subscribers {
#      Subscribers::new_noop() }` -- reading a `CopyValue` (via its own
#      `Deref<Target = dyn Fn() -> T>`, the same bare `x()` call sugar a
#      `Signal` supports) can NEVER register a subscription, so writing
#      to one afterward cannot re-trigger anything. Declarations of the
#      shape `let (mut )?NAME = use_hook(|| CopyValue::new(...))` are
#      found first (COPY_VALUE_DECL_RE, a whole-file pass) and every
#      name it finds is excluded everywhere in that file. HEAD has three
#      real examples this exclusion was written against: `main.rs`'s
#      `initial_route`, `color_picker.rs`'s `granular_value`, and
#      `lib.rs`'s `cleanup` (the last one doubly so -- its apparent
#      "read" is actually a *different*, shadowed local of the same name
#      bound by `if let Some(cleanup) = cleanup.take()`, not a read of
#      the outer `CopyValue` at all; excluding the name sidesteps having
#      to model shadowing to reach the same correct answer).
#   2. Code inside a `spawn(async move { ... })` nested in the effect's
#      own body. `spawn` hands a future to the async executor as an
#      independent, fire-and-forget task; unlike an effect whose own
#      body directly returns a future (not a pattern this crate uses),
#      Dioxus's dependency tracking only records reads made during the
#      effect closure's own synchronous execution, so nothing a spawned
#      task reads or writes later -- possibly milliseconds or seconds
#      after the effect itself already returned -- is attributed to that
#      effect's subscriptions at all. HEAD's `BlockPlayer` demo
#      (`main.rs`) is the real example this was written against: a
#      `progress_seconds` signal is read and `.set()` inside a `spawn`ed
#      polling loop, textually nested inside a `use_effect` whose own
#      synchronous body reads no signals (so it only ever runs once, on
#      mount) -- a plain top-level `spawn(` (never `.spawn(`, which this
#      crate uses once, in `pointer.rs`, but not inside any `use_effect`)
#      found inside the effect's span has its own matching-paren span
#      computed and excluded from the read/write scan the same way.
#
# Everything else stays a syntactic, conservative check: it does not
# attempt to prove a given read+write pair on a real `Signal` is
# REACHABLE on the same run, or that the re-trigger it causes is
# unbounded rather than self-terminating (see "Known HEAD findings"
# below for two real, analyzed examples of the latter). That pattern is
# inherently fragile even when a particular instance happens to
# terminate today, so it is worth surfacing on sight rather than only
# once it has already hung a page.
#
# Known HEAD findings (reported, not fixed here -- none of these files
# are owned by this change): three real `Signal`s, in two files, are
# read with tracked syntax and later written in the same `use_effect`,
# and none of them is the row-73 shape (an unconditional write on every
# firing with nothing to ever break the cycle):
#   - `drawer.rs`'s `DrawerContent` effect: `dragging`/`active_pointer_id`
#     are read at the top of the closure and only written (to `false`/
#     `None`) behind the same `if !dragging() { return; }` guard they
#     both sit under, so the one extra re-run the write schedules reads
#     `dragging()` as `false` and returns immediately without reading or
#     writing anything else -- bounded, not infinite.
#   - `color_picker/component.rs`'s two hex/hue sync effects: `value` and
#     `current_hue` are each written only inside an `if` that already
#     checked the new value differs from what was just read (`parsed !=
#     external_rgb`, `value != current`) -- the standard "sync from an
#     external source, but do not clobber unless it actually changed"
#     idiom. Bounded as long as the round trip through that comparison
#     converges (i.e. writing the synced value makes the next run's same
#     comparison false); not re-proven here for floating-point/hex
#     rounding edge cases, which is exactly the kind of runtime property
#     this script cannot and should not try to prove.
#   - `collection.rs`'s `use_deferred_collection_focus`: `placement` is
#     read then written to `None` behind an `else`/`if` shape that, like
#     `dragging` above, makes the next run's read see the very state
#     that stops it writing again.
# All three are left for this script to keep finding rather than
# allowlisted away -- NON_SELF_TERMINATING below stays empty until each
# file's owner or the main loop decides what, if anything, to do about
# them.
#
# Why python3, not grep/bash regex: same reasoning as
# check-hooks-in-closures.sh -- finding "the body of this use_effect"
# needs paren-depth tracking that sees past nested parens/braces/strings/
# comments, and distinguishing `x()` from `x(some_arg)` or `x.read()` from
# `x.peek()` needs real matching, not a line-oriented grep.
#
# Scope: primitives/src/**/*.rs and preview/src/**/*.rs.
#
# Usage: scripts/check-self-subscribing-effects.sh
# Exit 0: clean. Exit 1: a signal was read with tracked syntax and written
# in the same use_effect closure, with file:line detail on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

checker="$(mktemp -t check-self-subscribing-effects.XXXXXX.py)"
trap 'rm -f "$checker"' EXIT

cat >"$checker" <<'PYEOF'
import re
import sys
from pathlib import Path

# (identifier, containing-file-suffix) pairs confirmed, by hand, to
# re-trigger at most once and then go quiescent -- see the header
# comment's "Known HEAD findings". Empty by default: populating this is a
# deliberate, reviewed decision (by whoever owns the file in question),
# not something this script does on its own initiative.
NON_SELF_TERMINATING = {
    # ("drawer.rs", "dragging"),
    # ("drawer.rs", "active_pointer_id"),
    # ("component.rs", "value"),
    # ("component.rs", "current_hue"),
    # ("collection.rs", "placement"),
}

USE_EFFECT_RE = re.compile(r"\buse_effect\s*\(")
READ_BARE_RE = re.compile(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\(\s*\)")
READ_METHOD_RE = re.compile(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\.read\(\s*\)")
WRITE_SET_RE = re.compile(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\.set\(")
WRITE_WRITE_RE = re.compile(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\.write\(\s*\)")

# See the header comment's exclusion (1): a `CopyValue`'s `subscribers()`
# is a hardcoded no-op, so reading one (even with the same bare `x()`
# call sugar a `Signal` uses) can never subscribe an effect.
COPY_VALUE_DECL_RE = re.compile(
    r"\blet\s+(?:mut\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\b[^=;{}]*=\s*"
    r"use_hook\(\s*(?:move\s*)?\|\|\s*CopyValue::new\("
)

# See the header comment's exclusion (2): a plain top-level `spawn(`
# (never a `.spawn(` method call, which this crate uses but not inside
# any `use_effect`) hands its future to the executor independently of
# the enclosing effect's own dependency tracking.
SPAWN_RE = re.compile(r"\bspawn\s*\(")


def strip_rust(text):
    """See check-hooks-in-closures.sh -- identical comment/string/char
    stripping, preserving length and newlines, so brace/paren tracking
    and identifier matching never trips on a quoted or commented span
    (a `format!("{}-{}", a, b)`'s braces, or a doc comment's prose)."""
    out = []
    i, n = 0, len(text)
    while i < n:
        two = text[i : i + 2]
        if two == "//":
            while i < n and text[i] != "\n":
                out.append(" ")
                i += 1
            continue
        if two == "/*":
            out.append("  ")
            i += 2
            while i < n and text[i : i + 2] != "*/":
                out.append("\n" if text[i] == "\n" else " ")
                i += 1
            if i < n:
                out.append("  ")
                i += 2
            continue
        m = re.match(r'[bB]?r(#*)"', text[i:])
        if m:
            hashes = m.group(1)
            start = i + m.end()
            closer = '"' + hashes
            end = text.find(closer, start)
            end = n if end == -1 else end + len(closer)
            for ch in text[i:end]:
                out.append("\n" if ch == "\n" else " ")
            i = end
            continue
        if text[i] == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            for ch in text[i:j]:
                out.append("\n" if ch == "\n" else " ")
            i = j
            continue
        if text[i] == "'":
            mm = re.match(r"'(\\.|[^'\\])'", text[i:])
            if mm:
                out.append(" " * len(mm.group(0)))
                i += len(mm.group(0))
                continue
            out.append("'")
            i += 1
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def matching_paren(text, open_pos):
    depth = 0
    i = open_pos
    n = len(text)
    while i < n:
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return n


def preceded_by_dot(text, pos):
    window = text[max(0, pos - 10) : pos].rstrip()
    return window.endswith(".")


def preceded_by_colons(text, pos):
    window = text[max(0, pos - 10) : pos].rstrip()
    return window.endswith("::")


def spawned_task_spans(stripped, span_start, span_end):
    """Every `spawn(...)`'s own matching-paren span found inside
    [span_start, span_end) -- see the header comment's exclusion (2)."""
    spans = []
    for m in SPAWN_RE.finditer(stripped, span_start, span_end):
        if preceded_by_dot(stripped, m.start()):
            continue
        open_paren = m.end() - 1
        close_paren = matching_paren(stripped, open_paren)
        spans.append((open_paren, close_paren))
    return spans


def inside_any(pos, spans):
    return any(start <= pos < end for start, end in spans)


def find_violations(path, text):
    stripped = strip_rust(text)
    violations = []
    copy_value_names = {m.group(1) for m in COPY_VALUE_DECL_RE.finditer(stripped)}
    base_name = Path(path).name

    for m in USE_EFFECT_RE.finditer(stripped):
        if preceded_by_dot(stripped, m.start()):
            continue
        open_paren = m.end() - 1
        close_paren = matching_paren(stripped, open_paren)
        span_start = open_paren + 1
        span = stripped[span_start:close_paren]
        spawn_spans = spawned_task_spans(stripped, span_start, close_paren)

        reads = {}
        writes = {}

        for rm in READ_BARE_RE.finditer(span):
            abs_pos = span_start + rm.start()
            if preceded_by_dot(stripped, abs_pos) or preceded_by_colons(stripped, abs_pos):
                continue
            if inside_any(abs_pos, spawn_spans):
                continue
            reads.setdefault(rm.group(1), abs_pos)

        for rm in READ_METHOD_RE.finditer(span):
            abs_pos = span_start + rm.start()
            if inside_any(abs_pos, spawn_spans):
                continue
            reads.setdefault(rm.group(1), abs_pos)

        for wm in WRITE_SET_RE.finditer(span):
            abs_pos = span_start + wm.start()
            if inside_any(abs_pos, spawn_spans):
                continue
            writes.setdefault(wm.group(1), abs_pos)

        for wm in WRITE_WRITE_RE.finditer(span):
            abs_pos = span_start + wm.start()
            if inside_any(abs_pos, spawn_spans):
                continue
            writes.setdefault(wm.group(1), abs_pos)

        for name in sorted(set(reads) & set(writes)):
            if name in copy_value_names:
                continue
            if (base_name, name) in NON_SELF_TERMINATING:
                continue
            read_line = text.count("\n", 0, reads[name]) + 1
            write_line = text.count("\n", 0, writes[name]) + 1
            violations.append((path, min(read_line, write_line), name, read_line, write_line))

    return violations


def main():
    targets = sys.argv[1:]
    if targets:
        paths = [Path(t) for t in targets]
    else:
        paths = []
        for root in (Path("primitives/src"), Path("preview/src")):
            if root.is_dir():
                paths.extend(sorted(root.rglob("*.rs")))

    all_violations = []
    for path in paths:
        text = path.read_text(encoding="utf-8")
        all_violations.extend(find_violations(str(path), text))

    if all_violations:
        print(
            "check-self-subscribing-effects: found signal(s) read with "
            "tracked syntax and written in the same use_effect closure:",
            file=sys.stderr,
        )
        for path, _, name, read_line, write_line in sorted(all_violations):
            print(
                f"{path}:{read_line}: `{name}` is read with tracked syntax "
                f"here and written at line {write_line} in the same "
                f"use_effect closure -- read through `.peek()` instead "
                f"unless the write is proven to end the effect's own "
                f"re-entry (dev-docs/backlog.md row 73)",
                file=sys.stderr,
            )
        sys.exit(1)

    print(
        "check-self-subscribing-effects: OK -- no signal read with tracked "
        "syntax is written in the same use_effect closure."
    )


if __name__ == "__main__":
    main()
PYEOF

python3 "$checker" "$@"
