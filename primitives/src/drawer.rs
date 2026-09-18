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
//! Movement tracking reuses this crate's own house pattern for pointer
//! drags -- [`crate::slider`]'s `onpointerdown` seeding
//! [`crate::pointer::track_pointer_down`], then a reactive `use_effect`
//! polling [`crate::pointer::pointer_position`] on every global
//! `pointermove` the crate-wide listener in `pointer.rs` already installs --
//! rather than the DOM's native `setPointerCapture`, which is not reachable
//! from Dioxus's synthetic event handlers without an escape to
//! `document::eval` per element anyway.
//!
//! Two things `slider.rs` doesn't need that a drag-to-dismiss gesture does:
//!
//! - **Gating *where* a drag may start.** A pointerdown on an interactive
//!   descendant (button/link/form control) or on content that is scrolled
//!   away from the drag edge must not start a drawer drag. Neither is
//!   discoverable from Dioxus's synthetic `PointerData` (no `event.target`
//!   accessor -- see `dioxus-html`'s `HasPointerData` trait), so
//!   [`use_drawer_drag_start_gate`] installs one real
//!   `addEventListener('pointerdown', ...)` -- same `document::eval` idiom
//!   `lib.rs`'s `use_outside_dismiss` already uses for its own real-DOM-target
//!   need -- on [`DrawerContent`]'s own root element. A native `pointerdown`
//!   bubbles, so this one listener also catches gestures started on
//!   [`DrawerHandle`], a descendant; it forwards only an approved
//!   `(pointer_id, x, y)` triple back to [`crate::pointer::track_pointer_down`].
//! - **A release *decision*.** `slider.rs` only ever clamps a continuous
//!   value; [`should_close_on_release`]'s threshold/velocity math is new.
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
use crate::{merge_attributes, pointer, use_effect_with_cleanup, use_id_or, use_unique_id};
use dioxus::html::geometry::ClientPoint;
use dioxus::prelude::*;
use dioxus_attributes::attributes;
use std::rc::Rc;

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

    /// Whether this side's drag axis is vertical (`Top`/`Bottom`) rather
    /// than horizontal (`Left`/`Right`).
    fn is_vertical(self) -> bool {
        matches!(self, DrawerSide::Top | DrawerSide::Bottom)
    }

    /// `1.0` when the dismiss direction is the positive screen-axis
    /// direction (down for `Bottom`, right for `Right`); `-1.0` when it is
    /// the negative direction (up for `Top`, left for `Left`).
    fn dismiss_sign(self) -> f64 {
        match self {
            DrawerSide::Bottom | DrawerSide::Right => 1.0,
            DrawerSide::Top | DrawerSide::Left => -1.0,
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

/// How much a drag *away* from the dismiss direction -- past the
/// fully-open resting position -- is damped, instead of hard-clamped to
/// exactly `0`: gives a small, springy "rubber band" resistance rather
/// than a dead stop. Vaul has the same overdrag resistance; the exact
/// ratio/cap here are this crate's own tuning, not copied from a spec.
const RUBBER_BAND_RATIO: f64 = 0.25;
/// The rubber band's own maximum travel, in CSS pixels.
const RUBBER_BAND_MAX_PX: f64 = 24.0;

/// Projects a raw pointer-move delta onto `side`'s drag axis, oriented so a
/// positive result always means "further toward dismissed" -- regardless
/// of which of the four sides `side` is -- which is what lets the rest of
/// this module's clamp/threshold/rubber-band math be written once,
/// side-agnostically, instead of once per side.
fn dismiss_oriented_delta(side: DrawerSide, delta_x: f64, delta_y: f64) -> f64 {
    let raw = if side.is_vertical() { delta_y } else { delta_x };
    raw * side.dismiss_sign()
}

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
    // computing `offset * side.dismiss_sign()`: for `Top`/`Left`
    // (`dismiss_sign() == -1.0`), `0.0 * -1.0` is IEEE 754 negative zero,
    // which `f64`'s `Display` renders as `"-0"` -- a real, if purely
    // cosmetic, `"translate: 0 -0px"` for those two sides every time the
    // resting/reset value (by far the most common one -- every open and
    // every settled drag) was rendered.
    if offset == 0.0 {
        return "translate: 0 0px".to_string();
    }
    let signed = offset * side.dismiss_sign();
    if side.is_vertical() {
        format!("translate: 0 {signed}px")
    } else {
        format!("translate: {signed}px 0")
    }
}

/// Whether a release should close the drawer: dragged past
/// [`DISMISS_DRAG_RATIO`] of the panel's own size along its axis, or
/// released while moving faster than [`DISMISS_VELOCITY_PX_PER_MS`] toward
/// dismissal -- shadcn/ui Drawer (Vaul)'s own two release heuristics
/// (tier-3 "opinion", see `playwright/drawer.spec.ts`'s header). A `raw`
/// offset at or behind the resting position (`<= 0`, i.e. the rubber-band
/// overshoot case) never closes, regardless of velocity -- a fling in the
/// "open further" direction is not a dismiss gesture.
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
    past_distance_threshold || velocity_px_per_ms > DISMISS_VELOCITY_PX_PER_MS
}

/// Coarse wall-clock milliseconds, used only as the two-sample basis for a
/// release velocity (elapsed time between the last two pointer-move
/// samples within a single drag) -- never persisted or compared across
/// separate drags, so wall-clock jumps are not a concern.
/// `time::UtcDateTime::now()` rather than `std::time::Instant::now()`: the
/// latter panics on `wasm32-unknown-unknown` (no OS clock), which is
/// exactly why this crate's own `web` Cargo feature already enables
/// `time/wasm-bindgen` (`Cargo.toml`) for the `OffsetDateTime`/
/// `UtcDateTime` calls `calendar.rs`/`lib.rs` already make unconditionally
/// on both targets.
fn now_ms() -> f64 {
    (time::UtcDateTime::now().unix_timestamp_nanos() as f64) / 1_000_000.0
}

/// Selects an element that must never itself start a drawer drag when a
/// pointerdown lands on (or inside) it -- the drag would otherwise steal
/// the gesture from the control's own click/press behaviour.
const INTERACTIVE_SELECTOR: &str =
    "button, a[href], input, select, textarea, [contenteditable=\"\"], [contenteditable=\"true\"], [role=\"button\"]";

/// [`use_drawer_drag_start_gate`]'s script. Structurally the same
/// recv-id/addEventListener/recv-sentinel/removeEventListener shape as
/// `lib.rs`'s `use_outside_dismiss` and `use_dialog_backdrop_dismiss` --
/// the house idiom, in this crate, for anything that needs the real DOM
/// event target rather than Dioxus's synthetic one.
///
/// Two checks against the real `pointerdown`, in order:
/// 1. `event.target.closest(INTERACTIVE_SELECTOR)` -- reject a press that
///    landed on (or inside) an interactive descendant.
/// 2. Walk from `event.target` up to the content root looking for the
///    nearest scrollable ancestor *along the drag axis*; if it is
///    scrolled away from `0` (the edge nearest the drag gesture), reject
///    -- that pointerdown should scroll that region first, not start
///    dismissing the drawer. [`DrawerHandle`] itself is never nested
///    inside such a region (it sits alongside the scrollable body, not
///    within it), so a drag started there walks no scrollable ancestor at
///    all and is never rejected by this check -- no special-case needed.
const DRAG_START_GATE_JS: &str = r#"const id = await dioxus.recv();
const vertical = await dioxus.recv();
const interactiveSelector = await dioxus.recv();
const el = document.getElementById(id);
if (el) {
    const onPointerDown = (e) => {
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
        dioxus.send([e.pointerId, e.clientX, e.clientY]);
    };
    el.addEventListener('pointerdown', onPointerDown);
    await dioxus.recv();
    el.removeEventListener('pointerdown', onPointerDown);
}"#;

/// Installs the [`DRAG_START_GATE_JS`] listener on the element identified
/// by `content_id` for the lifetime of the calling component, calling
/// `on_start(pointer_id, client_x, client_y)` for every approved
/// pointerdown.
fn use_drawer_drag_start_gate(
    content_id: Memo<String>,
    side: ReadSignal<DrawerSide>,
    on_start: impl FnMut(i32, f64, f64) + Clone + 'static,
) {
    use_effect_with_cleanup(move || {
        // Reactive read: `side` changing (unusual, but not disallowed --
        // `side` is a plain `ReadSignal` prop) tears down and reinstalls
        // this listener with the new axis rather than leaving the
        // scroll-direction check silently stale.
        let vertical = side.cloned().is_vertical();
        let mut eval = document::eval(DRAG_START_GATE_JS);
        let _ = eval.send(content_id.cloned());
        let _ = eval.send(vertical);
        let _ = eval.send(INTERACTIVE_SELECTOR);
        let mut on_start = on_start.clone();
        spawn(async move {
            while let Ok((pointer_id, x, y)) = eval.recv::<(i32, f64, f64)>().await {
                on_start(pointer_id, x, y);
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
/// inside it -- see [`use_drawer_drag_start_gate`]'s doc for why one
/// listener on this component's own root element covers both surfaces.
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
    let mut raw_offset = use_signal(|| 0.0_f64);
    let mut panel_size = use_signal(|| None::<f64>);
    let mut panel_element = use_signal(|| None::<Rc<MountedData>>);
    let mut active_pointer_id = use_signal(|| None::<i32>);
    let mut last_sample = use_hook(|| CopyValue::new(None::<(ClientPoint, f64)>));
    let mut last_velocity = use_hook(|| CopyValue::new(0.0_f64));

    // Gate + start: see this module's doc for why starting a drag needs a
    // real DOM listener rather than a declarative `onpointerdown`.
    use_drawer_drag_start_gate(id, side, move |pointer_id, x, y| {
        if !dismissible() || active_pointer_id.peek().is_some() {
            return;
        }

        pointer::track_pointer_down(pointer_id, ClientPoint::new(x, y));
        active_pointer_id.set(Some(pointer_id));
        last_sample.set(Some((ClientPoint::new(x, y), now_ms())));
        last_velocity.set(0.0);
        dragging.set(true);

        // Measure the panel fresh for every drag -- see this module's doc
        // for why a live `onresize` refresh is not worth the extra
        // plumbing on top of this.
        if let Some(element) = panel_element.peek().clone() {
            let vertical = side.cloned().is_vertical();
            spawn(async move {
                if let Ok(rect) = element.get_client_rect().await {
                    let size = if vertical {
                        rect.height()
                    } else {
                        rect.width()
                    };
                    panel_size.set(Some(size));
                }
            });
        }
    });

    use_effect(move || {
        if !dragging() {
            return;
        }
        let Some(pointer_id) = active_pointer_id() else {
            return;
        };

        let Some(position) = pointer::pointer_position(pointer_id) else {
            // The crate-wide global `pointerup`/`pointercancel` listener
            // (`pointer.rs`) removed this id -- the gesture ended. Decide
            // close-vs-snap-back from whatever was accumulated up to the
            // last real move sample.
            let should_close =
                should_close_on_release(raw_offset(), panel_size(), last_velocity.cloned());

            dragging.set(false);
            active_pointer_id.set(None);
            last_sample.set(None);
            raw_offset.set(0.0);

            if should_close {
                dialog_ctx.set_open(false);
            }
            return;
        };

        let now = now_ms();
        if let Some((last_position, last_time)) = last_sample.cloned() {
            let delta_x = position.x - last_position.x;
            let delta_y = position.y - last_position.y;
            let elapsed_ms = (now - last_time).max(1.0);
            let axis_delta = dismiss_oriented_delta(side.cloned(), delta_x, delta_y);

            last_velocity.set(axis_delta / elapsed_ms);
            raw_offset.set(raw_offset() + axis_delta);
        }
        last_sample.set(Some((position, now)));
    });

    let is_open = dialog_ctx.is_open();
    let offset = damped_display_offset(raw_offset(), panel_size());
    let style = translate_style(side.cloned(), offset);

    let content_base = attributes!(div {
        "data-side": side.cloned().as_str(),
        "data-state": if is_open { "open" } else { "closed" },
        "data-dragging": if dragging() { "true" } else { "false" },
        style: style,
        onmounted: move |evt| panel_element.set(Some(evt.data())),
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

    #[test]
    fn dismiss_oriented_delta_is_positive_toward_each_sides_own_edge() {
        // Bottom: dragging down (positive Y) is toward dismissal.
        assert_eq!(dismiss_oriented_delta(DrawerSide::Bottom, 0.0, 10.0), 10.0);
        // Top: dragging up (negative Y) is toward dismissal.
        assert_eq!(dismiss_oriented_delta(DrawerSide::Top, 0.0, -10.0), 10.0);
        // Right: dragging right (positive X) is toward dismissal.
        assert_eq!(dismiss_oriented_delta(DrawerSide::Right, 10.0, 0.0), 10.0);
        // Left: dragging left (negative X) is toward dismissal.
        assert_eq!(dismiss_oriented_delta(DrawerSide::Left, -10.0, 0.0), 10.0);
    }

    #[test]
    fn dismiss_oriented_delta_is_negative_away_from_each_sides_own_edge() {
        assert_eq!(dismiss_oriented_delta(DrawerSide::Bottom, 0.0, -5.0), -5.0);
        assert_eq!(dismiss_oriented_delta(DrawerSide::Top, 0.0, 5.0), -5.0);
        assert_eq!(dismiss_oriented_delta(DrawerSide::Right, -5.0, 0.0), -5.0);
        assert_eq!(dismiss_oriented_delta(DrawerSide::Left, 5.0, 0.0), -5.0);
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
        // Regression: `0.0 * side.dismiss_sign()` is negative zero for
        // `Top`/`Left` (`dismiss_sign() == -1.0`), which `f64::Display`
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
        assert!(should_close_on_release(1.0, None, 0.6));
    }
}
