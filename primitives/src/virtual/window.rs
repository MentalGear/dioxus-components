//! Pure "which items are in the window" math, shared by [`crate::virtual_list`] and
//! (eventually) a looping virtual carousel.
//!
//! [`window`] is the single implementation of "given a logical range of positions plus
//! overscan padding, which data indices should be rendered, in which order, keyed by
//! which stable identity". `virtual_list`'s [`super::utils::default_range_extractor`]
//! delegates to it with `wrap: false`; a future `CarouselVirtual` will call it with
//! `wrap: true` to compute a looping window around a settled anchor index.

/// One rendered position in a virtual window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowItem {
    /// Index into the data; always `< count`.
    pub data_index: usize,
    /// The logical position this item occupies. Unique within one window even when
    /// `data_index` repeats (wrap with `count` smaller than the window), so it is the
    /// stable key for rendering. Equals `data_index` whenever `wrap` is false.
    pub position: isize,
}

/// Items covering logical positions `start..=end`, padded by `overscan` on both sides,
/// in ascending `position` order.
///
/// - `wrap == false`: positions are clamped to `0..count` (the exact behaviour of
///   today's `default_range_extractor`).
/// - `wrap == true`: every position `p` in `start - overscan ..= end + overscan` is kept
///   and maps to `data_index = p.rem_euclid(count)`.
/// - `count == 0` → empty. `start > end` → treated as the single range `start..=start`
///   (there is no "negative-length" window; this keeps the function total instead of
///   panicking on a caller's inverted range).
///
/// Runs in `O(window length)`, never `O(count)` — the window length is
/// `end - start + 1 + 2 * overscan` (after the `start > end` correction above), which is
/// independent of `count` however large `count` is.
pub(crate) fn window(
    count: usize,
    start: isize,
    end: isize,
    overscan: usize,
    wrap: bool,
) -> Vec<WindowItem> {
    if count == 0 {
        return Vec::new();
    }

    let end = if start > end { start } else { end };
    let overscan = overscan as isize;
    let padded_start = start - overscan;
    let padded_end = end + overscan;

    if wrap {
        let count_isize = count as isize;
        (padded_start..=padded_end)
            .map(|position| WindowItem {
                data_index: position.rem_euclid(count_isize) as usize,
                position,
            })
            .collect()
    } else {
        let clamped_start = padded_start.max(0);
        let clamped_end = padded_end.min(count as isize - 1);
        if clamped_start > clamped_end {
            return Vec::new();
        }
        (clamped_start..=clamped_end)
            .map(|position| WindowItem {
                data_index: position as usize,
                position,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_indices(items: &[WindowItem]) -> Vec<usize> {
        items.iter().map(|i| i.data_index).collect()
    }

    fn positions(items: &[WindowItem]) -> Vec<isize> {
        items.iter().map(|i| i.position).collect()
    }

    #[test]
    fn count_zero_is_empty_no_wrap() {
        assert_eq!(window(0, 0, 0, 8, false), Vec::new());
    }

    #[test]
    fn count_zero_is_empty_with_wrap() {
        assert_eq!(window(0, 0, 0, 8, true), Vec::new());
    }

    #[test]
    fn count_one_no_wrap() {
        let items = window(1, 0, 0, 8, false);
        assert_eq!(data_indices(&items), vec![0]);
        assert_eq!(positions(&items), vec![0]);
    }

    #[test]
    fn count_one_with_wrap_repeats_same_index() {
        // radius 2 around anchor 0, count 1: every position maps back to index 0.
        let items = window(1, 0, 0, 2, true);
        assert_eq!(data_indices(&items), vec![0, 0, 0, 0, 0]);
        assert_eq!(positions(&items), vec![-2, -1, 0, 1, 2]);
    }

    #[test]
    fn count_two_no_wrap_clamped() {
        let items = window(2, 0, 1, 1, false);
        // padded 0-1..=1+1 = -1..=2, clamped to 0..=1
        assert_eq!(data_indices(&items), vec![0, 1]);
    }

    #[test]
    fn count_two_with_wrap() {
        let items = window(2, 0, 0, 2, true);
        // positions -2..=2 -> rem_euclid 2: 0,1,0,1,0
        assert_eq!(positions(&items), vec![-2, -1, 0, 1, 2]);
        assert_eq!(data_indices(&items), vec![0, 1, 0, 1, 0]);
    }

    #[test]
    fn count_three_no_wrap() {
        let items = window(3, 1, 1, 1, false);
        assert_eq!(data_indices(&items), vec![0, 1, 2]);
    }

    #[test]
    fn count_three_with_wrap_around_anchor() {
        let items = window(3, 1, 1, 1, true);
        // positions 0..=2 -> indices 0,1,2 (no wrap needed here)
        assert_eq!(positions(&items), vec![0, 1, 2]);
        assert_eq!(data_indices(&items), vec![0, 1, 2]);
    }

    #[test]
    fn count_smaller_than_window_with_wrap_duplicates_data_index_unique_position() {
        // count 3, anchor 0, radius 4: window length 9 > count.
        let items = window(3, 0, 0, 4, true);
        assert_eq!(
            positions(&items),
            vec![-4, -3, -2, -1, 0, 1, 2, 3, 4],
            "positions must all be distinct even though data_index repeats"
        );
        // rem_euclid(3): -4->2, -3->0, -2->1, -1->2, 0->0, 1->1, 2->2, 3->0, 4->1
        assert_eq!(data_indices(&items), vec![2, 0, 1, 2, 0, 1, 2, 0, 1]);

        // Positions are unique even though data_index repeats.
        let mut sorted_positions = positions(&items);
        sorted_positions.sort_unstable();
        sorted_positions.dedup();
        assert_eq!(sorted_positions.len(), items.len());
    }

    #[test]
    fn negative_positions_wrap_via_rem_euclid() {
        let items = window(5, -7, -7, 0, true);
        // rem_euclid(-7, 5) == 3
        assert_eq!(data_indices(&items), vec![3]);
    }

    #[test]
    fn positions_at_or_beyond_count_wrap_correctly() {
        let items = window(5, 12, 12, 0, true);
        // 12 rem_euclid 5 == 2
        assert_eq!(data_indices(&items), vec![2]);

        let items = window(5, 5, 5, 0, true);
        assert_eq!(data_indices(&items), vec![0]);
    }

    #[test]
    fn clamping_without_wrap_at_left_edge() {
        // start near 0, large overscan: left side must clamp at 0, not go negative
        // or panic.
        let items = window(10, 0, 0, 5, false);
        assert_eq!(data_indices(&items), vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(positions(&items), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn clamping_without_wrap_at_right_edge() {
        let items = window(10, 9, 9, 5, false);
        assert_eq!(data_indices(&items), vec![4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn overscan_zero_no_wrap() {
        let items = window(10, 3, 6, 0, false);
        assert_eq!(data_indices(&items), vec![3, 4, 5, 6]);
    }

    #[test]
    fn overscan_zero_with_wrap() {
        let items = window(10, 3, 6, 0, true);
        assert_eq!(data_indices(&items), vec![3, 4, 5, 6]);
        assert_eq!(positions(&items), vec![3, 4, 5, 6]);
    }

    #[test]
    fn start_greater_than_end_no_wrap_treated_as_single_position() {
        let items = window(10, 5, 2, 0, false);
        assert_eq!(data_indices(&items), vec![5]);
    }

    #[test]
    fn start_greater_than_end_with_wrap_treated_as_single_position() {
        let items = window(10, 5, 2, 1, true);
        assert_eq!(positions(&items), vec![4, 5, 6]);
        assert_eq!(data_indices(&items), vec![4, 5, 6]);
    }

    #[test]
    fn start_greater_than_end_never_panics_for_extreme_values() {
        // Must not panic regardless of how inverted start/end are.
        let items = window(4, isize::MAX, isize::MIN, 0, false);
        assert_eq!(data_indices(&items), Vec::<usize>::new());
    }

    #[test]
    fn large_count_small_window_is_cheap_and_correct() {
        // This must not do O(count) work: if it iterated `0..count` this test would
        // hang/allocate ~2^32 entries. It completing quickly with the right output is
        // the evidence.
        let count = 1 << 32;
        let items = window(count, 1_000_000, 1_000_002, 2, false);
        assert_eq!(
            data_indices(&items),
            vec![999_998, 999_999, 1_000_000, 1_000_001, 1_000_002, 1_000_003, 1_000_004]
        );
    }

    #[test]
    fn large_count_small_window_with_wrap_is_cheap_and_correct() {
        let count = 1 << 32;
        let items = window(count, 10, 10, 3, true);
        assert_eq!(data_indices(&items), vec![7, 8, 9, 10, 11, 12, 13]);
        assert_eq!(positions(&items), vec![7, 8, 9, 10, 11, 12, 13]);
    }
}
