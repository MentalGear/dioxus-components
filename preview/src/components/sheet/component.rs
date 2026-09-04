use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::dialog::{
    self, DialogCtx, DialogDescriptionProps, DialogRootProps, DialogTitleProps,
};
use dioxus_primitives::merge_attributes;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

impl SheetSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            SheetSide::Top => "top",
            SheetSide::Right => "right",
            SheetSide::Bottom => "bottom",
            SheetSide::Left => "left",
        }
    }
}

#[component]
pub fn Sheet(props: DialogRootProps) -> Element {
    let content_base = attributes!(div {
        class: "dx-sheet",
        "data-slot": "sheet-content",
        "data-side": SheetSide::Right.as_str(),
    });
    let content_attributes = merge_attributes(vec![content_base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        dialog::DialogRoot {
            class: "dx-sheet-root",
            "data-slot": "sheet-root",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            dialog::DialogContent {
                class: None,
                attributes: content_attributes,
                {props.children}
            }
        }
    }
}

#[component]
pub fn SheetContentClose(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    // axe `button-name` (docs/backlog.md row 34's own round): this button's
    // only content is the `X` icon, with no text and no accessible name --
    // mirrors the fix already applied per-call-site for `DialogClose`/
    // `AlertDialogClose` (`dialog/variants/main/mod.rs`'s `aria_label:
    // "Close"`), baked in here instead since both current call sites
    // (`sheet/variants/main/mod.rs`, `sidebar/component.rs`) render this
    // shared wrapper with no children of their own to derive a name from.
    let base = attributes!(button {
        class: "dx-sheet-close",
        aria_label: "Close",
    });
    let attributes = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        SheetClose { attributes,
            X { size: "20px" }
        }
    }
}

#[component]
pub fn SheetHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        div { class: "dx-sheet-header", "data-slot": "sheet-header", ..attributes, {children} }
    }
}

#[component]
pub fn SheetFooter(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        div { class: "dx-sheet-footer", "data-slot": "sheet-footer", ..attributes, {children} }
    }
}

#[component]
pub fn SheetTitle(props: DialogTitleProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        dialog::DialogTitle {
            id: props.id,
            class: "dx-sheet-title",
            "data-slot": "sheet-title",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn SheetDescription(props: DialogDescriptionProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sheet/style.css") }
        dialog::DialogDescription {
            id: props.id,
            class: "dx-sheet-description",
            "data-slot": "sheet-description",
            attributes: props.attributes,
            {props.children}
        }
    }
}

// No `document::Link` here (unlike this file's other exported components):
// `SheetClose` reads `DialogCtx` via `use_context()` below, so it can only
// ever render as a descendant of a `dialog::DialogRoot` -- and in this file
// that context is provided by `Sheet` alone, which already links the sheet
// stylesheet. A context lookup failure would panic before this component
// could render unstyled, so there's no code path where `SheetClose` reaches
// the DOM without `Sheet`'s own `Link` already in the document head. It also
// has no single `rsx!` block both branches share (the `r#as` branch returns
// straight from the caller's own `Callback`), so adding a `Link` here would
// mean wrapping each branch individually -- unlike this file's other parts,
// that's more than a mechanical one-line insertion for a guarantee the
// context dependency already gives for free.
#[component]
pub fn SheetClose(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let ctx: DialogCtx = use_context();

    let base = attributes! {
        button {
            onclick: move |_| {
                ctx.set_open(false);
            }
        }
    };
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            button { ..merged, {children} }
        }
    }
}
