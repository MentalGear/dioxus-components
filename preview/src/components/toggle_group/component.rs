use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    toggle_group::{self, ToggleGroupProps, ToggleItemProps},
};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in both exported entry points so a
// `dx components add toggle_group`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane: its
// item class used to be the bare `dx-toggle-item`, not
// `dx-toggle-group-item`. `#[css_module]`'s hash kept it collision-safe
// regardless, but a plain `dx-toggle-item` isn't provably this component's
// own once the hash is gone. Renamed to `dx-toggle-group-item` in the same
// change that drops the macro, in both this file and `style.css` -- see
// `scripts/check-dx-class-prefix.sh`.
#[component]
pub fn ToggleGroup(props: ToggleGroupProps) -> Element {
    // `toggle_group::ToggleGroup` has no named `class` prop of its own (see
    // `primitives/src/toggle_group.rs`), so unlike `Toggle`/`RadioItem` this
    // theme class and a caller-supplied class both have to travel through
    // the shared `attributes` bucket -- a literal `class: "dx-toggle-group"`
    // shorthand here alongside an explicit `attributes: props.attributes`
    // field would hit the exact same clobber-not-merge bug commit 724bfee
    // fixed for `Input`, so this merges them the same way
    // `../popover/component.rs`'s `PopoverRoot` does.
    let base = attributes!(div { class: "dx-toggle-group" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toggle_group/style.css") }
        toggle_group::ToggleGroup {
            default_pressed: props.default_pressed,
            pressed: props.pressed,
            on_pressed_change: props.on_pressed_change,
            disabled: props.disabled,
            allow_multiple_pressed: props.allow_multiple_pressed,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ToggleItem(props: ToggleItemProps) -> Element {
    // Same reasoning as `ToggleGroup` above: `toggle_group::ToggleItem` has
    // no named `class` prop either, so the theme class and a caller class
    // are merged through `attributes` rather than risking one clobbering
    // the other.
    let base = attributes!(button { class: "dx-toggle-group-item" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toggle_group/style.css") }
        toggle_group::ToggleItem {
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn GroupWithCallerClasses() -> Element {
        rsx! {
            ToggleGroup {
                pressed: None,
                class: "caller-group-class",
                ToggleItem {
                    index: 0usize,
                    class: "caller-item-class",
                    "B"
                }
            }
        }
    }

    #[test]
    fn caller_class_and_theme_class_both_render() {
        let mut dom = VirtualDom::new(GroupWithCallerClasses);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        // A caller's own class must extend, not replace, the theme's
        // "dx-toggle-group"/"dx-toggle-group-item" -- see commit 724bfee.
        assert!(html.contains(r#"class="dx-toggle-group caller-group-class""#));
        assert!(html.contains(r#"class="dx-toggle-group-item caller-item-class""#));
    }

    #[test]
    fn theme_classes_alone_render_when_caller_sets_no_class() {
        let mut dom = VirtualDom::new(|| {
            rsx! {
                ToggleGroup {
                    pressed: None,
                    ToggleItem { index: 0usize, "B" }
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(r#"class="dx-toggle-group""#));
        assert!(html.contains(r#"class="dx-toggle-group-item""#));
    }
}
