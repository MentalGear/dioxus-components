use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight, Ellipsis};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in every exported entry point of this file so a
// `dx components add pagination`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane: its
// screen-reader-only helper used to be the bare `dx-sr-only`, which
// `sidebar/style.css` also defines (byte-identical). `#[css_module]`'s hash
// kept the two apart; renamed to `dx-pagination-sr-only` in the same change
// that drops the macro (`sidebar` gets its own `dx-sidebar-sr-only` copy) --
// see `style.css`'s comment on that rule and `scripts/check-dx-class-prefix.sh`.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum PaginationLinkSize {
    #[default]
    Icon,
    Default,
}

impl PaginationLinkSize {
    pub fn class(&self) -> &'static str {
        match self {
            PaginationLinkSize::Icon => "icon",
            PaginationLinkSize::Default => "default",
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum PaginationLinkKind {
    Previous,
    Next,
}

impl PaginationLinkKind {
    pub fn attr(&self) -> &'static str {
        match self {
            PaginationLinkKind::Previous => "previous",
            PaginationLinkKind::Next => "next",
        }
    }
}

#[component]
pub fn Pagination(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        nav {
            class: "dx-pagination",
            "data-slot": "pagination",
            role: "navigation",
            aria_label: "pagination",
            ..attributes,
            {children}
        }
    }
}

#[component]
pub fn PaginationContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        ul {
            class: "dx-pagination-content",
            "data-slot": "pagination-content",
            ..attributes,
            {children}
        }
    }
}

#[component]
pub fn PaginationItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        li {
            "data-slot": "pagination-item",
            ..attributes,
            {children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct PaginationLinkProps {
    #[props(default)]
    pub is_active: bool,
    #[props(default)]
    pub size: PaginationLinkSize,
    #[props(default)]
    pub data_kind: Option<PaginationLinkKind>,
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

#[component]
pub fn PaginationLink(props: PaginationLinkProps) -> Element {
    let aria_current = if props.is_active { Some("page") } else { None };
    let data_kind = props.data_kind.map(|kind| kind.attr());
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        a {
            class: "dx-pagination-link",
            "data-slot": "pagination-link",
            "data-active": props.is_active,
            "data-size": props.size.class(),
            "data-kind": data_kind,
            aria_current: aria_current,
            onclick: move |event| {
                if let Some(f) = &props.onclick {
                    f.call(event);
                }
            },
            onmousedown: move |event| {
                if let Some(f) = &props.onmousedown {
                    f.call(event);
                }
            },
            onmouseup: move |event| {
                if let Some(f) = &props.onmouseup {
                    f.call(event);
                }
            },
            ..props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn PaginationPrevious(
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        PaginationLink {
            size: PaginationLinkSize::Default,
            aria_label: "Go to previous page",
            data_kind: Some(PaginationLinkKind::Previous),
            onclick,
            onmousedown,
            onmouseup,
            attributes,
            ChevronLeft { size: "1rem" }
            span { class: "dx-pagination-label", "Previous" }
        }
    }
}

#[component]
pub fn PaginationNext(
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        PaginationLink {
            size: PaginationLinkSize::Default,
            aria_label: "Go to next page",
            data_kind: Some(PaginationLinkKind::Next),
            onclick,
            onmousedown,
            onmouseup,
            attributes,
            span { class: "dx-pagination-label", "Next" }
            ChevronRight { size: "1rem" }
        }
    }
}

#[component]
pub fn PaginationEllipsis(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/pagination/style.css") }
        span {
            class: "dx-pagination-ellipsis",
            "data-slot": "pagination-ellipsis",
            aria_hidden: "true",
            ..attributes,
            Ellipsis { size: "1rem" }
            span { class: "dx-pagination-sr-only", "More pages" }
        }
    }
}
