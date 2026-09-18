use super::super::component::*;
use dioxus::prelude::*;

/// Non-modal variant. shadcn/ui's `Popover` is non-modal by default (it
/// does not trap focus or block interaction with the rest of the page,
/// unlike this crate's own `main` variant demo above, which is
/// deliberately `is_modal: true` -- a confirm/cancel action panel that
/// wants exactly that trap) -- worth having as its own example rather than
/// leaving `is_modal: false` undemonstrated. Content mirrors shadcn's own
/// canonical Popover example ("Dimensions"/"Set the dimensions for the
/// layer."), trimmed to a title + description (no form inputs) to keep
/// this demo self-contained.
///
/// This variant also gives `playwright/popover.spec.ts`'s close-fade
/// regression (dev-docs/backlog.md rows 19, 7) a themed, non-modal subject
/// to sample: the `top_layer` oracle fixture's `#stack-popover-*` instance
/// composes the raw `dioxus_primitives::popover` primitive directly
/// (`scripts/check-preview-composition.sh`'s documented, deliberate
/// exemption for that fixture), so it never loads `../../style.css` and
/// has no close animation defined at all -- `use_animated_open` sees zero
/// running animations and unmounts immediately, which is a fixture-
/// composition gap, not a defect in rows 19/7's fix. This variant, routed
/// through the themed `PopoverRoot`/`PopoverTrigger`/`PopoverContent`
/// wrappers below (`use super::super::component::*`, same as `main`
/// above), loads the real stylesheet via each wrapper's own
/// `document::Link` (`../../component.rs`) the same way every other
/// component page does.
#[component]
pub fn Demo() -> Element {
    rsx! {
        PopoverRoot { is_modal: false,
            PopoverTrigger { "Open popover" }
            PopoverContent { gap: "0.25rem",
                h3 {
                    class: "dx-popover-content-title",
                    padding_top: "0.25rem",
                    padding_bottom: "0.25rem",
                    width: "100%",
                    text_align: "center",
                    margin: 0,
                    "Dimensions"
                }
                p {
                    class: "dx-popover-content-description",
                    text_align: "center",
                    "Set the dimensions for the layer."
                }
            }
        }
    }
}
