//! Defines the [`Collapsible`] component and its sub-components.

use crate::dioxus_core::AttributeValue;
use crate::{merge_attributes, use_controlled, use_effect_with_cleanup, use_id_or, use_unique_id};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

// TODO: more docs

/// How a panel that can opt into `hidden="until-found"` is hidden right now.
///
/// Shared by [`Collapsible`], [`crate::accordion::Accordion`] and
/// [`crate::tabs::Tabs`], so all three render the attribute the same way.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PanelHidden {
    /// Not hidden: no `hidden` attribute at all.
    Shown,
    /// Plain `hidden` (`display: none`): not rendered, not searchable, not revealable.
    Hidden,
    /// `hidden="until-found"`: skipped by layout and painting like `display: none`, but still
    /// searchable by find-in-page and reachable by `#fragment` navigation, both of which reveal
    /// it and fire `beforematch` first. Browsers without support treat it as plain `hidden`.
    UntilFound,
}

/// The attribute that renders a [`PanelHidden`] state.
///
/// It is spelled `HIDDEN`, upper case, on purpose, and every state of every opt-in panel goes
/// through this one slot. Both halves of that are load-bearing:
///
/// - Dioxus' web interpreter special-cases the *name* `hidden` as a boolean attribute and
///   removes it whenever the value is not literally `true` (`setAttributeDefault` in
///   `dioxus-interpreter-js`), so `hidden: "until-found"` is rendered by the server and then
///   *deleted* the first time the client patches it: the closed content would be visible. HTML
///   attribute names are ASCII case-insensitive (`setAttribute("HIDDEN", ..)` and
///   `removeAttribute("HIDDEN")` both address `hidden`, and the parser lower-cases it from the
///   server's markup), but the interpreter's switch is case-sensitive, so the upper-case name
///   takes the plain `setAttribute` path.
/// - Dioxus diffs an element's attributes as a list sorted by name, and an upper-case spelling
///   sorts *before* `hidden`. Alternating between a `hidden` slot and a `HIDDEN` slot would
///   write the new attribute and then remove the old one, which is the same DOM attribute. One
///   slot for every state means no transition can do that.
pub(crate) fn hidden_attribute(state: PanelHidden) -> Attribute {
    let value = match state {
        PanelHidden::Shown => AttributeValue::None,
        PanelHidden::Hidden => AttributeValue::Text(String::new()),
        PanelHidden::UntilFound => AttributeValue::Text("until-found".to_string()),
    };
    Attribute::new("HIDDEN", value, None, false)
}

/// Run `on_reveal` when the browser reveals the element with the given `id` because find-in-page
/// or `#fragment` navigation matched content inside it (`beforematch`).
///
/// The browser removes `hidden="until-found"` itself, right after the event, so the component's
/// open state has to follow it: `on_reveal` is where the caller opens the panel. Dioxus has no
/// typed `onbeforematch`, so this is an `eval` bridge with the same shape as the other listener
/// hooks (`use_global_keydown_listener`, `use_dialog_close_sync`): the listener is registered
/// from an effect, and the effect's cleanup tears it down when the component unmounts or any
/// input changes, so nothing outlives the element.
///
/// Only elements the event is dispatched *at* count (`event.target === element`): `beforematch`
/// bubbles, so a nested panel's reveal would otherwise also open every panel around it twice.
///
/// Listens only while `enabled` is true; effects never run on the server, so SSR is unaffected.
pub(crate) fn use_beforematch(
    id: impl Readable<Target = String> + Copy + 'static,
    enabled: impl Readable<Target = bool> + Copy + 'static,
    on_reveal: impl FnMut() + Clone + 'static,
) {
    use_effect_with_cleanup(move || -> Box<dyn FnOnce()> {
        if !enabled.cloned() {
            return Box::new(|| {});
        }
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            const element = document.getElementById(id);
            if (element) {
                const onBeforeMatch = (event) => {
                    if (event.target === element) dioxus.send(true);
                };
                element.addEventListener('beforematch', onBeforeMatch);
                await dioxus.recv();
                element.removeEventListener('beforematch', onBeforeMatch);
            }",
        );
        let _ = eval.send(id.cloned());
        let mut on_reveal = on_reveal.clone();
        spawn(async move {
            while let Ok(true) = eval.recv::<bool>().await {
                on_reveal();
            }
        });
        Box::new(move || {
            let _ = eval.send(true);
        })
    });
}

#[derive(Clone, Copy)]
struct CollapsibleCtx {
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,
    keep_mounted: ReadSignal<bool>,
    hidden_until_found: ReadSignal<bool>,
    aria_controls_id: Signal<String>,
}

/// The props for the [`Collapsible`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleProps {
    /// Keep [`CollapsibleContent`] mounted in the DOM when the collapsible is closed.
    ///
    /// While closed the content element carries the plain `hidden` attribute, so it is not
    /// displayed, not searchable and not in the accessibility tree.
    #[props(default)]
    pub keep_mounted: ReadSignal<bool>,

    /// Keep [`CollapsibleContent`] mounted while closed and mark it `hidden="until-found"`, so the
    /// browser's find-in-page (Ctrl+F) and `#fragment` navigation can reach text inside it. When
    /// they do, the browser reveals the content and the collapsible opens (`on_open_change` is
    /// called with `true`).
    ///
    /// Overrides [`keep_mounted`](Self::keep_mounted). Defaults to false, which unmounts closed
    /// content, so it costs nothing until opted in; opting in renders the content into the
    /// server-rendered HTML as well.
    ///
    /// Browser support: Chrome 102+, Safari 26.2+ (does not scroll to the match) and Firefox 139+
    /// (148+ for a correct scroll target). Engines that do not know `until-found` treat it as plain
    /// `hidden`: the content stays closed and unsearchable, as without the prop. Like `hidden`, the
    /// content is not in the accessibility tree while closed.
    ///
    /// Revealing is not blocked by `disabled`. The browser un-hides the content itself, so a
    /// controlled `open` that ignores `on_open_change(true)` leaves the content revealed while the
    /// collapsible reports closed: a controlled parent should honor it.
    #[props(default)]
    pub hidden_until_found: ReadSignal<bool>,

    /// The default `open` state.
    ///
    /// This will be overridden if the component is controlled.
    #[props(default)]
    pub default_open: bool,

    /// The disabled state of the collapsible.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The controlled `open` state of the collapsible.
    ///
    /// If this is provided, you must use `on_open_change`.
    pub open: ReadSignal<Option<bool>>,

    /// A callback for when the open state changes.
    ///
    /// The provided argument is a bool of whether the collapsible is open or closed.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Render the root element as a custom component/element.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// Additional attributes for the collapsible element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the collapsible component.
    pub children: Element,
}

/// # Collapsible
///
/// The [`Collapsible`] component is a container that can be expanded or collapsed to show or hide its content.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Collapsible {
///             CollapsibleTrigger {
///                 b { "Recent Activity" }
///             }
///             CollapsibleContent {
///                 div {
///                     "Fixed a bug in the collapsible component",
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`Collapsible`] component defines the following data attributes you can use to control styling:
/// - `data-open`: Indicates if the collapsible is open. Values are `true` or `false`.
/// - `data-disabled`: Indicates if the collapsible is disabled. values are `true` or `false`.
#[component]
pub fn Collapsible(props: CollapsibleProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);

    let aria_controls_id = use_unique_id();
    use_context_provider(|| CollapsibleCtx {
        open,
        set_open,
        disabled: props.disabled,
        keep_mounted: props.keep_mounted,
        hidden_until_found: props.hidden_until_found,
        aria_controls_id,
    });

    let base = attributes!(div {
        "data-open": open,
        "data-disabled": props.disabled,
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    if let Some(dynamic) = props.r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            div {
                ..merged,
                {props.children}
            }
        }
    }
}

/// The props for the [`CollapsibleContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleContentProps {
    /// The ID of the collapsible content element.
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes for the collapsible content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the collapsible content.
    pub children: Element,
}

/// # CollapsibleContent
///
/// The [`CollapsibleContent`] component defines the content of a collapsible section. The
/// contents will only be rendered if the collapsible is open, or if the [`CollapsibleProps::keep_mounted`]
/// or [`CollapsibleProps::hidden_until_found`] prop is set to `true`.
///
/// This must be used inside a [`Collapsible`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Collapsible {
///             CollapsibleTrigger {
///                 b { "Recent Activity" }
///             }
///             CollapsibleContent {
///                 div {
///                     "Fixed a bug in the collapsible component",
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`CollapsibleContent`] component defines the following data attributes you can use to control styling:
/// - `data-open`: Indicates if the collapsible is open. Values are `true` or `false`.
/// - `data-disabled`: Indicates if the collapsible is disabled. values are `true` or `false`.
///
/// Closed content that stays mounted (`keep_mounted`, `hidden_until_found`) carries the `hidden`
/// attribute (`hidden="until-found"` for the latter). A stylesheet that sets `display` on this
/// element overrides the browser's own `hidden` rule and must handle `[hidden]` itself.
#[component]
pub fn CollapsibleContent(props: CollapsibleContentProps) -> Element {
    let ctx: CollapsibleCtx = use_context();
    let id = use_id_or(ctx.aria_controls_id, props.id);

    let open = ctx.open;
    let until_found = ctx.hidden_until_found;

    // `hidden_until_found` wins over `keep_mounted`: both keep the children mounted, and the
    // former is the one that also hides them in a searchable way.
    let hidden = match (open(), until_found(), (ctx.keep_mounted)()) {
        (true, _, _) => PanelHidden::Shown,
        (false, true, _) => PanelHidden::UntilFound,
        (false, false, true) => PanelHidden::Hidden,
        // Unmounted: there is nothing to hide, only an empty wrapper.
        (false, false, false) => PanelHidden::Shown,
    };

    // The browser un-hides the content itself; open the collapsible to match.
    use_beforematch(id, until_found, move || {
        if !*open.peek() {
            ctx.set_open.call(true);
        }
    });

    let mut owned = attributes!(div {
        "data-open": open,
        "data-disabled": ctx.disabled,
    });
    owned.push(hidden_attribute(hidden));
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            id: id,
            ..merged,

            if open() || (ctx.keep_mounted)() || until_found() {
                {props.children}
            }
        }
    }
}

/// The props for the [`CollapsibleTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CollapsibleTriggerProps {
    /// Render the trigger element as a custom component/element.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// Additional attributes for the collapsible trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the collapsible trigger.
    pub children: Element,
}

/// # CollapsibleTrigger
///
/// The [`CollapsibleTrigger`] component is the button or element that toggles the visibility of the collapsible content.
///
/// This must be used inside a [`Collapsible`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Collapsible {
///             CollapsibleTrigger {
///                 b { "Recent Activity" }
///             }
///             CollapsibleContent {
///                 div {
///                     "Fixed a bug in the collapsible component",
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`CollapsibleTrigger`] component defines the following data attributes you can use to control styling:
/// - `data-open`: Indicates if the collapsible is open. Values are `true` or `false`.
/// - `data-disabled`: Indicates if the collapsible is disabled. values are `true` or `false`.
#[component]
pub fn CollapsibleTrigger(props: CollapsibleTriggerProps) -> Element {
    let ctx: CollapsibleCtx = use_context();

    let open = ctx.open;

    let base = attributes!(button {
        r#type: "button",
        "data-open": open,
        "data-disabled": ctx.disabled,
        disabled: ctx.disabled,
        "aria-controls": ctx.aria_controls_id,
        "aria-expanded": open,
        onclick: move |_| {
            let new_open = !open();
            ctx.set_open.call(new_open);
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    if let Some(dynamic) = props.r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            button {
                ..merged,
                {props.children}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::find_element;

    fn render(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// The `hidden` slot of the content element (`None`: no such attribute).
    fn content_hidden(html: &str) -> Option<String> {
        let content = find_element(html, |a| {
            a.contains_key("id") && a.contains_key("data-open")
        })
        .expect("the content element renders");
        content.attrs.get("HIDDEN").cloned()
    }

    #[component]
    fn Closed() -> Element {
        rsx! {
            Collapsible {
                CollapsibleContent { "closed text" }
            }
        }
    }

    #[component]
    fn KeepMounted() -> Element {
        rsx! {
            Collapsible { keep_mounted: true,
                CollapsibleContent { "closed text" }
            }
        }
    }

    #[component]
    fn UntilFound() -> Element {
        rsx! {
            Collapsible { hidden_until_found: true,
                CollapsibleContent { "closed text" }
            }
        }
    }

    #[component]
    fn UntilFoundOverridesKeepMounted() -> Element {
        rsx! {
            Collapsible { keep_mounted: true, hidden_until_found: true,
                CollapsibleContent { "closed text" }
            }
        }
    }

    #[component]
    fn UntilFoundOpen() -> Element {
        rsx! {
            Collapsible { hidden_until_found: true, default_open: true,
                CollapsibleContent { "open text" }
            }
        }
    }

    #[test]
    fn closed_content_is_unmounted_by_default() {
        let html = render(Closed);
        assert!(!html.contains("closed text"), "{html}");
        assert_eq!(content_hidden(&html), None, "{html}");
    }

    #[test]
    fn keep_mounted_hides_closed_content_with_plain_hidden() {
        // Before the fix this rendered the text with no `hidden` at all, so a styled
        // layer (or no layer) showed closed content.
        let html = render(KeepMounted);
        assert!(html.contains("closed text"), "{html}");
        assert_eq!(content_hidden(&html), Some(String::new()), "{html}");
    }

    #[test]
    fn hidden_until_found_keeps_closed_content_mounted_and_searchable() {
        let html = render(UntilFound);
        assert!(html.contains("closed text"), "{html}");
        assert_eq!(
            content_hidden(&html).as_deref(),
            Some("until-found"),
            "{html}"
        );
    }

    #[test]
    fn hidden_until_found_wins_over_keep_mounted() {
        let html = render(UntilFoundOverridesKeepMounted);
        assert_eq!(
            content_hidden(&html).as_deref(),
            Some("until-found"),
            "{html}"
        );
    }

    #[test]
    fn open_content_is_never_hidden() {
        let html = render(UntilFoundOpen);
        assert!(html.contains("open text"), "{html}");
        assert_eq!(content_hidden(&html), None, "{html}");
    }

    #[test]
    fn hidden_attribute_uses_one_slot_for_every_state() {
        // Dioxus' web interpreter removes any attribute *named* `hidden` whose value is not
        // `true` and diffs attributes sorted by name, so every state must use the same
        // upper-case slot (see `hidden_attribute`).
        for state in [
            PanelHidden::Shown,
            PanelHidden::Hidden,
            PanelHidden::UntilFound,
        ] {
            assert_eq!(hidden_attribute(state).name, "HIDDEN");
        }
    }
}
