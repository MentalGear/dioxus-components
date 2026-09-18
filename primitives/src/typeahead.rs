//! Typeahead ("type letters to jump to a menu item") for the APG "Menu and
//! Menubar" pattern's menu family: [`crate::dropdown_menu`],
//! [`crate::context_menu`] and [`crate::menubar`] (and, where the
//! per-submenu collection already makes it free, their own
//! [`crate::menu_sub`] submenus). `docs/backlog.md` row 11, `docs/plan.md`
//! Phase 6.
//!
//! # Rule source
//!
//! W3C WAI-ARIA Authoring Practices, "Menu and Menubar" pattern, "Keyboard
//! Interaction" (h2), pinned commit
//! `7e4034b262bc0d25332e330d8a582aaf34113829` of `w3c/aria-practices` (see
//! `playwright/oracle/reference/README.md`):
//!
//! > Any key that corresponds to a printable character (Optional): Move
//! > focus to the next item in the current menu whose label begins with
//! > that printable character.
//!
//! # Not `select/`'s typeahead
//!
//! `select/text_search.rs`'s Levenshtein, keyboard-layout-aware matcher
//! (driven by `select/context.rs::SelectContext::add_to_typeahead_buffer`)
//! is deliberately kept for `Select` -- `docs/recommended-
//! implementations.md` §8 already settled the best-of comparison in
//! upstream's favor there ("Best-of here means recognising that upstream
//! already wins one"). This module only reuses that file's *buffer/timeout
//! idiom* -- a `Signal<String>` buffer plus a self-cancelling
//! `Signal<Option<Task>>` clear-after-timeout task, backed by
//! `dioxus_sdk_time::sleep` -- never its matcher: dignifiedquire's much
//! smaller `typeahead.rs` (78 lines, plain prefix matching) is the concept
//! ported here instead, per `docs/recommended-implementations.md` §8's own
//! citation.
//!
//! # Construction
//!
//! [`find_next_match`] is the pure matcher: given every candidate item's
//! `(index, label, disabled)`, the currently focused index (if any), and
//! the accumulated search buffer, it decides which item's index (if any)
//! roving focus should move to. It knows nothing about Dioxus, collections,
//! or keyboard events, and is exercised directly by this module's own
//! `#[cfg(test)]` unit tests below -- the exact Radix-parity cases (wrap,
//! skip-disabled, same-character cycling, case-insensitivity) this
//! function was built against.
//!
//! [`TypeaheadState`]/[`TypeaheadState::handle_key`] is the glue: it owns
//! the live buffer/timer (mirroring `select/context.rs`'s idiom, see
//! above), decides whether a keydown event even qualifies (a single,
//! unmodified, non-space printable character -- never Ctrl/Meta/Alt, never
//! Space, which every call site's own `onkeydown` already routes to item
//! activation before this ever runs), and -- on a match -- actually moves
//! roving focus via [`CollectionState::set_focus`], the same primitive
//! every host's own Arrow/Home/End handling already uses.
//!
//! Item labels come from [`CollectionState::text_entries`], an additive
//! read of the *same* per-item registration `use_item()` already performs
//! for `key`/`disabled`/`hidden`/`selected` (`collection.rs`) -- not a
//! second, parallel item registry that could drift from it. Each host's own
//! `*Item`/`*SubItem`/`*SubTrigger` component resolves its own label
//! (preferring an explicit `text_value` prop, falling back to its own
//! `value` where that is meaningful) and registers the resolved string
//! through the same `collection_item(...)` builder chain it already uses
//! for `disabled`/etc. -- see each component's own doc for its exact
//! fallback.

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_core::Task;
use dioxus_sdk_time::sleep;

use crate::collection::CollectionState;

/// Buffer reset window: a printable character extends the current search if
/// it arrives within this long of the previous one, else starts a fresh
/// search. Not an APG-specified number (the pattern page leaves exact
/// timing unspecified, "Optional") -- matches Radix's own menu typeahead
/// timeout for parity, the same number `select/context.rs`'s own
/// (caller-configurable) `typeahead_timeout` defaults to elsewhere in this
/// crate.
pub(crate) const TYPEAHEAD_RESET_MS: u64 = 1000;

/// The pure matcher. `items` is every candidate's `(index, label,
/// disabled)` -- typically [`CollectionState::text_entries`]'s own return
/// value, passed straight through; need not be pre-sorted by index (this
/// function sorts its own copy). `current` is the collection's currently
/// focused index, if any. `buffer` is the accumulated, not-yet-reset
/// typeahead search string for this keystroke.
///
/// Mirrors Radix's `getNextMatch`/`wrapArray`: typing the same character
/// repeatedly is special-cased to a single-character search that always
/// advances to the *next* matching item (cycling through every item that
/// starts with it, wrapping past the end back to the first); any other
/// buffer content searches on the buffer as a whole, starting *at* (not
/// after) the currently focused item, so an item that still satisfies a
/// longer, narrowing search keeps focus exactly where it is instead of
/// jumping away to some other item that also happens to match. Either way
/// the search wraps past the last item back to the first, and disabled
/// items are never candidates and are never counted against another item's
/// position. Matching is always case-insensitive.
pub(crate) fn find_next_match(
    items: &[(usize, String, bool)],
    current: Option<usize>,
    buffer: &str,
) -> Option<usize> {
    if buffer.is_empty() {
        return None;
    }

    let buffer_chars: Vec<char> = buffer.chars().flat_map(char::to_lowercase).collect();
    if buffer_chars.is_empty() {
        return None;
    }
    // Radix's `isRepeated`: typing the same character over and over (e.g.
    // "aa", "aaa") is treated as a single-character search that cycles
    // through matches, not a literal (and near-never-matching) multi-char
    // prefix.
    let is_repeated = buffer_chars.len() > 1 && buffer_chars.iter().all(|&c| c == buffer_chars[0]);
    let search: String = if is_repeated {
        buffer_chars[0].to_string()
    } else {
        buffer_chars.into_iter().collect()
    };

    let mut available: Vec<&(usize, String, bool)> =
        items.iter().filter(|(_, _, disabled)| !disabled).collect();
    if available.is_empty() {
        return None;
    }
    available.sort_by_key(|(index, _, _)| *index);

    let current_pos =
        current.and_then(|idx| available.iter().position(|(index, _, _)| *index == idx));
    // A single-effective-character search (a fresh letter, or a repeated
    // one) always moves *away* from wherever focus already is -- start
    // right after it. Any other (narrowing) search starts *at* the
    // currently focused item instead, so it wins immediately if it still
    // matches the longer buffer.
    let exclude_current = search.chars().count() == 1;
    let n = available.len();
    let start = match current_pos {
        Some(pos) if exclude_current => (pos + 1) % n,
        Some(pos) => pos,
        None => 0,
    };

    (0..n)
        .map(|offset| (start + offset) % n)
        .find(|&pos| available[pos].1.to_lowercase().starts_with(&search))
        .map(|pos| available[pos].0)
}

/// One host's live typeahead buffer -- the same buffer/self-cancelling-
/// clear-task shape `select/context.rs`'s `SelectContext::
/// add_to_typeahead_buffer` already uses (see this module's own doc). A
/// hook (calls `use_signal`); create one per keydown handler that needs its
/// own independent search buffer -- currently once per menu-family host's
/// root/content keydown handler, so a buffer being built up in one open
/// submenu is never confused with the enclosing menu's own search. See each
/// host's own call site.
#[derive(Clone, Copy)]
pub(crate) struct TypeaheadState {
    buffer: Signal<String>,
    clear_task: Signal<Option<Task>>,
}

/// Create a fresh, empty [`TypeaheadState`]. Call unconditionally, once per
/// render, like every other hook in this crate.
pub(crate) fn use_typeahead_state() -> TypeaheadState {
    TypeaheadState {
        buffer: use_signal(String::new),
        clear_task: use_signal(|| None),
    }
}

impl TypeaheadState {
    /// Handle one keydown event: if (and only if) `event`'s key is a single
    /// printable character with no Ctrl/Meta/Alt modifier held and is not
    /// Space (menu-item activation -- every call site's own `onkeydown`
    /// already matches that case separately, before this ever runs), append
    /// it to the buffer (restarting the reset timer), search `items` via
    /// [`find_next_match`] starting from `collection`'s own currently
    /// focused item, and move roving focus there via
    /// [`CollectionState::set_focus`] on a match.
    ///
    /// Returns whether the key qualified as typeahead input at all -- not
    /// whether a match was found. Every call site uses this to decide
    /// whether to fall through to the same `event.prevent_default()` (and,
    /// where present, `event.stop_propagation()`) its other handled keys
    /// already call, and to `return` early, untouched, otherwise -- so a
    /// disqualified key (a held modifier, Space, anything else) behaves
    /// exactly as if this host had no typeahead handling for it at all.
    pub(crate) fn handle_key(
        &mut self,
        mut collection: CollectionState,
        items: &[(usize, String, bool)],
        event: &Event<KeyboardData>,
    ) -> bool {
        let Key::Character(c) = event.key() else {
            return false;
        };
        if c.chars().count() != 1 || c == " " {
            return false;
        }
        let modifiers = event.modifiers();
        if modifiers.ctrl() || modifiers.meta() || modifiers.alt() {
            return false;
        }

        // Same cancel-then-reschedule shape as
        // `SelectContext::add_to_typeahead_buffer` -- see that function's
        // doc for why the old task is explicitly cancelled rather than left
        // to resolve on its own (a stale clear firing after a fresher
        // keystroke would wipe the buffer that keystroke just extended).
        if let Some(existing_task) = self.clear_task.write().take() {
            existing_task.cancel();
        }
        let buffer = {
            let mut buffer = self.buffer.write();
            buffer.push_str(&c);
            buffer.clone()
        };
        let mut buffer_signal = self.buffer;
        let mut clear_task_signal = self.clear_task;
        let task = spawn(async move {
            sleep(Duration::from_millis(TYPEAHEAD_RESET_MS)).await;
            buffer_signal.write().clear();
            clear_task_signal.write().take();
        });
        self.clear_task.write().replace(task);

        if let Some(index) = find_next_match(items, collection.focused_index(), &buffer) {
            collection.set_focus(Some(index));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(index: usize, label: &str) -> (usize, String, bool) {
        (index, label.to_string(), false)
    }

    fn disabled_item(index: usize, label: &str) -> (usize, String, bool) {
        (index, label.to_string(), true)
    }

    #[test]
    fn finds_first_match_from_no_current_focus() {
        let items = vec![item(0, "Apple"), item(1, "Banana"), item(2, "Cherry")];
        assert_eq!(find_next_match(&items, None, "b"), Some(1));
    }

    #[test]
    fn wraps_past_the_end_back_to_the_first_match() {
        let items = vec![item(0, "Apple"), item(1, "Banana"), item(2, "Cherry")];
        // Cherry is focused; typing "a" has nowhere to go but wrap back to Apple.
        assert_eq!(find_next_match(&items, Some(2), "a"), Some(0));
    }

    #[test]
    fn skips_disabled_items_even_when_their_label_matches() {
        let items = vec![
            disabled_item(0, "Apple"),
            item(1, "Avocado"),
            item(2, "Banana"),
        ];
        assert_eq!(find_next_match(&items, None, "a"), Some(1));
    }

    #[test]
    fn repeating_the_same_character_cycles_through_matches() {
        let items = vec![item(0, "Apple"), item(1, "Avocado"), item(2, "Banana")];
        // Apple focused; "aa" (typed 'a' twice) cycles to the next "a" item.
        assert_eq!(find_next_match(&items, Some(0), "aa"), Some(1));
        // Avocado focused; "aaa" cycles onward, wrapping past Banana back to Apple.
        assert_eq!(find_next_match(&items, Some(1), "aaa"), Some(0));
    }

    #[test]
    fn matching_is_case_insensitive() {
        let items = vec![item(0, "Apple")];
        assert_eq!(find_next_match(&items, None, "A"), Some(0));
        assert_eq!(find_next_match(&items, Some(0), "a"), Some(0));
    }

    #[test]
    fn a_narrowing_search_keeps_the_still_matching_current_item() {
        let items = vec![item(0, "Delete"), item(1, "Delta")];
        // Delete focused (matched via "d"); "de" still matches Delete, so
        // focus stays there rather than jumping to Delta.
        assert_eq!(find_next_match(&items, Some(0), "de"), Some(0));
    }

    #[test]
    fn a_narrowing_search_moves_on_when_the_current_item_no_longer_matches() {
        let items = vec![item(0, "Duplicate"), item(1, "Delete")];
        assert_eq!(find_next_match(&items, Some(0), "de"), Some(1));
    }

    #[test]
    fn empty_buffer_matches_nothing() {
        let items = vec![item(0, "Apple")];
        assert_eq!(find_next_match(&items, None, ""), None);
    }

    #[test]
    fn all_items_disabled_matches_nothing() {
        let items = vec![disabled_item(0, "Apple")];
        assert_eq!(find_next_match(&items, None, "a"), None);
    }

    #[test]
    fn unordered_input_is_still_navigated_in_index_order() {
        // `CollectionState::text_entries` already returns index-ordered
        // input, but this function sorts its own copy rather than trusting
        // that -- verify it actually does.
        let items = vec![item(2, "Cherry"), item(0, "Apple"), item(1, "Banana")];
        assert_eq!(find_next_match(&items, None, "b"), Some(1));
        assert_eq!(find_next_match(&items, Some(0), "c"), Some(2));
    }
}
