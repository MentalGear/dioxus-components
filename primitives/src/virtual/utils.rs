//! Utility functions for the virtual list implementation.

use std::ops::RangeInclusive;

use super::window::window;

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
