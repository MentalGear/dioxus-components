use dioxus::prelude::*;

use super::super::component::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders several
// `Badge` themed-wrapper instances below, each of which (as of this
// migration) now carries its own `document::Link` for `style.css`.
// `document::Link` dedupes on `(href, rel)`, so those links cover this page.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { class: "dx-badge-example",

            Badge { "Primary" }
            Badge { variant: BadgeVariant::Secondary, "Secondary" }
            Badge { variant: BadgeVariant::Destructive, "Destructive" }
            Badge { variant: BadgeVariant::Outline, "Outline" }
            Badge {
                variant: BadgeVariant::Secondary,
                style: "background-color: var(--focused-border-color)",
                VerifiedIcon {}
                "Verified"
            }
        }
    }
}
