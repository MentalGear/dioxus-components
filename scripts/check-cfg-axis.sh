#!/usr/bin/env bash
#
# check-cfg-axis.sh
#
# Enforces the corrected cfg axis documented in
# docs/recommended-implementations.md, Caveat 1 (2026-09-01 update):
#
#   Rendered markup, component structure, and attribute choice must split
#   on the `web` Cargo feature (`#[cfg(feature = "web")]` /
#   `#[cfg(not(feature = "web"))]`), never on `target_family = "wasm"`.
#   `web` is a *renderer* question -- true for both the wasm browser client
#   AND a host (non-wasm) fullstack SSG server build -- while
#   `target_family = "wasm"` is an *execution-target* question that a host
#   SSG server build fails.
#
# The 2026-09-01 production incident this guards against: primitives split
# markup on `target_family = "wasm"`, so the fullstack SSG prerender (a host
# binary, not wasm) rendered the plain native arm -- no `popover` attribute,
# no `<dialog>` elements -- while the wasm client then hydrated against that
# structurally different markup, breaking events page-wide on every
# hard-loaded page of the deployed site. `playwright/oracle/
# hydration-parity.spec.ts` is the regression oracle for the symptom;
# this script is the regression guard for the cfg pattern that caused it.
#
# `target_family = "wasm"` remains legitimate ONLY for genuinely
# wasm-only *execution* internals -- and none exist in this crate today:
# `document::eval` itself is a cross-renderer Dioxus API that compiles and
# runs (inertly, off a real browser/webview) on every target, so every
# leaf hook that used to gate on `target_family = "wasm"`
# (`use_popover_sync`, `use_dialog_open_driver`, etc., in `top_layer.rs`/
# `lib.rs`) now gates on `feature = "web"` too, verified by `cargo check -p
# dioxus-primitives --features web` (host) and `--target
# wasm32-unknown-unknown` both succeeding. Should a genuinely wasm-only
# execution primitive ever get added, gate it inside a hook's *body*, one
# level under an outer `#[cfg(feature = "web")]` -- never at module/
# component granularity -- and add it to the allowlist below rather than
# reintroducing a markup-level `target_family` split.
#
# Usage: scripts/check-cfg-axis.sh
# Exit 0: clean. Exit 1: a `target_family` cfg predicate was found outside
# the allowlist, with file:line detail on stderr.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

SRC_DIR="primitives/src"

# Files allowed to contain a `target_family = "wasm"` cfg predicate, should
# one ever legitimately need to reappear (see the module doc above). Empty
# today: the axis fix removed every occurrence, including from these leaf
# execution modules -- `document::eval` needed no inner wasm-only guard.
# Kept as a real (if currently-empty) allowlist, not a hardcoded zero,
# so a single narrowly-scoped exception doesn't require rewriting this
# script's logic, only adding a path here.
ALLOWLIST=(
  # "primitives/src/top_layer.rs"
  # "primitives/src/lib.rs"
  # "primitives/src/scroll_lock.rs"
)

is_allowlisted() {
  local path="$1"
  for allowed in "${ALLOWLIST[@]+"${ALLOWLIST[@]}"}"; do
    if [[ "$path" == "$allowed" ]]; then
      return 0
    fi
  done
  return 1
}

# Match only real cfg predicates -- `#[cfg(target_family = "wasm")]`,
# `#[cfg(not(target_family = "wasm"))]`, and the runtime `cfg!(target_family
# = "wasm")` macro form -- never a doc-comment or string mention of the
# phrase `target_family = "wasm"` on its own (several files legitimately
# narrate the old, now-corrected axis in prose for historical context).
MATCHES="$(grep -rnE 'cfg!?\(.*target_family' "$SRC_DIR" || true)"

VIOLATIONS=""
if [[ -n "$MATCHES" ]]; then
  while IFS= read -r line; do
    file="${line%%:*}"
    if ! is_allowlisted "$file"; then
      VIOLATIONS+="$line"$'\n'
    fi
  done <<< "$MATCHES"
fi

if [[ -n "$VIOLATIONS" ]]; then
  echo "check-cfg-axis: found target_family cfg predicate(s) outside the allowlist:" >&2
  echo "$VIOLATIONS" >&2
  echo >&2
  echo "Markup/structure/attribute cfgs must split on feature = \"web\", not" >&2
  echo "target_family = \"wasm\" -- see docs/recommended-implementations.md" >&2
  echo "Caveat 1 for the corrected axis rule and the 2026-09-01 incident" >&2
  echo "that proved it." >&2
  exit 1
fi

echo "check-cfg-axis: OK -- no target_family cfg predicate outside the allowlist."
