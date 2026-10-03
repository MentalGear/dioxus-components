use super::super::component::*;
use dioxus::prelude::*;

/// A root-absolute site path with the app base path (`dx --base-path`, e.g.
/// `/dioxus-components` on Pages) in front. `NavigationMenuLink` renders a
/// plain `<a>`, not a router `Link`, so it does not get the prefix for free
/// and a bare `"/docs"` would 404 when the site is served below a sub-path.
fn site_href(path: &str) -> String {
    let prefix = try_router()
        .and_then(|router| router.prefix())
        .unwrap_or_default();
    format!("{}{path}", prefix.trim_end_matches('/'))
}

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
                                    href: site_href("/"),
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
                                    NavigationMenuLink { href: site_href("/docs"), content_index: 1usize,
                                        div { class: "dx-navigation-menu-link-title", "Introduction" }
                                        div { class: "dx-navigation-menu-link-description",
                                            "Re-usable primitives you can copy into your own project."
                                        }
                                    }
                                    NavigationMenuLink { href: site_href("/component/button/"), content_index: 2usize, "Button" }
                                    NavigationMenuLink { href: site_href("/component/input/"), content_index: 3usize, "Input" }
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 1usize,
                        NavigationMenuTrigger { "Components" }
                        NavigationMenuContent {
                            div { class: "dx-navigation-menu-grid",
                                NavigationMenuLink { href: site_href("/component/accordion/"), content_index: 0usize,
                                    div { class: "dx-navigation-menu-link-title", "Accordion" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A vertically stacked set of collapsible panels."
                                    }
                                }
                                NavigationMenuLink { href: site_href("/component/dialog/"), content_index: 1usize,
                                    div { class: "dx-navigation-menu-link-title", "Dialog" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A modal window layered above the page."
                                    }
                                }
                                NavigationMenuLink { href: site_href("/component/tooltip/"), content_index: 2usize,
                                    div { class: "dx-navigation-menu-link-title", "Tooltip" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "A short message shown on hover or focus."
                                    }
                                }
                                NavigationMenuLink { href: site_href("/component/tabs/"), content_index: 3usize,
                                    div { class: "dx-navigation-menu-link-title", "Tabs" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Switch between panels of related content."
                                    }
                                }
                                NavigationMenuLink { href: site_href("/component/select/"), content_index: 4usize,
                                    div { class: "dx-navigation-menu-link-title", "Select" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Pick one value from a list of options."
                                    }
                                }
                                NavigationMenuLink { href: site_href("/component/progress/"), content_index: 5usize,
                                    div { class: "dx-navigation-menu-link-title", "Progress" }
                                    div { class: "dx-navigation-menu-link-description",
                                        "Displays the completion progress of a task."
                                    }
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 2usize,
                        NavigationMenuLink { href: site_href("/docs"), "Docs" }
                    }
                }
            }
        }
    }
}
