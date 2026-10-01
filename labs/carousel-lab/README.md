# carousel-lab

A tiny, router-free Dioxus 0.7 web app that renders the **real** `dioxus-primitives` horizontal virtual Carousel. It's built for embedding in an external telemetry page (a claude.ai artifact), where real-trackpad sessions can be recorded against the shipping component.

It renders two carousels, stacked, into one mount element (default id `dx-carousel-lab`):

- **`virtual_many`**: 200 items, not looping.
- **`virtual_loop`**: 12 items, seamless loop, autoplay at 1200ms, dot picker.

Both are configured exactly like `preview/src/components/carousel/variants/{virtual_many,virtual_loop}`: default `radius` 2, a `1rem` gap, `26rem` max width, and the theme's 48px Previous/Next reservation. Only minimal inline styles are used; the primitive needs no theme CSS.

A host page can choose the mount element by setting `window.__dxCarouselLabMount = "<id>"` before booting.

## Detached from the workspace

`Cargo.toml` carries its own empty `[workspace]` table, so this crate is its own workspace root. The repository's gates (`cargo clippy --workspace`, `cargo test --workspace`, and the rest) never see it. It depends on `../../primitives` by path with `default-features = false, features = ["web"]`, so the router is never pulled in. It has its own `Cargo.lock`.

## Build

Use an absolute, isolated target dir (see the repo's CLAUDE.md):

```bash
cd labs/carousel-lab
CARGO_TARGET_DIR=/abs/path/to/target dx build --platform web --release
```

Output goes to `$CARGO_TARGET_DIR/dx/carousel-lab/release/web/public/`: `index.html` (mounts into `#main`, not used by the embed), plus `assets/carousel-lab-<hash>.js` and `assets/carousel-lab_bg-<hash>.wasm`.

The embeddable dist is made from those two assets by `make-dist.sh` (`CARGO_TARGET_DIR=... ./make-dist.sh <out-dir>`):

1. Copy the wasm as `lab_bg.wasm`.
2. Copy the glue as `lab.js`, with the dx-appended auto-boot statement removed. That's the trailing `init({module_or_path:"/./assets/carousel-lab_bg-<hash>.wasm"}).then(...)`, which fetches an origin-absolute path.
3. Point the default wasm URL at `lab_bg.wasm`.
4. Add `globalThis.__dxCarouselLabInit = <init>` so an inlined copy can still be booted.

How to boot the result, and how `document::eval` runs JS in the web renderer, are documented in the `LOADER.md` that ships with the dist.

## Telemetry

Set `localStorage["dx-carousel-debug"] = "1"` before booting. The component then records:
- per-gesture wheel records in `window.__dxCarouselWheel`;
- per-event traces in `window.__dxCarouselTrace` (idle gate, scroll ends, settles, virtual window re-renders, every scroll write). See `carousel_trace_js!` in `primitives/src/carousel.rs`.

With the flag off, the cost is zero.
