use dioxus::prelude::*;
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
        }
    }
}
