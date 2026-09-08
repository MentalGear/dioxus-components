use crate::components::label::Label;
use crate::components::separator::Separator;
use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// How a [`Field`]'s label/description stack against its control.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum FieldOrientation {
    /// Label above the control (the default -- a standard labeled input).
    #[default]
    Vertical,
    /// Control beside the label (a checkbox/switch/radio row, label and
    /// description to its side).
    Horizontal,
    /// Vertical on narrow layouts, horizontal from `640px` up.
    Responsive,
}

impl FieldOrientation {
    pub fn class(&self) -> &'static str {
        match self {
            FieldOrientation::Vertical => "vertical",
            FieldOrientation::Horizontal => "horizontal",
            FieldOrientation::Responsive => "responsive",
        }
    }
}

/// One labeled form field -- a label, its control, and optional
/// description/error text, laid out together. Builds on this repo's landed
/// native form participation (every themed control already mirrors its
/// state onto a real, hidden native input/select, see
/// `docs/preview-composition.md`): `Field` only supplies the *layout* and
/// label/description/error wiring around whatever control (`Input`,
/// `Checkbox`, `NativeSelect`, ...) it wraps, so that control's constraint
/// validation and `FormData` participation keep working unchanged. Plain
/// layout, no ARIA widget role of its own, so no primitive underneath.
#[component]
pub fn Field(
    #[props(default)] orientation: FieldOrientation,
    /// Marks the field's control as failing validation -- styles the
    /// label/control and is meant to be paired with a [`FieldError`].
    #[props(default)]
    invalid: bool,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        div {
            class: "dx-field",
            role: "group",
            "data-orientation": orientation.class(),
            "data-invalid": invalid,
            ..attributes,
            {children}
        }
    }
}

/// Groups several [`Field`]s (and optional [`FieldSeparator`]s) with
/// consistent spacing between them.
#[component]
pub fn FieldGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        div { class: "dx-field-group", ..attributes, {children} }
    }
}

/// A native `<fieldset>` wrapping a related group of [`Field`]s, paired
/// with a [`FieldLegend`] -- use this instead of `FieldGroup` when the
/// group itself needs an accessible group name (radio sets, a shared
/// "Notifications" section).
#[component]
pub fn FieldSet(
    #[props(extends = GlobalAttributes)]
    #[props(extends = fieldset)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        fieldset { class: "dx-field-set", ..attributes, {children} }
    }
}

/// The accessible group name for a [`FieldSet`]'s native `<legend>`.
#[component]
pub fn FieldLegend(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        legend { class: "dx-field-legend", ..attributes, {children} }
    }
}

/// A [`Field`]'s label. Composes the themed
/// [`Label`](crate::components::label::Label) -- which already renders the
/// real `<label>` element with a properly wired `html_for` -- so `Field`
/// never has to reimplement label/control association.
#[component]
pub fn FieldLabel(
    html_for: ReadSignal<String>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(label {
        class: "dx-field-label",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        Label { html_for, attributes: merged, {children} }
    }
}

/// Wraps a [`FieldLabel`] + [`FieldDescription`] pair beside a control in a
/// `FieldOrientation::Horizontal` field (e.g. a checkbox's label and
/// helper text, stacked next to the checkbox itself).
#[component]
pub fn FieldContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        div { class: "dx-field-content", ..attributes, {children} }
    }
}

/// Supporting helper text under a [`Field`]'s control.
#[component]
pub fn FieldDescription(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        p { class: "dx-field-description", ..attributes, {children} }
    }
}

/// A validation message for a [`Field`] with `invalid: true`. Renders
/// `role="alert"` so the message is announced when it appears.
#[component]
pub fn FieldError(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        div { class: "dx-field-error", role: "alert", ..attributes, {children} }
    }
}

/// A horizontal rule between two [`Field`]s/[`FieldGroup`]s, with an
/// optional centered label (e.g. "OR"). Composes the themed
/// [`Separator`](crate::components::separator::Separator).
#[component]
pub fn FieldSeparator(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    #[props(default)] children: Option<Element>,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/field/style.css") }
        div { class: "dx-field-separator", ..attributes,
            Separator { horizontal: true, decorative: true }
            if let Some(children) = &children {
                span { class: "dx-field-separator-label", {children.clone()} }
            }
        }
    }
}
