#!/usr/bin/env bash
#
# check-installed-paths.sh
#
# Keeps the preview's component and variant sources compiling in a USER's
# tree, not only in this repo. `dx components add <name>` installs a component
# as `src/components/<name>/mod.rs` = `mod component; pub use component::*;`,
# so `component` is a PRIVATE module there and the only public path to a
# component's items is `crate::components::<name>::Item` (or `::*`). Inside
# this repo `examples!` (preview/src/components/mod.rs) keeps `component`
# private for the same reason, so such a path fails to compile here too -- but
# it hit twice before that was true (the `pub use
# crate::components::chart::component::*;` in every *_chart/component.rs, and
# the `use crate::components::button::component::Button;` in six variants),
# each compiling fine in the repo and failing in an installed tree.
#
# Rule: no `components::<name>::component::` path anywhere in
# preview/src/**/*.rs outside comments (the app shell in main.rs may name
# `components::<name>::variants`: that module is repo-only). A component reaches a
# sibling as `crate::components::<name>::*`; a variant reaches its own
# component as `super::super::component::*` (installed_source.rs rewrites that
# for the Copy/View Code tabs).
#
# Second half (install completeness). dioxus-cli 0.7.9 (`src/cli/component.rs`, `add_component` /
# `copy_component_files`) installs a component by copying its WHOLE directory minus the manifest's
# `exclude` list (each entry is canonicalized, so one that does not exist fails the install) to
# `<crate>/src/components/<name>/`, copying every `globalAssets` file into `assets/`, installing every
# `componentDependencies` entry the same way, and appending `pub mod <name>;` to the user's
# `src/components/mod.rs`. There is no `files` field. So a component installs and compiles only if its
# directory itself holds a `mod.rs` that declares `component` (preview's `examples!` declares the module
# inline and never reads that file, so a missing one compiled fine here and broke with E0583 in a user's
# tree for 27 components). For every `preview/src/components/*/component.json` this checks:
#   - the dir is a member of the root `component.json` and the manifest `name` is the dir name (a dir that
#     is deliberately not installable goes in NOT_INSTALLABLE below, with the reason);
#   - `mod.rs` exists and is exactly `mod component; pub use component::*;`, `component.rs` exists;
#   - every `exclude` and `globalAssets` entry exists, and no `exclude` covers `mod.rs`/`component.rs`/`style.css`;
#   - every file `component.rs` needs from its own dir survives the copy: `mod x;`, `#[path = ".."]`,
#     `asset!("/src/components/<name>/..")`, `include_str!("..")`;
#   - every `crate::components::<other>::` it names is in its `componentDependencies` closure, so
#     `dx components add <name>` alone brings it in.
# `scripts/registry-install-smoke.sh` is the definitive (slow, pre-merge) check that this approximates.
#
# Usage: scripts/check-installed-paths.sh
# Exit status: 0 when clean.

set -uo pipefail
cd "$(dirname "$0")/.."

hits=$(grep -rnE 'components::[a-z_0-9]+::component::' preview/src --include='*.rs' \
  | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
  | grep -v "^preview/src/installed_source.rs:" || true)
if [ -n "$hits" ]; then
  echo "check-installed-paths: path(s) through a component's private \`component\` module (does not compile after \`dx components add\`):" >&2
  echo "$hits" >&2
  echo "Use crate::components::<name>::* (or ::Item) instead." >&2
  exit 1
fi
echo "check-installed-paths: OK -- no components::<name>::component:: paths in preview/src."

python3 -I - <<'PY' || exit 1
import glob, json, os, re, sys

# dir -> why it has a component.json but is not an installable registry member.
NOT_INSTALLABLE = {
    "top_layer": "oracle fixture for playwright/oracle/tier2-html/top-layer.spec.ts, not a component",
}
root = "preview/src/components"
members = {m.rsplit("/", 1)[-1] for m in json.load(open("component.json"))["members"]}
dirs = {os.path.basename(os.path.dirname(f)): f for f in glob.glob(f"{root}/*/component.json")}
problems = []

for name in sorted(members - dirs.keys()):
    problems.append(f"root component.json lists member '{name}' but {root}/{name}/component.json does not exist")
for name in sorted(dirs.keys() - members):
    if name not in NOT_INSTALLABLE:
        problems.append(f"{root}/{name}/component.json exists but '{name}' is not a member of the root component.json (dx cannot find it); add it, or list it in NOT_INSTALLABLE")
for name in sorted(NOT_INSTALLABLE.keys() & members):
    problems.append(f"'{name}' is in NOT_INSTALLABLE but is also a root component.json member")

manifests = {n: json.load(open(f)) for n, f in dirs.items()}

def deps(name, seen=None):
    seen = set() if seen is None else seen
    for d in manifests.get(name, {}).get("componentDependencies") or []:
        dn = d if isinstance(d, str) else d.get("name")
        if dn not in seen:
            seen.add(dn)
            deps(dn, seen)
    return seen

for name in sorted(dirs.keys() & members):
    d = f"{root}/{name}"
    m = manifests[name]
    if m.get("name") != name:
        problems.append(f"{d}/component.json: name is {m.get('name')!r}, want the directory name {name!r}")
    for dn in sorted(deps(name)):
        if dn not in members:
            problems.append(f"{d}/component.json: componentDependencies names '{dn}', which is not a root member")
    excluded = [os.path.normpath(e) for e in (m.get("exclude") or [])]
    for e in excluded:
        if not os.path.exists(os.path.join(d, e)):
            problems.append(f"{d}/component.json: exclude entry '{e}' does not exist (dx canonicalizes it and fails the whole install)")
    for a in m.get("globalAssets") or []:
        if not os.path.exists(os.path.join(d, a)):
            problems.append(f"{d}/component.json: globalAssets entry '{a}' does not exist")

    def survives(rel):
        rel = os.path.normpath(rel)
        return not any(rel == e or rel.startswith(e + os.sep) for e in excluded)

    for need in ("mod.rs", "component.rs", "style.css"):
        if not os.path.isfile(f"{d}/{need}"):
            problems.append(f"{d}/{need} is missing (dx components add {name} would not install it)" + (" -> E0583 `file not found for module`" if need == "mod.rs" else ""))
        elif not survives(need):
            problems.append(f"{d}/component.json: exclude covers {need}, which an install needs")
    if os.path.isfile(f"{d}/mod.rs"):
        body = re.sub(r"\s+", " ", re.sub(r"//[^\n]*", "", open(f"{d}/mod.rs").read())).strip()
        if body != "mod component; pub use component::*;":
            problems.append(f"{d}/mod.rs must be exactly `mod component; pub use component::*;` (it declares what component.rs exports), got: {body!r}")
    if not os.path.isfile(f"{d}/component.rs"):
        continue
    src = "\n".join(l for l in open(f"{d}/component.rs").read().splitlines() if not l.lstrip().startswith("//"))
    for mm in re.finditer(r'(?:#\[path\s*=\s*"([^"]+)"\]\s*)?^\s*(?:pub(?:\([a-z]+\))?\s+)?mod\s+([a-z_0-9]+)\s*;', src, re.M):
        rel = mm.group(1) or mm.group(2) + ".rs"
        if not os.path.isfile(f"{d}/{rel}"):
            problems.append(f"{d}/component.rs declares `mod {mm.group(2)};` but {d}/{rel} does not exist")
        elif not survives(rel):
            problems.append(f"{d}/component.rs declares `mod {mm.group(2)};` but exclude drops {rel}")
    for a in set(re.findall(r'asset!\(\s*"([^"]+)"', src)):
        if a.startswith("/src/components/"):
            rel = a[len("/src/components/"):]
            if not os.path.isfile(f"{root}/{rel}"):
                problems.append(f"{d}/component.rs: asset!(\"{a}\") does not exist")
            elif rel.split("/", 1)[0] not in deps(name) | {name}:
                problems.append(f"{d}/component.rs: asset!(\"{a}\") points into a component that is not in its componentDependencies closure")
            elif rel.split("/", 1)[0] == name and not survives(rel.split("/", 1)[1]):
                problems.append(f"{d}/component.rs: asset!(\"{a}\") is dropped by exclude")
    for a in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"', src):
        if not os.path.isfile(os.path.join(d, a)):
            problems.append(f"{d}/component.rs: include_str!(\"{a}\") does not exist relative to the component directory")
        elif not survives(a):
            problems.append(f"{d}/component.rs: include_str!(\"{a}\") is dropped by exclude")
    if re.search(r"\bvariants\b", re.sub(r'"[^"]*"', "", src)):
        problems.append(f"{d}/component.rs names `variants`, which exclude drops from an install")
    allowed = deps(name) | {name}
    for other in sorted(set(re.findall(r"crate::components::([a-z_0-9]+)::", src)) - allowed):
        problems.append(f"{d}/component.rs uses crate::components::{other}::, but '{other}' is not in its componentDependencies closure (dx components add {name} would not bring it)")

if problems:
    print("check-installed-paths: components that would not install and compile with `dx components add`:", file=sys.stderr)
    for p in problems:
        print("  " + p, file=sys.stderr)
    sys.exit(1)
print(f"check-installed-paths: OK -- {len(members)} components each carry mod.rs + component.rs + style.css, with exclude/assets/dependencies consistent.")
PY
