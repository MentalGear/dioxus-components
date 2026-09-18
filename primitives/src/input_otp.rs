//! Defines the [`InputOtp`] component and its subcomponents, a one-time-passcode
//! entry control: a row of single-character boxes backed by one real, focusable,
//! form-participating `<input>`.
//!
//! ## Provenance
//!
//! This is original composition, not a port. It builds on the *shape* of
//! [`crate::checkbox`]'s `BubbleInput` pattern -- a single real, accessible,
//! form-participating `<input>` kept in sync with a purely visual, styled
//! presentation -- but not on `BubbleInput` itself, because the two controls
//! solve different halves of that shape's problem:
//!
//! - `Checkbox`'s *visible*, interactive control is a `<button role="checkbox">`.
//!   Buttons are not [submittable
//!   elements](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#category-submit)
//!   the way `input`/`select`/`textarea` are, so `Checkbox` needs a *second*,
//!   hidden native `<input type="checkbox">` (`BubbleInput`) purely to give the
//!   surrounding `<form>` something real to submit -- one-way synced
//!   Rust-state-to-DOM via [`dioxus::document::eval`].
//! - `InputOtp`'s real `<input type="text">` is simultaneously the interactive
//!   control (it receives every keystroke, paste, and the caret) *and* the
//!   submittable one -- one control does both jobs a checkbox needs two
//!   controls for, so there is no second hidden input here at all. The boxes
//!   ([`InputOtpSlot`]) are a purely visual, `aria-hidden` overlay that mirrors
//!   this one real input's value and caret position; per
//!   `docs/conformance-harness.md`'s tier-2 HTML rules, the real, focusable,
//!   labelled `<input>` is the accessible source of truth.
//!
//! The caret-position bridge (`use_caret_sync`, DOM caret index -> Rust
//! signal) is new code but the *shape* -- a small `document::eval` script that
//! attaches DOM listeners and streams values back over the eval channel,
//! cleaned up on drop -- mirrors this crate's other browser-to-signal bridges
//! (`crate::use_form_reset_listener`, `crate::use_dialog_close_sync` in
//! `lib.rs`), reused here because `dioxus-html`'s own `SelectionEvent` carries
//! no cross-platform-readable selection range (`SelectionData` is an opaque
//! marker type -- see `dioxus-html`'s `events/selection.rs`), so there is no
//! way to read a caret position from the Dioxus event itself.
//!
//! Two references were read (never vendored or copied from), per
//! `docs/lifting-from-forks.md` §1: upstream Dioxus PR #255 (`ealmloff`,
//! draft) and `dignifiedquire`/`rust-ui`'s `input_otp.rs`, both for shape and
//! edge-case awareness only.
//!
//! ## APG / WAI-ARIA
//!
//! There is no APG pattern for "OTP input" -- it is not a distinct ARIA
//! widget, just a styled single-line text input (confirmed against APG's own
//! pattern index; shadcn/ui's own `InputOTP` docs describe it the same way,
//! built on the `input-otp` library over a plain `<input>`). This component
//! therefore follows WCAG's generic text-input requirements rather than
//! inventing a widget role that doesn't exist: a real, focusable, labelled
//! `<input>` (`type="text"`, `inputmode="numeric"`, name/required/disabled
//! all forwarded) is the whole accessibility surface; [`InputOtpSlot`],
//! [`InputOtpGroup`], and [`InputOtpSeparator`] render inside an
//! `aria-hidden="true"` presentational layer and contribute nothing to the
//! accessibility tree.

use crate::{use_controlled, use_form_reset_listener, use_id_or, use_previous, use_unique_id};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
struct InputOtpCtx {
    value: Memo<String>,
    /// The slot index the caret currently sits at, or `None` when the real
    /// input isn't focused (or the whole control is disabled) -- no slot is
    /// "active" then, matching a plain text input's own behavior.
    active_index: Memo<Option<usize>>,
    disabled: ReadSignal<bool>,
}

/// The props for the [`InputOtp`] component.
#[derive(Props, Clone, PartialEq)]
pub struct InputOtpProps {
    /// The controlled value of the passcode (each character is one slot).
    pub value: ReadSignal<Option<String>>,

    /// The default value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Called whenever the value changes (typing, backspace/delete, paste).
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Called once when the value's length reaches `max_length` (shadcn's
    /// `onComplete`) -- fires on the transition into "full", not on every
    /// keystroke made while already full.
    #[props(default)]
    pub on_complete: Callback<String>,

    /// The number of character slots. Must be at least 1; values below 1 are
    /// clamped up to 1.
    #[props(default = 6)]
    pub max_length: usize,

    /// Whether the whole control is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the passcode is required in a form.
    #[props(default)]
    pub required: ReadSignal<bool>,

    /// The name of the input, used in forms.
    #[props(default)]
    pub name: ReadSignal<String>,

    /// Optional id for the real input element -- also what a `<label for>`
    /// should point at, since the boxes are `aria-hidden` presentation.
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes for the real `<input>` -- this is the
    /// accessible surface (`aria-label`, `class`, `data-*`, ...).
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The visual layer: [`InputOtpGroup`]/[`InputOtpSlot`]/
    /// [`InputOtpSeparator`], entirely `aria-hidden`.
    pub children: Element,
}

/// # InputOtp
///
/// A one-time-passcode entry control: one real `<input>` drives every
/// visible box's displayed character and the active-slot/caret highlight.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::input_otp::{InputOtp, InputOtpGroup, InputOtpSlot, InputOtpSeparator};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         InputOtp {
///             max_length: 6,
///             aria_label: "Verification code",
///             InputOtpGroup {
///                 for i in 0..3 {
///                     InputOtpSlot { index: i }
///                 }
///             }
///             InputOtpSeparator {}
///             InputOtpGroup {
///                 for i in 3..6 {
///                     InputOtpSlot { index: i }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// [`InputOtpSlot`] defines the following data attributes you can use to
/// control styling:
/// - `data-active`: `true` while this slot holds the caret (the real input
///   is focused and its caret sits at this slot's index).
/// - `data-disabled`: mirrors the control's `disabled` prop.
#[component]
pub fn InputOtp(props: InputOtpProps) -> Element {
    let id = use_unique_id();
    let id = use_id_or(id, props.id);

    let max_length = props.max_length.max(1);
    let disabled = props.disabled;
    let required = props.required;
    let name = props.name;
    let on_complete = props.on_complete;

    let default_value = props.default_value.clone();
    let (value, set_value) =
        use_controlled(props.value, default_value.clone(), props.on_value_change);

    // The browser's own form-reset algorithm restores this `<input>`'s
    // `.value` IDL property from its `value` content attribute -- it has no
    // way to know about, let alone update, this component's own Rust
    // signal. Same fix as `checkbox.rs`'s `BubbleInput::on_reset`, applied
    // directly to the real input here (see this module's doc for why no
    // second hidden input is needed to get there).
    use_form_reset_listener(id, move || set_value.call(default_value.clone()));

    // Fire `on_complete` exactly on the transition into "full" (value's
    // length reaches `max_length`), not on every keystroke made while
    // already full -- matches shadcn's own `onComplete` semantics.
    let previous_value = use_previous(value.into());
    use_effect(move || {
        let current = value();
        if current.chars().count() >= max_length && previous_value().chars().count() < max_length {
            on_complete.call(current);
        }
    });

    let mut selection_start = use_signal(|| 0usize);
    let mut has_focus = use_signal(|| false);
    use_caret_sync(id, selection_start);

    let active_index = use_memo(move || {
        if !has_focus() || disabled() {
            None
        } else {
            Some(selection_start().min(max_length.saturating_sub(1)))
        }
    });

    use_context_provider(|| InputOtpCtx {
        value,
        active_index,
        disabled,
    });

    rsx! {
        div {
            position: "relative",
            display: "inline-flex",

            input {
                id: id.cloned(),
                r#type: "text",
                inputmode: "numeric",
                pattern: "[0-9]*",
                autocomplete: "one-time-code",
                spellcheck: "false",
                autocapitalize: "off",
                maxlength: "{max_length}",
                value: value(),
                name,
                required,
                disabled,

                // Fully invisible but still real: still focusable, still
                // receives every keystroke/paste/click, still the thing a
                // screen reader announces as a labelled text field -- only
                // its own rendering (text, caret) is hidden, since the boxes
                // underneath show both instead. `opacity: 0` (not
                // `display: none`/`visibility: hidden`) is what keeps it
                // focusable and hit-testable while hiding both the glyphs
                // and the caret uniformly (caret rendering follows element
                // opacity like everything else about the element).
                position: "absolute",
                top: "0",
                left: "0",
                right: "0",
                bottom: "0",
                width: "100%",
                height: "100%",
                margin: "0",
                padding: "0",
                border: "none",
                background: "transparent",
                opacity: "0",
                outline: "none",
                cursor: "text",

                onfocus: move |_| has_focus.set(true),
                onblur: move |_| has_focus.set(false),
                oninput: move |e: FormEvent| set_value.call(e.value()),

                // Overrides the browser's own click-to-caret placement,
                // which is wrong here (see `snap_caret_to_slot` and this
                // module's doc): the real input is `opacity: 0` with no
                // `letter-spacing` tuned to the visual slot grid
                // (`preview/src/components/input_otp/style.css`'s
                // `.dx-input-otp-input` sets none), so the browser maps a
                // click's x-coordinate to a caret index using invisible
                // glyph positions that were never aligned to the 40px-wide
                // slot boxes -- confirmed live: on an empty field, clicking
                // slot 0, 3, or 5 all placed the caret at index 0; on a
                // field filled with "123", clicking slot 0 landed at index
                // 2 while slots 1/4/5 all landed at index 3. This handler
                // recomputes the intended slot from geometry instead (the
                // slots are `pointer-events: none`, so `elementFromPoint`
                // can't be used -- see this module's doc on the earlier hit
                // -target fix) and explicitly moves the caret there,
                // matching upstream `input-otp`'s own approach of not
                // trusting native click-to-glyph mapping at all.
                onclick: move |e: MouseEvent| {
                    if disabled() {
                        return;
                    }
                    let input_id = id.cloned();
                    let value_len = value().chars().count();
                    let point = e.client_coordinates();
                    spawn(async move {
                        if let Some(idx) =
                            snap_caret_to_slot(input_id, value_len, point.x, point.y).await
                        {
                            selection_start.set(idx);
                        }
                    });
                },

                ..props.attributes,
            }

            // Purely visual/presentational -- the real input above is the
            // whole accessibility surface (this module's doc, WCAG generic
            // text-input requirements). `display: contents` so this wrapper
            // adds no layout box of its own between the caller's
            // `InputOtpGroup`(s) and this root.
            //
            // `pointer-events: none` (inherited by every descendant --
            // `InputOtpGroup`/`InputOtpSlot`/`InputOtpSeparator` set none of
            // their own) is the actual hit-target fix: this overlay paints
            // *after*, and therefore on top of, the real input above in the
            // same stacking context, and its `InputOtpSlot` boxes visually
            // cover almost the entire row. Without this, every click/tap
            // lands on a decorative, non-interactive box instead of the real
            // input beneath it -- confirmed via `elementFromPoint` on a
            // running instance: clicking dead-center of a visible slot
            // resolved to that slot's own `<div>`, not `#otp-main`, and only
            // the few-pixel gaps *between* slots (untouched by any box)
            // reached the input at all. `pointer-events: none` makes this
            // layer transparent to hit-testing, same as it already is
            // visually (`opacity: 0` on the real input, this module's doc),
            // so every click/tap across the full visible row -- slots,
            // separators, and the gaps between them -- reaches the real
            // input instead of stopping at its decorative overlay.
            div {
                aria_hidden: "true",
                display: "contents",
                pointer_events: "none",
                {props.children}
            }
        }
    }
}

/// Computes which [`InputOtpSlot`] a click at `(client_x, client_y)` landed
/// in by geometry, then explicitly moves the real `<input>`'s native caret
/// there -- overriding the browser's own (wrong, see [`InputOtp`]'s
/// `onclick`) click-to-glyph caret placement.
///
/// The slot boxes are `pointer-events: none` (the earlier "almost no hit
/// target" fix, this module's doc), so `elementFromPoint` would just return
/// the real input itself, never the slot underneath. Instead this reads
/// every slot's own `getBoundingClientRect()` directly and does a plain
/// point-in-rect test -- a fixed, non-scrolling layout, so this is exact.
/// Slots are queried from `id`'s own `aria-hidden` overlay sibling (its
/// `next_element_sibling` in the DOM, matching this module's own `input {
/// ... } div { aria_hidden: ... }` structure) rather than `document`-wide,
/// so this stays correct with more than one [`InputOtp`] on the same page.
/// Grouping ([`InputOtpGroup`]/[`InputOtpSeparator`]) needs no special
/// handling: every rendered slot carries its own `data-slot-index` and its
/// own rect regardless of which group it's in, so iterating all of them
/// flattens any grouping automatically.
///
/// Clamping rule: the target slot's index is clamped to `min(slot_index,
/// value_len)` -- clicking on or before an existing character goes to that
/// exact position (ordinary text-input UX), while clicking past the last
/// character (an empty, untyped slot) goes to `value_len`, i.e. right after
/// the last real character, the same place any plain text input puts the
/// caret when you click past its end. A click that lands in a gap no slot's
/// rect covers (e.g. a separator) is left alone -- returns `None`, so the
/// caller makes no change and the browser's own (harmless there, since
/// there's no slot grid to misalign with) placement stands.
///
/// Selection, not just a collapsed caret (2026-09-17 production report:
/// "I'm in the selected field then typing a number right [and it doesn't
/// correct]"): when the clamped index falls on an *existing* character
/// (`clamped < value_len`), this selects that one character --
/// `setSelectionRange(clamped, clamped + 1, ...)` -- instead of collapsing
/// to `(clamped, clamped)`. A collapsed caret makes native typing *insert*
/// at that position rather than overwrite, which produced two confirmed
/// symptoms: on a full value (already at `maxlength`), the browser silently
/// refuses the keystroke outright (inserting would exceed `maxlength`), and
/// on a partial value, typing a digit shifts every character after the
/// caret one slot right instead of correcting the clicked one. Typing over
/// an actual selection *replaces* it -- the standard `<input>` semantics for
/// "correct this character" -- and, because a replacement never grows the
/// string, it also can't hit the `maxlength` insert-block. When `clamped ==
/// value_len` (clicked past the last real character, nothing there to
/// select), the collapsed-caret behavior stays: there is no character at
/// that position for a range to cover.
///
/// Direction: `"forward"` (anchor at `clamped`, focus/caret at `clamped +
/// 1`), not the default `"none"` or `"backward"`. This matches the
/// left-to-right reading order every other caret movement in this component
/// already assumes (`ArrowRight` moving to a higher index, `Delete` acting
/// on the character *at* the caret going forward) and makes
/// Shift+`ArrowRight`/Shift+`ArrowLeft` extend/shrink from the natural end
/// if a caller ever drives this with a real keyboard selection. It has no
/// effect on the unshifted nav already covered by this module's tests:
/// unshifted `ArrowLeft`/`ArrowRight` on any non-collapsed selection collapse
/// to that selection's left/right edge regardless of direction (confirmed
/// live and re-verified by the keyboard-nav test below, which still passes
/// unmodified), so a single-character selection here behaves exactly like a
/// collapsed caret already would for every keyboard interaction this
/// component exercises -- the only user-visible change is that *typing*
/// over it now replaces instead of inserting.
async fn snap_caret_to_slot(
    id: String,
    value_len: usize,
    client_x: f64,
    client_y: f64,
) -> Option<usize> {
    let mut eval = document::eval(
        r#"
        const id = await dioxus.recv();
        const valueLen = await dioxus.recv();
        const x = await dioxus.recv();
        const y = await dioxus.recv();
        const input = document.getElementById(id);
        const overlay = input ? input.nextElementSibling : null;
        const slots = overlay ? Array.from(overlay.querySelectorAll('[data-slot-index]')) : [];
        let target = null;
        for (const slot of slots) {
            const r = slot.getBoundingClientRect();
            if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
                target = Number(slot.getAttribute('data-slot-index'));
                break;
            }
        }
        if (target !== null && input) {
            const clamped = Math.min(target, valueLen);
            if (clamped < valueLen) {
                // An existing character sits here -- select it so typing
                // replaces it instead of inserting before it.
                input.setSelectionRange(clamped, clamped + 1, 'forward');
            } else {
                // Past the last real character -- nothing to select, same
                // "click to continue typing" collapsed caret as before.
                input.setSelectionRange(clamped, clamped, 'forward');
            }
            dioxus.send(clamped);
        } else {
            dioxus.send(null);
        }
        "#,
    );
    let _ = eval.send(id);
    let _ = eval.send(value_len);
    let _ = eval.send(client_x);
    let _ = eval.send(client_y);
    eval.recv::<Option<usize>>().await.ok().flatten()
}

/// Keeps a Rust signal in sync with the real `<input>`'s live caret
/// position, so [`InputOtpSlot`] can show which slot currently holds the
/// caret. See this module's doc for why this can't be done from the Dioxus
/// event objects alone (`SelectionEvent`'s `SelectionData` carries no
/// readable range) and reads the DOM directly instead, the same
/// eval-listener-then-stream-back shape as `crate::use_form_reset_listener`/
/// `crate::use_dialog_close_sync`.
fn use_caret_sync(
    id: impl Readable<Target = String> + Copy + 'static,
    mut selection_start: Signal<usize>,
) {
    crate::use_effect_with_cleanup(move || {
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            const input = document.getElementById(id);
            const send = () => dioxus.send(input.selectionStart ?? input.value.length);
            const events = ['input', 'keyup', 'click', 'focus', 'select'];
            events.forEach((ev) => input.addEventListener(ev, send));
            await dioxus.recv();
            events.forEach((ev) => input.removeEventListener(ev, send));",
        );
        let _ = eval.send(id.cloned());
        spawn(async move {
            while let Ok(pos) = eval.recv::<usize>().await {
                selection_start.set(pos);
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// The props for the [`InputOtpGroup`] component.
#[derive(Props, Clone, PartialEq)]
pub struct InputOtpGroupProps {
    /// Additional attributes to apply to the group element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The [`InputOtpSlot`]s in this group.
    pub children: Element,
}

/// # InputOtpGroup
///
/// Lays out a cluster of [`InputOtpSlot`]s (e.g. the first half of a
/// 3-3-split six-digit code) with consistent spacing between them. Purely
/// visual -- must be used inside an [`InputOtp`], whose `aria-hidden`
/// wrapper already covers it.
#[component]
pub fn InputOtpGroup(props: InputOtpGroupProps) -> Element {
    rsx! {
        div { ..props.attributes, {props.children} }
    }
}

/// The props for the [`InputOtpSlot`] component.
#[derive(Props, Clone, PartialEq)]
pub struct InputOtpSlotProps {
    /// This slot's character index into the passcode value (`0`-based).
    pub index: usize,

    /// Additional attributes to apply to the slot element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # InputOtpSlot
///
/// A single visual box showing one character of the passcode's value at
/// `index`, and a caret indicator (via `data-active`, see [`InputOtp`]'s
/// styling section) while the real input's caret sits at this slot. Must be
/// used inside an [`InputOtp`].
#[component]
pub fn InputOtpSlot(props: InputOtpSlotProps) -> Element {
    let ctx: InputOtpCtx = use_context();
    let index = props.index;

    let value = ctx.value;
    let ch = use_memo(move || {
        value()
            .chars()
            .nth(index)
            .map(|c| c.to_string())
            .unwrap_or_default()
    });
    let active_index = ctx.active_index;
    let active = use_memo(move || active_index() == Some(index));
    let disabled = ctx.disabled;

    rsx! {
        div {
            "data-active": active(),
            "data-disabled": disabled(),
            "data-slot-index": "{index}",
            ..props.attributes,
            {ch()}
        }
    }
}

/// The props for the [`InputOtpSeparator`] component.
#[derive(Props, Clone, PartialEq)]
pub struct InputOtpSeparatorProps {
    /// Additional attributes to apply to the separator element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Custom separator content; defaults to a middle dot ("•") when omitted.
    #[props(default)]
    pub children: Option<Element>,
}

/// # InputOtpSeparator
///
/// A purely visual divider between [`InputOtpGroup`]s (e.g. the dash/dot
/// between a 3-3-split six-digit code). Contributes nothing to the
/// accessibility tree -- it sits inside [`InputOtp`]'s `aria-hidden`
/// wrapper alongside the groups/slots it separates.
#[component]
pub fn InputOtpSeparator(props: InputOtpSeparatorProps) -> Element {
    let children = props.children.unwrap_or_else(|| rsx! { "\u{2022}" });
    rsx! {
        div { ..props.attributes, {children} }
    }
}
