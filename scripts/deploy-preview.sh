#!/usr/bin/env bash
# Builds the preview/gallery app and publishes it into /docs for GitHub
# Pages, which is configured (repo Settings -> Pages) to serve straight
# from the `main` branch's /docs folder -- no CI build step involved.
#
# Usage: scripts/deploy-preview.sh
# Then: git add docs && git commit -m "..." && git push (to main).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root/preview"

public_dir="$repo_root/target/dx/preview/release/web/public"

# dx's asset pipeline content-hashes the compiled wasm/js on every build but
# never cleans the previous run's copies out of $public_dir, so repeated
# builds accumulate multiple stale preview_bg-*.wasm/preview-*.js -- wipe it
# first so only the build we're about to run ends up in /docs.
rm -rf "$public_dir"

echo "==> Building preview (release, ssg, fullstack) ..."
dx build --platform web --release --ssg --features fullstack --base-path dioxus-components --force-sequential=true

if [ ! -d "$public_dir" ]; then
  echo "error: expected build output at $public_dir, not found" >&2
  exit 1
fi

# `dx build --ssg` has a known, pre-existing race (unrelated to any one
# source change -- reproduced identically on an untouched checkout, see
# session notes) where its SSG render pass fires all routes' requests at
# the freshly-started fullstack server concurrently, and some of those
# requests can land before something the server needs to inject the
# hydration bootstrap `<script type="module">` is ready. The affected
# route's static HTML then has no script tag at all -- not a visible
# error, just a dead, never-hydrates page if it ships.
#
# Earlier note in this file claimed a cold cargo cache "reliably self-heals"
# this -- that was wrong, disproved by direct testing (2026-09-18 docs
# deploy session): two independent fully-cold rebuilds (all of
# target/wasm-release, target/wasm32-unknown-unknown/wasm-release,
# target/server-release, target/x86_64-*/server-release, and
# target/dx/preview/release cleared first -- confirmed by mtime, not just
# assumed) reproduced the *identical* 5-of-6 failure, at near-identical
# internal timing both times. Compile-cache warmth was never the variable.
# The actual cause: by default `dx build --fullstack` compiles the server
# and client targets in parallel, and the client side's post-compile work
# (wasm-bindgen, wasm-opt -- both CPU-heavy) is still competing for CPU
# when the SSG pass's concurrent requests hit the freshly-listening server,
# starving whatever it needs to finish before it can inject the script tag.
# On this sandbox's 4 cores, `--force-sequential=true` above (server build,
# then client build, so nothing competes with the server during its SSG
# window) fixed it 2/2 clean runs after 2/2 failures without it -- dx's own
# `--help` already default-enables this under `CI=1` for what's presumably
# this same reason. The verification below stays as a safety net regardless
# (belt-and-suspenders: don't ship a dead page even if this analysis is
# ever wrong for some other environment), on every route dx actually
# produced, rather than trust that the flag always suffices.
echo "==> Verifying every route's build output includes its hydration bootstrap script ..."
missing_script=()
route_count=0
while IFS= read -r -d '' html_file; do
  route_count=$((route_count + 1))
  if ! grep -q '<script type="module"' "$html_file"; then
    missing_script+=("${html_file#"$public_dir"/}")
  fi
done < <(find "$public_dir" -name "index.html" -print0)

if [ "${#missing_script[@]}" -gt 0 ]; then
  echo "error: ${#missing_script[@]} of $route_count route(s) built WITHOUT their wasm" >&2
  echo "bootstrap <script type=\"module\"> tag -- they would ship as dead, never-" >&2
  echo "hydrating pages:" >&2
  printf '  - %s\n' "${missing_script[@]}" >&2
  echo "This looks like the dx/dioxus-fullstack SSG server/client-build race" >&2
  echo "described above -- but this script already passes --force-sequential=true" >&2
  echo "specifically to avoid it, so seeing this means that mitigation didn't hold" >&2
  echo "(different environment, dx version, core count, ...). A cold cache alone" >&2
  echo "did NOT fix this when it was tested (see the comment above) -- diagnose" >&2
  echo "fresh from this build's own log rather than assume that remedy." >&2
  exit 1
fi
echo "==> OK: all $route_count route(s) have their bootstrap script."

# dev-docs/backlog.md row 46: before the path-segment route construction
# (preview/src/main.rs's ComponentDemoPath/ComponentBlockDemoPath), every
# `/component/?name=X&` page prerendered as the "Component not found" shell
# for every X, because a query-string route is not enumerable at build time
# -- this check is the safety net for a recurrence of that class, on every
# route dx actually produced, the same belt-and-suspenders shape as the
# bootstrap-script check above. A well-formed build should never ship this
# marker at all: `ComponentDemoPath`'s own not-found branch only renders for
# a name absent from `components::DEMOS`, and `server_static_routes`
# (preview/src/main.rs) only ever asks dx to prerender names that ARE in
# that list; the legacy query-string routes (`ComponentDemo`/
# `ComponentBlockDemo`) render a differently-classed loading/redirect shell
# (`dx-component-demo-redirect`), not this marker.
echo "==> Verifying no route's build output is the \"Component not found\" shell ..."
not_found_routes=()
while IFS= read -r -d '' html_file; do
  if grep -q 'dx-component-demo-not-found' "$html_file"; then
    not_found_routes+=("${html_file#"$public_dir"/}")
  fi
done < <(find "$public_dir" -name "index.html" -print0)

if [ "${#not_found_routes[@]}" -gt 0 ]; then
  echo "error: ${#not_found_routes[@]} of $route_count route(s) built as the" >&2
  echo "\"Component not found\" shell (dx-component-demo-not-found) instead of" >&2
  echo "their real content -- this is dev-docs/backlog.md row 46's regression:" >&2
  printf '  - %s\n' "${not_found_routes[@]}" >&2
  echo "Check preview/src/main.rs's server_static_routes() actually lists this" >&2
  echo "route's name, and that the name matches a components::DEMOS entry." >&2
  exit 1
fi
echo "==> OK: no route shipped the not-found shell."

echo "==> Refreshing $repo_root/docs from $public_dir ..."
mkdir -p "$repo_root/docs"
find "$repo_root/docs" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
cp -r "$public_dir"/. "$repo_root/docs/"
touch "$repo_root/docs/.nojekyll"

echo "==> Done. Review with 'git status'/'git diff --stat -- docs', then commit and push to main."
