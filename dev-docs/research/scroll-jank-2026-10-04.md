# Scroll jank on the overview page (`/`) -- investigation, 2026-10-04

Owner report: "Scrolling the full overview demo page can feel a bit janky ... why is that?"

Status: sections 0-6 are the investigation (no file changed at that point; the diffs in section 5 were proposals). The fixes landed uncommitted
in the SCROLL-FIX lane, with one correction to the Cause 1 mechanism: see section 7. Reusable harness: `scripts/measure-scroll.mjs`.

## 0. Verdict

The page is not janky because of anything scroll-triggered: scrolling runs no handlers, makes no layout reads, writes
no signals and mutates no DOM. It is janky because **the main thread is never idle and every scroll frame drags it in**:

1. **Leaked, never-cleared 100 ms ticker (`BlockPlayer`, plus the Progress demo's 1 s one)** -- a new `setInterval` every
   ~1.2 s that nothing clears (128 live after ~100 s, 1,326 timer wakeups/s at 155 s), and even a single 100 ms ticker
   re-lays-out the multi-column masonry 10x/s. Layouts during one scroll: 387 vs 0-1 without it.
2. **The main thread joins every scroll frame, and the compositor waits for it**: 8 infinite CSS animations (6 skeleton
   pulses, 2 spinners; not compositor-driven) force a style recalc + pre-paint + layerize + commit per vsync (-60% main
   busy when paused), and `Input`'s unconditional `onwheel` (+ three `ontouchstart: prevent_default`) makes Dioxus put
   non-passive wheel/touchstart listeners on `#main`, so every wheel tick is dispatched to the main thread first
   (wheel latency max 388 vs 143 ms under a busy main thread).
3. **No containment**, so each of those frames is O(page): `PrePaint` walks 4.4k nodes (1,851 ms -> 135 ms with
   `content-visibility: auto` on the cards, -63% total busy).

Head-to-head, same box and minute, 3-run medians, page age 20 s at scroll start (`final-*` rows in section 4):

| | idle main ms/s | scroll main busy | layouts | style recalcs | tasks > 16 ms | wheel p50/p95/max ms | live `setInterval`s |
|---|---|---|---|---|---|---|---|
| as shipped | 249 | 8,146 ms | 387 | 1,196 | 25 | 39 / 61 / 145 | 128 |
| A+B+C(`content-visibility`) via harness stubs | 2.4 | 1,385 ms (-83%) | 119 (one per card entering view) | 469 | 2 | 28 / 34 / 50 | 3 (2 live) |
| A+B+C(`contain: layout paint style`) | 1.3 | 3,015 ms (-63%) | 0 | 778 | 10 | 26 / 36 / 80 | 3 (2 live) |

The fix rows use the harness's stubs (`leak,tick1hz,nowheel` + `cv`/`contain`) as an upper bound of what the proposed
constructions in section 5 buy; they are not the real fixes and have not been built.

## 1. Which page, how measured

* "Overview demo page" is the home route `/` (`Route::Home`, `preview/src/main.rs`): hero, then
  `WidgetMasonry` (14 live "sample interface" blocks in a CSS multi-column container,
  `column-count: 3`, `preview/assets/main.css:1238` at HEAD), then `ComponentGallery` (70 live demos, one
  `.dx-component-card` per `components::DEMOS` entry except `top_layer`).
  `/demos` is a one-thumbnail hub (email-client) and has nothing to scroll.
* At 1280x800 the page is 29,234 px tall (36 viewports), 4,361 DOM nodes, 1,051 of them SVG
  (251 `<svg>`), 73 compositor layers (one 1265x29234 content layer).
* Build: release, no base path, `main` source at the time (`/home/user/tgt/tip/dx/preview/release/web/public`,
  served by `python3 -m http.server 8090`). Chromium 1194 headless, software compositing.
* Method: `node scripts/measure-scroll.mjs http://127.0.0.1:8090/ ...` scrolls top to bottom with real
  CDP wheel input (288 ticks of 100 px, 16 ms apart), records a CDP trace (`devtools.timeline`, `cc`,
  `input`, `latencyInfo`, `benchmark`) and `Performance.getMetrics` for the scroll window only, plus an
  idle window and an inventory of listeners / observers / timers / layout reads installed by an
  `addInitScript` shim. Suspects are switched off one at a time (`--disable ...`) on the same box.
* Caveat that shapes how to read every number: the 4-core box was shared with other lanes' cargo/dx
  builds (load average 8-28 during the runs) and Chromium here composites in software. Absolute ms are
  inflated and noisy (the same scenario varied by up to 40% between runs); compare COUNTS (layouts,
  style recalcs, timer fires, input blocked on main) and ratios inside one batch.

## 2. What the page does NOT do (suspects cleared with numbers)

| Suspect | Evidence |
|---|---|
| Scroll / resize listeners doing layout reads | No `scroll` listener on `window`/`document` at all. Only two private scrollers register `scroll` (virtual list, carousel), both `passive: true`, and page scroll never fires them. During a full scroll: `getBoundingClientRect` 0, `getClientRects` 0, `getComputedStyle` 0, `offsetWidth/Height/...` 0, rAF calls 0, `ResizeObserver` fires 0, `IntersectionObserver` fires 0. |
| `use_anchor_position_fallback` (`primitives/src/top_layer.rs:1416` at HEAD) always-on tracking | It returns early while the overlay is closed (`if !open.cloned() { return ... }`), so closed overlays install nothing. Measured: 0 layout reads during scroll (above). Only an OPEN overlay tracks scroll/resize. |
| Dioxus signal writes / re-render caused by scroll | A `MutationObserver` on `document.body` records 0 mutations during a 280-tick scroll and 0 during a 2 s idle (with timers neutralised). Chart SVGs are not re-rendered by scrolling. |
| Charts / SVG weight | Hiding every non-icon `<svg>` (`--disable svg`): main-thread busy 3,874 ms vs 3,762 ms (no gain). Chart entrance animations are one-shot (500-1500 ms) and finished before any scroll. |
| `backdrop-filter` / `filter` | 0 elements on the page (`dialog/style.css:10` is the only `backdrop-filter` in the repo, and only while open). |
| Large shadows / `will-change` | 77 shadowed and 10 `will-change` elements. `--disable shadows,willchange`: no improvement in any count. |
| `scroll-behavior: smooth` (`main.css:60` at HEAD) | Does not apply to wheel/touch scrolling, only to programmatic and anchor jumps. `--disable smooth`: 8,397 ms / 373 layouts vs 9,592 ms / 401 baseline in the same batch (within noise). |
| Raster / paint area | Raster is cheap: ~529 raster tasks, ~0.4 s total for 200 ticks. The 8.9 ms/frame `DirectRenderer::DrawFrame` is the headless software compositor, not representative of a GPU browser. Layout shift (CLS) during scroll: 0. |
| Carousel scroll-snap handlers | `dx-carousel-content` registers `scroll`/`scrollend`/`wheel`/`touchstart`/`touchend` as `passive: true`; horizontal only; silent during page scroll. |

## 3. Causes found (ranked by measured contribution)

### Cause 1 -- timers that are never cleaned up, and a 10 Hz re-layout that exists even when they are

`BlockPlayer` (`preview/src/main.rs:2431`, HEAD) starts its ticker with
`use_effect(|| { document::eval("setInterval(() => dioxus.send(performance.now()), 100)") ... })` and never
calls `clearInterval`. The Progress demo (`preview/src/components/progress/variants/main/mod.rs:10`) has the
same shape at 1000 ms.

* The player's effect re-runs about every 1.2 s once ticks flow. The exact subscription was not pinned to one
  line (the effect body reads nothing; its spawned task reads `playing()` / `progress_seconds()` and writes
  `progress_seconds`, and `dioxus-hooks` 0.7.9 `use_effect.rs:25` documents "the effect is rerun due to an async
  read at any time"); the Progress demo, whose task only `write()`s, does not re-run. Each re-run creates
  a NEW `setInterval` plus a NEW task, and nothing ever clears the old ones. Evidence: blocking all
  100 ms intervals stops the creation entirely; allowing only the first one lets exactly one re-run
  through, then none; blocking the 1000 ms Progress interval changes nothing; the DOM nodes are not
  re-mounted (identity preserved).
* Idle cost by page age (no scrolling at all, `/tmp` probe, same box, same minute):

  | page age | live intervals (as created) | timer fires/s | script ms/s | main ms/s (box-inflated) |
  |---|---|---|---|---|
  | 8 s | 9 | 58 | 20 | 237 |
  | 25 s | 24 | 204 | 57 | 248 |
  | 50 s | 46 | 423 | 76 | 249 |
  | 95 s | 84 | 803 | 91 | 267 |
  | 155 s | 135 | 1,326 | 104 | 272 |
  | 155 s with ONE interval per period (leak fixed) | 3 created (2 live) | 11 | 6 | 95 |

  Layout/paint work stays frame-capped (about 20 layouts/s) but the page is never idle and gets steadily
  worse the longer it stays open: by 155 s there are 1,326 timer wakeups per second.
* Even with the leak fixed, the 100 ms ticker alone costs 18 layouts/s and ~86 ms/s of main thread at
  idle, because every tick moves the slider thumb (`left`/`style`) inside a multi-column container.
  Moving the ticker to 1 Hz (the slider has `step: 1.0` and the label shows whole seconds) takes idle to
  0-2 layouts per 2 s.
* During a full scroll: as shipped 387 layouts (b5; 386 in b1). With the leak fixed but the 100 ms ticker kept: 280 layouts
  and 6,780 ms main busy (b4 `r7`); with no ticker: 0 layouts and 5,409 ms (b4 `r0`); with the ticker at 1 Hz and
  nothing else changed: 1 layout, 5,147 ms (b2 `c1`, single run). Idle main-thread cost: 249 ms/s as shipped,
  110 ms/s leak-fixed, 11 ms/s leak-fixed at 1 Hz, 0.1 ms/s with no ticker.

### Cause 2 -- scrolling drags the main thread into every frame

During a quiet scroll (tickers off) the main thread still produces a frame every vsync:
~870 `Layerize`/`Commit` pairs and ~1,150 style recalcs in 16 s, i.e. ~55 main frames/s, when pure
compositor scrolling would need none. Two things force that, and they multiply:

2a. **Infinite CSS animations.** 8 run on the page (6 `dx-skeleton-pulse`, 2 `dx-spinner-spin`). A trace
    with invalidation tracking attributes the recalcs to them one-for-one (3,546 + 1,182 `Animation`
    invalidations in 150 ticks); each alone is enough (skeletons off: 650 recalcs; spinners off: 635;
    all CSS animations off: 10). The per-frame recalc is tiny (0.2-0.3 ms, 2-8 elements); the cost is
    the pre-paint / layerize / commit that every such frame triggers.
    `--disable anim` on the quiet page (3-run medians): main busy 5,409 -> 2,187 ms (-60%), style recalcs
    1,163 -> 581, `PrePaint` 1,851 -> 42 ms, tasks > 16 ms 28 -> 12, dropped smooth frames 16 -> 2.
    (`animation: none` instead of paused drops the recalcs to ~10 in the invalidation trace.)

2b. **Root-level non-passive wheel/touch listeners.** Dioxus registers every bubbling rsx event handler
    once on `#main` with no options (`dioxus-interpreter-js` `createListener`:
    `this.root.addEventListener(event_name, this.handler)`), so any `onwheel` / `ontouchstart` in the app
    becomes a non-passive wheel/touchstart listener over the whole page. Sources:
    `preview/src/components/input/component.rs:67` (`onwheel: move |e| _ = onwheel.map(..)` is set on every
    `<input>` even when the prop is `None`), and `ontouchstart: evt.prevent_default()` in
    `primitives/src/slider.rs:843`, `primitives/src/resizable.rs:690`, `primitives/src/color_picker.rs:440`.
    CDP confirms `#main` wheel + touchstart are `BLOCKING`. Effect: every wheel tick is dispatched to the
    main thread (`DidHandleInputEventSentToMain` 40/40 ticks; 0 when the listeners are dropped), and the
    compositor waits for it. With a synthetic 50 ms main-thread burn every 150 ms (stand-in for
    layout/GC/the ticker), same scroll:

    | listeners | wheel latency p50 / p95 / max | tasks > 50 ms |
    |---|---|---|
    | as shipped (non-passive on `#main`) | 62 / 126 / 388 ms | 170 |
    | forced `passive: true` | 47 / 85 / 143 ms | 102 |
    | all wheel/touch listeners dropped | 43 / 103 / 251 ms | 112 |

    On a quiet main thread the listener costs nothing measurable (p95 59 vs 42-47 ms); it only bites when
    the main thread is busy, which is exactly what Cause 1 and 2a keep it. On touch devices the
    `touchstart` one additionally delays every scroll start.

### Cause 3 -- no containment, so each main frame costs O(page)

Every one of those main frames walks the whole 4.4k-node, 29k px tree in `PrePaint` (quiet page, 3-run
median: 1,851 ms for ~1,450 calls, ~1.3 ms each on this box; 42 ms once animations are off, so it is the animations
that make every frame pay it). Card-level containment turns that into O(card): `contain: layout` on
`.dx-component-card` / `.dx-widget-card` (84 cards) -> `PrePaint` 245 ms, `contain: paint` -> 217 ms,
`contain: style` alone -> 1,894 ms (no effect); busy 5,409 -> 4,616 / 4,082 ms (-15% / -25%; `Layerize` grows a little
because there are more paint chunks). With the 100 ms ticker still on, `contain: layout paint style` took `PrePaint`
2,075 -> 229 ms and busy 6,366 -> 4,188 ms (single run). `content-visibility: auto` is stronger because offscreen
cards stop participating at all: busy 5,409 -> 2,018 ms (-63%), tasks > 16 ms 28 -> 0, at the price of ~119 layouts as
cards enter view (CLS 0).

### Minor (UX, not frame cost) -- scroll latching in nested scrollers

With the pointer over the virtual-list demo (471x576 px, 196,011 px of content, the last card on the page)
20 wheel ticks scroll the list by 2,000 px and the page by 0 px; over the 160x160 px scroll-area demo the
list takes ~8 ticks before the page moves again. Pointer over the page body scrolls normally.

## 4. Scenario table

All rows: release build, `/`, 1280x800, 288 wheel ticks. "quiet" = `--disable intervals` (no tickers).
Single run unless noted; use the same-batch rows for comparison.

Batches (all same build, `/`, 288 wheel ticks of 100 px / 16 ms, pointer at x=40): **b1** first bisect with the leak live
(page age 60-70 s at scroll time, 2 runs, median); **b2/b3** single-run bisect on the "quiet" base; **b4** 3-run medians; **b5** back-to-back final head-to-head at page age 20 s, 3-run medians
(b4/b5 are the rows to trust). `--disable` names are the harness's stubs: `intervals` = no `setInterval`, `leak` = one interval per
period (what a clean-up fix gives), `tick1hz` = 100 ms tickers run at 1 s, `nowheel` = drop all page wheel/touch listeners,
`passive` = force them passive, `anim` = pause CSS animations, `contain` = `contain: layout paint style` on the cards,
`cv` = `content-visibility: auto` on the cards, `b*-busy` rows add a 50 ms main-thread burn every 150 ms. The wheel
columns are Chromium `EventLatency` (MOUSE_WHEEL; GESTURE_SCROLL_UPDATE for the `g*` fling rows). "dropped/smooth" is the
compositor's `PipelineReporter` verdict and is the least reliable column on a loaded box.

| batch | scenario (`--disable`) | runs | idle layouts/s | scroll main busy ms | layouts | style recalcs | tasks >16 / >50 ms | PrePaint ms | wheel p50/p95/max ms | dropped/smooth frames |
|---|---|---|---|---|---|---|---|---|---|---|
| b1 | baseline (`-`) | 2 | 28.7 | 7992 | 386 | 1218 | 32 / 3 | 1191 | 38/66/262 | 62/104 |
| b1 | intervals (`intervals`) | 2 | 0.0 | 4993 | 0 | 1142 | 10 / 0 | 1871 | 37/52/158 | 20/21 |
| b1 | leakfix (`leak`) | 2 | 12.7 | 7300 | 277 | 1196 | 47 / 5 | 2289 | 39/94/297 | 136/181 |
| b1 | passive (`passive`) | 2 | 25.0 | 9171 | 344 | 973 | 64 / 10 | 1511 | 36/142/299 | 194/240 |
| b1 | smooth (`smooth`) | 2 | 24.0 | 7454 | 372 | 1202 | 26 / 0 | 1142 | 38/66/172 | 52/88 |
| b2 | b0-busy-blocking (`intervals`) | 1 | 0.0 | 15350 | 1 | 1232 | 201 / 170 | 2222 | 62/126/388 | 128/129 |
| b2 | b1-busy-passive (`intervals,passive`) | 1 | 0.0 | 8777 | 1 | 792 | 113 / 102 | 1439 | 47/85/143 | 17/18 |
| b2 | b2-busy-nolisten (`intervals,scrolllisten`) | 1 | 0.0 | 16754 | 1 | 1237 | 228 / 176 | 2772 | 67/155/322 | 135/136 |
| b2 | c0-leakfix (`leak`) | 1 | 18.5 | 6366 | 270 | 1180 | 19 / 1 | 2075 | 39/64/191 | 81/119 |
| b2 | c1-leak+1hz (`leak,tick1hz`) | 1 | 1.0 | 5147 | 1 | 1188 | 14 / 1 | 1919 | 38/58/103 | 12/13 |
| b2 | c2-leak+contain (`leak,contain`) | 1 | 15.0 | 4188 | 267 | 1146 | 4 / 0 | 229 | 37/47/73 | 17/18 |
| b2 | q0-quiet (`intervals`) | 1 | 0.0 | 3762 | 1 | 1161 | 3 / 0 | 1481 | 38/47/71 | 8/9 |
| b2 | q2-anim (`intervals,anim`) | 1 | 0.0 | 1764 | 1 | 576 | 1 / 0 | 45 | 36/47/65 | 4/4 |
| b2 | q3-svg (`intervals,svg`) | 1 | 0.0 | 3874 | 1 | 1058 | 0 / 0 | 1494 | 36/47/62 | 9/9 |
| b2 | q4-shadow-wc (`intervals,shadows,willchange`) | 1 | 0.0 | 5363 | 1 | 1154 | 20 / 1 | 2098 | 38/61/162 | 24/25 |
| b2 | q5-contain (`intervals,contain`) | 1 | 0.0 | 4390 | 1 | 1147 | 14 / 2 | 228 | 34/55/114 | 9/9 |
| b2 | q7-ptr-right (`intervals`) | 1 | 0.0 | 3525 | 1 | 784 | 8 / 0 | 1463 | 30/39/59 | 7/8 |
| b3 | b2-busy-nowheel (`intervals,nowheel`) | 1 | 0.0 | 10415 | 1 | 839 | 137 / 112 | 1568 | 43/102/251 | 68/69 |
| b3 | f0-fixall-js (`leak,tick1hz,nowheel`) | 1 | 1.0 | 3951 | 1 | 870 | 33 / 7 | 272 | 29/100/375 | 105/106 |
| b3 | f1-fixall-js+contain (`leak,tick1hz,nowheel,contain`) | 1 | 1.0 | 2451 | 1 | 814 | 1 / 0 | 146 | 28/34/113 | 4/4 |
| b3 | f2-fixall-js+contain+cv (`leak,tick1hz,nowheel,contain,cv`) | 1 | 0.5 | 3104 | 120 | 744 | 25 / 6 | 185 | 27/89/274 | 51/52 |
| b3 | g0-gesture-baseline (`-`) | 1 | 32.0 | 2553 | 67 | 204 | 27 / 9 | 352 | 3/93/582 | 75/142 |
| b3 | g1-gesture-fixall (`leak,tick1hz,nowheel,contain`) | 1 | 0.5 | 1409 | 1 | 218 | 17 / 0 | 68 | 7/46/155 | 43/43 |
| b3 | q10-anim+contain (`intervals,anim,contain`) | 1 | 0.0 | 1993 | 1 | 579 | 5 / 0 | 46 | 36/53/116 | 10/10 |
| b3 | q11-cv (`intervals,cv`) | 1 | 0.0 | 2785 | 120 | 965 | 12 / 2 | 161 | 39/70/164 | 16/17 |
| b3 | q8-nowheel (`intervals,nowheel`) | 1 | 0.0 | 3686 | 1 | 761 | 15 / 0 | 1488 | 30/42/67 | 10/11 |
| b3 | q9-nowheel+anim (`intervals,nowheel,anim`) | 1 | 0.0 | 1402 | 1 | 304 | 5 / 0 | 41 | 27/47/118 | 1/1 |
| b4 | r0-quiet (`intervals`) | 3 | 0.0 | 5409 | 0 | 1163 | 28 / 1 | 1851 | 38/59/139 | 16/17 |
| b4 | r1-quiet+anim (`intervals,anim`) | 3 | 0.0 | 2187 | 0 | 581 | 12 / 3 | 42 | 35/55/230 | 2/2 |
| b4 | r2-quiet+contain-all (`intervals,contain`) | 3 | 0.0 | 4208 | 0 | 1167 | 19 / 0 | 225 | 36/48/173 | 3/3 |
| b4 | r3-quiet+contain-layout-style (`intervals`) | 3 | 0.0 | 4616 | 0 | 1156 | 15 / 0 | 245 | 37/50/112 | 16/17 |
| b4 | r4-quiet+contain-paint (`intervals`) | 3 | 0.0 | 4082 | 0 | 1136 | 13 / 1 | 217 | 36/51/139 | 6/6 |
| b4 | r5-quiet+contain-style (`intervals`) | 3 | 0.0 | 5120 | 0 | 1164 | 11 / 0 | 1894 | 37/48/112 | 13/14 |
| b4 | r6-quiet+cv (`intervals,cv`) | 3 | 0.0 | 2018 | 119 | 764 | 0 / 0 | 135 | 37/47/78 | 2/2 |
| b4 | r7-leakfix (`leak`) | 3 | 20.5 | 6780 | 280 | 1171 | 25 / 1 | 2149 | 38/61/144 | 46/77 |
| b4 | r8-leakfix+1hz+contain+nowheel (`leak,tick1hz,contain,nowheel`) | 3 | 1.0 | 2430 | 0 | 769 | 2 / 0 | 151 | 27/34/68 | 1/1 |
| b5 | final-shipped (`-`) | 3 | 31.5 | 8146 | 387 | 1196 | 25 / 1 | 1109 | 39/61/145 | 42/78 |
| b5 | final-fixall-cv (`leak,tick1hz,nowheel,cv`) | 3 | 0.0 | 1385 | 119 | 469 | 2 / 0 | 118 | 28/34/50 | 1/1 |
| b5 | final-fixall-contain (`leak,tick1hz,nowheel,contain`) | 3 | 0.0 | 3015 | 0 | 778 | 10 / 1 | 154 | 26/36/80 | 4/4 |


## 5. Proposed constructions (not applied; files belong to other lanes)

Line numbers are `git show HEAD:<file>` (other lanes are editing `main.css`/`main.rs`, so working-tree lines drift).
Each fix is a construction, not an instance patch: the class cannot recur because a hook/CSS rule/gate makes it
unrepresentable. Every diff below is a SKETCH that the owning lane must compile and run its specs against.

### A. Timers owned by the component (fixes Cause 1; subsumes 2 instances + future ones)

Instances subsumed: `BlockPlayer` (`preview/src/main.rs:2431`, ticker at `:2442`) and the Progress demo
(`preview/src/components/progress/variants/main/mod.rs:10`). Not subsumed: CSS animations (Cause 2a),
`setTimeout` chains.

1. `primitives/src/lib.rs` (next to `use_global_keydown_listener`, `:292`, which already has the exact shape
   "eval, `await dioxus.recv()`, tear down" and `use_effect_with_cleanup`, `:246`): add one hook whose
   interval cannot outlive or be duplicated by its owner.
```rust
/// Calls `on_tick(performance.now())` every `period_ms` while mounted. The JS interval is cleared
/// by the cleanup half of `use_effect_with_cleanup`, so an effect re-run replaces it instead of
/// stacking a second one. `on_tick` runs in a task: read signals with `.peek()`, never `()`.
pub fn use_interval(period_ms: u32, mut on_tick: impl FnMut(f64) + 'static) {
    use_effect_with_cleanup(move || {
        let mut eval = document::eval(&format!(
            "const id = setInterval(() => dioxus.send(performance.now()), {period_ms});
             await dioxus.recv();
             clearInterval(id);"
        ));
        spawn(async move { while let Ok(now) = eval.recv::<f64>().await { on_tick(now); } });
        move || _ = eval.send(true)
    });
}
```
2. `preview/src/main.rs` `BlockPlayer` (`:2441-2470`): replace the `use_effect(.. setInterval ..)` block. 1 Hz is enough
   (`Slider { step: 1.0 }`, label in whole seconds) and is 10x fewer wakeups; widen the elapsed clamp to match.
```rust
-    use_effect(move || {
-        let mut timer = document::eval("setInterval(() => { dioxus.send(performance.now()); }, 100);");
-        spawn(async move { ... playing() ... progress_seconds() ... });
-    });
+    let last_tick_ms = use_hook(|| CopyValue::new(None::<f64>));
+    use_interval(1000, move |now_ms| {
+        let elapsed = last_tick_ms.replace(Some(now_ms))
+            .map_or(0.0, |last| ((now_ms - last) / 1000.0).clamp(0.0, 1.5));
+        if !*playing.peek() { return; }
+        let current = progress_seconds.peek().unwrap_or(0.0);
+        progress_seconds.set(Some(if current >= TRACK_DURATION_SECONDS { 0.0 }
+            else { (current + elapsed).min(TRACK_DURATION_SECONDS) }));
+    });
```
   Progress demo (`progress/variants/main/mod.rs:6-20`): same swap,
   `use_interval(1000, move |now| progress.set((*progress.peek() + (now as usize) % 30) % 101));`.
3. Gate (repo convention, new `scripts/check-uncleared-intervals.sh`, add to the CLAUDE.md gate list):
   fail on any `setInterval(` inside a `*.rs` file under `preview/` or `primitives/` except `primitives/src/lib.rs`
   (the hook). ~20 lines of `grep`, same style as `check-self-subscribing-effects.sh`.
4. Oracle (the construction that covers the whole class, also for future timers): a counts-based spec, no
   timing, so it is valid on debug SSG builds. `playwright/oracle/tier2-html/scroll-main-thread.spec.ts`: init-script
   shim counts `setInterval` calls; load `/`, assert the count at t=3 s equals the count at t=15 s, and that
   idle `Performance.getMetrics().LayoutCount` grows by <= 3 over 2 s.

### B. No blocking wheel/touch listener on `#main` (fixes Cause 2b; subsumes 4 instances)

Why it is a class: Dioxus registers every bubbling rsx handler on `#main` with no options (cannot be passive), so
one `onwheel`/`ontouchstart` anywhere makes the whole page's wheel/touch input wait for the main thread.
Instances: `preview/src/components/input/component.rs:67`, `primitives/src/slider.rs:843`,
`primitives/src/resizable.rs:690`, `primitives/src/color_picker.rs:440`. Not subsumed: handlers that genuinely need
`prevent_default` on wheel (none today); the carousel's own `{passive: true}` JS listeners are the right pattern.

1. `Input` (`component.rs:37/67`): stop shimming every optional handler through an unconditional closure
   (`onwheel: move |e| _ = onwheel.map(|cb| cb(e))` registers a root wheel listener on every `<input>` even when the
   prop is `None`). Forward the `Option` itself, the way `primitives/src/navbar.rs:1045` carries
   `Option<EventHandler<MouseEvent>>`:
```rust
-            onwheel: move |e| _ = onwheel.map(|callback| callback(e)),
+            onwheel,
```
   If an element attribute will not accept `Option<EventHandler<_>>` directly, build the attribute only when `Some`
   and add it to `merged` (`merge_attributes`) instead. Same shim in `textarea/component.rs` (19) and
   `native_select/component.rs` (4): not scroll-blocking, but each is an always-registered listener per instance.
2. Delete the three `ontouchstart: move |evt| { evt.prevent_default(); }` blocks. Their only job is "don't focus the
   thumb"; `onmousedown { prevent_default }` directly above each already covers the compat mouse event a tap
   produces, and drags are pointer-driven with `touch-action: none` on the thumb/track
   (`slider/style.css:7`, `resizable/style.css:44`, `color_picker/style.css:55,108`). Verify with the touch-emulated
   (`hasTouch`) slider / resizable / color_picker specs before landing; if focus-steal on touch regresses, keep that one
   handler and add its tag to the allowlist below rather than reintroducing the pattern elsewhere.
3. Gate (new `scripts/check-blocking-scroll-listeners.sh`): fail on `^\s*(onwheel|onmousewheel|ontouchstart|ontouchmove)\s*:`
   in `*.rs` under `preview/src` and `primitives/src` unless the line carries `// blocking-ok: <reason>`.
4. Oracle: in the same spec as A.4, `DOMDebugger.getEventListeners` on `#main`, assert no `wheel`/`touchstart`/`touchmove`
   entry has `passive: false`.

### C. Bound the per-frame main-thread cost (fixes Cause 2a + 3)

Why: with any animation running, the main thread builds a frame per vsync even while scrolling, and an uncontained
page makes every one of those frames O(page) (`PrePaint` 1,851 ms quiet -> 42 ms with animations off -> 217-245 ms
with `contain: layout` or `contain: paint`). The construction is to make per-frame cost O(card) and to stop
offscreen cards from taking part at all.

1. `preview/assets/main.css` (HEAD `:1373` `.dx-component-card`, `:1243` `.dx-widget-card`, `:1257` `.dx-widget-card-popout`):
```css
/* Offscreen demo cards are skipped entirely (no style/layout/paint, their skeleton/spinner animations stop
   costing main frames); visible ones are an independent layout/paint/style boundary, so a ticker or pulse in
   one card never re-walks the 4.4k-node page. `auto` in contain-intrinsic-size remembers the real height after
   first render, so the scrollbar stops moving once a card has been seen. */
.dx-component-card,
.dx-widget-card:not(.dx-widget-card-popout) {
  content-visibility: auto;
  contain-intrinsic-size: auto 440px;
}
```
   Measured (3-run medians, tickers off): main busy 5,409 -> 2,018 ms (-63%), tasks > 16 ms 28 -> 0, dropped smooth
   frames 16/17 -> 2/2, `PrePaint` 1,851 -> 135 ms, `Layerize` 1,087 -> 85 ms; cost: ~119 layouts over the scroll (one
   per card as it enters view), CLS 0. (Measured with the popout cards included; they must still be excluded in the real rule, because
   `content-visibility: auto` implies paint containment, which would clip their dropdown.) Fallback if `content-visibility` is unwanted (it
   changes offscreen geometry that computed-style/visual specs read without scrolling): plain
   `contain: layout` (or `paint`) on the same selectors, `PrePaint` 1,851 -> 245 ms (217 for `paint`), busy -15% to -25%;
   `contain: style` alone does nothing (1,894 ms). Verify with the gallery visual / computed-style-snapshot specs.
2. Skeleton / spinner (`skeleton/style.css:3`, `spinner/style.css:11`): Chromium reports these as not run on the
   compositor (`compositeFailed=131072`, bit 17, not decoded here), so each costs a main frame per vsync for as long
   as it is rendered. Item 1 removes the offscreen cost; finding and removing whatever blocks compositing of a plain
   `opacity`/`transform` animation (untested suspects: the animated element sitting inside a grid/multi-column fragment, or having no own layer) would
   remove the on-screen cost too. Do not "fix" it by pausing the animations: that removes the component's visible
   behaviour from the page that exists to show it.
3. Regression oracle (covers every future always-on animation/timer/listener at once): in A.4's spec, scroll `/`
   top to bottom with `page.mouse.wheel` under a CDP trace and assert `Layout` count <= ~100 (one per card entering view with `content-visibility`; <= 5 with plain `contain`) and `UpdateLayoutTree`
   count under a recorded ceiling (today ~1,150; ~580 with the 8 animations paused, ~10 with `animation: none`).
   Counts, not ms, so it is load-proof and valid on a debug SSG build.

### Not proposed (cleared by measurement)
`scroll-behavior: smooth`, shadows, `will-change`, SVG chart weight, anchor-position tracking, carousel handlers and
observers: none moved any count (section 2). Nested-scroller latching (virtual-list / scroll-area demos) is a UX
note, not frame cost: `overscroll-behavior: contain` would make it worse, and CSS has no wheel-chaining threshold, so
the only construction is to not put a 196,011 px scroller at the bottom of the page the user is scrolling.


## 6. Re-measure

```bash
ln -s playwright/node_modules node_modules        # once; gitignored, same as scripts/inspect.mjs needs
python3 -m http.server 8090 -d <release public dir> &
node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --runs 3 --label after-fix --brief
# page-age growth (the leak): idle cost at 60 s / 150 s
node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --warmup 60 --runs 1 --brief
# the same suspects switched off by stub, for a before/after bound without rebuilding
node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --disable leak,tick1hz,nowheel,contain --runs 3 --brief
```
Pass criteria for a fix lane: idle layouts 0-2 per 2 s; live `setInterval`s constant over page age
(`--warmup 150`); scroll-window layouts 0-1 (about one per card entering view if `content-visibility: auto` is used), tasks > 50 ms = 0, `#main` wheel/touchstart `passive` or
absent (`--no-listeners` off prints them), `PrePaint` per main frame well under 0.5 ms.


## 7. Fix landed (SCROLL-FIX lane, 2026-10-04) -- results

Status: constructions A, B and C are implemented, gated and covered by an oracle; NOT committed. Numbers below are from the
shared debug `dx serve` build (:8083) against the release build of HEAD (:8090); a release re-measure of the final build is
pending (the main loop does it on the SSG release build). Compare COUNTS across the two builds, not milliseconds.

### 7.1 The exact leak subscription (Cause 1 was mis-attributed in section 3)

Section 3 guessed the `use_effect` re-ran on an async read. It does not: the effect body reads no signal and `spawn`ed tasks are
polled outside any reactive context (dioxus-core 0.7.9 `tasks.rs::handle_task_wakeup`, `reactive_context.rs::CURRENT`), so nothing
subscribes the effect. The leak was one level up, in how `WidgetMasonry` mounted the blocks:

```rust
MasonryCard { component: move |()| (entry.component)(), popout: .. }     // HEAD
// MasonryCard: div { {component.call(())} }
```
`Callback::call` runs its closure with the scope it was CREATED in on the scope stack (`events.rs`: `with_scope_on_stack(self.origin, ..)`),
and `origin` is `WidgetMasonry`, not the card. So a block that calls hooks directly (`BlockPlayer`, `BlockColorPalette`,
`BlockCommand`, `BlockComposer`) pushed them onto WidgetMasonry's hook list. `Scope::use_hook` indexes by `hook_index`, which is reset
only when THAT scope renders (`scope_arena.rs::run_scope`) -- never again for WidgetMasonry. So every later re-render of the card ran the
block's hooks at an index past the end of the list and allocated a FRESH set (new signal at its initial value, new memo, new
effect -> a new `setInterval`) while the old set stayed alive, unreachable. `BlockPlayer`'s card re-renders whenever its label
ticks over, and every fresh player does so one second after it starts: one new, never-cleared 100 ms interval about every 1.2 s.
It also explains everything section 3 could not: the Progress demo (a real component) never leaked; the DOM never re-mounted;
`leak` stubbing "allowed exactly one re-run".

Visible side effects of the same bug, probed on the HEAD release build and on the fixed build (`/tmp` probes, same minute):

| block | HEAD (:8090) | fixed (:8083) |
|---|---|---|
| player readout, t = 2.5 s / 7.5 s | 1:24 / 1:24 (stuck: every fresh signal restarts at 84 s) | 1:27 / 1:32 |
| player pause | label and 100 ms intervals keep going (3 -> 7 -> 10 live) | holds still, no JS interval at all |
| composer textarea, type " XYZ" | typed text lost (fresh `draft` signal) | kept |
| color palette, 5 x ArrowRight on the area | trigger stays `#7C3AED` | `#7C3AED -> #742DED` |

Construction (by construction, covers the class): blocks are mounted as real components (`let Block = entry.component; rsx! { Block {} }`
as `MasonryCard`'s children), each owning a scope. `MasonryCard`'s doc records the rule. Not mechanically gated (a `Callback<_, Element>`
render prop is legitimate and common, and a hook inside one is not visible to a grep); the oracle's test 2 (readout advances, pause works)
and test 1 (live intervals constant) are the runtime backstop.

### 7.2 A. Timers own their lifetime

* `primitives/src/interval.rs`: `use_interval(period, on_tick)`. No JS timer: a task spawned in the caller's scope sleeping one `period`
  between ticks (`dioxus_sdk_time::sleep`: tokio on native, gloo on wasm; the same code on every arm, no `eval`), dropped with the
  scope. A re-render neither restarts nor duplicates it; `on_tick` is refreshed per render. Two unit tests on tokio's paused clock
  (`interval::tests`): one tick per period and no restart/duplicate across re-renders; no tick after unmount. Mutation-checked: spawning
  per render fails the first (8 ticks vs 5), `spawn_forever` fails the second.
* `BlockPlayer` and the Progress demo use it at 1 Hz (the slider has `step: 1.0`; the label shows whole seconds). The tick reads
  signals with `.peek()` only. The Progress demo's `Math.random()` over `eval` became a xorshift step (no `rand` dependency).
* Other timers: `setInterval`/`set_interval`/`Interval::new`/`spawn_forever` appear nowhere else in `preview/src`, `primitives/src` or
  `preview/index.html`. The remaining `setTimeout`s are one-shots (form fixture, charts gallery, message scroller helper) or carry their own
  cleanup (virtual list, carousel, message scroller); rAF loops are per-gesture. Nothing else converted.
* Gate `scripts/check-uncleared-intervals.sh` (0.1 s): fails on any of the four spellings above unless marked `// interval-ok: <reason>`;
  red against HEAD's `BlockPlayer`/Progress sources. `check-self-subscribing-effects.sh` was NOT extended: it would not have caught this
  (the effect never read the signals; the spawned task is excluded by design).

### 7.3 B. No blocking wheel/touch listener on `#main`

* `Input`: `onwheel` is attached only when the caller passed one (an rsx event attribute cannot take an `Option<EventHandler>`, so it is
  built with `dioxus::html::events::onwheel` and spread through `merge_attributes`). `Textarea` has no wheel prop and `NativeSelect`'s
  shims are not scroll-related (Dioxus registers a root listener once per event TYPE, so the per-instance shims cost nothing): left alone.
* The three `ontouchstart: prevent_default` handlers are removed (slider thumb, resizable handle, color-area thumb). Evidence, real touch
  via CDP `Input.dispatchTouchEvent`, `hasTouch`: the slider thumb and color area need nothing (their non-focusable containers are
  `touch-action: none`, and the effective value is the intersection down the ancestor chain; a vertical-biased drag moves the control,
  `scrollY` stays 0, nothing selected). **The resizable handle did NOT survive the first attempt**: `preview/assets/main.css`'s
  `touch-action: manipulation !important` catch-all (double-tap zoom oracle) matches the handle (it has `tabindex`) and beats its own
  `touch-action: none`; without the touchstart handler Chromium started a page pan after the touch slop and cancelled the pointer
  (`pointercancel(nc)` after 4 moves): drags of 40/50/80 px resized by 19.9/10.7/20.3 px instead of 39.8/49.8/79.6. Fix: one
  exemption rule in `main.css` (`.dx-resizable-handle[role="separator"] { touch-action: none !important }`) and a matching `role="separator"`
  carve-out in `touch-double-tap.spec.ts`. Verified: 39.8/49.8/79.6 again, `resizable.spec.ts` real-touch cases green, all 77 tests of the
  double-tap oracle green. (A `touch-action: none` on the handle's `::after`/grip does not work: tried.)
* Gate `scripts/check-blocking-scroll-listeners.sh` (0.3 s): unconditional `onwheel:`/`onmousewheel:`/`ontouchstart:`/`ontouchmove:` rsx
  attributes and `addEventListener('wheel'|'touchstart'|'touchmove'|..)` without `passive: true`; red against HEAD's four sources. Allowlist
  (with reasons): `context_menu.rs` (window listeners only while a menu is open) and `scroll_lock.rs`.
* **Known gap, not fixed (not this lane's file): `scroll_lock.rs::ensure_scroll_block_listeners_installed` installs NON-passive `wheel` and
  `touchmove` listeners on `window` PERMANENTLY at the first lock** (first Dialog/AlertDialog/Popover/DropdownMenu/ContextMenu open), gated
  afterwards by a `__dxScrollLocked` flag. After any overlay has opened once, every wheel/touch on the page waits for the main thread again.
  Fix: add on lock, remove on unlock. The oracle checks `#main` only (Playwright's own capture listeners pollute `window`).

### 7.4 C. `content-visibility: auto` on the demo cards

`.dx-component-card` (est. 380 px), non-popout `.dx-widget-card` (330 px), and, per the coordinator, `.dx-component-variant` (520 px; the
stacked variants on component pages) -- **the variant rule was withdrawn in round-2 verification (2026-10-04):** a skipped subtree has no layout box, so
demos that measure on mount (carousel slides and virtual window, a chart's width and default tooltip) measured 0 inside skipped variants and never
recovered; 33 Playwright tests went red in the full run, 30 on a serial rerun (`carousel-virtual-wheel` 18, `carousel` 7 / 4, `chart_tooltip`
`default_index` 8) and 51/51 are green with variants `visible`. Home cards keep the rule (their demos measure identically with and without it, checked by
scrolling every carousel and chart card into view on the pre-round and fixed builds). With narrow-viewport estimates; `contain-intrinsic-block-size: auto <n>` (logical, not the shorthand).
Sizes from measured heights at 1280 / 390 px: component cards mean 371 / 451, blocks median 333, variants mean 505 / 427.

Checked on the fixed build:
* CLS 0 over a full top -> bottom -> top wheel scroll of `/` and of five component pages (area_chart 9 variants, carousel 13, select, date_picker, combobox).
* Top-layer overlays opened from inside cards at the viewport's bottom edge (select, popover, dropdown menu, hover card, context menu, date
  picker, combobox, dialog, alert dialog, sheet, drawer, menubar, navigation menu) and from inside variants (select, date_picker) open, stay in the viewport
  and hit-test correctly; geometry identical with `content-visibility` forced off. No non-top-layer `position: fixed` descendant exists in any card.
* Find-in-page: `window.find()` for text in a never-rendered card (74 of 75 cards skipped) finds it and scrolls there. Chromium/Firefox only: **not in Safari 18-25
  (WebKit bug 283846; fixed in Safari 26)**.
* **Cost found: paint containment clips ink outside the box.** The card has no horizontal padding, so the title link's focus ring
  lost its left edge. Fixed with `overflow-clip-margin: 0.75rem` on cards and variants (screenshot-compared; not Safari).
* **Cost NOT fixed: first-visit programmatic jumps land off-target.** `scrollIntoView({block: "start"})` to a never-rendered gallery card from the top
  lands -600..+1,200 px off (est. 380; est. 256 is worse, up to +1,800; with `content-visibility` off the offset is a constant 60). Neighbouring cards render at
  their real height and move the target; `auto` remembers sizes only for cards that have been rendered. Wheel/keyboard scrolling is unaffected (CLS 0), and the home page has
  no in-page jump into the gallery, but any `scrollIntoView`/`#fragment` into the unseen part is imprecise on first visit. Playwright's actionability retries absorb it
  (`resizable`, `calendar`, `color-picker`, `slider`, `input`, `native_select`, `preview` specs green); a raw `boundingBox()` right after a jump does not.
* Not verified here: SSG/hydration of the final release build (CSS-only change plus `MasonryCard` children; markup is identical). Main loop: release SSG build + `hydration-parity`.

### 7.5 Oracle `playwright/oracle/tier2-html/scroll-main-thread.spec.ts` (counts, valid on debug)

| test | HEAD release (:8090) | fixed dev (:8083) |
|---|---|---|
| 1 live `setInterval`s constant over 12 s idle; timer fires/s bounded | FAIL: live 4 -> 10 -> 15; 59 then 111 fires/s | pass: 0 -> 0 -> 0; 2.0 fires/s |
| 2 player readout advances, pause works | FAIL: +0 s in 15 s | pass |
| 3 `#main` has no non-passive wheel/touchstart/touchmove | FAIL: `["wheel", "touchstart"]` | pass |
| 5 cards `content-visibility: auto`; >= 70% skipped at top and bottom; idle layouts <= 24 / 4 s | FAIL: all `visible` | pass: 74/75 skipped at top, 70/75 at bottom; 3-4 idle layouts / 4 s |
| 6 component-page variants skipped (area_chart) | FAIL: `visible` | pass |
| touch drag, slider / resizable / color area | pass / pass / pass | pass / pass / pass (resizable was RED on the first fixed build, see 7.3) |

The scripted-scroll layout and recalc counts are REPORTED, not capped: they do not separate HEAD from the fix on a dev build
(fixed dev: 351 layouts / 651 recalcs over 160 wheel ticks of 200 px; HEAD release 387 / 1,196 over 288 ticks of 100 px). What changed is the cost
per frame (PrePaint/Layerize are O(visible card)), which is time, not a count -- re-measure on the release build. One remaining layout source at idle on screen is the Progress
indicator's `transition: width 250ms` (114 of HEAD's 178 layouts in a 150-tick trace): O(card) now, inherent to the demo.

### 7.6 Before / after (`node scripts/measure-scroll.mjs`, 1280x800, 288 wheel ticks of 100 px / 16 ms, median of 3)

| | idle main ms/s | idle layouts / 3 s | scroll main busy ms | layouts | style recalcs | tasks > 16 / > 50 ms | wheel p50 / p95 / max ms | dropped / smooth frames | live `setInterval` |
|---|---|---|---|---|---|---|---|---|---|
| HEAD, release build (:8090) | 234.9 | 76 | 7,131 | 388 | 1,148 | 18 / 0 | 40 / 59.9 / 88.7 | 38 / 74 | 77 and growing |
| fixed, DEBUG dev build (:8083) | 175.9 | 3 | 2,552 | 172 | 699 | 3 / 0 | 30.6 / 49 / 66.7 | 35 / 36 | 0 (none exist) |
| same, page age 60 s at measurement: HEAD | 281.5 | 76 | 10,669 (1 run) | 417 | 1,248 | 72 / 0 | 41.9 / 79.4 / 147.2 | 73 / 117 | 87 |
| same, fixed (idle columns only: a clippy build shared the box during its scroll run) | 188.5 | 4 | n/a | n/a | n/a | n/a | n/a | n/a | 0 |
The fixed row is a DEBUG wasm build under a shared box: its milliseconds are inflated against HEAD's release build (idle 176 ms/s is debug wasm running two 1 Hz ticks), so read the counts. 

**Release re-measure of the final tree (2026-10-04, round-2 verification; both builds served by a static server, run back to back with nothing else on the box; `--runs 3` median unless noted; "before" is the frozen pre-round release build):**

| | idle main ms/s | idle layouts / 3 s | scroll main busy ms | layouts | style recalcs | tasks > 16 / > 50 ms | wheel p50 / p95 / max ms | dropped / smooth frames | live `setInterval` |
|---|---|---|---|---|---|---|---|---|---|
| before (pre-round release) | 178.3 | 71 | 6,744.5 | 387 | 1,181 | 8 / 0 | 41.2 / 57.8 / 79.5 | 30 / 54 | 74 |
| after (final tree, release) | 45.1 | 3 | 2,088.6 | 170 | 685 | 0 / 0 | 30.5 / 49.0 / 63.7 | 29 / 29 | 0 |
| before, 150 s warm-up (1 run) | 346.2 | 99 | 8,122.7 | 358 | 1,130 | 6 / 0 | 39.3 / 51.5 / 105.1 | 26 / 52 | 160 |
| after, 150 s warm-up (1 run) | 42.4 | 3 | 3,426.6 | 388 | 964 | 8 / 0 | 31.3 / 50.1 / 73.7 | 56 / 61 | 0 |

Reading it: idle cost, idle layouts and live intervals are the leak fix and they are flat over page age (before: 74 -> 160 intervals, idle 178 -> 346 ms/s; after: 0 and 42-45). The single-run scroll rows
are noisy (the after build's busy time ranged 2.1-3.4 s over five single runs; use the median of 3), and the pages differ in length: before 28.4k px (288 ticks to the bottom), after 38.7k px at load (383 ticks; 32.0k, 316 ticks and 1 stalled sample of 79 with the skip forced off via `--css`). The extra 6.7k px are the `contain-intrinsic-block-size` estimates (380 px) being taller than most real cards, and the 18 stalled samples of 96 are ticks where the page shrank under the scroll as skipped cards reported their real height (the scrollbar settling that section 7.4 accepted; 3.6k px of the difference to before is new content, the chat kit), so wheel ticks are not comparable across the rows, the per-second and per-frame columns are.
The pass criteria of section 6 hold: idle layouts 3 per 3 s (one per second, the same as the debug row above; was 71), live `setInterval`s constant over page age (0 at 2.5 s and at 150 s), tasks > 50 ms 0, and `#main` has only a
`pointermove` listener (no wheel/touch).


## 8. Anchor jumps over skipped cards (ANCHOR-JUMP lane, 2026-10-04)

Question: section 7.4 left "first-visit programmatic jumps land off-target" unfixed. Which real navigations does that hit, and is
there a construction that makes every one of them exact while keeping `content-visibility: auto`?

### 8.1 Audit: every same-page navigation in the app

`href: "#` and heading `id`s across `preview/src`: the docs sidebar's section links (`page_sections_submenu`, `main.rs`: Home
`#sample-interfaces`, `#all-components`; Overview `#how-it-works`, `#add-a-component`, `#recommended-workflow`), the charts hero's
`#charts` ("Browse charts"), and `href: "#"` placeholders inside demos (Pagination, Bubble, Marker: scroll to top, no target). No
component page heading has an id, no `#variant` link exists. Programmatic `scrollIntoView`: `charts_gallery.rs` (instant, to a row of its
own list, not a `content-visibility` card). So the live set is: link click, fresh load of `<page>#id`, Back/Forward to a hash. Only Home
has cards above a target; Overview and Charts have none, so `content-visibility` cannot move them.

### 8.2 Measured BEFORE (1280x800, first visit per case, settled = 2 rAF + nothing moved for 300 ms; error = `target.top - scroll-padding-top (60)`)

| anchor | click | fresh load `<page>#id` | Back / Forward to the hash |
|---|---|---|---|
| Home `#sample-interfaces` | 0 | NOT scrolled (+731, y = 0) | Back: stays put (-2,049) |
| Home `#all-components` | **+34.7 px** (cv: the skipped masonry blocks above it were estimates) | NOT scrolled (+2,695) | Forward: stays put |
| Overview `#how-it-works` | -0.2 | NOT scrolled (+293) | Back: stays put (-645) |
| Overview `#add-a-component` | +0.2 | NOT scrolled (+938) | Forward: stays put |
| Overview `#recommended-workflow` | +214 = the page's end (max scroll 981, physically unreachable) | NOT scrolled | |
| Charts `#charts` | -0.2 | NOT scrolled (+291) | |

Reference, the release build WITHOUT `content-visibility` (:8090): click and fresh load land within 0.5 px (SSG: the target is in the static
HTML); Back to a hash is the same -2,049 / -645. Reading it:

* Only ONE cell is caused by `content-visibility`: Home `#all-components` by click. It is small (35 px) because only masonry blocks sit above it;
  the -600..+1,200 px of 7.4 is for a target INSIDE the gallery, which no link has. (A fresh load of the SSG page lands even with cv on, 0.5 px, measured by serving the frozen release SSG pages with the new CSS;
  Chromium keeps re-applying the load-time fragment scroll while the document loads.)
* Two are PRE-EXISTING and independent of cv, found by this audit: (a) on the client-rendered `dx serve` page a fresh `/#id` never scrolls (the element is
  not in the DOM when the browser looks for it); (b) Back/Forward to a hash never scrolls, because the router sets `history.scrollRestoration = "manual"` and
  the browser then does nothing on a same-document traversal (`popstate` + `hashchange` fire, `scrollY` is unchanged). Same on the no-cv release build.
* The cv error would scale with how much skipped content sits above a target; today that is 35 px, tomorrow any anchor placed below the gallery.

### 8.3 Construction (one switch, one handler)

Subsumes: the click error (cv), (a) and (b) above, and any future anchor, with no per-link code.

* `main.css`: the three cv rules read `content-visibility: var(--dx-cv, auto)`; `html[data-cv-settle] { --dx-cv: visible }` is the only thing that
  turns it off. One variable, so a new card opts in by writing the same declaration (a bare `content-visibility: auto` would be exempt: stated in the
  comment; a gate that greps for it is the obvious follow-up, not added here).
* `preview/index.html`: one inline head script (like the theme pre-paint script; no wasm, no `eval`, so no cfg-axis question, runs on the SSG HTML and on
  `dx serve` alike). It sets the attribute BEFORE the scroll offset is computed and removes it ~300 ms after the last `scroll` event (>= 500 ms, <= 4 s), by which
  time every card has been rendered for real and recorded its true height for `contain-intrinsic-size: auto`, so the skipped cards keep the right size:
  1. click on a same-page `a[href^="#"]`: capture phase; sets the attribute and forces layout (`void target.offsetTop`) and lets the browser do its own (smooth) scroll,
     which therefore computes against final geometry;
  2. fresh load with a hash: the attribute is set from the first layout (so a static page's load-time fragment scroll is exact); once the target exists (DOMContentLoaded, or a
     `MutationObserver` for the client-rendered page) it scrolls there itself;
  3. `hashchange` not caused by one of our clicks (Back/Forward): scrolls there itself, as a fragment navigation would.
  Our own scrolls are `scrollIntoView({block: "start", behavior: "instant"})` (honours `scroll-padding-top`), re-aimed per frame for <= 1 s while fonts/hydration move the target;
  wheel/touch/key/pointer input ends that. A hash with no element releases after 15 s. Nothing is held if the script never runs: the CSS fallback is `auto`.
* Cost: one full-page layout per anchor click (`--dx-cv` flips every card for ~1 s), paid only on a fragment navigation; the page height drops to its true value
  (e.g. 35,956 px of estimates -> 29,793 px on the SSG emulation), so the scrollbar is also exact afterwards.

### 8.4 AFTER

Dev build (:8083), same probe: every cell above lands within 0.5 px (Home click `#all-components` +34.7 -> -0.3; fresh loads +291..+2,695 -> within 0.5; Back/Forward -2,049 / -645 -> within 0.3);
Overview `#recommended-workflow` still reads +214 because it is clamped at the document's end (`scrollY == max`; the spec computes the expectation with that clamp). Frozen release SSG pages
(:8090) + the new CSS + the script injected by route (a stand-in for the final SSG build): the same, `/` click `#all-components` +34.7 -> -0.3. At 390 px wide (touch, sheet sidebar): within 0.5 px.

Oracle `playwright/oracle/tier2-html/anchor-landing.spec.ts` (read the anchors off the sidebar, one first-visit page load per anchor; click with smooth and instant scroll, fresh load, Back/Forward chain, and
`data-cv-settle` released with the gallery skipped again): RED on :8083 before (6 of 12 fail: Home click -34.7 in both scroll modes, fresh load on both pages, Back/Forward on both), 12/12 green after.
`scroll-main-thread.spec.ts` 8/8 green. `node scripts/measure-scroll.mjs http://127.0.0.1:8083/ --runs 3 --brief` unchanged (no hash on the page, the variable falls back to `auto`):
170 layouts / 691 recalcs / tasks > 16 ms 3 / > 50 ms 0 / idle layouts 3 per 3 s, against 172 / 699 / 3 / 0 / 3 in 7.6. `window.find()` for the title of the last gallery card (74 of 75 skipped) still finds it.

Not covered / for the main loop: the release SSG build of the final tree (`scripts/build-ssg.sh release`) must be checked to carry the inline script verbatim into every page's head (the emulation injected it by route),
plus `hydration-parity`; Back to the ENTRY WITHOUT a hash (empty fragment) still does not scroll to the top (pre-existing, same cause as (b), no section anchor involved);
Safari: `scrollIntoView({behavior: "instant"})` and the capture-phase click handler are plain DOM, nothing here is Chromium-only except the measurements.
