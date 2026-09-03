use super::super::component::*;
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            // axe `landmark-unique` (docs/backlog.md row 34's own round):
            // this demo's `nav` and the site's own top-level chrome
            // (`preview/src/main.rs`) were both `aria-label="Components"`,
            // a duplicate-landmark collision on this component's own demo
            // page -- a distinct instance of the same defect class row 26
            // named for `role="menuitem"` (a demo's content colliding with
            // the site's own chrome, both visible in the same accessibility
            // tree). Renamed rather than the site chrome, which is correct
            // on every route.
            Navbar { aria_label: "Example navigation",
                NavbarNav { index: 0usize,
                    NavbarTrigger { "Inputs" }
                    NavbarContent {
                        NavbarItem {
                            index: 0usize,
                            value: "calendar".to_string(),
                            to: Route::component("calendar"),
                            "Calendar"
                        }
                        NavbarItem {
                            index: 1usize,
                            value: "slider".to_string(),
                            to: Route::component("slider"),
                            disabled: true,
                            "Slider"
                        }
                        NavbarItem {
                            index: 2usize,
                            value: "checkbox".to_string(),
                            to: Route::component("checkbox"),
                            "Checkbox"
                        }
                        NavbarItem {
                            index: 3usize,
                            value: "radio_group".to_string(),
                            to: Route::component("radio_group"),
                            "Radio Group"
                        }
                    }
                }
                NavbarNav { index: 1usize,
                    NavbarTrigger { "Information" }
                    NavbarContent {
                        NavbarItem {
                            index: 0usize,
                            value: "toast".to_string(),
                            to: Route::component("toast"),
                            "Toast"
                        }
                        NavbarItem {
                            index: 1usize,
                            value: "tabs".to_string(),
                            to: Route::component("tabs"),
                            "Tabs"
                        }
                        NavbarItem {
                            index: 2usize,
                            value: "dialog".to_string(),
                            to: Route::component("dialog"),
                            "Dialog"
                        }
                        NavbarItem {
                            index: 3usize,
                            value: "alert_dialog".to_string(),
                            to: Route::component("alert_dialog"),
                            "Alert Dialog"
                        }
                        NavbarItem {
                            index: 4usize,
                            value: "tooltip".to_string(),
                            to: Route::component("tooltip"),
                            "Tooltip"
                        }
                    }
                }
                NavbarItem {
                    index: 2usize,
                    value: "home".to_string(),
                    to: Route::home(),
                    "Home"
                }
            }
        }
    }
}
