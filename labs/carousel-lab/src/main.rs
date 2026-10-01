//! The REAL `dioxus-primitives` virtual Carousel, router-free, mounted into a
//! host page's own element -- see `../README.md`.
//!
//! Renders both horizontal virtual demos, one above the other, configured
//! exactly as `preview/src/components/carousel/variants/{virtual_many,
//! virtual_loop}` configure them: 200 non-looping items, and 12 items with
//! `loop` (the seamless windowed loop) plus `CarouselAutoplay { delay_ms:
//! 1200 }` and a `CarouselIndicators` dot picker; the default `radius` (10,
//! capped at 5 on the 12-item loop so no item repeats);
//! a `1rem` gap (the theme's `--dx-space-4`); a `26rem` max width with the
//! theme's 48px Previous/Next reservation on each side. Only minimal inline
//! styling -- the primitive itself needs no theme CSS.

use dioxus::prelude::*;
use dioxus_primitives::carousel::{
    Carousel, CarouselAutoplay, CarouselIndicator, CarouselIndicators, CarouselNext,
    CarouselPrevious, CarouselVirtualContent,
};

/// The default mount element id. A host page may override it by setting
/// `window.__dxCarouselLabMount = "<id>"` before booting the module.
const DEFAULT_MOUNT_ID: &str = "dx-carousel-lab";

fn mount_id() -> String {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"__dxCarouselLabMount".into()).ok())
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MOUNT_ID.to_string())
}

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().rootname(mount_id()))
        .launch(App);
}

const FRAME: &str =
    "width: 100%; max-width: 26rem; margin: 0 auto 2rem; font-family: system-ui, sans-serif;";
// The theme's `.dx-carousel` root: border-box, reserving 48px (28px button
// + 20px clearance) on each inline side for Previous/Next.
const ROOT: &str = "position: relative; box-sizing: border-box; width: 100%; padding-inline: 48px;";
const BUTTON: &str = "position: absolute; inset-block-start: 50%; transform: translateY(-50%); \
     width: 28px; height: 28px; border-radius: 50%; border: 1px solid #c8c8c8; background: #fff; \
     color: #222; font-size: 14px; line-height: 1; cursor: pointer; padding: 0;";
const CARD: &str = "display: flex; align-items: center; justify-content: center; aspect-ratio: 1; \
     font-size: 1.875rem; font-weight: 600; border: 1px solid #ddd; border-radius: 12px; \
     background: #fafafa; color: #111; user-select: none;";
const DOTS: &str = "display: flex; gap: 6px; justify-content: center; margin-block-start: 12px;";
const DOT: &str = "width: 8px; height: 8px; border-radius: 50%; border: 0; padding: 0; background: #bbb; cursor: pointer;";

#[component]
fn App() -> Element {
    let many: Vec<String> = (0..200).map(|i| format!("{}", i + 1)).collect();
    let looped: Vec<String> = (0..12).map(|i| format!("{}", i + 1)).collect();
    rsx! {
        div { style: FRAME,
            h3 { style: "margin: 0 0 8px; font-size: 14px; font-weight: 600;", "virtual_many -- 200 items, not looping" }
            Carousel { aria_label: "Large product catalog", style: ROOT, "data-lab": "virtual_many",
                CarouselPrevious { style: "{BUTTON} inset-inline-start: 0;", "‹" }
                CarouselNext { style: "{BUTTON} inset-inline-end: 0;", "›" }
                CarouselVirtualContent::<String> {
                    items: many,
                    gap: "1rem",
                    render_item: move |(_idx, label): (usize, String)| rsx! {
                        div { style: CARD, "{label}" }
                    },
                }
            }
        }
        div { style: FRAME,
            h3 { style: "margin: 0 0 8px; font-size: 14px; font-weight: 600;", "virtual_loop -- 12 items, seamless loop, autoplay" }
            Carousel { aria_label: "Seamless looping gallery", r#loop: true, style: ROOT, "data-lab": "virtual_loop",
                CarouselAutoplay { delay_ms: 1200u64 }
                CarouselPrevious { style: "{BUTTON} inset-inline-start: 0;", "‹" }
                CarouselNext { style: "{BUTTON} inset-inline-end: 0;", "›" }
                CarouselVirtualContent::<String> {
                    items: looped,
                    gap: "1rem",
                    render_item: move |(_idx, label): (usize, String)| rsx! {
                        div { style: CARD, "{label}" }
                    },
                }
                CarouselIndicators { style: DOTS,
                    for i in 0..12usize {
                        CarouselIndicator { key: "{i}", index: i, style: DOT }
                    }
                }
            }
        }
    }
}
