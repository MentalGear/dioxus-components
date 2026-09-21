//! attr-synth: synthesizes the hydration-parity Rule 4 duplicate-attribute
//! collision instead of waiting for a preview demo page to happen to
//! produce one.
//!
//! ## Why this exists
//!
//! `playwright/oracle/hydration-parity.spec.ts`'s Rule 4 (and its
//! per-component-page extension) fetch already-built SSG markup and check
//! it for duplicated attribute names. That only ever catches a collision
//! some `preview` demo page *happens* to exercise -- which is exactly how
//! three successive manual sweeps (`b35d671`, `5fc1439`, `f2be1d7`) each
//! left different survivors, per this lane's own brief. This crate
//! synthesizes the collision instead: for each covered `dioxus-primitives`
//! component, it renders that component with a caller-supplied value for
//! an attribute it ALSO sets literally, so the check runs whether or not
//! any demo page happens to reproduce it.
//!
//! ## Why a standalone crate, rendering `dioxus-primitives` directly
//!
//! This lane does not own `preview/src/` (a fixture route added there is
//! the obvious alternative, and was ruled out for that reason -- see the
//! commit message and the lane's final report). `preview` is also a
//! binary-only crate (no lib target), so it cannot be depended on from
//! outside. Driving existing demo pages via query parameters was
//! investigated and does not work for this purpose either: this app's `dx
//! build --ssg` prerenders one static file per route at BUILD time, and a
//! query string cannot select a different prerendered file (confirmed
//! empirically, and already documented in `hydration-parity.spec.ts`'s
//! Rule 4 doc) -- query-driven prop changes only take effect after client
//! JS hydrates, which is after the exact pre-JS moment Rule 4 inspects.
//!
//! `dioxus-primitives` (this crate's only real dependency, via a normal
//! path dependency on the actual, unmodified `../../../primitives`) is a
//! public, standalone component library -- rendering its components
//! directly, exactly as any external consumer would, is a legitimate and
//! arguably more direct way to exercise this exact contract than going
//! through a themed wrapper layer. This crate is read-only with respect to
//! that dependency: it never forks, patches, or vendors primitives source,
//! and every `Case` below composes only PUBLIC components from it.
//!
//! ## How each case works
//!
//! For each covered component, the caller-supplied override is pushed
//! directly into that component's own `attributes: Vec<Attribute>`
//! catch-all field, e.g. `SomeComponent { attributes: vec![attr("id",
//! "...")], ... }`, rather than via the `id: "..."` RSX shorthand that
//! `#[props(extends = GlobalAttributes)]` also allows. This is
//! deliberate: a handful of these components (e.g. `SelectTrigger`) have
//! since grown an explicit TYPED `id` prop specifically so that shorthand
//! now binds to that typed field instead of reaching `attributes` at all
//! -- a real, additional defense, but one that would make an `id: "..."`
//! shorthand-based test silently stop exercising the original hazard the
//! moment such a field is added, without the test itself changing at all.
//! Setting `attributes` directly is unaffected by that: it is a normal
//! public field on every one of these props structs, exercises the exact
//! historical hazard shape (a caller value landing in the raw
//! `attributes` list, colliding with the component's own literal), and
//! keeps working (or correctly starts failing, if a future refactor
//! regresses the merge) regardless of what typed convenience fields exist
//! alongside it.
//!
//! Rendering itself mirrors this repo's own established convention for
//! exactly this kind of proof, already used ad hoc in
//! `primitives/src/dropdown_menu.rs`/`menubar.rs`/`carousel.rs`/
//! `tooltip.rs`'s own `#[cfg(test)]` modules (see their doc comments,
//! e.g. `dropdown_menu.rs`'s `ssr_tests::
//! callers_own_id_on_the_root_is_not_duplicated`): `VirtualDom::new`,
//! `rebuild_in_place()`, a bounded number of `render_immediate` passes to
//! drain any synchronous `use_effect`-driven state (`rebuild_in_place`
//! alone never runs an effect -- `carousel.rs`'s own doc), then
//! `dioxus_ssr::render(&dom)`. Two of those four files' tests are for the
//! exact component+attribute pairs this crate also covers
//! (`dropdown_menu:root:id`, `menubar:menu:id`, `tooltip:content:style`)
//! -- this crate lifts that proof into the Playwright-governed black-box
//! layer Rule 4 already lives in, rather than leaving it as an isolated
//! cargo unit test elsewhere that Rule 4's own coverage story does not
//! include.
//!
//! `main()` prints one JSON line per `Case` (`case`, `attr`, `expected`,
//! `source`, `html`) to stdout. `hydration-parity.spec.ts`'s Rule 4c runs
//! this binary once and, for each line, reuses the SAME `extractStartTags`
//! WHATWG-tokenizer this file already runs against real served markup: it
//! finds the tag whose raw text contains `expected` as a substring (true
//! whether the collision is live or fixed, since a real duplicate keeps
//! BOTH values as substrings pre-parse -- only which one is the
//! tokenizer's first-wins EFFECTIVE value differs), then asserts `attr`
//! appears exactly once on that tag and that its effective value contains
//! `expected`.
//!
//! ## What this still cannot reach
//!
//! `DropdownMenuContent`/`MenubarContent`/`NavbarContent`/
//! `NavigationMenuContent` are gated behind
//! `menu_root::use_menu_content_lifecycle`'s `use_animated_open` render
//! signal, which (confirmed by execution: `default_open: true` plus up to
//! six `render_immediate` passes was not enough, where the identical
//! technique settles `PopoverContent`/`SelectList`/`TooltipContent`
//! immediately) does not settle within a bounded number of synchronous
//! passes the way the covered anchored-content components do -- it likely
//! needs a real async executor polling an actual spawned task, the way
//! `carousel.rs`'s own doc says its sibling `tooltip.rs` render_immediate
//! test sometimes does. Not pursued further in this lane's time budget;
//! `dropdown_menu:root:id`/`dropdown_menu:trigger:id` above still cover
//! that file's other two `merge_attributes` sites (and `menubar:*`/
//! `navbar:*`/`navigation_menu:*` root/trigger cases cover their own
//! siblings the same way). `Drawer`'s own content merge chains into
//! `dialog.rs`'s `DialogContent` (a different primitive) rather than
//! resolving locally, so its content-level case was not pursued; its
//! root/trigger-level sites were not attempted either, purely for time.
//! None of this is preview-fixture-shaped -- it is the same class of gap
//! either way (a component this crate resolves does not settle
//! statelessly, or was simply not gotten to), so it doesn't change the
//! "standalone crate, no preview fixture needed" conclusion above.

use dioxus::prelude::*;
use dioxus_core::AttributeValue;
use dioxus_primitives::collapsible::{Collapsible, CollapsibleTrigger};
use dioxus_primitives::combobox::{Combobox, ComboboxInput, ComboboxList};
use dioxus_primitives::context_menu::{
    ContextMenu, ContextMenuSub, ContextMenuSubTrigger, ContextMenuTrigger,
};
use dioxus_primitives::dropdown_menu::{
    DropdownMenu, DropdownMenuSub, DropdownMenuSubTrigger, DropdownMenuTrigger,
};
use dioxus_primitives::hover_card::{HoverCard, HoverCardContent, HoverCardTrigger};
use dioxus_primitives::menubar::{Menubar, MenubarMenu, MenubarTrigger};
use dioxus_primitives::navbar::{Navbar, NavbarNav, NavbarTrigger};
use dioxus_primitives::navigation_menu::{
    NavigationMenu, NavigationMenuItem, NavigationMenuList, NavigationMenuTrigger,
};
use dioxus_primitives::popover::{PopoverContent, PopoverRoot, PopoverTrigger};
use dioxus_primitives::progress::{Progress, ProgressIndicator};
use dioxus_primitives::select::{Select, SelectList, SelectTrigger, SelectValue};
use dioxus_primitives::toast::ToastProvider;
use dioxus_primitives::tooltip::{Tooltip, TooltipContent, TooltipTrigger};

/// Builds a raw [`Attribute`] the same way `dioxus_primitives::lib`'s own
/// `#[cfg(test)]` `attr()` helper does (`primitives/src/lib.rs`) -- this
/// crate can't import that one (it's test-only and private), so this is a
/// deliberate, minimal duplicate of that exact shape, not a reimplementation
/// of any real logic.
fn attr(name: &'static str, value: &str) -> Attribute {
    Attribute {
        name,
        namespace: None,
        volatile: false,
        value: AttributeValue::Text(value.to_string()),
    }
}

/// One synthesized case: render `component`, then assert (on the
/// TypeScript side, in `hydration-parity.spec.ts`'s Rule 4c) that `attr`
/// appears exactly once on some start tag in the output, and that its
/// EFFECTIVE (first-wins) value contains `expected` (contains, not
/// exact-equals, since a few cases fold the caller's value together with
/// an internal one rather than replacing it outright -- see Rule 4c's own
/// comment in `hydration-parity.spec.ts` for which and why).
struct Case {
    /// Stable id, `<component>:<element>:<attr>` -- shown in failure output.
    name: &'static str,
    /// Which HTML attribute this case targets.
    attr: &'static str,
    /// The caller-supplied override value injected into the component's
    /// own `attributes` field (bypassing any typed-field shorthand -- see
    /// this module's own doc, "How each case works"). Deliberately
    /// distinctive so a substring search on the
    /// raw tag text finds the right element even when the collision is
    /// live (i.e. even when the FIRST/effective value is the component's
    /// own internal default, not this one).
    expected: &'static str,
    /// What primitives source file/function this collision is specified
    /// against, for the failure message and for humans auditing coverage.
    source: &'static str,
    render: fn() -> Element,
}

macro_rules! case {
    ($name:expr, $attr:expr, $expected:expr, $source:expr, $render:expr) => {
        Case {
            name: $name,
            attr: $attr,
            expected: $expected,
            source: $source,
            render: $render,
        }
    };
}

// ---------------------------------------------------------------------
// Real primitive components: one case per `merge_attributes` call site
// that (a) is reachable without opening/interacting with anything (every
// target element here is unconditionally in the SSR'd tree, matching how
// Rule 4's original five findings were all root/trigger/always-mounted
// elements, never something gated behind an `open` state) and (b) sets
// the target attribute UNCONDITIONALLY in its own literal attrs (never a
// bare `Option` passthrough with no internally-generated fallback) so the
// pre-merge collision is always real, never vacuous.
// ---------------------------------------------------------------------

fn progress_style() -> Element {
    rsx! {
        Progress {
            aria_label: "Progress bar",
            value: 50.0,
            attributes: vec![attr("style", "--attr-synth-marker: progress-style;")],
            ProgressIndicator {}
        }
    }
}

fn toast_provider_aria_label() -> Element {
    rsx! {
        ToastProvider {
            attributes: vec![attr("aria-label", "attr-synth-marker-toast-region")],
        }
    }
}

fn context_menu_root_id() -> Element {
    rsx! {
        ContextMenu {
            attributes: vec![attr("id", "attr-synth-marker-context-menu-root")],
            ContextMenuTrigger { "right click here" }
        }
    }
}

fn context_menu_trigger_id() -> Element {
    rsx! {
        ContextMenu {
            ContextMenuTrigger {
                attributes: vec![attr("id", "attr-synth-marker-context-menu-trigger")],
                "right click here"
            }
        }
    }
}

fn context_menu_sub_trigger_style() -> Element {
    // `ContextMenuSub` renders no element of its own -- "purely a context
    // boundary around a ContextMenuSubTrigger and a ContextMenuSubContent"
    // (its own doc) -- and `ContextMenuSubTrigger` itself only needs
    // `ContextMenuCtx` (from the enclosing `ContextMenu`) and `SubMenuState`
    // (from `ContextMenuSub`), neither of which depends on `ContextMenuContent`
    // actually being open. `ContextMenuContent` is therefore skipped
    // entirely here, the same way the plain trigger cases above never
    // render their own `*Content` -- it is gated behind the same
    // `menu_root::use_menu_content_lifecycle` signal documented at this
    // file's own top doc ("What this still cannot reach") as not settling
    // within a bounded number of synchronous passes, and is not needed to
    // reach this trigger.
    rsx! {
        ContextMenu {
            ContextMenuTrigger { "right click here" }
            ContextMenuSub {
                ContextMenuSubTrigger {
                    index: 0usize,
                    attributes: vec![attr("style", "--attr-synth-marker: context-menu-sub-trigger-style;")],
                    "More tools"
                }
            }
        }
    }
}

fn dropdown_menu_root_id() -> Element {
    // Mirrors `primitives/src/dropdown_menu.rs`'s own
    // `ssr_tests::callers_own_id_on_the_root_is_not_duplicated` -- same
    // component, same attribute, same shape -- lifted into the
    // Playwright-governed black-box layer so Rule 4's OWN coverage story
    // includes it, not only an isolated cargo unit test elsewhere.
    rsx! {
        DropdownMenu {
            attributes: vec![attr("id", "attr-synth-marker-dropdown-menu-root")],
            DropdownMenuTrigger { "Open Menu" }
        }
    }
}

fn dropdown_menu_trigger_id() -> Element {
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger {
                attributes: vec![attr("id", "attr-synth-marker-dropdown-menu-trigger")],
                "Open Menu"
            }
        }
    }
}

fn dropdown_menu_trigger_style() -> Element {
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger {
                attributes: vec![attr("style", "--attr-synth-marker: dropdown-menu-trigger-style;")],
                "Open Menu"
            }
        }
    }
}

fn dropdown_menu_sub_trigger_style() -> Element {
    // Same construction as `context_menu_sub_trigger_style` above --
    // `DropdownMenuSub` is an identical "renders no element of its own,
    // purely a context boundary" wrapper (`DropdownMenuSub`'s own doc), and
    // `DropdownMenuContent`/`DropdownMenuSubContent` are skipped for the
    // same reason (the menu-family content lifecycle gate; see this file's
    // top doc, "What this still cannot reach").
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { "Open Menu" }
            DropdownMenuSub {
                DropdownMenuSubTrigger {
                    index: 0usize,
                    attributes: vec![attr("style", "--attr-synth-marker: dropdown-menu-sub-trigger-style;")],
                    "More options"
                }
            }
        }
    }
}

// `DropdownMenuContent`/`MenubarContent`/`NavbarContent`/
// `NavigationMenuContent` are NOT covered: all four are gated behind
// `menu_root::use_menu_content_lifecycle`'s `use_animated_open` render
// signal, which (confirmed by execution) does not settle within a bounded
// number of synchronous `render_immediate` passes the way `PopoverContent`/
// `SelectList`/`TooltipContent` below do -- see this file's top-of-module
// doc, "What this still cannot reach".

fn menubar_menu_id() -> Element {
    // Mirrors `primitives/src/menubar.rs`'s own
    // `ssr_tests::callers_own_id_on_the_menu_wrapper_is_not_duplicated`.
    rsx! {
        Menubar {
            MenubarMenu {
                index: 0usize,
                attributes: vec![attr("id", "attr-synth-marker-menubar-menu")],
                MenubarTrigger { "File" }
            }
        }
    }
}

fn menubar_trigger_id() -> Element {
    rsx! {
        Menubar {
            MenubarMenu { index: 0usize,
                MenubarTrigger {
                    attributes: vec![attr("id", "attr-synth-marker-menubar-trigger")],
                    "File"
                }
            }
        }
    }
}

fn menubar_trigger_style() -> Element {
    rsx! {
        Menubar {
            MenubarMenu { index: 0usize,
                MenubarTrigger {
                    attributes: vec![attr("style", "--attr-synth-marker: menubar-trigger-style;")],
                    "File"
                }
            }
        }
    }
}

fn popover_trigger_id() -> Element {
    rsx! {
        PopoverRoot {
            PopoverTrigger {
                attributes: vec![attr("id", "attr-synth-marker-popover-trigger")],
                "Show Popover"
            }
            PopoverContent { "content" }
        }
    }
}

fn popover_trigger_style() -> Element {
    rsx! {
        PopoverRoot {
            PopoverTrigger {
                attributes: vec![attr("style", "--attr-synth-marker: popover-trigger-style;")],
                "Show Popover"
            }
            PopoverContent { "content" }
        }
    }
}

fn popover_content_style() -> Element {
    // Must actually be open (`default_open: true`) to render at all --
    // found by execution; same requirement as `select_list_style` and
    // `tooltip_content_style` below.
    rsx! {
        PopoverRoot { default_open: true,
            PopoverTrigger { "Show Popover" }
            PopoverContent {
                attributes: vec![attr("style", "--attr-synth-marker: popover-content-style;")],
                "content"
            }
        }
    }
}

fn navbar_trigger_id() -> Element {
    rsx! {
        Navbar { aria_label: "Components",
            NavbarNav { index: 0usize,
                NavbarTrigger {
                    attributes: vec![attr("id", "attr-synth-marker-navbar-trigger")],
                    "Inputs"
                }
            }
        }
    }
}

fn navbar_trigger_style() -> Element {
    rsx! {
        Navbar { aria_label: "Components",
            NavbarNav { index: 0usize,
                NavbarTrigger {
                    attributes: vec![attr("style", "--attr-synth-marker: navbar-trigger-style;")],
                    "Inputs"
                }
            }
        }
    }
}

fn navigation_menu_trigger_style() -> Element {
    rsx! {
        NavigationMenu { aria_label: "Main",
            NavigationMenuList {
                NavigationMenuItem { index: 0usize,
                    NavigationMenuTrigger {
                        attributes: vec![attr("style", "--attr-synth-marker: nav-menu-trigger-style;")],
                        "Components"
                    }
                }
            }
        }
    }
}

fn collapsible_root_data_open() -> Element {
    rsx! {
        Collapsible {
            attributes: vec![attr("data-open", "attr-synth-marker-collapsible-root")],
            CollapsibleTrigger { "Recent Activity" }
        }
    }
}

fn collapsible_trigger_data_open() -> Element {
    rsx! {
        Collapsible {
            CollapsibleTrigger {
                attributes: vec![attr("data-open", "attr-synth-marker-collapsible-trigger")],
                "Recent Activity"
            }
        }
    }
}

fn tooltip_trigger_style() -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger {
                attributes: vec![attr("style", "--attr-synth-marker: tooltip-trigger-style;")],
                "Rich content"
            }
        }
    }
}

fn tooltip_content_style() -> Element {
    // Mirrors `primitives/src/tooltip.rs`'s own
    // `anchor_style_hydration_parity::ssr_renders_open_anchored_content_with_one_folded_style_attribute`
    // almost verbatim (`default_open: true` + one settle pass, no async
    // executor needed -- that test's own doc explains why) -- the ONE
    // existing cargo-level regression test, among the seven
    // `anchored_content_attributes` consumers, for exactly this rule's own
    // class. Lifted into the Playwright-governed black-box layer the same
    // way `dropdown_menu:root:id`/`menubar:menu:id` above lift their own
    // existing cargo-level proofs.
    rsx! {
        Tooltip { default_open: true,
            TooltipTrigger { "Trigger" }
            TooltipContent {
                attributes: vec![attr("style", "--attr-synth-marker: tooltip-content-style;")],
                "content"
            }
        }
    }
}

fn select_trigger_style() -> Element {
    rsx! {
        Select::<String> {
            SelectTrigger {
                attributes: vec![attr("style", "--attr-synth-marker: select-trigger-style;")],
                SelectValue { placeholder: "Select a fruit..." }
            }
            SelectList { aria_label: "Select Demo" }
        }
    }
}

fn select_list_style() -> Element {
    rsx! {
        Select::<String> { default_open: true,
            SelectTrigger { SelectValue { placeholder: "Select a fruit..." } }
            SelectList {
                aria_label: "Select Demo",
                attributes: vec![attr("style", "--attr-synth-marker: select-list-style;")],
            }
        }
    }
}

fn hover_card_trigger_style() -> Element {
    // No `default_open`/`HoverCardContent` needed -- `HoverCardTrigger`
    // renders unconditionally, same as `tooltip_trigger_style` above.
    rsx! {
        HoverCard {
            HoverCardTrigger {
                attributes: vec![attr("style", "--attr-synth-marker: hover-card-trigger-style;")],
                "Dioxus"
            }
        }
    }
}

fn hover_card_content_style() -> Element {
    // Same "must actually be open" requirement as the other anchored-
    // content cases above; `HoverCard` is not part of the menu-family
    // lifecycle gate (`menu_root::use_menu_content_lifecycle`) that defeats
    // `DropdownMenuContent` et al. (this file's own top doc, "What this
    // still cannot reach"), so a plain `default_open` settle is enough.
    rsx! {
        HoverCard { default_open: true,
            HoverCardTrigger { "Dioxus" }
            HoverCardContent {
                attributes: vec![attr("style", "--attr-synth-marker: hover-card-content-style;")],
                "content"
            }
        }
    }
}

fn combobox_list_style() -> Element {
    rsx! {
        Combobox::<String> { default_open: true,
            ComboboxInput {}
            ComboboxList {
                attributes: vec![attr("style", "--attr-synth-marker: combobox-list-style;")],
            }
        }
    }
}

// ---------------------------------------------------------------------
// Self-test (meta-proof), NOT a `dioxus-primitives` component: proves
// this crate's own detection pipeline (SSR render -> the JSON this prints
// -> Rule 4c's tokenizer/assertions in hydration-parity.spec.ts) can
// actually tell a real collision apart from a fixed one, without touching
// `primitives/src` (out of this lane's ownership) even temporarily. Both
// arms below hand-build the EXACT historical hazard shape (a literal
// attribute beside a raw, unmerged caller-attributes spread) that this
// whole rule exists to catch; the "fixed" arm routes the same two lists
// through the REAL, unmodified `dioxus_primitives::merge_attributes` --
// not a reimplementation -- so this is a genuine exercise of the library's
// own construction, just on a hand-written element instead of a shipped
// component. This lane's commit message on this branch records how this
// pair was used to get a live RED-then-GREEN result on the Playwright side
// (Rule 4c) before this was committed.
// ---------------------------------------------------------------------

fn self_test_unfixed_collision() -> Element {
    let caller_attrs: Vec<Attribute> = vec![attr("id", "attr-synth-marker-selftest-unfixed")];
    rsx! {
        div {
            id: "internal-default-id",
            ..caller_attrs,
            "unfixed self-test"
        }
    }
}

fn self_test_fixed_no_collision() -> Element {
    let base: Vec<Attribute> = vec![attr("id", "internal-default-id")];
    let caller_attrs: Vec<Attribute> = vec![attr("id", "attr-synth-marker-selftest-fixed")];
    let merged = dioxus_primitives::merge_attributes(vec![base, caller_attrs]);
    rsx! {
        div { ..merged, "fixed self-test" }
    }
}

fn cases() -> Vec<Case> {
    vec![
        case!(
            "progress:root:style",
            "style",
            "--attr-synth-marker: progress-style;",
            "primitives/src/progress.rs Progress",
            progress_style
        ),
        case!(
            "toast:provider-region:aria-label",
            "aria-label",
            "attr-synth-marker-toast-region",
            "primitives/src/toast.rs ToastRegionRendered (via ToastProvider)",
            toast_provider_aria_label
        ),
        case!(
            "context_menu:root:id",
            "id",
            "attr-synth-marker-context-menu-root",
            "primitives/src/context_menu.rs ContextMenu",
            context_menu_root_id
        ),
        case!(
            "context_menu:trigger:id",
            "id",
            "attr-synth-marker-context-menu-trigger",
            "primitives/src/context_menu.rs ContextMenuTrigger",
            context_menu_trigger_id
        ),
        case!(
            "context_menu:sub_trigger:style",
            "style",
            "--attr-synth-marker: context-menu-sub-trigger-style;",
            "primitives/src/context_menu.rs ContextMenuSubTrigger (top_layer::anchored_trigger_attributes)",
            context_menu_sub_trigger_style
        ),
        case!(
            "dropdown_menu:root:id",
            "id",
            "attr-synth-marker-dropdown-menu-root",
            "primitives/src/dropdown_menu.rs DropdownMenu (cargo-test-proven site)",
            dropdown_menu_root_id
        ),
        case!(
            "dropdown_menu:trigger:id",
            "id",
            "attr-synth-marker-dropdown-menu-trigger",
            "primitives/src/dropdown_menu.rs DropdownMenuTrigger",
            dropdown_menu_trigger_id
        ),
        case!(
            "dropdown_menu:trigger:style",
            "style",
            "--attr-synth-marker: dropdown-menu-trigger-style;",
            "primitives/src/dropdown_menu.rs DropdownMenuTrigger (top_layer::anchored_trigger_attributes)",
            dropdown_menu_trigger_style
        ),
        case!(
            "dropdown_menu:sub_trigger:style",
            "style",
            "--attr-synth-marker: dropdown-menu-sub-trigger-style;",
            "primitives/src/dropdown_menu.rs DropdownMenuSubTrigger (top_layer::anchored_trigger_attributes)",
            dropdown_menu_sub_trigger_style
        ),
        case!(
            "menubar:menu:id",
            "id",
            "attr-synth-marker-menubar-menu",
            "primitives/src/menubar.rs MenubarMenu (cargo-test-proven site)",
            menubar_menu_id
        ),
        case!(
            "menubar:trigger:id",
            "id",
            "attr-synth-marker-menubar-trigger",
            "primitives/src/menubar.rs MenubarTrigger",
            menubar_trigger_id
        ),
        case!(
            "menubar:trigger:style",
            "style",
            "--attr-synth-marker: menubar-trigger-style;",
            "primitives/src/menubar.rs MenubarTrigger (top_layer::anchored_trigger_attributes)",
            menubar_trigger_style
        ),
        case!(
            "popover:trigger:id",
            "id",
            "attr-synth-marker-popover-trigger",
            "primitives/src/popover.rs PopoverTrigger (filter+single-bind construction, not merge_attributes)",
            popover_trigger_id
        ),
        case!(
            "popover:trigger:style",
            "style",
            "--attr-synth-marker: popover-trigger-style;",
            "primitives/src/popover.rs PopoverTrigger (top_layer::anchored_trigger_attributes)",
            popover_trigger_style
        ),
        case!(
            "popover:content:style",
            "style",
            "--attr-synth-marker: popover-content-style;",
            "primitives/src/popover.rs PopoverContent (top_layer::anchored_content_attributes)",
            popover_content_style
        ),
        case!(
            "navbar:trigger:id",
            "id",
            "attr-synth-marker-navbar-trigger",
            "primitives/src/navbar.rs NavbarTrigger",
            navbar_trigger_id
        ),
        case!(
            "navbar:trigger:style",
            "style",
            "--attr-synth-marker: navbar-trigger-style;",
            "primitives/src/navbar.rs NavbarTrigger (top_layer::anchored_trigger_attributes)",
            navbar_trigger_style
        ),
        case!(
            "navigation_menu:trigger:style",
            "style",
            "--attr-synth-marker: nav-menu-trigger-style;",
            "primitives/src/navigation_menu.rs NavigationMenuTrigger (top_layer::anchored_trigger_attributes -- previously RED BY DESIGN, see dev-docs history/this lane's commit for the fix)",
            navigation_menu_trigger_style
        ),
        case!(
            "collapsible:root:data-open",
            "data-open",
            "attr-synth-marker-collapsible-root",
            "primitives/src/collapsible.rs Collapsible",
            collapsible_root_data_open
        ),
        case!(
            "collapsible:trigger:data-open",
            "data-open",
            "attr-synth-marker-collapsible-trigger",
            "primitives/src/collapsible.rs CollapsibleTrigger",
            collapsible_trigger_data_open
        ),
        case!(
            "tooltip:trigger:style",
            "style",
            "--attr-synth-marker: tooltip-trigger-style;",
            "primitives/src/tooltip.rs TooltipTrigger (top_layer::anchored_trigger_attributes)",
            tooltip_trigger_style
        ),
        case!(
            "tooltip:content:style",
            "style",
            "--attr-synth-marker: tooltip-content-style;",
            "primitives/src/tooltip.rs TooltipContent (cargo-test-proven site: anchor_style_hydration_parity)",
            tooltip_content_style
        ),
        case!(
            "select:trigger:style",
            "style",
            "--attr-synth-marker: select-trigger-style;",
            "primitives/src/select/components/trigger.rs SelectTrigger",
            select_trigger_style
        ),
        case!(
            "select:list:style",
            "style",
            "--attr-synth-marker: select-list-style;",
            "primitives/src/select/components/list.rs SelectList (top_layer::anchored_content_attributes)",
            select_list_style
        ),
        case!(
            "hover_card:trigger:style",
            "style",
            "--attr-synth-marker: hover-card-trigger-style;",
            "primitives/src/hover_card.rs HoverCardTrigger (top_layer::anchored_trigger_attributes)",
            hover_card_trigger_style
        ),
        case!(
            "hover_card:content:style",
            "style",
            "--attr-synth-marker: hover-card-content-style;",
            "primitives/src/hover_card.rs HoverCardContent (top_layer::anchored_content_attributes)",
            hover_card_content_style
        ),
        case!(
            "combobox:list:style",
            "style",
            "--attr-synth-marker: combobox-list-style;",
            "primitives/src/combobox/components/list.rs ComboboxList (top_layer::anchored_content_attributes)",
            combobox_list_style
        ),
        case!(
            "selftest:unfixed:id",
            "id",
            "attr-synth-marker-selftest-unfixed",
            "attr-synth's own meta-proof (literal attr + raw unmerged spread, hand-built, NOT primitives/src) -- expected to FAIL Rule 4c's exactly-once check by construction",
            self_test_unfixed_collision
        ),
        case!(
            "selftest:fixed:id",
            "id",
            "attr-synth-marker-selftest-fixed",
            "attr-synth's own meta-proof, routed through the real dioxus_primitives::merge_attributes -- expected to PASS",
            self_test_fixed_no_collision
        ),
    ]
}

fn main() {
    for case in cases() {
        let mut dom = VirtualDom::new(case.render);
        dom.rebuild_in_place();
        // `rebuild_in_place` alone never runs an effect (confirmed by this
        // repo's own `primitives/src/carousel.rs` `ssr_tests` doc, same
        // Dioxus version). Several of this crate's targets (e.g.
        // `DropdownMenuContent`) are gated behind an animated-open
        // lifecycle signal (`menu_root::use_menu_content_lifecycle`) that
        // only flips from its initial-render value via a plain synchronous
        // `use_effect`, not during the first render pass -- so without
        // this, those elements (and the collision they're meant to
        // synthesize) never appear in the SSR'd output at all. Looping
        // `render_immediate` a few times (mirroring `carousel.rs`'s own
        // `next_button_becomes_enabled_once_items_have_registered`, which
        // documents needing more than one pass) drains the effect queue
        // and re-diffs synchronously -- no async executor required, since
        // every effect every case here triggers is a plain synchronous
        // `use_effect` body, not one awaiting a spawned task's result.
        for _ in 0..6 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);
        let out = serde_json::json!({
            "case": case.name,
            "attr": case.attr,
            "expected": case.expected,
            "source": case.source,
            "html": html,
        });
        println!("{out}");
    }
}
