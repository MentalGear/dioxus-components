use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::ContentSide;

#[component]
pub fn Demo() -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { "Rich content" }
            TooltipContent { side: ContentSide::Left, style: "width: 200px;",
                // axe `heading-order` (docs/backlog.md row 34's own round):
                // this demo's main-variant content sits directly after the
                // page's own h1 (no h2/h3 between them -- see
                // `main.rs`'s `ComponentVariantHighlight`), so an h4 here
                // skipped straight from 1 to 4. A tooltip's ephemeral popup
                // content is not part of the document's heading outline
                // anyway -- a styled `p`, same visual weight, not a heading.
                p { style: "margin: 0 0 8px; font-weight: 660;", "Tooltip title" }
                p { style: "margin: 0;", "This tooltip contains rich HTML content with styling." }
            }
        }
    }
}
