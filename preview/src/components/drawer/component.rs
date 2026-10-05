use dioxus::prelude::*;
use dioxus_primitives::dialog::{DialogDescriptionProps, DialogTitleProps};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::drawer::{
    self, DrawerCloseProps, DrawerContentProps, DrawerHandleProps, DrawerRootProps,
};
use dioxus_primitives::merge_attributes;

pub use dioxus_primitives::drawer::DrawerSide;

#[component]
pub fn Drawer(props: DrawerRootProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::Drawer {
            class: "dx-drawer-root",
            "data-slot": "drawer-root",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            side: props.side,
            dismissible: props.dismissible,
            {props.children}
        }
    }
}

#[component]
pub fn DrawerContent(props: DrawerContentProps) -> Element {
    // Merges `class` the same way `../popover/component.rs`'s `PopoverContent`
    // and `../toggle/component.rs`'s `Toggle` do: a caller's own class extends
    // this theme's `"dx-drawer"`, it does not replace it (this wrapper used to
    // pass `class: None` and drop `props.class` on the floor). It travels as the
    // primitive's typed `class` prop, so it is ONE literal on the dialog element
    // and never rides beside the `..attributes` spread
    // (`scripts/check-attr-spread-collision.sh`).
    let content_class = if let Some(class) = props.class {
        format!("{} {}", "dx-drawer", class)
    } else {
        "dx-drawer".to_string()
    };
    let content_base = attributes!(div {
        "data-slot": "drawer-content",
    });
    let content_attributes = merge_attributes(vec![content_base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::DrawerContent {
            id: props.id,
            class: content_class,
            attributes: content_attributes,
            {props.children}
        }
    }
}

#[component]
pub fn DrawerHandle(props: DrawerHandleProps) -> Element {
    let base = attributes!(div {
        class: "dx-drawer-handle",
        "data-slot": "drawer-handle",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::DrawerHandle { attributes: merged }
    }
}

#[component]
pub fn DrawerHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-drawer-header", "data-slot": "drawer-header" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn DrawerFooter(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-drawer-footer", "data-slot": "drawer-footer" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn DrawerTitle(props: DialogTitleProps) -> Element {
    let base = attributes!(div { class: "dx-drawer-title", "data-slot": "drawer-title" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::DrawerTitle { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DrawerDescription(props: DialogDescriptionProps) -> Element {
    let base =
        attributes!(div { class: "dx-drawer-description", "data-slot": "drawer-description" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::DrawerDescription { id: props.id, attributes: merged, {props.children} }
    }
}

// No `document::Link` here (unlike this file's other exported components):
// `DrawerClose` reads `DialogCtx` via `use_context()` (inside the primitive
// it wraps), so it can only ever render as a descendant of a `Drawer` --
// and in this file that context is provided by `Drawer` alone, which
// already links the drawer stylesheet. Mirrors
// `preview/src/components/sheet/component.rs`'s `SheetClose`'s identical
// reasoning.
#[component]
pub fn DrawerClose(props: DrawerCloseProps) -> Element {
    let base = attributes!(button {
        class: "dx-drawer-close",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        drawer::DrawerClose { attributes: merged, r#as: props.r#as, {props.children} }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn DrawerWithCallerClass() -> Element {
        rsx! {
            Drawer {
                default_open: true,
                DrawerContent { class: "caller-class".to_string(), "body" }
            }
        }
    }

    #[component]
    fn DrawerWithoutCallerClass() -> Element {
        rsx! {
            Drawer {
                default_open: true,
                DrawerContent { "body" }
            }
        }
    }

    /// Renders `app` and lets the primitive's open effect (`use_animated_open`) run, which is
    /// what puts the drawer's content in the DOM: it is absent from the first pass.
    fn render_open(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        // First pass: the effect flips `show_in_dom`. Second: the content renders.
        dom.process_events();
        dom.render_immediate_to_vec();
        dioxus_ssr::render(&dom)
    }

    /// The `class` forwarding regression: the themed wrapper passed `class: None`
    /// to the primitive, so a caller's own class never reached the element.
    #[test]
    fn caller_class_and_theme_class_both_render_on_the_drawer_content() {
        let html = render_open(DrawerWithCallerClass);

        assert!(
            html.contains(r#"class="dx-drawer caller-class""#),
            "a caller's class must extend, not replace, the theme's `dx-drawer`: {html}"
        );
    }

    #[test]
    fn theme_class_alone_renders_when_caller_sets_no_class() {
        let html = render_open(DrawerWithoutCallerClass);

        assert!(html.contains(r#"class="dx-drawer""#), "{html}");
        // The primitive's own `dx-dialog` fallback must not leak in beside it.
        assert!(!html.contains("dx-dialog"), "{html}");
    }
}
