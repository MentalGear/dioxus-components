#!/usr/bin/env bash
# Build the embeddable dist (lab.js + lab_bg.wasm) from a finished
# `dx build --platform web --release` of this crate.
#
#   CARGO_TARGET_DIR=/abs/target dx build --platform web --release
#   CARGO_TARGET_DIR=/abs/target ./make-dist.sh /abs/out/dir
#
# What it does to dx's output, and why (see README.md):
#   1. copies the wasm as lab_bg.wasm;
#   2. copies the JS glue as lab.js, removing the auto-boot dx appends (it
#      fetches an origin-absolute /./assets/<hashed>.wasm, which breaks in any
#      page served from elsewhere) -- the host page calls init() itself;
#   3. points the glue's default wasm URL at lab_bg.wasm;
#   4. publishes init as globalThis.__dxCarouselLabInit, so a copy of lab.js
#      inlined into a <script type="module"> (whose exports cannot be
#      imported) can still be booted.
# Every transform asserts it matched exactly once, so a future dx/wasm-bindgen
# change in the glue's shape fails loudly instead of shipping a broken dist.
set -euo pipefail

: "${CARGO_TARGET_DIR:?set CARGO_TARGET_DIR to the absolute target dir of the dx build}"
out="${1:?usage: make-dist.sh <out-dir>}"
public="$CARGO_TARGET_DIR/dx/carousel-lab/release/web/public"
mkdir -p "$out"

python3 - "$public" "$out" <<'PY'
import os, re, shutil, sys
public, out = sys.argv[1], sys.argv[2]
# Take exactly the files THIS build uses, never a glob of the assets dir: dx
# does not clear old hashed assets out of a reused target dir, so a second
# build leaves two of each there (found live). The build's own index.html
# names its glue, and the glue's own auto-boot names its wasm.
html = open(f"{public}/index.html").read()
js = sorted(set(re.findall(r'assets/carousel-lab-dxh\w+\.js', html)))
if len(js) != 1:
    sys.exit(f"index.html: expected one glue reference, found {js}")
src = open(f"{public}/{js[0]}").read()

boot = re.findall(r'\w+\(\{module_or_path:"([^"]*)"\}\)\.then\(r=>\{[^}]*\}\);', src)
if len(boot) != 1:
    sys.exit(f"auto-boot statement: expected 1 match, found {len(boot)}")
wasm = [f"{public}/assets/{os.path.basename(boot[0])}"]
if not os.path.isfile(wasm[0]):
    sys.exit(f"the auto-boot's wasm {boot[0]} is not in {public}/assets")
src = re.sub(r'\w+\(\{module_or_path:"[^"]*"\}\)\.then\(r=>\{[^}]*\}\);', "", src, count=1)

default_url = 'new URL("carousel-lab_bg.wasm",import.meta.url)'
if src.count(default_url) != 1:
    sys.exit("default wasm URL: expected 1 match")
src = src.replace(default_url, 'new URL("lab_bg.wasm",import.meta.url)')

exports = re.search(r'export\{[^}]*\b(\w+) as default\b[^}]*\};\s*$', src)
if not exports:
    sys.exit("trailing export statement not found")
src = src[:exports.start()] + f"globalThis.__dxCarouselLabInit={exports.group(1)};" + src[exports.start():]

if "</script" in src or "<!--" in src:
    sys.exit("glue contains a sequence that breaks inlining into <script>")
open(f"{out}/lab.js", "w").write(src)
shutil.copy(wasm[0], f"{out}/lab_bg.wasm")
print(f"wrote {out}/lab.js and {out}/lab_bg.wasm")
PY
