use dioxus::prelude::*;
use dioxus_primitives::radio_group::{self, RadioGroupProps, RadioItemProps};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in both exported entry points so a
// `dx components add radio_group`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane: its
// item class used to be the bare `dx-radio-item`, not `dx-radio-group-item`.
// `#[css_module]`'s hash kept it collision-safe regardless, but a plain
// `dx-radio-item` isn't provably this component's own once the hash is
// gone. Renamed to `dx-radio-group-item` in the same change that drops the
// macro, in both this file and `style.css` -- see
// `scripts/check-dx-class-prefix.sh`.
#[component]
pub fn RadioGroup(props: RadioGroupProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/radio_group/style.css") }
        radio_group::RadioGroup {
            class: "dx-radio-group",
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
        format!("{} {}", "dx-radio-group-item", class)
    } else {
        "dx-radio-group-item".to_string()
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/radio_group/style.css") }
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
