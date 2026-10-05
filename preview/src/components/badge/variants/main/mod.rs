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
            // The "Verified" badge: a brand-blue fill (`--dx-ring-color`) with a
            // pinned white ink. The fill's OKLCH lightness is clamped to at most 0.5,
            // which is a no-op for a ring colour that is already deep and pulls a
            // light one (the unclamped #2b7fff under the secondary variant's
            // near-white dark-mode ink was 3.60:1) down to >= 4.5:1 for 12px text.
            // `--dx-primary-ink` (theme) is the same clamp idea for ink on a page.
            Badge {
                variant: BadgeVariant::Secondary,
                style: "background-color: oklch(from var(--dx-ring-color) min(l, 0.5) c h); color: #fff",
                VerifiedIcon {}
                "Verified"
            }
        }
    }
}
