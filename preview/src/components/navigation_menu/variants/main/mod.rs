use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            // A distinct accessible name from the site's own top-level nav
            // avoids an axe `landmark-unique` collision on this demo page
            // -- same reasoning as `../../navbar/variants/main/mod.rs`'s
            // identical comment.
            NavigationMenu { aria_label: "Component navigation menu",
                NavigationMenuList {
                    NavigationMenuItem { index: 0usize,
                        NavigationMenuTrigger { "Getting started" }
                        NavigationMenuContent {
                            div { class: "dx-navigation-menu-grid dx-navigation-menu-grid-2col",
                                NavigationMenuLink {
                                    href: "/",
                                    class: "dx-navigation-menu-featured",
                                    div { class: "dx-navigation-menu-link-title", "dioxus-components" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Beautifully designed, accessible primitives for Dioxus."
                                    }
                                }
                                div { class: "dx-navigation-menu-grid",
                                    style: "grid-template-columns: 1fr;",
                                    NavigationMenuLink { href: "/docs",
                                        div { class: "dx-navigation-menu-link-title", "Introduction" }
                                        div { class: "dx-navigation-menu-link-description",
                                            "Re-usable primitives you can copy into your own project."
                                        }
                                    }
                                    NavigationMenuLink { href: "/component/?name=button", "Button" }
                                    NavigationMenuLink { href: "/component/?name=input", "Input" }
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 1usize,
                        NavigationMenuTrigger { "Components" }
                        NavigationMenuContent {
                            div { class: "dx-navigation-menu-grid",
                                NavigationMenuLink { href: "/component/?name=accordion",
                                    div { class: "dx-navigation-menu-link-title", "Accordion" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A vertically stacked set of collapsible panels."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=dialog",
                                    div { class: "dx-navigation-menu-link-title", "Dialog" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A modal window layered above the page."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=tooltip",
                                    div { class: "dx-navigation-menu-link-title", "Tooltip" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A short message shown on hover or focus."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=tabs",
                                    div { class: "dx-navigation-menu-link-title", "Tabs" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Switch between panels of related content."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=select",
                                    div { class: "dx-navigation-menu-link-title", "Select" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Pick one value from a list of options."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=progress",
                                    div { class: "dx-navigation-menu-link-title", "Progress" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Displays the completion progress of a task."
                                    }
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 2usize,
                        NavigationMenuLink { href: "/docs", "Docs" }
                    }
                }
            }
        }
    }
}
