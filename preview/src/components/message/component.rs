use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// Which side of the conversation a [`Message`] row sits on.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum MessageAlign {
    /// The receiver's side: avatar first, content after it.
    #[default]
    Start,
    /// The sender's side: the row is mirrored.
    End,
}

impl MessageAlign {
    pub fn class(&self) -> &'static str {
        match self {
            MessageAlign::Start => "start",
            MessageAlign::End => "end",
        }
    }
}

/// Stacks consecutive messages from the same sender. Render an empty
/// [`MessageAvatar`] on the earlier messages to keep them aligned with the
/// avatar on the last one.
#[component]
pub fn MessageGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message-group",
        "data-slot": "message-group",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}

/// One message row: it lays out the avatar, alignment, header and footer
/// around the message surface (a `Bubble`, or any content). Pure flex layout
/// with no ARIA role, so there is no primitive underneath -- the labelling
/// burden is on what you put inside (icon-only footer actions need an
/// `aria-label`; an in-progress state is a `Marker` with `role: "status"`).
#[component]
pub fn Message(
    #[props(default)] align: MessageAlign,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message",
        "data-slot": "message",
    });
    // `data-align` reflects this wrapper's own typed `align` prop, not a caller
    // default -- owned-wins.
    let owned = attributes!(div { "data-align": align.class() });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}

/// The avatar slot, anchored to the bottom of the row. When the message has a
/// [`MessageFooter`] the avatar shifts up to stay level with the message
/// surface instead of the footer. Leave it empty to reserve the avatar's
/// width.
#[component]
pub fn MessageAvatar(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message-avatar",
        "data-slot": "message-avatar",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}

/// Wraps the header, the message surface and the footer.
#[component]
pub fn MessageContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message-content",
        "data-slot": "message-content",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}

/// Content above the message surface, such as the sender's name.
#[component]
pub fn MessageHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message-header",
        "data-slot": "message-header",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}

/// Content below the message surface, such as delivery status or message-level
/// actions (copy, retry, feedback). Follows the message's side, so actions stay
/// at the end of an `End` row. Icon-only actions need an `aria-label`.
#[component]
pub fn MessageFooter(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-message-footer",
        "data-slot": "message-footer",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message/style.css") }
        div { ..merged, {children} }
    }
}
