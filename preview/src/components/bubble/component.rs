use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// The visual treatment of a [`Bubble`].
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum BubbleVariant {
    /// A strong primary bubble, usually for the current user.
    #[default]
    Default,
    /// The standard neutral bubble for conversation content.
    Secondary,
    /// A lower-emphasis bubble for quiet supporting content.
    Muted,
    /// A subtle primary-tinted bubble.
    Tinted,
    /// A bordered bubble for secondary or rich content.
    Outline,
    /// Unframed content for assistant text or rich content; spans the full row.
    Ghost,
    /// A destructive bubble for errors or failed actions.
    Destructive,
}

impl BubbleVariant {
    pub fn class(&self) -> &'static str {
        match self {
            BubbleVariant::Default => "default",
            BubbleVariant::Secondary => "secondary",
            BubbleVariant::Muted => "muted",
            BubbleVariant::Tinted => "tinted",
            BubbleVariant::Outline => "outline",
            BubbleVariant::Ghost => "ghost",
            BubbleVariant::Destructive => "destructive",
        }
    }
}

/// The inline alignment of a [`Bubble`] (or of its [`BubbleReactions`]).
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum BubbleAlign {
    #[default]
    Start,
    End,
}

impl BubbleAlign {
    pub fn class(&self) -> &'static str {
        match self {
            BubbleAlign::Start => "start",
            BubbleAlign::End => "end",
        }
    }
}

/// Which edge of the bubble its [`BubbleReactions`] row overlaps.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum BubbleSide {
    Top,
    #[default]
    Bottom,
}

impl BubbleSide {
    pub fn class(&self) -> &'static str {
        match self {
            BubbleSide::Top => "top",
            BubbleSide::Bottom => "bottom",
        }
    }
}

/// Stacks consecutive bubbles from the same sender. Set `align` on each
/// [`Bubble`], not on the group.
#[component]
pub fn BubbleGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-bubble-group",
        "data-slot": "bubble-group",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bubble/style.css") }
        div { ..merged, {children} }
    }
}

/// The message surface: framed conversational content that sizes to its text,
/// up to 80% of the row (`Ghost` spans it). Presentational -- it holds no ARIA
/// role, so keep the conversation's semantics on the surrounding container, and
/// pair a variant's tone with text or an icon rather than colour alone.
/// Avatars, names, timestamps and message-level actions belong in `Message`.
#[component]
pub fn Bubble(
    #[props(default)] variant: BubbleVariant,
    #[props(default)] align: BubbleAlign,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-bubble",
        "data-slot": "bubble",
    });
    // `data-variant` / `data-align` reflect this wrapper's own typed props, not
    // caller defaults -- owned-wins.
    let owned = attributes!(div {
        "data-variant": variant.class(),
        "data-align": align.class(),
    });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bubble/style.css") }
        div { ..merged, {children} }
    }
}

/// The bubble's content. To turn the bubble into a real link or button (so it
/// is focusable and exposes the right role), pass `as` and render the element
/// yourself with the text inside it; its accessible name is that text.
#[component]
pub fn BubbleContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-bubble-content",
        "data-slot": "bubble-content",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bubble/style.css") }
        if let Some(dynamic) = r#as {
            {dynamic.call(merged)}
        } else {
            div { ..merged, {children} }
        }
    }
}

/// Reactions or quick-action buttons that overlap one edge of the bubble.
/// A static row of emoji needs `role: "img"` and an `aria-label` that names
/// them, so it is announced once; interactive reactions are real buttons, each
/// with its own `aria-label` when icon-only. Reactions overlap the bubble, so
/// leave vertical room between rows.
#[component]
pub fn BubbleReactions(
    #[props(default)] side: BubbleSide,
    #[props(default = BubbleAlign::End)] align: BubbleAlign,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-bubble-reactions",
        "data-slot": "bubble-reactions",
    });
    // Placement reflects this wrapper's own typed props -- owned-wins.
    let owned = attributes!(div {
        "data-side": side.class(),
        "data-align": align.class(),
    });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bubble/style.css") }
        div { ..merged, {children} }
    }
}
