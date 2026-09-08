use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    let mut fruit = use_signal(|| "apple".to_string());
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 0.5rem; width: 16rem;",
            // A real `<label for>` rather than an aria-label: this component IS a
            // native `<select>`, so the demo should show the native labelling
            // idiom a consumer will actually use. Without it the control has no
            // accessible name at all -- axe `select-name`, critical, which this
            // component's own scan caught on its first run.
            label { r#for: "native-select-fruit", "Favorite fruit" }
            NativeSelect {
                id: "native-select-fruit",
                value: "{fruit}",
                onchange: move |e: FormEvent| fruit.set(e.value()),
                option { value: "apple", "Apple" }
                option { value: "banana", "Banana" }
                option { value: "blueberry", "Blueberry" }
                option { value: "grapes", "Grapes" }
                option { value: "pineapple", "Pineapple" }
            }
            p { style: "margin: 0; font-size: 0.875rem;", "Selected: {fruit}" }
        }
    }
}
