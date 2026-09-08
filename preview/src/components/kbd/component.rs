use dioxus::prelude::*;

/// A single keyboard key or shortcut, e.g. `Kbd { "⌘" }`. Renders the
/// native `<kbd>` element, which already carries the right semantics --
/// no ARIA widget role, no primitive underneath.
#[component]
pub fn Kbd(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/kbd/style.css") }
        kbd { class: "dx-kbd", ..attributes, {children} }
    }
}

/// Lays out several [`Kbd`]s that together spell one shortcut (e.g.
/// `Ctrl` `Shift` `P`), with consistent spacing between them.
#[component]
pub fn KbdGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/kbd/style.css") }
        span { class: "dx-kbd-group", ..attributes, {children} }
    }
}
