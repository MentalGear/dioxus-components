use super::super::component::*;
use dioxus::prelude::*;

/// shadcn's "With Checkboxes" example: three independent toggles, the last one
/// disabled. Controlled (`checked` + `on_checked_change`) -- an uncontrolled
/// item's state lives in the item, and the menu content unmounts when it
/// closes, so state that must survive a close has to be held by the caller --
/// and keeping the menu open after each toggle (`close_on_select: false`) so
/// several can be toggled in one visit. The primitive's own default is
/// Radix's `onSelect` one (selecting closes the menu); `main` shows that.
/// `text_value` is set because a checkbox item has no `value` to make it a
/// typeahead target.
///
/// A "More options" submenu holds two more checkbox items, to show that a
/// checkable item works inside a submenu (it registers in that submenu's own
/// roving-focus collection, not the root's): "Word Wrap" keeps the menu open
/// like the rest; "Minimap" uses the primitive's default, so choosing it
/// closes the whole menu tree.
///
/// The labels ("Right click for checkboxes"/"Show Bookmarks Bar"/...) differ
/// from `main`'s and `rtl`'s on purpose: this variant renders alongside them
/// on the same page, and `context-menu.spec.ts` plus the shared
/// `oracle/tier1-apg/*` specs query those by name (no `exact`).
#[component]
pub fn Demo() -> Element {
    let mut bookmarks = use_signal(|| true);
    let mut full_urls = use_signal(|| false);
    let mut developer_tools = use_signal(|| true);
    let mut word_wrap = use_signal(|| false);
    let mut minimap = use_signal(|| false);

    rsx! {
        ContextMenu {
            ContextMenuTrigger { "Right click for checkboxes" }
            ContextMenuContent {
                ContextMenuCheckboxItem {
                    index: 0usize,
                    checked: Some(bookmarks()),
                    on_checked_change: move |checked| bookmarks.set(checked),
                    close_on_select: false,
                    text_value: "Show Bookmarks Bar",
                    "Show Bookmarks Bar"
                }
                ContextMenuCheckboxItem {
                    index: 1usize,
                    checked: Some(full_urls()),
                    on_checked_change: move |checked| full_urls.set(checked),
                    close_on_select: false,
                    text_value: "Show Full URLs",
                    "Show Full URLs"
                }
                ContextMenuCheckboxItem {
                    index: 2usize,
                    checked: Some(developer_tools()),
                    on_checked_change: move |checked| developer_tools.set(checked),
                    close_on_select: false,
                    disabled: true,
                    text_value: "Show Developer Tools",
                    "Show Developer Tools"
                }
                ContextMenuSub {
                    ContextMenuSubTrigger {
                        index: 3usize,
                        text_value: "More options",
                        "More options"
                    }
                    ContextMenuSubContent {
                        ContextMenuCheckboxItem {
                            index: 0usize,
                            checked: Some(word_wrap()),
                            on_checked_change: move |checked| word_wrap.set(checked),
                            close_on_select: false,
                            text_value: "Word Wrap",
                            "Word Wrap"
                        }
                        ContextMenuCheckboxItem {
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
        p { "Bookmarks {bookmarks()}, full URLs {full_urls()}, word wrap {word_wrap()}, minimap {minimap()}" }
    }
}
