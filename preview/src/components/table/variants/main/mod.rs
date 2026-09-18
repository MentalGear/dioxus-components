use super::super::component::*;
use dioxus::prelude::*;

struct Invoice {
    invoice: &'static str,
    status: &'static str,
    method: &'static str,
    amount: &'static str,
}

const INVOICES: &[Invoice] = &[
    Invoice { invoice: "INV001", status: "Paid", method: "Credit Card", amount: "$250.00" },
    Invoice { invoice: "INV002", status: "Pending", method: "PayPal", amount: "$150.00" },
    Invoice { invoice: "INV003", status: "Unpaid", method: "Bank Transfer", amount: "$350.00" },
    Invoice { invoice: "INV004", status: "Paid", method: "Credit Card", amount: "$450.00" },
    Invoice { invoice: "INV005", status: "Paid", method: "PayPal", amount: "$550.00" },
    Invoice { invoice: "INV006", status: "Pending", method: "Bank Transfer", amount: "$200.00" },
];

#[component]
pub fn Demo() -> Element {
    rsx! {
        Table {
            TableCaption { "A list of your recent invoices." }
            TableHeader {
                TableRow {
                    TableHead { "Invoice" }
                    TableHead { "Status" }
                    TableHead { "Method" }
                    TableHead { "Amount" }
                }
            }
            TableBody {
                for invoice in INVOICES.iter() {
                    TableRow { key: "{invoice.invoice}",
                        TableCell { "{invoice.invoice}" }
                        TableCell { "{invoice.status}" }
                        TableCell { "{invoice.method}" }
                        TableCell { "{invoice.amount}" }
                    }
                }
            }
            TableFooter {
                TableRow {
                    TableCell { colspan: 3, "Total" }
                    TableCell { "$1,950.00" }
                }
            }
        }
    }
}
