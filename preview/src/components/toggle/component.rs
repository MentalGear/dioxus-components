use dioxus::prelude::*;
use dioxus_primitives::toggle::{self, ToggleProps};

#[component]
pub fn Toggle(props: ToggleProps) -> Element {
    // Merges `class` the same way `../popover/component.rs`'s
    // `PopoverContent` and `../radio_group/component.rs`'s `RadioItem` do:
    // a caller's own class should extend, not replace, this theme's
    // `"dx-toggle"` -- see commit 724bfee ("merge the caller's class
    // instead of clobbering") for the clobber bug this shape avoids.
    let class = if let Some(class) = props.class {
        format!("{} {}", "dx-toggle", class)
    } else {
        "dx-toggle".to_string()
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toggle/style.css") }
        toggle::Toggle {
            class,
            pressed: props.pressed,
            default_pressed: props.default_pressed,
            disabled: props.disabled,
            on_pressed_change: props.on_pressed_change,
            onmounted: props.onmounted,
            onfocus: props.onfocus,
            onkeydown: props.onkeydown,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn ToggleWithCallerClass() -> Element {
        rsx! {
            Toggle {
                pressed: None,
                class: "caller-class".to_string(),
                "B"
            }
        }
    }

    #[test]
    fn caller_class_and_theme_class_both_render_on_the_button() {
        let mut dom = VirtualDom::new(ToggleWithCallerClass);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        // A caller's own class must extend, not replace, the theme's
        // "dx-toggle" -- see commit 724bfee.
        assert!(html.contains(r#"class="dx-toggle caller-class""#));
    }

    #[test]
    fn theme_class_alone_renders_when_caller_sets_no_class() {
        let mut dom = VirtualDom::new(|| {
            rsx! {
                Toggle { pressed: None, "B" }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(r#"class="dx-toggle""#));
    }
}
