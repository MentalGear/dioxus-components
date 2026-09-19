use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};
use time::{macros::date, Date};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. `Calendar`'s day grid `ArrowLeft`/`ArrowRight`
/// resolves this ambient RTL direction via `horizontal_day_step`
/// (`primitives/src/calendar.rs`); `ArrowUp`/`ArrowDown` (+/-7 days) are
/// untouched.
#[component]
pub fn Demo() -> Element {
    let mut selected_date = use_signal(|| None::<Date>);
    let mut view_date = use_signal(|| date!(2026 - 05 - 15));
    rsx! {
        div { dir: "rtl", style: "padding: 20px;",
            DirectionProvider { direction: Direction::Rtl,
                Calendar {
                    selected_date: selected_date(),
                    on_date_change: move |date| selected_date.set(date),
                    view_date: view_date(),
                    on_view_change: move |new_view: Date| view_date.set(new_view),
                    min_date: date!(1995 - 07 - 21),
                    max_date: date!(2035 - 09 - 11),
                }
            }
        }
    }
}
