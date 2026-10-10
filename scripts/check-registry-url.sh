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
#   - the README's install commands use REGISTRY_GIT_URL.
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

[ "$fail" -ne 0 ] && exit 1
echo "check-registry-url: OK -- every component.json dependency names $url."
