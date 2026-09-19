use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape, including why the item labels below
/// ("Amber"/"Teal"/"Violet") deliberately differ from the `main` variant's
/// ("Blue"/"Red"/"Green"): `radio-group.spec.ts`'s `getByRole("radio",
/// { name: "Blue" })` has no `exact`, so once this variant renders
/// alongside `main` on the same page, an identical label would make that
/// locator match both. `RadioGroup`'s own `ArrowLeft`/`ArrowRight` roving
/// focus (only reachable with `horizontal: true`) resolves this ambient
/// RTL direction. Deliberately all-enabled (unlike the `main` variant's
/// disabled "Green"/third item): the oracle tests the *middle* item's
/// adjacent-neighbor transition, which needs no wrap-around or
/// skip-disabled behavior entangled with the direction flip under test.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                RadioGroup { horizontal: true,
                    RadioItem { value: "option1".to_string(), index: 0usize, "Amber" }
                    RadioItem { value: "option2".to_string(), index: 1usize, "Teal" }
                    RadioItem { value: "option3".to_string(), index: 2usize, "Violet" }
                }
            }
        }
    }
}
