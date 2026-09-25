use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronRight, Ellipsis};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// A trail of links showing the current page's place in a hierarchy.
/// Renders as `<nav aria-label="breadcrumb"><ol>...</ol></nav>` -- no ARIA
/// widget role beyond the landmark, so there is no APG contract here and no
/// primitive underneath.
#[component]
pub fn Breadcrumb(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(nav { class: "dx-breadcrumb", "aria-label": "breadcrumb" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        nav { ..merged, {children} }
    }
}

/// The ordered list of breadcrumb items.
#[component]
pub fn BreadcrumbList(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(ol { class: "dx-breadcrumb-list" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        ol { ..merged, {children} }
    }
}

/// One `<li>` wrapping a [`BreadcrumbLink`] or [`BreadcrumbPage`].
#[component]
pub fn BreadcrumbItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(li { class: "dx-breadcrumb-item" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li { ..merged, {children} }
    }
}

/// A navigable, not-current-page crumb.
#[component]
pub fn BreadcrumbLink(
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(a { class: "dx-breadcrumb-link" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        a { ..merged, {children} }
    }
}

/// The current page's crumb -- not a link, marked `aria-current="page"`.
#[component]
pub fn BreadcrumbPage(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(span { class: "dx-breadcrumb-page" });
    // `role`/`aria-current`/`aria-disabled` are semantics this wrapper
    // asserts for "the current page" and must win over anything the caller
    // passes -- merged in LAST, after the caller's own attributes. See the
    // merge-precedence policy in dev-docs/issues/duplicate-attribute-findings.md.
    let owned = attributes!(span {
        role: "link",
        "aria-disabled": "true",
        "aria-current": "page",
    });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        span { ..merged, {children} }
    }
}

/// The `>` divider between crumbs. `role="presentation"` +
/// `aria-hidden="true"` per the shadcn/Radix shape -- it carries no
/// semantic content, so it must not be announced.
#[component]
pub fn BreadcrumbSeparator(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    #[props(default)] children: Option<Element>,
) -> Element {
    let base = attributes!(li { class: "dx-breadcrumb-separator" });
    let owned = attributes!(li { role: "presentation", "aria-hidden": "true" });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li {
            ..merged,
            if let Some(children) = &children {
                {children.clone()}
            } else {
                ChevronRight {}
            }
        }
    }
}

/// A collapsed run of crumbs, shown as `...`. Carries an `sr-only` label so
/// the collapse is still announced even though the glyph itself is hidden.
#[component]
pub fn BreadcrumbEllipsis(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(li { class: "dx-breadcrumb-ellipsis" });
    let owned = attributes!(li { role: "presentation", "aria-hidden": "true" });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li {
            ..merged,
            Ellipsis {}
            span { class: "dx-breadcrumb-sr-only", "More" }
        }
    }
}
