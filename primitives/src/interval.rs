//! `use_interval`: the one sanctioned repeating timer.
//!
//! A repeating timer must belong to the component that asked for it: it starts when the
//! component mounts, runs exactly once, and is gone when the component is. The previous shape
//! of this code was a JS `setInterval` started from a `use_effect` through `document::eval`,
//! which nothing could ever clear (a JS interval is owned by the page, not by the Dioxus scope
//! that evaluated the script). Every re-run of that effect, and every visit to the page,
//! stacked another interval on top of the old ones, so the page's idle main-thread cost grew
//! with its age (dev-docs/research/scroll-jank-2026-10-04.md, Cause 1: 128 live intervals after
//! ~100 s, 1,326 timer wakeups per second at 155 s).
//!
//! This hook has no JS interval at all. The tick loop is a Dioxus task spawned in the calling
//! component's scope that sleeps one `period` between ticks
//! (`dioxus_sdk_time::sleep`: `tokio` on native, `gloo_timers` on wasm), so it is dropped with the
//! scope on unmount, there is nothing to "forget to clear", it works on every renderer (an
//! `eval`-based timer needs a webview), and a tick does not cross the JS/wasm boundary. Sleeping
//! between ticks, rather than `tokio::time::interval`, also means a stalled main thread (a debug
//! build, a throttled background tab) yields ONE late tick, not a burst of catch-up ticks.
//! `scripts/check-uncleared-intervals.sh` keeps `setInterval` out of the Rust and JS sources
//! entirely, so a repeating timer can only be written this way.
//!
//! ```rust,ignore
//! let mut seconds = use_signal(|| 0u32);
//! use_interval(Duration::from_secs(1), move || seconds += 1);
//! ```

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_core::Task;
use dioxus_sdk_time::sleep;

/// Calls `on_tick` once every `period` for as long as the calling component is mounted.
///
/// * The timer starts when the component first renders (the first tick is one `period` later,
///   on every target) and is dropped when it unmounts. A re-render never restarts it, so
///   `period` is read once, on mount.
/// * `on_tick` is refreshed on every render (like `use_callback`), so it may capture props and
///   other non-`Copy` values and always sees the latest ones.
/// * `on_tick` runs in a task, outside any reactive context: reads inside it do not subscribe
///   anything. Read signals with `.peek()` to make that explicit and write them with `.set()`.
///   Do not drive a timer from a `use_effect` that also reads the signals the tick writes: the
///   effect re-runs on its own writes (`scripts/check-self-subscribing-effects.sh`).
/// * Pick the slowest period that still looks right. A tick that changes layout is a main-thread
///   frame, and on a page full of live demos every frame is paid whether or not anyone is
///   looking at the demo.
pub fn use_interval(period: Duration, on_tick: impl FnMut() + 'static) {
    let mut on_tick = on_tick;
    let on_tick = use_callback(move |()| on_tick());
    // `use_hook` runs its initializer once per component instance, which is what makes the timer
    // survive re-renders without restarting; the spawned task is owned by the scope and is
    // cancelled when the scope drops.
    use_hook(move || {
        spawn(async move {
            loop {
                sleep(period).await;
                on_tick.call(());
            }
        })
    });
}

/// [`use_interval`] that only runs while `active` is true.
///
/// A timer that drives something visible (a clock, a demo's progress, a slideshow) is wasted
/// work whenever nobody can see it. While `active` is false there is NO task at all, so no
/// wakeups; when it turns true the loop starts again with a fresh full `period` (no instant
/// catch-up tick). Pair it with [`crate::activity::use_motion`]'s `active()`:
///
/// ```rust,ignore
/// let motion = use_motion();
/// use_interval_while(motion.active(), Duration::from_secs(1), move || seconds += 1);
/// ```
///
/// A timer that must keep counting while scrolled away (a toast's auto-dismiss) uses
/// [`use_timeout_while`] with [`crate::activity::use_document_visible`] instead.
/// `scripts/check-motion-gating.sh` rejects a bare `use_interval` outside this file.
pub fn use_interval_while(
    active: impl Into<ReadSignal<bool>>,
    period: Duration,
    on_tick: impl FnMut() + 'static,
) {
    let active: ReadSignal<bool> = active.into();
    let mut on_tick = on_tick;
    let on_tick = use_callback(move |()| on_tick());
    // Not a signal: the effect must not subscribe to its own bookkeeping.
    let mut running = use_hook(|| CopyValue::new(None as Option<Task>));
    use_effect(move || {
        if let Some(task) = running.write().take() {
            task.cancel();
        }
        if !active() {
            return;
        }
        let task = spawn(async move {
            loop {
                sleep(period).await;
                on_tick.call(());
            }
        });
        running.set(Some(task));
    });
    use_drop(move || {
        if let Some(task) = running.write().take() {
            task.cancel();
        }
    });
}

/// Calls `on_fire` once, `delay` after `active` is (or becomes) true. If `active` turns false
/// before it fires the countdown is dropped, and when it turns true again it starts over with
/// the full `delay` -- so a toast the user could not see (hidden tab) is not gone when they come
/// back. Never fires twice.
pub fn use_timeout_while(
    active: impl Into<ReadSignal<bool>>,
    delay: Duration,
    on_fire: impl FnMut() + 'static,
) {
    let active: ReadSignal<bool> = active.into();
    let mut on_fire = on_fire;
    let on_fire = use_callback(move |()| on_fire());
    let mut running = use_hook(|| CopyValue::new(None as Option<Task>));
    let mut fired = use_hook(|| CopyValue::new(false));
    use_effect(move || {
        if let Some(task) = running.write().take() {
            task.cancel();
        }
        if !active() || *fired.peek() {
            return;
        }
        let task = spawn(async move {
            sleep(delay).await;
            fired.set(true);
            on_fire.call(());
        });
        running.set(Some(task));
    });
    use_drop(move || {
        if let Some(task) = running.write().take() {
            task.cancel();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    const PERIOD: Duration = Duration::from_millis(100);

    /// Shared with the test through a root context: ticks seen, whether the ticker is mounted, and
    /// a counter the host passes down as a prop so the ticker itself re-renders on demand.
    #[derive(Clone, Default)]
    struct Harness {
        ticks: Rc<Cell<usize>>,
        mounted: Rc<Cell<bool>>,
        generation: Rc<Cell<usize>>,
    }

    #[component]
    fn Host() -> Element {
        let harness: Harness = consume_context();
        rsx! {
            if harness.mounted.get() {
                Ticker { generation: harness.generation.get() }
            }
        }
    }

    #[component]
    fn Ticker(generation: usize) -> Element {
        let _ = generation;
        let ticks = consume_context::<Harness>().ticks;
        use_interval(PERIOD, move || ticks.set(ticks.get() + 1));
        rsx! { "ticking" }
    }

    fn build() -> (VirtualDom, Harness) {
        let harness = Harness::default();
        harness.mounted.set(true);
        let mut dom = VirtualDom::new(Host).with_root_context(harness.clone());
        dom.rebuild_in_place();
        // Poll the spawned task once so its first `sleep` is registered at virtual t = 0.
        dom.process_events();
        (dom, harness)
    }

    /// Re-renders the host (and with it the ticker, whose `generation` prop changed).
    fn rerender(dom: &mut VirtualDom, harness: &Harness) {
        harness.generation.set(harness.generation.get() + 1);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate_to_vec();
    }

    /// Moves the paused clock forward and lets the virtual dom poll whatever woke. (Not
    /// `wait_for_work`: it only returns for a dirty scope, and a tick that bumps a plain counter
    /// dirties none.)
    async fn advance(dom: &mut VirtualDom, by: Duration) {
        tokio::time::advance(by).await;
        dom.process_events();
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn ticks_once_per_period_and_survives_rerenders() {
        let (mut dom, harness) = build();
        assert_eq!(
            harness.ticks.get(),
            0,
            "the first tick is one period after mount, not at mount"
        );

        for want in 1..=3 {
            advance(&mut dom, PERIOD).await;
            assert_eq!(harness.ticks.get(), want, "exactly one tick per period");
        }

        // The ticker re-renders (its prop changes) in the MIDDLE of a period. A restarted timer would
        // tick 60 ms later than before, a duplicated one would tick twice: neither may happen.
        advance(&mut dom, PERIOD * 6 / 10).await;
        assert_eq!(harness.ticks.get(), 3, "no tick mid-period");
        for _ in 0..3 {
            rerender(&mut dom, &harness);
        }
        advance(&mut dom, PERIOD * 6 / 10).await;
        assert_eq!(
            harness.ticks.get(),
            4,
            "a restarted timer would not have ticked yet"
        );
        // A duplicated timer (one more per re-render) would have started at the re-render and tick
        // again before the original's next one: one full period on, still exactly one tick.
        advance(&mut dom, PERIOD).await;
        assert_eq!(
            harness.ticks.get(),
            5,
            "re-renders do not duplicate the timer"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn stops_when_the_component_unmounts() {
        let (mut dom, harness) = build();
        advance(&mut dom, PERIOD).await;
        advance(&mut dom, PERIOD).await;
        assert_eq!(harness.ticks.get(), 2);

        harness.mounted.set(false);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate_to_vec();

        // Ten more periods of virtual time: an owned timer is gone with its component.
        for _ in 0..10 {
            advance(&mut dom, PERIOD).await;
        }
        assert_eq!(
            harness.ticks.get(),
            2,
            "no tick after the component unmounted"
        );
    }

    /// A ticker whose gate the test flips from outside.
    #[derive(Clone, Default)]
    struct GateHarness {
        ticks: Rc<Cell<usize>>,
        gate: Rc<Cell<Option<Signal<bool>>>>,
    }

    #[component]
    fn GatedHost() -> Element {
        let harness: GateHarness = consume_context();
        let gate = use_signal(|| true);
        harness.gate.set(Some(gate));
        let ticks = harness.ticks;
        use_interval_while(gate, PERIOD, move || ticks.set(ticks.get() + 1));
        rsx! { "gated" }
    }

    fn set_gate(dom: &mut VirtualDom, harness: &GateHarness, on: bool) {
        let mut gate = harness.gate.get().expect("the host rendered");
        dom.in_scope(ScopeId::APP, || gate.set(on));
        dom.render_immediate_to_vec();
        dom.process_events();
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn use_interval_while_stops_while_inactive_and_restarts_with_a_full_period() {
        let harness = GateHarness::default();
        let mut dom = VirtualDom::new(GatedHost).with_root_context(harness.clone());
        dom.rebuild_in_place();
        dom.process_events();

        advance(&mut dom, PERIOD).await;
        assert_eq!(harness.ticks.get(), 1);

        set_gate(&mut dom, &harness, false);
        for _ in 0..10 {
            advance(&mut dom, PERIOD).await;
        }
        assert_eq!(harness.ticks.get(), 1, "no tick while inactive");

        set_gate(&mut dom, &harness, true);
        advance(&mut dom, PERIOD * 6 / 10).await;
        assert_eq!(
            harness.ticks.get(),
            1,
            "no catch-up tick on resume: the countdown starts over"
        );
        advance(&mut dom, PERIOD * 6 / 10).await;
        assert_eq!(
            harness.ticks.get(),
            2,
            "one tick a full period after resume"
        );
    }
}
