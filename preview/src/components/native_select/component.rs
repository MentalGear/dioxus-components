use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;

/// A styled native `<select>` -- for the common case where the full custom
/// listbox behaviour of [`Select`](crate::components::select::Select)
/// (search, portalled popover, custom option rendering) isn't needed and
/// the platform's own picker (native mobile wheel, OS-native styling) is
/// preferable. Wraps a plain `<select>`, so it comes with the browser's
/// own keyboard/typeahead/form-participation behaviour for free -- there is
/// no APG contract to add on top, and no primitive underneath.
#[component]
pub fn NativeSelect(
    onchange: Option<EventHandler<FormEvent>>,
    oninput: Option<EventHandler<FormEvent>>,
    onfocus: Option<EventHandler<FocusEvent>>,
    onblur: Option<EventHandler<FocusEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = select)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/native_select/style.css") }
        div { class: "dx-native-select-wrapper",
            select {
                class: "dx-native-select",
                onchange: move |e| _ = onchange.map(|callback| callback(e)),
                oninput: move |e| _ = oninput.map(|callback| callback(e)),
                onfocus: move |e| _ = onfocus.map(|callback| callback(e)),
                onblur: move |e| _ = onblur.map(|callback| callback(e)),
                ..attributes,
                {children}
            }
            ChevronDown { class: "dx-native-select-icon" }
        }
    }
}
