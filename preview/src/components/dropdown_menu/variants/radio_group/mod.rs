use super::super::component::{
    DropdownMenu, DropdownMenuContent, DropdownMenuLabel, DropdownMenuRadioGroup,
    DropdownMenuRadioItem, DropdownMenuTrigger,
};
use dioxus::prelude::*;

/// shadcn's "With Radio Group" example: one position out of three under a
/// "Panel Position" heading, the last one disabled. Controlled (`value` +
/// `on_value_change`), and every item sets `close_on_select: false` so the
/// checked item can be seen to move without reopening the menu. The group is
/// named by its heading (`aria-labelledby`). A radio item falls back to its
/// `value` as its typeahead label, so no `text_value` is needed.
///
/// Labels ("Radio Group"/"Top"/"Bottom"/"Right") differ from `main`'s and
/// `rtl`'s on purpose -- see the sibling `checkboxes` variant.
#[component]
pub fn Demo() -> Element {
    let mut position = use_signal(|| "bottom".to_string());

    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { "Radio Group" }
            DropdownMenuContent {
                DropdownMenuLabel { id: "dx-dd-position-label", "Panel Position" }
                DropdownMenuRadioGroup {
                    aria_labelledby: "dx-dd-position-label",
                    value: Some(position()),
                    on_value_change: move |value| position.set(value),
                    DropdownMenuRadioItem {
                        index: 0usize,
                        value: "top".to_string(),
                        close_on_select: false,
                        "Top"
                    }
                    DropdownMenuRadioItem {
                        index: 1usize,
                        value: "bottom".to_string(),
                        close_on_select: false,
                        "Bottom"
                    }
                    DropdownMenuRadioItem {
                        index: 2usize,
                        value: "right".to_string(),
                        close_on_select: false,
                        disabled: true,
                        "Right"
                    }
                }
            }
        }
        p { "Panel position {position()}" }
    }
}
