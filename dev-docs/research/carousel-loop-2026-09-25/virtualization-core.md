<!-- Research notes written 2026-09-25 by a read-only research lane (round 8 follow-up). Point-in-time; source SHAs cited inline. -->

# Can `virtual_list` and a seamless-loop carousel share one virtualization core?

## 1. `virtual_list` today

**Files:** `primitives/src/virtual_list.rs` (component), `primitives/src/virtual/{virtualizer,types,utils}.rs` (pure core), `preview/src/components/virtual_list/**` (demo), `playwright/virtual_list.spec.ts`.

**API:** children-less, data-driven: `count: ReadSignal<usize>`, `buffer: ReadSignal<usize>` (overscan, default 8), `estimate_size: Option<Callback<usize, u32>>`, `render_item: Callback<usize, Element>`. One axis: vertical only (`scrollTop`/`clientHeight`, translateY). No RTL/orientation prop.

**Sizing:** dynamic/measured by default, `ResizeObserver` (`onresize` per rendered item) feeds `item_size_cache: HashMap<usize, u32>` keyed **by absolute index**, not a stable content id. Falls back to `estimate_size` or an adaptive average of measured sizes (100px if nothing measured yet). `compute_measurements` (pure, unit-tested) rebuilds a `Vec<VirtualItem>{key, index, start, size}` from the cache every time `count` or the cache changes (a `use_memo`).

**Window computation:** `calculate_range` binary-searches `measurements` for the item at `scroll_offset`, walks forward until `end >= scroll_offset+viewport`, then `default_range_extractor` pads by `overscan` and clamps to `[0, count-1]`. Pure functions, real `#[test]`s (no browser) via a `with_runtime` harness that drives a `VirtualDom` so `Store`/`Memo` are usable.

**Positioning:** absolute-position + transform sizer, not spacer children — outer `div` sized to `total_height` (`get_total_size`, frozen via `stable_total_size` while `is_scrolling` to stop scrollbar-thumb drift on resize churn — this freeze is a virtual_list-only refinement, not present in the shared window math), inner `div` gets `transform: translateY(first_visible.start)`, and only the windowed items render inside it. No spacer elements in the DOM at all.

**Scroll reading:** JS `document::eval` bridge on `#container` listens to native `scroll` (immediate) + debounced `scrollend`/timeout (600ms) → posts `{offset, viewport, isScrolling}` back over a channel; Rust never reads `scrollTop` outside that bridge. A correction (from resize-triggered scroll adjustment) is applied by **writing** `container.scrollTop` directly in a second `eval` — this happens only when idle, but it is a bona fide scroll-position write, unlike the carousel's physical-delta convention.

**Keying / a11y:** DOM key is `item.key()` = absolute data index (not slot position) — Dioxus mounts/unmounts elements as they scroll in/out of the window, it does not reuse DOM nodes across index changes. Each item gets `role="listitem"`, `aria-setsize`, `aria-posinset` (1-based). Container: `role="list"`.

**SSR/hydration:** first render has `viewport_size: 0` (only set by the JS effect post-mount) → `calculate_range` returns `None` → **zero items render** pre-hydration. The canvas div (with computed height) is present but empty until JS runs.

**Known bug class (backlog row 2.2 / `sarendipitee@799a4ff3`, still guarded by regression tests):** holding a `Memo::peek()` guard (measurements) across a call that mutates one of that memo's own dependencies (`item_size_cache`, via `resize_item`) — fixed by snapshotting (`measurements.peek().clone()`) before the mutating call. Two tests pin this: `test_resize_item_while_measurements_memo_peek_held` (Rust) and the Playwright "resize churn while scrolling does not panic" browser fallback oracle.

## 2. Shared vs. distinct with the carousel

**Genuinely shared — a pure "window math" module:** given `(item_count, anchor, radius/overscan, wrap: bool)` → the ordered list of `(data_index, slot, key)` to render. `calculate_range` + `default_range_extractor` + a `mod(i, n)` wrap mode is exactly this, generalized. It is pure, deterministic, and directly unit-testable without a DOM — `virtual_list`'s own `virtualizer.rs` tests are the template. Keyed rendering (so mounted components survive window movement) is shared in principle, but the *key scheme* is not: see below.

**Must differ:**
- **Anchor derivation.** List: continuous `scroll_offset` read from a live scroll bridge. Carousel: a discrete *settled snap index* only computed at idle (scrollend/debounce) — the list never has an "idle" concept, it re-derives the window on every scroll tick.
- **Key scheme.** List keys by **data index** — items mount/unmount as the window slides. The carousel's bench prototype instead keeps DOM nodes **and slot identity fixed** (keys by slot 0..2R) and repaints their content on rotation, because it needs the *same element* alive across the idle-time correction (`offsetOf(target)` reads a physical rect from an element that must not have just been unmounted/remounted, and CSS scroll-snap wants continuity of the scroll target). A Dioxus port must key `CarouselItem`s by slot, not by `data_index`, or every rotation triggers mount/unmount churn exactly where the design demands DOM continuity.
- **Position maintenance.** List: sizer + translateY, recomputed every render off measured/estimated sizes; correction is applied via a genuine `scrollTop` write (safe because it never fights a live scroll — THE RULE's "no native scrolling in flight" case, but virtual_list doesn't currently reason about it that way). Carousel: no sizer at all — native scroll-snap owns positioning during the gesture; rotation + a `scrollBy` **physical-delta** correction happens only at idle, with `scroll-snap-type` suspended for that one step.
- **The list's positioning approach would violate carousel invariant 5** (no spacer children — they enter the snap candidate list) if reused naively; the carousel's rotation approach (no sizer, no total-size canvas) is inapplicable to the list's free/continuous scrolling, which has no fixed "N" to wrap and needs a real scrollable extent.
- The list never writes mid-scroll either, but it *does* write scroll position at idle-after-resize — same idle-write allowance the carousel relies on, so this constraint is compatible, not identical.

## 3. Carousel-specific edge cases the shared core must handle

N=1/N=2/N<2R+1: window must de-duplicate — same data index can appear in two window slots simultaneously; slot-keyed rendering handles this for free (each slot still gets a unique key), but a11y ("k of N") must read the *data* index, not the slot, and must collapse correctly when N shrinks below the window. Dynamic N: window math must reclamp `current` and rebuild in one step, mirroring `test_range_clamps_stale_scroll_offset_after_count_shrinks`. `loop=false`: wrap off — window math clamps at `[0, N-1]`, existing rubber-band/edge-clamp logic (already in `carousel.rs`) applies unchanged since it's an orthogonal concern (transform-based overdrag, not windowing). RTL/vertical: window math itself is axis-agnostic (indices only); physical-delta measurement (`getBoundingClientRect`, left vs top) already generalizes per `carousel.rs`'s existing orientation handling. Multi-slide-per-view (`basis-1/3`): radius scales with visible-slides-per-view, not fixed at 1; anchor becomes "first fully visible slide," not a single centered index. Tablist/dot picker: maps dot index → data index directly (already how `CarouselApi`/`scroll_to` works); with a small N stays 1:1, with wrap needs `mod`. Autoplay: unaffected by windowing — it already drives `step_next`, which the seamless mechanism intercepts at the "which physical child to scroll to" layer. SSR: unlike `virtual_list` (renders nothing pre-hydration), the carousel must render the initial window (indices `-R..=R` mod N around index 0) on the server, matching the bench's `centreNow()` — a real behavioral divergence from `virtual_list`'s "empty until JS" SSR story that the shared core needs to parameterize, not inherit.

## 4. Recommendation

**(a) Generalize now, at a new pure module,** `primitives/src/virtualize.rs` (or `primitives/src/virtual/window.rs` beside the existing core): extract window math with a `wrap: bool` parameter and a pluggable key scheme (index-keyed vs slot-keyed), zero behavior change to `virtual_list`. Do this **before** building the carousel adapter — it's the low-risk, well-tested half (mirrors `compute_measurements`/`calculate_range`'s existing pure-function/unit-test pattern) and gives the carousel work a validated foundation instead of a shared guess.

**(b) API shape**, matching `virtual_list`'s existing data-driven convention rather than introducing a new pattern:
```rust
CarouselVirtual<T: Clone + PartialEq> {
    items: ReadSignal<Vec<T>>,          // or count + render_item like virtual_list, if items live outside Dioxus state
    render_item: Callback<(usize, T), Element>,
    r#loop: ReadSignal<bool>,           // wrap on/off, reuses existing prop name
    radius: ReadSignal<usize>,          // slides-per-side kept mounted (2R+1 window)
    // orientation, draggable, autoplay, etc. stay as on today's Carousel
}
```
This is additive, not a replacement for `render_item: Callback<usize, Element>` if items are better left owned by the caller (matches `virtual_list`'s own choice of index-in, element-out rather than owning a `Vec<T>`).

**(c) Keep the existing children-based `Carousel`/`CarouselContent`/`CarouselItem`** for the non-loop, small-fixed-slide-count case — it's simpler, has no windowing cost, and rewind-loop is a legitimate distinct behavior (not obsoleted by seamless loop). `CarouselVirtual` is a new, parallel entry point for data-driven seamless-loop use, not a rewrite.

**(d) Effort/sequence:** S — extract window-math core from `virtual_list`, zero behavior change (unit tests carry over). M — build the carousel adapter (slot-keyed rendering, idle-only rotation + physical-delta correction, SSR initial window) reusing the extracted core plus the already-approved bench mechanism. L — full generalization is not recommended in one step; the anchor-derivation and position-maintenance halves are different enough that forcing one trait/interface over both now risks the "two occurrences ≠ shared abstraction, forced early" trap. Sequence: (1) extract pure window math with `wrap` param + tests, (2) port `CarouselVirtual` against it using the bench's proven idle-rotation mechanism, (3) only after both exist, revisit whether the anchor/position halves also warrant a shared trait — likely not, per CLAUDE.md's own bar ("two or more occurrences" of a *problem*, not of a *shape*).

**(e) Risks:** slot-vs-index keying is easy to get backwards and would silently reintroduce mount/unmount churn the bench specifically avoided; SSR divergence (empty-until-JS vs render-initial-window) must be deliberate, not accidental, or carousel SSR breaks; N<2R+1 duplicate-key a11y correctness (posinset must reflect data index, not slot) needs its own test; reusing `virtual_list`'s scrollTop-write correction path for the carousel would violate THE RULE if applied while momentum could still be live — the carousel's idle-detection (scrollend/debounce) is stricter than virtual_list's and must not be weakened to "unify" the two.
