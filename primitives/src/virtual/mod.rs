//! Virtual list implementation using Dioxus Store for fine-grained reactivity.
//!
//! This module provides the core algorithms needed for efficient list virtualization:
//!
//! - Computing item positions from measured or estimated sizes
//! - Calculating the visible range using binary search
//! - Handling scroll position corrections when items resize

pub(crate) mod types;
mod utils;
mod virtualizer;
mod window;

pub(crate) use virtualizer::{
    compute_measurements, get_total_size, get_virtual_items, resize_item, set_scroll_offset,
    set_viewport_size, VirtualizerState, VirtualizerStateStoreExt,
};
// Not yet consumed within this lane: `default_range_extractor` reaches the module
// directly (`super::window::window`). Re-exported here per the shared-window-math API
// contract so the planned looping `CarouselVirtual` (a separate, concurrent lane) can
// call `crate::r#virtual::window(..., wrap: true)` without reaching into a submodule.
#[allow(unused_imports)]
pub(crate) use window::{window, WindowItem};
