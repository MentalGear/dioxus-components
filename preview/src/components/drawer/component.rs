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
    let content_base = attributes!(div {
        class: "dx-drawer",
        "data-slot": "drawer-content",
    });
    let content_attributes = merge_attributes(vec![content_base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drawer/style.css") }
        drawer::DrawerContent {
            id: props.id,
            class: None,
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
    let base = attributes!(div { class: "dx-drawer-description", "data-slot": "drawer-description" });
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
