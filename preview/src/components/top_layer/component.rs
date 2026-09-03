use dioxus::prelude::*;
use dioxus_primitives::alert_dialog::{AlertDialogContent, AlertDialogRoot, AlertDialogTitle};
use dioxus_primitives::dialog::{DialogContent, DialogRoot, DialogTitle};
use dioxus_primitives::context_menu::{ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger};
use dioxus_primitives::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger};
use dioxus_primitives::hover_card::{HoverCard, HoverCardContent, HoverCardTrigger};
use dioxus_primitives::menubar::{Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger};
use dioxus_primitives::navbar::{Navbar, NavbarContent, NavbarItem, NavbarNav, NavbarTrigger};
use dioxus_primitives::popover::{PopoverContent, PopoverRoot, PopoverTrigger};
use dioxus_primitives::tooltip::{Tooltip, TooltipContent, TooltipTrigger};
use dioxus_primitives::select::{Select, SelectList, SelectOption, SelectTrigger, SelectValue};
use dioxus_primitives::combobox::{Combobox, ComboboxInput, ComboboxList, ComboboxOption};
use dioxus_primitives::toast::{use_toast, ToastOptions, ToastProvider};
use dioxus_primitives::ContentSide;

#[css_module("/src/components/top_layer/style.css")]
struct Styles;

/// A fixture for the Phase 4.4 (docs/plan.md) top-layer oracle
/// (`playwright/oracle/tier2-html/top-layer.spec.ts`).
///
/// Every rule the oracle checks is calibrated against a native reference --
/// a plain `<div popover>` + `<button popovertarget>`, the browser's own
/// implementation of the exact same WHATWG HTML feature our components now
/// use -- placed in the same fixture, tier-2 style (see
/// `docs/conformance-harness.md` and `preview/src/components/form` for the
/// established pattern).
#[component]
pub fn TopLayerFixture() -> Element {
    // Additive fixture for the Phase 4.2 oracle
    // (`playwright/oracle/tier2-html/native-dialog.spec.ts`): the modal
    // `Dialog` renders a real `<dialog>` + `showModal()` on the web arm
    // (docs/plan.md Phase 4.2). Calibrated against a plain native `<dialog>`
    // opened by a single `showModal()` call -- there is no fully declarative
    // opener for `<dialog>` here (the Invoker Commands API's
    // `command="show-modal"` would give one, but its cross-engine support
    // wasn't something this slice could verify) -- so the reference below
    // has exactly one line of JS to open it and closes natively via
    // `<form method="dialog">`, same spirit as the `popovertarget` reference
    // above: no library state or logic under test, just the platform
    // feature.
    let mut dialog_clip_open = use_signal(|| false);
    let mut dialog_inert_open = use_signal(|| false);
    let mut bg_clicks = use_signal(|| 0u32);
    // Additive fixture for the two-engine overlay architecture completion
    // (modal `Popover` -> native `<dialog>`/`showModal()` on the web arm):
    // its own background-inertness click counter, mirroring `bg_clicks`
    // above but kept separate so the two sections' assertions can never
    // cross-contaminate each other's counts.
    let mut popover_modal_bg_clicks = use_signal(|| 0u32);

    rsx! {
        div { class: Styles::dx_top_layer_fixture,

            section { class: Styles::dx_top_layer_section,
                h2 { "Clipping escape" }
                p { class: Styles::dx_top_layer_hint,
                    "Every trigger below sits inside "
                    code { "#clip-box" }
                    ", an ancestor with "
                    code { "overflow: hidden; height: 60px; transform: translateZ(0)" }
                    ". Each content panel sets an explicit "
                    code { "min-height: 100px" }
                    ", taller than the clip -- open one and it should render fully "
                    "visible outside the clip (WHATWG HTML §popover, top layer), not "
                    "cut off at 60px."
                }
                div { id: "clip-box", class: Styles::dx_top_layer_clip_box,

                    Tooltip {
                        TooltipTrigger { id: "clip-tooltip-trigger", "Tooltip trigger" }
                        TooltipContent {
                            id: "clip-tooltip-content",
                            style: "min-height: 100px;",
                            "Tooltip content for the clipping-escape rule."
                        }
                    }

                    HoverCard {
                        HoverCardTrigger { id: "clip-hovercard-trigger", "HoverCard trigger" }
                        HoverCardContent {
                            id: "clip-hovercard-content",
                            style: "min-height: 100px;",
                            "HoverCard content for the clipping-escape rule."
                        }
                    }

                    PopoverRoot { id: "clip-popover-root", is_modal: false,
                        PopoverTrigger { id: "clip-popover-trigger", "Popover trigger" }
                        PopoverContent {
                            id: "clip-popover-content",
                            style: "min-height: 100px;",
                            "Popover content for the clipping-escape rule."
                        }
                    }

                    // DropdownMenu (docs/backlog.md item 2): web-arm
                    // `popover="auto"` migration, added alongside the three
                    // Phase 4.4 exemplars above -- see this rule's addition
                    // in `top-layer.spec.ts` (RED before the migration:
                    // `DropdownMenuContent` was a plain, non-`popover` div,
                    // so it clipped exactly like the pre-4.4 behavior these
                    // three did).
                    DropdownMenu { id: "clip-dropdown-menu-root",
                        DropdownMenuTrigger { id: "clip-dropdown-menu-trigger", "DropdownMenu trigger" }
                        DropdownMenuContent {
                            id: "clip-dropdown-menu-content",
                            style: "min-height: 100px;",
                            DropdownMenuItem::<String> {
                                value: "one".to_string(),
                                index: 0usize,
                                on_select: move |_: String| {},
                                "Item one"
                            }
                        }
                    }

                    // Migration A slice 2/3: ContextMenu's web arm ->
                    // `popover="manual"` (`ContextMenuContentRendered`,
                    // `context_menu.rs`). Written RED first against the
                    // pre-migration plain, `position: fixed`-only div
                    // (confirmed by execution: it clipped at the 60px
                    // ancestor exactly like DropdownMenu did pre-4.4/pre-
                    // slice-2). Opened by right-click, like every other
                    // `ContextMenu` in this workspace.
                    ContextMenu { id: "clip-context-menu-root",
                        ContextMenuTrigger { id: "clip-context-menu-trigger", "ContextMenu trigger (right-click)" }
                        ContextMenuContent {
                            id: "clip-context-menu-content",
                            style: "min-height: 100px;",
                            ContextMenuItem {
                                value: "one".to_string(),
                                index: 0usize,
                                on_select: move |_: String| {},
                                "Item one"
                            }
                        }
                    }

                    // Migration A slice 2/3: Menubar menus' web arm ->
                    // `popover="auto"`, anchored to their own trigger
                    // (`MenubarContentRendered`, `menubar.rs`). Written RED
                    // first against the pre-migration plain,
                    // `position: absolute`-only div (confirmed by
                    // execution: it clipped at the 60px ancestor exactly
                    // like DropdownMenu did pre-4.4/pre-slice-2).
                    Menubar { id: "clip-menubar-root",
                        MenubarMenu { index: 0usize,
                            MenubarTrigger { id: "clip-menubar-trigger", "Menubar trigger" }
                            MenubarContent {
                                id: "clip-menubar-content",
                                style: "min-height: 100px;",
                                MenubarItem {
                                    index: 0usize,
                                    value: "one".to_string(),
                                    on_select: move |_: String| {},
                                    "Item one"
                                }
                            }
                        }
                    }

                    // Migration A slice 3/3 (final): Select's web arm ->
                    // `popover="auto"` (`SelectListRendered`, `list.rs`).
                    // Written RED first against the pre-migration plain div
                    // (confirmed by execution: it clipped at the 60px
                    // ancestor exactly like the others did pre-migration).
                    Select::<String> { id: "clip-select-root",
                        SelectTrigger { id: "clip-select-trigger", SelectValue { placeholder: "Select trigger" } }
                        SelectList {
                            id: "clip-select-content",
                            style: "min-height: 100px;",
                            SelectOption::<String> {
                                index: 0usize,
                                value: "one",
                                "Option one"
                            }
                        }
                    }

                    // Migration A slice 3/3 (final): Combobox's web arm ->
                    // `popover="auto"` (`ComboboxListRendered`, `list.rs`).
                    // Written RED first against the pre-migration plain div
                    // (confirmed by execution: it clipped at the 60px
                    // ancestor exactly like the others did pre-migration).
                    Combobox::<String> { id: "clip-combobox-root",
                        ComboboxInput { id: "clip-combobox-trigger", aria_label: "Combobox trigger" }
                        ComboboxList {
                            id: "clip-combobox-content",
                            style: "min-height: 100px;",
                            ComboboxOption::<String> {
                                index: 0usize,
                                value: "one",
                                "Option one"
                            }
                        }
                    }

                    // 2026-09-03, finding C: `NavbarNav`'s web arm ->
                    // `popover="auto"` (`NavbarContentRendered`,
                    // `navbar.rs`) -- `Navbar` never migrated onto the
                    // top-layer engine during Migration A, so it never got
                    // clipping escape, top-layer stacking, or (Rule 5-style
                    // below) viewport-edge flip the way `DropdownMenu`/
                    // `Menubar`/`Select` did. Written RED first against the
                    // pre-migration plain, `position: absolute`-only div
                    // (confirmed by execution: it clipped at the 60px
                    // ancestor exactly like `Menubar`'s identical
                    // pre-migration shape did).
                    Navbar { id: "clip-navbar-root", aria_label: "Clip navbar test",
                        NavbarNav { index: 0usize,
                            NavbarTrigger { id: "clip-navbar-trigger", "Navbar trigger" }
                            NavbarContent {
                                id: "clip-navbar-content",
                                style: "min-height: 100px;",
                                NavbarItem {
                                    index: 0usize,
                                    value: "one".to_string(),
                                    to: crate::Route::home(),
                                    "Item one"
                                }
                            }
                        }
                    }

                    // Native reference: the browser's own implementation of the
                    // same WHATWG HTML feature, with no Dioxus involvement at
                    // all -- the calibration control for this rule.
                    button {
                        id: "clip-native-trigger",
                        popovertarget: "clip-native-content",
                        "Native trigger"
                    }
                    div {
                        id: "clip-native-content",
                        popover: "auto",
                        style: "min-height: 100px; min-width: 200px; border: 1px solid; padding: 0.5rem; background: Canvas; color: CanvasText;",
                        "Native reference content for the clipping-escape rule."
                    }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Light dismiss, Escape, and stacking" }
                p { class: Styles::dx_top_layer_hint,
                    "The red panel is a high-"
                    code { "z-index" }
                    " sibling positioned where each popover opens. Opening a "
                    code { "popover=\"auto\"" }
                    " (or a non-modal "
                    code { "Popover" }
                    ") should render its content above that sibling (top layer "
                    "stacking). Clicking elsewhere, or pressing Escape, should "
                    "close it and sync back to "
                    code { "data-state" }
                    " on the root/trigger -- the Rust signal must never strand."
                }
                div { class: Styles::dx_top_layer_stack_area,
                    div { id: "stack-sibling", class: Styles::dx_top_layer_stack_sibling, "High z-index sibling" }

                    div { class: Styles::dx_top_layer_stack_row,
                        PopoverRoot { id: "stack-popover-root", is_modal: false,
                            PopoverTrigger { id: "stack-popover-trigger", "Stacking popover trigger" }
                            PopoverContent { id: "stack-popover-content", "Stacking popover content" }
                        }

                        button {
                            id: "stack-native-trigger",
                            popovertarget: "stack-native-content",
                            "Native stacking trigger"
                        }
                        div {
                            id: "stack-native-content",
                            // Plain, hand-written marker class -- never run
                            // through `#[css_module]`'s hashing, deliberately:
                            // that hashing only rewrites selectors it finds as
                            // flat top-level rules, not ones nested inside an
                            // `@supports` block (confirmed by execution: the
                            // hashed class landed on this element, but the
                            // `@supports`-scoped rule below kept referencing
                            // the *unhashed* name, so it silently never
                            // matched). Same reasoning and the same fix as
                            // `primitives/src/tooltip.rs`'s `dx-anchor-tooltip`
                            // (see its comment) -- this fixture just needs its
                            // own instance since it isn't a primitives
                            // component.
                            class: "dx-anchor-stack-native",
                            popover: "auto",
                            style: "min-width: 200px; border: 1px solid; padding: 0.5rem; background: Canvas; color: CanvasText;",
                            "Native stacking content"
                        }
                    }

                    // An element well away from both triggers, for the
                    // light-dismiss ("click outside") assertions.
                    button { id: "outside-click-target", class: Styles::dx_top_layer_outside, "Click outside target" }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Toast top-layer stacking (Migration A slice 3/3, final)" }
                p { class: Styles::dx_top_layer_hint,
                    "The small red panel is a high-"
                    code { "z-index" }
                    " sibling pinned to the same fixed viewport corner the toast "
                    "below will render at. Toasts never light-dismiss and never "
                    "anchor to a trigger -- "
                    code { "ToastProvider" }
                    "'s region is `popover=\"manual\"` purely so it always paints "
                    "in the top layer, above whatever else is on the page "
                    "(top-layer stacking, the same rule as the section above). "
                    "Click the trigger to add a toast and cover the sibling."
                }
                // `position: fixed`, not `absolute`, on both -- confirmed by
                // execution to matter, not a style preference: moving an
                // element into the top layer changes its containing block
                // to the *initial* containing block regardless of its own
                // `position` value (see `crate::top_layer::anchor_name_
                // style`'s doc in `primitives/src/top_layer.rs` for the
                // general rule this is an instance of), and for `position:
                // absolute` specifically that initial containing block is
                // sized like the viewport but anchored to the *document's*
                // origin, not the current scroll position -- an earlier
                // version of this fixture used `position: absolute; top: 0;
                // left: 0` on the (popover-promoted) toast region nested
                // inside a `position: relative` wrapper, which rendered
                // correctly relative to that wrapper only when the page
                // happened to be scrolled to the very top, and hundreds of
                // pixels off-screen otherwise -- exactly the "toast not
                // findable" failure this rule caught. `position: fixed`
                // sidesteps this: fixed's containing block is the viewport
                // itself either way, top layer or not, so `top: 0; left: 0`
                // reliably means the actual visible viewport corner
                // regardless of scroll -- matching this component's own
                // real-world CSS (`preview/src/components/toast/style.css`'s
                // `.dx-toast-container` is `position: fixed` for the same
                // reason).
                div {
                    id: "toast-stack-sibling",
                    style: "position: fixed; bottom: 0; right: 0; z-index: 99999; width: 40px; height: 20px; background: crimson;",
                }
                // No `id` here, deliberately: `ToastProviderProps` has no
                // dedicated `id` field (the region's id has always been
                // purely internal, generated by `use_unique_id` inside
                // `toast.rs`, since the F6-focus-region wiring needs one
                // but nothing caller-visible does) -- an `id` passed here
                // would be absorbed by its `#[props(extends =
                // GlobalAttributes)]` catch-all into `attributes`.
                // `ToastRegionRendered` now drops any `id` found there
                // explicitly before merging (see its doc, "Attribute-
                // override dedup") rather than relying on spread order, but
                // an earlier version relied on `..attributes` losing to a
                // preceding `id: id.clone()` on the client only -- which an
                // id passed here would have silently overridden, stranding
                // `use_popover_shown_while_mounted`'s `document.
                // getElementById` -- confirmed by execution: this is
                // exactly why an earlier version of this fixture's toast
                // never became `:popover-open` at all.
                //
                // `top: auto; left: auto; margin: 0;` alongside `bottom`/
                // `right` -- also confirmed necessary by execution, and the
                // exact reason `preview/src/components/toast/style.css`'s
                // real `.dx-toast-container[popover]` rule resets all four
                // sides plus `margin`, not just the two this fixture cares
                // about: the WHATWG popover UA stylesheet's `[popover] {
                // inset: 0; margin: auto; ... }` still supplies `top: 0;
                // left: 0` for the two sides this element's own style never
                // mentions, and combined with the UA's own `margin: auto`
                // (never triggered pre-popover, since the ordinary initial
                // value of `margin` is `0`) that is exactly the four-sides-
                // plus-auto-margin shape CSS's own centering algorithm
                // looks for -- an earlier version of this fixture, setting
                // only `bottom`/`right`, rendered the toast dead-centered
                // in the viewport instead of at the corner, nowhere near
                // the sibling.
                ToastProvider {
                    style: "position: fixed; top: auto; right: 0; bottom: 0; left: auto; margin: 0;",
                    // The app shell already mounts a toast region on every
                    // page; two `role="region"` landmarks sharing the
                    // primitive's default "N notifications" name fail axe's
                    // `landmark-unique` on the all-components page. The
                    // primitive merges this override with its own default
                    // `aria_label` via `merge_attributes` (caller-wins,
                    // deduped -- see `ToastRegionRendered`'s doc in
                    // `toast.rs`, "Attribute-override dedup"), so this
                    // value wins on both the CSR and SSR/SSG lanes, not
                    // just in the live DOM.
                    aria_label: "Top-layer fixture notifications",
                    ToastStackTrigger {}
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Point-positioned vs. anchored scroll behavior (Migration A slice 2/3)" }
                p { class: Styles::dx_top_layer_hint,
                    "Both controls below sit normally in-flow (no clip ancestor, plenty of "
                    "room on every side) so a modest scroll cannot itself cross a "
                    "viewport-edge flip threshold -- these rules aren't about "
                    "flipping (Rules 5-7 already cover that), they're about what "
                    "happens to already-open content while the page scrolls. "
                    code { "ContextMenu" }
                    " opens at a raw click point with no anchor -- pre-migration "
                    "behavior (measured, then preserved by this migration) is "
                    "that its content stays at the click's "
                    code { "viewport" }
                    "-relative position, i.e. it does not move on screen as the "
                    "page scrolls underneath it (same as a native OS context "
                    "menu). "
                    code { "Menubar" }
                    "'s content, by contrast, is anchored to its own trigger "
                    "(like " code { "DropdownMenu" } "'s Rule 8 case) and must "
                    "keep tracking that trigger's position through a scroll."
                }
                ContextMenu { id: "scroll-context-menu-root",
                    ContextMenuTrigger { id: "scroll-context-menu-trigger", "Scroll test: right-click here" }
                    ContextMenuContent { id: "scroll-context-menu-content",
                        ContextMenuItem {
                            value: "one".to_string(),
                            index: 0usize,
                            on_select: move |_: String| {},
                            "Item one"
                        }
                    }
                }
                Menubar { id: "scroll-menubar-root",
                    MenubarMenu { index: 0usize,
                        MenubarTrigger { id: "scroll-menubar-trigger", "Scroll test menu" }
                        MenubarContent { id: "scroll-menubar-content",
                            MenubarItem {
                                index: 0usize,
                                value: "one".to_string(),
                                on_select: move |_: String| {},
                                "Item one"
                            }
                        }
                    }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Near-viewport-edge flip (CSS position-try-fallbacks)" }
                p { class: Styles::dx_top_layer_hint,
                    "W3C CSS Anchor Positioning's "
                    code { "position-try-fallbacks" }
                    " ("
                    a {
                        href: "https://www.w3.org/TR/css-anchor-position-1/#fallback-var",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "spec"
                    }
                    ") -- a CSS-spec citation, not WHATWG HTML, unlike the rest of "
                    "this fixture; see the oracle spec's header for why this rule "
                    "still lives in this tier-2 file. Every trigger below is "
                    "pinned by fixed positioning right at a viewport edge, with "
                    "its preferred "
                    code { "side" }
                    " pointing off-viewport -- the bottom row's triggers request "
                    code { "side=\"bottom\"" }
                    " while sitting a few pixels from the bottom edge; the right "
                    "column's request "
                    code { "side=\"right\"" }
                    " a few pixels from the right edge -- so content can only "
                    "render fully on-screen by flipping to the opposite side on "
                    "the relevant axis. A native "
                    code { "<div popover>" }
                    " + "
                    code { "position-try-fallbacks" }
                    " reference sits in each row/column too: the browser's own "
                    "implementation of the identical spec feature, no library "
                    "code involved (CALIBRATION)."
                }
                div { class: Styles::dx_top_layer_edge_bottom_row,
                    Tooltip {
                        TooltipTrigger { id: "edge-bottom-tooltip-trigger", "Bottom tooltip" }
                        TooltipContent {
                            id: "edge-bottom-tooltip-content",
                            side: ContentSide::Bottom,
                            "Flips above its trigger when the preferred side runs off-viewport."
                        }
                    }

                    HoverCard {
                        HoverCardTrigger { id: "edge-bottom-hovercard-trigger", "Bottom hover card" }
                        HoverCardContent {
                            id: "edge-bottom-hovercard-content",
                            side: ContentSide::Bottom,
                            "Flips above its trigger when the preferred side runs off-viewport."
                        }
                    }

                    PopoverRoot { id: "edge-bottom-popover-root", is_modal: false,
                        PopoverTrigger { id: "edge-bottom-popover-trigger", "Bottom popover" }
                        PopoverContent {
                            id: "edge-bottom-popover-content",
                            side: ContentSide::Bottom,
                            "Flips above its trigger when the preferred side runs off-viewport."
                        }
                    }

                    // 2026-09-03, finding C: unlike `Tooltip`/`HoverCard`/
                    // `PopoverContent` above, `NavbarContent` has no `side`
                    // prop -- its placement is always below/start-aligned,
                    // the same fixed convention `MenubarContent`/
                    // `DropdownMenuContent` use (see `NavbarContentRendered`'s
                    // doc, `navbar.rs`). So the flip case here is driven by
                    // pinning the *trigger* at the bottom edge instead of by
                    // an explicit `side` prop -- the same shape a real page
                    // footer or bottom nav bar would put a `Navbar` in.
                    // Written RED first against the pre-migration plain,
                    // `position: absolute; top: 100%`-only div (confirmed by
                    // execution: no flip of any kind, content rendered
                    // straight off the bottom of the viewport).
                    Navbar { id: "edge-bottom-navbar-root", aria_label: "Edge bottom navbar test",
                        NavbarNav { index: 0usize,
                            NavbarTrigger { id: "edge-bottom-navbar-trigger", "Bottom navbar" }
                            NavbarContent {
                                id: "edge-bottom-navbar-content",
                                NavbarItem {
                                    index: 0usize,
                                    value: "one".to_string(),
                                    to: crate::Route::home(),
                                    "Flips above its trigger when the preferred side runs off-viewport."
                                }
                            }
                        }
                    }

                    // Native reference (CALIBRATION): the browser's own
                    // `position-try-fallbacks: flip-block` on a plain
                    // `<div popover>`, anchored via the `dx-top-layer-edge-
                    // native-trigger-bottom` class's `anchor-name` (see
                    // style.css). No Dioxus positioning logic involved.
                    button {
                        id: "edge-bottom-native-trigger",
                        class: Styles::dx_top_layer_edge_native_trigger_bottom,
                        popovertarget: "edge-bottom-native-content",
                        "Native bottom trigger"
                    }
                    div {
                        id: "edge-bottom-native-content",
                        // Plain, hand-written marker class -- see this
                        // file's `dx-anchor-stack-native` comment above for
                        // why (`@supports`-nested selectors aren't scoped
                        // by `#[css_module]`).
                        class: "dx-anchor-edge-bottom-native",
                        popover: "auto",
                        "data-side": "bottom",
                        style: "min-height: 60px; min-width: 200px; border: 1px solid; padding: 0.5rem; background: Canvas; color: CanvasText;",
                        "Native reference: flips above via position-try-fallbacks: flip-block."
                    }
                }

                div { class: Styles::dx_top_layer_edge_right_col,
                    Tooltip {
                        TooltipTrigger { id: "edge-right-tooltip-trigger", "Right tooltip" }
                        TooltipContent {
                            id: "edge-right-tooltip-content",
                            side: ContentSide::Right,
                            "Flips left of its trigger when the preferred side runs off-viewport."
                        }
                    }

                    HoverCard {
                        HoverCardTrigger { id: "edge-right-hovercard-trigger", "Right hover card" }
                        HoverCardContent {
                            id: "edge-right-hovercard-content",
                            side: ContentSide::Right,
                            "Flips left of its trigger when the preferred side runs off-viewport."
                        }
                    }

                    PopoverRoot { id: "edge-right-popover-root", is_modal: false,
                        PopoverTrigger { id: "edge-right-popover-trigger", "Right popover" }
                        PopoverContent {
                            id: "edge-right-popover-content",
                            side: ContentSide::Right,
                            "Flips left of its trigger when the preferred side runs off-viewport."
                        }
                    }

                    // Native reference (CALIBRATION): `position-try-
                    // fallbacks: flip-inline` counterpart of the row above.
                    button {
                        id: "edge-right-native-trigger",
                        class: Styles::dx_top_layer_edge_native_trigger_right,
                        popovertarget: "edge-right-native-content",
                        "Native right trigger"
                    }
                    div {
                        id: "edge-right-native-content",
                        class: "dx-anchor-edge-right-native",
                        popover: "auto",
                        "data-side": "right",
                        style: "min-width: 200px; border: 1px solid; padding: 0.5rem; background: Canvas; color: CanvasText;",
                        "Native reference: flips left via position-try-fallbacks: flip-inline."
                    }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Native <dialog> clipping escape (Phase 4.2)" }
                p { class: Styles::dx_top_layer_hint,
                    "The modal "
                    code { "Dialog" }
                    " below sits inside "
                    code { "#dialog-clip-box" }
                    ", the same "
                    code { "overflow: hidden; height: 60px; transform: translateZ(0)" }
                    " ancestor as the section above. Its content is taller than "
                    "the clip, so it must render fully visible outside it -- same "
                    "top-layer rule, now for "
                    code { "showModal()" }
                    " rather than "
                    code { "popover=" }
                    "."
                }
                div { id: "dialog-clip-box", class: Styles::dx_top_layer_clip_box,
                    button {
                        id: "dialog-clip-trigger",
                        onclick: move |_| dialog_clip_open.set(true),
                        "Dialog clip trigger"
                    }
                    DialogRoot {
                        id: "dialog-clip-root",
                        open: dialog_clip_open(),
                        on_open_change: move |v| dialog_clip_open.set(v),
                        DialogContent {
                            id: "dialog-clip-content",
                            style: "min-height: 140px; min-width: 220px; border: 1px solid; padding: 1rem; background: Canvas; color: CanvasText;",
                            DialogTitle { "Clip-escape dialog" }
                            p { "Content deliberately taller than the 60px clip ancestor." }
                            button {
                                id: "dialog-clip-close",
                                onclick: move |_| dialog_clip_open.set(false),
                                "Close"
                            }
                        }
                    }

                    // Native reference: a plain `<dialog>` opened with
                    // `showModal()` -- see this function's doc comment for
                    // why the trigger needs one line of JS.
                    button {
                        id: "dialog-clip-native-trigger",
                        onclick: move |_| {
                            document::eval(
                                "document.getElementById('dialog-clip-native-content').showModal();",
                            );
                        },
                        "Native dialog clip trigger"
                    }
                    dialog {
                        id: "dialog-clip-native-content",
                        style: "min-height: 140px; min-width: 220px; border: 1px solid; padding: 1rem; background: Canvas; color: CanvasText;",
                        p { "Native dialog reference content for the clipping-escape rule." }
                        form { method: "dialog",
                            button { id: "dialog-clip-native-close", "Close" }
                        }
                    }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Native <dialog> background inertness (Phase 4.2)" }
                p { class: Styles::dx_top_layer_hint,
                    "While the alert dialog below is open, the background "
                    "button/input here must be behaviourally inert: a real "
                    "click must not reach the button's handler, and calling "
                    code { ".focus()" }
                    " on the input directly must not move "
                    code { "document.activeElement" }
                    ". (Chromium does not reflect this onto "
                    code { "Element.inert" }
                    " for "
                    code { "<body>" }
                    " -- docs/phase4-spike-findings.md -- so this fixture is measured "
                    "behaviourally, not via that property.) Uses "
                    code { "AlertDialog" }
                    ", not "
                    code { "Dialog" }
                    ", deliberately: "
                    code { "Dialog" }
                    "'s own backdrop-click dismiss (dialog.spec.ts) would "
                    "otherwise close the dialog the instant a background "
                    "click lands outside it, which is a correct interaction "
                    "but confounds this specific probe -- "
                    code { "AlertDialog" }
                    " never light-dismisses, so it stays open through the "
                    "whole check."
                }
                div {
                    div { id: "dialog-inert-bg-count", "{bg_clicks()}" }
                    button {
                        id: "dialog-inert-bg-button",
                        onclick: move |_| bg_clicks.set(bg_clicks() + 1),
                        "Background button"
                    }
                    input { id: "dialog-inert-bg-input", placeholder: "Background input" }
                }
                button {
                    id: "dialog-inert-trigger",
                    onclick: move |_| dialog_inert_open.set(true),
                    "Open inertness dialog"
                }
                AlertDialogRoot {
                    id: "dialog-inert-root",
                    open: dialog_inert_open(),
                    on_open_change: move |v| dialog_inert_open.set(v),
                    AlertDialogContent { id: "dialog-inert-content",
                        AlertDialogTitle { "Inertness dialog" }
                        button {
                            id: "dialog-inert-close",
                            onclick: move |_| dialog_inert_open.set(false),
                            "Close"
                        }
                    }
                }
            }

            section { class: Styles::dx_top_layer_section,
                h2 { "Native <dialog> modal Popover (two-engine completion)" }
                p { class: Styles::dx_top_layer_hint,
                    "The modal "
                    code { "Popover" }
                    " below (default "
                    code { "is_modal" }
                    ") now renders a real "
                    code { "<dialog>" }
                    " opened with "
                    code { "showModal()" }
                    " on the web arm, driven by the same open-driver/"
                    "close-sync/backdrop-dismiss trio as modal "
                    code { "Dialog" }
                    " -- except, unlike "
                    code { "Dialog" }
                    ", a modal "
                    code { "Popover" }
                    " stays anchored to its own trigger rather than "
                    "viewport-centered. Three checks share the "
                    code { "#popover-modal-clip-box" }
                    "/inertness/anchor sub-fixtures below: clipping escape "
                    "(same "
                    code { "overflow: hidden" }
                    " ancestor as the plain-Dialog case above), background "
                    "inertness (click/focus must not reach the background "
                    "button/input), and trigger-anchored placement -- the "
                    "anchor trigger sits pinned near the top-left viewport "
                    "corner, far from the viewport center, so a "
                    "viewport-centered "
                    code { "showModal()" }
                    " (the UA default this migration must override) would "
                    "measurably disagree with a trigger-anchored one."
                }
                div { id: "popover-modal-clip-box", class: Styles::dx_top_layer_clip_box,
                    PopoverRoot { id: "popover-modal-clip-root",
                        PopoverTrigger { id: "popover-modal-clip-trigger", "Modal popover clip trigger" }
                        PopoverContent {
                            id: "popover-modal-clip-content",
                            // An explicit `width` (not just `min-width`) --
                            // a native modal `<dialog>` (UA `dialog:modal {
                            // width: fit-content; ... }`, un-reset by this
                            // component's own CSS) sizes to its content's
                            // natural line width by default. Without a cap,
                            // this fixture's sentence-length text renders
                            // wide enough that `align: Center`'s
                            // `transform: translateX(-50%)` -- centering it
                            // under a trigger sitting near this box's own
                            // left edge -- pushes a meaningful chunk of the
                            // content off the left edge of the *viewport*,
                            // confusing this rule's clip-escape probe
                            // (which needs the content to still land
                            // somewhere `elementFromPoint` can see) with a
                            // symptom that looks like clipping but is
                            // actually just off-canvas.
                            style: "min-height: 140px; width: 220px;",
                            "Modal popover content, taller than the clip ancestor."
                        }
                    }
                }

                div {
                    div { id: "popover-modal-inert-bg-count", "{popover_modal_bg_clicks()}" }
                    button {
                        id: "popover-modal-inert-bg-button",
                        onclick: move |_| popover_modal_bg_clicks.set(popover_modal_bg_clicks() + 1),
                        "Background button"
                    }
                    input { id: "popover-modal-inert-bg-input", placeholder: "Background input" }
                }
                PopoverRoot { id: "popover-modal-inert-root",
                    PopoverTrigger { id: "popover-modal-inert-trigger", "Open inertness popover" }
                    PopoverContent { id: "popover-modal-inert-content",
                        "Modal popover inertness content."
                    }
                }

                // Pinned well away from the viewport center (Escape/reopen
                // and focus-restore cycles, plus the trigger-anchored
                // placement measurement, all share this one trigger/content
                // pair).
                //
                // `top: calc(var(--dx-navbar-height) + 20px)`, not a plain
                // `40px` -- found by execution
                // (`oracle/tier2-html/native-dialog.spec.ts` 6c/6d/6e, SSG
                // lane only): a plain `40px` sits partly underneath
                // `.dx-preview-navbar` (`position: sticky; top: 0`, ~54px
                // tall), which is a real, ordinary `position: fixed`
                // element (no scrolling involved for
                // `scrollIntoViewIfNeeded` to correct via the sticky-nav
                // `scroll-padding-top` added in `main.css` -- that only
                // helps things that actually scroll into place), so it
                // needs its own clearance instead. Only reliably visible
                // pre-JS on a fullstack SSG prerender, where the sticky
                // nav's CSS is already in effect at first paint; on the
                // CSR dev server the same collision was masked in this
                // session's testing by an unrelated, environment-specific
                // race (`main.css`'s Google Fonts `@import` and this
                // dev-only page's own `document::Link` head injection both
                // taking long enough that the nav was still briefly
                // `position: static` when Playwright clicked) -- not a
                // structural difference between the two lanes, so this is
                // still fixed here rather than only in the SSG build.
                div { style: "position: fixed; top: calc(var(--dx-navbar-height, 60px) + 20px); left: 40px;",
                    PopoverRoot { id: "popover-modal-anchor-root",
                        PopoverTrigger { id: "popover-modal-anchor-trigger", "Modal popover anchor trigger" }
                        PopoverContent { id: "popover-modal-anchor-content",
                            "Modal popover anchored content."
                        }
                    }
                }
            }
        }
    }
}

/// Adds a permanent toast on click -- a plain, unstyled use of
/// [`use_toast`], split into its own component only because `use_toast`
/// must be called from a descendant of `ToastProvider`, not `ToastProvider`
/// itself.
#[component]
fn ToastStackTrigger() -> Element {
    let toast_api = use_toast();

    rsx! {
        button {
            id: "toast-stack-trigger",
            // `ToastProvider`'s own children (this button) render in normal
            // flow, wherever `ToastProvider` sits in the tree; the toast
            // region itself (and the sibling above) are both `position:
            // fixed` to the bottom-right viewport corner, well away from
            // wherever this button's own normal-flow position happens to
            // land, so nothing here needs to work around an overlap.
            onclick: move |_| {
                toast_api
                    .info(
                        "Stacking test".to_string(),
                        ToastOptions::new().permanent(true),
                    );
            },
            "Add toast"
        }
    }
}
