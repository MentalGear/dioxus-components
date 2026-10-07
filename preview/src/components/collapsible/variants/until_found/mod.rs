use super::super::component::*;
use dioxus::prelude::*;

/// `hidden_until_found`: closed content stays mounted as `hidden="until-found"`, so the browser's
/// find-in-page (and a `#fragment` link) can reach it, reveal it and open the collapsible.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 20rem;",
            Collapsible { hidden_until_found: true,
                CollapsibleTrigger {
                    b { "Searchable activity" }
                }
                CollapsibleList {
                    CollapsibleItem { "Added a new feature to the searchable collapsible" }
                    CollapsibleContent {
                        CollapsibleItem { "Shipped the quokka tracker inside the closed panel" }
                        CollapsibleItem { "Updated the documentation for find-in-page" }
                    }
                }
                p { style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                    "Try Ctrl+F for \"quokka\": the closed panel opens to show the match."
                }
            }
        }
    }
}
