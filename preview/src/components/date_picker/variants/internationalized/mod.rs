use super::super::component::*;
use dioxus::prelude::*;

use dioxus_i18n::tid;
use time::{Date, Month};

#[component]
pub fn Demo() -> Element {
    let mut selected_date = use_signal(|| None::<Date>);

    rsx! {
        div {
            DatePicker {
                selected_date: selected_date(),
                on_value_change: move |v| {
                    tracing::info!("Selected date changed: {:?}", v);
                    selected_date.set(v);
                },
                on_format_day_placeholder: || tid!("D_Abbr"),
                on_format_month_placeholder: || tid!("M_Abbr"),
                on_format_year_placeholder: || tid!("Y_Abbr"),
                // backlog row 84 finding 2: the month segment's
                // `aria-valuetext` needs a locale-aware month name, the same
                // way the standalone `Calendar` internationalized demo
                // already feeds its grid's month title
                // (`calendar/variants/internationalized/mod.rs`'s identical
                // `on_format_month: |month: Month| tid!(& month.to_string())`).
                on_format_month: |month: Month| tid!(& month.to_string()),
            }
        }
    }
}
