use super::super::component::*;
use dioxus::prelude::*;

/// shadcn's "With Radio" example: a profile-switching menu and a "Theme" menu,
/// each a radio group with one checked item, controlled (`value` +
/// `on_value_change`) and keeping the menu open after a choice
/// (`close_on_select: false`) so the checked item can be seen to move. Each
/// group is named by its heading (`aria-labelledby`). A radio item falls
/// back to its `value` as its typeahead label.
///
/// Labels ("Accounts"/"Theme"/"Andy"/...) differ from `main`'s and `rtl`'s
/// on purpose -- see the sibling `checkboxes` variant. shadcn calls the first
/// menu "Profiles"; that word contains "file", and Playwright's `name`
/// matches by case-insensitive substring, so `getByRole("menuitem", { name:
/// "File" })` in `menubar.spec.ts` and the shared `oracle/tier1-apg/*` specs
/// would match this trigger too ("resolved to 2 elements").
#[component]
pub fn Demo() -> Element {
    let mut account = use_signal(|| "benoit".to_string());
    let mut theme = use_signal(|| "system".to_string());

    rsx! {
        Menubar {
            MenubarMenu { index: 0usize,
                MenubarTrigger { "Accounts" }
                MenubarContent {
                    MenubarLabel { id: "dx-mb-account-label", "Switch account" }
                    MenubarRadioGroup {
                        aria_labelledby: "dx-mb-account-label",
                        value: Some(account()),
                        on_value_change: move |value| account.set(value),
                        MenubarRadioItem {
                            index: 0usize,
                            value: "andy".to_string(),
                            close_on_select: false,
                            "Andy"
                        }
                        MenubarRadioItem {
                            index: 1usize,
                            value: "benoit".to_string(),
                            close_on_select: false,
                            "Benoit"
                        }
                        MenubarRadioItem {
                            index: 2usize,
                            value: "luis".to_string(),
                            close_on_select: false,
                            "Luis"
                        }
                    }
                }
            }
            MenubarMenu { index: 1usize,
                MenubarTrigger { "Theme" }
                MenubarContent {
                    MenubarRadioGroup {
                        aria_label: "Theme",
                        value: Some(theme()),
                        on_value_change: move |value| theme.set(value),
                        MenubarRadioItem {
                            index: 0usize,
                            value: "light".to_string(),
                            close_on_select: false,
                            "Light"
                        }
                        MenubarRadioItem {
                            index: 1usize,
                            value: "dark".to_string(),
                            close_on_select: false,
                            "Dark"
                        }
                        MenubarRadioItem {
                            index: 2usize,
                            value: "system".to_string(),
                            close_on_select: false,
                            "System"
                        }
                    }
                }
            }
        }
    }
}
