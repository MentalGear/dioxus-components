use dioxus::prelude::*;
use dioxus_primitives::aspect_ratio::{self, AspectRatioProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

// docs/backlog.md row 32: `#[css_module]` is gone from every themed wrapper's
// classes -- see checkbox/component.rs's header comment (this crate's
// original migration) for the full asset!()+document::Link rationale.
//
// PRE-EXISTING BUG, found (not caused) by this migration, fixed here as its
// own distinct change: unlike every other themed wrapper in this crate, this
// file previously had NO `#[css_module]` at all -- it just forwarded `props`
// straight into the raw primitive (`dioxus_primitives::aspect_ratio::AspectRatio(props)`),
// attaching no class and linking no stylesheet. `style.css`
// (`.dx-aspect-ratio-container`/`.dx-aspect-ratio-image`) was reachable only
// through the demo-only `variants/main/mod.rs`'s own `#[css_module]`, which
// `dx components add aspect_ratio` never copies (`component.json`'s
// `exclude` drops `variants`) -- so a real consumer got the CSS file on disk
// with nothing in the shipped code that ever loaded or referenced it. Fixed
// the same way `../separator/component.rs` (itself not yet migrated, but
// already compiling) already handles a primitive whose props carry a bare
// `attributes: Vec<Attribute>` instead of a dedicated `class` field: an
// `attributes!`-built base class merged ahead of the caller's own
// attributes via `merge_attributes`, so a caller-supplied `class` still
// composes rather than overwriting this one.
#[component]
pub fn AspectRatio(props: AspectRatioProps) -> Element {
    let base = attributes!(div {
        class: "dx-aspect-ratio",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/aspect_ratio/style.css") }
        aspect_ratio::AspectRatio {
            ratio: props.ratio,
            attributes: merged,
            {props.children}
        }
    }
}
