use dioxus::prelude::*;

// `dx components add <name>` writes each component into `src/components/`.
mod components;

// After your first `dx components add`, the shared theme exists at `assets/dx-components-theme.css`.
// Link it once in `App` by adding the line below (before that the file does not exist, and `asset!`
// refuses to compile for a missing file):
//
//     document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        main {
            h1 { "{{project-name}}" }
            p { "Add a component with `dx components add button`, then use it from here." }
        }
    }
}
