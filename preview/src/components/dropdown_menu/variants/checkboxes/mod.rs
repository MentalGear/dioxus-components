use super::super::component::{
    DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuLabel,
    DropdownMenuSub, DropdownMenuSubContent, DropdownMenuSubTrigger, DropdownMenuTrigger,
};
use dioxus::prelude::*;

/// shadcn's "With Checkboxes" example: three independent toggles under an
/// "Appearance" heading, the middle one disabled. Controlled (`checked` +
/// `on_checked_change`), and every item sets `close_on_select: false` so
/// several can be toggled in one visit -- the primitive's own default is
/// Radix's `onSelect` one (selecting closes the menu); see
/// `dioxus_primitives::menu_item`. The readout under the menu proves the
/// callbacks round-trip. `text_value` is set because a checkbox item has no
/// `value` to make it a typeahead target.
///
/// A "More options" submenu holds two more checkbox items, to show that a
/// checkable item works inside a submenu (it registers in that submenu's own
/// roving-focus collection, not the root's): "Word Wrap" keeps the menu open
/// like the rest; "Minimap" uses the primitive's default, so choosing it
/// closes the whole menu tree.
///
/// The labels ("Checkboxes"/"Status Bar"/"Activity Bar"/"Panel") deliberately
/// differ from `main`'s and `rtl`'s: this variant renders alongside them on
/// the same page, and `dropdown-menu.spec.ts` plus the shared
/// `oracle/tier1-apg/*` specs query those by name (no `exact`).
#[component]
pub fn Demo() -> Element {
    let mut status_bar = use_signal(|| true);
    let mut activity_bar = use_signal(|| false);
    let mut panel = use_signal(|| false);
    let mut word_wrap = use_signal(|| false);
    let mut minimap = use_signal(|| false);

    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { "Checkboxes" }
            DropdownMenuContent {
                DropdownMenuLabel { "Appearance" }
                DropdownMenuCheckboxItem {
                    index: 0usize,
                    checked: Some(status_bar()),
                    on_checked_change: move |checked| status_bar.set(checked),
                    close_on_select: false,
                    text_value: "Status Bar",
                    "Status Bar"
                }
                DropdownMenuCheckboxItem {
                    index: 1usize,
                    checked: Some(activity_bar()),
                    on_checked_change: move |checked| activity_bar.set(checked),
                    close_on_select: false,
                    disabled: true,
                    text_value: "Activity Bar",
                    "Activity Bar"
                }
                DropdownMenuCheckboxItem {
                    index: 2usize,
                    checked: Some(panel()),
                    on_checked_change: move |checked| panel.set(checked),
                    close_on_select: false,
                    text_value: "Panel",
                    "Panel"
                }
                DropdownMenuSub {
                    DropdownMenuSubTrigger {
                        index: 3usize,
                        text_value: "More options",
                        "More options"
                    }
                    DropdownMenuSubContent {
                        DropdownMenuCheckboxItem {
                            index: 0usize,
                            checked: Some(word_wrap()),
                            on_checked_change: move |checked| word_wrap.set(checked),
                            close_on_select: false,
                            text_value: "Word Wrap",
                            "Word Wrap"
                        }
                        DropdownMenuCheckboxItem {
                            index: 1usize,
                            checked: Some(minimap()),
                            on_checked_change: move |checked| minimap.set(checked),
                            text_value: "Minimap",
                            "Minimap"
                        }
                    }
                }
            }
        }
        p { "Status bar {status_bar()}, panel {panel()}, word wrap {word_wrap()}, minimap {minimap()}" }
    }
}
