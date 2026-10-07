use super::super::component::*;
use dioxus::prelude::*;

/// Modal popover with the page-dimming overlay switched on (`overlay: true`).
///
/// A popover is modal by default (focus trap, inert page, click-outside dismissal) but, like
/// shadcn's, does not dim the page: `overlay` defaults to `false`. Setting it paints the shared
/// modal scrim (the same one Dialog and Sheet use) behind the panel. The overlay is independent
/// of modality: `overlay: true` with `is_modal: false` has nothing to dim, because a non-modal
/// popover has no `::backdrop`.
#[component]
pub fn Demo() -> Element {
    rsx! {
        PopoverRoot { overlay: true,
            PopoverTrigger { "Open with overlay" }
            PopoverContent {
                PopoverHeader {
                    PopoverTitle { "Dimensions" }
                    PopoverDescription { "The page behind is dimmed while this is open." }
                }
            }
        }
    }
}
