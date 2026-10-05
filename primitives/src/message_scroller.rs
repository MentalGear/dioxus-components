//! Defines the [`MessageScroller`] family: the scroll behaviour a chat
//! transcript needs, and nothing else.
//!
//! A port of shadcn/ui's `message-scroller` (the `@shadcn/react` headless
//! controller, MIT) per `dev-docs/research/shadcn-catalog-2026-10-04.md`
//! section 4. It owns *scroll*: where the transcript opens, whether it
//! follows a streaming reply, where a new turn lands, keeping the reader's
//! place while older rows are prepended, and a jump-to-latest control. It
//! does not own messages, models, transport or persistence -- the rows are
//! whatever markup the caller puts inside a [`MessageScrollerItem`].
//!
//! # Scope (stage 1)
//!
//! Covered: follow-the-stream while the reader is at the live edge
//! (`auto_scroll`), released by wheel / touch / scroll keys / a scrollbar
//! drag; the jump-to-latest/earliest [`MessageScrollerButton`] (inert while
//! there is nowhere to go); new-turn anchoring with a `scroll_previous_item_peek`
//! and a tail spacer; the opening position ([`DefaultScrollPosition`]) behind
//! a `data-pending-scroll` flash guard; prepend preservation; the commands
//! [`MessageScrollerApi::scroll_to_end`], [`MessageScrollerApi::scroll_to_start`]
//! and [`MessageScrollerApi::scroll_to_message`]; the `data-scrollable` /
//! `data-autoscrolling` styling hooks and [`use_message_scroller_scrollable`].
//!
//! Deferred to stage 2 (not a silent gap): the IntersectionObserver-backed
//! `use_message_scroller_visibility` hook (`currentAnchorId` /
//! `visibleMessageIds`), a per-call `scroll_margin`, an RTL pass over the
//! button placement, a release-build timing spec, and handing the viewport to
//! [`crate::virtual_list`]. See "Virtualization" below for how long
//! transcripts stay cheap.
//!
//! # Architecture: Rust renders the shell, one JS controller owns scrolling
//!
//! Scroll-frequency work cannot round-trip through the framework, so the
//! controller (`message_scroller.js`, one `document::eval` per
//! [`MessageScrollerProvider`]) is **DOM-driven**: it finds the viewport, the
//! content and the rows through `data-message-scroller-*` / `data-message-id`
//! / `data-scroll-anchor` attributes, watches content with a
//! `MutationObserver` and sizes with a `ResizeObserver`, and writes its state
//! back as `data-*` attributes. A streamed token is just a text-node update:
//! the `ResizeObserver` sees the growth and the controller scrolls, with no
//! per-token message in either direction.
//!
//! Rust -> JS messages are rare and cheap: one `init`, a `config` when a
//! provider prop changes, a command (`end`/`start`/`message`), and `sync` when
//! a button mounts. JS -> Rust is **change-only**: one
//! `["scrollable", start, end]` message when the published scrollability
//! changes, which feeds [`use_message_scroller_scrollable`]. While the reader
//! is following the stream `end` is published as `false`, so the jump button
//! does not strobe once per chunk.
//!
//! `data-scrollable`, `data-autoscrolling`, and the jump button's
//! `data-active` / `inert` / `tabindex` are **JS-owned**: they are written by
//! the controller and are deliberately not in the vdom attribute set that a
//! later render could clobber.
//!
//! # Virtualization
//!
//! [`Virtualization`] (the provider's `virtualize` prop) chooses how off-screen
//! rows are kept cheap:
//!
//! - [`Virtualization::None`]: every row is fully rendered all the time.
//! - [`Virtualization::ContentVisibility`] (the default): rows stay in the DOM
//!   and the browser skips laying out and painting the ones that are off-screen
//!   (`content-visibility: auto` with `contain-intrinsic-size: auto <estimate>`,
//!   written by the styled layer keyed on the content's `data-virtualize`
//!   attribute). The accessibility tree and cross-row selection keep working
//!   because the rows are real DOM, and so does native find-in-page where the
//!   engine searches skipped content (Chromium, Firefox 125+, Safari 26+; not
//!   Safari 18-25, WebKit bug 283846). The controller keeps
//!   three groups of rows out of this: the last few rows (the live edge, so the
//!   end geometry is exact while a reply streams), the row holding focus, and
//!   the rows holding the ends of the selection. It marks them with a
//!   controller-owned `data-keep-rendered` attribute, which the stylesheet
//!   excludes from skipping.
//!
//!   Skipping every row separately costs the browser main-thread time
//!   proportional to the row count on every scroll frame (20 ms per frame for
//!   2,000 rows in the long demo), so a long transcript should render its rows
//!   through [`MessageScrollerRows`], which groups them in chunks of 20 and
//!   skips each chunk as a unit (3.7 ms per frame, measured under load). Rows written directly as
//!   children of the content still work and keep the per-row behaviour.
//!
//! Mounting only a window of rows (so unmounted rows would be unreachable by
//! native find-in-page, screen-reader history and cross-message selection)
//! is a possible future option, deliberately not part of this API.
//!
//! # SSR, hydration and the flash guard
//!
//! A scroll container always opens at the top, so a server-rendered transcript
//! would flash its oldest rows and then jump. When the opening position is
//! `End` or `LastAnchor` the root and viewport therefore render
//! `data-pending-scroll` (in the server HTML and again on the hydrating
//! client, from the provider's mount-time props, so the first render is
//! identical on both sides), and the styled viewport is `visibility: hidden`
//! while it is present. The controller removes it as soon as the position is
//! applied; it also removes it for an empty transcript, and after a bounded
//! wait if the viewport never becomes a scroll container. It is mount-only:
//! changing `default_scroll_position` later does not hide the viewport again.
//!
//! # Renderer arms
//!
//! Markup and attribute choice split on the `web` Cargo feature (never
//! `target_family`): with it, a real document runs the controller; without it
//! (Blitz / `dioxus-native`, where `document::eval` is a no-op) the shell
//! renders as plain overflow with no `data-pending-scroll`, no auto-follow,
//! and a permanently inert button, and every command is a no-op.
//!
//! # Accessibility
//!
//! Defaults follow shadcn. The viewport is `role="region"` with
//! `aria-label="Messages"` and `tabindex="0"` (keyboard users can scroll it,
//! and it satisfies axe's `scrollable-region-focusable`); the content is
//! `role="log"` with `aria-relevant="additions"`, so new rows are announced
//! and token-level text changes are not. All three are overridable through the
//! components' `attributes`. The jump control is a real `<button>`; when it
//! turns inert focus moves to the viewport rather than to `<body>`.

use crate::r#virtual::{cv_chunks, ChunkSkip};
#[cfg(feature = "web")]
use crate::use_effect_with_cleanup;
use crate::{merge_attributes, use_unique_id};
use dioxus::document::Eval;
use dioxus::prelude::*;
use dioxus_attributes::attributes;
use serde::Serialize;

/// The controller, run as one `document::eval` per provider. See the module
/// doc for the message protocol.
#[cfg(feature = "web")]
const MESSAGE_SCROLLER_JS: &str = include_str!("message_scroller.js");

/// Where a saved transcript opens on its first non-empty render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultScrollPosition {
    /// The oldest message.
    Start,
    /// The newest message. The default.
    #[default]
    End,
    /// The last row marked `scroll_anchor` (typically the user's latest
    /// message) with its reply below it; falls back to `End` when there is no
    /// anchor or the last turn already fits in the viewport.
    LastAnchor,
}

impl DefaultScrollPosition {
    /// The wire / `data-*` spelling: `"start"`, `"end"` or `"last-anchor"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::LastAnchor => "last-anchor",
        }
    }

    /// Whether the viewport must stay hidden until the controller applies
    /// this position (see the module doc's "flash guard").
    fn holds_pending(self) -> bool {
        !matches!(self, Self::Start)
    }
}

/// How off-screen rows are kept cheap in a long transcript. See the module
/// doc's "Virtualization" section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Virtualization {
    /// Every row is fully rendered all the time.
    None,
    /// Rows stay in the DOM and the browser skips the ones that are
    /// off-screen (`content-visibility: auto`), except the rows at the live
    /// edge and any row holding focus or the selection. The accessibility tree
    /// and selection keep working across the whole transcript, and so does
    /// find-in-page in engines that search skipped content (not Safari 18-25).
    /// The default.
    #[default]
    ContentVisibility,
}

impl Virtualization {
    /// The wire / `data-virtualize` spelling: `"none"` or
    /// `"content-visibility"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ContentVisibility => "content-visibility",
        }
    }
}

/// How a programmatic scroll moves. `Smooth` is skipped under
/// `prefers-reduced-motion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollBehavior {
    /// Jump (or follow the viewport's own CSS `scroll-behavior`).
    #[default]
    Auto,
    /// Animate.
    Smooth,
}

/// Which viewport edge or point a jumped-to row is aligned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollAlign {
    /// The row's top at the viewport's top. The default.
    #[default]
    Start,
    /// The row centred in the viewport.
    Center,
    /// The row's bottom at the viewport's bottom.
    End,
    /// The least movement that brings the row fully into view.
    Nearest,
}

/// Options for [`MessageScrollerApi::scroll_to_message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollToMessageOptions {
    /// Where in the viewport the row lands.
    pub align: ScrollAlign,
    /// How the viewport moves there.
    pub behavior: ScrollBehavior,
}

/// Which transcript edge a [`MessageScrollerButton`] scrolls toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageScrollerDirection {
    /// Toward the oldest message.
    Start,
    /// Toward the newest message. The default.
    #[default]
    End,
}

impl MessageScrollerDirection {
    /// The `data-direction` spelling: `"start"` or `"end"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
        }
    }
}

/// Which edges the viewport can still scroll toward, from
/// [`use_message_scroller_scrollable`]. While the reader is following the
/// stream `end` stays `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Scrollable {
    /// Content is hidden above the viewport.
    pub start: bool,
    /// Content is hidden below the viewport.
    pub end: bool,
}

/// Rust -> controller messages. `t` is the discriminant the controller
/// switches on. `Init`, `Config` and `Teardown` are only built by the `web`
/// arm's controller wiring.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(not(feature = "web"), allow(dead_code))]
#[serde(tag = "t", rename_all = "camelCase")]
enum Msg {
    /// The first message: the provider's props and its root id.
    #[serde(rename_all = "camelCase")]
    Init {
        id: String,
        auto_scroll: bool,
        default_scroll_position: &'static str,
        edge: f64,
        peek: f64,
        margin: f64,
        virtualize: &'static str,
    },
    /// A provider prop changed.
    #[serde(rename_all = "camelCase")]
    Config {
        auto_scroll: bool,
        edge: f64,
        peek: f64,
        margin: f64,
        virtualize: &'static str,
    },
    /// `scrollToEnd`; `focus` is set when a button asked for it.
    End {
        behavior: ScrollBehavior,
        focus: bool,
    },
    /// `scrollToStart`.
    Start {
        behavior: ScrollBehavior,
        focus: bool,
    },
    /// `scrollToMessage`.
    Message {
        id: String,
        align: ScrollAlign,
        behavior: ScrollBehavior,
    },
    /// A button mounted: rewrite the JS-owned button state.
    Sync,
    /// The provider unmounted.
    Teardown,
}

/// The provider's shared state. Everything is `Copy` (signals and
/// `CopyValue`s), so handlers capture it freely.
#[derive(Clone, Copy)]
struct MessageScrollerCtx {
    /// The `data-message-scroller-root` value the controller looks the root
    /// element up by (a data attribute, so callers stay free to set their own
    /// `id`).
    id: Signal<String>,
    /// The controller's eval channel once it is running.
    eval: Signal<Option<Eval>>,
    /// Messages sent before the controller started, flushed in order after
    /// `init`.
    queue: CopyValue<Vec<Msg>>,
    /// The last `scrollable` state the controller reported.
    scrollable: Signal<Scrollable>,
    /// Whether the root and viewport render `data-pending-scroll`.
    pending_scroll: bool,
    /// The provider's `virtualize` prop, mirrored on the content as
    /// `data-virtualize`.
    virtualize: ReadSignal<Virtualization>,
}

impl MessageScrollerCtx {
    /// Deliver `msg` to the controller, or hold it until the controller
    /// starts. A no-op without the `web` feature (no controller exists).
    fn send(mut self, msg: Msg) {
        if !cfg!(feature = "web") {
            return;
        }
        let eval = *self.eval.peek();
        match eval {
            Some(eval) => {
                let _ = eval.send(msg);
            }
            None => self.queue.write().push(msg),
        }
    }
}

/// The props for the [`MessageScrollerProvider`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerProviderProps {
    /// Follow new content while the reader is at the live edge. Scrolling
    /// away (wheel, touch, scroll keys, scrollbar drag, an explicit jump)
    /// releases it; the jump button or [`MessageScrollerApi::scroll_to_end`]
    /// re-engages it. Off by default: nothing moves unless asked to.
    #[props(default)]
    pub auto_scroll: ReadSignal<bool>,

    /// Where a saved transcript opens, applied once on the first non-empty
    /// render. Read at mount (it also decides whether the flash guard
    /// renders).
    #[props(default)]
    pub default_scroll_position: DefaultScrollPosition,

    /// Distance in pixels from an edge that still counts as "at" that edge.
    #[props(default = ReadSignal::new(Signal::new(8.0)))]
    pub scroll_edge_threshold: ReadSignal<f64>,

    /// Pixels of the previous row kept visible above a newly anchored row.
    #[props(default = ReadSignal::new(Signal::new(64.0)))]
    pub scroll_previous_item_peek: ReadSignal<f64>,

    /// Extra margin in pixels on the aligned edge for anchoring and jumps.
    #[props(default = ReadSignal::new(Signal::new(0.0)))]
    pub scroll_margin: ReadSignal<f64>,

    /// How off-screen rows are kept cheap. Defaults to
    /// [`Virtualization::ContentVisibility`].
    #[props(default)]
    pub virtualize: ReadSignal<Virtualization>,

    /// The scroller's parts.
    pub children: Element,
}

/// # MessageScrollerProvider
///
/// The headless root: owns the controller and the scroll state, renders no
/// element of its own. Wrap a [`MessageScroller`] (and anything that calls
/// [`use_message_scroller`]) in it.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::message_scroller::*;
/// #[component]
/// fn Demo() -> Element {
///     let messages = use_signal(|| vec![("m1", "Hello"), ("m2", "Hi there")]);
///     rsx! {
///         MessageScrollerProvider { auto_scroll: true,
///             MessageScroller {
///                 MessageScrollerViewport {
///                     MessageScrollerContent {
///                         for (id, text) in messages() {
///                             MessageScrollerItem { key: "{id}", message_id: id, "{text}" }
///                         }
///                     }
///                 }
///                 MessageScrollerButton { "Scroll to end" }
///             }
///         }
///     }
/// }
/// ```
///
/// `MessageScroller` fills its parent, so place it in a height-constrained
/// container.
///
/// ## Styling
///
/// The root and viewport carry these data attributes (written by the
/// controller):
/// - `data-scrollable`: `"start"`, `"end"`, `"start end"`, or absent.
/// - `data-autoscrolling`: present while a programmatic scroll to the end runs.
/// - `data-pending-scroll`: present until the opening position is applied.
#[component]
pub fn MessageScrollerProvider(props: MessageScrollerProviderProps) -> Element {
    let id = use_unique_id();
    let eval = use_signal(|| None::<Eval>);
    let queue = use_hook(|| CopyValue::new(Vec::<Msg>::new()));
    let scrollable = use_signal(Scrollable::default);
    let position = props.default_scroll_position;
    let pending_scroll = use_hook(|| cfg!(feature = "web") && position.holds_pending());

    let virtualize = props.virtualize;

    let ctx = MessageScrollerCtx {
        id,
        eval,
        queue,
        scrollable,
        pending_scroll,
        virtualize,
    };
    use_context_provider(|| ctx);

    use_message_scroller_controller(
        ctx,
        position,
        props.auto_scroll,
        props.scroll_edge_threshold,
        props.scroll_previous_item_peek,
        props.scroll_margin,
        virtualize,
    );

    rsx! {
        {props.children}
    }
}

/// Installs the controller and keeps its config in sync -- the `web` arm.
///
/// The install effect reads every prop with `.peek()` so it runs exactly
/// once; the second effect tracks them and forwards changes as `config`
/// messages (queued if the controller has not started yet).
#[cfg(feature = "web")]
fn use_message_scroller_controller(
    ctx: MessageScrollerCtx,
    position: DefaultScrollPosition,
    auto_scroll: ReadSignal<bool>,
    edge: ReadSignal<f64>,
    peek: ReadSignal<f64>,
    margin: ReadSignal<f64>,
    virtualize: ReadSignal<Virtualization>,
) {
    let mut eval_slot = ctx.eval;
    let mut scrollable = ctx.scrollable;
    let mut queue = ctx.queue;
    let id = ctx.id;

    use_effect_with_cleanup(move || {
        let mut eval = document::eval(MESSAGE_SCROLLER_JS);
        let _ = eval.send(Msg::Init {
            id: id.peek().clone(),
            auto_scroll: *auto_scroll.peek(),
            default_scroll_position: position.as_str(),
            edge: *edge.peek(),
            peek: *peek.peek(),
            margin: *margin.peek(),
            virtualize: virtualize.peek().as_str(),
        });
        for msg in queue.take() {
            let _ = eval.send(msg);
        }
        eval_slot.set(Some(eval));
        spawn(async move {
            while let Ok((kind, start, end)) = eval.recv::<(String, bool, bool)>().await {
                if kind == "scrollable" {
                    scrollable.set(Scrollable { start, end });
                }
            }
        });
        move || {
            let _ = eval.send(Msg::Teardown);
        }
    });

    use_effect(move || {
        ctx.send(Msg::Config {
            auto_scroll: auto_scroll(),
            edge: edge(),
            peek: peek(),
            margin: margin(),
            virtualize: virtualize().as_str(),
        });
    });
}

/// The native (Blitz) arm: `document::eval` is a no-op there, so there is no
/// controller. The shell renders as plain overflow.
#[cfg(not(feature = "web"))]
fn use_message_scroller_controller(
    _ctx: MessageScrollerCtx,
    _position: DefaultScrollPosition,
    _auto_scroll: ReadSignal<bool>,
    _edge: ReadSignal<f64>,
    _peek: ReadSignal<f64>,
    _margin: ReadSignal<f64>,
    _virtualize: ReadSignal<Virtualization>,
) {
}

/// The props for the [`MessageScroller`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerProps {
    /// Additional attributes to apply to the root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The viewport and any controls.
    pub children: Element,
}

/// # MessageScroller
///
/// The frame element: lays out the viewport and the jump controls. Must be
/// inside a [`MessageScrollerProvider`].
///
/// ## Example
///
/// See [`MessageScrollerProvider`]'s example.
#[component]
pub fn MessageScroller(props: MessageScrollerProps) -> Element {
    let ctx: MessageScrollerCtx = use_context();
    let pending = ctx.pending_scroll;
    let base = attributes!(div {
        "data-message-scroller-root": ctx.id.cloned(),
        "data-pending-scroll": pending.then_some(""),
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div { ..merged, {props.children} }
    }
}

/// The props for the [`MessageScrollerViewport`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerViewportProps {
    /// Keep the first visible row where it is when older rows are prepended.
    /// On by default.
    #[props(default = true)]
    pub preserve_scroll_on_prepend: bool,

    /// Additional attributes to apply to the viewport element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The [`MessageScrollerContent`].
    pub children: Element,
}

/// # MessageScrollerViewport
///
/// The scrollable element. It must be the one that scrolls (give it an
/// `overflow-y` and a bounded height through the stylesheet). Defaults to
/// `role="region"`, `aria-label="Messages"` and `tabindex="0"`.
///
/// ## Example
///
/// See [`MessageScrollerProvider`]'s example.
#[component]
pub fn MessageScrollerViewport(props: MessageScrollerViewportProps) -> Element {
    let ctx: MessageScrollerCtx = use_context();
    let pending = ctx.pending_scroll;
    let preserve = props.preserve_scroll_on_prepend;
    let base = attributes!(div {
        role: "region",
        "aria-label": "Messages",
        tabindex: "0",
        "data-message-scroller-viewport": "",
        "data-preserve-scroll-on-prepend": if preserve { "true" } else { "false" },
        "data-pending-scroll": pending.then_some(""),
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div { ..merged, {props.children} }
    }
}

/// The props for the [`MessageScrollerContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerContentProps {
    /// A class for the internal tail spacer used when anchoring a turn near
    /// the top.
    #[props(default)]
    pub spacer_class: Option<String>,

    /// Additional attributes to apply to the content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The rows: every direct child should be a [`MessageScrollerItem`].
    pub children: Element,
}

/// # MessageScrollerContent
///
/// The transcript container. Holds the rows, plus a hidden `aria-hidden`
/// tail spacer the controller sizes so an anchored turn can reach the top of
/// a short transcript. Defaults to `role="log"` and
/// `aria-relevant="additions"`: new rows are announced politely, token-level
/// text changes inside a row are not.
///
/// ## Styling
///
/// - `data-virtualize`: `"none"` or `"content-visibility"`, the
///   [`Virtualization`] mode. Skip off-screen rows with
///   `content-visibility: auto` only under `"content-visibility"`, and never on
///   a row carrying `data-keep-rendered` (the live-edge, focused and selected
///   rows, written by the controller).
///
/// ## Example
///
/// See [`MessageScrollerProvider`]'s example.
#[component]
pub fn MessageScrollerContent(props: MessageScrollerContentProps) -> Element {
    let ctx: MessageScrollerCtx = use_context();
    let virtualize = ctx.virtualize.cloned().as_str();
    let base = attributes!(div {
        role: "log",
        "aria-relevant": "additions",
        "data-message-scroller-content": "",
        "data-virtualize": virtualize,
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div { ..merged,
            {props.children}
            div {
                "aria-hidden": "true",
                "data-message-scroller-spacer": "",
                class: props.spacer_class,
                hidden: true,
            }
        }
    }
}

/// The props for the [`MessageScrollerItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerItemProps {
    /// A stable row id, for [`MessageScrollerApi::scroll_to_message`] and
    /// prepend preservation. Rows without one still scroll and anchor but
    /// cannot be jumped to or preserved.
    #[props(default)]
    pub message_id: Option<String>,

    /// Marks a turn boundary: a newly appended anchor is scrolled near the
    /// top, and `DefaultScrollPosition::LastAnchor` opens on the last one.
    #[props(default)]
    pub scroll_anchor: bool,

    /// Additional attributes to apply to the row element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The row's content: a message, marker, typing indicator, separator or
    /// "load earlier" row.
    pub children: Element,
}

/// # MessageScrollerItem
///
/// One transcript row. Wrap every direct child of the content in one so the
/// scroller can measure, anchor, preserve and jump to it.
///
/// ## Example
///
/// See [`MessageScrollerProvider`]'s example.
#[component]
pub fn MessageScrollerItem(props: MessageScrollerItemProps) -> Element {
    let base = attributes!(div {
        "data-message-id": props.message_id.clone(),
        "data-scroll-anchor": if props.scroll_anchor { "true" } else { "false" },
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div { ..merged, {props.children} }
    }
}

/// The props for the [`MessageScrollerRows`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerRowsProps {
    /// How many rows there are.
    pub count: ReadSignal<usize>,

    /// The absolute position of row 0 (default 0). Rows are grouped by absolute
    /// position, so when older rows are prepended, lower `start` by the number
    /// prepended: every existing row then stays in its chunk and keeps its DOM
    /// node. (Appending needs no change.)
    #[props(default = ReadSignal::new(Signal::new(0)))]
    pub start: ReadSignal<isize>,

    /// Renders row `index` (an index into `0..count`): a [`MessageScrollerItem`].
    pub render_row: Callback<usize, Element>,

    /// Additional attributes to apply to each chunk element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # MessageScrollerRows
///
/// Renders a long transcript's rows in chunks of 20 consecutive rows, one
/// `div[data-message-scroller-chunk]` per chunk, so that
/// [`Virtualization::ContentVisibility`] skips (and un-skips) a chunk as a
/// unit instead of every row separately. One skippable element per row costs
/// the browser main-thread time proportional to the row count on every scroll
/// frame; per chunk it is one twentieth of that. Use it in place of a `for`
/// loop of [`MessageScrollerItem`]s inside [`MessageScrollerContent`]; the
/// controller treats the chunks as transparent, so following the stream,
/// anchoring, prepend preservation and jumps behave as with plain rows.
///
/// A chunk is the unit that stays rendered when it holds the live edge, focus
/// or an end of the selection.
///
/// ## Styling
///
/// Each chunk carries `--dx-chunk-rows`, its row count, for the stylesheet's
/// height guess, and the styled layer lays its rows out as a column with the
/// content's gap.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::message_scroller::{
///     MessageScroller, MessageScrollerContent, MessageScrollerItem, MessageScrollerProvider,
///     MessageScrollerRows, MessageScrollerViewport,
/// };
///
/// #[component]
/// fn Transcript(messages: Vec<String>) -> Element {
///     let count = messages.len();
///     rsx! {
///         MessageScrollerProvider {
///             MessageScroller {
///                 MessageScrollerViewport {
///                     MessageScrollerContent {
///                         MessageScrollerRows {
///                             count,
///                             render_row: move |i: usize| rsx! {
///                                 MessageScrollerItem { key: "{i}", message_id: "m{i}",
///                                     p { "message {i}" }
///                                 }
///                             },
///                         }
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn MessageScrollerRows(props: MessageScrollerRowsProps) -> Element {
    let base = attributes!(div {
        "data-message-scroller-chunk": "",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    let render_row = props.render_row;

    cv_chunks(
        (props.count)(),
        (props.start)(),
        ChunkSkip::Stylesheet,
        &merged,
        move |index| render_row.call(index),
    )
}

/// The props for the [`MessageScrollerButton`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MessageScrollerButtonProps {
    /// The transcript edge to scroll toward.
    #[props(default)]
    pub direction: MessageScrollerDirection,

    /// How the viewport moves. Smooth by default.
    #[props(default = ScrollBehavior::Smooth)]
    pub behavior: ScrollBehavior,

    /// Called before the scroll; call `prevent_default()` to cancel it.
    #[props(default)]
    pub onclick: Option<EventHandler<MouseEvent>>,

    /// Render a different element instead of the default `button`, keeping
    /// the scroll behaviour and the controller-owned state attributes.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// Additional attributes to apply to the button element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The button's content (an icon plus a screen-reader label).
    pub children: Element,
}

/// # MessageScrollerButton
///
/// Scrolls to the end (or start) of the transcript and, with `auto_scroll`,
/// re-engages following. A real `<button>`: while there is nothing to scroll
/// toward it is `inert` with `tabindex="-1"` and `data-active="false"`, all
/// written by the controller (the styled version slides and fades it out).
/// When it turns inert after being activated, focus moves to the viewport.
///
/// ## Example
///
/// See [`MessageScrollerProvider`]'s example.
///
/// ## Styling
///
/// - `data-active`: `"true"` when there is content in this direction.
/// - `data-direction`: `"start"` or `"end"`.
#[component]
pub fn MessageScrollerButton(props: MessageScrollerButtonProps) -> Element {
    let ctx: MessageScrollerCtx = use_context();
    let direction = props.direction;
    let behavior = props.behavior;
    let user_onclick = props.onclick;

    // A button that mounts after the last scroll state change still has to
    // pick up the current state.
    use_effect(move || ctx.send(Msg::Sync));

    let base = attributes!(button {
        r#type: "button",
        "data-message-scroller-button": "",
        "data-direction": direction.as_str(),
        "data-active": "false",
        "inert": true,
        tabindex: "-1",
        onclick: move |event: MouseEvent| {
            if let Some(handler) = &user_onclick {
                handler.call(event.clone());
            }
            if !event.default_action_enabled() {
                return;
            }
            ctx.send(match direction {
                MessageScrollerDirection::End => Msg::End { behavior, focus: true },
                MessageScrollerDirection::Start => Msg::Start { behavior, focus: true },
            });
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    if let Some(dynamic) = props.r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            button { ..merged, {props.children} }
        }
    }
}

/// The scroll commands for one scroller, from [`use_message_scroller`].
/// `Copy`; safe to move into handlers and tasks.
#[derive(Clone, Copy)]
pub struct MessageScrollerApi {
    ctx: MessageScrollerCtx,
}

impl MessageScrollerApi {
    /// Scroll to the newest message. With `auto_scroll` this also
    /// re-engages following. A no-op without the `web` feature.
    pub fn scroll_to_end(&self, behavior: ScrollBehavior) {
        self.ctx.send(Msg::End {
            behavior,
            focus: false,
        });
    }

    /// Scroll to the oldest message. Releases following. A no-op without the
    /// `web` feature.
    pub fn scroll_to_start(&self, behavior: ScrollBehavior) {
        self.ctx.send(Msg::Start {
            behavior,
            focus: false,
        });
    }

    /// Scroll to the row with this `message_id`. If the transcript is still
    /// empty the jump is queued until rows mount (and replaces the opening
    /// position); a missing id in a populated transcript is ignored. An
    /// explicit jump releases following. A no-op without the `web` feature.
    pub fn scroll_to_message(&self, id: impl Into<String>, options: ScrollToMessageOptions) {
        self.ctx.send(Msg::Message {
            id: id.into(),
            align: options.align,
            behavior: options.behavior,
        });
    }
}

/// The scroll commands for the nearest [`MessageScrollerProvider`].
///
/// # Panics
///
/// Panics outside a [`MessageScrollerProvider`].
pub fn use_message_scroller() -> MessageScrollerApi {
    MessageScrollerApi {
        ctx: use_context::<MessageScrollerCtx>(),
    }
}

/// Which edges the viewport can still scroll toward, as a signal fed by
/// change-only messages from the controller (never per scroll event or per
/// token). While the reader is following the stream `end` stays `false`.
/// Always the default on the native arm.
///
/// # Panics
///
/// Panics outside a [`MessageScrollerProvider`].
pub fn use_message_scroller_scrollable() -> ReadSignal<Scrollable> {
    use_context::<MessageScrollerCtx>().scrollable.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::find_element;

    #[component]
    fn Transcript(position: DefaultScrollPosition, preserve: bool) -> Element {
        rsx! {
            MessageScrollerProvider { default_scroll_position: position,
                MessageScroller {
                    MessageScrollerViewport { preserve_scroll_on_prepend: preserve,
                        MessageScrollerContent {
                            MessageScrollerItem { message_id: "m1".to_string(), scroll_anchor: true, "hello" }
                            MessageScrollerItem { "no id" }
                        }
                    }
                    MessageScrollerButton { "Scroll to end" }
                    MessageScrollerButton { direction: MessageScrollerDirection::Start, "Scroll to start" }
                }
            }
        }
    }

    fn render(position: DefaultScrollPosition, preserve: bool) -> String {
        let mut dom =
            VirtualDom::new_with_props(Transcript, TranscriptProps { position, preserve });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn virtualization_defaults_to_content_visibility() {
        assert_eq!(Virtualization::default(), Virtualization::ContentVisibility);
        assert_eq!(Virtualization::None.as_str(), "none");
        assert_eq!(
            Virtualization::ContentVisibility.as_str(),
            "content-visibility"
        );
    }

    #[component]
    fn Virtualized(mode: Virtualization) -> Element {
        rsx! {
            MessageScrollerProvider { virtualize: mode,
                MessageScroller {
                    MessageScrollerViewport {
                        MessageScrollerContent {
                            MessageScrollerItem { message_id: "m1".to_string(), "hello" }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn content_mirrors_the_effective_virtualization_mode() {
        for (mode, expected) in [
            (Virtualization::None, "none"),
            (Virtualization::ContentVisibility, "content-visibility"),
        ] {
            let mut dom = VirtualDom::new_with_props(Virtualized, VirtualizedProps { mode });
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            let content =
                find_element(&html, |a| a.contains_key("data-message-scroller-content")).unwrap();
            assert_eq!(
                content.attrs.get("data-virtualize").map(String::as_str),
                Some(expected),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn default_position_is_end() {
        assert_eq!(DefaultScrollPosition::default(), DefaultScrollPosition::End);
        assert_eq!(DefaultScrollPosition::LastAnchor.as_str(), "last-anchor");
    }

    #[test]
    fn viewport_and_content_carry_shadcn_aria_defaults() {
        let html = render(DefaultScrollPosition::Start, true);

        let viewport =
            find_element(&html, |a| a.contains_key("data-message-scroller-viewport")).unwrap();
        assert_eq!(
            viewport.attrs.get("role").map(String::as_str),
            Some("region")
        );
        assert_eq!(
            viewport.attrs.get("aria-label").map(String::as_str),
            Some("Messages")
        );
        assert_eq!(
            viewport.attrs.get("tabindex").map(String::as_str),
            Some("0")
        );

        let content =
            find_element(&html, |a| a.contains_key("data-message-scroller-content")).unwrap();
        assert_eq!(content.attrs.get("role").map(String::as_str), Some("log"));
        assert_eq!(
            content.attrs.get("aria-relevant").map(String::as_str),
            Some("additions")
        );
    }

    #[test]
    fn rows_and_spacer_render_their_data_hooks() {
        let html = render(DefaultScrollPosition::Start, true);

        let anchor = find_element(&html, |a| {
            a.get("data-message-id").map(String::as_str) == Some("m1")
        })
        .unwrap();
        assert_eq!(
            anchor.attrs.get("data-scroll-anchor").map(String::as_str),
            Some("true")
        );
        // A row without an id renders no `data-message-id` at all.
        let plain = find_element(&html, |a| {
            a.get("data-scroll-anchor").map(String::as_str) == Some("false")
        })
        .unwrap();
        assert!(!plain.attrs.contains_key("data-message-id"));

        let spacer =
            find_element(&html, |a| a.contains_key("data-message-scroller-spacer")).unwrap();
        assert!(spacer.attrs.contains_key("hidden"));
        assert_eq!(
            spacer.attrs.get("aria-hidden").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn preserve_scroll_on_prepend_is_mirrored_as_a_data_attribute() {
        for preserve in [true, false] {
            let html = render(DefaultScrollPosition::Start, preserve);
            let viewport =
                find_element(&html, |a| a.contains_key("data-message-scroller-viewport")).unwrap();
            assert_eq!(
                viewport
                    .attrs
                    .get("data-preserve-scroll-on-prepend")
                    .map(String::as_str),
                Some(if preserve { "true" } else { "false" })
            );
        }
    }

    #[test]
    fn button_renders_inert_and_inactive_before_the_controller_runs() {
        let html = render(DefaultScrollPosition::Start, true);
        let buttons = crate::test_support::find_elements(&html, |a| {
            a.contains_key("data-message-scroller-button")
        });
        assert_eq!(buttons.len(), 2);
        for button in &buttons {
            assert_eq!(button.name, "button");
            assert_eq!(button.attrs.get("type").map(String::as_str), Some("button"));
            assert!(button.attrs.contains_key("inert"));
            assert_eq!(button.attrs.get("tabindex").map(String::as_str), Some("-1"));
            assert_eq!(
                button.attrs.get("data-active").map(String::as_str),
                Some("false")
            );
        }
        assert_eq!(
            buttons[0].attrs.get("data-direction").map(String::as_str),
            Some("end")
        );
        assert_eq!(
            buttons[1].attrs.get("data-direction").map(String::as_str),
            Some("start")
        );
    }

    #[test]
    fn root_is_addressed_by_a_data_attribute_not_an_id() {
        let html = render(DefaultScrollPosition::Start, true);
        let root = find_element(&html, |a| a.contains_key("data-message-scroller-root")).unwrap();
        assert!(!root.attrs.contains_key("id"));
        assert!(root
            .attrs
            .get("data-message-scroller-root")
            .is_some_and(|v| v.starts_with("dxc-")));
    }

    /// The flash guard is part of the SSR/hydration contract: present in the
    /// server HTML for `End`/`LastAnchor` (and identical on the hydrating
    /// client), absent for `Start`.
    #[cfg(feature = "web")]
    #[test]
    fn pending_scroll_guard_renders_for_end_and_last_anchor_only() {
        for (position, expected) in [
            (DefaultScrollPosition::End, true),
            (DefaultScrollPosition::LastAnchor, true),
            (DefaultScrollPosition::Start, false),
        ] {
            let html = render(position, true);
            for marker in [
                "data-message-scroller-root",
                "data-message-scroller-viewport",
            ] {
                let el = find_element(&html, |a| a.contains_key(marker)).unwrap();
                assert_eq!(
                    el.attrs.contains_key("data-pending-scroll"),
                    expected,
                    "{marker} with {position:?}"
                );
            }
        }
    }

    /// The native (Blitz) arm has no controller to remove the guard, so it
    /// must never render it.
    #[cfg(not(feature = "web"))]
    #[test]
    fn native_arm_never_renders_the_pending_scroll_guard() {
        for position in [
            DefaultScrollPosition::End,
            DefaultScrollPosition::LastAnchor,
        ] {
            let html = render(position, true);
            assert!(!html.contains("data-pending-scroll"), "{position:?}");
        }
    }
}
