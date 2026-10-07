use super::super::component::*;
use dioxus::prelude::*;

/// Non-modal variant -- shadcn's default. A non-modal popover does not trap
/// focus or block the rest of the page and paints no overlay at all (shadcn's
/// `Popover` has none); it is light-dismissed by the browser (`popover="auto"`).
/// The `main` variant above is deliberately `is_modal` (the primitive's
/// default): it traps focus and makes the page inert, but dims nothing unless
/// `overlay: true` (see the `overlay` variant). Content is the
/// header of shadcn's own canonical Popover example ("Dimensions" / "Set the
/// dimensions for the layer."), without the form rows, to keep this demo small.
///
/// This variant also gives `playwright/popover.spec.ts`'s close-fade regression
/// (dev-docs/backlog.md rows 19, 7) a themed, non-modal subject to sample: the
/// `top_layer` oracle fixture's `#stack-popover-*` instance composes the raw
/// `dioxus_primitives::popover` primitive directly
/// (`scripts/check-preview-composition.sh`'s documented, deliberate exemption
/// for that fixture), so it never loads `../../style.css` and has no close
/// animation defined at all -- `use_animated_open` sees zero running animations
/// and unmounts immediately, which is a fixture-composition gap, not a defect in
/// rows 19/7's fix. This variant, routed through the themed `PopoverRoot`/
/// `PopoverTrigger`/`PopoverContent` wrappers (`use super::super::component::*`,
/// same as `main`), loads the real stylesheet via each wrapper's own
/// `document::Link` (`../../component.rs`) the same way every other component
/// page does.
#[component]
pub fn Demo() -> Element {
    rsx! {
        PopoverRoot { is_modal: false,
            PopoverTrigger { "Open popover" }
            PopoverContent {
                PopoverHeader {
                    PopoverTitle { "Dimensions" }
                    PopoverDescription { "Set the dimensions for the layer." }
                }
            }
        }
    }
}
