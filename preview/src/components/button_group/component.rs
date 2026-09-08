use crate::components::separator::Separator;
use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// Groups a row (or column) of [`Button`](crate::components::button::Button)s
/// so their borders/radii visually merge into one control, matching the
/// shadcn/ui `ButtonGroup`. Plain layout, `role="group"` -- no ARIA widget
/// pattern and no primitive underneath.
#[component]
pub fn ButtonGroup(
    #[props(default)] orientation_vertical: bool,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/button_group/style.css") }
        div {
            class: "dx-button-group",
            role: "group",
            "data-orientation": if orientation_vertical { "vertical" } else { "horizontal" },
            ..attributes,
            {children}
        }
    }
}

/// A visual divider between two clusters of buttons inside a
/// [`ButtonGroup`] -- e.g. separating a primary action from an overflow
/// menu trigger. Composes the themed [`Separator`], oriented opposite the
/// group so it always crosses the group's main axis.
#[component]
pub fn ButtonGroupSeparator(
    #[props(default)] orientation_vertical: bool,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(div {
        class: "dx-button-group-separator",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/button_group/style.css") }
        Separator {
            horizontal: !orientation_vertical,
            decorative: true,
            attributes: merged,
        }
    }
}

/// A static, non-interactive label chip inside a [`ButtonGroup`] -- e.g. a
/// unit ("px", "$") flanking an input-like control.
#[component]
pub fn ButtonGroupText(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/button_group/style.css") }
        div { class: "dx-button-group-text", ..attributes, {children} }
    }
}
