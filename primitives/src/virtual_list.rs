//! Defines the [`VirtualList`] component for rendering large lists cheaply.
//!
//! # Modes
//!
//! [`VirtualListMode`] (the `virtualize` prop) chooses how rows that are off-screen are kept
//! cheap. It uses the same prop name, the same `ContentVisibility` variant and the same
//! `data-virtualize` spelling as [`crate::message_scroller::Virtualization`].
//!
//! - [`VirtualListMode::ContentVisibility`]: **every row is in the DOM** (and in the server's
//!   HTML) and the browser skips laying out and painting the ones that are off-screen
//!   (`content-visibility: auto` with `contain-intrinsic-block-size: auto <estimate>`). There is
//!   no scroll listener, no measurement and no Rust work while scrolling. The accessibility tree,
//!   cross-row selection and focus reach every row, and so does the browser's find-in-page
//!   wherever the engine searches skipped content (Chromium, Firefox 125+, Safari 26+; **not
//!   Safari 18-25**, WebKit bug 283846). The chunk holding focus or an end of the selection is
//!   never skipped: that is the browser's own "relevant to the user" rule for
//!   `content-visibility: auto`, so no controller is needed to keep them rendered. The ceiling
//!   is that DOM and memory grow linearly with the row count (a 9-node row is about 4.5 KB: ~72
//!   MB at 10k rows, ~445 MB at 100k, against ~12 MB for a window) and that the first mount
//!   renders every row (about 90 ms of main-thread time for 1,000 rows and 205 ms for 5,000 in
//!   the demo, release build). Rows are skipped in groups of [`CONTENT_VISIBILITY_CHUNK`] (20):
//!   the browser does per-frame work for every element it may skip, so one skippable element
//!   per row made a scroll cost O(rows) of main-thread time (7.7 ms per frame at 1,000 rows and
//!   31 ms at 5,000 in the demo), and one per 20 rows brings it to a windowed list's cost (1.1
//!   and 1.5 ms, against 1.4 ms windowed).
//! - [`VirtualListMode::Windowed`]: only the rows in view plus `buffer` rows each side are
//!   mounted, and a spacer gives the scrollbar the full height. DOM and memory stay flat from
//!   10k to millions of rows, rows may have any height (measured, with `estimate_size` as the
//!   first guess), and the cost is that rows outside the window do not exist: native find-in-page
//!   cannot reach them, the accessibility tree holds only the window (each row announces
//!   `aria-posinset` of `aria-setsize`), the server renders no rows, and a focused element in a
//!   row that scrolls out of the window is unmounted with it. Browsers also cap an element's
//!   height (about 17M px in Firefox, 33M px in Chromium), which bounds a single scroll canvas
//!   to roughly 300k rows of 100px.
//! - [`VirtualListMode::Auto`] (the default): `ContentVisibility` while
//!   `count <= auto_threshold` (1000 unless overridden), `Windowed` above it. The mode is
//!   decided from the live `count`, so a list that grows past the threshold re-mounts its rows
//!   in the other mode. Pass an explicit mode to opt out (a heavy row at 2000 rows may want
//!   `Windowed`; a 20k-row list of one-line rows may be fine as `ContentVisibility`).
//!
//! Both modes render the same container (`role="list"`, `tabindex="0"`, `data-virtualize`) and
//! the same rows (`role="listitem"`, `aria-setsize`, `aria-posinset`, `data-virtual-index`).
//!
//! # Windowed mode: what a scroll costs
//!
//! The container reports scroll offsets to Rust, but a re-render is requested only when the
//! rendered output would differ: the engine (`primitives/src/virtual/`) keeps the mounted range with
//! hysteresis (it is re-centred once the viewport comes within `buffer / 2` rows of its edge),
//! so a steady scroll re-renders about once per `buffer / 2` rows, not once per scroll event.
//! Row positions are a Fenwick tree, so measuring a row and finding the row at a pixel are
//! `O(log N)`; nothing is copied per measurement.

use dioxus::prelude::*;
use dioxus_attributes::attributes;
use serde::Deserialize;

use crate::r#virtual::{cv_chunks, ChunkSkip, Engine, SizeEstimates, CHUNK_ROWS, DEFAULT_SIZE};
use crate::{merge_attributes, use_effect_with_cleanup, use_unique_id};

/// The row count up to which [`VirtualListMode::Auto`] keeps every row in the DOM.
///
/// 1,000 because the cost of keeping every row grows with the count: mounting them is one
/// main-thread task proportional to the row count (about 90 ms for 1,000 rows and 205 ms, one
/// 147 ms task, for 5,000 in the demo, release build), while a window mounts about 40 rows
/// whatever the count. Raise it for lists of cheap rows, lower it for heavy ones.
pub const DEFAULT_AUTO_THRESHOLD: usize = 1_000;

/// How many consecutive rows [`VirtualListMode::ContentVisibility`] skips (and un-skips)
/// together. Chromium does per-frame work for every element that `content-visibility: auto` may
/// skip, so skipping each row separately costs the main thread `O(rows)` on every scroll frame;
/// groups of 20 keep that to `O(rows / 20)`. The grouping lives in `primitives/src/virtual/`
/// and is shared with `MessageScroller`.
pub const CONTENT_VISIBILITY_CHUNK: usize = CHUNK_ROWS;

/// How a [`VirtualList`] keeps off-screen rows cheap. See the module doc's "Modes".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VirtualListMode {
    /// [`ContentVisibility`](Self::ContentVisibility) while `count <= auto_threshold`,
    /// [`Windowed`](Self::Windowed) above it. The default.
    #[default]
    Auto,
    /// Every row stays in the DOM and the browser skips the ones that are off-screen
    /// (`content-visibility: auto`). Native find-in-page, selection, focus and the accessibility
    /// tree reach every row (find-in-page: not Safari 18-25). DOM and memory grow with the row
    /// count.
    ContentVisibility,
    /// Only the rows in view plus `buffer` rows each side are mounted. Flat cost for any row
    /// count; rows outside the window are unreachable by find-in-page and the server renders
    /// none.
    Windowed,
}

impl VirtualListMode {
    /// The `virtualize` prop's wire spelling: `"auto"`, `"content-visibility"` or `"windowed"`.
    /// A rendered list's `data-virtualize` is always one of the last two.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::ContentVisibility => "content-visibility",
            Self::Windowed => "windowed",
        }
    }

    /// The concrete mode for a list of `count` rows: `self` unless it is `Auto`, which becomes
    /// `ContentVisibility` up to `auto_threshold` rows and `Windowed` above. Never returns
    /// `Auto`.
    pub fn resolve(self, count: usize, auto_threshold: usize) -> Self {
        match self {
            Self::Auto if count <= auto_threshold => Self::ContentVisibility,
            Self::Auto => Self::Windowed,
            concrete => concrete,
        }
    }
}

/// The props for the [`VirtualList`] component.
#[derive(Props, Clone, PartialEq)]
pub struct VirtualListProps {
    /// The total number of items in the list.
    pub count: ReadSignal<usize>,
    /// Windowed mode: how many rows to mount beyond each edge of the viewport (the window is
    /// re-centred once fewer than half of them remain, so between `buffer / 2` and `buffer`
    /// rows are always mounted beyond each edge). Ignored by content-visibility mode.
    #[props(default = ReadSignal::new(Signal::new(8)))]
    pub buffer: ReadSignal<usize>,
    /// Estimates the height of an item by index, in px. Windowed mode uses it until a row is
    /// measured; content-visibility mode uses it as `contain-intrinsic-block-size` (the height
    /// the browser assumes for a row it has not rendered yet, then remembers the real one).
    /// For the steadiest scrollbar, return values close to the real heights. If not provided,
    /// windowed mode estimates from the average of the measured rows and content-visibility
    /// mode assumes 100px.
    pub estimate_size: Option<Callback<usize, u32>>,
    /// Renders a single item by its absolute index.
    pub render_item: Callback<usize, Element>,
    /// How off-screen rows are kept cheap. Defaults to [`VirtualListMode::Auto`].
    #[props(default = ReadSignal::new(Signal::new(VirtualListMode::Auto)))]
    pub virtualize: ReadSignal<VirtualListMode>,
    /// With [`VirtualListMode::Auto`]: the largest `count` that keeps every row in the DOM.
    /// Defaults to [`DEFAULT_AUTO_THRESHOLD`] (1000).
    #[props(default = ReadSignal::new(Signal::new(DEFAULT_AUTO_THRESHOLD)))]
    pub auto_threshold: ReadSignal<usize>,
    /// Additional attributes to apply to the container element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props the two mode bodies share.
#[derive(Props, Clone, PartialEq)]
struct ListBodyProps {
    count: ReadSignal<usize>,
    buffer: ReadSignal<usize>,
    estimate_size: Option<Callback<usize, u32>>,
    render_item: Callback<usize, Element>,
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
}

/// # VirtualList
///
/// The `VirtualList` component renders a long list cheaply, in one of two modes (see the
/// module documentation for the full trade-offs): every row in the DOM with the browser skipping
/// the off-screen ones ([`VirtualListMode::ContentVisibility`]), or only a window of rows mounted
/// over a full-height canvas ([`VirtualListMode::Windowed`]). By default
/// ([`VirtualListMode::Auto`]) it picks the first up to [`DEFAULT_AUTO_THRESHOLD`] rows and the
/// second above.
///
/// Each row is a `role="listitem"` with `aria-setsize` and `aria-posinset`, in both modes, so a
/// screen reader announces the list's real size even when only a window of it is in the DOM.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::virtual_list::{VirtualList, VirtualListMode};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         VirtualList {
///             count: 100usize,
///             // Optional: the height guess for rows that have not been measured/rendered.
///             estimate_size: |_idx| 48,
///             // Optional: `Auto` (the default) picks by `count`.
///             virtualize: VirtualListMode::ContentVisibility,
///             render_item: move |idx: usize| rsx! {
///                 article { "Row {idx}" }
///             },
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The container is a `div` the caller makes a scroll container (give it a height or
/// `max-height` and `overflow-y: auto`); the styled layer adds the class
/// `dx-virtual-list-container`. All user-provided `attributes` are spread onto it, except `id`,
/// `role` and `tabindex`, which the list owns. The container carries `data-virtualize`
/// (`"content-visibility"` or `"windowed"`) for styling and tests.
#[component]
pub fn VirtualList(props: VirtualListProps) -> Element {
    let VirtualListProps {
        count,
        buffer,
        estimate_size,
        render_item,
        virtualize,
        auto_threshold,
        attributes,
    } = props;

    // The mode body is a separate component, so switching modes (a count crossing the
    // threshold, or a new `virtualize`) drops the old body's scope and everything it owns (the
    // windowed engine, its scroll listeners) with it. A memo, so only a *change of mode*
    // re-renders this component, not every count change.
    let windowed = use_memo(move || {
        virtualize().resolve(count(), auto_threshold()) == VirtualListMode::Windowed
    });

    if windowed() {
        rsx! {
            VirtualListWindowed {
                count,
                buffer,
                estimate_size,
                render_item,
                attributes,
            }
        }
    } else {
        rsx! {
            VirtualListContentVisibility {
                count,
                buffer,
                estimate_size,
                render_item,
                attributes,
            }
        }
    }
}

/// Every row in the DOM, off-screen ones skipped by the browser. No hooks beyond an id, no
/// effects, no listeners: a scroll costs the page nothing but the browser's own work.
#[component]
fn VirtualListContentVisibility(props: ListBodyProps) -> Element {
    let ListBodyProps {
        count,
        estimate_size,
        render_item,
        attributes,
        ..
    } = props;

    let container_id = use_unique_id();
    let owned = attributes!(div {
        id: container_id,
        role: "list",
        tabindex: "0",
        "data-virtualize": "content-visibility",
    });
    let merged = merge_attributes(vec![attributes, owned]);

    let rows = count();
    let set_size = rows.to_string();

    // The browser skips layout and paint for a chunk of rows that is off-screen and remembers
    // its real height afterwards (`auto`); until it has rendered the chunk it assumes the sum of
    // the rows' estimates. A chunk that holds focus or an end of the selection is never skipped
    // (the browser's "relevant to the user" rule), and `window.find` un-skips the chunk holding
    // a match.
    let skip = ChunkSkip::Inline {
        estimate: estimate_size,
        fallback: DEFAULT_SIZE,
    };

    rsx! {
        div { ..merged,
            {cv_chunks(rows, 0, skip, &[], |idx| rsx! {
                div {
                    key: "{idx}",
                    role: "listitem",
                    "data-virtual-index": "{idx}",
                    "aria-setsize": "{set_size}",
                    "aria-posinset": "{idx + 1}",
                    {render_item(idx)}
                }
            })}
        }
    }
}

/// Only the rows in view (plus a buffer) mounted over a full-height canvas.
#[component]
fn VirtualListWindowed(props: ListBodyProps) -> Element {
    let ListBodyProps {
        count,
        buffer,
        estimate_size,
        render_item,
        attributes,
    } = props;

    let container_id = use_unique_id();

    // Non-reactive state: scroll position, measured sizes, the mounted range. `rev` is the one
    // reactive signal, bumped by `publish` only when the engine says a render would draw
    // something different.
    let engine = use_hook(|| CopyValue::new(Engine::new()));
    let rev = use_signal(|| 0u64);

    // The estimates are evaluated in a memo so that signals the `estimate_size` callback reads
    // are tracked, as they always were. Measurements are *not* an input: they update the engine
    // in `O(log N)` instead of rebuilding anything.
    let estimates = use_memo(move || {
        let estimate = estimate_size.as_ref().map(|c| move |i: usize| c(i));
        SizeEstimates::evaluate(
            count(),
            estimate.as_ref().map(|f| f as &dyn Fn(usize) -> u32),
        )
    });

    let publish = move || {
        let (mut engine, mut rev) = (engine, rev);
        if engine.with_mut(|engine| engine.needs_render()) {
            let next = rev.peek().wrapping_add(1);
            rev.set(next);
        }
    };

    // Subscribe to scroll events via a JS bridge, and unsubscribe when the list unmounts.
    use_effect_with_cleanup(move || {
        let mut eval = document::eval(SCROLL_BRIDGE_JS);
        let _ = eval.send(container_id.peek().clone());

        spawn(async move {
            while let Ok(scroll_msg) = eval.recv::<ScrollMsg>().await {
                let mut engine = engine;
                let correction = engine.with_mut(|engine| {
                    engine.scroll(
                        scroll_msg.offset,
                        scroll_msg.viewport,
                        scroll_msg.is_scrolling,
                    )
                });

                if let Some(delta) = correction {
                    let new_scroll = (scroll_msg.offset as i32 + delta).max(0) as u32;
                    sync_container_scroll(container_id.peek().clone(), new_scroll).await;
                    engine.with_mut(|engine| engine.set_scroll_offset(new_scroll));
                }
                publish();
            }
        });

        // The bridge holds the container's scroll listener and the window's resize listener,
        // both owned by the page, not by this scope: one message tells it to remove them.
        move || {
            let _ = eval.send("teardown");
        }
    });

    let onresize = move |idx| {
        move |event: Event<ResizeData>| {
            let rect = event.data().get_content_box_size().unwrap_or_default();
            let measured = rect.height.max(1.0).round() as u32;

            let mut engine = engine;
            let (adjustment, current) = engine
                .with_mut(|engine| (engine.resize_item(idx, measured), engine.scroll_offset()));
            publish();

            if let Some(delta) = adjustment {
                let new_scroll = (current as i32 + delta).max(0) as u32;
                spawn(async move {
                    sync_container_scroll(container_id.peek().clone(), new_scroll).await;
                });
            }
        }
    };

    // Subscribe: this render runs again whenever `publish` bumps `rev`.
    let _ = rev();
    let plan = {
        let estimates = estimates.read();
        let mut engine = engine;
        engine.with_mut(|engine| {
            engine.sync(&estimates);
            engine.plan(buffer())
        })
    };
    let set_size = count.to_string();

    // `id` is owned: `sync_container_scroll` above looks this element up by
    // id (a JS eval), so a caller override would silently break scroll sync
    // (backlog row 93 names this site explicitly). `role`/`tabindex` are
    // this container's own widget semantics, also owned.
    let owned = attributes!(div {
        id: container_id,
        role: "list",
        tabindex: "0",
        "data-virtualize": "windowed",
    });
    let merged = merge_attributes(vec![attributes, owned]);

    rsx! {
        div {
            ..merged,

            div {
                style: "position: relative; height:{plan.canvas_height}px; width: 100%;",
                div {
                    style: "position: absolute; inset: 0 auto auto 0; width: 100%; transform: translateY({plan.top_offset}px); will-change: transform;",
                    {plan.items.iter().map(move |item| {
                        let idx = item.index();

                        rsx! {
                            div {
                                key: "{item.key()}",
                                role: "listitem",
                                "data-virtual-index": "{idx}",
                                "aria-setsize": "{set_size}",
                                "aria-posinset": "{idx + 1}",
                                onresize: onresize(idx),
                                {render_item(idx)}
                            }
                        }
                    })}
                }
            }
        }
    }
}

/// The scroll bridge for windowed mode: reports the container's scroll state to Rust and, on
/// the second message from Rust, removes every listener and timer it installed.
const SCROLL_BRIDGE_JS: &str = r#"
    const container = document.getElementById(await dioxus.recv());
    if (!container) return;

    let scrollEndTimer = null;
    let lastOffset = null;
    let lastViewport = null;
    let lastIsScrolling = null;

    function publish(isScrolling) {
        const scroll = Math.round(container.scrollTop);
        const viewport = Math.min(container.clientHeight, window.innerHeight) || 600;
        // Deduplicate only if the full scroll state is unchanged.
        if (
            scroll === lastOffset &&
            viewport === lastViewport &&
            isScrolling === lastIsScrolling
        ) {
            return;
        }
        lastOffset = scroll;
        lastViewport = viewport;
        lastIsScrolling = isScrolling;
        dioxus.send({
            offset: scroll,
            viewport: viewport,
            isScrolling: isScrolling
        });
    }

    function onScroll() {
        // Clear any pending scroll-end detection
        if (scrollEndTimer !== null) {
            clearTimeout(scrollEndTimer);
        }

        // Send scroll event immediately (no RAF batching)
        // This ensures Rust receives the event before the next render.
        // Rust turns most of these into no render at all (see virtual/virtualizer.rs).
        publish(true);

        // Debounce scroll-end detection. Firefox in CI can take long
        // enough between scroll events and measurement reads that a
        // shorter timeout unfreezes the scroll canvas mid-scroll.
        scrollEndTimer = setTimeout(() => {
            scrollEndTimer = null;
            publish(false);
        }, 600);
    }

    // Named, so it can be removed below.
    function onResize() {
        publish(false);
    }

    // Initial publish
    publish(false);

    container.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onResize, { passive: true });

    // Held until the list unmounts: Rust sends one message from its cleanup.
    await dioxus.recv();
    if (scrollEndTimer !== null) clearTimeout(scrollEndTimer);
    container.removeEventListener("scroll", onScroll);
    window.removeEventListener("resize", onResize);
"#;

/// Parsed scroll message from JS bridge.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrollMsg {
    offset: u32,
    viewport: u32,
    is_scrolling: bool,
}

async fn sync_container_scroll(container_id: String, scroll_top: u32) {
    let eval = document::eval(
        r#"
        const id = await dioxus.recv();
        const targetScroll = await dioxus.recv();
        const container = document.getElementById(id);
        if (container) {
            container.scrollTop = targetScroll;
        }
        "#,
    );
    let _ = eval.send(container_id);
    let _ = eval.send(scroll_top);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_picks_by_count_and_threshold() {
        let auto = VirtualListMode::Auto;
        assert_eq!(auto.resolve(0, 1_000), VirtualListMode::ContentVisibility);
        assert_eq!(
            auto.resolve(1_000, 1_000),
            VirtualListMode::ContentVisibility
        );
        assert_eq!(auto.resolve(1_001, 1_000), VirtualListMode::Windowed);
        assert_eq!(auto.resolve(100, 10), VirtualListMode::Windowed);
    }

    #[test]
    fn an_explicit_mode_ignores_the_count() {
        for count in [0, 1, 1_000, 1_001, 1_000_000] {
            assert_eq!(
                VirtualListMode::ContentVisibility.resolve(count, 1_000),
                VirtualListMode::ContentVisibility
            );
            assert_eq!(
                VirtualListMode::Windowed.resolve(count, 1_000),
                VirtualListMode::Windowed
            );
        }
    }

    #[test]
    fn the_default_is_auto_and_resolve_never_returns_it() {
        assert_eq!(VirtualListMode::default(), VirtualListMode::Auto);
        for count in [0, 1, 999, 1_000, 1_001, usize::MAX] {
            for mode in [
                VirtualListMode::Auto,
                VirtualListMode::ContentVisibility,
                VirtualListMode::Windowed,
            ] {
                assert_ne!(
                    mode.resolve(count, DEFAULT_AUTO_THRESHOLD),
                    VirtualListMode::Auto
                );
            }
        }
    }

    #[test]
    fn wire_spellings_match_message_scroller() {
        assert_eq!(
            VirtualListMode::ContentVisibility.as_str(),
            "content-visibility"
        );
        assert_eq!(
            VirtualListMode::ContentVisibility.as_str(),
            crate::message_scroller::Virtualization::ContentVisibility.as_str()
        );
        assert_eq!(VirtualListMode::Windowed.as_str(), "windowed");
        assert_eq!(VirtualListMode::Auto.as_str(), "auto");
    }
}
