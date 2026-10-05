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

/// One entry of a panel's link list -- shadcn's demo `ListItem`: a title
/// (`text-sm leading-none font-medium`) over a muted description clamped to
/// two lines. Every list link in this demo goes through it, so a link can
/// never render as bare text next to titled siblings (the bug that made
/// "Button"/"Input" larger and lighter than "Introduction").
#[component]
fn ListItem(href: String, index: usize, title: String, description: String) -> Element {
    rsx! {
        NavigationMenuLink { href, content_index: index,
            div { class: "dx-navigation-menu-link-title", "{title}" }
            div { class: "dx-navigation-menu-link-description", "{description}" }
        }
    }
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
                                ListItem {
                                    href: site_href("/docs"),
                                    index: 1usize,
                                    title: "Introduction",
                                    description: "Re-usable primitives you can copy into your own project.",
                                }
                                ListItem {
                                    href: site_href("/component/button/"),
                                    index: 2usize,
                                    title: "Button",
                                    description: "Displays a button or a component that looks like a button.",
                                }
                                ListItem {
                                    href: site_href("/component/input/"),
                                    index: 3usize,
                                    title: "Input",
                                    description: "Displays a form input field or a component that looks like an input field.",
                                }
                            }
                        }
                    }
                    NavigationMenuItem { index: 1usize,
                        NavigationMenuTrigger { "Components" }
                        NavigationMenuContent {
                            div { class: "dx-navigation-menu-grid",
                                ListItem {
                                    href: site_href("/component/accordion/"),
                                    index: 0usize,
                                    title: "Accordion",
                                    description: "A vertically stacked set of collapsible panels.",
                                }
                                ListItem {
                                    href: site_href("/component/dialog/"),
                                    index: 1usize,
                                    title: "Dialog",
                                    description: "A modal window layered above the page.",
                                }
                                ListItem {
                                    href: site_href("/component/tooltip/"),
                                    index: 2usize,
                                    title: "Tooltip",
                                    description: "A short message shown on hover or focus.",
                                }
                                ListItem {
                                    href: site_href("/component/tabs/"),
                                    index: 3usize,
                                    title: "Tabs",
                                    description: "Switch between panels of related content.",
                                }
                                ListItem {
                                    href: site_href("/component/select/"),
                                    index: 4usize,
                                    title: "Select",
                                    description: "Pick one value from a list of options.",
                                }
                                ListItem {
                                    href: site_href("/component/progress/"),
                                    index: 5usize,
                                    title: "Progress",
                                    description: "Displays the completion progress of a task.",
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
