//! Shared state for the combobox component.

use crate::selectable::{OptionState, RcPartialEqValue, SelectableContext};
use dioxus::prelude::*;

/// The default case-insensitive substring filter.
pub fn default_combobox_filter(query: &str, text: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || text.to_lowercase().contains(&query)
}

#[derive(Clone, Copy)]
pub(super) struct ComboboxContext {
    pub selectable: SelectableContext,
    pub query: Memo<String>,
    pub set_query: Callback<String>,
    pub filter: Callback<(String, String), bool>,

    /// The current `ComboboxInput`'s own element id, kept in sync by that
    /// component -- mirrors
    /// `DropdownMenuContext::content_id` (`dropdown_menu.rs`), except this
    /// one holds the *trigger* side's id rather than the content side's:
    /// `ComboboxList`'s listbox anchors to the input (there is no separate
    /// trigger button here), and the input's id -- unlike `SelectTrigger`'s
    /// already-stable `trigger_id` -- is generated locally by
    /// `ComboboxInput` itself (`use_id_or`/`use_unique_id`), so it needs
    /// this same sync-back to be visible to the listbox's anchor-name
    /// wiring. See `PopoverCtx::content_id`'s doc in `popover.rs` for the
    /// exact bug this guards against if the two ever named different ids.
    pub input_id: Signal<String>,

    /// Whether the user has edited the input's text since the popup opened.
    ///
    /// This is what separates *engaging* the input from *querying* with it.
    /// Opening the popup (a click, ArrowDown/ArrowUp, or a controlled `open`)
    /// must leave the input's text alone -- it is the selected option's label
    /// -- and list every option; only the first edit turns that text into a
    /// query (`input_text`, `filter_query`). Reset by [`Self::open_session`]
    /// for every user-initiated open and by `ComboboxList` once the popup has
    /// finished closing (not at close: the closing list keeps its filter
    /// through its exit animation).
    pub edited: Signal<bool>,

    /// The last label seen for the selected value -- see [`Self::selected_label`].
    pub label: Signal<Option<(RcPartialEqValue, String)>>,
}

impl ComboboxContext {
    pub fn set_open(&mut self, open: bool) {
        if open {
            self.selectable.collection.clear_focus();
        }
        self.selectable.set_open(open);
    }

    fn predicate_for(&self, query: String) -> impl Fn(&OptionState) -> bool {
        let filter = self.filter;
        move |option| filter.call((query.clone(), option.text_value.clone()))
    }

    /// The selected option's label.
    ///
    /// Read from the option registry, which is briefly empty every time the
    /// list mounts or unmounts: `ComboboxList` renders its options in two
    /// different places (inline while closed, inside the popup while open), so
    /// each one's cleanup removes it before its replacement has registered.
    /// For that moment the input would flash empty -- and a write to a text
    /// input's `value` throws the caret to the end, which is exactly the click
    /// position this component promises to keep. So fall back to the last label
    /// seen for the *same* value ([`Self::remember_label`]).
    fn selected_label(&self) -> Option<String> {
        if let Some(label) = self.selectable.selected_text() {
            return Some(label);
        }
        let values = self.selectable.values.read();
        let cached = self.label.read();
        (*cached)
            .as_ref()
            .filter(|(value, _)| values.iter().any(|selected| selected == value))
            .map(|(_, label)| label.clone())
    }

    /// Record the selected value's label while the registry can supply it.
    pub fn remember_label(&mut self) {
        let value = self.selectable.values.read().first().cloned();
        let (Some(value), Some(label)) = (value, self.selectable.selected_text()) else {
            return;
        };
        let unchanged = (*self.label.peek())
            .as_ref()
            .is_some_and(|(seen, text)| *seen == value && *text == label);
        if !unchanged {
            self.label.set(Some((value, label)));
        }
    }

    /// The text the options are filtered by: the query once the user has
    /// edited the input (or when there is no selected label to show), and
    /// otherwise nothing, so a popup opened over a selection lists every option
    /// instead of only the one matching its own label.
    fn filter_query(&self) -> String {
        if *self.edited.read() || self.selected_label().is_none() {
            self.query.cloned()
        } else {
            String::new()
        }
    }

    fn predicate(&self) -> impl Fn(&OptionState) -> bool {
        self.predicate_for(self.filter_query())
    }

    /// What the input shows. The user's query while they are editing it; the
    /// selected option's label otherwise -- including while the popup is open
    /// and untouched (clicking or focusing must not change the text), and the
    /// instant it closes (Escape, blur or a pick restore the label).
    pub fn input_text(&self) -> String {
        let open = (self.selectable.open)();
        match self.selected_label() {
            Some(label) if !(open && *self.edited.read()) => label,
            _ if open => self.query.cloned(),
            _ => String::new(),
        }
    }

    pub fn is_visible(&self, tab_index: usize) -> bool {
        let predicate = self.predicate();
        self.selectable
            .options
            .read()
            .iter()
            .find(|option| option.index == tab_index)
            .is_some_and(predicate)
    }

    pub fn has_visible_options(&self) -> bool {
        self.selectable.options.read().iter().any(self.predicate())
    }

    /// Start a fresh editing session: no edit yet, no query.
    fn begin_session(&mut self) {
        if *self.edited.peek() {
            self.edited.set(false);
        }
        if !self.query.peek().is_empty() {
            self.set_query.call(String::new());
        }
    }

    /// The registration index of the selected option, if any. Read from the
    /// option registry rather than the collection: a stale filter from the
    /// previous session may still have the selected option marked hidden
    /// there for a moment after a reopen.
    fn selected_index(&self) -> Option<usize> {
        let options = self.selectable.options.read();
        options
            .iter()
            .filter(|option| self.selectable.is_selected(&option.value))
            .map(|option| option.index)
            .min()
    }

    fn open_session(&mut self, initial_focus: Option<usize>) {
        self.begin_session();
        self.selectable.initial_focus.set(initial_focus);
        self.set_open(true);
    }

    /// Open on a click: the selected option (if any) becomes the active one.
    pub fn open_at_selected(&mut self) {
        let selected = self.selected_index();
        self.open_session(selected);
    }

    /// Open on ArrowDown: the selected option, else the first.
    pub fn open_at_selected_or_first(&mut self) {
        let first = self
            .selectable
            .first_matching_enabled_index(self.predicate_for(String::new()));
        let target = self.selected_index().or(first);
        self.open_session(target);
    }

    /// Open on ArrowUp: the selected option, else the last.
    pub fn open_at_selected_or_last(&mut self) {
        let last = self
            .selectable
            .last_matching_enabled_index(self.predicate_for(String::new()));
        let target = self.selected_index().or(last);
        self.open_session(target);
    }

    pub fn focused_option_id(&self) -> Option<String> {
        self.selectable.focused_option_id()
    }

    pub fn focus_next_visible(&mut self) {
        self.selectable.focus_next_where(self.predicate());
    }

    pub fn focus_prev_visible(&mut self) {
        self.selectable.focus_prev_where(self.predicate());
    }

    pub fn focus_first_visible(&mut self) {
        self.selectable.focus_first_where(self.predicate());
    }

    pub fn focus_last_visible(&mut self) {
        self.selectable.focus_last_where(self.predicate());
    }

    pub fn select_focused(&mut self) {
        self.selectable.select_focused();
    }
}
