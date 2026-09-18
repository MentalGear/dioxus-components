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
                                    content_index: 0usize,
                                    class: "dx-navigation-menu-featured",
                                    // Deliberately not "dioxus-components" --
                                    // the site's own persistent chrome
                                    // (`main.rs`'s navbar brand link) already
                                    // renders an `<a>` with that exact
                                    // accessible name on every page, and
                                    // `getByRole('link', { name: ... })`
                                    // (both this demo's own oracle/smoke
                                    // specs and any real screen-reader user's
                                    // link-by-name navigation) would
                                    // otherwise hit two matches -- confirmed
                                    // by live reproduction against a running
                                    // dev server. Same class of fix as this
                                    // component's own `aria_label` above.
                                    div { class: "dx-navigation-menu-link-title", "Component Library" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Beautifully designed, accessible primitives for Dioxus."
                                    }
                                }
                                div { class: "dx-navigation-menu-grid",
                                    style: "grid-template-columns: 1fr;",
                                    NavigationMenuLink { href: "/docs", content_index: 1usize,
                                        div { class: "dx-navigation-menu-link-title", "Introduction" }
                                        div { class: "dx-navigation-menu-link-description",
                                            "Re-usable primitives you can copy into your own project."
                                        }
                                    }
                                    NavigationMenuLink { href: "/component/?name=button", content_index: 2usize, "Button" }
                                    NavigationMenuLink { href: "/component/?name=input", content_index: 3usize, "Input" }
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 1usize,
                        NavigationMenuTrigger { "Components" }
                        NavigationMenuContent {
                            div { class: "dx-navigation-menu-grid",
                                NavigationMenuLink { href: "/component/?name=accordion", content_index: 0usize,
                                    div { class: "dx-navigation-menu-link-title", "Accordion" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A vertically stacked set of collapsible panels."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=dialog", content_index: 1usize,
                                    div { class: "dx-navigation-menu-link-title", "Dialog" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A modal window layered above the page."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=tooltip", content_index: 2usize,
                                    div { class: "dx-navigation-menu-link-title", "Tooltip" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A short message shown on hover or focus."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=tabs", content_index: 3usize,
                                    div { class: "dx-navigation-menu-link-title", "Tabs" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Switch between panels of related content."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=select", content_index: 4usize,
                                    div { class: "dx-navigation-menu-link-title", "Select" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Pick one value from a list of options."
                                    }
                                }
                                NavigationMenuLink { href: "/component/?name=progress", content_index: 5usize,
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
