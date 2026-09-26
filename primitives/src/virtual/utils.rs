//! Utility functions for the virtual list implementation.

use std::ops::RangeInclusive;

use super::types::VirtualItem;
use super::window::window;

/// Binary search to find the nearest item at or before the given offset.
///
/// Returns the index of the item whose `start` position is closest to
/// (but not exceeding) the given offset.
pub(crate) fn find_nearest_binary_search(measurements: &[VirtualItem], offset: u32) -> usize {
    measurements
        .binary_search_by(|item| item.start().cmp(&offset))
        .unwrap_or_else(|idx| idx.saturating_sub(1))
}

/// Extract indices from a range with overscan applied.
///
/// Delegates to the shared [`window`] math (`wrap: false`) so there is a single
/// implementation of "which indices are in the window, with overscan" — the same
/// one a looping virtual carousel uses with `wrap: true`. `range` is passed through
/// verbatim (its `end` is the caller's exclusive bound, not an inclusive position);
/// `window`'s clamp-and-pad arithmetic is applied to `range.start`/`range.end` exactly
/// as this function historically applied it directly, which is what reproduces the
/// existing numeric behaviour bit-for-bit.
pub(crate) fn default_range_extractor(
    range: std::ops::Range<usize>,
    overscan: usize,
    count: usize,
) -> RangeInclusive<usize> {
    if count == 0 {
        return 0..=0;
    }

    let items = window(
        count,
        range.start as isize,
        range.end as isize,
        overscan,
        false,
    );
    match (items.first(), items.last()) {
        (Some(first), Some(last)) => first.data_index..=last.data_index,
        (_, _) => 0..=0,
    }
}
