#!/usr/bin/env bash
#
# check-component-catalog.sh
#
# Two manifest-consistency rules over preview/src/components/<dir>/component.json.
#
# 1. EVERY COMPONENT IS CLASSIFIED (shadcn or extra).
#    `CATALOG` in preview/src/components/mod.rs is the one list saying whether a
#    component directory is a page of shadcn/ui's catalog (`Origin::Shadcn`) or
#    one of our additions (`Origin::Extra`, which renders the "Extra" badge on
#    the component page, the sidebar entry and the homepage card). The list
#    carries BOTH kinds on purpose: a list of extras alone makes "not decided
#    yet" look identical to "shadcn", so a new component would silently skip the
#    badge. Here a new directory with a component.json and no row fails, and the
#    author has to decide. Also fails on: a row with no directory (stale), two
#    rows for one directory, and a row that is not the strict
#    `("name", Origin::Shadcn|Extra),` shape (a typo such as `Origin::Extras`
#    would otherwise be skipped by the extraction and look like a missing row).
#    The same totality is asserted from the compiled side by the
#    `catalog_classifies_every_demo` test, which also covers `examples!`.
#
# 2. EVERY STYLED COMPONENT LISTS THE THEME (dev-docs/backlog.md row 129).
#    A component directory with a style.css must name
#    `../../../assets/dx-components-theme.css` in its component.json
#    `globalAssets`, else `dx components add <name>` ships `.dx-<name>` rules
#    without the role tokens, the scoped box-sizing rule or the radius scale.
#    navbar was the one component that lacked it.
#
# 3. EVERY SIBLING COMPONENT A COMPONENT IMPORTS IS A DECLARED DEPENDENCY.
#    `dx components add <name>` installs the component's own files plus the
#    `componentDependencies` of its component.json, and nothing else. A
#    component's installed files (everything outside `variants/`, which is
#    repo-only) that reach `crate::components::<other>` must therefore list
#    `<other>`, else the installed tree does not compile (`form` missed four,
#    `top_layer` one, until 2026-10-05). Comments and string literals are
#    ignored; `crate::components::{a, b::{X}}` groups are read. The reverse
#    (a dependency declared but never imported) installs unused code but
#    compiles, so it is only a note.
#
# Usage: scripts/check-component-catalog.sh
# Exit status: 0 when clean, 1 on a violation.

set -uo pipefail
cd "$(dirname "$0")/.."

mod=preview/src/components/mod.rs
comp_root=preview/src/components
theme='../../../assets/dx-components-theme.css'
fail=0
complain() { echo "check-component-catalog: $*" >&2; fail=1; }

[ -f "$mod" ] || { echo "check-component-catalog: $mod not found" >&2; exit 2; }

# --- the CATALOG block: lines between `pub const CATALOG` and its closing `];` ----------------
block="$(awk '/^pub const CATALOG/ { in_block = 1; next } in_block && /^\];/ { exit } in_block' "$mod")"
if [ -z "$block" ]; then
  echo "check-component-catalog: no \`pub const CATALOG\` block found in $mod" >&2
  exit 2
fi

row_re='^[[:space:]]*\("[a-z0-9_]+", Origin::(Shadcn|Extra)\),[[:space:]]*(//.*)?$'
while IFS= read -r line; do
  trimmed="${line#"${line%%[![:space:]]*}"}"
  [ -z "$trimmed" ] && continue
  case "$trimmed" in //*) continue ;; esac
  if ! [[ "$line" =~ $row_re ]]; then
    complain "malformed CATALOG row in $mod (want \`(\"name\", Origin::Shadcn|Extra),\`): $line"
  fi
done <<< "$block"

catalog_names="$(printf '%s\n' "$block" | grep -oE '^[[:space:]]*\("[a-z0-9_]+"' | grep -oE '"[a-z0-9_]+"' | tr -d '"' | sort)"

dir_names="$(for f in "$comp_root"/*/component.json; do basename "$(dirname "$f")"; done | sort)"

dups="$(printf '%s\n' "$catalog_names" | uniq -d)"
for n in $dups; do complain "CATALOG lists \`$n\` more than once"; done

unclassified="$(comm -23 <(printf '%s\n' "$dir_names") <(printf '%s\n' "$catalog_names" | uniq))"
for n in $unclassified; do
  complain "unclassified component \`$n\`: add \`(\"$n\", Origin::Shadcn)\` (a shadcn/ui catalog page) or \`(\"$n\", Origin::Extra)\` (our addition, gets the \"Extra\" badge) to CATALOG in $mod"
done

stale="$(comm -13 <(printf '%s\n' "$dir_names") <(printf '%s\n' "$catalog_names" | uniq))"
for n in $stale; do
  complain "CATALOG row \`$n\` has no $comp_root/$n/component.json (renamed or removed?)"
done

# --- rule 2: styled components list the theme (row 129) ---------------------------------------
for f in "$comp_root"/*/component.json; do
  d="$(dirname "$f")"
  [ -f "$d/style.css" ] || continue
  if ! grep -q "\"$theme\"" "$f"; then
    complain "$f has a style.css but no \"globalAssets\": [\"$theme\"] (row 129: \`dx components add\` would ship the component without the theme)"
  fi
done

# --- rule 3: componentDependencies cover every imported sibling component ----------------------
deps_report="$(python3 - "$comp_root" <<'PY'
import glob, json, os, re, sys

root = sys.argv[1]


def strip(src):
    """Drop // and /* */ comments and string literals (raw ones too); keep everything else."""
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif c == "r" and re.match(r'r#*"', src[i:i + 8]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            m = re.match(r'r(#*)"', src[i:])
            end = '"' + m.group(1)
            j = src.find(end, i + m.end())
            i = n if j < 0 else j + len(end)
        elif c == '"':
            i += 1
            while i < n and src[i] != '"':
                i += 2 if src[i] == "\\" else 1
            i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def refs(src):
    found = set()
    for m in re.finditer(r"crate::components::", src):
        rest = src[m.end():]
        if rest.startswith("{"):
            depth, start, parts = 0, 1, []
            for j, ch in enumerate(rest):
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        parts.append(rest[start:j])
                        break
                elif ch == "," and depth == 1:
                    parts.append(rest[start:j])
                    start = j + 1
            for part in parts:
                mm = re.match(r"\s*([a-z_0-9]+)", part)
                if mm:
                    found.add(mm.group(1))
        else:
            mm = re.match(r"([a-z_0-9]+)", rest)
            if mm:
                found.add(mm.group(1))
    return found


for f in sorted(glob.glob(f"{root}/*/component.json")):
    d = os.path.dirname(f)
    name = os.path.basename(d)
    # An entry is a bare name (the CLI resolves it in dx's DEFAULT registry, upstream) or a
    # {"name", "git"} object (resolved from `git`); check-registry-url.sh requires the object form.
    declared = {d if isinstance(d, str) else d["name"] for d in json.load(open(f)).get("componentDependencies") or []}
    used = set()
    for rs in glob.glob(f"{d}/**/*.rs", recursive=True):
        if os.path.relpath(rs, d).split(os.sep)[0] == "variants":
            continue
        used |= refs(strip(open(rs).read()))
    used.discard(name)
    if used - declared:
        print(f"MISSING {name} {' '.join(sorted(used - declared))}")
    if declared - used:
        print(f"UNUSED {name} {' '.join(sorted(declared - used))}")
PY
)"
unused_deps=0
while read -r kind name rest; do
  case "$kind" in
    MISSING) complain "$comp_root/$name imports crate::components::{${rest// /, }} but its component.json \"componentDependencies\" does not list them: \`dx components add $name\` would install a tree that does not compile" ;;
    UNUSED) unused_deps=$((unused_deps + 1)) ;;
  esac
done <<< "$deps_report"

if [ "$fail" -ne 0 ]; then
  exit 1
fi
[ "$unused_deps" -gt 0 ] && echo "check-component-catalog: note -- $unused_deps component(s) declare a dependency their own source never imports (installs unused code; compiles)."
echo "check-component-catalog: OK -- $(printf '%s\n' "$dir_names" | wc -l | tr -d ' ') components classified, $(printf '%s\n' "$block" | grep -c 'Origin::Extra') extra, every styled component lists the theme, every imported sibling is a declared dependency."
