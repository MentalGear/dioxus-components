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
dx build --platform web --release --ssg --features fullstack --base-path dioxus-components

if [ ! -d "$public_dir" ]; then
  echo "error: expected build output at $public_dir, not found" >&2
  exit 1
fi

# `dx build --ssg` has a known, pre-existing race (unrelated to any one
# source change -- reproduced identically on an untouched checkout, see
# session notes) where its SSG render pass fires all routes' requests at
# the freshly-started fullstack server concurrently, and on a build fast
# enough (an already-warm cargo/dx cache -- this repeated-build sandbox's
# normal state, not just a one-off) some of those requests can land before
# something the server needs to inject the hydration bootstrap
# `<script type="module">` is ready. The affected route's static HTML then
# has no script tag at all -- not a visible error, just a dead,
# never-hydrates page if it ships. It reliably self-heals on a build that
# actually has to recompile (a cold cache), but this script's own `rm -rf
# "$public_dir"` above only clears dx's *output* directory, not that
# compile cache, so a run here can still land on the fast/racy side. Catch
# it here, on every route dx actually produced, rather than trust that.
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
  echo "This is the dx/dioxus-fullstack SSG cache-warm race described above, not a" >&2
  echo "config problem with this script. It reliably clears on a build that has to" >&2
  echo "really recompile: run 'rm -rf \"$repo_root/target/dx/preview\"' (or 'cargo" >&2
  echo "clean' for a full reset) and re-run this script." >&2
  exit 1
fi
echo "==> OK: all $route_count route(s) have their bootstrap script."

echo "==> Refreshing $repo_root/docs from $public_dir ..."
mkdir -p "$repo_root/docs"
find "$repo_root/docs" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
cp -r "$public_dir"/. "$repo_root/docs/"
touch "$repo_root/docs/.nojekyll"

echo "==> Done. Review with 'git status'/'git diff --stat -- docs', then commit and push to main."
