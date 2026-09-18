//! Pure state helpers for the Data Table composition pattern
//! (dev-docs/component-backlog.md row 69: "a composition pattern, not a
//! new primitive").
//!
//! Deliberately plain, dioxus-free generic functions over slices and
//! closures, not signals/hooks -- the caller (`component.rs`'s themed
//! composition pieces, or a consumer's own demo like
//! `variants/main/mod.rs`) owns the reactive state and calls these as
//! ordinary Rust to derive what to render. Kept as a sibling file of
//! `component.rs` rather than folded into it so this logic can be unit
//! tested in isolation (`#[cfg(test)] mod tests` below) -- the
//! `dev-docs/plan.md` definition of done's "a conformance/regression test
//! that ... would fail without the change" for this lane's pure-logic
//! half (the composition components themselves are covered by
//! `playwright/data_table.spec.ts` and the tier-1 APG oracle instead).
//!
//! Module wiring note: `component.rs` is itself a *file* module
//! (`preview/src/components/mod.rs`'s `examples!` macro declares
//! `pub(crate) mod component;` inline inside an already-inline
//! `mod data_table { ... }` block), so a plain `mod state;` written inside
//! `component.rs` would resolve to `data_table/component/state.rs` (a
//! nested `component/` subdirectory), not this file. `component.rs`
//! instead writes `#[path = "state.rs"] mod state;`, which is relative to
//! `component.rs`'s own directory and so resolves to this file, a sibling
//! of `component.rs` -- the same directory `dx components add data_table`
//! copies verbatim, so the copied module keeps compiling with zero extra
//! wiring in a consumer's project.

/// Which way a column is sorted. Mirrors the APG Table pattern's
/// `aria-sort` token set (`ascending` | `descending`; this crate's sortable
/// columns are always in one of these two states once sorted -- see
/// `DataTableColumnHeader`'s doc for the `Option<SortDirection>` /
/// `"none"` third state, which lives one level up since it is really "no
/// sort direction *for this column*", not a third direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// Flips the direction -- used when the same column's sort control is
    /// activated again. Mirrors the APG sortable-table reference
    /// implementation's own toggle (`playwright/oracle/reference/7e4034b/
    /// content/patterns/table/examples/js/sortable-table.js`,
    /// `setColumnHeaderSort`: sets `descending` unless the column is
    /// already `descending`, in which case `ascending`).
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    /// The `aria-sort` token for this direction.
    pub fn aria_sort(self) -> &'static str {
        match self {
            SortDirection::Ascending => "ascending",
            SortDirection::Descending => "descending",
        }
    }
}

/// Returns a new `Vec` with `rows` sorted by the key `key_fn` extracts,
/// ordered by `direction`. Uses `slice::sort_by`, which is a stable sort,
/// so rows whose keys compare equal keep their relative order -- e.g.
/// sorting by `status` alone doesn't reshuffle same-status rows.
pub fn sort_rows<T: Clone, K: Ord>(rows: &[T], direction: SortDirection, key_fn: impl Fn(&T) -> K) -> Vec<T> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        let (ka, kb) = (key_fn(a), key_fn(b));
        match direction {
            SortDirection::Ascending => ka.cmp(&kb),
            SortDirection::Descending => kb.cmp(&ka),
        }
    });
    sorted
}

/// Returns a new `Vec` containing only the rows `predicate` accepts.
/// Whether/how an empty query means "match everything" is the caller's own
/// predicate's business (see the demo's `filter_query.is_empty() || ...`) --
/// this function stays a pure, unopinionated filter.
pub fn filter_rows<T: Clone>(rows: &[T], predicate: impl Fn(&T) -> bool) -> Vec<T> {
    rows.iter().filter(|row| predicate(row)).cloned().collect()
}

/// Splits `rows` into 0-based `page`'s slice of (at most) `page_size` rows,
/// plus the total page count. `page_size` is floored at 1 (a zero page
/// size would otherwise divide by zero); the total page count is floored
/// at 1 too, even for zero rows, so a caller can always render "Page 1 of
/// 1" against an empty table rather than special-casing it. An
/// out-of-range `page` (e.g. the filtered row count shrank after a filter
/// change) is clamped to the last valid page rather than returning an
/// empty slice.
pub fn paginate<T: Clone>(rows: &[T], page: usize, page_size: usize) -> (Vec<T>, usize) {
    let page_size = page_size.max(1);
    let total_pages = rows.len().div_ceil(page_size).max(1);
    let page = page.min(total_pages - 1);
    let start = (page * page_size).min(rows.len());
    let end = (start + page_size).min(rows.len());
    (rows[start..end].to_vec(), total_pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Row {
        name: &'static str,
        amount: i64,
    }

    const ROWS: &[Row] = &[
        Row { name: "carol", amount: 30 },
        Row { name: "alice", amount: 10 },
        Row { name: "bob", amount: 20 },
        Row { name: "dana", amount: 20 },
    ];

    #[test]
    fn sort_rows_ascending_orders_by_key() {
        let sorted = sort_rows(ROWS, SortDirection::Ascending, |r| r.amount);
        assert_eq!(
            sorted.iter().map(|r| r.name).collect::<Vec<_>>(),
            vec!["alice", "bob", "dana", "carol"]
        );
    }

    #[test]
    fn sort_rows_descending_reverses_order() {
        let sorted = sort_rows(ROWS, SortDirection::Descending, |r| r.amount);
        assert_eq!(
            sorted.iter().map(|r| r.name).collect::<Vec<_>>(),
            vec!["carol", "bob", "dana", "alice"]
        );
    }

    #[test]
    fn sort_rows_is_stable_for_equal_keys() {
        // bob and dana both have amount 20 and appear bob-then-dana in
        // ROWS; ascending sort must keep that relative order rather than
        // an unstable sort silently swapping them.
        let sorted = sort_rows(ROWS, SortDirection::Ascending, |r| r.amount);
        let bob_index = sorted.iter().position(|r| r.name == "bob").unwrap();
        let dana_index = sorted.iter().position(|r| r.name == "dana").unwrap();
        assert!(bob_index < dana_index, "stable sort must keep bob before dana");
    }

    #[test]
    fn sort_rows_does_not_mutate_input() {
        let _ = sort_rows(ROWS, SortDirection::Ascending, |r| r.amount);
        assert_eq!(ROWS[0].name, "carol", "sort_rows must not reorder the caller's slice");
    }

    #[test]
    fn sort_direction_toggled_flips_both_ways() {
        assert_eq!(SortDirection::Ascending.toggled(), SortDirection::Descending);
        assert_eq!(SortDirection::Descending.toggled(), SortDirection::Ascending);
    }

    #[test]
    fn sort_direction_aria_sort_tokens() {
        assert_eq!(SortDirection::Ascending.aria_sort(), "ascending");
        assert_eq!(SortDirection::Descending.aria_sort(), "descending");
    }

    #[test]
    fn filter_rows_keeps_only_matching() {
        let filtered = filter_rows(ROWS, |r| r.name.starts_with('d'));
        assert_eq!(filtered, vec![Row { name: "dana", amount: 20 }]);
    }

    #[test]
    fn filter_rows_empty_predicate_result_is_empty_not_error() {
        let filtered = filter_rows(ROWS, |r| r.name == "nobody");
        assert!(filtered.is_empty());
    }

    #[test]
    fn paginate_splits_into_pages() {
        let (page0, total_pages) = paginate(ROWS, 0, 2);
        assert_eq!(page0.iter().map(|r| r.name).collect::<Vec<_>>(), vec!["carol", "alice"]);
        assert_eq!(total_pages, 2);

        let (page1, total_pages) = paginate(ROWS, 1, 2);
        assert_eq!(page1.iter().map(|r| r.name).collect::<Vec<_>>(), vec!["bob", "dana"]);
        assert_eq!(total_pages, 2);
    }

    #[test]
    fn paginate_last_page_can_be_short() {
        let (page1, total_pages) = paginate(ROWS, 1, 3);
        assert_eq!(page1.iter().map(|r| r.name).collect::<Vec<_>>(), vec!["dana"]);
        assert_eq!(total_pages, 2);
    }

    #[test]
    fn paginate_clamps_out_of_range_page_to_last() {
        let (page, total_pages) = paginate(ROWS, 99, 2);
        assert_eq!(total_pages, 2);
        assert_eq!(page.iter().map(|r| r.name).collect::<Vec<_>>(), vec!["bob", "dana"]);
    }

    #[test]
    fn paginate_empty_rows_yields_one_page_no_panic() {
        let empty: Vec<Row> = Vec::new();
        let (page, total_pages) = paginate(&empty, 0, 5);
        assert!(page.is_empty());
        assert_eq!(total_pages, 1);
    }

    #[test]
    fn paginate_zero_page_size_does_not_divide_by_zero() {
        let (page, total_pages) = paginate(ROWS, 0, 0);
        assert_eq!(total_pages, ROWS.len());
        assert_eq!(page.len(), 1);
    }
}
