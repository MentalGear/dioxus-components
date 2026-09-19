#!/usr/bin/env bash
#
# check-hooks-in-closures.sh
#
# Enforces Dioxus's Rules of Hooks against the one shape that's easy to
# write by accident and impossible to catch at compile time: calling a
# hook from inside the closure passed to ANOTHER hook. Dioxus tracks each
# component's hooks in a per-scope, call-order-indexed list guarded by a
# `RefCell`; a hook call inside another hook's own initializer closure
# re-enters that list while the outer hook is already borrowing it,
# panicking at runtime ("BorrowMutError: The hook list is already
# borrowed ... hook inside a hook", dioxus-core's `scope_context.rs`) the
# first time the component renders -- `cargo check`, clippy, and
# `cargo test` all stay green, since nothing about this is a type error.
#
# The 2026-09-18 incident this guards against (dev-docs/backlog.md row
# 74): `NavigationMenu`'s root component called `use_delayed_action()` --
# itself a hook wrapping `use_signal`, `menu_sub.rs` -- three times
# *inside* the closure passed to `use_context_provider`. Because
# `ComponentGallery` (the home page) renders every demo to build its
# gallery, this one bad demo's first-render panic took down the entire
# home page and every home-page-dependent test with it. Fixed (commit
# c3dda45, cherry-picked from 963d630; the original defect shipped in
# bb65cd6) by hoisting the three calls out of the closure into the
# component body, matching every other call site in the file. Proven
# below: this script fails against a scratch copy of bb65cd6's
# navigation_menu.rs and passes against HEAD's.
#
# What this catches: a call shaped like a hook -- `use_[a-z0-9_]+(`,
# called as a free function or path-qualified function, never as a
# `.method()` -- found textually inside the argument span of a call to
# one of the eight hook constructors below, each of which takes a closure
# (or async block) that becomes part of ITS OWN hook bookkeeping:
#
#   use_context_provider(|| ...)   use_signal(|| ...)
#   use_hook(|| ...)               use_resource(...)
#   use_memo(move || ...)          use_future(...)
#   use_effect(move || ...)        use_callback(...)
#
# Detection is deliberately conservative: it flags every `use_*(`-shaped
# free-function call found in that span without first trying to prove the
# callee really is a hook. This was verified empirically against this
# crate's own ~60 `fn use_*` definitions and ~300 call sites: every one of
# them -- including less obviously-named ones like `use_controlled` -- is
# in fact a hook, so a zero-false-positive run on HEAD needed no entries
# in the NON_HOOK_NAMES exclusion below. `.method()`-style calls are
# excluded from matching (this crate defines no `.use_something()`
# method today, but a future one would not be a free-function hook call
# in the sense this check cares about); so are nested `fn use_*` *item*
# definitions (as opposed to calls), which the brace-depth scan would
# otherwise be free to walk into.
#
# Why python3, not grep/bash regex: correctly finding "the argument span
# of this call" needs paren-depth tracking that also has to see past
# nested parens/braces/strings/comments -- a `format!("{}-{}", a, b)`
# inside a hook closure must not desynchronize the depth count, and a doc
# comment mentioning `use_effect(...)` in prose must not be mistaken for
# code. Same reasoning check-dx-class-prefix.sh gives for shelling out to
# python3 for CSS brace-depth tracking rather than doing it in bash.
#
# Scope: primitives/src/**/*.rs and preview/src/**/*.rs -- the two trees
# this repo's hooks and components live in.
#
# Usage: scripts/check-hooks-in-closures.sh
# Exit 0: clean. Exit 1: a hook call was found inside another hook's
# closure, with file:line detail on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

checker="$(mktemp -t check-hooks-in-closures.XXXXXX.py)"
trap 'rm -f "$checker"' EXIT

cat >"$checker" <<'PYEOF'
import re
import sys
from pathlib import Path

# The eight hook constructors whose argument is a closure (or async block)
# that becomes part of their own hook bookkeeping -- see the header
# comment above for why a hook call nested inside one of these panics.
OUTER_HOOKS = [
    "use_context_provider",
    "use_hook",
    "use_memo",
    "use_effect",
    "use_callback",
    "use_signal",
    "use_resource",
    "use_future",
]

# Free functions matching `use_[a-z0-9_]+` confirmed NOT to be hooks,
# should one ever be added to this crate. Empty today: an audit of every
# `fn use_*` this crate defines (dev-docs/backlog.md row 74's own script)
# found none that aren't real hooks. Kept as a real allowlist, not a
# hardcoded assumption, so a single narrowly-scoped exception doesn't
# require rewriting this script's logic, only adding a name here.
NON_HOOK_NAMES = {
    # "use_something_that_is_not_a_hook",
}

OUTER_RE = re.compile(r"\b(?:" + "|".join(OUTER_HOOKS) + r")\s*\(")
INNER_RE = re.compile(r"\buse_[a-z0-9_]+\s*\(")


def strip_rust(text):
    """Blank out comments and string/char literal contents, preserving
    length and newline positions, so paren-depth tracking and identifier
    matching on the remainder never trips on a brace/paren/`use_*(`-
    looking span that is only quoted or commented rather than real code
    (e.g. a `format!("{}-{}", a, b)` inside a hook closure, or a doc
    comment mentioning `use_effect(...)` in prose)."""
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
    """`text[open_pos]` is '('; return the index of its matching ')'
    (or len(text) if the file is unbalanced, which fails loudly enough
    elsewhere that this script doesn't need to special-case it)."""
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
    """True if the nearest non-whitespace character before `pos` is `.`
    -- used to exclude `.use_something(` method calls (none exist in this
    crate today; see NON_HOOK_NAMES above) from matching as free-function
    hook calls."""
    window = text[max(0, pos - 10) : pos].rstrip()
    return window.endswith(".")


def preceded_by_fn(text, pos):
    """True if `pos` sits right after the keyword `fn` -- i.e. this is a
    `fn use_x(...)` item definition, not a call, and the brace-depth scan
    should not mistake it for a nested hook invocation."""
    window = text[max(0, pos - 80) : pos]
    return re.search(r"(?<![A-Za-z0-9_])fn\s+$", window) is not None


def find_violations(path, text):
    stripped = strip_rust(text)
    violations = []
    reported_offsets = set()
    for outer in OUTER_RE.finditer(stripped):
        if preceded_by_dot(stripped, outer.start()):
            continue
        open_paren = outer.end() - 1
        close_paren = matching_paren(stripped, open_paren)
        span_start = open_paren + 1
        span = stripped[span_start:close_paren]
        for inner in INNER_RE.finditer(span):
            abs_pos = span_start + inner.start()
            if abs_pos in reported_offsets:
                continue
            name = inner.group().split("(")[0].strip()
            if name in NON_HOOK_NAMES:
                continue
            if preceded_by_dot(stripped, abs_pos):
                continue
            if preceded_by_fn(stripped, abs_pos):
                continue
            reported_offsets.add(abs_pos)
            line = text.count("\n", 0, abs_pos) + 1
            outer_name = outer.group().split("(")[0].strip()
            violations.append((path, line, name, outer_name))
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
            "check-hooks-in-closures: found hook(s) called inside another "
            "hook's closure:",
            file=sys.stderr,
        )
        for path, line, name, outer_name in sorted(all_violations):
            print(
                f"{path}:{line}: `{name}(` is called inside the closure "
                f"passed to `{outer_name}(` -- move it into the component "
                f"body instead (dev-docs/backlog.md row 74)",
                file=sys.stderr,
            )
        sys.exit(1)

    print(
        "check-hooks-in-closures: OK -- no hook called inside another "
        "hook's closure."
    )


if __name__ == "__main__":
    main()
PYEOF

python3 "$checker" "$@"
