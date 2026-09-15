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

echo "==> Refreshing $repo_root/docs from $public_dir ..."
mkdir -p "$repo_root/docs"
find "$repo_root/docs" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
cp -r "$public_dir"/. "$repo_root/docs/"
touch "$repo_root/docs/.nojekyll"

echo "==> Done. Review with 'git status'/'git diff --stat -- docs', then commit and push to main."
