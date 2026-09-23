# Carousel overscroll — root cause, platform measurements, and the rule that comes out of it

**Date:** 2026-09-23
**Status:** investigation closed; the port into `primitives/` is open work (backlog rows 102–104)
**Bench:** [`carousel-overscroll-bench.html`](./carousel-overscroll-bench.html), revision 28 — open it directly in a browser, no build step
**Provenance:** the stored file is byte-identical to the published bench
the owner drove during the session (<https://claude.ai/artifact/Pn3txmSwfhc5fLV2rRtLgw>);
it is checked in so the measurements below stay reproducible without it.

This is the write-up of a long interactive session driven by the repo owner on
real hardware (trackpad and mouse), not a desk study. Every number below came
off the instrumented bench during that session. Where something is inference
rather than measurement it says so.

---

## 1. The question

Round 6 shipped a carousel whose drag release animates, and round 7 fixed the
arrows and the pagination fade. What neither round fixed is the edges: you
cannot drag past slide 1 or the last slide and get the rubber-band that a
native scroller gives you. The owner asked for it twice, and the obvious
answers all failed in ways that were worth understanding.

Five candidate mechanisms were built side by side in one page so they could be
switched between mid-gesture and compared by feel, with telemetry recorded for
each:

| Mode | Internal id | Mechanism |
|---|---|---|
| A · Clamped | `clamped` | what the component does today — the scroller simply stops |
| B · Dead space | `deadspace` | real scrollable room past the last slide |
| B2 · Damped margin | `damped` | the same room, with a damping curve applied inside it |
| B3 · Edge transform | `bounce` | no extra room; overdrag expressed as a `transform` on the track |
| C · Damped transform | `transform` | whole-element transform, damped |

The owner's first verdict, before any of the analysis below: *"C matches native
the best, but has z-index issues. B works as realistic, but only as far as the
margin allows."* Both halves of that turned out to be structural, not tuning.

---

## 2. Root cause: the scroll position has three would-be writers

This is the finding the whole investigation reduces to.

At any moment during a wheel/trackpad gesture on a scroll-snap container, three
independent parties want to set `scrollLeft`:

1. **The compositor**, running the platform's momentum/fling animation. It runs
   off the main thread. `preventDefault()` on a wheel event is **advisory** here
   — once momentum is live, the scroll is no longer ours to veto.
2. **Scroll snapping**, on its own schedule, re-targeting the scroller whenever
   it decides the gesture has ended.
3. **Our code**, whenever it calls `scrollBy`/`scrollLeft` or toggles
   `scroll-snap-type`.

Two of those three are **neither observable nor sequenceable from JavaScript**.
You cannot read the compositor's in-flight offset (`scrollLeft` and
`getBoundingClientRect` report the last committed frame, not the one being
composited), and you cannot order your write against the snap engine's.

Every single glitch reported across ~20 bench revisions was this contest,
wearing a different hat each time: a slide jumping forward then back, the track
freezing at the edge, mid-range slides getting cut off, the release firing and
being cancelled in the same frame.

**The confirming evidence is negative and it is decisive.** The *mouse-drag*
path stayed clean for 20+ consecutive revisions with no tuning at all. A mouse
drag has no native scrolling behind it — there is no compositor animation and
no snap re-target, so our code is the only writer and owning the scroll
position is simply correct there. Same code, same maths, same constants: the
only difference is how many parties are writing.

---

## 3. THE RULE

> **Where the browser is already the scroller, never write the scroll position
> and never disable snapping. Express the overdrag as a transform only.
> Where there is no native scrolling — a mouse drag — owning it is safe.**

This is a construction, not a mitigation: it removes the contest rather than
timing around it. A transform is a compositor-side visual offset that does not
touch the scroll position at all, so there is nothing for the other two writers
to disagree with.

**The nuance that matters for the port.** The contest exists only *while
momentum is live*. Once a burst is spent the browser has stopped writing, and a
spring-back ease may safely own the offset from there. So the rule is not "never
animate" — it is "never animate against a live momentum stream". Detecting the
end of that stream accurately is the entire remaining difficulty, and §5 is the
measurement work that makes it possible.

---

## 4. What the platform actually does (measured)

An unmodified, script-free scroll-snap container was added to the bench
(*"Native control · no JavaScript"*) and instrumented purely as an observer, so
the platform could be measured rather than assumed. The owner drove it; the
bench captured.

| Observation | Value | Note |
|---|---|---|
| Visible rubber-band on a **sub-scroller** | **none observable from JS** | see below — this correction matters |
| Rest after the last wheel event | **64–78 ms** typical | outliers 210 / 526 / 609 ms on deliberately gentle gestures |
| Device delta quantum | **1 px** | the smallest non-zero `deltaY` this trackpad emits |
| Momentum decay | **≈ 0.96× per event** | geometric, across the tail of a fling |
| WebKit/macOS rubber-band curve | `b(x) = (1 − 1/(x·c/d + 1))·d`, `c ≈ 0.55` | algebraically identical to `L·x/(x+L)` with `L = d/c` |
| That curve's asymptote on a 320 px track | **582 px** | `L = 320 / 0.55` |

**The correction on native bounce, because I got it wrong twice and the owner
was right both times.** I first reported "leave the wheel to the platform,
macOS does this natively" — wrong: macOS gives elasticity to the *document*,
not to an arbitrary sub-scroller. I then over-corrected to "there is no native
rubber-band here at all" — also wrong. The owner's observation settles it:

> *"there is an end rubber-band if there is still momentum coming from slide 2
> to slide 1 and over, even if there is none from slide 1 to end."*

The bounce **exists** when momentum arrives at an edge. It is **composited**,
so `getBoundingClientRect` and `scrollLeft` cannot see it — which is why the
instrumented observer recorded nothing and why I misread the data. It is
invisible to JS, not absent. The practical consequence is the useful part:
**we can model it**, because the curve above is published and the asymptote is
derivable from the track width. We cannot *measure* it from inside the page.

---

## 5. Verdict: B2 cannot resist; B3 is the only variant that can

**B2 (damped margin) has a hard ceiling, and it is arithmetic.** The damping
needs to *slow the scroller down*, and slowing a scroller means owning its
position — exactly the fight THE RULE removes. With the fight removed, B2's
extra room fills at **1:1 with raw travel**, so the room's *size* is the only
thing controlling how far you go. The recording is unambiguous:

- a gesture peaking at **25 px/event consumed the entire 260 px room**;
- gestures at the other edge peaking at **145 and 121 px/event produced 3 px and
  0 px** of depth.

There is no resistance anywhere in that path, and no constant can add it.

The room was resized 260 → **140 px**, derived rather than guessed: the
deliberate end-edge attempts in that session ran 139, 183 and 331 px of raw
travel, and the platform's own curve maps their median to 139 px of visible
depth — so B2's wall now sits roughly where native would have eased to. **That
is mitigation, not a fix.** B2 trades a long free slide into a far wall for a
short one into a near wall. v27 had already shown it cannot even reach the
native asymptote: 582 px of curve against the 260 px of room it had.

What B2 *did* gain, and keeps: a correct snap-back, no glitching, and no stuck
state. The owner's closing assessment — *"overall B2 seems fine, it only
overshoots too quickly/strongly at the end"* — is precisely the wall.

**B3 (edge transform) has no wall.** Because the overdrag is a transform rather
than travel through finite room, the depth is bounded by the curve's asymptote
(582 px on a 320 px track) instead of by a `padding` value, and the curve can
be applied at full strength without owning the scroll position. B3 is the only
one of the five that can both *resist* and stay inside THE RULE. Its wheel path
is audited clean of `preventDefault`, `scrollBy`, `scrollLeft`,
`scrollIntoView`, snap-type writes, settle calls and rAF-deferred inference:

```js
// B3 wheel path — the whole decision, synchronous, no scroll writes
var g = edgeGaps();
var atLimit = cd > 0 ? g.left >= -0.5 : g.right <= 0.5;
if (atLimit) { rawOver += cd; applyBounce(); }
else if (rawOver !== 0 && !bounceHoming) { release("reversed"); return; }
```

```js
var RUBBER_C = 0.55;
function nativeLimit() { return (track.clientWidth || 320) / RUBBER_C; }
function rubberLimit() {
  return mode === "bounce" ? nativeLimit() : Math.min(nativeLimit(), DAMP_LIMIT);
}
function rubber(depth) { var L = rubberLimit(); return L * depth / (depth + L); }
```

**C (whole-element damped transform)** felt closest to native to the owner and
is subsumed by B3: same mechanism, but applied to the track rather than the
whole element, which is what dissolves C's z-index problem — a transform on the
carousel root creates a containing block that the arrow buttons and any
portalled content sit inside, and moving it to the track leaves them alone.

**A (clamped)** is the status quo. **B (plain dead space)** is B2 without the
curve and is strictly worse than it.

---

## 6. Port invariants

These are binding on whatever lands in `primitives/`. Each one is the
generalisation of a specific bench regression; the regression is named so the
invariant is falsifiable rather than folklore.

1. **Never read or write `scrollLeft`.** Use `scrollBy` with physical deltas, or
   read geometry with `getBoundingClientRect`. In RTL the zero point and the
   sign of `scrollLeft` disagree across engines — round 5 measured
   `scrollLeft: 0 → -153` on the `rtl` variant of this very component. The
   bench uses `scrollLeft` freely because it is a single-direction throwaway;
   the component must not copy that.
2. **Detect the edge synchronously, from layout gaps.** Compare each child's
   `getBoundingClientRect()` start edge against the container's, in the same
   turn as the decision. *(v18: "trackpad scrolling from 1→5 makes slide 3
   cut off" — the edge was being inferred by comparing a request against a
   measurement taken a frame later. That test is out of phase with the
   compositor and accumulated 198–212 px of false overdrag mid-range on long
   flicks.)*
3. **Never `scrollIntoView({behavior: 'smooth'})` for spring-back or settle.**
   It hands the position back to the browser's animator, which is writer #1.
4. **Never restart a spring-back that is already in flight.** *(v16: the release
   threshold became peak-relative (~10 px) while the interrupt threshold stayed
   a fixed 6 px, so deltas between the two both triggered and cancelled a
   release every frame. Fixed by holding the interrupt floor at 3× the release
   threshold.)*
5. **Never use spacer children** to create the room. They enter the snap
   engine's candidate list and the `aria` slide count, and they are visible to
   `:nth-child` styling.
6. **Suspend `scroll-snap-type` at most once per gesture, and restore it only at
   release.** *(v12: "might now even mess with in-between slides sliding" —
   `scrollSnapType = rawOver !== 0 ? "none" : ""` restored mandatory snapping
   *mid-gesture*, re-targeting the scroller under the user's finger.)*
7. **Change measurement or timing, never both at once.** *(v9: "weird bugs now
   for trackpad" — the wheel decision was moved to rAF in the same revision
   that changed how the edge was measured, and the two could not be told
   apart.)*
8. **Any threshold expressed in pixels must be at or above the device's own
   quantum.** *(The 1.6 px release floor sat below this trackpad's 1 px
   quantum — the owner flagged it before the data did. It is now
   `deviceMinDelta × 2`, learned from the stream, with 1.6 as a fallback only
   until the device speaks.)*

---

## 7. The bench, and how to read its telemetry

`carousel-overscroll-bench.html` is self-contained: open it in a browser, pick a
mode, and drive it. It records automatically, so an interaction session produces
data without anyone remembering to press a button.

Two record streams:

- **`record.wheelBursts[]`** — one entry per gesture against an *instrumented*
  mode: `mode`, `deltaMode`, `events`, `peak`, `deltas[]`, `maxOver`,
  `releasedBy`, `ms`, `releaseThreshold`, `interruptFloor`, `deviceFloor`,
  `edge`, `net`. `edge` was added in v28 so start/end asymmetry can be read
  directly instead of inferred from the sign of the sum.
- **`record.nativeBursts[]`** — the same for the script-free control:
  `deltaMode`, `events`, `peak`, `minDelta`, `deltas[]`, `trace[]`, `maxLeft`,
  `maxRight`, `visibleBounce`, `restAfterLastEventMs`, `clampedAt`,
  `velocityAtClamp`, `eventsAfterClamp`.

Tuned constants currently in the file, all of them measured or derived rather
than picked:

```js
var PAD_LONG              = 140;   // B2 room — the room IS the limit, so it is sized like one
var RUBBER_C              = 0.55;  // WebKit's published rubber-band constant
var DAMP_LIMIT            = PAD_LONG * 0.85;   // B2 only: capped by the room it has
var WHEEL_IDLE_MS         = 90;    // backstop; measured rest after last event is 64–78 ms
var WHEEL_RELEASE_DELTA   = 1.6;   // fallback only, until the device's quantum is known
var WHEEL_FLOOR_MULT      = 2;     // × the device's own quantum
var WHEEL_RELEASE_FRACTION= 0.12;
var WHEEL_RELEASE_MAX     = 10;    // bounded, or a hard flick sets an absurd bar
```

**Two telemetry defects were found and fixed mid-session, and both are worth
remembering** because each one made the data lie in the exact region being
investigated: burst tracking keyed off pad installation rather than the gesture
(one B2 burst recorded across a 49-second session — blind precisely where the
bug lived), and `releaseThreshold` was recorded *after* the peak was zeroed,
reporting a flat `1.6` for every burst regardless of what was used.

---

## 8. What is still open

- **The port itself.** None of this is in `primitives/` yet — the component
  still behaves as mode A. Backlog row 102.
- **The block axis.** Everything above was driven horizontally. The edge test is
  written axis-generically (it reads `orientation`), but `vertical` was never
  driven by hand.
- **RTL.** Invariant 1 keeps the design RTL-safe *by construction* — the edge
  test compares rect edges, which are direction-agnostic — but no RTL gesture
  was driven on the bench.
- **The `multiple` variant.** Untested; the edge test is per-child, so it should
  hold, but "should" is not "does".
- **Cross-browser.** All measurements are from one engine on one machine.
  `RUBBER_C = 0.55` is WebKit's; Chromium and Gecko have their own curves, and a
  wrong constant degrades gracefully (the band is stiffer or looser) rather than
  breaking.
- **Arrow-paging during an active bounce.** The interaction between a
  `CarouselPrevious`/`CarouselNext` press and an in-flight spring-back has a
  timing window that was never exercised.

---

## 9. Corrections in the record

Kept deliberately, because each was stated confidently and then withdrawn on the
owner's evidence, and the pattern is instructive:

1. *"Leave the wheel to the platform, macOS does this natively."* Wrong — macOS
   gives elasticity to the document, not to a sub-scroller.
2. *"There is no native rubber-band on a sub-scroller."* Also wrong — it exists
   when momentum arrives at an edge, but it is composited and invisible to
   `getBoundingClientRect`/`scrollLeft`. I had written the correct disjunction
   into the code comment and then reported the wrong branch of it.
3. *"B2 cannot be made to work on a trackpad."* Too strong. B2 can have a
   correct snap-back, and does. What it cannot do is **resist**.
4. The bench's use of `scrollLeft` was defended as convention when the owner
   pointed at it — *"I thought we wouldn't use scrollLeft because of RTL, and
   you proposed scrollBy"*. Fair: the bench's convention is not the component's,
   and invariant 1 exists so the distinction is not lost in the port.
