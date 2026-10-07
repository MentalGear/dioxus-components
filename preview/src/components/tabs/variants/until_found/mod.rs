use super::super::component::*;
use dioxus::prelude::*;

/// `hidden_until_found`: inactive panels stay mounted as `hidden="until-found"`, so the browser's
/// find-in-page (and a `#fragment` link) can reach them. The browser reveals the panel and its tab
/// becomes the active one.
///
/// The labels are distinct from the `main` variant's ("Tab 1".."Tab 3") so a locator by tab name
/// on the page stays unambiguous.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: var(--dx-space-3); width: 100%; max-width: 16rem;",
            Tabs {
                default_value: "overview".to_string(),
                horizontal: true,
                hidden_until_found: true,
                TabList {
                    TabTrigger { value: "overview".to_string(), index: 0usize, "Overview" }
                    TabTrigger { value: "specs".to_string(), index: 1usize, "Specs" }
                    TabTrigger { value: "reviews".to_string(), index: 2usize, "Reviews" }
                }
                TabContent { index: 0usize, value: "overview".to_string(),
                    div { "A small amphibian for small tanks." }
                }
                TabContent { index: 1usize, value: "specs".to_string(),
                    div { "Regenerates limbs: the axolotl is fully aquatic." }
                }
                TabContent { index: 2usize, value: "reviews".to_string(),
                    div { "Rated four stars by keepers." }
                }
            }
            p { style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                "Try Ctrl+F for \"axolotl\": its inactive tab activates to show the match."
            }
        }
    }
}
