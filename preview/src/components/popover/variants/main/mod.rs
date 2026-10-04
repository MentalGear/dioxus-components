use super::super::component::*;
use crate::components::{input::Input, label::Label};
use dioxus::prelude::*;

/// shadcn's canonical Popover demo (base-nova `popover-demo`,
/// shadcn-ui/ui@295a1f1): a "Dimensions" header and four label + input rows in
/// a 20rem panel (`w-80`). A popover holds arbitrary content, so this is a form
/// of real `Label`s wired to real `Input`s, not a stack of buttons styled like a
/// menu -- nothing here has `role="menuitem"` because nothing here is a menu.
///
/// Unlike shadcn's (non-modal) popover this one is `is_modal` (the primitive's
/// default): focus is trapped, the page behind it is inert and a click outside
/// dismisses it, so it dims the page with the same scrim every modal overlay
/// shares. The `non_modal` variant shows the shadcn default, which has no overlay.
#[component]
pub fn Demo() -> Element {
    rsx! {
        PopoverRoot {
            PopoverTrigger { "Show Popover" }
            PopoverContent { width: "20rem",
                div { display: "grid", gap: "1rem",
                    PopoverHeader { gap: "0.5rem",
                        PopoverTitle { "Dimensions" }
                        PopoverDescription { "Set the dimensions for the layer." }
                    }
                    div { display: "grid", gap: "0.5rem",
                        DimensionRow { id: "popover-demo-width", label: "Width", value: "100%" }
                        DimensionRow { id: "popover-demo-max-width", label: "Max. width", value: "300px" }
                        DimensionRow { id: "popover-demo-height", label: "Height", value: "25px" }
                        DimensionRow { id: "popover-demo-max-height", label: "Max. height", value: "none" }
                    }
                }
            }
        }
    }
}

/// One `grid-cols-3 items-center gap-4` row of shadcn's demo: the label in the
/// first column, the input spanning the other two.
#[component]
fn DimensionRow(id: &'static str, label: &'static str, value: &'static str) -> Element {
    rsx! {
        div {
            display: "grid",
            grid_template_columns: "repeat(3, minmax(0, 1fr))",
            align_items: "center",
            gap: "1rem",
            Label { html_for: id, "{label}" }
            Input { id, initial_value: value, grid_column: "span 2 / span 2" }
        }
    }
}
