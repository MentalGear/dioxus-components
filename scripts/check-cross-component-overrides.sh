#!/usr/bin/env bash
#
# check-cross-component-overrides.sh
#
# Flags a NON-custom property declared, in component X's stylesheet, on an element that is
# really component Y's:
#
#   .dx-field-label { font-size: var(--dx-text-sm); }   /* in field/style.css */
#   .dx-label       { font-size: 0.8rem; }              /* in label/style.css; same element */
#
# Both selectors are (0,1,0) and both set `font-size`, so the winner is whichever stylesheet
# loads LAST -- and the load order is per route (`<link>`s are injected as components render).
# The STYLE-DELIVERY lane found 208 sheet pairs ordered both ways across routes, and the
# Field label was 14px on the home gallery and 12.8px on /component/field/. Patching it with a
# specificity bump (`label.dx-field-label`) only moves the contest; it does not remove it.
#
# THE CONSTRUCTION: composition through custom properties (the CSS form of shadcn's `cn()` /
# tailwind-merge, which deterministically drops the losing utility). The base component reads
# a SLOT variable with its own default --
#
#   .dx-label       { font-size: var(--dx-label-font-size, var(--dx-text-sm)); }
#
# -- and the composing component sets the VARIABLE, never the property --
#
#   .dx-field-label { --dx-label-line-height: 1.375; }
#
# A custom property is a different property from the one the base declares, so the two rules
# never contest anything: no specificity race, no order dependence. This gate keeps the
# remaining non-custom declarations from growing. Slot variables are documented in each base
# component's docs.md ("Slot variables").
#
# WHAT IT DETECTS (both are properties declared by a rule in X's own `style.css`):
#
#   composed  The rule's subject (rightmost compound selector) carries a class that X's
#             `component.rs` passes to a themed component `Y` it composes -- i.e. one element
#             wearing both `dx-Y` and `dx-X-...` (`FieldLabel` renders
#             `<label class="dx-label dx-field-label">`). Derived from the Rust: a class
#             literal inside `attributes!(..)` (followed through
#             `let m = merge_attributes(vec![base, ..])`) that reaches `Y { attributes: m }`,
#             or `Y { class: "dx-X-.." }`, where `Y` is imported from `crate::components::<y>`.
#   tag       The rule's subject is a bare `input|select|textarea|button|label` element (no
#             class). Every themed control (Input, NativeSelect, Textarea, Button, Checkbox,
#             Switch, RadioItem, Label) renders one of those, and `check-dx-class-prefix.sh`
#             forbids naming another component's `dx-Y` class in X's sheet -- so an element
#             selector (`.dx-button-group > button`) is the one remaining way to reach Y's root.
#             This also matches an X's OWN raw element (`.dx-toolbar button`); those sit in the
#             baseline too, and the cure is the same (give the element its own class).
#
# A class selector naming ANOTHER component's class cannot occur at all: that is exactly what
# `check-dx-class-prefix.sh` enforces, so it is not re-checked here.
#
# WHAT IT DOES NOT DETECT: a composition whose class reaches Y through a path this script does
# not follow (a `class` threaded through a prop it does not recognise, a builder helper, a
# macro); element selectors for tags other than the five above (`svg`, `span`, `a`, `img`,
# `option` -- no themed component has one of those as its root); a placement/layout rule
# written in the PARENT's own wrapper element. A miss is a missing baseline entry, never a
# false failure.
#
# THE RATCHET, NOT A WALL (same shape as check-attr-spread-collision.sh):
# `scripts/check-cross-component-overrides.baseline.tsv` lists the sites that existed when the
# gate was introduced, one `file<TAB>kind<TAB>selector<TAB>property` per line (selector with
# whitespace collapsed; NOT file:line, so unrelated edits above a site do not disturb it; a
# multiset, because the same selector under two media queries is two sites). The gate fails
# only on a site that is NOT in the baseline. It is a DEBT REGISTER, NOT AN APPROVAL: do not add
# a line to make the gate pass -- convert the site to a slot variable instead. When a site
# disappears the gate prints a note and `--update-baseline` locks the shrink in.
#
# Usage:
#   scripts/check-cross-component-overrides.sh                  check against the baseline
#   scripts/check-cross-component-overrides.sh --inventory      print every site, with file:line
#   scripts/check-cross-component-overrides.sh --update-baseline
#       rewrite the baseline from the current tree. Refuses to ADD a line (shrink-only) unless
#       the baseline file does not exist yet, or --allow-grow is also given (say why in the commit).
# Exit status: 0 = no new site; 1 = a site not in the baseline; 2 = bad usage / parse failure.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

baseline="scripts/check-cross-component-overrides.baseline.tsv"
mode=check
allow_grow=0
for arg in "$@"; do
    case "$arg" in
        --inventory) mode=inventory ;;
        --update-baseline) mode=update ;;
        --allow-grow) allow_grow=1 ;;
        -h|--help) sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "check-cross-component-overrides: unknown option: $arg" >&2; exit 2 ;;
    esac
done

extractor="$(mktemp -t cross-component-overrides.XXXXXX.py)"
raw="$(mktemp -t cross-component-overrides.XXXXXX.raw)"
trap 'rm -f "$extractor" "$raw"' EXIT

cat >"$extractor" <<'PYEOF'
import os
import re
import sys

COMPONENTS = "preview/src/components"
FORM_TAGS = ("input", "select", "textarea", "button", "label")


# ----------------------------------------------------------------------------- CSS
def strip_comments(css):
    # Keep the newlines, so line numbers survive.
    return re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), css, flags=re.S)


def parse_rules(css):
    """Return [(prelude, [declaration-text], line, [enclosing at-rule preludes])] for every
    style rule. Quote- and paren-aware, so `url("data:..;..{")` cannot split a declaration."""
    css = strip_comments(css)
    rules, stack, buf = [], [], []
    quote, paren, i, n = None, 0, 0, len(css)
    while i < n:
        ch = css[i]
        if quote:
            buf.append(ch)
            if ch == "\\" and i + 1 < n:
                buf.append(css[i + 1])
                i += 1
            elif ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            buf.append(ch)
        elif ch == "(":
            paren += 1
            buf.append(ch)
        elif ch == ")":
            paren = max(0, paren - 1)
            buf.append(ch)
        elif paren > 0:
            buf.append(ch)
        elif ch == "{":
            prelude = "".join(buf)
            lead = len(prelude) - len(prelude.lstrip())
            start = i - len(prelude) + lead
            stack.append({
                "kind": "at" if prelude.strip().startswith("@") else "rule",
                "prelude": prelude.strip(),
                "line": css.count("\n", 0, start) + 1,
                "decls": [],
            })
            buf = []
        elif ch == ";":
            decl = "".join(buf).strip()
            if stack and stack[-1]["kind"] == "rule" and decl:
                stack[-1]["decls"].append(decl)
            buf = []
        elif ch == "}":
            decl = "".join(buf).strip()
            if stack:
                top = stack.pop()
                if top["kind"] == "rule":
                    if decl:
                        top["decls"].append(decl)
                    ctx = [s["prelude"] for s in stack if s["kind"] == "at"]
                    rules.append((top["prelude"], top["decls"], top["line"], ctx))
            buf = []
        else:
            buf.append(ch)
        i += 1
    return rules


def split_top(s, sep):
    out, depth, cur, quote = [], 0, [], None
    for ch in s:
        if quote:
            cur.append(ch)
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            cur.append(ch)
        else:
            if ch in "([":
                depth += 1
            elif ch in ")]":
                depth -= 1
            if ch == sep and depth == 0:
                out.append("".join(cur))
                cur = []
            else:
                cur.append(ch)
    out.append("".join(cur))
    return out


def subject_compound(sel):
    """The rightmost compound selector of a complex selector."""
    parts, depth, cur, quote = [], 0, [], None
    for ch in sel.strip():
        if quote:
            cur.append(ch)
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            cur.append(ch)
        else:
            if ch in "([":
                depth += 1
            elif ch in ")]":
                depth -= 1
            if depth == 0 and ch in " \t\n>+~":
                if cur:
                    parts.append("".join(cur))
                    cur = []
            else:
                cur.append(ch)
    if cur:
        parts.append("".join(cur))
    return parts[-1] if parts else ""


def strip_not_has(compound):
    """Drop `:not(..)` / `:has(..)` arguments: a class there is not what the rule styles."""
    out, i = [], 0
    while i < len(compound):
        m = re.match(r":(not|has)\(", compound[i:])
        if not m:
            out.append(compound[i])
            i += 1
            continue
        depth, j = 0, i + m.end() - 1
        while j < len(compound):
            if compound[j] == "(":
                depth += 1
            elif compound[j] == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        i = j + 1
    return "".join(out)


# ----------------------------------------------------------------------------- Rust
def balanced(src, open_idx):
    """src[open_idx] is `{`, `(` or `[`; return the text between it and its partner."""
    pairs = {"{": "}", "(": ")", "[": "]"}
    o, c = src[open_idx], pairs[src[open_idx]]
    depth, j = 0, open_idx
    while j < len(src):
        if src[j] == o:
            depth += 1
        elif src[j] == c:
            depth -= 1
            if depth == 0:
                return src[open_idx + 1:j]
        j += 1
    return src[open_idx + 1:]


def depth1(body):
    """`body` with everything nested inside `{}` `()` `[]` blanked: the call's own arguments."""
    out, depth = [], 0
    for ch in body:
        if ch in "{([":
            depth += 1
            out.append(" ")
        elif ch in "})]":
            depth -= 1
            out.append(" ")
        else:
            out.append(ch if depth == 0 else (" " if ch != "\n" else "\n"))
    return "".join(out)


def class_literals(text):
    found = []
    for m in re.finditer(r'\bclass\s*:\s*"([^"]*)"', text):
        found += [c for c in m.group(1).split() if c.startswith("dx-")]
    return found


def exported_idents(y):
    """Component idents a glob `use ..::<y>::*` brings in: `<y>/component.rs`'s `pub fn Name`."""
    path = f"{COMPONENTS}/{y}/component.rs"
    if not os.path.isfile(path):
        return set()
    with open(path, encoding="utf-8") as f:
        return set(re.findall(r"^\s*pub(?:\([^)]*\))?\s+fn\s+([A-Z]\w*)", f.read(), flags=re.M))


def themed_imports(src, own):
    """ident -> component dir, for `use crate::components::<y>::{A, B}` / `::*`, the
    `super::[super::]<y>::..` spellings, `pub use` re-exports, and fully qualified calls."""
    m = {}
    pat = r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+(?:crate::components|super::super|super)::(\w+)::(\{[^}]*\}|\*|\w+)(?:\s+as\s+\w+)?\s*;"
    for im in re.finditer(pat, src, flags=re.M | re.S):
        y, items = im.group(1), im.group(2)
        if y == own or not os.path.isdir(f"{COMPONENTS}/{y}"):
            continue
        idents = exported_idents(y) if items == "*" else set(re.findall(r"\b[A-Z]\w*", items))
        for ident in idents:
            m[ident] = y
    return m


def composed_classes(src, own):
    """{class: composed component dir} for classes this file's functions put on a themed
    component's root element."""
    imports = themed_imports(src, own)
    qualified = {}  # ident -> y, for `crate::components::<y>::Ident {` written out in full
    for im in re.finditer(r"crate::components::(\w+)::([A-Z]\w*)\s*\{", src):
        qualified[im.group(2)] = im.group(1)
    if not imports and not qualified:
        return {}
    result = {}
    starts = [m.start() for m in re.finditer(r"^(?:pub(?:\([^)]*\))? )?fn \w+", src, flags=re.M)] + [len(src)]
    for a, b in zip(starts, starts[1:]):
        fn = src[a:b]
        env = {}
        for m in re.finditer(r"let\s+(?:mut\s+)?(\w+)\s*=\s*attributes!\s*\(", fn):
            env[m.group(1)] = set(class_literals(balanced(fn, m.end() - 1)))
        for m in re.finditer(r"let\s+(?:mut\s+)?(\w+)\s*=\s*merge_attributes\s*\(", fn):
            args = balanced(fn, m.end() - 1)
            names = re.findall(r"\b([a-z_]\w*)\b(?:\.clone\(\))?", args)
            env[m.group(1)] = set().union(*(env.get(nm, set()) for nm in names)) if names else set()
        calls = []  # (component dir, index of the call's opening brace)
        for ident, y in imports.items():
            for m in re.finditer(r"(?<![\w:])" + ident + r"\s*\{", fn):
                calls.append((y, m.end() - 1))
        for ident, y in qualified.items():
            for m in re.finditer(r"crate::components::" + y + r"::" + ident + r"\s*\{", fn):
                calls.append((y, m.end() - 1))
        for y, brace in calls:
            raw_body = balanced(fn, brace)
            args = depth1(raw_body)  # the call's own arguments, children blanked
            classes = set()
            # `class: "dx-.."` written directly on the call (top-level argument only)
            for cm in re.finditer(r'\bclass\s*:\s*"([^"]*)"', raw_body):
                if args[cm.start():cm.start() + 5] == "class":
                    classes |= {c for c in cm.group(1).split() if c.startswith("dx-")}
            for vm in re.finditer(r"\battributes\s*(?::\s*(\w+)|,|$|\n)", args):
                var = vm.group(1) or "attributes"
                if var in env:
                    classes |= env[var]
                else:
                    # `attributes` is a closure parameter -- a primitive's `as:` hand-off
                    # (`Prim { attributes: merged, r#as: move |attributes| .. Button { attributes } }`):
                    # everything this function forwards through `attributes: <var>` flows into it.
                    for fm in re.finditer(r"\battributes\s*:\s*(\w+)", fn):
                        classes |= env.get(fm.group(1), set())
            for c in classes:
                result.setdefault(c, y)
    return result


# ----------------------------------------------------------------------------- scan
def all_composed():
    """Classes put on a composed themed component's root, from every `.rs` under a component
    dir -- demos (`variants/`) included, because the stylesheet is shipped and a demo that hangs
    `dx-chart-interactive` on a `Card` is exercising exactly that rule. Global, not per
    component: `bar_chart/component.rs` puts `dx-chart-footer` on a `CardFooter` while the rule
    for it lives in `chart/style.css`."""
    composed = {}
    for x in sorted(os.listdir(COMPONENTS)):
        d = f"{COMPONENTS}/{x}"
        if not os.path.isdir(d):
            continue
        for root, _dirs, files in os.walk(d):
            for fname in sorted(files):
                if fname.endswith(".rs"):
                    with open(os.path.join(root, fname), encoding="utf-8") as f:
                        for c, y in composed_classes(f.read(), x).items():
                            composed.setdefault(c, y)
    return composed


def scan():
    sites = []  # (file, line, kind, selector, property)
    composed = all_composed()
    for x in sorted(os.listdir(COMPONENTS)):
        css_path = f"{COMPONENTS}/{x}/style.css"
        if not os.path.isfile(css_path):
            continue
        with open(css_path, encoding="utf-8") as f:
            rules = parse_rules(f.read())
        for prelude, decls, line, ctx in rules:
            if any(c.startswith(("@keyframes", "@-webkit-keyframes")) for c in ctx):
                continue
            props = []
            for d in decls:
                if ":" not in d:
                    continue
                p = d.split(":", 1)[0].strip().lower()
                if p and not p.startswith("--"):
                    props.append(p)
            if not props:
                continue
            offset = 0
            for sel in split_top(prelude, ","):
                sel_line = line + prelude[:offset].count("\n")
                offset += len(sel) + 1
                sel = " ".join(sel.split())
                subj = strip_not_has(subject_compound(sel))
                classes = re.findall(r"\.([A-Za-z][\w-]*)", subj)
                kind = None
                hit = [c for c in classes if c in composed]
                if hit:
                    kind = "composed:" + composed[hit[0]]
                elif not classes and re.match(r"(?:%s)(?![\w-])" % "|".join(FORM_TAGS), subj):
                    kind = "tag"
                if kind is None:
                    continue
                for p in props:
                    sites.append((css_path, sel_line, kind, sel, p))
    return sites


if __name__ == "__main__":
    try:
        sites = scan()
    except Exception as e:  # a parse failure must not read as "clean"
        print(f"check-cross-component-overrides: internal error: {e!r}", file=sys.stderr)
        sys.exit(2)
    for file, line, kind, sel, prop in sites:
        print(f"{file}\t{kind}\t{sel}\t{prop}\t{line}")
PYEOF

python3 "$extractor" >"$raw"

# `file<TAB>kind<TAB>selector<TAB>property`, sorted, duplicates kept (a multiset).
current="$(cut -f1-4 "$raw" | LC_ALL=C sort)"

if [[ "$mode" == inventory ]]; then
    awk -F'\t' '{ printf "%s:%s\t%s\t%s\t%s\n", $1, $5, $2, $3, $4 }' "$raw" | LC_ALL=C sort -t: -k1,1 -k2,2n
    echo "-- $(wc -l <"$raw" | tr -d ' ') declaration site(s) in $(cut -f1,3 "$raw" | sort -u | wc -l | tr -d ' ') rule selector(s) across $(cut -f1 "$raw" | sort -u | wc -l | tr -d ' ') stylesheet(s)" >&2
    exit 0
fi

header='# DEBT REGISTER, NOT AN APPROVAL -- generated by scripts/check-cross-component-overrides.sh
# --update-baseline. Every line is a NON-custom property declared in component X'"'"'s stylesheet on
# an element that is really component Y'"'"'s, so which rule wins depends on stylesheet order or
# specificity (see the .sh header). Convert a site to a slot variable and delete its line; do not
# add one. Format: file<TAB>kind<TAB>selector<TAB>property, sorted, one line per declaration
# (a multiset). kind = composed:<Y> (X'"'"'s class on Y'"'"'s root) | tag (bare form-control element).'

baseline_body=""
if [[ -f "$baseline" ]]; then
    baseline_body="$(grep -v '^#' "$baseline" | grep -v '^[[:space:]]*$' | LC_ALL=C sort || true)"
fi

new_sites="$(LC_ALL=C comm -23 <(printf '%s\n' "$current") <(printf '%s\n' "$baseline_body") | grep -v '^$' || true)"
gone_sites="$(LC_ALL=C comm -13 <(printf '%s\n' "$current") <(printf '%s\n' "$baseline_body") | grep -v '^$' || true)"

if [[ "$mode" == update ]]; then
    if [[ -n "$new_sites" && -f "$baseline" && "$allow_grow" -ne 1 ]]; then
        echo "check-cross-component-overrides: refusing to GROW the baseline (shrink-only). New sites:" >&2
        printf '%s\n' "$new_sites" | sed 's/^/  + /' >&2
        echo "Convert them to slot variables instead, or re-run with --allow-grow and justify it in the commit." >&2
        exit 1
    fi
    { printf '%s\n' "$header"; printf '%s\n' "$current"; } >"$baseline"
    echo "check-cross-component-overrides: wrote $(printf '%s\n' "$current" | grep -c . || true) entries to $baseline" >&2
    exit 0
fi

n_cur="$(printf '%s\n' "$current" | grep -c . || true)"
if [[ -n "$new_sites" ]]; then
    echo "check-cross-component-overrides: FAILED -- a NON-custom property is declared on another component's element:" >&2
    while IFS=$'\t' read -r file kind sel prop; do
        ln="$(awk -F'\t' -v f="$file" -v k="$kind" -v s="$sel" -v p="$prop" '$1==f && $2==k && $3==s && $4==p { print $5; exit }' "$raw")"
        echo "  $file:$ln  [$kind]  $sel { $prop }" >&2
    done <<<"$new_sites"
    echo >&2
    echo "Two sheets that declare one property on one element are decided by stylesheet order (it differs per route)." >&2
    echo "Have the base component read a slot variable (\`font-size: var(--dx-label-font-size, var(--dx-text-sm))\`)" >&2
    echo "and set the VARIABLE from here (\`--dx-label-font-size: ...\`). Own raw elements get their own class." >&2
    echo "See the header of scripts/check-cross-component-overrides.sh." >&2
    exit 1
fi

if [[ -n "$gone_sites" ]]; then
    echo "check-cross-component-overrides: note -- $(printf '%s\n' "$gone_sites" | grep -c .) baseline site(s) no longer exist; lock the shrink in with --update-baseline:" >&2
    printf '%s\n' "$gone_sites" | sed 's/^/  - /' >&2
fi
echo "check-cross-component-overrides: OK ($n_cur declaration site(s), all in the baseline)"
