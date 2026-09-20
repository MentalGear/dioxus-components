#!/usr/bin/env python3
# PROTOTYPE - NOT A GATE. This is deliberately NOT in scripts/ and is not run by any
# check; the eleven gates CLAUDE.md lists are unaffected by it. It is the evidence behind
# dev-docs/issues/duplicate-attribute-root-cause.md and the starting point for a real,
# syn-based scripts/check-*.sh gate if the repository owner green-lights that work
# (dev-docs/backlog.md row 93).
#
# Known limits, disclosed rather than fixed: it is an indentation/line heuristic, not a
# parser, so single-line element bodies are invisible to it - its 623-finding count is a
# floor, not a ceiling. Two real false-positive bugs (multi-line fn signatures; non-pub
# Props structs) were found and fixed during its own development; more may remain.
#
# Usage: python3 dev-docs/issues/duplicate-attribute-guard-prototype.py
"""
Prototype static guard (v2): flag a literal `name: value,` (or
`"kebab-name": value,`) attribute rendered in the same brace-block as a
`..spread` when the enclosing component has no typed field/parameter of
that exact name.

PROTOTYPE for a root-cause investigation (dev-docs/backlog.md row 93 /
CLAUDE.md "deconstruct until root cause"), not a shipped gate.

Two component-authoring styles both appear in this repo and are both handled:
  (a) manual: `pub struct FooProps { pub id: ..., pub attributes: Vec<Attribute>, ... }`
      + `pub fn Foo(props: FooProps) -> Element` (primitives/src/**).
      Typed-field set = the Props struct's own field names.
  (b) `#[component]` sugar: `pub fn Foo(variant: X, attributes: Vec<Attribute>,
      children: Element) -> Element` (preview/src/components/**).
      Typed-field set = the function's own parameter names.

Known, stated limitations (a prototype, not a parser):
  - Indentation-based block nesting (relies on this repo's `cargo fmt`
    convention), not a real tokenizer; does not track string/comment
    contents char-by-char.
  - A literal attribute value that wraps across multiple lines is invisible
    (false negative, not a false positive).
  - `..spread` is recognized lexically; it cannot distinguish an rsx
    attribute spread from an ordinary Rust struct-update-syntax `..expr`
    outside of rsx! (in practice the scanned directories are overwhelmingly
    component bodies, so this is a minor residual risk, not zero).
  - Bare-ident CSS shorthand style properties (`padding`, `border_radius`,
    `flex_direction`, ...) are excluded via an AUTHORITATIVE denylist
    mechanically extracted from this Cargo.lock's own
    `dioxus-html-0.7.9/src/attribute_groups.rs` (the `ident: "kebab" in
    "style";` table) -- these accumulate into one `style="...;"` string on
    SSR (verified: dioxus-ssr-0.7.9/src/renderer.rs's `accumulated_dynamic_
    styles` handling) and are a different, non-erroring mechanism from the
    WHATWG duplicate-HTML-attribute class this investigation is about.
  - Event handlers (`on*`) and `key` are excluded, the same heuristic
    dioxus-rsx itself uses (`AttributeName::is_likely_event`).

Usage: python3 check-attr-spread-collision.py <root1> [root2 ...]
"""
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
STYLE_IDENTS_FILE = SCRIPT_DIR / "duplicate-attribute-style-shorthand-idents.txt"

STRUCT_RE = re.compile(r'^\s*(?:pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)')
FIELD_RE = re.compile(r'^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s')
FN_NAME_RE = re.compile(r'\bfn\s+([A-Za-z_][A-Za-z0-9_]*)')
OLD_STYLE_PROPS_RE = re.compile(r'^\s*(?:mut\s+)?props\s*:\s*&?\s*([A-Za-z_][A-Za-z0-9_]*)\s*$')
ATTR_RE = re.compile(
    r'^\s*(?:"([A-Za-z][\w-]*)"|([A-Za-z_][A-Za-z0-9_]*))\s*:\s*.+,\s*$'
)
SPREAD_RE = re.compile(r'^\s*\.\.\s*([A-Za-z_][\w.]*)\s*,?\s*$')

EXCLUDED_NAMES = {"key"}


def load_style_idents():
    if STYLE_IDENTS_FILE.exists():
        return {l.strip() for l in STYLE_IDENTS_FILE.read_text().splitlines() if l.strip()}
    return set()


STYLE_SHORTHAND_IDENTS = load_style_idents()


def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip(' '))


def collect_struct_fields(lines):
    """Pass 1 (global, across all files): map `pub struct FooProps` -> its top-level field names."""
    stack = []
    result = {}
    for raw in lines:
        stripped = raw.strip()
        if not stripped:
            continue
        indent = indent_of(raw)
        while stack and indent <= stack[-1][0]:
            popped = stack.pop()
            if popped[1] == 'struct':
                result[popped[2]] = popped[3]
        m = STRUCT_RE.match(raw)
        if m:
            stack.append([indent, 'struct', m.group(1), set()])
            continue
        if stripped.endswith('{'):
            stack.append([indent, 'other', None, None])
            continue
        if stack and stack[-1][1] == 'struct' and indent == stack[-1][0] + 4:
            fm = FIELD_RE.match(raw)
            if fm:
                stack[-1][3].add(fm.group(1))
    while stack:
        popped = stack.pop()
        if popped[1] == 'struct':
            result[popped[2]] = popped[3]
    return result


def extract_fn_param_names(text, struct_fields):
    """Whole-file, paren-balanced scan: fn_name -> set(typed field/param names) | None.

    Handles both authoring styles: if the parsed parameter list is exactly
    `props: XProps`, resolve to struct_fields[XProps] (manual style);
    otherwise use the literal parameter names themselves (#[component] style).
    """
    result = {}
    for m in FN_NAME_RE.finditer(text):
        fn_name = m.group(1)
        i = m.end()
        n = len(text)
        # skip generic params <...> before the paren, if present
        while i < n and text[i] in ' \t\r\n':
            i += 1
        if i < n and text[i] == '<':
            depth = 0
            while i < n:
                if text[i] == '<':
                    depth += 1
                elif text[i] == '>':
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                i += 1
            while i < n and text[i] in ' \t\r\n':
                i += 1
        if i >= n or text[i] != '(':
            continue
        start = i
        depth = 0
        j = i
        while j < n:
            if text[j] == '(':
                depth += 1
            elif text[j] == ')':
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        else:
            continue
        param_text = text[start + 1:j - 1]

        # Split top-level (bracket-depth 0) commas
        parts = []
        depth = 0
        cur = []
        for ch in param_text:
            if ch in '([<':
                depth += 1
            elif ch in ')]>':
                depth = max(0, depth - 1)
            if ch == ',' and depth == 0:
                parts.append(''.join(cur))
                cur = []
            else:
                cur.append(ch)
        if ''.join(cur).strip():
            parts.append(''.join(cur))

        # Old style: a single `props: XProps` parameter
        if len(parts) == 1:
            om = OLD_STYLE_PROPS_RE.match(parts[0].strip())
            if om:
                result[fn_name] = struct_fields.get(om.group(1))
                continue

        names = set()
        ok = True
        for part in parts:
            p = re.sub(r'#\[[^\]]*\]', '', part).strip()
            p = re.sub(r'^mut\s+', '', p)
            if not p or p in ('self', '&self', '&mut self'):
                continue
            pm = re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*:', p)
            if pm:
                names.add(pm.group(1))
            else:
                ok = False
        result[fn_name] = names if ok else (names or None)
    return result


def scan_elements(path, lines, fn_typed_fields, findings, unresolved_ctx_counter):
    stack = []
    pending_fn_name = [None]

    def finalize(frame):
        if frame['kind'] != 'other':
            return
        if not frame['has_spread'] or not frame['attrs']:
            return
        governing = frame['ctx_fn']
        typed_fields = fn_typed_fields.get(governing) if governing else None
        for (name, lineno, is_kebab_literal) in frame['attrs']:
            if name in EXCLUDED_NAMES or name.startswith('on'):
                continue
            if (not is_kebab_literal) and name in STYLE_SHORTHAND_IDENTS:
                continue
            if is_kebab_literal:
                findings.append((path, lineno, name, governing, 'always (string-literal name, cannot be a typed field)'))
            elif typed_fields is None:
                unresolved_ctx_counter[0] += 1
                findings.append((path, lineno, name, governing, 'unresolved enclosing fn'))
            elif name not in typed_fields:
                findings.append((path, lineno, name, governing, 'no typed field/param of this name'))

    for i, raw in enumerate(lines):
        lineno = i + 1
        stripped = raw.strip()
        if not stripped or stripped.startswith('//'):
            continue
        indent = indent_of(raw)
        while stack and indent <= stack[-1]['start_indent']:
            finalize(stack.pop())

        ctx = stack[-1]['ctx_fn'] if stack else None

        if stack and indent == stack[-1]['start_indent'] + 4:
            am = ATTR_RE.match(raw)
            if am:
                name = am.group(1) or am.group(2)
                is_kebab = am.group(1) is not None
                stack[-1]['attrs'].append((name, lineno, is_kebab))
            sm = SPREAD_RE.match(raw)
            if sm:
                stack[-1]['has_spread'] = True

        # Track a not-yet-consumed `fn NAME` signature across possibly
        # multiple lines (rustfmt wraps multi-param signatures one-per-line,
        # so the line ending in `{` is often just `) -> Element {`, with no
        # `fn`/name on it at all).
        fn_m = FN_NAME_RE.search(raw)
        if fn_m:
            pending_fn_name[0] = fn_m.group(1)

        if stripped.endswith('{'):
            is_struct = STRUCT_RE.match(raw)
            if pending_fn_name[0] is not None and not is_struct:
                stack.append({'start_indent': indent, 'kind': 'fn',
                               'ctx_fn': pending_fn_name[0], 'attrs': [], 'has_spread': False})
            elif is_struct:
                stack.append({'start_indent': indent, 'kind': 'struct',
                               'ctx_fn': ctx, 'attrs': [], 'has_spread': False})
            else:
                stack.append({'start_indent': indent, 'kind': 'other',
                               'ctx_fn': ctx, 'attrs': [], 'has_spread': False})
            pending_fn_name[0] = None

    while stack:
        finalize(stack.pop())


def main(roots):
    files = []
    for root in roots:
        p = Path(root)
        if p.is_file():
            files.append(p)
        else:
            files.extend(sorted(p.rglob('*.rs')))

    texts = {}
    all_lines_by_file = {}
    struct_fields = {}
    for f in files:
        try:
            text = f.read_text()
        except Exception as e:
            print(f"# SKIP {f}: {e}", file=sys.stderr)
            continue
        texts[f] = text
        lines = text.splitlines()
        all_lines_by_file[f] = lines
        struct_fields.update(collect_struct_fields(lines))

    findings = []
    unresolved_ctr = [0]
    for f, lines in all_lines_by_file.items():
        fn_typed_fields = extract_fn_param_names(texts[f], struct_fields)
        scan_elements(f, lines, fn_typed_fields, findings, unresolved_ctr)

    findings.sort(key=lambda x: (str(x[0]), x[1]))
    for (path, lineno, name, governing, reason) in findings:
        print(f"{path}:{lineno}: literal `{name}` beside a spread; enclosing fn={governing!r}; {reason}")

    print(f"\n# TOTAL findings: {len(findings)}", file=sys.stderr)
    print(f"# ...of which unresolved-context: {unresolved_ctr[0]}", file=sys.stderr)
    print(f"# Struct defs indexed: {len(struct_fields)}", file=sys.stderr)
    print(f"# Files scanned: {len(all_lines_by_file)}", file=sys.stderr)
    print(f"# Style-shorthand idents loaded (excluded): {len(STYLE_SHORTHAND_IDENTS)}", file=sys.stderr)


if __name__ == '__main__':
    # Default to the two roots the original run used. Scanning '.' from the repo root is
    # wrong: it walks .claude/worktrees/ and target-lane-*/ and reports tens of thousands of
    # findings from vendored and lane-local copies.
    DEFAULT_ROOTS = ['primitives/src', 'preview/src/components']
    main(sys.argv[1:] or DEFAULT_ROOTS)
