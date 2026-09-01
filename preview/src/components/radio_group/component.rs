use dioxus::prelude::*;
use dioxus_primitives::radio_group::{self, RadioGroupProps, RadioItemProps};

#[css_module("/src/components/radio_group/style.css")]
struct Styles;

#[component]
pub fn RadioGroup(props: RadioGroupProps) -> Element {
    rsx! {
        radio_group::RadioGroup {
            class: Styles::dx_radio_group,
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn RadioItem(props: RadioItemProps) -> Element {
    // Item 1 fix (2026-09-01, live-site report): this wrapper used to drop
    // `props.id` entirely (never forwarded to the underlying primitive) and
    // clobber `props.class` outright instead of merging it -- harmless for
    // every pre-existing caller (none set either), but a real bug surfaced
    // by `preview/src/components/form/component.rs` switching to this
    // themed component for its own root-cause fix: its fixture relies on
    // stable ids (`#plan-lib-pro` etc.) for both its `<label for>`
    // associations and this repo's Playwright oracles to find each radio
    // button. Merges `class` the same way `../popover/component.rs`'s
    // `PopoverContent` does, for the same reason: a caller's own class
    // should extend, not replace, this theme's.
    let class = if let Some(class) = props.class {
        format!("{} {}", Styles::dx_radio_item, class)
    } else {
        Styles::dx_radio_item.to_string()
    };

    rsx! {
        radio_group::RadioItem {
            id: props.id,
            class,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}
