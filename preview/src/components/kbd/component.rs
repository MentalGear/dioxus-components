use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// A single keyboard key or shortcut, e.g. `Kbd { "⌘" }`. Renders the
/// native `<kbd>` element, which already carries the right semantics --
/// no ARIA widget role, no primitive underneath.
#[component]
pub fn Kbd(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(kbd { class: "dx-kbd" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/kbd/style.css") }
        kbd { ..merged, {children} }
    }
}

/// Lays out several [`Kbd`]s that together spell one shortcut (e.g.
/// `Ctrl` `Shift` `P`), with consistent spacing between them.
#[component]
pub fn KbdGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(span { class: "dx-kbd-group" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/kbd/style.css") }
        span { ..merged, {children} }
    }
}
