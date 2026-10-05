//! Defines the [`Tabs`] component and its sub-components.

use crate::{
    collapsible::{hidden_attribute, use_beforematch, PanelHidden},
    collection::{collection_item, use_collection_provider, use_item, CollectionState},
    direction::{use_direction, Direction, HorizontalNav},
    merge_attributes, use_controlled, use_id_or, use_unique_id,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

#[derive(Clone, Copy)]
struct TabsContext {
    // State
    value: ReadSignal<String>,
    set_value: Callback<String>,
    disabled: ReadSignal<bool>,

    // Whether inactive panels stay mounted as `hidden="until-found"`
    hidden_until_found: ReadSignal<bool>,

    // Focus state
    focus: CollectionState,

    // Orientation
    horizontal: ReadSignal<bool>,

    // Text direction, for `ArrowLeft`/`ArrowRight`'s roving-focus role --
    // see `direction::Direction::resolve_horizontal`'s doc.
    direction: Direction,

    // ARIA attributes
    tab_content_ids: Signal<Vec<String>>,
}

/// The props for the [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    /// The controlled value of the active tab.
    pub value: ReadSignal<Option<String>>,

    /// The default active tab value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired when the active tab changes.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Whether the tabs are disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the tabs are horizontal.
    #[props(default)]
    pub horizontal: ReadSignal<bool>,

    /// Keep every inactive [`TabContent`] mounted and mark it `hidden="until-found"`, so the
    /// browser's find-in-page (Ctrl+F) and `#fragment` navigation can reach text inside inactive
    /// tabs. When they do, the browser reveals the panel and its tab becomes the active one
    /// (`on_value_change` is called with that tab's value).
    ///
    /// Defaults to false, which unmounts inactive panels (they render as an empty `hidden`
    /// element), so it costs nothing until opted in. Opting in mounts, and on a server-rendered
    /// page hydrates, the content of every tab: effects, timers and charts in inactive tabs all
    /// run. The content of inactive tabs is also in the server-rendered HTML.
    ///
    /// A stylesheet must not set `display` on an inactive panel: an author `display: none` defeats
    /// the browser rule that makes `until-found` content searchable.
    ///
    /// Browser support: Chrome 102+, Safari 26.2+ (does not scroll to the match) and Firefox 139+
    /// (148+ for a correct scroll target). Engines that do not know `until-found` treat it as plain
    /// `hidden`: inactive tabs stay unsearchable, as without the prop. Like `hidden`, inactive panels
    /// are not in the accessibility tree.
    ///
    /// Revealing is not blocked by `disabled`. The browser un-hides the panel itself, so a
    /// controlled `value` that ignores `on_value_change` leaves the panel revealed while another
    /// tab reports active: a controlled parent should honor it.
    #[props(default)]
    pub hidden_until_found: ReadSignal<bool>,

    /// The text direction for `ArrowLeft`/`ArrowRight` roving focus.
    /// Defaults to the nearest [`crate::direction::DirectionProvider`], or
    /// LTR if there is none. See [`crate::direction::use_direction`].
    #[props(default)]
    pub dir: Option<Direction>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the tabs element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tabs component.
    pub children: Element,
}

/// # Tabs
///
/// The `Tabs` component creates a tabbed interface that allows users to switch between different panels
/// of content. The [`TabTrigger`] component is used to switch between the different [`TabContent`]s.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::tabs::{TabContent, TabTrigger, Tabs, TabList};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tabs {
///             default_value: "tab1".to_string(),
///             horizontal: true,
///             TabList {
///                 TabTrigger {
///                     value: "tab1".to_string(),
///                     index: 0usize,
///                     "Tab 1"
///                 }
///                 TabTrigger {
///                     value: "tab2".to_string(),
///                     index: 1usize,
///                     "Tab 2"
///                 }
///             }
///             TabContent {
///                 index: 0usize,
///                 value: "tab1".to_string(),
///                 "Tab 1 Content"
///             }
///             TabContent {
///                 index: 1usize,
///                 value: "tab2".to_string(),
///                 "Tab 2 Content"
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`Tabs`] component defines the following data attributes you can use to control styling:
/// - `data-orientation`: Indicates the orientation of the tabs. Values are `horizontal` or `vertical`.
/// - `data-disabled`: Indicates if the tabs are disabled. Values are `true` or `false`.
/// - `data-direction`: The resolved text direction. Values are `ltr` or `rtl`.
#[component]
pub fn Tabs(props: TabsProps) -> Element {
    let (value, set_value) =
        use_controlled(props.value, props.default_value, props.on_value_change);
    let direction = use_direction(props.dir);

    let focus = use_collection_provider(props.roving_loop);
    let mut ctx = use_context_provider(|| TabsContext {
        value: value.into(),
        set_value,
        disabled: props.disabled,
        hidden_until_found: props.hidden_until_found,

        focus,

        horizontal: props.horizontal,
        direction,
        tab_content_ids: Signal::new(Vec::new()),
    });

    let owned = attributes!(div {
        "data-orientation": if (props.horizontal)() { "horizontal" } else { "vertical" },
        "data-disabled": (props.disabled)(),
        "data-direction": direction.as_str(),
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            dir: direction.as_str(),

            onfocusout: move |_| ctx.focus.clear_focus(),
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`TabList`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabListProps {
    /// Additional attributes to apply to the tab list element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tab list component.
    pub children: Element,
}

/// # TabList
///
/// The `TabList` component contains a list of [`TabTrigger`] components that allow users to switch between different tabs.
///
/// This must be used inside a [`Tabs`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::tabs::{TabContent, TabTrigger, Tabs, TabList};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tabs {
///             default_value: "tab1".to_string(),
///             horizontal: true,
///             TabList {
///                 TabTrigger {
///                     value: "tab1".to_string(),
///                     index: 0usize,
///                     "Tab 1"
///                 }
///                 TabTrigger {
///                     value: "tab2".to_string(),
///                     index: 1usize,
///                     "Tab 2"
///                 }
///             }
///             TabContent {
///                 index: 0usize,
///                 value: "tab1".to_string(),
///                 "Tab 1 Content"
///             }
///             TabContent {
///                 index: 1usize,
///                 value: "tab2".to_string(),
///                 "Tab 2 Content"
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn TabList(props: TabListProps) -> Element {
    let owned = attributes!(div { role: "tablist" });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`TabTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct TabTriggerProps {
    /// The value of the tab trigger, which is used to identify the corresponding tab content. This
    /// must match the `value` prop of the corresponding [`TabContent`].
    pub value: String,
    /// The index of the tab trigger. This is used to define the focus order for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Whether the tab trigger is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The ID of the tab trigger element.
    pub id: Option<String>,
    /// The class of the tab trigger element.
    pub class: Option<String>,

    /// Additional attributes to apply to the tab trigger element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the tab trigger component.
    pub children: Element,
}

/// # TabTrigger
///
/// The `TabTrigger` component is a button that switches to the [`TabContent`] with the same `value` when clicked.
///
/// This must be used inside a [`TabList`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::tabs::{TabContent, TabTrigger, Tabs, TabList};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tabs {
///             default_value: "tab1".to_string(),
///             horizontal: true,
///             TabList {
///                 TabTrigger {
///                     value: "tab1".to_string(),
///                     index: 0usize,
///                     "Tab 1"
///                 }
///                 TabTrigger {
///                     value: "tab2".to_string(),
///                     index: 1usize,
///                     "Tab 2"
///                 }
///             }
///             TabContent {
///                 index: 0usize,
///                 value: "tab1".to_string(),
///                 "Tab 1 Content"
///             }
///             TabContent {
///                 index: 1usize,
///                 value: "tab2".to_string(),
///                 "Tab 2 Content"
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`TabTrigger`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the state of the tab trigger. Values are `active` or `inactive`.
/// - `data-disabled`: Indicates if the tab trigger is disabled. Values are `true` or `false`.
#[component]
pub fn TabTrigger(props: TabTriggerProps) -> Element {
    let mut ctx: TabsContext = use_context();

    let value = props.value.clone();
    let selected = use_memo(move || (ctx.value)() == value);

    let disabled = move || (ctx.disabled)() || (props.disabled)();
    let item = use_item(
        collection_item(ctx.focus, props.index)
            .disabled(disabled)
            .selected(move || selected.cloned()),
    );
    let onmounted = item.onmounted();
    let tab_index = item.tabindex;

    // `type` is an overridable default; the rest is this trigger's own
    // functional state (role/tabindex define the tab widget and its
    // roving-focus wiring, aria_selected/data-state/data-disabled mirror
    // its real state, aria_controls references its `TabContent` by id).
    let defaults = attributes!(button { r#type: "button" });
    let owned = attributes!(button {
        role: "tab",
        tabindex: tab_index,
        aria_selected: selected,
        aria_controls: (ctx.tab_content_ids)().get((props.index)()).cloned(),
        "data-state": if selected() { "active" } else { "inactive" },
        "data-disabled": disabled(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        button {
            id: props.id,
            class: props.class,
            disabled: disabled(),

            onmounted,
            onclick: move |_| {
                let value = props.value.clone();
                if !selected() {
                    ctx.set_value.call(value);
                }
            },

            onfocus: move |_| ctx.focus.set_focus(Some((props.index)())),

            onkeydown: move |event: Event<KeyboardData>| {
                let key = event.key();
                let horizontal = (ctx.horizontal)();
                let mut prevent_default = true;
                match key {
                    Key::ArrowUp if !horizontal => ctx.focus.focus_prev(),
                    Key::ArrowDown if !horizontal => ctx.focus.focus_next(),
                    Key::ArrowLeft | Key::ArrowRight if horizontal => {
                        match ctx.direction.resolve_horizontal(&key) {
                            Some(HorizontalNav::Prev) => ctx.focus.focus_prev(),
                            Some(HorizontalNav::Next) => ctx.focus.focus_next(),
                            None => {}
                        }
                    }
                    Key::Home => ctx.focus.focus_first(),
                    Key::End => ctx.focus.focus_last(),
                    _ => prevent_default = false,
                };
                if prevent_default {
                    event.prevent_default();
                }
            },

            ..merged,
            {props.children}
        }
    }
}

/// The props for the [`TabContent`] component
#[derive(Props, Clone, PartialEq)]
pub struct TabContentProps {
    /// The value of the tab content, which must match the `value` prop of the corresponding [`TabTrigger`].
    pub value: String,

    /// The ID of the tab content element.
    pub id: ReadSignal<Option<String>>,
    /// The class of the tab content element.
    pub class: Option<String>,

    /// The index of the tab content. This is used to define the focus order for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Additional attributes to apply to the tab content element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = div)]
    pub attributes: Vec<Attribute>,

    /// The children of the tab content element.
    pub children: Element,
}

/// # TabContent
///
/// The content of a tab panel. This component will only be rendered when its corresponding [`TabTrigger`] is active,
/// unless [`TabsProps::hidden_until_found`] keeps it mounted as `hidden="until-found"`.
///
/// This should be used inside a [`Tabs`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::tabs::{TabContent, TabTrigger, Tabs, TabList};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tabs {
///             default_value: "tab1".to_string(),
///             horizontal: true,
///             TabList {
///                 TabTrigger {
///                     value: "tab1".to_string(),
///                     index: 0usize,
///                     "Tab 1"
///                 }
///                 TabTrigger {
///                     value: "tab2".to_string(),
///                     index: 1usize,
///                     "Tab 2"
///                 }
///             }
///             TabContent {
///                 index: 0usize,
///                 value: "tab1".to_string(),
///                 "Tab 1 Content"
///             }
///             TabContent {
///                 index: 1usize,
///                 value: "tab2".to_string(),
///                 "Tab 2 Content"
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`TabContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the state of the tab panel. Values are `active` or `inactive`.
///
/// An inactive panel carries the `hidden` attribute (`hidden="until-found"` with
/// [`TabsProps::hidden_until_found`]). A stylesheet that sets `display` on this element overrides the
/// browser's own `hidden` rule and must handle `[hidden]` itself.
#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    let mut ctx: TabsContext = use_context();
    let panel_value = props.value.clone();
    let selected = use_memo(move || (ctx.value)() == props.value);
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);
    let until_found = ctx.hidden_until_found;

    // The browser un-hides the panel itself; activate its tab to match.
    use_beforematch(id, until_found, move || {
        if !*selected.peek() {
            ctx.set_value.call(panel_value.clone());
        }
    });

    use_effect(move || {
        let mut tab_ids = ctx.tab_content_ids.write();
        let index = (props.index)();
        while tab_ids.len() <= index {
            tab_ids.push(String::new());
        }
        tab_ids[index] = id();
    });

    let hidden = match (selected(), until_found()) {
        (true, _) => PanelHidden::Shown,
        (false, true) => PanelHidden::UntilFound,
        (false, false) => PanelHidden::Hidden,
    };
    let mut owned = attributes!(div {
        role: "tabpanel",
        tabindex: "0",
        "data-state": if selected() { "active" } else { "inactive" },
    });
    owned.push(hidden_attribute(hidden));
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            id,
            class: props.class,
            ..merged,

            if selected() || until_found() {
                {props.children}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::find_elements;

    fn render(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// `(data-state, HIDDEN)` of every tab panel, in document order.
    fn panels(html: &str) -> Vec<(String, Option<String>)> {
        find_elements(html, |a| {
            a.get("role").map(String::as_str) == Some("tabpanel")
        })
        .into_iter()
        .map(|el| {
            (
                el.attrs["data-state"].clone(),
                el.attrs.get("HIDDEN").cloned(),
            )
        })
        .collect()
    }

    #[component]
    fn Plain() -> Element {
        rsx! {
            Tabs { default_value: "a".to_string(),
                TabContent { index: 0usize, value: "a".to_string(), "alpha text" }
                TabContent { index: 1usize, value: "b".to_string(), "beta text" }
            }
        }
    }

    #[component]
    fn UntilFound() -> Element {
        rsx! {
            Tabs { default_value: "a".to_string(), hidden_until_found: true,
                TabContent { index: 0usize, value: "a".to_string(), "alpha text" }
                TabContent { index: 1usize, value: "b".to_string(), "beta text" }
            }
        }
    }

    #[test]
    fn inactive_panel_is_empty_and_plain_hidden_by_default() {
        let html = render(Plain);
        assert!(
            html.contains("alpha text") && !html.contains("beta text"),
            "{html}"
        );
        assert_eq!(
            panels(&html),
            vec![
                ("active".to_string(), None),
                ("inactive".to_string(), Some(String::new())),
            ],
            "{html}"
        );
    }

    #[test]
    fn hidden_until_found_mounts_inactive_panels_as_until_found() {
        let html = render(UntilFound);
        assert!(
            html.contains("alpha text") && html.contains("beta text"),
            "{html}"
        );
        assert_eq!(
            panels(&html),
            vec![
                ("active".to_string(), None),
                ("inactive".to_string(), Some("until-found".to_string())),
            ],
            "{html}"
        );
    }
}
