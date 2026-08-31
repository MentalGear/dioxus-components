use dioxus::prelude::*;
use dioxus_primitives::alert_dialog::{AlertDialogContent, AlertDialogRoot, AlertDialogTitle};
use dioxus_primitives::dialog::{DialogContent, DialogRoot, DialogTitle};
use dioxus_primitives::hover_card::{HoverCard, HoverCardContent, HoverCardTrigger};
use dioxus_primitives::popover::{PopoverContent, PopoverRoot, PopoverTrigger};
use dioxus_primitives::tooltip::{Tooltip, TooltipContent, TooltipTrigger};

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
        }
    }
}
