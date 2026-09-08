use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronRight, Ellipsis};

/// A trail of links showing the current page's place in a hierarchy.
/// Renders as `<nav aria-label="breadcrumb"><ol>...</ol></nav>` -- no ARIA
/// widget role beyond the landmark, so there is no APG contract here and no
/// primitive underneath.
#[component]
pub fn Breadcrumb(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        nav { class: "dx-breadcrumb", "aria-label": "breadcrumb", ..attributes, {children} }
    }
}

/// The ordered list of breadcrumb items.
#[component]
pub fn BreadcrumbList(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        ol { class: "dx-breadcrumb-list", ..attributes, {children} }
    }
}

/// One `<li>` wrapping a [`BreadcrumbLink`] or [`BreadcrumbPage`].
#[component]
pub fn BreadcrumbItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li { class: "dx-breadcrumb-item", ..attributes, {children} }
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        a { class: "dx-breadcrumb-link", ..attributes, {children} }
    }
}

/// The current page's crumb -- not a link, marked `aria-current="page"`.
#[component]
pub fn BreadcrumbPage(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        span {
            class: "dx-breadcrumb-page",
            role: "link",
            "aria-disabled": "true",
            "aria-current": "page",
            ..attributes,
            {children}
        }
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li {
            class: "dx-breadcrumb-separator",
            role: "presentation",
            "aria-hidden": "true",
            ..attributes,
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/breadcrumb/style.css") }
        li {
            class: "dx-breadcrumb-ellipsis",
            role: "presentation",
            "aria-hidden": "true",
            ..attributes,
            Ellipsis {}
            span { class: "dx-breadcrumb-sr-only", "More" }
        }
    }
}
