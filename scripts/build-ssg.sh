#!/usr/bin/env bash
# The ONE way to run the preview's `dx build --ssg` (backlog row 116; rows 98/100).
# Used by scripts/deploy-preview.sh and by every verification lane.
#
#   CARGO_TARGET_DIR=/abs/isolated/dir scripts/build-ssg.sh [debug|release] [--base-path P]
#
# Default profile: debug (the cheap one; deploy-preview.sh passes `release --base-path
# dioxus-components`). On success prints, as its last two lines,
#   PUBLIC_DIR=<$CARGO_TARGET_DIR/dx/preview/<profile>/web/public>
#   STALE_HTML=0
# and exits non-zero (with a reason on stderr) otherwise.
#
# Guards -- each one closes a failure class by construction instead of relying on the caller:
#  1. CARGO_TARGET_DIR must be set and ABSOLUTE. dx 0.7.9's `--ssg` pass chdirs before exec'ing
#     the still-relative server binary and dies with an opaque "No such file or directory
#     (os error 2)" (row 98); and an unset value means the shared repo `target/` (row 100).
#  2. A no-base-path build refuses a tree carrying the `.base-path-build` marker: such a tree
#     keeps emitting `/dioxus-components/` URLs even after `rm -rf public` (row 100 -- the
#     tree is the problem, not its output). The marker is written HERE, before any
#     `--base-path` build; scripts/lane-target.sh refuses marked trees as lane templates.
#     The vice-versa direction (base-path build into an unmarked tree) is NOT refused: no
#     evidence of harm was ever recorded, the marker cannot detect it, and refusing would block
#     the normal first deploy into a warm tree. What is real there is a wrong *output*, so both
#     directions are checked on the output instead (step 5).
#  3. The profile's `dx/preview/<profile>/web/public` is wiped before building. dx 0.7.9 `--ssg`
#     only rewrites `public/index.html` when `public/` already exists; every other pre-rendered
#     route keeps the previous build's HTML (stale theme/main CSS hashes -- false "border-box
#     regressions"). It also never cleans old content-hashed wasm/js out of `public/`.
#  4. Freshness: every `public/**/index.html` must be newer than a marker taken just before dx
#     starts (stale ones are counted and fail the build; zero pages fails too).
#  5. Output matches the requested base path: with `--base-path P` the root page must reference
#     `/P/...`; without it NO page may reference `/dioxus-components/...` (same pattern as
#     scripts/lane-target.sh's contamination grep).
set -euo pipefail

profile=debug
base_path=""
while [ $# -gt 0 ]; do
  case "$1" in
    debug|release) profile="$1"; shift ;;
    --base-path) base_path="${2:?--base-path needs a value}"; shift 2 ;;
    --base-path=*) base_path="${1#--base-path=}"; shift ;;
    *) echo "usage: CARGO_TARGET_DIR=/abs/dir $0 [debug|release] [--base-path P]" >&2; exit 2 ;;
  esac
done
base_path="${base_path#/}"; base_path="${base_path%/}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# --- Guard 1: absolute, explicit CARGO_TARGET_DIR ---------------------------------------
if [ -z "${CARGO_TARGET_DIR:-}" ]; then
  echo "error: CARGO_TARGET_DIR is not set. Every SSG build needs an explicit, isolated, ABSOLUTE" >&2
  echo "       target dir (dev-docs/backlog.md rows 98/100; e.g. eval \"\$(scripts/lane-target.sh create <name>)\")." >&2
  exit 1
fi
target_dir="${CARGO_TARGET_DIR%/}"
case "$target_dir" in
  /*) ;;
  *)
    echo "error: CARGO_TARGET_DIR must be an absolute path, got: '$target_dir'" >&2
    echo "       (dx build --ssg breaks on relative target dirs -- dev-docs/backlog.md row 98;" >&2
    echo "        e.g. CARGO_TARGET_DIR=/home/user/tgt/deploy)." >&2
    exit 1
    ;;
esac
export CARGO_TARGET_DIR="$target_dir"
marker_file="$target_dir/.base-path-build"
public_dir="$target_dir/dx/preview/$profile/web/public"

# --- Guard 2: no-base-path build must not reuse a base-path-tainted tree ----------------
if [ -z "$base_path" ] && [ -e "$marker_file" ]; then
  echo "error: $target_dir carries $marker_file ($(head -n1 "$marker_file"))." >&2
  echo "       A no-base-path build in a tree that ever held a --base-path build emits" >&2
  echo "       /dioxus-components/ URLs everywhere, and wiping public/ does not fix it" >&2
  echo "       (dev-docs/backlog.md row 100). Use a fresh tree: eval \"\$(scripts/lane-target.sh create <name>)\"." >&2
  exit 1
fi

echo "==> Target dir: $target_dir (public dir: $public_dir)"

# --- Guard 3: wipe this profile's public/ ------------------------------------------------
rm -rf "$public_dir"

if [ -n "$base_path" ]; then
  # Tag the tree BEFORE building (even a failed base-path build taints the cache). Row 100.
  mkdir -p "$target_dir"
  echo "dx build ... --base-path $base_path ($(date -u +%FT%TZ))" > "$marker_file"
fi

# Freshness marker for guard 4: an existing dir/200 does not prove it is THIS build's output
# (same discipline as dev-docs/dx-serve-hot-reload.md).
build_marker="$(mktemp)"
trap 'rm -f "$build_marker"' EXIT

dx_args=(build --platform web --ssg --features fullstack --force-sequential=true)
[ "$profile" = release ] && dx_args+=(--release)
[ -n "$base_path" ] && dx_args+=(--base-path "$base_path")

# `--force-sequential=true` is NOT optional: it avoids the dx SSG server/client-build race that
# ships dead, never-hydrating pages. Full analysis: the comment above the "Verifying every
# route's build output includes its hydration bootstrap script" step in scripts/deploy-preview.sh.
echo "==> Building preview ($profile, ssg, fullstack${base_path:+, base-path $base_path}) ..."
cd "$repo_root/preview"
dx "${dx_args[@]}"

if [ ! -d "$public_dir" ]; then
  echo "error: expected build output at $public_dir, not found" >&2
  echo "       (is dx writing to a different CARGO_TARGET_DIR than '$target_dir'?)." >&2
  exit 1
fi

# --- Guard 4: every page is newer than the pre-build marker ------------------------------
total_html=0 stale_html=0
stale_list=()
while IFS= read -r -d '' html_file; do
  total_html=$((total_html + 1))
  if [ ! "$html_file" -nt "$build_marker" ]; then
    stale_html=$((stale_html + 1))
    stale_list+=("${html_file#"$public_dir"/}")
  fi
done < <(find "$public_dir" -name index.html -print0)
if [ "$total_html" -eq 0 ]; then
  echo "error: $public_dir holds no index.html at all -- dx produced no pages." >&2
  exit 1
fi
if [ "$stale_html" -gt 0 ]; then
  echo "error: STALE_HTML=$stale_html of $total_html page(s) in $public_dir were not written by this build" >&2
  echo "       (dx 0.7.9 --ssg keeps the previous build's HTML for routes it did not re-render):" >&2
  printf '  - %s\n' "${stale_list[@]:0:20}" >&2
  exit 1
fi

# --- Guard 5: output matches the requested base path -------------------------------------
if [ -n "$base_path" ]; then
  if ! grep -qE "[\"'(]/$base_path/" "$public_dir/index.html"; then
    echo "error: built with --base-path $base_path but $public_dir/index.html references no /$base_path/ URL --" >&2
    echo "       the output would 404 on its host (stale cached artifacts in $target_dir?)." >&2
    exit 1
  fi
else
  if hit="$(grep -rlE --include='*.html' "[\"'(]/dioxus-components/" "$public_dir" | head -n 3)" && [ -n "$hit" ]; then
    echo "error: no --base-path requested but the output references /dioxus-components/ URLs (row 100):" >&2
    printf '  - %s\n' $hit >&2
    echo "       This tree carries base-path build artifacts; use a fresh one (scripts/lane-target.sh create)." >&2
    exit 1
  fi
fi

echo "==> OK: $total_html page(s), all written by this build."
echo "PUBLIC_DIR=$public_dir"
echo "STALE_HTML=0"
