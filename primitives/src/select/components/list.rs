//! SelectList component implementation.

use crate::{
    has_own_accessible_name, listbox::use_listbox_container, merge_attributes, use_effect,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

use super::super::context::SelectContext;

/// The props for the [`SelectList`] component
#[derive(Props, Clone, PartialEq)]
pub struct SelectListProps {
    /// The ID of the list for ARIA attributes
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes for the list
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children to render inside the list
    pub children: Element,
}

/// # SelectList
///
/// The dropdown list container for the [`Select`](super::select::Select) component that contains the
/// [`SelectOption`](super::option::SelectOption)s. The list will only be rendered when the select is open.
///
/// This must be used inside a [`Select`](super::select::Select) component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::select::{
///     Select, SelectGroup, SelectGroupLabel, SelectItemIndicator, SelectList, SelectOption,
///     SelectTrigger, SelectValue,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Select::<String> {
///             SelectTrigger {
///                 aria_label: "Select Trigger",
///                 width: "12rem",
///                 SelectValue { placeholder: "Select a fruit..." }
///             }
///             SelectList {
///                 aria_label: "Select Demo",
///                 SelectGroup {
///                     SelectGroupLabel { "Fruits" }
///                     SelectOption::<String> {
///                         index: 0usize,
///                         value: "apple",
///                         "Apple"
///                         SelectItemIndicator { "✔️" }
///                     }
///                     SelectOption::<String> {
///                         index: 1usize,
///                         value: "banana",
///                         "Banana"
///                         SelectItemIndicator { "✔️" }
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn SelectList(props: SelectListProps) -> Element {
    let ctx = use_context::<SelectContext>();

    let listbox = use_listbox_container(props.id, ctx.selectable);
    let render = listbox.render;

    rsx! {
        if render() {
            SelectListRendered {
                id: listbox.id.cloned(),
                attributes: props.attributes,
                children: props.children,
            }
        } else {
            // If not rendering, return children directly so we can populate the selected list, but they should choose to not render themselves
            {props.children}
        }
    }
}

/// Shared listbox keydown handling for both render arms below -- unchanged
/// from before this slice's migration, just factored out so it isn't
/// duplicated across the web/native split.
fn select_list_onkeydown(mut ctx: SelectContext) -> impl FnMut(KeyboardEvent) {
    move |event: KeyboardEvent| {
        let key = event.key();
        let code = event.code();

        // Learn from keyboard events for adaptive matching
        if let Key::Character(actual_char) = &key {
            if let Some(actual_char) = actual_char.chars().next() {
                ctx.learn_from_keyboard_event(&code.to_string(), actual_char);
            }
        }

        let mut arrow_key_navigation = |event: &KeyboardEvent| {
            // Clear the typeahead buffer
            ctx.typeahead_buffer.take();
            event.prevent_default();
            event.stop_propagation();
        };

        match key {
            Key::Character(new_text) => {
                if new_text == " " {
                    ctx.select_current_item();
                    event.prevent_default();
                    event.stop_propagation();
                    return;
                }

                ctx.add_to_typeahead_buffer(&new_text);
            }
            Key::ArrowUp => {
                arrow_key_navigation(&event);
                ctx.selectable.collection.focus_prev();
            }
            Key::End => {
                arrow_key_navigation(&event);
                ctx.selectable.collection.focus_last();
            }
            Key::ArrowDown => {
                arrow_key_navigation(&event);
                ctx.selectable.collection.focus_next();
            }
            Key::Home => {
                arrow_key_navigation(&event);
                ctx.selectable.collection.focus_first();
            }
            Key::Enter => {
                ctx.select_current_item();
                event.prevent_default();
                event.stop_propagation();
            }
            Key::Escape => {
                // Web arm: the listbox's own `popover="auto"` (see
                // `SelectListRendered` below) owns Escape dismissal
                // natively (WHATWG HTML light dismiss), and its own
                // restore-focus-on-hide step returns focus to the trigger
                // for free (the "previously focused element" recorded when
                // `showPopover()` was called) -- returning here before
                // `prevent_default()` leaves the key's default action alone
                // so that native algorithm still runs, matching
                // `DropdownMenu`'s identical Escape carve-out
                // (`dropdown_menu.rs`) and for the same reason: calling
                // both `set_open` *and* `prevent_default` unconditionally
                // here would race it. Native (Blitz) arm: unchanged, closes
                // here directly (Blitz has no popover-API light dismiss to
                // defer to).
                if cfg!(feature = "web") {
                    return;
                }
                ctx.set_open(false);
                event.prevent_default();
                event.stop_propagation();
            }
            _ => {}
        }
    }
}

/// Web arm (Migration A slice 3/3, final): promote the listbox to the top
/// layer via `popover="auto"` so it escapes clipping/transformed ancestors,
/// the same fix already shipped for `Tooltip`/`HoverCard`/`Popover`/
/// `DropdownMenu`/`Menubar` (docs/plan.md Phase 4.4, Migration A slices
/// 1-2). Anchored to `SelectTrigger` below/start-aligned, matching this
/// component's pre-migration `top: 100%; left: 0` CSS
/// (`../../../../preview/src/components/select/style.css`).
///
/// ## `auto`, decided by execution
///
/// This listbox holds real DOM focus while open (`listbox_ref.set_focus`
/// below, and individual `SelectOption`s via the roving-tabindex collection
/// -- `collection.rs`'s `control_mount_focus`) and closes itself on
/// `onblur` (below) when that focus leaves the whole widget -- exactly the
/// CAUTION this slice's task named: does `showPopover()` fight that
/// self-driven blur-close? Confirmed by execution that it does not:
///
/// - **Escape**: `select_list_onkeydown` above defers to native light
///   dismiss on this arm (the same carve-out `DropdownMenu` uses). WHATWG's
///   popover hide algorithm restores focus to the "previously focused
///   element" recorded when `showPopover()` ran -- here, `SelectTrigger`,
///   since that is whatever had focus at the moment the trigger's own click
///   opened this popover -- *for free*, satisfying this slice's guarded
///   "Escape -> trigger" oracle without any Rust-side `.focus()` call on
///   this path. The pre-existing `use_refocus_on_close_unless`
///   (`select.rs`, unchanged) still independently focuses the trigger on
///   every `open` "closes" transition regardless of cause; by the time it
///   runs here, native has already moved focus there, so its own
///   `.focus()` call is a same-element no-op, not a fight.
/// - **Outside pointerdown**: light dismiss and this listbox's own
///   `onblur` (or an individual option's, `option.rs`) both react to the
///   same interaction and both land on the same outcome --
///   `interacted_outside = true`, `open = false` -- calling `set_open`
///   twice with the same value is idempotent, not a double-fire bug. The
///   native restore-focus step is a no-op here too: by the time
///   `hidePopover()`'s algorithm runs (mid pointerdown, before the click's
///   own default focus-shift action), a real outside interaction hasn't
///   moved focus away *yet* on this exact tick in the case that does
///   matter for this slice's guarded suite (Tab, which `SelectOption`'s/
///   this component's `onblur` already handle exactly as before -- Tab
///   does not close the popover as a WHATWG light-dismiss trigger at all,
///   it is not a `pointerdown` -- so light dismiss never enters into the
///   `select.spec.ts` "tabbing out ... closes" cases).
/// - **Animation**: does *not* use [`crate::top_layer::use_popover_sync`]
///   bound to the raw `open` signal, unlike every prior `dx-anchor-*`
///   consumer -- see [`crate::top_layer::use_popover_shown_while_mounted`]'s
///   doc for the animate-out race that binding would otherwise cause with
///   [`crate::use_animated_open`] (`SelectList`, above), and why this
///   mount/unmount-scoped variant avoids it.
#[cfg(feature = "web")]
#[component]
fn SelectListRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let mut ctx: SelectContext = use_context();
    let open = ctx.selectable.open;
    // axe `aria-input-field-name` (docs/backlog.md row 34's own round): an
    // ARIA `listbox` is an input field and needs an accessible name.
    // Default it from the trigger (whose own visible text is the
    // placeholder/selected value) -- but ONLY if the caller hasn't already
    // supplied one, since this component's own doc example demonstrates
    // `SelectList { aria_label: "Select Demo", ... }` as the intended
    // override, and `aria-labelledby` would otherwise take ARIA precedence
    // over that caller-supplied `aria-label` and silently shadow it.
    let mut listbox_ref: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    // See `SelectContext::keep_trigger_focus`'s doc: an Alt+ArrowDown open
    // must leave DOM focus on the trigger, so this "nothing focused yet --
    // focus the listbox container" fallback must not fire for that open.
    let focused =
        move || open() && !ctx.selectable.collection.any_focused() && !(ctx.keep_trigger_focus)();

    use_effect(move || {
        let Some(listbox_ref) = listbox_ref() else {
            return;
        };
        if focused() {
            spawn(async move {
                _ = listbox_ref.set_focus(true);
            });
        }
    });

    let onkeydown = select_list_onkeydown(ctx);

    crate::top_layer::use_popover_shown_while_mounted(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            if !is_open {
                // Native light dismiss (Escape or outside pointerdown)
                // fired. Mirrors `DropdownMenuContentRendered`'s identical
                // belt-and-suspenders: the item/listbox that actually held
                // DOM focus is also blurred by the browser's own
                // `[popover]:not(:popover-open) { display: none }` UA rule
                // taking effect, which already runs this same
                // `interacted_outside`/`clear_focus`/`set_open` sequence via
                // this component's or `SelectOption`'s own `onblur` in the
                // common case -- this is a backstop for the rare native-only
                // close that wouldn't otherwise produce a blur (see this
                // component's doc, "Outside pointerdown").
                ctx.selectable.interacted_outside.set(true);
                ctx.selectable.collection.clear_focus();
            }
            ctx.set_open(is_open);
        }),
    );
    // JS-measured static positioning fallback for engines without CSS
    // Anchor Positioning -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `side`/`align`/gap match this component's pre-migration CSS
    // (`top: 100%; left: 0; margin-top: 0.25rem` == 4px at the default root
    // font size): anchored below the trigger, left-aligned with it, the
    // same bottom/start convention `DropdownMenu`/`Menubar` already use.
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        ctx.selectable.trigger_id.cloned(),
        open,
        crate::ContentSide::Bottom,
        crate::ContentAlign::Start,
        4,
    );

    // See `dropdown_menu.rs`'s `DropdownMenuContentRendered` for why this
    // hand-written, never-`Styles::`-routed marker class exists: it is what
    // the shared, engine-injected anchor-positioning stylesheet
    // (`top_layer::ensure_anchor_positioning_styles`) selects on,
    // sidestepping `manganis-core`'s `css_module_parser` not scoping
    // classes inside `@supports` bodies.
    //
    // axe `aria-input-field-name` (docs/backlog.md row 34's own round): an
    // ARIA `listbox` is an input field and needs an accessible name.
    // Default it from the trigger (whose own visible text is the
    // placeholder/selected value) -- but ONLY if the caller hasn't already
    // supplied one, since this component's own doc example demonstrates
    // `SelectList { aria_label: "Select Demo", ... }` as the intended
    // override, and `aria-labelledby` would otherwise take ARIA precedence
    // over that caller-supplied `aria-label` and silently shadow it.
    // Contributed as its own merge input -- present only when applicable --
    // rather than a bare `aria_labelledby: ...` literal alongside the
    // `..attributes` spread below: a caller attribute list can carry a
    // same-named `aria-labelledby`/`aria-label` with an
    // empty/`AttributeValue::None` value (`has_own_accessible_name`'s own
    // doc explains why), and two entries for one attribute name is exactly
    // the duplicate-attribute hazard `merge_attributes` exists to prevent
    // (`docs/conformance-harness.md` hydration-parity Rule 4) -- a literal
    // plus a raw spread cannot dedupe that; only routing every contributor
    // through one `merge_attributes` call can. Ordered after `attributes`
    // so it wins over exactly that spurious caller-side empty value, while
    // a caller's own *real* name (which makes `has_own_accessible_name`
    // true) leaves this contributing nothing at all to collide with.
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{ctx.selectable.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-select"
        }),
        labelledby,
    ]);

    rsx! {
        div {
            id: id.clone(),
            role: "listbox",
            tabindex: if focused() { "0" } else { "-1" },
            aria_multiselectable: ctx.multi(),
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            style: crate::top_layer::position_anchor_style(&ctx.selectable.trigger_id.cloned()),

            // Data attributes
            "data-state": if open() { "open" } else { "closed" },

            onmounted: move |evt| listbox_ref.set(Some(evt.data())),
            onkeydown,
            onblur: move |_| {
                if focused() {
                    ctx.selectable.interacted_outside.set(true);
                    ctx.set_open(false);
                }
            },

            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice -- Blitz has no
/// popover-API support at all (`docs/recommended-implementations.md`
/// Caveat 2), so this stays the functional floor, a plain, always-in-flow
/// `div`.
#[cfg(not(feature = "web"))]
#[component]
fn SelectListRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let mut ctx: SelectContext = use_context();
    let open = ctx.selectable.open;
    let mut listbox_ref: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    // See `SelectContext::keep_trigger_focus`'s doc: an Alt+ArrowDown open
    // must leave DOM focus on the trigger, so this "nothing focused yet --
    // focus the listbox container" fallback must not fire for that open.
    let focused =
        move || open() && !ctx.selectable.collection.any_focused() && !(ctx.keep_trigger_focus)();

    use_effect(move || {
        let Some(listbox_ref) = listbox_ref() else {
            return;
        };
        if focused() {
            spawn(async move {
                _ = listbox_ref.set_focus(true);
            });
        }
    });

    let onkeydown = select_list_onkeydown(ctx);
    // See the web arm's identical construction (docs/backlog.md row 34) for
    // why this is conditional and routed through `merge_attributes` rather
    // than a bare literal alongside `..attributes`.
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{ctx.selectable.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![attributes, labelledby]);

    rsx! {
        div {
            id,
            role: "listbox",
            tabindex: if focused() { "0" } else { "-1" },
            aria_multiselectable: ctx.multi(),

            // Data attributes
            "data-state": if open() { "open" } else { "closed" },

            onmounted: move |evt| listbox_ref.set(Some(evt.data())),
            onkeydown,
            onblur: move |_| {
                if focused() {
                    ctx.selectable.interacted_outside.set(true);
                    ctx.set_open(false);
                }
            },

            ..attributes,
            {children}
        }
    }
}
