#!/usr/bin/env bash
# Idempotent toolchain bootstrap for a fresh Claude Code *remote* container
# (dev-docs/backlog.md row 116, "Environment note" at the top of backlog.md).
# Every step is skipped when its result already exists, so a second run is a
# cheap no-op.
#
# Installs / seeds:
#   1. rustup target wasm32-unknown-unknown
#   2. dx (dioxus-cli) at the version pinned below -- prebuilt GitHub release
#      binary first (seconds), `cargo install --locked` (~20 min) as fallback
#   3. the three tools dx would otherwise download itself and cannot, because
#      dx's reqwest client rejects the agent proxy's CA (`UnknownIssuer`):
#      esbuild, binaryen/wasm-opt, wasm-bindgen -> ~/.local/share/.dx/tools/
#      (fetched with curl, which does trust the CA; TLS is never disabled)
#   4. `npm ci` in playwright/ and preview/ (never plain `npm install` --
#      backlog row 72; Playwright browsers are preinstalled at
#      /opt/pw-browsers, so `playwright install` is deliberately NOT run)
#   5. `cargo fetch --locked`
#
# Usage: scripts/session-setup.sh          (runs anywhere; the SessionStart hook
#        in .claude/settings.json only invokes it when CLAUDE_CODE_REMOTE=true)
set -euo pipefail

DX_VERSION="0.7.9"
ESBUILD_VERSION="0.27.3"
BINARYEN_VERSION="129"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tools_dir="${HOME}/.local/share/.dx/tools"
export PATH="${HOME}/.cargo/bin:${PATH}"

log() { printf '[session-setup] %s\n' "$*" >&2; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Prefer the proxy CA bundle when present (curl already honours it via
# CURL_CA_BUNDLE/SSL_CERT_FILE in this sandbox; this is just a belt-and-braces).
curl_opts=(-fsSL --retry 3 --retry-delay 2)
if [ -z "${CURL_CA_BUNDLE:-}${SSL_CERT_FILE:-}" ] && [ -f /root/.ccr/ca-bundle.crt ]; then
  curl_opts+=(--cacert /root/.ccr/ca-bundle.crt)
fi

# --- 1. wasm target ---------------------------------------------------------
if ! rustup target list --installed 2>/dev/null | grep -qx 'wasm32-unknown-unknown'; then
  log "adding rustup target wasm32-unknown-unknown"
  rustup target add wasm32-unknown-unknown
fi

# --- 2. dx -------------------------------------------------------------------
if ! dx --version 2>/dev/null | grep -q "^dioxus ${DX_VERSION} "; then
  log "installing dx ${DX_VERSION}"
  mkdir -p "${HOME}/.cargo/bin"
  if curl "${curl_opts[@]}" -o "$tmp/dx.tgz" \
      "https://github.com/DioxusLabs/dioxus/releases/download/v${DX_VERSION}/dx-x86_64-unknown-linux-gnu.tar.gz" \
    && tar xzf "$tmp/dx.tgz" -C "$tmp" dx \
    && install -m755 "$tmp/dx" "${HOME}/.cargo/bin/dx" \
    && dx --version 2>/dev/null | grep -q "^dioxus ${DX_VERSION} "; then
    log "dx ${DX_VERSION} installed from the GitHub release binary"
  else
    log "prebuilt dx unavailable; falling back to cargo install --locked (~20 min)"
    cargo install dioxus-cli --version "${DX_VERSION}" --locked --force
  fi
fi

# --- 3. dx build tools (seeded by hand; see header) --------------------------
mkdir -p "$tools_dir"

# wasm-bindgen must match the wasm-bindgen crate version in Cargo.lock.
wb_version="$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[^0-9.]/, "", $0); print; exit}' "$repo_root/Cargo.lock")"
if [ -z "$wb_version" ]; then
  log "could not read the wasm-bindgen version from Cargo.lock"; exit 1
fi

if [ ! -x "$tools_dir/esbuild-${ESBUILD_VERSION}/esbuild" ]; then
  log "seeding esbuild ${ESBUILD_VERSION}"
  curl "${curl_opts[@]}" -o "$tmp/esbuild.tgz" \
    "https://registry.npmjs.org/@esbuild/linux-x64/-/linux-x64-${ESBUILD_VERSION}.tgz"
  tar xzf "$tmp/esbuild.tgz" -C "$tmp" package/bin/esbuild
  mkdir -p "$tools_dir/esbuild-${ESBUILD_VERSION}"
  install -m755 "$tmp/package/bin/esbuild" "$tools_dir/esbuild-${ESBUILD_VERSION}/esbuild"
fi

if [ ! -x "$tools_dir/binaryen-${BINARYEN_VERSION}/bin/wasm-opt" ]; then
  log "seeding binaryen version_${BINARYEN_VERSION}"
  curl "${curl_opts[@]}" -o "$tmp/binaryen.tgz" \
    "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-x86_64-linux.tar.gz"
  tar xzf "$tmp/binaryen.tgz" -C "$tmp"
  rm -rf "$tools_dir/binaryen-${BINARYEN_VERSION}"
  mv "$tmp/binaryen-version_${BINARYEN_VERSION}" "$tools_dir/binaryen-${BINARYEN_VERSION}"
fi

if [ ! -x "$tools_dir/wasm-bindgen-${wb_version}/wasm-bindgen" ]; then
  log "seeding wasm-bindgen ${wb_version}"
  if curl "${curl_opts[@]}" -o "$tmp/wb.tgz" \
      "https://github.com/rustwasm/wasm-bindgen/releases/download/${wb_version}/wasm-bindgen-${wb_version}-x86_64-unknown-linux-musl.tar.gz"; then
    tar xzf "$tmp/wb.tgz" -C "$tmp"
    mkdir -p "$tools_dir/wasm-bindgen-${wb_version}"
    install -m755 "$tmp/wasm-bindgen-${wb_version}-x86_64-unknown-linux-musl/wasm-bindgen" \
      "$tools_dir/wasm-bindgen-${wb_version}/wasm-bindgen"
  else
    log "wasm-bindgen download failed; dx will compile it from source on first build (~2.5 min)"
  fi
fi

# --- 4. node deps (npm ci only) -----------------------------------------------
npm_ci_if_stale() {
  local dir="$1"
  [ -f "$dir/package.json" ] || return 0
  # preview/package-lock.json is gitignored (root .gitignore: package-lock.json),
  # so a fresh checkout has none: generate one (no install) so `npm ci` can run.
  if [ ! -f "$dir/package-lock.json" ]; then
    log "no lockfile in ${dir#"$repo_root"/}; generating one (--package-lock-only)"
    (cd "$dir" && npm install --package-lock-only --no-audit --no-fund >/dev/null)
  fi
  # node_modules/.package-lock.json is npm's own marker, newer than the lockfile
  # whenever node_modules is in sync. A symlinked node_modules (worktree lanes)
  # is left alone.
  if [ -L "$dir/node_modules" ] \
     || { [ -f "$dir/node_modules/.package-lock.json" ] \
          && [ "$dir/node_modules/.package-lock.json" -nt "$dir/package-lock.json" ]; }; then
    return 0
  fi
  log "npm ci in ${dir#"$repo_root"/}"
  (cd "$dir" && npm ci --no-audit --no-fund >/dev/null)
}
npm_ci_if_stale "$repo_root/playwright"
npm_ci_if_stale "$repo_root/preview"

# --- 5. crates ----------------------------------------------------------------
# Fast when the registry/git caches are warm.
(cd "$repo_root" && cargo fetch --locked >/dev/null 2>&1) || log "cargo fetch failed (non-fatal)"

log "done"
