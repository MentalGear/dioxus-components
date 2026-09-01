//! Shared state for the combobox component.

use crate::selectable::{OptionState, SelectableContext};
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

    fn predicate(&self) -> impl Fn(&OptionState) -> bool {
        self.predicate_for(self.query.cloned())
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

    pub fn open_with_empty_query_and_focus_first(&mut self) {
        let query = String::new();
        self.set_query.call(query.clone());
        let initial_focus = self
            .selectable
            .first_matching_enabled_index(self.predicate_for(query));
        self.selectable.initial_focus.set(initial_focus);
        self.set_open(true);
    }

    pub fn open_with_empty_query_and_focus_last(&mut self) {
        let query = String::new();
        self.set_query.call(query.clone());
        let initial_focus = self
            .selectable
            .last_matching_enabled_index(self.predicate_for(query));
        self.selectable.initial_focus.set(initial_focus);
        self.set_open(true);
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
