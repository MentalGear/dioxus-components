use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        Breadcrumb {
            BreadcrumbList {
                BreadcrumbItem {
                    BreadcrumbLink { href: "#", "Home" }
                }
                BreadcrumbSeparator {}
                BreadcrumbItem {
                    BreadcrumbEllipsis {}
                }
                BreadcrumbSeparator {}
                BreadcrumbItem {
                    BreadcrumbLink { href: "#", "Components" }
                }
                BreadcrumbSeparator {}
                BreadcrumbItem {
                    BreadcrumbPage { "Breadcrumb" }
                }
            }
        }
    }
}
