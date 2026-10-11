#!/usr/bin/env bash
#
# registry-install-smoke.sh
#
# The definitive "does a user's `dx components add` work" check. Installs EVERY component of this registry
# into one scratch Dioxus app and runs `cargo check` on it. Slow (a cold `cargo check` of dioxus plus the
# primitives, minutes), so it is NOT in scripts/run-gates.sh: run it before merging whenever a
# `component.json` or any file under `preview/src/components/<name>/` (except `variants/` and `docs.md`)
# changes. `scripts/check-installed-paths.sh` is the fast static approximation that is in run-gates.sh; this
# is what it approximates (it exists because 27 components lacked the `mod.rs` an install needs and still
# passed every gate, the preview declaring them inline: E0583 in a user's tree).
#
# What it does, all in one scratch directory that is deleted on exit:
#   1. copies the working tree (tracked + untracked, honouring .gitignore, so uncommitted edits are tested)
#      into a throwaway git repo and rewrites the registry URL in its component.json files to that repo's
#      `file://` URL. Without the rewrite every `componentDependencies` entry and the `dioxus-primitives`
#      cargo dependency would be read from github.com's main, not from this tree;
#   2. renders `templates/starter` (the app `dx new` produces) as the user's app, with the same rewrite; with
#      `--cli BIN` the app comes from `BIN new app --path <repo>` and is diffed against the hand rendering;
#   3. `dx components add <every member of the root component.json> --git file://<repo>` inside it
#      (dioxus-cli 0.7.9), or with `--cli BIN` (the `shadcn-dioxus` binary built from `cli/`)
#      `BIN add <names> --path <repo>`;
#   4. checks every component directory and its `pub mod` line landed, then
#      `cargo check` with CARGO_INCREMENTAL=0 and an absolute scratch CARGO_TARGET_DIR (never the repo's).
#
# Usage: scripts/registry-install-smoke.sh [--cli BIN] [--only a,b,c] [--target TRIPLE] [--keep]
#   --cli BIN        install with this `shadcn-dioxus` binary instead of `dx components`
#   --only a,b,c     install just these components (default: all of them)
#   --target TRIPLE  cargo check --target TRIPLE (default: the host; the template's default `web` feature
#                    compiles there too; wasm32-unknown-unknown also works and is the shipped target)
#   --keep           keep the scratch directory (it holds the cargo target dir: several GB) and print it
# Env: REGISTRY_SMOKE_TMP  parent of the scratch dir (default /tmp); needs ~6 GB free.
# Exit status: 0 when every component installed and the app compiles, 1 otherwise, 2 on bad usage/environment.

set -uo pipefail
cd "$(dirname "$0")/.."
repo_root="$PWD"

cli=""
only=""
target=""
keep=0
while [ $# -gt 0 ]; do
  case "$1" in
    --cli) cli="${2:-}"; shift 2 ;;
    --only) only="${2:-}"; shift 2 ;;
    --target) target="${2:-}"; shift 2 ;;
    --keep) keep=1; shift ;;
    -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "registry-install-smoke: unknown argument '$1'" >&2; exit 2 ;;
  esac
done

if [ -n "$cli" ]; then
  [ -x "$cli" ] || { echo "registry-install-smoke: --cli '$cli' is not an executable file" >&2; exit 2; }
  cli="$(cd "$(dirname "$cli")" && pwd)/$(basename "$cli")"
else
  command -v dx >/dev/null || { echo "registry-install-smoke: dx is not installed (scripts/session-setup.sh installs it)" >&2; exit 2; }
fi
command -v cargo >/dev/null || { echo "registry-install-smoke: cargo is not installed" >&2; exit 2; }

tmp_parent="${REGISTRY_SMOKE_TMP:-/tmp}"
mkdir -p "$tmp_parent" || exit 2
avail_kb="$(df -Pk "$tmp_parent" | awk 'NR==2 {print $4}')"
if [ "${avail_kb:-0}" -lt 6000000 ]; then
  echo "registry-install-smoke: only $((avail_kb / 1024)) MB free under $tmp_parent, need ~6 GB (set REGISTRY_SMOKE_TMP)" >&2
  exit 2
fi

work="$(mktemp -d "$tmp_parent/registry-smoke.XXXXXX")" || exit 2
cleanup() {
  if [ "$keep" = 1 ]; then
    echo "registry-install-smoke: kept $work (delete it when done: it holds the cargo target dir)"
  else
    rm -rf "$work"
  fi
}
trap cleanup EXIT
start=$SECONDS
lap() { echo "registry-install-smoke: $1 ($((SECONDS - start))s)"; }

url="$(sed -n 's/^pub(crate) const REGISTRY_GIT_URL: &str = "\(.*\)";$/\1/p' preview/src/main.rs)"
if [ -z "$url" ]; then
  echo "registry-install-smoke: no REGISTRY_GIT_URL in preview/src/main.rs" >&2
  exit 2
fi

# 1. The registry: a throwaway git repo of the working tree.
reg="$work/registry"
mkdir -p "$reg"
git ls-files -z --cached --others --exclude-standard \
  | while IFS= read -r -d '' f; do [ -e "$f" ] && printf '%s\0' "$f"; done \
  | tar --null -T - -cf - | tar -xf - -C "$reg"
reg_url="file://$reg"
find "$reg" -name component.json -not -path '*/node_modules/*' -print0 | xargs -0 sed -i "s|$url|$reg_url|g"
git -C "$reg" init -q
git -C "$reg" add -A
git -C "$reg" -c user.name=smoke -c user.email=smoke@localhost -c commit.gpgsign=false commit -q -m "registry-install-smoke snapshot"
lap "registry snapshot at $reg_url"

if [ -n "$only" ]; then
  names="$(printf '%s' "$only" | tr ',' '\n')"
else
  names="$(python3 -I -c 'import json,sys; print("\n".join(m.rsplit("/",1)[-1] for m in json.load(open(sys.argv[1]))["members"]))' "$reg/component.json")"
fi
count="$(printf '%s\n' "$names" | grep -c .)"

# 2. The user's app: templates/starter, rendered the way `dx new` (cargo-generate) would. The hand rendering below
# is the reference; with --cli the app comes from `shadcn-dioxus new` instead and must equal it (the `authors`
# line aside, which depends on whose git config runs the script), so the CLI's renderer is checked against the
# template on every run.
render_ref() {
  local dest="$1" name="$2"
  mkdir -p "$dest"
  tar -C templates/starter -cf - . | tar -xf - -C "$dest"
  mv "$dest/Cargo.toml.liquid" "$dest/Cargo.toml"
  rm -f "$dest/cargo-generate.toml"
  sed -i -e "s/{{project-name}}/$name/g" -e 's/{{authors}}/smoke/g' -e "s|$url|$reg_url|g" "$dest/Cargo.toml" "$dest/Dioxus.toml" "$dest/README.md" "$dest/src/main.rs"
}
app="$work/app"
if [ -n "$cli" ]; then
  # The cache of registries (for `add` below too) stays inside $work.
  export SHADCN_DIOXUS_HOME="$work/shadcn-dioxus-home"
  if ! (cd "$work" && "$cli" new app --path "$reg" --vcs none) >"$work/new.log" 2>&1; then
    echo "registry-install-smoke: RED -- \`shadcn-dioxus new\` failed:" >&2
    tail -20 "$work/new.log" >&2
    exit 1
  fi
  render_ref "$work/app-ref" app
  sed -i -e "s|$url|$reg_url|g" "$app/Cargo.toml" "$app/Dioxus.toml" "$app/README.md" "$app/src/main.rs"
  if ! diff -r -I '^authors = ' "$work/app-ref" "$app" >"$work/new.diff"; then
    echo "registry-install-smoke: RED -- \`shadcn-dioxus new\` does not render templates/starter like cargo-generate would:" >&2
    head -30 "$work/new.diff" >&2
    exit 1
  fi
else
  render_ref "$app" smoke_app
fi
printf '\n[workspace]\n' >> "$app/Cargo.toml"

# Everything dx and cargo write goes under $work: dx's component cache is keyed on the URL text, so a stale
# clone from an earlier run must never be reused.
export DX_HOME="$work/dx-home"
export XDG_CACHE_HOME="$work/xdg-cache"
export XDG_DATA_HOME="$work/xdg-data"
export CARGO_TARGET_DIR="$work/target"
export CARGO_INCREMENTAL=0

# 3. Install.
install_log="$work/install.log"
# shellcheck disable=SC2086
if [ -n "$cli" ]; then
  (cd "$app" && "$cli" add $(printf '%s ' $names | sed 's/ $//; s/ /,/g') --path "$reg") >"$install_log" 2>&1
else
  (cd "$app" && dx components add $names --git "$reg_url") >"$install_log" 2>&1
fi
install_status=$?
if [ "$install_status" != 0 ]; then
  echo "registry-install-smoke: RED -- the install itself failed (exit $install_status):" >&2
  tail -30 "$install_log" >&2
  exit 1
fi
lap "install finished"

# 4a. Everything landed: a directory with mod.rs + component.rs + style.css and a `pub mod` line per component.
missing=0
installed=0
for n in $names; do
  ok=1
  for f in mod.rs component.rs style.css; do
    [ -f "$app/src/components/$n/$f" ] || { echo "registry-install-smoke: $n: src/components/$n/$f was not installed" >&2; ok=0; }
  done
  grep -qx "pub mod $n;" "$app/src/components/mod.rs" || { echo "registry-install-smoke: $n: no \`pub mod $n;\` line in src/components/mod.rs" >&2; ok=0; }
  if [ "$ok" = 1 ]; then installed=$((installed + 1)); else missing=$((missing + 1)); fi
done
if [ "$missing" != 0 ]; then
  echo "registry-install-smoke: RED -- $missing of $count components did not install completely" >&2
  exit 1
fi
if [ ! -f "$app/assets/dx-components-theme.css" ]; then
  echo "registry-install-smoke: RED -- assets/dx-components-theme.css (a globalAssets file) was not installed" >&2
  exit 1
fi

# 4b. It compiles. `mod components;` is already in the template's main.rs.
check_log="$work/cargo-check.log"
target_args=()
[ -n "$target" ] && target_args=(--target "$target")
if ! (cd "$app" && cargo check --message-format short "${target_args[@]}") >"$check_log" 2>&1; then
  echo "registry-install-smoke: RED -- cargo check failed after installing $installed/$count components:" >&2
  grep -E '^(error|[^ ]+: error)' "$check_log" | sort | uniq -c | sort -rn | head -40 >&2
  echo "--- tail of cargo check ---" >&2
  tail -15 "$check_log" >&2
  exit 1
fi
warnings="$(grep -c 'warning' "$check_log" || true)"
lap "cargo check finished"

echo "registry-install-smoke: OK -- $installed/$count components installed with $([ -n "$cli" ] && echo "shadcn-dioxus" || echo "dx $(dx --version | awk '{print $2}')") and the app compiles (cargo check${target:+ --target $target}, ${warnings} warning lines), $((SECONDS - start))s."
