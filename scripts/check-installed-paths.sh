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
