//! `use_motion` / `use_motion_active_for` / `use_document_visible`: is anybody able to see this
//! element's motion right now?
//!
//! A looping animation or a self-driven timer costs main-thread work whether or not anyone is
//! looking at it. A page full of live demos paid for hundreds of animation frames per scroll for
//! spinners and skeletons in cards that were not on screen
//! (`dev-docs/research/scroll-profile-2026-10-10.md`: removing the infinite animations alone took
//! 15% off the scroll busy time). Pausing them one by one at each call site is the same
//! mistake as the uncleared `setInterval` (`interval.rs`): the next animation added forgets to.
//! Here "is this visible" is answered once, in one place, and components subscribe to the answer.
//!
//! Two signals share the registry (and one observer per kind).
//!
//! * **Continuous loops and timers** -- [`use_motion`], [`use_motion_when`],
//!   [`use_motion_active_for`]: spinner, skeleton, progress, avatar, shimmer hosts, carousel
//!   autoplay, the home-page tickers. These have a pre-roll margin so they are already moving
//!   when they scroll in.
//! * **One-shot entrances** -- [`use_entered_view_when`]: chart load animations. NO pre-roll: an
//!   entrance that starts before the element is on screen is an entrance nobody saw.
//! * **Timers that carry meaning** -- [`use_document_visible`]: toast auto-dismiss.
//!
//! A loop's motion is **active** unless any of these holds:
//!
//! * the element is outside the viewport. ONE `IntersectionObserver` is shared by the whole page
//!   (every subscribed element is `observe`d on it), with a `rootMargin` of [`ROOT_MARGIN`], so the
//!   motion is already running a moment before it scrolls into view (no visible restart) and stops
//!   once it is clearly off-screen above, below or to the side. This is the primary signal.
//! * the document is hidden (`visibilitychange`: another tab, a minimized window);
//! * the element sits inside a `content-visibility: auto` subtree the browser is skipping
//!   (`contentvisibilityautostatechange`, captured once at the document). A skipped subtree has
//!   no box, so the observer already reports it as outside the viewport; the event is the
//!   explicit second signal and also covers a subtree the observer's margin would consider close.
//!
//! The reduced-motion preference is deliberately NOT part of it: it is a different question
//! ("should this move at all") that the stylesheet answers with `@media (prefers-reduced-motion)`.
//!
//! # Pausing a CSS animation
//!
//! ```rust,ignore
//! let motion = use_motion();
//! rsx! { span { ..merge_attributes(vec![base, motion.attributes()]) } }
//! ```
//!
//! `motion.attributes()` carries a lookup key and, while inactive, `data-dx-motion="paused"`.
//! ONE rule in `dx-components-theme.css` turns that into
//! `animation-play-state: paused` for the element and everything under it, so the component
//! never mentions an animation by name and a future animation is covered without being listed.
//! `paused` (not `animation: none`) keeps the phase: the spinner resumes where it stopped, at no
//! cost while stopped. An element that animates ONLY sometimes (an indeterminate bar, an avatar
//! that is still loading) uses [`use_motion_when`], which subscribes nothing while it is false.
//!
//! # Stopping a timer
//!
//! ```rust,ignore
//! let motion = use_motion();
//! use_interval_while(motion.active(), Duration::from_secs(1), move || seconds += 1);
//! ```
//!
//! [`crate::interval::use_interval_while`] drops its task while the signal is false and starts a
//! fresh countdown when it turns true, so a hidden component makes no timer wakeups at all.
//! [`use_motion_active_for`] does the same for a component that does not own its root element
//! (the carousel's autoplay observes the carousel's scroller by id).
//!
//! # Timers that carry meaning
//!
//! A timer the user is waiting on (a toast's auto-dismiss) must NOT stop because its element
//! scrolled out of view, only because nobody can read the page at all: use
//! [`use_document_visible`], which follows the tab and ignores position.
//!
//! # Cost and lifetime
//!
//! Each subscription is one `document::eval` script through [`crate::js_listener`]: its removal is
//! derived from the owning scope. The page-wide registry (`window.__dxActivity`) is reference
//! counted: the first subscriber installs the two document listeners and the observer, the last
//! one to unmount removes them, so the listener inventory returns to its baseline. Before the
//! observer reports (and on the server, and when the element cannot be found) a motion is
//! active, which is also what the server renders, so there is no hydration mismatch.

use dioxus::prelude::*;

use crate::dioxus_attributes::attributes;
use crate::js_listener::{use_js_listeners, JsListeners};

/// The observer's `rootMargin`: how far outside the viewport a motion keeps running.
///
/// 200 px is a quarter of a laptop viewport. It is large enough that a spinner or shimmer is
/// already moving when its card enters (a wheel tick is 100 px, a trackpad flick covers it in
/// a frame or two, and an animation that resumes one frame late is invisible because `paused`
/// keeps the phase), and small enough that only the handful of elements within one screen of the
/// viewport animate, instead of the whole page. A bigger margin buys nothing visible and keeps
/// off-screen motion alive; a smaller one lets the resume land inside the viewport on a fast
/// scroll.
pub const ROOT_MARGIN: &str = "200px";

/// How much of an element must be on screen before an entrance ([`use_entered_view_when`])
/// counts as seen: 40% of its own area, or 40% of the viewport's height for an element taller
/// than the viewport (a ratio alone would never be reached by a tall card). Low enough that a
/// chart whose top third has scrolled in starts animating while the user is still looking at
/// it, high enough that a card brushing the viewport edge while flinging past does not spend
/// its one entrance unseen. No pre-roll margin: an entrance that starts before it is on screen
/// is simply missed.
pub const ENTER_RATIO: f64 = 0.4;

/// The page-wide registry and the per-subscription body. The first script to run creates the
/// registry; its two document listeners and its observer live exactly as long as it has
/// subscribers.
const SCRIPT: &str = r#"
const sel = await dioxus.recv();
const mode = await dioxus.recv();
const ROOT_MARGIN = await dioxus.recv();
const ENTER_RATIO = await dioxus.recv();
const make = () => {
    const subs = new Set();
    // One observer per kind, created on first use and shared by every subscriber of that kind:
    // `loop` (pre-roll margin) and `enter` (no margin, visible past a threshold).
    const ios = { loop: null, enter: null };
    const byEl = { loop: new Map(), enter: new Map() };
    const skipped = new Set();
    const hidden = () => document.visibilityState === 'hidden';
    // loop: unknown (`vis === null`: the observer has not reported yet) sends nothing, so the
    // first message is a real answer; a hidden tab is a real answer without waiting for it.
    // enter: sends `true` exactly once, when the element is past the threshold with the tab
    // visible, then the subscription lets go of the observer.
    const emit = (s) => {
        const h = hidden();
        if (s.kind === 'enter') {
            if (!s.last && !h && s.vis && !s.skip) { s.last = true; s.cb(true); drop(s); }
            return;
        }
        if (!h && s.el && s.vis === null) return;
        const a = !h && (s.el === null || (s.vis && !s.skip));
        if (a !== s.last) { s.last = a; s.cb(a); }
    };
    const inSkipped = (el) => {
        for (const r of skipped) {
            if (!r.isConnected) skipped.delete(r);
            else if (r.contains(el)) return true;
        }
        return false;
    };
    const onVis = () => subs.forEach(emit);
    const onCv = (e) => {
        if (e.skipped) skipped.add(e.target); else skipped.delete(e.target);
        subs.forEach((s) => {
            if (s.el && e.target.contains(s.el)) { s.skip = inSkipped(s.el); emit(s); }
        });
    };
    const onIo = (kind) => (entries) => {
        for (const en of entries) {
            const set = byEl[kind].get(en.target);
            if (!set) continue;
            const room = en.rootBounds ? en.rootBounds.height : innerHeight;
            const seen = kind === 'loop'
                ? en.isIntersecting
                : en.isIntersecting && (en.intersectionRatio >= ENTER_RATIO
                    || en.intersectionRect.height >= ENTER_RATIO * room);
            for (const s of [...set]) { s.vis = seen; emit(s); }
        }
    };
    const observe = (s) => {
        if (!ios[s.kind]) {
            ios[s.kind] = s.kind === 'loop'
                ? new IntersectionObserver(onIo('loop'), { rootMargin: ROOT_MARGIN })
                : new IntersectionObserver(onIo('enter'), {
                    threshold: [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1],
                });
        }
        let set = byEl[s.kind].get(s.el);
        if (!set) { set = new Set(); byEl[s.kind].set(s.el, set); ios[s.kind].observe(s.el); }
        set.add(s);
    };
    const unobserve = (s) => {
        const set = s.el && byEl[s.kind].get(s.el);
        if (!set) return;
        set.delete(s);
        if (set.size === 0) { byEl[s.kind].delete(s.el); if (ios[s.kind]) ios[s.kind].unobserve(s.el); }
    };
    // An entrance has done its one job: let go of the observer, keep the subscription (and so
    // the registry) until the owner unmounts.
    const drop = (s) => unobserve(s);
    const start = () => {
        // listener-ok: reference-counted by the registry; removed in stop() when the last subscriber leaves
        document.addEventListener('visibilitychange', onVis);
        // listener-ok: reference-counted by the registry; removed in stop() when the last subscriber leaves
        document.addEventListener('contentvisibilityautostatechange', onCv, true);
    };
    const stop = () => {
        document.removeEventListener('visibilitychange', onVis);
        document.removeEventListener('contentvisibilityautostatechange', onCv, true);
        for (const k of ['loop', 'enter']) { if (ios[k]) ios[k].disconnect(); ios[k] = null; }
        skipped.clear();
        if (window.__dxActivity === reg) delete window.__dxActivity;
    };
    const reg = {
        subs,
        add(el, kind, cb) {
            if (subs.size === 0) start();
            const s = { el, kind, cb, vis: null, skip: false, last: kind === 'enter' ? false : null };
            subs.add(s);
            if (el) { observe(s); s.skip = inSkipped(el); }
            emit(s);
            return () => {
                unobserve(s);
                subs.delete(s);
                if (subs.size === 0) stop();
            };
        },
    };
    return reg;
};
const reg = window.__dxActivity || (window.__dxActivity = make());
const el = mode === 'doc' ? null : document.querySelector(sel);
if (mode === 'doc' || el) {
    cleanup(reg.add(el, mode === 'enter' ? 'enter' : 'loop', (a) => dioxus.send(a)));
}
"#;

/// Which signals decide `active`.
#[derive(Clone, Copy)]
enum Mode {
    /// Viewport + `content-visibility` + tab.
    Element,
    /// The tab only.
    Document,
    /// One-shot: true once, when the element is first seen past a threshold.
    Enter,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Element => "el",
            Mode::Document => "doc",
            Mode::Enter => "enter",
        }
    }
}

/// Escapes `id` for use inside `[id="..."]`.
fn id_selector(id: &str) -> String {
    format!("[id=\"{}\"]", id.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Subscribes the calling component to the activity of the element `selector()` names (`None`:
/// nothing to observe yet, the motion stays active). Re-subscribes when a signal `selector`
/// reads changes.
fn use_activity(
    mut selector: impl FnMut() -> Option<String> + 'static,
    mode: Mode,
    mut active: Signal<bool>,
    mut known: Signal<bool>,
) {
    use_js_listeners(move || {
        let Some(selector) = selector() else {
            // An entrance keeps its answer (the owner latches it); the others fall back to the
            // optimistic default.
            if !matches!(mode, Mode::Enter) {
                if !*active.peek() {
                    active.set(true);
                }
                if *known.peek() {
                    known.set(false);
                }
            }
            return None;
        };
        Some(
            JsListeners::install(SCRIPT)
                .arg(selector)
                .arg(mode.as_str())
                .arg(ROOT_MARGIN)
                .arg(ENTER_RATIO)
                .on_message(move |now: bool| {
                    if *active.peek() != now {
                        active.set(now);
                    }
                    if !*known.peek() {
                        known.set(true);
                    }
                }),
        )
    });
}

/// The handle [`use_motion`] returns.
#[derive(Clone, Copy)]
pub struct Motion {
    key: Signal<String>,
    active: Signal<bool>,
    known: Signal<bool>,
    enabled: Signal<bool>,
}

impl Motion {
    /// Whether the element's motion should be running. Read it in a task, an effect or a
    /// `use_interval_while`; it is a signal, so reading it in render subscribes the component.
    pub fn active(&self) -> ReadSignal<bool> {
        self.active.into()
    }

    /// Whether the observer has answered at least once (until then [`active`](Self::active) is
    /// the optimistic default `true`). A one-shot "play when first seen" effect must wait for
    /// this, or it fires on the default.
    pub fn known(&self) -> ReadSignal<bool> {
        self.known.into()
    }

    /// The attributes to spread on the element that carries the motion: the lookup key, and
    /// `data-dx-motion="paused"` while the motion is inactive (the theme stylesheet pauses every
    /// CSS animation on and under it). Reads `active`, so the component re-renders when it flips.
    pub fn attributes(&self) -> Vec<Attribute> {
        let paused = !(self.active)();
        let key = self.key.cloned();
        attributes!(span {
            "data-dx-motion-key": key,
            "data-dx-motion": paused.then_some("paused"),
        })
    }

    /// Whether this handle is currently observing anything (`false` while [`use_motion_when`]'s
    /// condition is false).
    pub fn enabled(&self) -> bool {
        (self.enabled)()
    }
}

/// Tracks whether the element the caller spreads [`Motion::attributes`] onto is visible. See the
/// [module docs](self).
pub fn use_motion() -> Motion {
    use_motion_when(|| true)
}

/// Like [`use_motion`], but only observes while `enabled()` is true (an indeterminate progress
/// bar, an avatar that is still loading): while it is false no script runs and the motion is
/// reported active. `enabled` runs in an effect, so signals it reads re-subscribe.
pub fn use_motion_when(enabled: impl FnMut() -> bool + 'static) -> Motion {
    let (key, enabled_signal) = use_enabled_key(enabled);
    let active = use_signal(|| true);
    let known = use_signal(|| false);
    use_activity(
        move || key_selector(key, enabled_signal),
        Mode::Element,
        active,
        known,
    );
    Motion {
        key,
        active,
        known,
        enabled: enabled_signal,
    }
}

fn key_selector(key: Signal<String>, enabled: Signal<bool>) -> Option<String> {
    enabled().then(|| format!("[data-dx-motion-key=\"{}\"]", key.read().replace('"', "")))
}

fn use_enabled_key(mut enabled: impl FnMut() -> bool + 'static) -> (Signal<String>, Signal<bool>) {
    let key = crate::use_unique_id();
    let mut enabled_signal = use_signal(&mut enabled);
    use_effect(move || {
        let now = enabled();
        if *enabled_signal.peek() != now {
            enabled_signal.set(now);
        }
    });
    (key, enabled_signal)
}

/// The handle [`use_entered_view_when`] returns.
#[derive(Clone, Copy)]
pub struct EnterView {
    key: Signal<String>,
    entered: Signal<bool>,
}

impl EnterView {
    /// `false` until the element has been seen, then `true` (and it stays `true`: the owner
    /// reads it once to start its entrance and never needs to un-play it).
    pub fn entered(&self) -> ReadSignal<bool> {
        self.entered.into()
    }

    /// The lookup key to spread on the element. No `data-dx-motion` here: an entrance is not
    /// a loop and is never paused.
    pub fn attributes(&self) -> Vec<Attribute> {
        let key = self.key.cloned();
        attributes!(span { "data-dx-motion-key": key })
    }
}

/// One-shot entrances: `entered()` turns `true` once, when the element is actually on screen
/// (at least [`ENTER_RATIO`] of it, no pre-roll margin) and the tab is visible, and never
/// while the tab is hidden or the element is inside a skipped `content-visibility` subtree. An
/// element already in view at mount enters as soon as the browser has laid it out. After it
/// fires the shared observer lets go of the element. Charts play their load animation on it.
///
/// `enabled` false (no animation wanted) subscribes to nothing; turning it false after the
/// entrance has fired (the usual "latch it" pattern) tears the subscription down and keeps
/// the answer.
pub fn use_entered_view_when(enabled: impl FnMut() -> bool + 'static) -> EnterView {
    let (key, enabled_signal) = use_enabled_key(enabled);
    let entered = use_signal(|| false);
    let known = use_signal(|| false);
    use_activity(
        move || key_selector(key, enabled_signal),
        Mode::Enter,
        entered,
        known,
    );
    EnterView { key, entered }
}

/// Like [`use_motion`], for an element the component does not render itself: observes the
/// element whose DOM `id` is `element_id` (re-subscribes when it changes; an empty id observes
/// nothing). Used by the carousel's autoplay, which lives beside the scroller it should watch.
pub fn use_motion_active_for(element_id: impl Into<ReadSignal<String>>) -> ReadSignal<bool> {
    let element_id: ReadSignal<String> = element_id.into();
    let active = use_signal(|| true);
    let known = use_signal(|| false);
    use_activity(
        move || {
            let id = element_id.cloned();
            (!id.is_empty()).then(|| id_selector(&id))
        },
        Mode::Element,
        active,
        known,
    );
    active.into()
}

/// Whether the tab is visible. Ignores where the element is on the page: for a timer the user is
/// waiting on (a toast's auto-dismiss), which must keep counting while scrolled away but not while
/// nobody can read the page.
pub fn use_document_visible() -> ReadSignal<bool> {
    let active = use_signal(|| true);
    let known = use_signal(|| false);
    use_activity(|| Some(String::new()), Mode::Document, active, known);
    active.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_selector_escapes_quotes_and_backslashes() {
        assert_eq!(id_selector("a\"b\\c"), "[id=\"a\\\"b\\\\c\"]");
    }

    /// The registry script must not leave a listener behind: every `addEventListener` has its
    /// `removeEventListener` in `stop()`.
    #[test]
    fn the_registry_removes_what_it_adds() {
        assert_eq!(
            SCRIPT.matches("document.addEventListener").count(),
            SCRIPT.matches("document.removeEventListener").count()
        );
    }
}
