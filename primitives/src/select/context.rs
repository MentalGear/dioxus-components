//! Context types and implementations for the select component.

use dioxus::prelude::*;
use dioxus_core::Task;
use dioxus_sdk_time::sleep;

use std::time::Duration;

use super::text_search::AdaptiveKeyboard;
use crate::selectable::SelectableContext;

/// Main context for the select component containing all shared state
#[derive(Clone, Copy)]
pub(super) struct SelectContext {
    /// Shared selectable listbox state.
    pub selectable: SelectableContext,
    /// Adaptive keyboard system for multi-language support
    pub adaptive_keyboard: Signal<AdaptiveKeyboard>,
    /// The typeahead buffer for searching options
    pub typeahead_buffer: Signal<String>,
    /// The ID of the list for ARIA attributes
    pub typeahead_clear_task: Signal<Option<Task>>,
    /// Timeout before clearing typeahead buffer
    pub typeahead_timeout: ReadSignal<Duration>,

    /// Set for the duration of an Alt+ArrowDown-opened popup (APG select-
    /// only combobox, Optional: "Alt + Down Arrow: ... displays the popup
    /// without moving focus"), so `SelectListRendered`'s own "nothing is
    /// focused yet -- focus the listbox container" fallback (`list.rs`)
    /// does not steal DOM focus off the trigger for this one open path.
    /// Reset to `false` whenever the popup closes (`select.rs`), so it can
    /// never leak into the next, differently-triggered open.
    pub keep_trigger_focus: Signal<bool>,
}

impl SelectContext {
    pub fn set_open(&mut self, open: bool) {
        self.selectable.set_open(open);
    }

    /// The path Enter and Space on `SelectTrigger` both route through: open
    /// the popup and request focus on the currently-selected option, or the
    /// first available one if nothing is selected yet. APG select-only
    /// combobox: "focus the listbox with the current option active" --
    /// unlike a plain click open (unchanged; leaves DOM focus on the
    /// listbox container itself, matching this component's pre-existing,
    /// still-green `select.spec.ts` expectation), keyboard activation must
    /// land real DOM focus on an option, same as ArrowDown/ArrowUp already
    /// do just below this in `trigger.rs`.
    pub fn open_with_selected_or_first_focus(&mut self) {
        self.set_open(true);
        let target = self
            .selectable
            .collection
            .selected_available_index()
            .or_else(|| self.selectable.collection.first_available_index());
        self.selectable.initial_focus.set(target);
    }

    pub fn multi(&self) -> bool {
        self.selectable.selection_mode.is_multiple()
    }

    /// Select the currently focused item
    pub fn select_current_item(&mut self) {
        self.selectable.select_focused();
    }

    /// Learn from a keyboard event mapping physical key to logical character
    pub fn learn_from_keyboard_event(&mut self, physical_code: &str, logical_char: char) {
        let mut adaptive = self.adaptive_keyboard.write();
        let logical_char = logical_char.to_lowercase().next().unwrap_or(logical_char);
        adaptive.learn_from_event(physical_code, logical_char);
    }

    /// Add text to the typeahead buffer for searching
    pub fn add_to_typeahead_buffer(&mut self, text: &str) {
        // Cancel any existing clear task to prevent race conditions
        if let Some(existing_task) = self.typeahead_clear_task.write().take() {
            existing_task.cancel();
        }

        // Update the buffer and get the current content
        let typeahead = {
            let mut typeahead_buffer = self.typeahead_buffer.write();
            typeahead_buffer.push_str(text);
            typeahead_buffer.clone()
        };

        // Create references for the async closure
        let mut typeahead_buffer_signal = self.typeahead_buffer;
        let mut typeahead_clear_task_signal = self.typeahead_clear_task;

        // Spawn a new task to clear the buffer after the configured timeout
        let timeout = self.typeahead_timeout.cloned();
        let new_task = spawn(async move {
            sleep(timeout).await;

            // Clear the buffer
            typeahead_buffer_signal.write().clear();

            // Remove our own task handle to indicate no task is active
            typeahead_clear_task_signal.write().take();
        });

        // Store the new task handle
        self.typeahead_clear_task.write().replace(new_task);

        // Focus the best match using adaptive keyboard
        let options = self.selectable.options.read();
        let keyboard = self.adaptive_keyboard.read();

        if let Some(best_match_index) =
            super::text_search::best_match(&keyboard, &typeahead, &options, |index| {
                self.selectable.collection.is_available(index)
            })
        {
            self.selectable.collection.set_focus(Some(best_match_index));
        }
    }
}

/// Context for select group components
#[derive(Clone, Copy)]
pub(super) struct SelectGroupContext {
    /// ID of the element that labels this group
    pub labeled_by: Signal<Option<String>>,
}
