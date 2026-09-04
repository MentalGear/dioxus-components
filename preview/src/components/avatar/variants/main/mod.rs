use super::super::component::*;
use dioxus::prelude::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders several
// `ImageAvatar`/`Avatar` themed-wrapper instances below, each of which (as
// of this migration) now carries its own `document::Link` for `style.css`.
// `document::Link` dedupes on `(href, rel)`, so those links cover this page.
// Keep this request pending so the example uses the real avatar loading state.
const LOADING_AVATAR_SRC: &str = "https://httpbin.org/delay/3600";

#[component]
pub fn Demo() -> Element {
    let mut avatar_state = use_signal(|| "No state yet".to_string());
    rsx! {
        div {
            display: "flex",
            flex_direction: "row",
            align_items: "center",
            justify_content: "center",
            flex_wrap: "wrap",
            gap: "1rem",
            div { class: "dx-avatar-item",
                p { class: "dx-avatar-label", "Basic Usage" }
                ImageAvatar {
                    size: AvatarImageSize::Small,
                    src: "https://avatars.githubusercontent.com/u/66571940?s=96&v=4",
                    alt: "User avatar",
                    on_state_change: move |state| {
                        avatar_state.set(format!("Avatar 1: {state:?}"));
                    },
                    aria_label: "Basic avatar",
                    "EA"
                }
            }
            div { class: "dx-avatar-item",
                p { class: "dx-avatar-label", "Rounded" }
                ImageAvatar {
                    size: AvatarImageSize::Small,
                    shape: AvatarShape::Rounded,
                    src: "https://avatars.githubusercontent.com/u/66571940?s=96&v=4",
                    alt: "User avatar",
                    on_state_change: move |state| {
                        avatar_state.set(format!("Avatar 2: {state:?}"));
                    },
                    aria_label: "Basic avatar",
                    "EA"
                }
            }
            div { class: "dx-avatar-item",
                p { class: "dx-avatar-label", "Loading" }
                Avatar {
                    size: AvatarImageSize::Small,
                    aria_label: "Loading avatar",
                    AvatarImage {
                        src: LOADING_AVATAR_SRC,
                        alt: "",
                    }
                }
            }
            div { class: "dx-avatar-item",
                p { class: "dx-avatar-label", "Error State" }
                ImageAvatar {
                    size: AvatarImageSize::Medium,
                    src: "https://invalid-url.example/image.jpg",
                    alt: "Invalid image",
                    on_state_change: move |state| {
                        avatar_state.set(format!("Avatar 3: {state:?}"));
                    },
                    aria_label: "Error avatar",
                    "JK"
                }
            }
            div { class: "dx-avatar-item",
                p { class: "dx-avatar-label", "Large Size" }
                ImageAvatar {
                    size: AvatarImageSize::Large,
                    src: asset!("/assets/dioxus-logo.png", ImageAssetOptions::new().with_avif()).to_string(),
                    alt: "Large avatar",
                    on_state_change: move |state| {
                        avatar_state.set(format!("Avatar 4: {state:?}"));
                    },
                    aria_label: "Large avatar",
                    "DX"
                }
            }
        }
    }
}
