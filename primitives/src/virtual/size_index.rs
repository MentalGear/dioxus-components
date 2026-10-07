//! Item geometry for the windowed list: prefix sums over per-item sizes with `O(log N)` point
//! update and `O(log N)` position lookup.
//!
//! The previous engine kept a `Vec<VirtualItem>` (`start`, `size` per item) that was rebuilt in
//! full, `O(N)`, whenever a row was measured, and cloned in full, `O(N)`, on every row resize
//! event. At 1M rows that is ~24 MB allocated per measurement. A [`SizeIndex`] is a Fenwick tree
//! (binary indexed tree), so measuring a row is one `O(log N)` update and "which row is at pixel
//! `y`" is one `O(log N)` descent. Nothing is copied.
//!
//! # What an unmeasured item weighs
//!
//! Exactly what the old `compute_measurements` did, so the numbers on screen do not change:
//!
//! - with a per-item estimate (`estimate_size` given): the estimate, until the item is measured;
//! - without one ("adaptive"): the integer average of every measured size so far, or
//!   [`DEFAULT_SIZE`] before the first measurement. That average moves with every measurement, so
//!   an unmeasured item's size is *not* stored: each tree node keeps the *count* of unmeasured
//!   items it covers and the weight is `count * average` at query time, which makes the average
//!   change free instead of an `O(N)` rewrite.
//!
//! The tree is a pure function of `(count, estimates, measured sizes)`; [`SizeIndex::new`] is the
//! only `O(N)` operation and runs when `count` or the estimates change, never per scroll event or
//! per measurement.

use std::collections::HashMap;
use std::rc::Rc;

use super::types::VirtualItem;

/// The size of an unmeasured item when there is neither an estimate nor any measurement yet.
pub(crate) const DEFAULT_SIZE: u32 = 100;

/// Per-item size estimates for a list of `count` items, produced once per `(count, estimate)`
/// change and shared (by `Rc`) with the [`SizeIndex`] built from it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SizeEstimates {
    count: usize,
    /// `None` selects adaptive estimation (see the module doc).
    per_item: Option<Rc<[u32]>>,
}

impl SizeEstimates {
    /// Evaluate `estimate` for every index in `0..count`. `O(N)` calls, made inside the caller's
    /// `use_memo` so signals the callback reads are tracked.
    pub(crate) fn evaluate(count: usize, estimate: Option<&dyn Fn(usize) -> u32>) -> Self {
        Self {
            count,
            per_item: estimate.map(|f| (0..count).map(f).collect()),
        }
    }

    /// Whether `self` and `other` are the same evaluation (same count, same shared slice).
    fn same_as(&self, other: &Self) -> bool {
        self.count == other.count
            && match (&self.per_item, &other.per_item) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}

/// One Fenwick node: the totals of the items it covers, split by what an item weighs.
#[derive(Clone, Copy, Default)]
struct Node {
    /// Sum of the estimates of the covered items that are still unmeasured (estimate mode).
    estimate: i64,
    /// How many covered items are still unmeasured (adaptive mode weighs each by the average).
    unmeasured: i64,
    /// Sum of the measured sizes of the covered items.
    measured: i64,
}

impl Node {
    fn add(&mut self, other: Node) {
        self.estimate += other.estimate;
        self.unmeasured += other.unmeasured;
        self.measured += other.measured;
    }
}

/// Prefix sums over item sizes. See the module doc.
pub(crate) struct SizeIndex {
    source: SizeEstimates,
    /// Measured sizes by item index. Entries at or beyond `count` are kept, so a list that
    /// shrinks and grows back remembers them (the old `item_size_cache` never evicted either).
    measured: HashMap<usize, u32>,
    measured_sum: u64,
    /// 1-based Fenwick array of length `count + 1`.
    tree: Vec<Node>,
    /// The highest power of two `<= count` (0 when empty): the first step of a descent.
    top_bit: usize,
}

impl SizeIndex {
    /// Build the index. `O(N + M)` for `N` items and `M` measured sizes.
    pub(crate) fn new(source: SizeEstimates, measured: HashMap<usize, u32>) -> Self {
        let count = source.count;
        let measured_sum = measured.values().map(|&v| v as u64).sum();
        let mut tree = vec![Node::default(); count + 1];
        for i in 0..count {
            tree[i + 1] = match measured.get(&i) {
                Some(&size) => Node {
                    measured: size as i64,
                    ..Node::default()
                },
                None => Node {
                    estimate: source
                        .per_item
                        .as_ref()
                        .map_or(0, |e| e.get(i).copied().unwrap_or(DEFAULT_SIZE) as i64),
                    unmeasured: 1,
                    measured: 0,
                },
            };
        }
        // Linear-time construction: push each node's total into its parent.
        for i in 1..=count {
            let parent = i + (i & i.wrapping_neg());
            if parent <= count {
                let child = tree[i];
                tree[parent].add(child);
            }
        }
        let top_bit = if count == 0 {
            0
        } else {
            1 << (usize::BITS - 1 - count.leading_zeros())
        };
        Self {
            source,
            measured,
            measured_sum,
            tree,
            top_bit,
        }
    }

    /// Rebuild for new estimates, keeping every measured size. A no-op, returning `false`, when
    /// `source` is the evaluation this index was already built from.
    pub(crate) fn sync(&mut self, source: &SizeEstimates) -> bool {
        if self.source.same_as(source) {
            return false;
        }
        let measured = std::mem::take(&mut self.measured);
        *self = Self::new(source.clone(), measured);
        true
    }

    pub(crate) fn count(&self) -> usize {
        self.source.count
    }

    /// The measured size of item `index`, if it has been measured.
    pub(crate) fn measured_size(&self, index: usize) -> Option<u32> {
        self.measured.get(&index).copied()
    }

    /// The average the adaptive mode weighs an unmeasured item by; 0 in estimate mode (where
    /// the per-item estimate carries the weight instead).
    fn average(&self) -> i64 {
        if self.source.per_item.is_some() {
            0
        } else if self.measured.is_empty() {
            DEFAULT_SIZE as i64
        } else {
            (self.measured_sum / self.measured.len() as u64) as i64
        }
    }

    fn weight(node: &Node, average: i64) -> i64 {
        node.estimate + average * node.unmeasured + node.measured
    }

    /// The summed size of the first `n` items.
    fn prefix(&self, n: usize) -> i64 {
        let average = self.average();
        let mut sum = 0;
        let mut i = n.min(self.source.count);
        while i > 0 {
            sum += Self::weight(&self.tree[i], average);
            i &= i - 1;
        }
        sum
    }

    fn clamp_px(value: i64) -> u32 {
        value.clamp(0, u32::MAX as i64) as u32
    }

    /// The current size of item `index` (measured, else estimated, else the adaptive average).
    pub(crate) fn size(&self, index: usize) -> u32 {
        if let Some(&size) = self.measured.get(&index) {
            return size;
        }
        match &self.source.per_item {
            Some(e) => e.get(index).copied().unwrap_or(DEFAULT_SIZE),
            None => Self::clamp_px(self.average()),
        }
    }

    /// The position of the top of item `index` (the summed size of the items before it).
    pub(crate) fn start(&self, index: usize) -> u32 {
        Self::clamp_px(self.prefix(index))
    }

    /// The summed size of every item.
    pub(crate) fn total(&self) -> u32 {
        Self::clamp_px(self.prefix(self.source.count))
    }

    /// Item `index` with its current position, or `None` past the end.
    pub(crate) fn item(&self, index: usize) -> Option<VirtualItem> {
        (index < self.source.count)
            .then(|| VirtualItem::new(index, index, self.start(index), self.size(index)))
    }

    /// The largest `k` in `0..=count` for which the first `k` items sum to at most `limit` (or,
    /// with `strict`, strictly less than `limit`). A Fenwick descent: `O(log N)`.
    fn descend(&self, limit: u32, strict: bool) -> usize {
        let average = self.average();
        let mut position = 0;
        let mut remaining = limit as i64;
        let mut step = self.top_bit;
        while step > 0 {
            let next = position + step;
            if next <= self.source.count {
                let weight = Self::weight(&self.tree[next], average);
                if weight < remaining || (!strict && weight == remaining) {
                    position = next;
                    remaining -= weight;
                }
            }
            step >>= 1;
        }
        position
    }

    /// The index of the item at or before pixel `offset`: the largest `i` with
    /// `start(i) <= offset`, clamped to the last item.
    pub(crate) fn index_at_or_before(&self, offset: u32) -> usize {
        self.descend(offset, false)
            .min(self.source.count.saturating_sub(1))
    }

    /// The index of the first item whose end reaches pixel `position` (clamped to the last
    /// item): the item holding the bottom edge of a viewport that ends at `position`.
    pub(crate) fn index_ending_at_or_after(&self, position: u32) -> usize {
        self.descend(position, true)
            .min(self.source.count.saturating_sub(1))
    }

    /// Record the measured size of item `index`. `O(log N)`. Returns the size it replaced
    /// (`None` the first time). The adaptive average moves with it, for free.
    pub(crate) fn set_measured(&mut self, index: usize, size: u32) -> Option<u32> {
        let previous = self.measured.insert(index, size);
        match previous {
            Some(old) => self.measured_sum = self.measured_sum - old as u64 + size as u64,
            None => self.measured_sum += size as u64,
        }
        if index < self.source.count {
            let delta = match previous {
                Some(old) => Node {
                    measured: size as i64 - old as i64,
                    ..Node::default()
                },
                None => Node {
                    estimate: -self
                        .source
                        .per_item
                        .as_ref()
                        .map_or(0, |e| e.get(index).copied().unwrap_or(DEFAULT_SIZE) as i64),
                    unmeasured: -1,
                    measured: size as i64,
                },
            };
            let mut i = index + 1;
            while i <= self.source.count {
                self.tree[i].add(delta);
                i += i & i.wrapping_neg();
            }
        }
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The old engine, verbatim: the oracle every `SizeIndex` answer is compared with.
    fn naive_items(
        count: usize,
        measured: &HashMap<usize, u32>,
        estimate: Option<&dyn Fn(usize) -> u32>,
    ) -> Vec<VirtualItem> {
        let adaptive = if estimate.is_none() && !measured.is_empty() {
            let sum: u64 = measured.values().map(|&v| v as u64).sum();
            Some((sum / measured.len() as u64) as u32)
        } else {
            None
        };
        let mut items: Vec<VirtualItem> = Vec::with_capacity(count);
        for i in 0..count {
            let size = measured.get(&i).copied().unwrap_or_else(|| match estimate {
                Some(est) => est(i),
                None => adaptive.unwrap_or(100),
            });
            let start = items.last().map(|m| m.end()).unwrap_or(0);
            items.push(VirtualItem::new(i, i, start, size));
        }
        items
    }

    /// The old `find_nearest_binary_search`.
    fn naive_nearest(items: &[VirtualItem], offset: u32) -> usize {
        items
            .binary_search_by(|item| item.start().cmp(&offset))
            .unwrap_or_else(|idx| idx.saturating_sub(1))
    }

    /// The old end-of-viewport walk, from `start`.
    fn naive_end(items: &[VirtualItem], start: usize, position: u32) -> usize {
        let last = items.len() - 1;
        let mut end = start;
        while end < last && items[end].end() < position {
            end += 1;
        }
        end
    }

    /// xorshift64*: deterministic, dependency-free.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn estimate_for(i: usize) -> u32 {
        // Distinct per index and including a few tiny sizes, like a real list.
        20 + (i as u32 * 37) % 90
    }

    fn assert_matches_naive(
        index: &SizeIndex,
        count: usize,
        measured: &HashMap<usize, u32>,
        estimate: Option<&dyn Fn(usize) -> u32>,
        rng: &mut Rng,
    ) {
        let items = naive_items(count, measured, estimate);
        assert_eq!(index.count(), count);
        assert_eq!(index.total(), items.last().map_or(0, |m| m.end()));
        for item in &items {
            assert_eq!(
                index.start(item.index()),
                item.start(),
                "start of {}",
                item.index()
            );
            assert_eq!(
                index.size(item.index()),
                item.size(),
                "size of {}",
                item.index()
            );
            assert_eq!(index.item(item.index()).as_ref(), Some(item));
        }
        assert!(index.item(count).is_none());
        if count == 0 {
            return;
        }
        let total = items.last().unwrap().end();
        // Unique starts only: with a zero-sized item the old binary search picked an arbitrary
        // one of several equal starts, the index picks the last. Real sizes are >= 1.
        let unique = items.windows(2).all(|w| w[0].start() < w[1].start());
        for _ in 0..200 {
            let offset = rng.below(total as u64 + 50) as u32;
            if unique {
                let start = naive_nearest(&items, offset);
                assert_eq!(
                    index.index_at_or_before(offset),
                    start,
                    "at_or_before({offset})"
                );
                let position = offset + rng.below(900) as u32;
                assert_eq!(
                    index.index_ending_at_or_after(position).max(start),
                    naive_end(&items, start, position),
                    "end({position}) from {start}"
                );
            }
        }
    }

    #[test]
    fn matches_the_naive_engine_in_estimate_mode() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for count in [0usize, 1, 2, 3, 7, 8, 9, 63, 64, 65, 1000] {
            let estimate: &dyn Fn(usize) -> u32 = &estimate_for;
            let source = SizeEstimates::evaluate(count, Some(estimate));
            let mut measured = HashMap::new();
            let mut index = SizeIndex::new(source, measured.clone());
            assert_matches_naive(&index, count, &measured, Some(estimate), &mut rng);
            for _ in 0..60 {
                if count == 0 {
                    break;
                }
                let i = rng.below(count as u64) as usize;
                let size = 1 + rng.below(300) as u32;
                let previous = index.set_measured(i, size);
                assert_eq!(previous, measured.insert(i, size));
                assert_matches_naive(&index, count, &measured, Some(estimate), &mut rng);
            }
        }
    }

    #[test]
    fn matches_the_naive_engine_in_adaptive_mode() {
        // The adaptive average moves with every measurement, so unmeasured items shift: the
        // part a per-item tree could not do for free.
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        for count in [1usize, 2, 5, 16, 17, 300] {
            let source = SizeEstimates::evaluate(count, None);
            let mut measured = HashMap::new();
            let mut index = SizeIndex::new(source, measured.clone());
            assert_matches_naive(&index, count, &measured, None, &mut rng);
            for _ in 0..60 {
                let i = rng.below(count as u64) as usize;
                let size = 1 + rng.below(500) as u32;
                index.set_measured(i, size);
                measured.insert(i, size);
                assert_matches_naive(&index, count, &measured, None, &mut rng);
            }
        }
    }

    #[test]
    fn sync_rebuilds_with_the_measured_sizes_kept() {
        let mut rng = Rng(7);
        let estimate: &dyn Fn(usize) -> u32 = &estimate_for;
        let mut measured = HashMap::new();
        let mut index = SizeIndex::new(SizeEstimates::evaluate(50, Some(estimate)), HashMap::new());
        for i in [3usize, 20, 49] {
            index.set_measured(i, 77);
            measured.insert(i, 77);
        }
        // Same evaluation: nothing happens.
        let same = index.source.clone();
        assert!(!index.sync(&same));
        assert_matches_naive(&index, 50, &measured, Some(estimate), &mut rng);
        // Shrink: entries beyond the new count are remembered, not applied.
        assert!(index.sync(&SizeEstimates::evaluate(10, Some(estimate))));
        assert_matches_naive(&index, 10, &measured, Some(estimate), &mut rng);
        assert_eq!(index.measured_size(49), Some(77));
        // Grow back: they apply again.
        index.sync(&SizeEstimates::evaluate(60, Some(estimate)));
        assert_matches_naive(&index, 60, &measured, Some(estimate), &mut rng);
        // Switch to adaptive estimation.
        index.sync(&SizeEstimates::evaluate(60, None));
        assert_matches_naive(&index, 60, &measured, None, &mut rng);
    }

    #[test]
    fn adaptive_average_counts_every_measurement_even_past_the_end() {
        // The old `item_size_cache` average included entries at or beyond `count`.
        let mut index = SizeIndex::new(SizeEstimates::evaluate(4, None), HashMap::new());
        index.set_measured(100, 300);
        assert_eq!(index.size(0), 300);
        assert_eq!(index.total(), 1200);
    }

    #[test]
    fn a_million_rows_measure_and_look_up_without_copying() {
        let count = 1_000_000;
        let estimate: &dyn Fn(usize) -> u32 = &|i| 40 + (i % 7) as u32;
        let mut index = SizeIndex::new(
            SizeEstimates::evaluate(count, Some(estimate)),
            HashMap::new(),
        );
        let before = index.total();
        index.set_measured(999_999, 140);
        assert_eq!(index.total(), before - (40 + (999_999 % 7) as u32) + 140);
        let at = index.start(500_000);
        assert_eq!(index.index_at_or_before(at), 500_000);
        assert_eq!(index.index_at_or_before(at - 1), 499_999);
    }

    /// Cost of one row measurement: the old engine rebuilt and cloned the whole geometry, the
    /// index updates one tree path. Run with
    /// `cargo test --release -p dioxus-primitives size_index -- --ignored --nocapture`.
    #[test]
    #[ignore = "benchmark, run by hand"]
    fn bench_one_measurement_old_rebuild_against_the_index() {
        use std::time::Instant;
        let estimate: &dyn Fn(usize) -> u32 = &|i| 40 + (i % 7) as u32;
        for count in [10_000usize, 100_000, 1_000_000] {
            let rounds = if count >= 1_000_000 { 20 } else { 200 };
            let mut measured: HashMap<usize, u32> = HashMap::new();

            // Old: every new measurement rebuilt the Vec<VirtualItem> (the memo), and every
            // resize event cloned it (`measurements.peek().clone()`).
            let t = Instant::now();
            let mut keep = 0u64;
            for i in 0..rounds {
                measured.insert(i * 7, 90 + (i % 13) as u32);
                let items = naive_items(count, &measured, Some(estimate));
                let snapshot = items.clone();
                keep += snapshot.last().map_or(0, |m| m.end() as u64);
            }
            let old = t.elapsed() / rounds as u32;

            // New: one tree update, one position lookup.
            let mut index = SizeIndex::new(
                SizeEstimates::evaluate(count, Some(estimate)),
                HashMap::new(),
            );
            let t = Instant::now();
            for i in 0..rounds {
                index.set_measured(i * 7, 90 + (i % 13) as u32);
                keep += index.start(i * 7 + 1) as u64
                    + index.index_at_or_before(keep as u32 % 1000) as u64;
            }
            let new = t.elapsed() / rounds as u32;

            // The one O(N) cost left: building the index when `count` or the estimates change.
            let t = Instant::now();
            let source = SizeEstimates::evaluate(count, Some(estimate));
            let built = SizeIndex::new(source, HashMap::new());
            let build = t.elapsed();
            keep += built.total() as u64;
            println!("N={count:>9}: old {old:>10.2?} / measurement, index {new:>8.2?} / measurement, one-off build {build:>9.2?} (ignore {keep})");
        }
    }
}
