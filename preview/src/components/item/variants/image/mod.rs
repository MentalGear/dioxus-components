use super::super::component::*;
use dioxus::prelude::*;

// (title, artist, album, duration, cover gradient from, cover gradient to)
const TRACKS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "Midnight City Lights",
        "Neon Dreams",
        "Electric Nights",
        "3:45",
        "1e1b4b",
        "f472b6",
    ),
    (
        "Coffee Shop Conversations",
        "The Morning Brew",
        "Urban Stories",
        "4:05",
        "78350f",
        "fbbf24",
    ),
    (
        "Digital Rain",
        "Cyber Symphony",
        "Binary Beats",
        "3:30",
        "064e3b",
        "67e8f9",
    ),
];

/// A self-contained gradient "cover" (an inline SVG), so the demo needs no
/// network and hot-links no remote image.
fn cover(from: &str, to: &str) -> String {
    format!(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 96 96'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' y1='0' x2='1' y2='1'%3E%3Cstop offset='0' stop-color='%23{from}'/%3E%3Cstop offset='1' stop-color='%23{to}'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='96' height='96' fill='url(%23g)'/%3E%3C/svg%3E"
    )
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            width: "100%",
            max_width: "28rem",

            ItemGroup { gap: "1rem",
                for (title , artist , album , duration , from , to) in TRACKS.iter() {
                    Item {
                        variant: ItemVariant::Outline,
                        as: move |attrs: Vec<Attribute>| rsx! {
                            a { href: "#", ..attrs,
                                ItemMedia { variant: ItemMediaVariant::Image,
                                    img {
                                        src: cover(from, to),
                                        alt: "{title}",
                                        style: "filter: grayscale(1)",
                                    }
                                }
                                ItemContent {
                                    ItemTitle { "{title} — {album}" }
                                    ItemDescription { "{artist}" }
                                }
                                ItemContent { flex: "none",
                                    ItemDescription { "{duration}" }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
