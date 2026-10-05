//! Defines the [`Drawer`] component and its sub-components -- a Vaul-style
//! (shadcn/ui "Drawer") sheet with a pointer drag-to-dismiss gesture.
//!
//! ## Composition, not reimplementation
//!
//! `Drawer` is the modal [`crate::dialog::DialogRoot`]/
//! [`crate::dialog::DialogContent`] primitive (see `dialog.rs`'s own module
//! doc for the web-arm native `<dialog>`/`showModal()` engine and the
//! native-arm vendored focus trap) plus a drag gesture layered on top --
//! exactly the composition `preview/src/components/sheet/component.rs`'s
//! themed `Sheet` already does for the *same* primitive, one layer higher.
//! `Drawer` does the same composition one layer lower, at the primitive
//! layer, because the drag gesture is new interaction/a11y logic and this
//! crate's own README asks for that to live here, not in `preview/`.
//!
//! Escape-to-close, focus trap/restore, background inertness, top-layer
//! rendering and scroll lock are therefore *inherited for free* from
//! [`crate::dialog::DialogContent`] -- this module adds nothing on top for
//! any of those; it only adds the pointer-drag gesture and the
//! data-attributes/inline styling that gesture needs.
//!
//! ## Drag construction
//!
//! The whole gesture lives in one real-DOM script ([`GESTURE_JS`], installed by
//! `use_drawer_gesture` on [`DrawerContent`]'s own root element), and the Rust
//! side only reacts to three messages: `start` (with the panel's measured size),
//! `move` (the cumulative offset, oriented so positive always means "toward the
//! closing edge") and `end` (the release velocity, and whether the browser
//! cancelled the gesture). It replaced a design that polled
//! `crate::pointer::pointer_position` from a reactive effect and timed its
//! samples with the wall clock of whenever that effect happened to run. Two
//! defects of that design are closed by construction:
//!
//! - **The release velocity was noise.** It was the displacement between the
//!   LAST TWO samples over their effect-run times, floored at 1 ms: on a busy
//!   main thread several `pointermove`s collapse into one effect run and a slow
//!   drag read as a fling (`drawer.spec.ts`'s old "short, slow drag" test noted
//!   it closed the drawer in roughly 1 run in 3). The script timestamps every
//!   sample with the event's own `timeStamp` and takes the velocity over the last
//!   [`VELOCITY_WINDOW_MS`] of movement before the release; a pause before
//!   letting go is therefore no flick, and a flick needs at least
//!   [`FLICK_MIN_DISTANCE_PX`] of travel so jitter cannot read as one.
//! - **The direction was decided in three places.** Orientation (down for
//!   `Bottom`, up for `Top`, and so on) is applied once, in the script, from the
//!   `vertical`/`sign` pair [`DrawerSide::drag_axis`] hands it; everything after
//!   -- the live offset, the rubber band, the release decision -- works on that
//!   one oriented number, so a drag toward the OPEN side can never be positive.
//!   `DrawerSide`s are physical screen edges (a `Right` drawer slides in from the
//!   physical right, in RTL too, exactly as `Sheet`'s do), so no `dir` flip
//!   belongs here.
//!
//! A drag that began inside the drawer and ended over its `::backdrop` used to be
//! read as a backdrop click by `crate::use_dialog_backdrop_dismiss` and closed the
//! drawer whichever way it was dragged; that hook now requires the PRESS to have
//! started on the backdrop too, so it is a dialog-wide fix, not a drawer one.
//!
//! Gating *where* a drag may start still needs the real DOM target (Dioxus's
//! synthetic `PointerData` has no `event.target`): a press on an interactive
//! descendant (button/link/form control), or on content scrolled away from the
//! drag edge, must not start a drawer drag. The same script does that, before it
//! starts tracking. A native `pointerdown` bubbles, so one listener on
//! [`DrawerContent`]'s root also catches a gesture begun on [`DrawerHandle`].
//! Mouse, touch and pen all arrive as Pointer Events, so one path serves them;
//! a mouse press other than the primary button never starts a drag, and a
//! `pointercancel` (the browser claiming the gesture for a pan) ends it as a
//! snap-back, never a dismissal.
//!
//! ## Why `translate`, not `transform`, carries the live drag offset
//!
//! [`DrawerContent`]'s *entrance* animation (`preview/src/components/drawer/
//! style.css`'s `.dx-drawer[data-state="open"]`) is a `@keyframes`/
//! `animation-fill-mode: forwards` pair, mirroring `Sheet`'s -- required
//! because a plain CSS `transition` never runs on an element's first style
//! computation, only on a later *change*, and this panel's `data-state` is
//! already `"open"` at the moment it is first inserted (`use_animated_open`
//! only mounts it once `open` is true).
//!
//! Per the CSS Animations cascade, a *running or forwards-held* animation's
//! value for a property overrides that property's normal-origin value for as
//! long as the animation applies to the element -- including a
//! higher-specificity *inline* style on that same property. Driving the live
//! drag offset through an inline `transform` would therefore be silently
//! overridden by the held `forwards` end-state of the entrance keyframe for
//! the entire time the drawer is open, i.e. the only time a drag can happen
//! -- the panel would not visibly move at all. `translate` (CSS Transforms
//! Level 2) is a separate property that *composes* with `transform` instead
//! of competing for the same cascade slot, so the keyframe's `transform` and
//! this module's inline `translate` both apply, combined, with no
//! precedence conflict and no extra timing coordination needed for the
//! drag-release-into-close handoff (the exit keyframe's `transform` and the
//! frozen-then-reset drag `translate` simply add). See
//! `preview/src/components/drawer/style.css` for the rest of the
//! construction, including its own `KNOWN GAP` note on the one remaining
//! cosmetic seam this choice leaves (a drag-triggered close and a
//! keyboard/backdrop-triggered close ease very slightly differently).

use crate::dialog::{
    DialogContent, DialogCtx, DialogDescription, DialogDescriptionProps, DialogRoot, DialogTitle,
    DialogTitleProps,
};
use crate::{merge_attributes, use_effect_with_cleanup, use_id_or, use_unique_id};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

/// Which edge of the viewport a [`Drawer`] slides in from, and is dragged
/// toward to dismiss.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DrawerSide {
    /// Slides in from, and is dragged upward toward, the top edge.
    Top,
    /// Slides in from, and is dragged rightward toward, the right edge.
    Right,
    /// Slides in from, and is dragged downward toward, the bottom edge.
    /// The default -- shadcn/ui's Drawer (Vaul) is a bottom sheet by
    /// default.
    #[default]
    Bottom,
    /// Slides in from, and is dragged leftward toward, the left edge.
    Left,
}

impl DrawerSide {
    /// The lowercase name used for the `data-side` attribute (and by the
    /// themed layer's CSS selectors).
    pub fn as_str(&self) -> &'static str {
        match self {
            DrawerSide::Top => "top",
            DrawerSide::Right => "right",
            DrawerSide::Bottom => "bottom",
            DrawerSide::Left => "left",
        }
    }

    /// The screen axis a drag toward this side's closing edge runs along, as
    /// `(vertical, sign)`: `vertical` is `true` for `Top`/`Bottom`, and `sign`
    /// is `1.0` when the closing direction is the positive screen-axis
    /// direction (down for `Bottom`, right for `Right`) and `-1.0` when it is
    /// the negative one (up for `Top`, left for `Left`). Multiplying a raw
    /// pointer delta along that axis by `sign` gives the number the whole
    /// gesture runs on: positive is toward the closing edge, negative is
    /// toward (and past) the open position. The screen sides are physical, so
    /// this does not depend on the text direction.
    fn drag_axis(self) -> (bool, f64) {
        match self {
            DrawerSide::Bottom => (true, 1.0),
            DrawerSide::Top => (true, -1.0),
            DrawerSide::Right => (false, 1.0),
            DrawerSide::Left => (false, -1.0),
        }
    }
}

/// shadcn/ui Drawer (Vaul)'s distance release heuristic: a drag past this
/// fraction of the panel's own size along its axis counts as "mostly
/// dismissed" and completes the close. Tier-3 "opinion" (see
/// `playwright/drawer.spec.ts`'s header) -- there is no APG/WHATWG rule for
/// a drag-to-dismiss gesture, only Vaul's own convention.
const DISMISS_DRAG_RATIO: f64 = 0.25;

/// shadcn/ui Drawer (Vaul)'s velocity release heuristic: a release *flung*
/// faster than this (in the dismiss direction, CSS pixels per millisecond)
/// closes the drawer even short of [`DISMISS_DRAG_RATIO`]'s distance.
/// Tier-3 "opinion", same source as that constant.
const DISMISS_VELOCITY_PX_PER_MS: f64 = 0.5;

/// A flick must have travelled at least this far toward the closing edge to
/// count: a press with a few pixels of jitter can momentarily exceed
/// [`DISMISS_VELOCITY_PX_PER_MS`] (1 px in 1 ms is already 1 px/ms) and must
/// not dismiss the drawer.
const FLICK_MIN_DISTANCE_PX: f64 = 10.0;

/// The release velocity is measured over the pointer movement of this many
/// milliseconds before letting go (using each event's own `timeStamp`), so a
/// pause before the release is no flick and a long, slow drag followed by a
/// quick wrist flick is read for what the hand did last.
const VELOCITY_WINDOW_MS: f64 = 100.0;

/// How much a drag *away* from the dismiss direction -- past the
/// fully-open resting position -- is damped, instead of hard-clamped to
/// exactly `0`: gives a small, springy "rubber band" resistance rather
/// than a dead stop. Vaul has the same overdrag resistance; the exact
/// ratio/cap here are this crate's own tuning, not copied from a spec.
const RUBBER_BAND_RATIO: f64 = 0.25;
/// The rubber band's own maximum travel, in CSS pixels.
const RUBBER_BAND_MAX_PX: f64 = 24.0;

/// The dismiss-oriented `raw` accumulated offset, converted to what should
/// actually be displayed: unchanged (up to the panel's own measured size,
/// once known) while dragging toward dismissal, or rubber-banded toward --
/// but never snapped to -- `0` while dragging the other way, past the
/// fully-open resting position.
///
/// A pure function of the *cumulative* raw offset (not of the previous
/// display value) on purpose: damping the already-damped display value
/// again each frame would make the panel visibly creep back toward `0`
/// while the pointer kept moving further into the overshoot, which is
/// wrong -- the damping curve must only ever be a function of how far off
/// the resting position the gesture has *actually* travelled.
fn damped_display_offset(raw: f64, panel_size: Option<f64>) -> f64 {
    if raw < 0.0 {
        (raw * RUBBER_BAND_RATIO).max(-RUBBER_BAND_MAX_PX)
    } else if let Some(size) = panel_size {
        raw.min(size)
    } else {
        raw
    }
}

/// The inline `translate` (not `transform` -- see this module's doc)
/// declaration for a dismiss-oriented `offset`, converted to this side's
/// real screen axis and sign.
fn translate_style(side: DrawerSide, offset: f64) -> String {
    // Explicit fast path for the resting value, rather than always
    // computing `offset * sign`: for `Top`/`Left`
    // (`sign == -1.0`), `0.0 * -1.0` is IEEE 754 negative zero,
    // which `f64`'s `Display` renders as `"-0"` -- a real, if purely
    // cosmetic, `"translate: 0 -0px"` for those two sides every time the
    // resting/reset value (by far the most common one -- every open and
    // every settled drag) was rendered.
    if offset == 0.0 {
        return "translate: 0 0px".to_string();
    }
    let (vertical, sign) = side.drag_axis();
    let signed = offset * sign;
    if vertical {
        format!("translate: 0 {signed}px")
    } else {
        format!("translate: {signed}px 0")
    }
}

/// Whether a release should close the drawer: dragged past
/// [`DISMISS_DRAG_RATIO`] of the panel's own size along its axis, or
/// released while moving faster than [`DISMISS_VELOCITY_PX_PER_MS`] toward
/// dismissal after travelling at least [`FLICK_MIN_DISTANCE_PX`] -- shadcn/ui
/// Drawer (Vaul)'s own two release heuristics (tier-3 "opinion", see
/// `playwright/drawer.spec.ts`'s header). `raw_offset` and `velocity` are both
/// dismiss-oriented (positive is toward the closing edge), so a drag toward the
/// OPEN side can never close: a `raw` offset at or behind the resting position
/// (`<= 0`, i.e. the rubber-band overshoot case) never closes, regardless of
/// velocity -- a fling in the "open further" direction is not a dismiss
/// gesture.
fn should_close_on_release(
    raw_offset: f64,
    panel_size: Option<f64>,
    velocity_px_per_ms: f64,
) -> bool {
    if raw_offset <= 0.0 {
        return false;
    }
    let past_distance_threshold = match panel_size {
        Some(size) if size > 0.0 => raw_offset / size > DISMISS_DRAG_RATIO,
        _ => false,
    };
    let flicked =
        velocity_px_per_ms > DISMISS_VELOCITY_PX_PER_MS && raw_offset >= FLICK_MIN_DISTANCE_PX;
    past_distance_threshold || flicked
}

/// Selects an element that must never itself start a drawer drag when a
/// pointerdown lands on (or inside) it -- the drag would otherwise steal
/// the gesture from the control's own click/press behaviour.
const INTERACTIVE_SELECTOR: &str =
    "button, a[href], input, select, textarea, [contenteditable=\"\"], [contenteditable=\"true\"], [role=\"button\"]";

/// [`use_drawer_gesture`]'s script: the whole drag gesture, in the real DOM.
/// Structurally the same recv-params/addEventListener/recv-sentinel/
/// removeEventListener shape as `lib.rs`'s `use_outside_dismiss` and
/// `use_dialog_backdrop_dismiss`.
///
/// Parameters, in order: the content element's id, `vertical`, `sign` (from
/// [`DrawerSide::drag_axis`]), [`INTERACTIVE_SELECTOR`], [`VELOCITY_WINDOW_MS`],
/// and `enabled` (the drawer's `dismissible`).
///
/// A `pointerdown` starts a drag unless
/// 1. it is a non-primary mouse button, or
/// 2. `event.target.closest(INTERACTIVE_SELECTOR)` matches -- the press landed on
///    (or inside) an interactive descendant, or
/// 3. the nearest scrollable ancestor *along the drag axis* is scrolled away from
///    `0` (the edge nearest the drag) -- that press should scroll that region
///    first. [`DrawerHandle`] itself is never nested inside such a region, so a
///    drag started there walks no scrollable ancestor at all.
///
/// Then, until that pointer is released or cancelled, every `pointermove` updates
/// the oriented offset (`raw delta * sign`, positive toward the closing edge) and
/// one timestamped sample; the offset is forwarded at most once a frame, and the
/// release forwards the last offset, then the velocity over the final
/// `windowMs` (0 when the pointer paused or was cancelled). Messages are
/// `[kind, a, b]`: `start` (size of the panel along the axis), `move` (offset),
/// `end` (velocity, cancelled as 0/1).
const GESTURE_JS: &str = r#"const id = await dioxus.recv();
const vertical = await dioxus.recv();
const sign = await dioxus.recv();
const interactiveSelector = await dioxus.recv();
const windowMs = await dioxus.recv();
const enabled = await dioxus.recv();
const el = document.getElementById(id);
if (el && enabled) {
    const finite = (n) => (Number.isFinite(n) ? n : 0);
    let drag = null;
    let raf = 0;
    const flush = () => {
        raf = 0;
        if (drag) dioxus.send(['move', finite(drag.offset), 0]);
    };
    const velocityAt = (t) => {
        const recent = drag.samples.filter(([at]) => at >= t - windowMs);
        if (recent.length < 2) return 0;
        const [t0, o0] = recent[0];
        const [t1, o1] = recent[recent.length - 1];
        return t1 > t0 ? (o1 - o0) / (t1 - t0) : 0;
    };
    const onMove = (e) => {
        if (!drag || e.pointerId !== drag.pointerId) return;
        drag.offset = ((vertical ? e.clientY - drag.startY : e.clientX - drag.startX)) * sign;
        drag.samples.push([e.timeStamp, drag.offset]);
        if (!raf) raf = requestAnimationFrame(flush);
    };
    const stop = () => {
        window.removeEventListener('pointermove', onMove);
        window.removeEventListener('pointerup', onUp);
        window.removeEventListener('pointercancel', onCancel);
        if (raf) { cancelAnimationFrame(raf); raf = 0; }
    };
    const end = (e, cancelled) => {
        if (!drag || e.pointerId !== drag.pointerId) return;
        const velocity = cancelled ? 0 : finite(velocityAt(e.timeStamp));
        const offset = finite(drag.offset);
        drag = null;
        stop();
        dioxus.send(['move', offset, 0]);
        dioxus.send(['end', velocity, cancelled ? 1 : 0]);
    };
    const onUp = (e) => end(e, false);
    const onCancel = (e) => end(e, true);
    const onDown = (e) => {
        if (drag) return;
        if (e.pointerType === 'mouse' && e.button !== 0) return;
        if (e.target.closest(interactiveSelector)) return;
        let node = e.target;
        while (node && node !== el.parentElement) {
            const style = getComputedStyle(node);
            const overflow = vertical ? style.overflowY : style.overflowX;
            const scrollable = (overflow === 'auto' || overflow === 'scroll')
                && (vertical ? node.scrollHeight > node.clientHeight : node.scrollWidth > node.clientWidth);
            if (scrollable) {
                const offset = vertical ? node.scrollTop : node.scrollLeft;
                if (offset !== 0) return;
                break;
            }
            if (node === el) break;
            node = node.parentElement;
        }
        const rect = el.getBoundingClientRect();
        drag = { pointerId: e.pointerId, startX: e.clientX, startY: e.clientY, offset: 0, samples: [[e.timeStamp, 0]] };
        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', onUp);
        window.addEventListener('pointercancel', onCancel);
        dioxus.send(['start', vertical ? rect.height : rect.width, 0]);
    };
    el.addEventListener('pointerdown', onDown);
    await dioxus.recv();
    el.removeEventListener('pointerdown', onDown);
    drag = null;
    stop();
}"#;

/// One message from [`GESTURE_JS`], already oriented toward the closing edge.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragEvent {
    /// A drag was approved; `size` is the panel's extent along the drag axis.
    Start { size: f64 },
    /// The cumulative offset toward the closing edge (negative past the open
    /// position).
    Move { offset: f64 },
    /// The pointer was released (or cancelled) at `velocity` px/ms toward the
    /// closing edge.
    End { velocity: f64, cancelled: bool },
}

impl DragEvent {
    /// Decodes a `[kind, a, b]` wire message; an unknown kind is ignored.
    fn from_wire(kind: &str, a: f64, b: f64) -> Option<Self> {
        match kind {
            "start" => Some(Self::Start { size: a }),
            "move" => Some(Self::Move { offset: a }),
            "end" => Some(Self::End {
                velocity: a,
                cancelled: b != 0.0,
            }),
            _ => None,
        }
    }
}

/// Installs the [`GESTURE_JS`] listener on the element identified by
/// `content_id` for the lifetime of the calling component, calling `on_event`
/// for every message. Reinstalled (not left stale) when `side` or `dismissible`
/// changes.
fn use_drawer_gesture(
    content_id: Memo<String>,
    side: ReadSignal<DrawerSide>,
    dismissible: ReadSignal<bool>,
    on_event: impl FnMut(DragEvent) + Clone + 'static,
) {
    use_effect_with_cleanup(move || {
        // Reactive reads: `side` and `dismissible` changing (unusual, but not
        // disallowed -- both are plain `ReadSignal` props) tears down and
        // reinstalls this listener with the new axis / enablement rather than
        // leaving the script silently stale.
        let (vertical, sign) = side.cloned().drag_axis();
        let enabled = dismissible.cloned();
        let mut eval = document::eval(GESTURE_JS);
        let _ = eval.send(content_id.cloned());
        let _ = eval.send(vertical);
        let _ = eval.send(sign);
        let _ = eval.send(INTERACTIVE_SELECTOR);
        let _ = eval.send(VELOCITY_WINDOW_MS);
        let _ = eval.send(enabled);
        let mut on_event = on_event.clone();
        spawn(async move {
            while let Ok((kind, a, b)) = eval.recv::<(String, f64, f64)>().await {
                if let Some(event) = DragEvent::from_wire(&kind, a, b) {
                    on_event(event);
                }
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Context shared from [`Drawer`] down to [`DrawerContent`]: the
/// `side`/`dismissible` configuration the drag gesture needs. Unlike
/// [`crate::dialog::DialogCtx`]'s `open`/`set_open` (which
/// [`DrawerContent`] fetches directly, since it is exactly what
/// `DialogCtx` already provides), neither field exists on `DialogCtx`, so
/// this module needs its own small context for them -- the same reason
/// `context_menu.rs`'s `ContextMenuCtx`, `command.rs`'s `CommandContext`,
/// etc. each define their own rather than trying to fit new fields into an
/// unrelated existing one.
#[derive(Clone, Copy)]
struct DrawerCtx {
    side: ReadSignal<DrawerSide>,
    dismissible: ReadSignal<bool>,
}

/// The props for the [`Drawer`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DrawerRootProps {
    /// The ID of the drawer root element.
    pub id: ReadSignal<Option<String>>,

    /// Whether the drawer is modal. If true, it will trap focus within the
    /// drawer when open. Forwarded unchanged to
    /// [`crate::dialog::DialogRoot`].
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub is_modal: ReadSignal<bool>,

    /// Whether the drawer dims the page behind it with the shared overlay scrim.
    /// Defaults to `true`. Forwarded unchanged to [`crate::dialog::DialogRoot`]:
    /// `false` puts `data-dx-overlay="off"` on the `<dialog>`, which the theme
    /// turns into a transparent `::backdrop`; focus trap, inertness and
    /// click-outside dismissal are untouched.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub overlay: ReadSignal<bool>,

    /// The controlled `open` state of the drawer.
    pub open: ReadSignal<Option<bool>>,

    /// The default `open` state of the drawer if it is not controlled.
    #[props(default)]
    pub default_open: bool,

    /// A callback that is called when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Which edge of the viewport the drawer slides in from, and is
    /// dragged toward to dismiss. Defaults to [`DrawerSide::Bottom`].
    #[props(default = ReadSignal::new(Signal::new(DrawerSide::Bottom)))]
    pub side: ReadSignal<DrawerSide>,

    /// Whether the drawer can be dismissed by dragging
    /// [`DrawerHandle`]/[`DrawerContent`] toward `side`. Escape and
    /// [`DrawerClose`] still close the drawer when this is `false` -- only
    /// the drag gesture is disabled, mirroring Vaul's own `dismissible`
    /// prop.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub dismissible: ReadSignal<bool>,

    /// Additional attributes to apply to the drawer root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the drawer root component.
    pub children: Element,
}

/// # Drawer
///
/// The entry point for the drawer -- a modal [`crate::dialog::DialogRoot`]
/// (see that component's own doc for the full modality/focus-trap/
/// scroll-lock behaviour, all inherited unchanged) plus the `side`/
/// `dismissible` configuration [`DrawerContent`]'s drag gesture consumes.
/// Manages the open state and provides context to its children; the
/// contents only render while open.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::drawer::{Drawer, DrawerContent, DrawerDescription, DrawerTitle};
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         button { onclick: move |_| open.set(true), "Show Drawer" }
///         Drawer {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             DrawerContent {
///                 DrawerTitle { "Item information" }
///                 DrawerDescription { "Here is some additional information about the item." }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// [`DrawerContent`] defines the following data attributes you can use to
/// control styling:
/// - `data-state`: `"open"` or `"closed"`.
/// - `data-side`: `"top"`, `"right"`, `"bottom"`, or `"left"`.
/// - `data-dragging`: `"true"` while an active pointer drag is in
///   progress, `"false"` otherwise.
#[component]
pub fn Drawer(props: DrawerRootProps) -> Element {
    use_context_provider(|| DrawerCtx {
        side: props.side,
        dismissible: props.dismissible,
    });

    rsx! {
        DialogRoot {
            id: props.id,
            is_modal: props.is_modal,
            overlay: props.overlay,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`DrawerContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DrawerContentProps {
    /// The ID of the drawer content element.
    pub id: ReadSignal<Option<String>>,

    /// The class to apply to the drawer content element.
    #[props(default)]
    pub class: Option<String>,

    /// Additional attributes to apply to the drawer content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the drawer content.
    pub children: Element,
}

/// # DrawerContent
///
/// The content of the drawer -- must be used inside a [`Drawer`]. Renders
/// [`crate::dialog::DialogContent`] (so it inherits that primitive's focus
/// trap/restore, inertness, top layer and Escape handling unchanged) with
/// `data-side`/`data-state`/`data-dragging` data attributes and, while a
/// drag has moved the panel, an inline `translate` (see this module's doc
/// for why `translate` and not `transform`) tracking the pointer.
///
/// Owns the drag gesture for both itself and any [`DrawerHandle`] rendered
/// inside it -- see [`GESTURE_JS`]'s doc for why one listener on this
/// component's own root element covers both surfaces.
///
/// ## Example
///
/// See [`Drawer`]'s own example.
#[component]
pub fn DrawerContent(props: DrawerContentProps) -> Element {
    let drawer_ctx: DrawerCtx = use_context();
    let dialog_ctx: DialogCtx = use_context();
    let side = drawer_ctx.side;
    let dismissible = drawer_ctx.dismissible;

    let gen_id = use_unique_id();
    let id = use_id_or(gen_id, props.id);
    let id_opt = use_memo(move || Some(id()));

    let mut dragging = use_signal(|| false);
    // Cumulative offset toward the closing edge, as `GESTURE_JS` reports it
    // (negative past the open position); the panel's extent along the drag
    // axis, measured by the script when the drag starts; and, once a drag has
    // CLOSED the drawer, the offset the panel was released at.
    let mut raw_offset = use_signal(|| 0.0_f64);
    let mut panel_size = use_signal(|| None::<f64>);
    let mut released_offset = use_signal(|| None::<f64>);

    // Gesture: see this module's doc for why it is a real DOM script rather
    // than a declarative `onpointerdown`.
    use_drawer_gesture(id, side, dismissible, move |event| match event {
        DragEvent::Start { size } => {
            if !dismissible() || *dragging.peek() {
                return;
            }
            raw_offset.set(0.0);
            panel_size.set(Some(size));
            released_offset.set(None);
            dragging.set(true);
        }
        DragEvent::Move { offset } => {
            if *dragging.peek() {
                raw_offset.set(offset);
            }
        }
        DragEvent::End {
            velocity,
            cancelled,
        } => {
            if !*dragging.peek() {
                return;
            }
            let raw = *raw_offset.peek();
            let size = *panel_size.peek();
            dragging.set(false);
            if !cancelled && should_close_on_release(raw, size, velocity) {
                // Hold the panel where the pointer left it: the exit keyframe
                // (`transform`) then carries it the rest of the way from HERE.
                // Resetting the offset to 0 instead made `translate` ease back
                // to rest while the exit slid the panel out -- a visible hitch
                // against the release point.
                released_offset.set(Some(damped_display_offset(raw, size)));
                dialog_ctx.set_open(false);
            } else {
                raw_offset.set(0.0);
            }
        }
    });

    // A drawer reopened while it was still closing starts from rest, not from
    // wherever the last drag released it. Reads `open` only; the writes are
    // guarded by a `peek`, so this never subscribes to what it writes.
    use_effect(move || {
        if dialog_ctx.is_open() && released_offset.peek().is_some() {
            released_offset.set(None);
            raw_offset.set(0.0);
        }
    });

    let is_open = dialog_ctx.is_open();
    let offset =
        released_offset().unwrap_or_else(|| damped_display_offset(raw_offset(), panel_size()));
    let style = translate_style(side.cloned(), offset);

    let content_base = attributes!(div {
        "data-side": side.cloned().as_str(),
        "data-state": if is_open { "open" } else { "closed" },
        "data-dragging": if dragging() { "true" } else { "false" },
        style: style,
    });
    let content_attributes = merge_attributes(vec![content_base, props.attributes]);

    rsx! {
        DialogContent {
            id: ReadSignal::from(id_opt),
            class: props.class,
            attributes: content_attributes,
            {props.children}
        }
    }
}

/// The props for the [`DrawerHandle`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DrawerHandleProps {
    /// Additional attributes to apply to the handle element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # DrawerHandle
///
/// A decorative grab bar, meant to be rendered inside [`DrawerContent`].
/// Purely a pointer-drag affordance (`aria-hidden="true"`): keyboard users
/// dismiss the drawer with Escape or [`DrawerClose`] instead.
///
/// This component owns no pointer-event logic of its own: starting a drag
/// here is handled by the *same* listener [`DrawerContent`] installs on
/// its own root element -- a native `pointerdown` bubbles up to it from
/// any descendant, this handle included. It only needs to render as a
/// recognizable surface (styled by the themed layer) somewhere inside a
/// [`DrawerContent`].
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_primitives::drawer::DrawerHandle;
/// # fn demo() -> Element {
/// rsx! {
///     DrawerHandle {}
/// }
/// # }
/// ```
#[component]
pub fn DrawerHandle(props: DrawerHandleProps) -> Element {
    let base = attributes!(div {
        "aria-hidden": "true",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div { ..merged }
    }
}

/// # DrawerTitle
///
/// The title of the drawer, used to label it for accessibility. Thin
/// wrapper over [`crate::dialog::DialogTitle`] -- must be used inside a
/// [`DrawerContent`]. An accessible name is required:
/// [`crate::dialog::DialogRoot`] always wires the dialog's
/// `aria-labelledby` to a `DialogTitle` id whether or not one is rendered,
/// so omitting this leaves a broken reference (axe's `aria-dialog-name`
/// rule) rather than simply an unnamed drawer -- see
/// [`crate::command::CommandDialog`]'s doc for the same lesson learned
/// there first.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_primitives::drawer::DrawerTitle;
/// # fn demo() -> Element {
/// rsx! {
///     DrawerTitle { "Move Goal" }
/// }
/// # }
/// ```
#[component]
pub fn DrawerTitle(props: DialogTitleProps) -> Element {
    rsx! {
        DialogTitle { id: props.id, attributes: props.attributes, {props.children} }
    }
}

/// # DrawerDescription
///
/// The description of the drawer, used to describe it for accessibility.
/// Thin wrapper over [`crate::dialog::DialogDescription`] -- must be used
/// inside a [`DrawerContent`].
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_primitives::drawer::DrawerDescription;
/// # fn demo() -> Element {
/// rsx! {
///     DrawerDescription { "Set your daily activity goal." }
/// }
/// # }
/// ```
#[component]
pub fn DrawerDescription(props: DialogDescriptionProps) -> Element {
    rsx! {
        DialogDescription { id: props.id, attributes: props.attributes, {props.children} }
    }
}

/// The props for the [`DrawerClose`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DrawerCloseProps {
    /// Additional attributes to apply to the close button element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Render a different element (e.g. an `a`) instead of the default
    /// `button`, while keeping the close behaviour -- shadcn/ui's
    /// `asChild` pattern. Mirrors
    /// `preview/src/components/sheet/component.rs`'s `SheetClose`.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// The children of the close button.
    pub children: Element,
}

/// # DrawerClose
///
/// A button that closes the drawer when activated. Thin wrapper over
/// [`crate::dialog::DialogCtx::set_open`] -- must be used inside a
/// [`Drawer`]. Mirrors
/// `preview/src/components/sheet/component.rs`'s `SheetClose`, which does
/// the same thing without a primitive-layer `DialogClose` to wrap (this
/// crate's `dialog.rs` does not define one); `DrawerClose` provides that
/// missing piece at the primitive layer for `Drawer` specifically.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_primitives::drawer::DrawerClose;
/// # fn demo() -> Element {
/// rsx! {
///     DrawerClose { "Close" }
/// }
/// # }
/// ```
#[component]
pub fn DrawerClose(props: DrawerCloseProps) -> Element {
    let ctx: DialogCtx = use_context();

    let base = attributes!(button {
        onclick: move |_| {
            ctx.set_open(false);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawer_side_defaults_to_bottom() {
        assert_eq!(DrawerSide::default(), DrawerSide::Bottom);
    }

    /// The orientation every drag runs on: `(vertical, sign)` such that a raw delta
    /// times `sign` is positive toward the side's own closing edge. `GESTURE_JS`
    /// applies exactly this, so this table is the specification of which way each
    /// side closes -- a bottom drawer closes DOWN (never up), a top drawer UP, a
    /// right drawer RIGHT, a left drawer LEFT.
    #[test]
    fn drag_axis_points_toward_each_sides_own_closing_edge() {
        assert_eq!(DrawerSide::Bottom.drag_axis(), (true, 1.0));
        assert_eq!(DrawerSide::Top.drag_axis(), (true, -1.0));
        assert_eq!(DrawerSide::Right.drag_axis(), (false, 1.0));
        assert_eq!(DrawerSide::Left.drag_axis(), (false, -1.0));
    }

    /// Applies the script's orientation to a pointer delta, for the cases below.
    fn oriented(side: DrawerSide, delta_x: f64, delta_y: f64) -> f64 {
        let (vertical, sign) = side.drag_axis();
        (if vertical { delta_y } else { delta_x }) * sign
    }

    #[test]
    fn movement_toward_the_closing_edge_is_positive_and_away_from_it_negative() {
        // Screen y grows downward: a bottom drawer closes with +y, a top one with -y.
        assert!(oriented(DrawerSide::Bottom, 0.0, 10.0) > 0.0);
        assert!(oriented(DrawerSide::Bottom, 0.0, -10.0) < 0.0);
        assert!(oriented(DrawerSide::Top, 0.0, -10.0) > 0.0);
        assert!(oriented(DrawerSide::Top, 0.0, 10.0) < 0.0);
        assert!(oriented(DrawerSide::Right, 10.0, 0.0) > 0.0);
        assert!(oriented(DrawerSide::Right, -10.0, 0.0) < 0.0);
        assert!(oriented(DrawerSide::Left, -10.0, 0.0) > 0.0);
        assert!(oriented(DrawerSide::Left, 10.0, 0.0) < 0.0);
        // The cross axis never counts.
        assert_eq!(oriented(DrawerSide::Bottom, 500.0, 0.0), 0.0);
        assert_eq!(oriented(DrawerSide::Right, 0.0, 500.0), 0.0);
    }

    #[test]
    fn drag_event_decodes_the_wire_messages_and_ignores_unknown_ones() {
        assert_eq!(
            DragEvent::from_wire("start", 346.0, 0.0),
            Some(DragEvent::Start { size: 346.0 })
        );
        assert_eq!(
            DragEvent::from_wire("move", -12.5, 0.0),
            Some(DragEvent::Move { offset: -12.5 })
        );
        assert_eq!(
            DragEvent::from_wire("end", 0.8, 1.0),
            Some(DragEvent::End {
                velocity: 0.8,
                cancelled: true
            })
        );
        assert_eq!(DragEvent::from_wire("nonsense", 1.0, 1.0), None);
    }

    #[test]
    fn damped_display_offset_passes_through_toward_dismissal_up_to_panel_size() {
        assert_eq!(damped_display_offset(50.0, Some(400.0)), 50.0);
        // Clamped at the panel's own measured size, not left to overshoot.
        assert_eq!(damped_display_offset(500.0, Some(400.0)), 400.0);
        // No measurement yet -- pass through unclamped rather than stall
        // the drag.
        assert_eq!(damped_display_offset(500.0, None), 500.0);
    }

    #[test]
    fn damped_display_offset_rubber_bands_overshoot_instead_of_hard_clamping_to_zero() {
        let damped = damped_display_offset(-40.0, Some(400.0));
        // Neither the raw overshoot (a hard "no resistance") nor exactly
        // 0.0 (a dead stop) -- a damped value strictly between them.
        assert!(damped < 0.0 && damped > -40.0);
        // Capped at the rubber band's own maximum travel.
        assert_eq!(
            damped_display_offset(-1000.0, Some(400.0)),
            -RUBBER_BAND_MAX_PX
        );
    }

    #[test]
    fn damped_display_offset_is_a_pure_function_of_cumulative_raw_offset() {
        // Regression for the exact bug this module's own doc warns against:
        // re-damping an already-damped *display* value each frame would
        // make two equal cumulative raw offsets, reached via different
        // paths, disagree -- they must not.
        let via_one_big_step = damped_display_offset(-40.0, Some(400.0));
        let raw_after_two_steps = -20.0 + -20.0;
        let via_two_small_steps = damped_display_offset(raw_after_two_steps, Some(400.0));
        assert_eq!(via_one_big_step, via_two_small_steps);
    }

    #[test]
    fn translate_style_resting_value_is_the_same_for_every_side() {
        // Regression: `0.0 * sign` is negative zero for
        // `Top`/`Left` (`sign == -1.0`), which `f64::Display`
        // would otherwise render as the literal text `"-0"`.
        for side in [
            DrawerSide::Top,
            DrawerSide::Right,
            DrawerSide::Bottom,
            DrawerSide::Left,
        ] {
            assert_eq!(translate_style(side, 0.0), "translate: 0 0px");
        }
    }

    #[test]
    fn translate_style_orients_toward_each_sides_own_edge() {
        assert_eq!(
            translate_style(DrawerSide::Bottom, 40.0),
            "translate: 0 40px"
        );
        assert_eq!(translate_style(DrawerSide::Top, 40.0), "translate: 0 -40px");
        assert_eq!(
            translate_style(DrawerSide::Right, 40.0),
            "translate: 40px 0"
        );
        assert_eq!(
            translate_style(DrawerSide::Left, 40.0),
            "translate: -40px 0"
        );
    }

    #[test]
    fn should_close_on_release_past_distance_threshold() {
        // 30% of a 400px panel exceeds the 25% ratio threshold.
        assert!(should_close_on_release(120.0, Some(400.0), 0.0));
        // 20% does not, and the velocity is below threshold too.
        assert!(!should_close_on_release(80.0, Some(400.0), 0.0));
    }

    #[test]
    fn should_close_on_release_past_velocity_threshold_even_short_of_distance() {
        // Only 5% of the panel's distance, but flung faster than 0.5px/ms.
        assert!(should_close_on_release(20.0, Some(400.0), 0.8));
    }

    #[test]
    fn should_close_on_release_never_closes_on_overshoot_regardless_of_velocity() {
        // A fast release while still past the resting position (dragging
        // the *wrong* way) must never be read as a dismiss.
        assert!(!should_close_on_release(-10.0, Some(400.0), 5.0));
        assert!(!should_close_on_release(0.0, Some(400.0), 5.0));
    }

    #[test]
    fn should_close_on_release_with_no_measured_panel_size_falls_back_to_velocity_only() {
        assert!(!should_close_on_release(1000.0, None, 0.0));
        assert!(should_close_on_release(40.0, None, 0.6));
    }

    #[test]
    fn should_close_on_release_ignores_a_fast_twitch_that_travelled_almost_nowhere() {
        // 3px in 2ms is 1.5px/ms, but it is jitter, not a flick.
        assert!(!should_close_on_release(3.0, Some(400.0), 1.5));
        // The same speed after a real throw is.
        assert!(should_close_on_release(30.0, Some(400.0), 1.5));
    }

    #[test]
    fn should_close_on_release_never_closes_a_drag_toward_the_open_side() {
        // The owner's bug: dragging a bottom drawer UP (negative, oriented) --
        // however far, however fast -- is never a dismissal.
        assert!(!should_close_on_release(-300.0, Some(400.0), 2.0));
        assert!(!should_close_on_release(-300.0, Some(400.0), -2.0));
    }
}
