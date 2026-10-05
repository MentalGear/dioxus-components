//! `content-visibility` chunking: the one place that decides how rows are grouped for skipping.
//! Used by [`crate::virtual_list::VirtualList`] (content-visibility mode) and by
//! [`crate::message_scroller::MessageScrollerRows`].
//!
//! # Why rows are skipped in chunks, not one by one
//!
//! `content-visibility: auto` makes the browser do work for every element it may skip: an
//! intersection check on every frame that moves or lays out the page, then style, layout and
//! paint bookkeeping. Put on each row, a scroll therefore costs the main thread time proportional
//! to the row count on every frame. Measured in the `VirtualList` demo (release build, headless
//! Chromium, main-thread ms per scroll frame):
//!
//! | rows | one skippable element per row | one per 20 rows | windowed |
//! |---|---|---|---|
//! | 1,000 | 7.7 | 1.1 | |
//! | 5,000 | 31 | 1.5 | 1.4 |
//!
//! and `MessageScroller`'s 2,000-row demo cost 20.3 ms per frame with one element per row and 3.7 ms with chunks (measured on a heavily loaded machine; the transcript's own controller work is included). A
//! chunk of [`CHUNK_ROWS`] consecutive rows is skipped, un-skipped and remembered (`auto`) as one,
//! so the work is `O(rows / 20)`. The price is that up to 19 more rows than the viewport needs
//! are rendered around it, and that the skip unit's paint containment is the chunk's box, not
//! each row's.
//!
//! # Chunk identity
//!
//! A row's chunk comes from its *absolute position* `start + index`, not its index in the
//! slice, so a caller that prepends rows and lowers `start` by the same amount leaves every
//! existing row in the chunk it was in (and its DOM node alive). Chunks are keyed by that
//! absolute chunk number.

use std::ops::Range;

use dioxus::prelude::*;
use dioxus_attributes::attributes;

use crate::merge_attributes;

/// How many consecutive rows are skipped (and un-skipped) together.
pub(crate) const CHUNK_ROWS: usize = 20;

/// One group of rows: `rows` are indices into the caller's `0..count`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Chunk {
    /// The absolute chunk number, `(start + index).div_euclid(CHUNK_ROWS)`.
    pub(crate) key: isize,
    pub(crate) rows: Range<usize>,
}

/// The chunks of `count` rows whose first row has absolute position `start`. `O(count / 20)`.
pub(crate) fn chunks(count: usize, start: isize) -> Vec<Chunk> {
    let size = CHUNK_ROWS as isize;
    let mut out = Vec::with_capacity(count / CHUNK_ROWS + 2);
    let mut first = 0usize;
    while first < count {
        let position = start + first as isize;
        // Rows left in this chunk: the first chunk may be partial when `start` is not aligned.
        let room = (size - position.rem_euclid(size)) as usize;
        let end = (first + room).min(count);
        out.push(Chunk {
            key: position.div_euclid(size),
            rows: first..end,
        });
        first = end;
    }
    out
}

/// Who writes the skip rule and the height guess of a chunk.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ChunkSkip {
    /// The chunk carries the rule inline (an unstyled primitive): `content-visibility: auto` and
    /// `contain-intrinsic-block-size: auto <sum of its rows' estimates>px`, the estimate being
    /// `estimate(index)` px per row, or `fallback` px for a row without one.
    Inline {
        estimate: Option<Callback<usize, u32>>,
        fallback: u32,
    },
    /// A stylesheet writes the rule; the chunk only says how many rows it holds, as the custom
    /// property `--dx-chunk-rows`, for the stylesheet's height guess.
    Stylesheet,
}

fn chunk_style(skip: ChunkSkip, rows: &Range<usize>) -> String {
    match skip {
        ChunkSkip::Inline { estimate, fallback } => {
            let height = rows
                .clone()
                .map(|idx| estimate.map_or(fallback, |estimate| estimate(idx)))
                .fold(0u32, u32::saturating_add);
            format!("content-visibility: auto; contain-intrinsic-block-size: auto {height}px;")
        }
        ChunkSkip::Stylesheet => format!("--dx-chunk-rows: {};", rows.len()),
    }
}

/// Render `count` rows grouped in chunks: one `div` per chunk carrying `attributes` (plus the
/// skip rule, see [`ChunkSkip`]), holding `row(index)` for each of its rows.
pub(crate) fn cv_chunks(
    count: usize,
    start: isize,
    skip: ChunkSkip,
    attributes: &[Attribute],
    row: impl Fn(usize) -> Element,
) -> Element {
    let nodes = chunks(count, start).into_iter().map(|chunk| {
        let style = chunk_style(skip, &chunk.rows);
        let chunk_attributes =
            merge_attributes(vec![attributes.to_vec(), attributes!(div { style: style })]);
        rsx! {
            div {
                key: "{chunk.key}",
                ..chunk_attributes,
                for idx in chunk.rows.clone() {
                    {row(idx)}
                }
            }
        }
    });
    rsx! {
        {nodes}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covered(count: usize, start: isize) -> Vec<usize> {
        chunks(count, start)
            .into_iter()
            .flat_map(|c| c.rows)
            .collect()
    }

    #[test]
    fn chunks_cover_every_row_exactly_once() {
        for start in [0isize, 1, 7, 19, 20, -1, -7, -20, -21, 1_000_003] {
            for count in [0usize, 1, 19, 20, 21, 39, 40, 41, 1_000, 5_001] {
                assert_eq!(
                    covered(count, start),
                    (0..count).collect::<Vec<_>>(),
                    "count {count} start {start}"
                );
                for chunk in chunks(count, start) {
                    assert!(
                        (1..=CHUNK_ROWS).contains(&chunk.rows.len()),
                        "chunk {} of {count} from {start}: {:?}",
                        chunk.key,
                        chunk.rows
                    );
                }
            }
        }
    }

    #[test]
    fn an_aligned_start_makes_full_chunks() {
        let all = chunks(100, 0);
        assert_eq!(all.len(), 5);
        assert!(all.iter().all(|c| c.rows.len() == CHUNK_ROWS));
        assert_eq!(
            all.iter().map(|c| c.key).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn a_misaligned_start_makes_a_short_first_chunk() {
        let all = chunks(50, 5);
        assert_eq!(
            all[0].rows,
            0..15,
            "positions 5..20 are the rest of chunk 0"
        );
        assert_eq!(all[0].key, 0);
        assert_eq!(all[1].rows, 15..35);
        assert_eq!(all.last().unwrap().rows, 35..50);
    }

    /// The chunk of every row by its absolute position, to compare two renders of one list.
    fn chunk_of_position(count: usize, start: isize) -> Vec<(isize, isize)> {
        chunks(count, start)
            .into_iter()
            .flat_map(|c| c.rows.map(move |i| (start + i as isize, c.key)))
            .collect()
    }

    #[test]
    fn prepending_rows_and_lowering_start_keeps_every_row_in_its_chunk() {
        // 100 rows from position 0; then 6 older rows are prepended and `start` becomes -6.
        let before = chunk_of_position(100, 0);
        let after: std::collections::HashMap<_, _> =
            chunk_of_position(106, -6).into_iter().collect();
        for (position, key) in before {
            assert_eq!(after[&position], key, "position {position} moved chunk");
        }
    }

    #[test]
    fn appending_rows_never_moves_an_existing_row() {
        let before = chunk_of_position(100, 3);
        let after: std::collections::HashMap<_, _> =
            chunk_of_position(137, 3).into_iter().collect();
        for (position, key) in before {
            assert_eq!(after[&position], key);
        }
    }

    #[test]
    fn the_stylesheet_arm_reports_the_row_count() {
        assert_eq!(
            chunk_style(ChunkSkip::Stylesheet, &(0..20)),
            "--dx-chunk-rows: 20;"
        );
        assert_eq!(
            chunk_style(ChunkSkip::Stylesheet, &(40..47)),
            "--dx-chunk-rows: 7;"
        );
    }

    #[test]
    fn the_inline_arm_sums_the_fallback_without_an_estimate() {
        let skip = ChunkSkip::Inline {
            estimate: None,
            fallback: 100,
        };
        assert_eq!(
            chunk_style(skip, &(0..20)),
            "content-visibility: auto; contain-intrinsic-block-size: auto 2000px;"
        );
        assert_eq!(
            chunk_style(skip, &(0..3)),
            "content-visibility: auto; contain-intrinsic-block-size: auto 300px;"
        );
    }
}
