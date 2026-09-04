use dioxus::prelude::*;
use dioxus_icons::lucide::Check;
use dioxus_primitives::checkbox::{self, CheckboxProps};

// docs/backlog.md row 32: `#[css_module]` is gone -- this stylesheet is now
// plain, unhashed `dx-`-prefixed CSS (collision safety comes from
// scripts/check-dx-class-prefix.sh instead of a hash), delivered the same
// way `main.rs`'s `GlobalHead`/`LanguageSelect` already deliver theirs: an
// `asset!()`-bundled path linked via `document::Link`. Unlike those two
// call sites this one lives *inside* the themed wrapper itself so a
// `dx components add checkbox`-copied component keeps working with zero
// extra wiring in the consumer's own app -- `asset!()` resolves this path
// from the crate root exactly like `#[css_module]`'s own path argument
// used to (manganis-macro's `resolve_path`), and `dx components add`
// preserves this file's `src/components/checkbox/` location verbatim in
// the consumer project (dioxus-cli's `components_root` defaults to
// `<crate>/src/components`), so the path keeps resolving post-copy.
// `document::Link` dedupes by `(href, rel)` (dioxus-document's own doc
// comment on `LinkProps::href`), so re-rendering `Checkbox` many times on
// one page inserts the stylesheet exactly once, the same guarantee
// `#[css_module]`'s own `OnceLock`-guarded injection used to give.
#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/checkbox/style.css") }
        checkbox::Checkbox {
            class: "dx-checkbox",
            checked: props.checked,
            default_checked: props.default_checked,
            required: props.required,
            disabled: props.disabled,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: props.attributes,
            checkbox::CheckboxIndicator { class: "dx-checkbox-indicator",
                Check { size: "1rem" }
            }
        }
    }
}
