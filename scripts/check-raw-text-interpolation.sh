#!/usr/bin/env bash
#
# check-raw-text-interpolation.sh
#
# Forbids any interpolation inside the children of an rsx `style { ... }`,
# `script { ... }`, `noscript { ... }` or `textarea { ... }` ELEMENT (not the
# `style: "..."` attribute), including path-qualified spellings:
#
#   style { "{css}" }          style { {CSS_CONST} }          style { "a {x} b" }
#   dioxus::html::style { "{css}" }      html::script { {JS} }
#   noscript { "{msg}" }       textarea { "{value}" }
#
# Why: `<style>` and `<script>` are raw-text elements (`<noscript>` is too in
# a scripting-enabled browser, and `<textarea>` is RCDATA: character
# references work but markup, comments included, does not). Dioxus SSR/SSG emits
# a hydration marker comment before every DYNAMIC text node, so the shipped
# HTML is
#
#   <style><!--node-id444-->[data-chart="dxc-394"]{--color-desktop:...;}<!--#--></style>
#
# and inside <style> that `<!--` is a CSS CDO token (ignored) while the rest
# parses as the selector `node-id444-- > [data-chart="dxc-394"]`, which
# never matches: the first rule is silently dead. (Static template text
# gets no marker, which is why a plain string LITERAL child ships clean.)
# In `<textarea>`/`<noscript>` the marker is not even inert: it ships as
# visible literal text (`<!--node-id7-->` in the box).
#
# Evidence (2026-10-03): `ChartContainer` rendered `style { "{style_rule}" }`,
# so on every deployed SSG page `--color-<series>` was never defined, bar/
# area marks fell back to black fill and line charts to no stroke. The same
# class showed up a second time in the preview app --
# `virtual_list/variants/random_heights` (`style { {INLINE_STYLE} }` shipped
# `<style><!--node-id746-->.dx-virtual-list-container {`, first rule dead) --
# and three more `style { {CONST} }` demos (carousel `sizes`, button `size`,
# drag_and_drop_list `main`) had the same shape. Two or more occurrences is
# a class, so this gate bans the shape, not the instances.
#
# What to do instead:
#   * put dynamic CSS in an inline `style:` attribute on an element (an
#     attribute value never gets a marker), or in a stylesheet asset; for a
#     textarea, set its `value:` attribute instead of a text child; or
#   * use a string LITERAL child -- `style { r#"a {{ b: c }}"# }` -- where
#     braces are escaped as `{{`/`}}`. Literal-only children are allowed.
#
# Detection: python3 (like check-hooks-in-closures.sh) because it needs a
# small Rust tokenizer (comments / string literals / code) to find the
# matching `}` of the element and to look at its direct children only. A
# string child with any `{` left after removing `{{`/`}}` pairs, or a `{`
# block child (`{expr}`, `if`/`for` blocks), is a violation. String
# attribute keys/values (`media: "{x}"`, `"data-x": ...`) are not text
# children and are ignored. `document::Style`/`document::Script` (capital
# letter, routed through the head mechanism) are out of scope, and so is
# `title`: an SVG `<title>` child is fine (84 of them ship legitimately) and
# an HTML title is set through `document::Title`. A block that is a closure
# body (`onfocus: move |e| { ... }`, `async move { ... }`) is never a child.
#
# Scope: primitives/, preview/src/, labs/, test-harness/ (*.rs).
#
# Usage: scripts/check-raw-text-interpolation.sh [ROOT_DIR]
#   ROOT_DIR defaults to the repo root (an override lets the gate be proven
#   against a scratch tree).
# Exit 0: clean. Exit 1: violation(s), file:line on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="${1:-$repo_root}"
cd "$root"

checker="$(mktemp -t check-raw-text-interpolation.XXXXXX.py)"
trap 'rm -f "$checker"' EXIT

cat >"$checker" <<'PYEOF'
import re
import sys
from pathlib import Path

SCAN_DIRS = ["primitives", "preview/src", "labs", "test-harness"]
SKIP_PARTS = {"target", "node_modules", ".git"}

# Optional lower-case module path (`dioxus::html::style {`, `html::script {`);
# `document::Style`/`Foo::style {` do not match (capital letter).
ELEM_RE = re.compile(r"(?<![\w.:])(?:[a-z_]\w*::)*(style|script|noscript|textarea)\s*\{")
# A lower-case `style`/`script`/... right after one of these is a Rust variable or
# item name (`match style {`, `if script {`, `mod style {`), not an rsx element.
ITEM_KW = {
    "mod", "struct", "enum", "fn", "impl", "trait", "use", "type", "const", "static",
    "match", "if", "while", "in", "for", "else", "return", "let", "mut", "ref", "as",
}


def tokenize(text):
    """Split Rust source into ('code'|'str'|'comment', start, end) spans.
    Strings keep their span so their contents can be inspected; comments
    and char literals are inert."""
    toks = []
    i, n = 0, len(text)
    code_start = 0

    def flush(upto):
        if upto > code_start:
            toks.append(("code", code_start, upto))

    while i < n:
        two = text[i : i + 2]
        if two == "//":
            flush(i)
            j = text.find("\n", i)
            j = n if j == -1 else j
            toks.append(("comment", i, j))
            i = code_start = j
            continue
        if two == "/*":
            flush(i)
            depth, j = 1, i + 2
            while j < n and depth:
                if text[j : j + 2] == "/*":
                    depth += 1
                    j += 2
                elif text[j : j + 2] == "*/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            toks.append(("comment", i, j))
            i = code_start = j
            continue
        m = re.match(r'[bB]?r(#*)"', text[i:])
        if m and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            flush(i)
            closer = '"' + m.group(1)
            end = text.find(closer, i + m.end())
            end = n if end == -1 else end + len(closer)
            toks.append(("str", i, end))
            i = code_start = end
            continue
        if text[i] == '"':
            flush(i)
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            toks.append(("str", i, j))
            i = code_start = j
            continue
        if text[i] == "'":
            mm = re.match(r"'(\\.|[^'\\])'", text[i:])
            if mm:
                flush(i)
                toks.append(("comment", i, i + len(mm.group(0))))
                i = code_start = i + len(mm.group(0))
                continue
        i += 1
    flush(n)
    return toks


def closure_body(text, lo, pos):
    """True if the `{` at `pos` opens a closure body (`|e| {`, `move {`,
    `async move {`) rather than an rsx child block. `lo` bounds the look-back
    to the current code span (a string/comment before it is not code)."""
    before = text[lo:pos].rstrip()
    if before.endswith("|"):
        return True
    words = re.findall(r"[A-Za-z_]\w*$", before)
    return bool(words) and words[0] in ("move", "async")


def str_body(text, s, e):
    raw = text[s:e]
    m = re.match(r'[bB]?r(#*)"', raw)
    if m:
        return raw[m.end() : len(raw) - (1 + len(m.group(1)))]
    return raw[1:-1]


def check(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    if not any(n in text for n in ("style", "script", "textarea")):
        return []
    toks = tokenize(text)
    line_of = lambda pos: text.count("\n", 0, pos) + 1
    problems = []
    for ti, (kind, s, e) in enumerate(toks):
        if kind != "code":
            continue
        for m in ELEM_RE.finditer(text, s, e):
            before = text[s : m.start()].rstrip().split()
            if before and before[-1] in ITEM_KW:
                continue
            if before and before[-1][-1] in "=&|!<":
                continue
            open_pos = m.end() - 1
            # Walk the element body token by token, tracking brace depth
            # in code spans; depth-1 strings are direct children.
            depth = 0
            prev_sig = ""  # last significant code char before a token
            end_found = False
            for tj in range(ti, len(toks)):
                k2, s2, e2 = toks[tj]
                if k2 == "comment":
                    continue
                if k2 == "str":
                    if depth == 1:
                        after = text[e2:].lstrip()[:1]
                        is_key = after == ":"
                        is_value = prev_sig == ":"
                        if not is_key and not is_value:
                            body = str_body(text, s2, e2)
                            body = body.replace("{{", "").replace("}}", "")
                            if "{" in body or "}" in body:
                                problems.append(
                                    (line_of(s2), "string child with `{` interpolation")
                                )
                    prev_sig = '"'
                    continue
                i = max(s2, open_pos) if tj == ti else s2
                while i < e2:
                    ch = text[i]
                    if ch == "{":
                        depth += 1
                        if depth == 2 and prev_sig != ":" and not closure_body(text, s2, i):
                            problems.append((line_of(i), "`{ ... }` expression/block child"))
                        prev_sig = "{"
                    elif ch == "}":
                        depth -= 1
                        prev_sig = "}"
                        if depth == 0:
                            end_found = True
                            break
                    elif not ch.isspace():
                        prev_sig = ch
                    i += 1
                if end_found:
                    break
            # Report each element once per problem (dedupe below).
    return problems


def main():
    bad = []
    for d in SCAN_DIRS:
        base = Path(d)
        if not base.is_dir():
            continue
        for p in sorted(base.rglob("*.rs")):
            if SKIP_PARTS & set(p.parts):
                continue
            for line, why in sorted(set(check(p))):
                bad.append(f"{p}:{line}: {why} inside a raw-text/RCDATA <style>/<script>/<noscript>/<textarea> element")
    if bad:
        sys.stderr.write("\n".join(bad) + "\n\n")
        sys.stderr.write(
            "Dynamic text inside <style>/<script>/<noscript>/<textarea> gets an SSR "
            "hydration marker (`<!--node-id..-->`) that corrupts the CSS/JS (or "
            "shows as literal text).\n"
            "Use an inline `style:` attribute, a stylesheet asset, a textarea "
            "`value:` attribute, or a string LITERAL child with `{{`/`}}` "
            "escapes.\n"
            "See the header of scripts/check-raw-text-interpolation.sh.\n"
        )
        return 1
    print("check-raw-text-interpolation: OK")
    return 0


sys.exit(main())
PYEOF

python3 "$checker"
