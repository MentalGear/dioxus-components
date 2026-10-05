//! `JsListeners` / `use_js_listeners`: the one sanctioned way to put a JS event listener on
//! `window`, `document` or any other node from Rust.
//!
//! A listener added from a `document::eval` script is owned by the PAGE, not by the Dioxus scope
//! that evaluated the script: dropping the component drops the task that listens to the eval,
//! but the `addEventListener` registration stays on its target for the life of the page, and a
//! script that is re-run (an effect, a second visit to the route) adds another one on top. That
//! is the same shape as the uncleared `setInterval` (`interval.rs`), and it was found in `form`
//! (6 adds, 0 removes), `sidebar`, `theme.rs`, `pointer.rs`, `scroll_lock.rs` and the focus trap
//! (backlog rows 149 and 159). For a non-passive `wheel`/`touchmove` one it also taxes every
//! scroll on the page: before the compositor may scroll, the main thread has to run the
//! handler (`scroll_lock.rs` kept two of those on `window` after the first modal ever opened).
//!
//! The fix is not "remember to write the `removeEventListener`" -- every hand-written script that
//! forgot it did so because the removal lived in a different place from the add, on a path nobody
//! exercised. Here the removal is derived, not written:
//!
//! * the script body registers through `listen(target, type, handler, options)` instead of
//!   `target.addEventListener(...)`; `listen` records how to undo that registration;
//! * the script ends when [`JsListeners`] is dropped (explicitly, or because the hook's
//!   component unmounted, or because the effect re-ran), and everything `listen` recorded is
//!   undone, in reverse order, whatever the body did or did not do;
//! * `cleanup(fn)` registers any other teardown (clearing a timer, resetting a flag) on the
//!   same path.
//!
//! Defaults that make the common mistake the wrong one to make: `listen` adds `{ passive: true }`
//! to `wheel`, `mousewheel`, `touchstart`, `touchmove` and `DOMMouseScroll` (the events whose
//! non-passive listeners block scrolling) unless the caller says `passive: false` itself, which
//! then needs a reason (`scripts/check-blocking-scroll-listeners.sh`). Other events keep the
//! browser default, because `passive: true` would also silently ignore their `preventDefault()`.
//! `listen` ignores a missing target (a `getElementById` that found nothing) and a call after
//! the teardown (a `requestAnimationFrame` that fires late).
//!
//! `scripts/check-js-listeners.sh` fails on a raw `addEventListener` in any Rust, JS or HTML
//! source of `primitives/` and `preview/` that neither lives in this file nor carries a
//! `listener-ok: <reason>` marker (a listener that genuinely lives as long as the page).
//! `playwright/oracle/tier2-html/listener-inventory.spec.ts` is the runtime backstop: it asks
//! Chromium which listeners `window` and `document` hold after every overlay and route has been
//! opened and closed again.
//!
//! ```rust,ignore
//! use_js_listeners(move || {
//!     Some(
//!         JsListeners::install(
//!             "const id = await dioxus.recv();
//!              listen(document, 'pointerdown', (e) => {
//!                  const root = document.getElementById(id);
//!                  if (root && !root.contains(e.target)) dioxus.send(true);
//!              }, true);",
//!         )
//!         .arg(root_id.cloned())
//!         .on_message(move |_: bool| on_dismiss()),
//!     )
//! });
//! ```

use dioxus::document::{self, Eval};
use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

/// The message that ends a script. Sent by [`JsListeners`]'s `Drop`; a stray `.arg(..)` the body
/// never read cannot be mistaken for it.
const TEARDOWN: &str = "__dx_listeners_off__";

/// Defines `listen` and `cleanup` and the bookkeeping behind them. `window.__dxJsListeners.live`
/// maps a registration id to its event type for as long as the registration is live, so a leak
/// can be named from the browser console.
const PRELUDE: &str = r#"
const __dxOff = [];
let __dxDone = false;
const __dxBlocking = { wheel: 1, mousewheel: 1, touchstart: 1, touchmove: 1, DOMMouseScroll: 1 };
const __dxReg = (window.__dxJsListeners = window.__dxJsListeners || { n: 0, live: new Map() });
const cleanup = (fn) => { if (__dxDone) fn(); else __dxOff.push(fn); };
const listen = (target, type, handler, options) => {
    if (!target || __dxDone) return;
    const o = typeof options === 'boolean' ? { capture: options } : Object.assign({}, options);
    if (o.passive === undefined && __dxBlocking[type]) o.passive = true;
    target.addEventListener(type, handler, o);
    const id = ++__dxReg.n;
    __dxReg.live.set(id, type);
    __dxOff.push(() => { target.removeEventListener(type, handler, o); __dxReg.live.delete(id); });
};
"#;

/// A script whose event listeners live exactly as long as this value does.
///
/// Build one with [`JsListeners::install`]; feed the script its parameters with
/// [`arg`](JsListeners::arg) (read in order with `await dioxus.recv()`); receive what it
/// `dioxus.send(..)`s with [`on_message`](JsListeners::on_message). Dropping the value removes
/// every listener the script registered through `listen`. Prefer [`use_js_listeners`], which
/// ties that drop to the calling component; hold a `JsListeners` yourself only when the
/// listeners follow something that is not a component (`scroll_lock.rs`: the last lock to
/// release).
#[must_use = "dropping a `JsListeners` removes its listeners at once"]
pub struct JsListeners {
    eval: Eval,
}

impl JsListeners {
    /// Runs `body` in the page. Inside it, `listen(target, type, handler, options)` registers a
    /// listener, `cleanup(fn)` registers other teardown, `dioxus.recv()` reads what
    /// [`arg`](Self::arg) sent and `dioxus.send(..)` reaches [`on_message`](Self::on_message).
    /// `body` is an `async` function body: `await` and an early `return` are allowed, and a
    /// `return` leaves the listeners it already registered in place until the teardown.
    ///
    /// Use `listen` for every listener, never `target.addEventListener` (the gate rejects it).
    pub fn install(body: &str) -> Self {
        let mut script = String::with_capacity(PRELUDE.len() + body.len() + 400);
        script.push_str(PRELUDE);
        script.push_str("try {\n await (async () => {\n");
        script.push_str(body);
        script.push_str("\n })();\n for (;;) { if ((await dioxus.recv()) === '");
        script.push_str(TEARDOWN);
        script.push_str("') break; }\n} finally {\n __dxDone = true;\n");
        script.push_str(
            " for (let i = __dxOff.length - 1; i >= 0; i--) { try { __dxOff[i](); } catch (_) {} }\n}",
        );
        Self {
            eval: document::eval(&script),
        }
    }

    /// Sends the next parameter the script reads with `await dioxus.recv()`.
    pub fn arg(self, value: impl Serialize) -> Self {
        let _ = self.eval.send(value);
        self
    }

    /// Calls `handler` for every `dioxus.send(..)` the script makes, in a task owned by the
    /// calling component's scope. The task ends with the scope, or when the script does.
    pub fn on_message<T: DeserializeOwned + 'static>(
        self,
        mut handler: impl FnMut(T) + 'static,
    ) -> Self {
        let mut eval = self.eval;
        spawn(async move {
            while let Ok(message) = eval.recv::<T>().await {
                handler(message);
            }
        });
        self
    }
}

impl JsListeners {
    /// Like [`on_message`](Self::on_message), but the task belongs to the app root instead of
    /// the calling component, for listeners whose owner is not a component
    /// (`pointer.rs`: the pointers currently held down).
    pub(crate) fn on_message_in_root<T: DeserializeOwned + 'static>(
        self,
        mut handler: impl FnMut(T) + 'static,
    ) -> Self {
        let mut eval = self.eval;
        crate::dioxus_core::Runtime::current().spawn(ScopeId::ROOT, async move {
            while let Ok(message) = eval.recv::<T>().await {
                handler(message);
            }
        });
        self
    }
}

impl Drop for JsListeners {
    fn drop(&mut self) {
        let _ = self.eval.send(TEARDOWN);
    }
}

/// Installs the listeners `setup` returns when the component mounts and again whenever a signal
/// `setup` read changes, and removes them when that happens and when the component unmounts.
///
/// `setup` runs inside an effect, so reading a signal in it (`id.cloned()`) subscribes to it:
/// a new id tears down the old listeners and installs new ones. Return `None` to install
/// nothing for the current state (an element that is not rendered yet).
pub fn use_js_listeners(mut setup: impl FnMut() -> Option<JsListeners> + 'static) {
    crate::use_effect_with_cleanup(move || {
        let installed = setup();
        move || drop(installed)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The teardown sentinel is a plain quoted JS string: nothing in it needs escaping, or the
    /// generated script would not parse.
    #[test]
    fn the_teardown_sentinel_is_safe_to_inline() {
        assert!(TEARDOWN
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_'));
    }

    /// The scroll-blocking events default to passive in `listen`; the set must cover exactly the
    /// ones `scripts/check-blocking-scroll-listeners.sh` knows about.
    #[test]
    fn the_prelude_defaults_the_scroll_blocking_events_to_passive() {
        for event in [
            "wheel",
            "mousewheel",
            "touchstart",
            "touchmove",
            "DOMMouseScroll",
        ] {
            assert!(PRELUDE.contains(&format!("{event}: 1")), "{event}");
        }
    }
}
