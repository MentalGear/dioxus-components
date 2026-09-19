use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. `Select`'s trigger/listbox navigation is
/// vertical-only (no `ArrowLeft`/`ArrowRight` to flip), so this fixture
/// exists mainly for the `dir`/`data-direction` attribute-presence
/// contract (`playwright/oracle/tier3-radix/rtl.spec.ts`) rather than a
/// keyboard behavior change.
///
/// Options are "First"/"Second"/"Third" rather than fruit names: the
/// `main` variant's own options come from a `Fruit` enum ("Apple"/
/// "Banana"/.../"Other") and `select.spec.ts` queries several of those by
/// name (e.g. `getByRole("option", { name: "apple" })`); since every
/// variant of a component renders on the same page at once
/// (`preview/src/main.rs`'s `ComponentHighlight`), reusing those words
/// here would make such a locator match this variant's option too.
///
/// `default_value` is set (unlike `main`, which starts unselected) so this
/// trigger's own accessible name is "First", not the generic placeholder
/// "Select an option" `Select` falls back to with nothing selected --
/// `select.spec.ts`'s own `singleSelectTrigger` helper matches exactly
/// that placeholder text, and every variant renders on the same page at
/// once, so an unselected trigger here would make that locator ambiguous
/// too (confirmed live).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                Select::<String> {
                    width: "12rem",
                    default_value: "first".to_string(),
                    SelectOption::<String> { index: 0usize, value: "first", "First" }
                    SelectOption::<String> { index: 1usize, value: "second", "Second" }
                    SelectOption::<String> { index: 2usize, value: "third", "Third" }
                }
            }
        }
    }
}
