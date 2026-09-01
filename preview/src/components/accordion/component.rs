use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::accordion::{
    self, AccordionContentProps, AccordionItemProps, AccordionProps, AccordionTriggerProps,
};

#[css_module("/src/components/accordion/style.css")]
struct Styles;

#[component]
pub fn Accordion(props: AccordionProps) -> Element {
    rsx! {
        accordion::Accordion {
            class: Styles::dx_accordion,
            width: "15rem",
            id: props.id,
            allow_multiple_open: props.allow_multiple_open,
            disabled: props.disabled,
            collapsible: props.collapsible,
            horizontal: props.horizontal,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn AccordionItem(props: AccordionItemProps) -> Element {
    rsx! {
        accordion::AccordionItem {
            class: Styles::dx_accordion_item,
            disabled: props.disabled,
            default_open: props.default_open,
            on_change: props.on_change,
            on_trigger_click: props.on_trigger_click,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn AccordionTrigger(props: AccordionTriggerProps) -> Element {
    rsx! {
        accordion::AccordionTrigger {
            class: Styles::dx_accordion_trigger,
            id: props.id,
            attributes: props.attributes,
            {props.children}
            ChevronDown {
                class: Styles::dx_accordion_expand_icon,
                size: "20px",
                stroke: "var(--secondary-color-4)",
            }
        }
    }
}

#[component]
pub fn AccordionContent(props: AccordionContentProps) -> Element {
    rsx! {
        accordion::AccordionContent {
            class: Styles::dx_accordion_content,
            style: "--collapsible-content-width: 140px",
            id: props.id,
            attributes: props.attributes,
            // This inner wrapper is the direct (and only) child of the
            // animated grid container, so it is what `.dx-accordion-content
            // > *`'s `overflow: hidden; min-height: 0;` used to target. It
            // deliberately carries no padding/margin of its own, so its box
            // can shrink all the way to 0 when the grid row collapses.
            // Consumer content -- which is free to put whatever padding it
            // wants on itself, as the demo below does -- lives one level
            // deeper, as a grandchild. Its padding no longer sets a floor on
            // how far the animated box can shrink, because that padding
            // belongs to a *different* box than the one being collapsed --
            // one that `overflow: hidden` on this wrapper clips away
            // entirely once the wrapper itself reaches 0. See the
            // close-animation root-cause comment in style.css.
            div { class: Styles::dx_accordion_content_inner, {props.children} }
        }
    }
}
