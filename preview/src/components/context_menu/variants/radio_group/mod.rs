use super::super::component::*;
use dioxus::prelude::*;

/// shadcn's "With Radio Group" example: two radio groups ("People" and
/// "Theme"), each with its own checked item, controlled (`value` +
/// `on_value_change`) and keeping the menu open after a choice
/// (`close_on_select: false`) so the checked item can be seen to move. Each
/// group is named by its heading (`aria-labelledby`). A radio item falls
/// back to its `value` as its typeahead label.
///
/// Labels ("Right click for radio group"/"Pedro Duarte"/...) differ from
/// `main`'s and `rtl`'s on purpose -- see the sibling `checkboxes` variant.
#[component]
pub fn Demo() -> Element {
    let mut person = use_signal(|| "pedro".to_string());
    let mut theme = use_signal(|| "light".to_string());

    rsx! {
        ContextMenu {
            ContextMenuTrigger { "Right click for radio group" }
            ContextMenuContent {
                ContextMenuLabel { id: "dx-cm-people-label", "People" }
                ContextMenuRadioGroup {
                    aria_labelledby: "dx-cm-people-label",
                    value: Some(person()),
                    on_value_change: move |value| person.set(value),
                    ContextMenuRadioItem {
                        index: 0usize,
                        value: "pedro".to_string(),
                        close_on_select: false,
                        "Pedro Duarte"
                    }
                    ContextMenuRadioItem {
                        index: 1usize,
                        value: "colm".to_string(),
                        close_on_select: false,
                        "Colm Tuite"
                    }
                }
                ContextMenuLabel { id: "dx-cm-theme-label", "Theme" }
                ContextMenuRadioGroup {
                    aria_labelledby: "dx-cm-theme-label",
                    value: Some(theme()),
                    on_value_change: move |value| theme.set(value),
                    ContextMenuRadioItem {
                        index: 2usize,
                        value: "light".to_string(),
                        close_on_select: false,
                        "Light"
                    }
                    ContextMenuRadioItem {
                        index: 3usize,
                        value: "dark".to_string(),
                        close_on_select: false,
                        "Dark"
                    }
                    ContextMenuRadioItem {
                        index: 4usize,
                        value: "system".to_string(),
                        close_on_select: false,
                        "System"
                    }
                }
            }
        }
    }
}
