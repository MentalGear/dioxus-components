use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_primitives::activity::use_motion_when;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// The upload lifecycle of an [`Attachment`]. `Uploading` and `Processing`
/// shimmer the title, `Error` switches to a destructive treatment, `Idle`
/// draws a dashed border.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AttachmentState {
    Idle,
    Uploading,
    Processing,
    Error,
    #[default]
    Done,
}

impl AttachmentState {
    pub fn class(&self) -> &'static str {
        match self {
            AttachmentState::Idle => "idle",
            AttachmentState::Uploading => "uploading",
            AttachmentState::Processing => "processing",
            AttachmentState::Error => "error",
            AttachmentState::Done => "done",
        }
    }

    /// Whether work is still in flight, which is what makes the title shimmer.
    fn is_busy(&self) -> bool {
        matches!(
            self,
            AttachmentState::Uploading | AttachmentState::Processing
        )
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AttachmentSize {
    #[default]
    Default,
    Sm,
    Xs,
}

impl AttachmentSize {
    pub fn class(&self) -> &'static str {
        match self {
            AttachmentSize::Default => "default",
            AttachmentSize::Sm => "sm",
            AttachmentSize::Xs => "xs",
        }
    }
}

/// Whether the media sits beside (`Horizontal`) or above (`Vertical`) the
/// content.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AttachmentOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl AttachmentOrientation {
    pub fn class(&self) -> &'static str {
        match self {
            AttachmentOrientation::Horizontal => "horizontal",
            AttachmentOrientation::Vertical => "vertical",
        }
    }
}

/// What an [`AttachmentMedia`] holds: an icon, or an `<img>` preview.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AttachmentMediaVariant {
    #[default]
    Icon,
    Image,
}

impl AttachmentMediaVariant {
    pub fn class(&self) -> &'static str {
        match self {
            AttachmentMediaVariant::Icon => "icon",
            AttachmentMediaVariant::Image => "image",
        }
    }
}

/// The parent [`Attachment`]'s state, shared with its title so it can shimmer
/// while the file is in flight. A `ReadSignal`, so a state change on the
/// attachment reaches the title without remounting it.
#[derive(Clone, Copy)]
struct AttachmentContext {
    state: ReadSignal<AttachmentState>,
}

/// A file or image attachment: its media, name and metadata, with optional
/// actions and an upload state. Used for files and images in chat composers,
/// message threads and upload lists. Styled composition with no ARIA widget
/// role, so there is no primitive underneath; it composes the existing
/// `Button` for its actions.
#[component]
pub fn Attachment(
    #[props(default)] state: ReadSignal<AttachmentState>,
    #[props(default)] size: AttachmentSize,
    #[props(default)] orientation: AttachmentOrientation,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    use_context_provider(|| AttachmentContext { state });
    let base = attributes!(div {
        class: "dx-attachment",
        "data-slot": "attachment",
    });
    // `data-state` / `data-size` / `data-orientation` reflect this wrapper's own
    // typed props, not caller defaults -- owned-wins.
    let owned = attributes!(div {
        "data-state": state().class(),
        "data-size": size.class(),
        "data-orientation": orientation.class(),
    });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        div { ..merged, {children} }
    }
}

/// The media slot: an icon, or (with `variant: AttachmentMediaVariant::Image`)
/// an `<img>` preview. Give the image an `alt`; a decorative icon is hidden by
/// the icon set itself.
#[component]
pub fn AttachmentMedia(
    #[props(default)] variant: AttachmentMediaVariant,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-attachment-media",
        "data-slot": "attachment-media",
    });
    // Reflects this wrapper's own typed `variant` prop -- owned-wins.
    let owned = attributes!(div { "data-variant": variant.class() });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        div { ..merged, {children} }
    }
}

/// Wraps the title and description.
#[component]
pub fn AttachmentContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-attachment-content",
        "data-slot": "attachment-content",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        div { ..merged, {children} }
    }
}

/// The attachment's name. Shimmers while the attachment is `Uploading` or
/// `Processing` (the `dx-shimmer` utility from `assets/dx-effects.css`).
#[component]
pub fn AttachmentTitle(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx = try_use_context::<AttachmentContext>();
    let busy = ctx.is_some_and(|ctx| (ctx.state)().is_busy());
    // The sweep is paused while the title is off-screen or in a hidden tab, and observed only while busy.
    let motion = use_motion_when(move || ctx.is_some_and(|ctx| (ctx.state)().is_busy()));
    let base = attributes!(span {
        class: if busy { "dx-attachment-title dx-shimmer" } else { "dx-attachment-title" },
        "data-slot": "attachment-title",
    });
    let merged = merge_attributes(vec![base, attributes, motion.attributes()]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        span { ..merged, {children} }
    }
}

/// Secondary metadata such as the file type, size or upload status. In the
/// `Error` state keep the failure reason here in text, so the state is not
/// conveyed by colour alone.
#[component]
pub fn AttachmentDescription(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(span {
        class: "dx-attachment-description",
        "data-slot": "attachment-description",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        span { ..merged, {children} }
    }
}

/// A container for one or more [`AttachmentAction`]s, aligned to the end of
/// the attachment (the top corner when vertical).
#[component]
pub fn AttachmentActions(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-attachment-actions",
        "data-slot": "attachment-actions",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        div { ..merged, {children} }
    }
}

/// An action button: a themed `Button` that defaults to the ghost variant and
/// the `IconXs` size. Usually icon-only, so give each one an `aria-label`
/// naming the action and its target ("Remove sales-dashboard.pdf").
#[component]
pub fn AttachmentAction(
    #[props(default = ButtonVariant::Ghost)] variant: ButtonVariant,
    #[props(default = ButtonSize::IconXs)] size: ButtonSize,
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let base = attributes!(button {
        class: "dx-attachment-action",
        "data-slot": "attachment-action",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        Button {
            variant,
            size,
            onclick: move |event| {
                if let Some(f) = &onclick {
                    f.call(event);
                }
            },
            attributes: merged,
            {children}
        }
    }
}

/// A full-card overlay that activates the attachment: a real `<button>` by
/// default, or (through `as`) a link. It sits behind the actions in the
/// stacking order, so a trigger and the actions never trap each other. It has
/// no text of its own, so give it an `aria-label` for what activating it does.
#[component]
pub fn AttachmentTrigger(
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
    onclick: Option<EventHandler<MouseEvent>>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let base = attributes!(button {
        class: "dx-attachment-trigger",
        "data-slot": "attachment-trigger",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        if let Some(dynamic) = r#as {
            {dynamic.call(merged)}
        } else {
            // `type` is an overridable default, and only for the real `<button>`
            // (an `as` link must not get one).
            button {
                onclick: move |event| {
                    if let Some(f) = &onclick {
                        f.call(event);
                    }
                },
                ..merge_attributes(vec![attributes!(button { r#type: "button" }), merged]),
                {children}
            }
        }
    }
}

/// Lays attachments out in a horizontally scrollable, snapping row with an
/// edge fade (`dx-scroll-fade-x`). When its attachments are interactive (a
/// trigger or actions), keyboard users reach the off-screen ones by tabbing.
/// For a row of presentational attachments, make the group itself focusable:
/// `tabindex: "0"`, `role: "group"` and an `aria-label`.
#[component]
pub fn AttachmentGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-attachment-group dx-scroll-fade-x",
        "data-slot": "attachment-group",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/attachment/style.css") }
        div { ..merged, {children} }
    }
}
