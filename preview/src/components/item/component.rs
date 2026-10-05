use crate::components::separator::Separator;
use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ItemVariant {
    #[default]
    Default,
    Outline,
    Muted,
}

impl ItemVariant {
    pub fn class(&self) -> &'static str {
        match self {
            ItemVariant::Default => "default",
            ItemVariant::Outline => "outline",
            ItemVariant::Muted => "muted",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ItemSize {
    #[default]
    Default,
    Sm,
}

impl ItemSize {
    pub fn class(&self) -> &'static str {
        match self {
            ItemSize::Default => "default",
            ItemSize::Sm => "sm",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ItemMediaVariant {
    #[default]
    Default,
    Icon,
    Image,
}

impl ItemMediaVariant {
    pub fn class(&self) -> &'static str {
        match self {
            ItemMediaVariant::Default => "default",
            ItemMediaVariant::Icon => "icon",
            ItemMediaVariant::Image => "image",
        }
    }
}

/// Provided by [`ItemGroup`] so an [`Item`] knows it is a list's child. `ItemGroup` is `role="list"`,
/// which ARIA requires to own `listitem`s (axe `aria-required-children`), and an `Item` outside a group
/// must NOT claim `listitem` (an orphan one is the mirror failure, `aria-required-parent`). Context
/// makes both hold by construction: the role follows the structure, nobody has to remember it.
#[derive(Clone, Copy)]
struct InItemGroup;

#[component]
pub fn ItemGroup(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    use_context_provider(|| InItemGroup);
    let base = attributes!(div {
        class: "dx-item-group",
        role: "list",
        "data-slot": "item-group",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemSeparator(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-separator",
        "data-slot": "item-separator",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        Separator { horizontal: true, decorative: true, attributes: merged }
    }
}

#[component]
pub fn Item(
    #[props(default)] variant: ItemVariant,
    #[props(default)] size: ItemSize,
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    onclick: Option<EventHandler<MouseEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let in_group = try_use_context::<InItemGroup>().is_some();
    let mut base = attributes!(div {
        class: "dx-item",
        "data-slot": "item",
        "data-variant": variant.class(),
        "data-size": size.class(),
    });
    // Inside an `ItemGroup` (`role="list"`) the item is a `listitem`; a caller's own `role` still wins.
    // With `as` the item is usually a link, and `role="listitem"` on an `<a>` would erase its link role,
    // so the list item is a transparent wrapper instead (`display: contents`: no box, the item stays the
    // group's flex child).
    if in_group && r#as.is_none() {
        base.extend(attributes!(div { role: "listitem" }));
    }
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        if let Some(dynamic) = r#as {
            if in_group {
                div { role: "listitem", display: "contents", {dynamic.call(merged)} }
            } else {
                {dynamic.call(merged)}
            }
        } else {
            div {
                onclick: move |event| {
                    if let Some(f) = &onclick {
                        f.call(event);
                    }
                },
                onkeydown: move |event| {
                    if let Some(f) = &onkeydown {
                        f.call(event);
                    }
                },
                ..merged,
                {children}
            }
        }
    }
}

#[component]
pub fn ItemMedia(
    #[props(default)] variant: ItemMediaVariant,
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-media",
        "data-slot": "item-media",
        "data-variant": variant.class(),
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemContent(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-content",
        "data-slot": "item-content",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemTitle(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-title",
        "data-slot": "item-title",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemDescription(
    #[props(extends=GlobalAttributes)]
    #[props(extends=p)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(p {
        class: "dx-item-description",
        "data-slot": "item-description",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        p { ..merged,{children} }
    }
}

#[component]
pub fn ItemActions(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-actions",
        "data-slot": "item-actions",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemHeader(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-header",
        "data-slot": "item-header",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}

#[component]
pub fn ItemFooter(
    #[props(extends=GlobalAttributes)]
    #[props(extends=div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-item-footer",
        "data-slot": "item-footer",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/item/style.css") }
        div { ..merged,{children} }
    }
}
