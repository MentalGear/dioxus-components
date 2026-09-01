//! ComboboxList component.

use dioxus::prelude::*;
#[cfg(feature = "web")]
use dioxus_attributes::attributes;

use super::super::context::ComboboxContext;
use crate::listbox::use_listbox_container;
#[cfg(feature = "web")]
use crate::merge_attributes;

/// Props for [`ComboboxList`].
#[derive(Props, Clone, PartialEq)]
pub struct ComboboxListProps {
    /// Optional id for the list element.
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Children, typically [`ComboboxOption`](super::option::ComboboxOption)s
    /// and an optional [`ComboboxEmpty`](super::empty::ComboboxEmpty).
    pub children: Element,
}

/// Listbox that contains the visible options.
#[component]
pub fn ComboboxList(props: ComboboxListProps) -> Element {
    let ctx = use_context::<ComboboxContext>();
    let listbox = use_listbox_container(props.id, ctx.selectable);
    let render = listbox.render;

    rsx! {
        if render() {
            ComboboxListRendered {
                id: listbox.id.cloned(),
                attributes: props.attributes,
                children: props.children,
            }
        } else {
            {props.children}
        }
    }
}

/// Web arm (Migration A slice 3/3, final): promote the listbox to the top
/// layer via `popover="manual"`, anchored to `ComboboxInput` below/
/// start-aligned -- the same top-layer fix already shipped for
/// `Tooltip`/`HoverCard`/`Popover`/`DropdownMenu`/`Menubar`/`Select`
/// (docs/plan.md Phase 4.4, Migration A slices 1-3), matching this
/// component's pre-migration `top: calc(100% + 0.25rem); left: 0` CSS
/// (`../../../../preview/src/components/combobox/style.css`).
///
/// ## `manual`, reversed from `auto` by execution
///
/// This slice's first cut used `auto` -- Combobox has no `Select`-style
/// DOM-focus/blur delicacy to worry about (`ComboboxInput` keeps focus
/// throughout via an `aria-activedescendant` model, never blurring into
/// this listbox), so `auto`'s light dismiss looked like free, strictly
/// additive insurance on top of `ComboboxInput`'s own `onblur`/`onkeydown`
/// dismissal. Confirmed wrong by execution: `combobox.spec.ts`'s "dynamic
/// option removal updates filtering and keyboard selection" test went red
/// -- clicking an *external* "Toggle SvelteKit" button while the list was
/// open (a supported pattern: that button's own `onpointerdown` already
/// calls `prevent_default()` specifically so it does not blur/close the
/// combobox) unexpectedly closed the list anyway. WHATWG light dismiss
/// classifies *any* pointerdown outside the popover's own DOM subtree as
/// "outside," full stop -- confirmed by execution that neither
/// `prevent_default()` on that pointerdown nor a `popovertarget`
/// association from `ComboboxInput` to this listbox stops it (`auto` has
/// no notion of "this button is allowed to interact with me without
/// closing me" beyond "is it a descendant"). That is fatal specifically for
/// Combobox's own supported use case -- an external control manipulating
/// state while the list stays open -- in a way it is not for `Select`
/// (whose own suite has no such case, and for which "any outside click
/// closes it" is the same behavior a plain HTML `<select>` already has).
/// `manual` sidesteps the whole question: WHATWG HTML never light-dismisses
/// a manual popover, so `ComboboxInput`'s own `onblur`/`onkeydown` stay the
/// *only* dismissal path -- unchanged from pre-migration, and immune to
/// this class of external-click interference -- exactly
/// `ContextMenuContentRendered`'s identical `manual` reasoning
/// (`context_menu.rs`).
///
/// The one behavior this reversal gives up: `auto`'s light dismiss would
/// additionally have closed the list on a click over a *non-focusable*
/// area (one `ComboboxInput`'s `onblur` alone cannot see, since nothing
/// there ever takes focus). No `combobox.spec.ts` case depends on that gap
/// being closed, so trading it away for the external-control case above is
/// a clear net win, not a compromise made blind.
///
/// **Animation**: still does *not* use [`crate::top_layer::use_popover_sync`]
/// bound to the raw `open` signal -- see
/// [`crate::top_layer::use_popover_shown_while_mounted`]'s doc for the
/// animate-out race that binding would otherwise cause with
/// [`crate::use_animated_open`] (`ComboboxList`, above). Confirmed by
/// execution to matter here specifically: `combobox.spec.ts`'s "keeps
/// filtered options during keyboard close animation" test (an Enter-key,
/// script-driven close) requires the `data-state="closed"` element to stay
/// visible through its exit animation, which binding
/// `showPopover()`/`hidePopover()` to the raw `open` signal breaks (see the
/// linked doc for the exact mechanism) -- this reasoning is orthogonal to
/// the `auto`-vs-`manual` reversal above and holds either way.
#[cfg(feature = "web")]
#[component]
fn ComboboxListRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let ctx: ComboboxContext = use_context();
    let mut selectable = ctx.selectable;
    let open = selectable.open;

    crate::top_layer::use_popover_shown_while_mounted(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            selectable.set_open(is_open);
        }),
    );
    // JS-measured static positioning fallback for engines without CSS
    // Anchor Positioning -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `side`/`align`/gap match this component's pre-migration CSS
    // (`top: calc(100% + 0.25rem); left: 0` == a 4px gap past the input's
    // own bottom edge): anchored below the input, left-aligned with it,
    // the same bottom/start convention every other migrated listbox here
    // uses.
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        ctx.input_id.cloned(),
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
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-combobox"
        }),
    ]);

    rsx! {
        div {
            id: id.clone(),
            role: "listbox",
            popover: crate::top_layer::PopoverKind::Manual.as_str(),
            style: crate::top_layer::position_anchor_style(&ctx.input_id.cloned()),
            "data-state": if open() { "open" } else { "closed" },
            onpointerdown: move |event| {
                event.prevent_default();
            },
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice -- Blitz has no
/// popover-API support at all, so this stays the functional floor, a plain,
/// always-in-flow `div`.
#[cfg(not(feature = "web"))]
#[component]
fn ComboboxListRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let ctx: ComboboxContext = use_context();
    let open = ctx.selectable.open;

    rsx! {
        div {
            id,
            role: "listbox",
            "data-state": if open() { "open" } else { "closed" },
            onpointerdown: move |event| {
                event.prevent_default();
            },
            ..attributes,
            {children}
        }
    }
}
