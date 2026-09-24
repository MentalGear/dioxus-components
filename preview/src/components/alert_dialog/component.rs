use dioxus::prelude::*;
use dioxus_primitives::alert_dialog::{
    self, AlertDialogActionProps, AlertDialogActionsProps, AlertDialogCancelProps,
    AlertDialogDescriptionProps, AlertDialogRootProps, AlertDialogTitleProps,
};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

#[component]
pub fn AlertDialog(props: AlertDialogRootProps) -> Element {
    let base = attributes!(div { class: "dx-alert-dialog-backdrop" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogRoot {
            id: props.id,
            default_open: props.default_open,
            open: props.open,
            on_open_change: props.on_open_change,
            attributes: merged,
            alert_dialog::AlertDialogContent {
                class: "dx-alert-dialog".to_string(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn AlertDialogTitle(props: AlertDialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "dx-alert-dialog-title" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogTitle { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogDescription(props: AlertDialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "dx-alert-dialog-description" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogDescription { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogActions(props: AlertDialogActionsProps) -> Element {
    let base = attributes!(div { class: "dx-alert-dialog-actions" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogActions { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogCancel(props: AlertDialogCancelProps) -> Element {
    let base = attributes!(button { class: "dx-alert-dialog-cancel" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogCancel {
            on_click: props.on_click,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn AlertDialogAction(props: AlertDialogActionProps) -> Element {
    let base = attributes!(button { class: "dx-alert-dialog-action" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert_dialog/style.css") }
        alert_dialog::AlertDialogAction {
            on_click: props.on_click,
            attributes: merged,
            {props.children}
        }
    }
}
