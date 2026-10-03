#!/usr/bin/env bash
#
# check-internal-hrefs.sh
#
# Keeps internal links on the docs site from escaping the deploy base path.
# The Pages build is served below `/dioxus-components/` (`dx --base-path`), so
# a root-absolute link that does not go through the router 404s there while
# working on a dev server mounted at `/`. It hit twice -- the build-time
# rendered docs.md HTML (now prefixed by `prefix_internal_hrefs` in
# preview/src/main.rs) and the NavigationMenu demo, whose links render as plain
# `<a>`, not a router `Link` -- so this makes the class unable to recur:
#
#   1. No root-absolute string literal as an `href:` attribute in
#      preview/src/**/*.rs (`href: "/docs"`). Build the value from the router
#      prefix (`router().prefix()`) or use a `Link { to: Route::... }`.
#      (docs.md code samples are exempt: they show users' code, not ours.)
#   2. No legacy `/component/?name=<x>` link anywhere in preview/src -- docs.md
#      or .rs code. It is only a client-side redirect shell; the canonical
#      route is `/component/<x>/`. Doc comments (`//`, `///`, `//!`) that
#      merely describe the legacy form are fine.
#
# Usage: scripts/check-internal-hrefs.sh
# Exit status: 0 when clean.

set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

hits=$(grep -rnE 'href:[[:space:]]*(format!\()?"/' preview/src --include='*.rs' || true)
if [ -n "$hits" ]; then
  echo "check-internal-hrefs: root-absolute href literal(s) in preview/src (escape the deploy base path):" >&2
  echo "$hits" >&2
  fail=1
fi

legacy=$(grep -rnF '/component/?name=' preview/src --include='*.md' --include='*.rs' \
  | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' || true)
if [ -n "$legacy" ]; then
  echo "check-internal-hrefs: legacy /component/?name= link(s) -- use /component/<name>/:" >&2
  echo "$legacy" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "check-internal-hrefs: OK -- no root-absolute href literals or legacy /component/?name= links."
