//! Combobox option components.

use dioxus::prelude::*;
use dioxus_attributes::attributes;

use super::super::context::ComboboxContext;
use crate::{
    collection::{collection_item, use_item},
    listbox::{ListboxContext, ListboxItemIndicator},
    merge_attributes,
    selectable::{
        pointer_select_cancel, pointer_select_commit, pointer_select_start, use_selectable_option,
        RcPartialEqValue, SelectableOptionConfig,
    },
};

/// Scrolls the option with this id into view inside its listbox -- the minimum
/// distance, like `scrollIntoView({ block: "nearest" })`, but only ever the
/// listbox itself (never the page) and measured in layout space: the popup's
/// open animation scales it, which would skew raw bounding-box deltas.
///
/// An active-descendant combobox keeps DOM focus on its input, so nothing
/// scrolls the highlighted option for us the way real focus would. The popup
/// may not be laid out yet when a highlight lands (it is shown a frame or two
/// after `open`), hence the short frame-by-frame retry. An option under the
/// pointer is left alone: a hover highlight must not scroll the list the
/// pointer is itself scrolling.
const SCROLL_OPTION_INTO_VIEW_JS: &str = r#"
const id = await dioxus.recv();
const place = () => {
  const el = document.getElementById(id);
  const list = el && el.closest('[role="listbox"]');
  if (!list || list.clientHeight === 0) return false;
  if (el.matches(':hover')) return true;
  const lr = list.getBoundingClientRect();
  const k = list.offsetHeight ? lr.height / list.offsetHeight : 1;
  const er = el.getBoundingClientRect();
  const style = getComputedStyle(list);
  const padTop = parseFloat(style.paddingTop) || 0;
  const padBottom = parseFloat(style.paddingBottom) || 0;
  const top = (er.top - lr.top) / k - list.clientTop + list.scrollTop;
  const bottom = top + er.height / k;
  if (top - padTop < list.scrollTop) list.scrollTop = Math.max(0, top - padTop);
  else if (bottom + padBottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom + padBottom - list.clientHeight;
  return true;
};
let tries = 0;
const step = () => { if (!place() && ++tries < 10) requestAnimationFrame(step); };
requestAnimationFrame(step);
"#;

/// Props for [`ComboboxOption`].
#[derive(Props, Clone, PartialEq)]
pub struct ComboboxOptionProps<T: Clone + PartialEq + 'static> {
    /// The value carried by this option.
    pub value: ReadSignal<T>,

    /// Display/searchable text. Required for non-string types.
    #[props(default)]
    pub text_value: ReadSignal<Option<String>>,

    /// Whether the option is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Optional id for the option element.
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Registration order used for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Optional aria-label.
    #[props(default)]
    pub aria_label: Option<String>,

    /// Optional aria-roledescription.
    #[props(default)]
    pub aria_roledescription: Option<String>,

    /// Additional attributes.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Children rendered inside the option.
    pub children: Element,
}

/// A selectable option inside a [`ComboboxList`](super::list::ComboboxList).
#[component]
pub fn ComboboxOption<T: PartialEq + Clone + 'static>(props: ComboboxOptionProps<T>) -> Element {
    let index = props.index;

    let mut ctx: ComboboxContext = use_context();
    let visible = move || ctx.is_visible(index());
    let option = use_selectable_option(
        ctx.selectable,
        SelectableOptionConfig {
            id: props.id,
            index,
            value: props.value,
            text_value: props.text_value,
            option_disabled: props.disabled,
            component_name: "ComboboxOption",
        },
    );
    use_item(
        collection_item(ctx.selectable.collection, index)
            .key(move || Some(option.id.cloned()))
            .disabled(move || option.disabled.cloned())
            .hidden(move || !visible())
            .selected(move || (option.selected)()),
    );

    let render = use_context::<ListboxContext>().render;

    // Keep the active option in view: on open (the selected option is
    // highlighted) and as arrow keys move the highlight.
    use_effect(move || {
        if render() && (option.focused)() {
            let eval = document::eval(SCROLL_OPTION_INTO_VIEW_JS);
            let _ = eval.send(option.id.cloned());
        }
    });

    // All owned: role/aria-selected/aria-disabled define this option's
    // widget semantics, the `data-*` pair mirrors its own state.
    let owned = attributes!(div {
        role: "option",
        aria_selected: (option.selected)(),
        aria_disabled: (option.disabled)(),
        "data-highlighted": (option.focused)(),
        "data-disabled": (option.disabled)(),
        "data-selected": (option.selected)(),
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        if render() && visible() {
            div {
                id: option.id,
                aria_label: props.aria_label.clone(),
                aria_roledescription: props.aria_roledescription.clone(),

                onmouseenter: move |_| {
                    if !(option.disabled)() {
                        ctx.selectable.collection.set_focus(Some((option.index)()));
                    }
                },
                onpointerdown: move |event| {
                    pointer_select_start(&event, (option.disabled)(), option.down_pos);
                },
                onpointerup: move |event| {
                    if pointer_select_commit(&event, (option.disabled)(), option.down_pos) {
                        ctx.selectable.select_value(RcPartialEqValue::new(option.value.cloned()));
                    }
                },
                onpointercancel: move |_| {
                    pointer_select_cancel(option.down_pos);
                },

                ..merged,
                {props.children}
            }
        }
    }
}

/// Props for [`ComboboxItemIndicator`].
#[derive(Props, Clone, PartialEq)]
pub struct ComboboxItemIndicatorProps {
    /// Children rendered only when the parent option is selected.
    pub children: Element,
}

/// Renders its children when the parent option is selected.
#[component]
pub fn ComboboxItemIndicator(props: ComboboxItemIndicatorProps) -> Element {
    rsx! {
        ListboxItemIndicator {
            {props.children}
        }
    }
}
