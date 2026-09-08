use crate::components::input::Input;
use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// Which edge of the group an [`InputGroupAddon`] sits against.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum InputGroupAlign {
    #[default]
    Start,
    End,
}

impl InputGroupAlign {
    pub fn class(&self) -> &'static str {
        match self {
            InputGroupAlign::Start => "start",
            InputGroupAlign::End => "end",
        }
    }
}

/// Wraps an [`InputGroupInput`] with leading/trailing
/// [`InputGroupAddon`]s (icons, buttons, static text) inside one bordered
/// control -- e.g. a search field with a leading search icon, or an amount
/// field with a trailing "USD" chip. Plain layout, no ARIA widget role, no
/// primitive underneath.
#[component]
pub fn InputGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_group/style.css") }
        div { class: "dx-input-group", ..attributes, {children} }
    }
}

/// The group's actual `<input>`, composing the themed
/// [`Input`](crate::components::input::Input) so the group inherits its
/// event wiring and focus/placeholder styling; its own border/background
/// are flattened by this component's stylesheet so only the group's outer
/// edge shows a border.
#[component]
pub fn InputGroupInput(
    oninput: Option<EventHandler<FormEvent>>,
    onchange: Option<EventHandler<FormEvent>>,
    onfocus: Option<EventHandler<FocusEvent>>,
    onblur: Option<EventHandler<FocusEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = input)]
    attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(input {
        class: "dx-input-group-control",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_group/style.css") }
        Input {
            oninput,
            onchange,
            onfocus,
            onblur,
            attributes: merged,
        }
    }
}

/// A leading or trailing slot inside an [`InputGroup`] -- icons, buttons,
/// or static text that sit flush with the input's border.
#[component]
pub fn InputGroupAddon(
    #[props(default)] align: InputGroupAlign,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_group/style.css") }
        div {
            class: "dx-input-group-addon",
            "data-align": align.class(),
            ..attributes,
            {children}
        }
    }
}

/// Static, non-interactive text inside an [`InputGroupAddon`] -- e.g. a
/// currency symbol or unit.
#[component]
pub fn InputGroupText(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_group/style.css") }
        span { class: "dx-input-group-text", ..attributes, {children} }
    }
}
