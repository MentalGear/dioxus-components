use super::super::component::{
    filter_rows, paginate, sort_rows, DataTableColumnHeader, DataTablePagination, DataTableToolbar,
    SortDirection,
};
use crate::components::checkbox::Checkbox;
use crate::components::table::*;
use dioxus::prelude::*;
use dioxus_primitives::checkbox::CheckboxState;
use std::collections::HashSet;

// Fixture shaped like shadcn/ui's own Data Table demo (id/status/email/
// amount, ~12 rows) per dev-docs/component-backlog.md row 69's demo brief.
// Values are this crate's own, chosen so the sort orders below are easy to
// verify by inspection -- `playwright/data_table.spec.ts` and
// `playwright/oracle/tier1-apg/sortable-table.spec.ts` assert the exact
// row transitions these produce, so this fixture is these specs' fixture
// too (single source of truth, not duplicated).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Payment {
    id: &'static str,
    status: &'static str,
    email: &'static str,
    /// Whole US cents, not a float -- avoids float-equality/ordering
    /// pitfalls in `sort_rows`'s `Ord` key.
    amount_cents: i64,
}

const PAYMENTS: &[Payment] = &[
    Payment { id: "PMT-01", status: "Success", email: "ken99@example.com", amount_cents: 31_600 },
    Payment { id: "PMT-02", status: "Processing", email: "carmella@example.com", amount_cents: 24_200 },
    Payment { id: "PMT-03", status: "Success", email: "alexandra@example.com", amount_cents: 83_700 },
    Payment { id: "PMT-04", status: "Failed", email: "michael@example.com", amount_cents: 72_100 },
    Payment { id: "PMT-05", status: "Pending", email: "jenny@example.com", amount_cents: 50_000 },
    Payment { id: "PMT-06", status: "Success", email: "derek@example.com", amount_cents: 12_000 },
    Payment { id: "PMT-07", status: "Pending", email: "olivia@example.com", amount_cents: 61_000 },
    Payment { id: "PMT-08", status: "Processing", email: "hannah@example.com", amount_cents: 7_500 },
    Payment { id: "PMT-09", status: "Success", email: "isaac@example.com", amount_cents: 43_000 },
    Payment { id: "PMT-10", status: "Failed", email: "monroe@example.com", amount_cents: 99_900 },
    Payment { id: "PMT-11", status: "Pending", email: "zara@example.com", amount_cents: 5_500 },
    Payment { id: "PMT-12", status: "Success", email: "felix@example.com", amount_cents: 28_900 },
];

fn format_amount(cents: i64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

#[component]
pub fn Demo() -> Element {
    // Exactly one column is the "active" sort at all times (default:
    // email ascending) rather than an `Option<(key, dir)>` with a
    // "nothing sorted" state -- matching the vendored APG sortable-table
    // reference, which likewise always has exactly one column carrying
    // `aria-sort` (see `DataTableColumnHeader`'s doc and
    // `playwright/oracle/tier1-apg/sortable-table.spec.ts`'s R1).
    let mut sort_key = use_signal(|| "email");
    let mut sort_direction = use_signal(|| SortDirection::Ascending);
    let mut filter = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let mut page_size = use_signal(|| 5usize);
    let mut selected = use_signal(HashSet::<&'static str>::new);

    let mut toggle_sort = move |key: &'static str| {
        if sort_key() == key {
            sort_direction.set(sort_direction().toggled());
        } else {
            sort_key.set(key);
            sort_direction.set(SortDirection::Ascending);
        }
        page.set(0);
    };

    let query = filter().trim().to_lowercase();
    let filtered: Vec<Payment> =
        filter_rows(PAYMENTS, |p| query.is_empty() || p.email.to_lowercase().contains(&query));

    let sorted: Vec<Payment> = match sort_key() {
        "amount" => sort_rows(&filtered, sort_direction(), |p| p.amount_cents),
        _ => sort_rows(&filtered, sort_direction(), |p| p.email),
    };

    let (page_rows, page_count) = paginate(&sorted, page(), page_size());

    let all_visible_selected = !page_rows.is_empty() && page_rows.iter().all(|p| selected().contains(p.id));
    let any_visible_selected = page_rows.iter().any(|p| selected().contains(p.id));
    let select_all_state = if all_visible_selected {
        CheckboxState::Checked
    } else if any_visible_selected {
        CheckboxState::Indeterminate
    } else {
        CheckboxState::Unchecked
    };
    let visible_ids: Vec<&'static str> = page_rows.iter().map(|p| p.id).collect();

    rsx! {
        div { class: "dx-data-table-demo", style: "display: flex; flex-direction: column; gap: 1rem;",
            DataTableToolbar {
                value: filter(),
                oninput: move |value| {
                    filter.set(value);
                    page.set(0);
                },
                placeholder: "Filter emails...",
                aria_label: "Filter by email",
            }
            Table {
                TableHeader {
                    TableRow {
                        TableHead {
                            Checkbox {
                                checked: select_all_state,
                                aria_label: "Select all",
                                on_checked_change: move |state: CheckboxState| {
                                    let checked = bool::from(state);
                                    selected
                                        .with_mut(|set| {
                                            for id in visible_ids.iter().copied() {
                                                if checked {
                                                    set.insert(id);
                                                } else {
                                                    set.remove(id);
                                                }
                                            }
                                        });
                                },
                            }
                        }
                        TableHead { "ID" }
                        TableHead { "Status" }
                        DataTableColumnHeader {
                            sorted: (sort_key() == "email").then_some(sort_direction()),
                            onclick: move |_| toggle_sort("email"),
                            "Email"
                        }
                        DataTableColumnHeader {
                            sorted: (sort_key() == "amount").then_some(sort_direction()),
                            onclick: move |_| toggle_sort("amount"),
                            "Amount"
                        }
                    }
                }
                TableBody {
                    if page_rows.is_empty() {
                        TableRow {
                            TableCell { colspan: 5, "No results." }
                        }
                    }
                    for row in page_rows.iter().copied() {
                        TableRow { key: "{row.id}", selected: selected().contains(row.id),
                            TableCell {
                                Checkbox {
                                    checked: if selected().contains(row.id) {
                                        CheckboxState::Checked
                                    } else {
                                        CheckboxState::Unchecked
                                    },
                                    aria_label: "Select row {row.id}",
                                    on_checked_change: move |state: CheckboxState| {
                                        let checked = bool::from(state);
                                        selected
                                            .with_mut(|set| {
                                                if checked {
                                                    set.insert(row.id);
                                                } else {
                                                    set.remove(row.id);
                                                }
                                            });
                                    },
                                }
                            }
                            TableCell { "{row.id}" }
                            TableCell { "{row.status}" }
                            TableCell { "{row.email}" }
                            TableCell { "{format_amount(row.amount_cents)}" }
                        }
                    }
                }
            }
            DataTablePagination {
                selected_count: selected().len(),
                total_count: filtered.len(),
                page: page(),
                page_count,
                page_size: page_size(),
                page_size_options: vec![5, 10, 20],
                on_page_size_change: move |value| {
                    page_size.set(value);
                    page.set(0);
                },
                on_previous: move |_| page.set(page().saturating_sub(1)),
                on_next: move |_| page.set((page() + 1).min(page_count.saturating_sub(1))),
            }
        }
    }
}
