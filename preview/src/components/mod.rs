use super::{ComponentDemoData, ComponentType, ComponentVariantDemoData, HighlightedCode};
// Only `css_highlight!`'s debug-only arm needs `asset!` -- gating the
// import too keeps a release build (which never calls it) from a bare
// `unused_imports` warning.
#[cfg(debug_assertions)]
use dioxus::prelude::*;

/// Builds this codebase's `CssHighlight` (`main.rs`) for one component's
/// CSS file (`style.css` or a Block-kind variant's `demo.css`). See
/// `CssHighlight`'s own doc comment for the full why: release/SSG builds
/// keep the previous compile-time `dioxus_code::code!()` embed unchanged;
/// debug builds (`dx serve`'s dev loop) carry only the file's own
/// `asset!()` URL instead, so an edit hot-reloads via the asset pipeline
/// rather than forcing a full rebuild through `code!()`'s
/// `include_str!(path)` (which makes rustc -- and in turn `dx serve`'s own
/// file-change classifier, which reads the compiled crate's rustc
/// dep-info -- treat the `.css` file as a source dependency of this
/// crate). Measured before/after: `dev-docs/dev-loop.md`'s CSS section.
///
/// Two full, cfg-gated definitions rather than one macro with a cfg'd
/// struct-literal field: `CssHighlight`'s own two fields are themselves
/// `#[cfg]`-gated (release-only `embedded`, debug-only `asset`), so each
/// mode's definition here can only ever construct the field that exists
/// in it.
#[cfg(not(debug_assertions))]
macro_rules! css_highlight {
    ($path:expr) => {
        crate::CssHighlight {
            embedded: crate::HighlightedCode {
                source: dioxus_code::code!($path),
            },
        }
    };
}

#[cfg(debug_assertions)]
macro_rules! css_highlight {
    ($path:expr) => {
        crate::CssHighlight {
            asset: asset!($path),
        }
    };
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ComponentCategory {
    Forms,
    Navigation,
    Overlays,
    Feedback,
    Disclosure,
    DataDisplay,
}

impl ComponentCategory {
    pub const ALL: &'static [Self] = &[
        Self::Forms,
        Self::Navigation,
        Self::Overlays,
        Self::Feedback,
        Self::Disclosure,
        Self::DataDisplay,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Forms => "Forms",
            Self::Navigation => "Navigation",
            Self::Overlays => "Overlays",
            Self::Feedback => "Feedback",
            Self::Disclosure => "Disclosure",
            Self::DataDisplay => "Data display",
        }
    }
}

pub fn category_of(name: &str) -> ComponentCategory {
    match name {
        "button" | "input" | "textarea" | "label" | "checkbox" | "switch" | "radio_group"
        | "toggle" | "toggle_group" | "select" | "slider" | "calendar" | "date_picker"
        | "color_picker" | "form" | "button_group" | "field" | "input_group" | "input_otp"
        | "native_select" => ComponentCategory::Forms,
        "navbar" | "sidebar" | "tabs" | "pagination" | "menubar" | "toolbar" | "context_menu"
        | "dropdown_menu" | "breadcrumb" | "navigation_menu" => ComponentCategory::Navigation,
        "dialog" | "alert_dialog" | "sheet" | "drawer" | "popover" | "tooltip" | "hover_card"
        | "top_layer" | "command" => ComponentCategory::Overlays,
        "toast" | "progress" | "skeleton" | "badge" | "alert" | "empty" | "spinner" => {
            ComponentCategory::Feedback
        }
        "accordion" | "collapsible" => ComponentCategory::Disclosure,
        "avatar" | "card" | "separator" | "aspect_ratio" | "item" | "drag_and_drop_list"
        | "virtual_list" | "scroll_area" | "tag_group" | "table" | "data_table" | "resizable" => {
            ComponentCategory::DataDisplay
        }
        _ => ComponentCategory::DataDisplay,
    }
}

macro_rules! examples {
    ($($name:ident $(($kind:ident))? $([$($variant:ident),*])?),* $(,)?) => {
        $(
            pub(crate) mod $name {
                pub(crate) mod component;
                #[allow(unused)]
                pub use component::*;
                pub(crate) mod variants {
                    pub(crate) mod main;
                    $(
                        $(
                            pub(crate) mod $variant;
                        )*
                    )?
                }
            }
        )*
        pub(crate) static DEMOS: &[ComponentDemoData] = &[
            $(
                examples!(@demo $name $( $kind )? $([$($variant),*])?),
            )*
        ];
    };

    (@kind) => { ComponentType::Normal };
    (@kind normal) => { ComponentType::Normal };
    (@kind block) => { ComponentType::Block };

    // Normal components: no variant-level css_highlighted
    (@demo $name:ident $([$($variant:ident),*])?) => {
        ComponentDemoData {
            name: stringify!($name),
            r#type: ComponentType::Normal,
            description: include_str!(concat!(
                env!("OUT_DIR"),
                "/",
                stringify!($name),
                "/description.txt"
            )),
            docs: include_str!(concat!(env!("OUT_DIR"), "/", stringify!($name), "/docs.html")),
            component: HighlightedCode {
                source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/component.rs")),
            },
            style: css_highlight!(concat!("/src/components/", stringify!($name), "/style.css")),
            variants: &[
                ComponentVariantDemoData {
                    name: "main",
                    rs_highlighted: HighlightedCode {
                        source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/variants/main/mod.rs")),
                    },
                    css_highlighted: None,
                    component: $name::variants::main::Demo,
                },
                $(
                    $(
                        ComponentVariantDemoData {
                            name: stringify!($variant),
                            rs_highlighted: HighlightedCode {
                                source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/variants/", stringify!($variant), "/mod.rs")),
                            },
                            css_highlighted: None,
                            component: $name::variants::$variant::Demo,
                        },
                    )*
                )?
            ],
        }
    };

    // Block components: rendered in iframe, with shared demo.css
    (@demo $name:ident block $([$($variant:ident),*])?) => {
        ComponentDemoData {
            name: stringify!($name),
            r#type: ComponentType::Block,
            description: include_str!(concat!(
                env!("OUT_DIR"),
                "/",
                stringify!($name),
                "/description.txt"
            )),
            docs: include_str!(concat!(env!("OUT_DIR"), "/", stringify!($name), "/docs.html")),
            component: HighlightedCode {
                source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/component.rs")),
            },
            style: css_highlight!(concat!("/src/components/", stringify!($name), "/style.css")),
            variants: &[
                ComponentVariantDemoData {
                    name: "main",
                    rs_highlighted: HighlightedCode {
                        source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/variants/main/mod.rs")),
                    },
                    css_highlighted: Some(css_highlight!(concat!("/src/components/", stringify!($name), "/variants/demo.css"))),
                    component: $name::variants::main::Demo,
                },
                $(
                    $(
                        ComponentVariantDemoData {
                            name: stringify!($variant),
                            rs_highlighted: HighlightedCode {
                                source: dioxus_code::code!(concat!("/src/components/", stringify!($name), "/variants/", stringify!($variant), "/mod.rs")),
                            },
                            css_highlighted: Some(css_highlight!(concat!("/src/components/", stringify!($name), "/variants/demo.css"))),
                            component: $name::variants::$variant::Demo,
                        },
                    )*
                )?
            ],
        }
    };
}

examples!(
    accordion,
    alert,
    alert_dialog,
    aspect_ratio,
    avatar,
    badge,
    breadcrumb,
    button[size, icon],
    button_group,
    calendar[simple, internationalized, range, multi_month, unavailable_dates, rtl],
    card,
    checkbox,
    collapsible,
    color_picker,
    combobox[controlled, disabled, dynamic],
    command,
    context_menu[rtl],
    data_table,
    date_picker[internationalized, range, multi_month, unavailable_dates],
    dialog,
    drag_and_drop_list[removable],
    drawer,
    dropdown_menu[rtl],
    empty,
    field,
    form,
    hover_card,
    input,
    input_group,
    input_otp,
    item[variant, size, image, group],
    kbd,
    label,
    menubar[rtl],
    native_select,
    navigation_menu,
    navbar[rtl],
    pagination,
    popover[non_modal],
    progress,
    radio_group[rtl],
    resizable[rtl],
    scroll_area[rtl],
    select[multi, rtl],
    separator,
    sheet,
    sidebar(block)[floating, inset],
    skeleton,
    slider[dynamic_range, range, rtl],
    spinner,
    switch,
    table,
    tabs[rtl],
    tag_group[multi, states],
    textarea[outline, fade, ghost],
    toast,
    toggle,
    toggle_group[rtl],
    toolbar[rtl],
    tooltip,
    top_layer,
    virtual_list[random_heights],
);
