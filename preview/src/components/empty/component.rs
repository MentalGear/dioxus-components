use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// The visual treatment of an [`EmptyMedia`] -- a plain icon glyph, or an
/// icon inside a filled rounded square.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum EmptyMediaVariant {
    #[default]
    Default,
    Icon,
}

impl EmptyMediaVariant {
    pub fn class(&self) -> &'static str {
        match self {
            EmptyMediaVariant::Default => "default",
            EmptyMediaVariant::Icon => "icon",
        }
    }
}

/// A placeholder shown in place of content that hasn't loaded, doesn't
/// exist yet, or was filtered away to nothing -- an empty inbox, a search
/// with no results, a not-yet-created list. Plain layout with no ARIA
/// widget role, so there is no primitive underneath.
#[component]
pub fn Empty(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}

/// Groups the icon/title/description at the top of an [`Empty`] state.
#[component]
pub fn EmptyHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty-header" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}

/// The leading icon or illustration. `variant: EmptyMediaVariant::Icon`
/// (the default look shadcn calls out) wraps it in a filled rounded
/// square; `Default` renders it plain.
#[component]
pub fn EmptyMedia(
    #[props(default)] variant: EmptyMediaVariant,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty-media" });
    // `data-style` reflects this wrapper's own typed `variant` prop, not a
    // caller default -- owned-wins.
    let owned = attributes!(div { "data-style": variant.class() });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}

/// The empty state's heading.
#[component]
pub fn EmptyTitle(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty-title" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}

/// Supporting body text under the title.
#[component]
pub fn EmptyDescription(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty-description" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}

/// A slot below the header for actions -- typically one or more `Button`s.
#[component]
pub fn EmptyContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-empty-content" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/empty/style.css") }
        div { ..merged, {children} }
    }
}
