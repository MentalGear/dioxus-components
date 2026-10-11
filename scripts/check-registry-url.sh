#!/usr/bin/env bash
#
# check-registry-url.sh
#
# This repository is a `dx components` registry, and `dx components add <name>` reads dx's DEFAULT
# registry (upstream `DioxusLabs/components`) unless told otherwise. Two things in a manifest silently
# send an install back to upstream even when the user passed `--git <this repo>` (dioxus-cli 0.7.9,
# src/cli/component.rs and dioxus-component-manifest):
#   1. a bare-name `componentDependencies` entry (`"calendar"`) is resolved in the DEFAULT registry, so
#      `dx components add date_picker --git <us>` installed upstream's calendar/popover next to our
#      date_picker and then died with "Cannot copy global asset ... outside of the component registry".
#      Only the object form `{ "name": "calendar", "git": "<url>" }` resolves from the named repo;
#   2. a `cargoDependencies` entry for `dioxus-primitives` with another `git` URL puts upstream's crate in
#      the user's Cargo.toml, not the primitives these styled components were written against.
# dx also keys its clone cache on the exact URL text, so every spelling has to be the same string, and it is
# `REGISTRY_GIT_URL` in preview/src/main.rs (which the docs, the copy buttons and the README commands
# are built from).
#
# Rules, over preview/src/components/*/component.json:
#   - every `componentDependencies` entry is an object whose `git` is REGISTRY_GIT_URL;
#   - every `cargoDependencies` entry named dioxus-primitives or dioxus-attributes has `git` == REGISTRY_GIT_URL;
#   - the README's install commands use REGISTRY_GIT_URL;
#   - the starter template (`templates/starter`, `dx new --template ... --subtemplate templates/starter`)
#     pins the same string twice: `[components.registry] git` in its Dioxus.toml (so a new app's plain
#     `dx components add <name>` reads this registry) and the `dioxus-primitives` `git` in its
#     Cargo.toml.liquid.
#
#   - the installer CLI (`cli/`, package `shadcn-dioxus`) cannot reach preview's constant, so it keeps its own
#     `pub const DEFAULT_REGISTRY_GIT_URL: &str = "...";` in cli/src/registry.rs: it must equal REGISTRY_GIT_URL
#     (it is the registry `shadcn-dioxus add` reads when nothing else is named), and cli/README.md's
#     `cargo install --git` command must use the same string.
#
# Usage: scripts/check-registry-url.sh
# Exit status: 0 when clean, 1 on a violation, 2 when REGISTRY_GIT_URL cannot be read.

set -uo pipefail
cd "$(dirname "$0")/.."

url="$(sed -n 's/^pub(crate) const REGISTRY_GIT_URL: &str = "\(.*\)";$/\1/p' preview/src/main.rs)"
if [ -z "$url" ]; then
  echo "check-registry-url: no \`pub(crate) const REGISTRY_GIT_URL: &str = \"...\";\` in preview/src/main.rs" >&2
  exit 2
fi

fail=0
report="$(python3 -I - "$url" <<'PY'
import glob, json, sys

url = sys.argv[1]
for f in sorted(glob.glob("preview/src/components/*/component.json")):
    m = json.load(open(f))
    for d in m.get("componentDependencies") or []:
        if isinstance(d, str):
            print(f"{f}: componentDependencies entry \"{d}\" is a bare name (dx resolves it in its DEFAULT registry); write {{ \"name\": \"{d}\", \"git\": \"{url}\" }}")
        elif d.get("git") != url:
            print(f"{f}: componentDependencies entry \"{d.get('name')}\" has git {d.get('git')!r}, want {url!r}")
    for c in m.get("cargoDependencies") or []:
        if isinstance(c, dict) and c.get("name") in ("dioxus-primitives", "dioxus-attributes") and c.get("git") != url:
            print(f"{f}: cargoDependencies \"{c['name']}\" has git {c.get('git')!r}, want {url!r}")
PY
)"
if [ -n "$report" ]; then
  echo "check-registry-url: component.json files out of step with REGISTRY_GIT_URL ($url):" >&2
  echo "$report" >&2
  fail=1
fi

if ! grep -q -- "dx components add .* --git $url" README.md; then
  echo "check-registry-url: README.md has no \`dx components add <name> --git $url\` command" >&2
  fail=1
fi

tpl=templates/starter
tpl_dx="$tpl/Dioxus.toml"
tpl_cargo="$tpl/Cargo.toml.liquid"
if [ ! -f "$tpl_dx" ] || [ ! -f "$tpl_cargo" ]; then
  echo "check-registry-url: $tpl_dx or $tpl_cargo is missing" >&2
  fail=1
else
  tpl_report="$(python3 -I - "$url" "$tpl_dx" "$tpl_cargo" <<'PY'
import re, sys

url, dx_path, cargo_path = sys.argv[1:4]
dx = open(dx_path).read()
m = re.search(r'^\[components\.registry\]\s*\n(?:[^\[\n]*\n)*?\s*git\s*=\s*"([^"]*)"', dx, re.M)
if not m:
    print(f"{dx_path}: no `[components.registry]` table with a `git = \"...\"` key")
elif m.group(1) != url:
    print(f"{dx_path}: [components.registry] git is {m.group(1)!r}, want {url!r}")
cargo = open(cargo_path).read()
m = re.search(r'^dioxus-primitives\s*=\s*\{[^}]*\bgit\s*=\s*"([^"]*)"', cargo, re.M)
if not m:
    print(f"{cargo_path}: no `dioxus-primitives = {{ git = \"...\" }}` dependency")
elif m.group(1) != url:
    print(f"{cargo_path}: dioxus-primitives git is {m.group(1)!r}, want {url!r}")
PY
)"
  if [ -n "$tpl_report" ]; then
    echo "check-registry-url: starter template out of step with REGISTRY_GIT_URL ($url):" >&2
    echo "$tpl_report" >&2
    fail=1
  fi
fi

if ! grep -q -- "dx new .* --template $url --subtemplate templates/starter" README.md; then
  echo "check-registry-url: README.md has no \`dx new <name> --template $url --subtemplate templates/starter\` command" >&2
  fail=1
fi

cli_url="$(sed -n 's/^pub const DEFAULT_REGISTRY_GIT_URL: &str = "\(.*\)";$/\1/p' cli/src/registry.rs)"
if [ -z "$cli_url" ]; then
  echo "check-registry-url: no \`pub const DEFAULT_REGISTRY_GIT_URL: &str = \"...\";\` in cli/src/registry.rs" >&2
  fail=1
elif [ "$cli_url" != "$url" ]; then
  echo "check-registry-url: cli/src/registry.rs DEFAULT_REGISTRY_GIT_URL is $cli_url, want $url" >&2
  fail=1
fi
if ! grep -q -- "cargo install --git $url shadcn-dioxus" cli/README.md; then
  echo "check-registry-url: cli/README.md has no \`cargo install --git $url shadcn-dioxus\` command" >&2
  fail=1
fi

[ "$fail" -ne 0 ] && exit 1
echo "check-registry-url: OK -- every component.json dependency, the starter template and the installer CLI name $url."
