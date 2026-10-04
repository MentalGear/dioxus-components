use super::super::component::*;
use dioxus::prelude::*;

/// shadcn's "With Checkboxes" example: a "View" menu mixing two checkbox
/// items with plain items, and a "Format" menu of three checkbox items.
/// Controlled (`checked` + `on_checked_change`) -- an uncontrolled item's
/// state lives in the item, and the menu content unmounts when it closes, so
/// state that must survive a close has to be held by the caller -- and
/// keeping the menu open after each toggle (`close_on_select: false`). The
/// plain items carry `data-inset="true"`
/// (shadcn's `inset`) so, in the one menu that mixes both, their labels sit
/// on the same inline-start edge as the checkable items' -- the menubar
/// draws its check at the inline start. `text_value` is set because a
/// checkbox item has no `value` to make it a typeahead target.
///
/// Neither `main` nor `rtl` uses these labels ("View"/"Format"/"Reload"/...):
/// this variant renders alongside them on the same page, and
/// `menubar.spec.ts` plus the shared `oracle/tier1-apg/*` specs query "File"/
/// "Edit"/"New"/"Cut"/... by name (no `exact`), and the keyboard matrix pins
/// `main`'s two-trigger shape.
#[component]
pub fn Demo() -> Element {
    let mut bookmarks_bar = use_signal(|| false);
    let mut full_urls = use_signal(|| true);
    let mut strikethrough = use_signal(|| true);
    let mut code = use_signal(|| false);
    let mut superscript = use_signal(|| false);

    rsx! {
        Menubar {
            MenubarMenu { index: 0usize,
                MenubarTrigger { "View" }
                MenubarContent {
                    MenubarCheckboxItem {
                        index: 0usize,
                        checked: Some(bookmarks_bar()),
                        on_checked_change: move |checked| bookmarks_bar.set(checked),
                        close_on_select: false,
                        text_value: "Always Show Bookmarks Bar",
                        "Always Show Bookmarks Bar"
                    }
                    MenubarCheckboxItem {
                        index: 1usize,
                        checked: Some(full_urls()),
                        on_checked_change: move |checked| full_urls.set(checked),
                        close_on_select: false,
                        text_value: "Always Show Full URLs",
                        "Always Show Full URLs"
                    }
                    MenubarItem {
                        index: 2usize,
                        value: "reload".to_string(),
                        "data-inset": "true",
                        "Reload"
                    }
                    MenubarItem {
                        index: 3usize,
                        value: "force-reload".to_string(),
                        disabled: true,
                        "data-inset": "true",
                        "Force Reload"
                    }
                }
            }
            MenubarMenu { index: 1usize,
                MenubarTrigger { "Format" }
                MenubarContent {
                    MenubarCheckboxItem {
                        index: 0usize,
                        checked: Some(strikethrough()),
                        on_checked_change: move |checked| strikethrough.set(checked),
                        close_on_select: false,
                        text_value: "Strikethrough",
                        "Strikethrough"
                    }
                    MenubarCheckboxItem {
                        index: 1usize,
                        checked: Some(code()),
                        on_checked_change: move |checked| code.set(checked),
                        close_on_select: false,
                        text_value: "Code",
                        "Code"
                    }
                    MenubarCheckboxItem {
                        index: 2usize,
                        checked: Some(superscript()),
                        on_checked_change: move |checked| superscript.set(checked),
                        close_on_select: false,
                        text_value: "Superscript",
                        "Superscript"
                    }
                }
            }
        }
    }
}
