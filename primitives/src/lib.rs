#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::core::{current_scope_id, use_drop};
use dioxus::prelude::*;
use dioxus::prelude::{asset, manganis, Asset};
use dioxus_core::AttributeValue::Text;
use time::OffsetDateTime;

pub use dioxus_attributes;

pub mod accordion;
pub mod alert_dialog;
pub mod aspect_ratio;
pub mod avatar;
pub mod calendar;
pub mod checkbox;
pub mod collapsible;
mod collection;
pub mod color_picker;
pub mod combobox;
pub mod context_menu;
pub mod date_picker;
pub mod dialog;
pub mod drag_and_drop_list;
pub mod dropdown_menu;
pub mod hover_card;
pub mod label;
mod listbox;
pub mod menubar;
mod move_interaction;
#[cfg(feature = "router")]
pub mod navbar;
mod pointer;
pub mod popover;
mod portal;
pub mod progress;
pub mod radio_group;
pub mod scroll_area;
mod scroll_lock;
pub mod select;
mod selectable;
mod selection;
pub mod separator;
pub mod slider;
pub mod switch;
pub mod tabs;
pub mod tag_group;
pub mod toast;
pub mod toggle;
pub mod toggle_group;
pub mod toolbar;
pub mod tooltip;
mod top_layer;
pub(crate) mod r#virtual;
pub mod virtual_list;

pub(crate) const FOCUS_TRAP_JS: Asset = asset!("/src/js/focus-trap.js");

/// Generate a runtime-unique id.
fn use_unique_id() -> Signal<String> {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    #[allow(unused_mut)]
    let mut initial_value = use_hook(|| {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let id_str = format!("dxc-{id}");
        id_str
    });

    fullstack! {
        let server_id = dioxus::prelude::use_server_cached(move || {
            initial_value.clone()
        });
        initial_value = server_id;
    }
    use_signal(|| initial_value)
}

// Elements can only have one id so if the user provides their own, we must use it as the aria id.
fn use_id_or<T: Clone + PartialEq + 'static>(
    mut gen_id: Signal<T>,
    user_id: ReadSignal<Option<T>>,
) -> Memo<T> {
    // First, check if we have a user-provided ID
    let has_user_id = use_memo(move || user_id().is_some());

    // If we have a user ID, update the gen_id in an effect
    use_effect(move || {
        if let Some(id) = user_id() {
            gen_id.set(id);
        }
    });

    // Return the appropriate ID
    use_memo(move || {
        if has_user_id() {
            user_id().unwrap()
        } else {
            gen_id.peek().clone()
        }
    })
}

/// A controlled-or-uncontrolled prop trio: external value signal,
/// fallback default signal, and change callback. Bundles the three
/// pieces that always travel together when forwarding props into
/// internal hooks like [`use_controlled`].
#[derive(Clone, Copy)]
pub(crate) struct Controlled<T: Clone + PartialEq + 'static> {
    pub(crate) value: ReadSignal<Option<T>>,
    pub(crate) default: ReadSignal<T>,
    pub(crate) on_change: Callback<T>,
}

/// Allows some state to be either controlled or uncontrolled.
pub fn use_controlled<T: Clone + PartialEq + 'static>(
    prop: ReadSignal<Option<T>>,
    default: T,
    on_change: Callback<T>,
) -> (Memo<T>, Callback<T>) {
    let mut internal_value = use_signal(|| prop.cloned().unwrap_or(default));
    let value = use_memo(move || prop.cloned().unwrap_or_else(&*internal_value));

    let set_value = use_callback(move |x: T| {
        internal_value.set(x.clone());
        on_change.call(x);
    });

    (value, set_value)
}

/// Run some cleanup code when the component is unmounted if the effect was run.
fn use_effect_cleanup<F: FnOnce() + 'static>(#[allow(unused)] cleanup: F) {
    client!(crate::dioxus_core::use_drop(cleanup))
}

/// Run some cleanup code when the component is unmounted if the effect was run.
fn use_effect_with_cleanup<F: FnMut() -> C + 'static, C: FnOnce() + 'static>(mut effect: F) {
    let mut cleanup = use_hook(|| CopyValue::new(None as Option<C>));
    use_effect(move || {
        if let Some(cleanup) = cleanup.take() {
            cleanup();
        }
        cleanup.set(Some(effect()));
    });
    client!(crate::dioxus_core::use_drop(move || {
        if let Some(cleanup) = cleanup.take() {
            cleanup();
        }
    }))
}

/// A stack of escape listeners to allow only the top-most listener to be called.
#[derive(Clone)]
struct EscapeListenerStack(Rc<RefCell<Vec<ScopeId>>>);

fn use_global_escape_listener(mut on_escape: impl FnMut() + Clone + 'static) {
    let scope_id = current_scope_id();
    let stack = use_hook(move || {
        // Get or create the escape listener stack
        let stack: EscapeListenerStack = try_consume_context()
            .unwrap_or_else(|| provide_context(EscapeListenerStack(Default::default())));
        // Push the current scope onto the stack
        stack.0.borrow_mut().push(scope_id);
        stack
    });
    // Remove the current scope id from the stack when we unmount
    use_drop({
        let stack = stack.clone();
        move || {
            let mut stack = stack.0.borrow_mut();
            stack.retain(|id| *id != scope_id);
        }
    });
    use_global_keydown_listener("Escape", move || {
        // Only call the listener if this component is on top of the stack
        let stack = stack.0.borrow();
        if stack.last() == Some(&scope_id) {
            on_escape();
        }
    });
}

fn use_global_keydown_listener(key: &'static str, on_escape: impl FnMut() + Clone + 'static) {
    use_effect_with_cleanup(move || {
        let mut escape = document::eval(
            "let targetKey = await dioxus.recv();
            function listener(event) {
                if (event.key === targetKey) {
                    event.preventDefault();
                    dioxus.send(true);
                }
            }
            document.addEventListener('keydown', listener);
            await dioxus.recv();
            document.removeEventListener('keydown', listener);",
        );
        let _ = escape.send(key);
        let mut on_escape = on_escape.clone();
        spawn(async move {
            while let Ok(true) = escape.recv().await {
                on_escape();
            }
        });
        move || _ = escape.send(true)
    });
}

/// Light-dismiss when pointerdown/focusin lands outside the element with the given `id`.
/// `id` should be the id of the popover/dialog root that contains every "inside" element.
fn use_outside_dismiss(
    id: impl Readable<Target = String> + Copy + 'static,
    on_dismiss: impl FnMut() + Clone + 'static,
) {
    use_effect_with_cleanup(move || {
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            // A pointer press outside the root is always a dismiss, even when it
            // lands on an ancestor element that wraps the popover (e.g. a scroll
            // container around it).
            const onPointer = e => {
                const root = document.getElementById(id);
                if (root && !root.contains(e.target)) dioxus.send(true);
            };
            // Focus moving outside the root dismisses too (e.g. tabbing away), but
            // ignore focus that lands on an *ancestor* of the root. Clicking the
            // popover's own non-focusable background blurs the focused control and
            // the browser moves focus to the nearest focusable ancestor, which
            // still contains the popover — that is not a real focus-out.
            const onFocus = e => {
                const root = document.getElementById(id);
                if (root && !root.contains(e.target) && !e.target.contains(root)) dioxus.send(true);
            };
            document.addEventListener('pointerdown', onPointer, true);
            document.addEventListener('focusin', onFocus, true);
            await dioxus.recv();
            document.removeEventListener('pointerdown', onPointer, true);
            document.removeEventListener('focusin', onFocus, true);",
        );
        let _ = eval.send(id.cloned());
        let mut on_dismiss = on_dismiss.clone();
        spawn(async move {
            while let Ok(true) = eval.recv().await {
                on_dismiss();
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Returns the previous value of a reactive signal.
///
/// Ported from dignifiedquire/dx-components (MIT OR Apache-2.0),
/// `primitives/src/lib.rs` @ 5af3cc292559a0e8d73c7b9a827c4ca08ef34d99
/// (`use_previous`, matching upstream Radix's `usePrevious(value)`).
/// Adapted: none -- taken as-is.
///
/// On each render, if `value` has changed since the last observed value, the
/// previous value is stored and returned. The initial previous value equals
/// the initial `value`.
pub fn use_previous<T: Clone + PartialEq + 'static>(value: ReadSignal<T>) -> Memo<T> {
    let mut prev = use_signal(|| value.cloned());
    let mut last_seen = use_signal(|| value.cloned());

    use_memo(move || {
        let current = value.cloned();
        let seen = last_seen.cloned();
        if current != seen {
            prev.set(seen);
            last_seen.set(current);
        }
        prev.cloned()
    })
}

/// Refocus the trigger when a menu-family surface closes, unless something
/// outside caused the close.
///
/// Ported from dignifiedquire/dx-components (MIT OR Apache-2.0),
/// `primitives/src/lib.rs` @ 5af3cc292559a0e8d73c7b9a827c4ca08ef34d99
/// (`use_refocus_on_close_unless`). Adapted: none -- taken as-is; only the
/// per-component wiring of `interacted_outside` is new (docs/plan.md
/// Phase 3.1).
///
/// Matches Radix's `onCloseAutoFocus` on `DropdownMenuContent`/
/// `ContextMenuContent`/etc.: when `open` transitions from `true` to
/// `false`, focus is returned to the trigger element by id -- *unless*
/// `interacted_outside` is `true`, meaning the close was caused by the user
/// clicking or focusing something outside the menu, in which case focus
/// should stay wherever the user put it rather than being yanked back to
/// the trigger.
pub(crate) fn use_refocus_on_close_unless(
    open: Memo<bool>,
    trigger_id: Signal<String>,
    interacted_outside: ReadSignal<bool>,
) {
    let prev_open = use_previous(open.into());
    use_effect(move || {
        if prev_open() && !open() && !interacted_outside() {
            let id = trigger_id();
            document::eval(&format!(
                "var e=document.getElementById('{id}');if(e)e.focus()"
            ));
        }
    });
}

/// Listens for the native `reset` event on the `<form>` that owns the element
/// with the given `id`, calling `on_reset` whenever the form resets.
///
/// The browser's own form-reset algorithm only restores a form control's own
/// DOM state (checkedness/selectedness derived from its `defaultChecked`/
/// `defaultSelected` content attribute) -- it has no way to know about, let
/// alone update, this crate's own Dioxus signals. Radix's form-participation
/// components (`Checkbox`, `RadioGroup`, `Select`, `Switch`) solve this the
/// same way: a `reset` listener on the hidden control's owning form that
/// pushes the component's visible/Rust state back to its default.
///
/// Looks up the owning form via `element.form` (present on `input`/`select`)
/// falling back to `element.closest('form')` for elements -- like a
/// `RadioGroup`'s container `<div>` -- that are not themselves form-associated.
fn use_form_reset_listener(
    id: impl Readable<Target = String> + Copy + 'static,
    on_reset: impl FnMut() + Clone + 'static,
) {
    use_effect_with_cleanup(move || {
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            const el = document.getElementById(id);
            const form = el && (el.form || el.closest('form'));
            const listener = () => dioxus.send(true);
            if (form) form.addEventListener('reset', listener);
            await dioxus.recv();
            if (form) form.removeEventListener('reset', listener);",
        );
        let _ = eval.send(id.cloned());
        let mut on_reset = on_reset.clone();
        spawn(async move {
            while let Ok(true) = eval.recv().await {
                on_reset();
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Drive `showModal()`/`close()` on the `<dialog>` element with the given
/// `id` from `open`, guarded by the element's own `.open` DOM property.
///
/// Docs/plan.md Phase 4.2. Ported *by construction*, not by copying code,
/// from `docs/phase4-spike-findings.md`'s `use_dialog_open_driver`
/// (experiment 2's fix for experiment 1's stranded-signal defect, see
/// `docs/recommended-implementations.md` Caveat 1). The guard (checking
/// `dialog.open` before calling either method) mirrors
/// [`crate::top_layer::use_popover_sync`]'s `:popover-open` check for the
/// identical reason: a redundant call throws `InvalidStateError` (spike
/// experiment 3b).
///
/// Never bind the `<dialog>` element's `open` attribute declaratively
/// alongside this hook in the same build -- spike experiment 3c found that
/// combination doesn't crash, it silently *skips* `showModal()` (Dioxus
/// commits the declarative attribute during render, before this effect
/// runs, so the guard's own condition is already false by the time it
/// checks). That is exactly why this hook only exists in the
/// `#[cfg(target_family = "wasm")]` leaf of `dialog.rs`/`alert_dialog.rs`'s
/// component split (`docs/phase4-spike-findings.md` Construction B) -- the
/// `not(wasm)` arm never binds `open` as an attribute either, since that
/// arm renders a plain `div`, not a `<dialog>`, at all.
///
/// Gated on `target_family = "wasm"` for the same reason
/// [`crate::top_layer::use_popover_sync`] is (see that function's doc): it
/// is the only axis this repo's CI can build and check both sides of today.
/// The correct production axis is a renderer Cargo feature mirroring this
/// crate's own `web` feature, matching Construction B's finding that
/// `dioxus-desktop` is a non-wasm binary with a real, working webview
/// `eval` and belongs on this same arm, not on native/Blitz's no-op one.
#[cfg(target_family = "wasm")]
fn use_dialog_open_driver(
    id: impl Readable<Target = String> + Copy + 'static,
    open: impl Readable<Target = bool> + Copy + 'static,
) {
    use_effect(move || {
        let want_open = open.cloned();
        let id = id.cloned();
        document::eval(&format!(
            "const dialog = document.getElementById('{id}');
            if (!dialog) return;
            if ({want_open} && !dialog.open) dialog.showModal();
            if (!{want_open} && dialog.open) dialog.close();"
        ));
    });
}

/// Sync the `<dialog>` element's native `close` event -- fired on Escape's
/// default `cancel` action, a `::backdrop`/outside-click `close()` call we
/// drive ourselves (see `dialog.rs`'s `use_dialog_backdrop_dismiss`), a
/// `method="dialog"` form submission, or any other close, browser- or
/// script-driven alike -- back into `set_open`, so the Rust signal can never
/// strand.
///
/// This is the fix for the exact defect class
/// `docs/recommended-implementations.md` Caveat 1 documents for upstream's
/// first `<dialog>` (`b3f6de53`): a one-way `showModal()`/`close()` binding
/// with no listener on the browser's own `close` event, so a native Escape
/// left the `open` signal stranded at `true` and the dialog could never
/// reopen (reproduced by execution in `docs/phase4-spike-findings.md`
/// experiment 1; fixed the same way here as experiment 2's
/// `use_dialog_close_sync`). Same eval-channel shape as
/// `use_form_reset_listener` above and
/// [`crate::top_layer::use_popover_sync`]'s browser-to-signal half.
///
/// See [`use_dialog_open_driver`]'s doc for why this is `#[cfg]`-gated the
/// same way.
#[cfg(target_family = "wasm")]
fn use_dialog_close_sync(
    id: impl Readable<Target = String> + Copy + 'static,
    set_open: Callback<bool>,
) {
    use_effect_with_cleanup(move || {
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            const dialog = document.getElementById(id);
            const onClose = () => dioxus.send(true);
            dialog.addEventListener('close', onClose);
            await dioxus.recv();
            dialog.removeEventListener('close', onClose);",
        );
        let _ = eval.send(id.cloned());
        spawn(async move {
            while let Ok(true) = eval.recv::<bool>().await {
                set_open.call(false);
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Whether a completed (or aborted) close-animation cycle should still write
/// its result to `show_in_dom`.
///
/// Each run of [`use_animated_open`]'s effect -- whether it opens or closes --
/// bumps a generation counter before spawning its async work. A cycle is
/// stale, and its result must be dropped, once a newer cycle has started:
/// that newer cycle already owns `show_in_dom` (an open cycle sets it
/// synchronously; a closing cycle will set it when its own animation
/// settles), so applying a superseded cycle's result would clobber fresher
/// state with stale data. A cycle that is still current -- including one
/// whose animation was aborted by something other than a new open/close,
/// e.g. a script directly cancelling the animation -- must still apply, or
/// the element leaks in the DOM forever with no cycle left to unmount it.
fn should_apply_animation_result(spawned_generation: u64, current_generation: u64) -> bool {
    spawned_generation == current_generation
}

fn use_animated_open(
    id: impl Readable<Target = String> + Copy + 'static,
    open: impl Readable<Target = bool> + Copy + 'static,
) -> impl Fn() -> bool + Copy {
    // Show in dom is a few frames behind the open signal to allow for the animation to start.
    // If it does start, we wait for the animation to finish before showing removing the element from the DOM.
    let mut show_in_dom = use_signal(|| false);

    // Bumped at the top of every effect run (open or close) so a closing
    // task still in flight when a newer cycle starts can tell it is stale.
    // Written through `.write()` / read through `.peek()` only -- never
    // `.read()` -- so this effect never subscribes to its own counter.
    let mut generation = use_signal(|| 0u64);

    use_effect(move || {
        *generation.write() += 1;
        let my_generation = *generation.peek();

        let open = open.cloned();
        if open {
            show_in_dom.set(open);
        } else {
            spawn(async move {
                let id = id.cloned();
                let mut eval = dioxus::document::eval(
                    r#"const id = await dioxus.recv();
                    await new Promise(resolve => requestAnimationFrame(resolve));
                    const element = document.getElementById(id);
                    if (!element) {
                        dioxus.send(true);
                        return;
                    }
                    const anims = element.getAnimations();
                    if (anims.length > 0) {
                        // Hold the element in the DOM for an extra ~250ms after
                        // the close animation finishes so external observers
                        // (e.g. Playwright polling) reliably see the
                        // data-state="closed" element before it unmounts. The
                        // element is opacity:0 / pointer-events:none here, so
                        // the user sees nothing.
                        const hold = () => new Promise(r => setTimeout(r, 250));
                        Promise.all(anims.map((a) => a.finished))
                            .then(hold)
                            .then(() => dioxus.send(true))
                            .catch(() => {
                                // Animation aborted -- most often because a
                                // newer open/close cycle re-triggered it, but
                                // possibly because something else (e.g. a
                                // script) cancelled it directly. Always send
                                // so this task's recv() completes either way;
                                // the generation check on the Rust side is
                                // what decides whether the result still
                                // applies.
                                dioxus.send(false);
                            });
                    } else {
                        dioxus.send(true);
                    }"#,
                );
                let _ = eval.send(id);
                let _ = eval.recv::<bool>().await;

                if should_apply_animation_result(my_generation, *generation.peek()) {
                    show_in_dom.set(open);
                }
            });
        }
    });

    move || show_in_dom()
}

#[cfg(test)]
mod use_animated_open_tests {
    use super::should_apply_animation_result;

    #[test]
    fn current_generation_applies() {
        assert!(should_apply_animation_result(3, 3));
    }

    #[test]
    fn superseded_generation_is_skipped() {
        // A newer cycle started (generation moved on) while this one was
        // still awaiting its animation -- applying it now would clobber the
        // newer cycle's state.
        assert!(!should_apply_animation_result(3, 4));
    }
}

/// The side where the content will be displayed relative to the trigger
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentSide {
    /// The content will appear above the trigger
    Top,
    /// The content will appear to the right of the trigger
    Right,
    /// The content will appear below the trigger
    Bottom,
    /// The content will appear to the left of the trigger
    Left,
}

impl ContentSide {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }
}

/// The alignment of the content relative to the trigger
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentAlign {
    /// The content will be aligned to the start of the trigger
    Start,
    /// The content will be centered relative to the trigger
    Center,
    /// The content will be aligned to the end of the trigger
    End,
}

impl ContentAlign {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

pub(crate) trait LocalDateExt {
    /// A small extension method function to get the local date with a fallback to UTC date if this fails
    fn now_local_date() -> time::Date;
}

impl LocalDateExt for time::OffsetDateTime {
    fn now_local_date() -> time::Date {
        OffsetDateTime::now_local()
            .map(|x| x.date())
            .unwrap_or_else(|_| time::UtcDateTime::now().date())
    }
}

/// Merge multiple attribute vectors.
///
/// Rules:
/// - Later lists win for the same (name, namespace) pair.
/// - `class` is concatenated with a single space separator (trimmed); last wins for volatility flag.
/// - Other attributes are overwritten by the last occurrence.
///
/// TODO: event handler attributes are not merged/combined yet.
pub fn merge_attributes(mut lists: Vec<Vec<Attribute>>) -> Vec<Attribute> {
    let mut merged = Vec::new();
    // The inputs are usually sorted by name, so we can do a k-way merge cheaply
    for list in &mut lists {
        list.sort_by_key(|a| a.name);
    }
    let mut iters: Vec<_> = lists
        .into_iter()
        .map(|l| l.into_iter().peekable())
        .collect();

    loop {
        // Find the minimum name among all current heads
        let min_name = iters
            .iter_mut()
            .filter_map(|it| it.peek().map(|a| a.name))
            .min();

        let Some(min_name) = min_name else {
            break;
        };

        // Collect all attributes with this name, grouped by namespace
        let mut by_namespace: Vec<Attribute> = Vec::new();

        for iter in &mut iters {
            while iter.peek().map(|a| a.name) == Some(min_name) {
                let attr = iter.next().unwrap();
                if let Some(existing) = by_namespace
                    .iter_mut()
                    .find(|a| a.namespace == attr.namespace)
                {
                    if attr.name == "class" {
                        let was_volatile = existing.volatile;
                        *existing = match (&existing.value, &attr.value) {
                            (Text(a), Text(b)) => Attribute {
                                name: attr.name,
                                namespace: attr.namespace,
                                volatile: was_volatile || attr.volatile,
                                value: Text(join_class(a, b)),
                            },
                            _ => attr,
                        };
                    } else {
                        *existing = attr;
                    }
                } else {
                    by_namespace.push(attr);
                }
            }
        }

        merged.extend(by_namespace);
    }

    merged
}

fn join_class(a: &str, b: &str) -> String {
    let (a, b) = (a.trim(), b.trim());
    if !a.is_empty() && !b.is_empty() {
        format!("{a} {b}")
    } else {
        format!("{a}{b}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(name: &'static str, value: &str) -> Attribute {
        Attribute {
            name,
            namespace: None,
            volatile: false,
            value: Text(value.to_string()),
        }
    }

    fn get_value(attr: &Attribute) -> &str {
        match &attr.value {
            Text(s) => s,
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn merge_empty_lists() {
        let result = merge_attributes(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_single_list() {
        let result = merge_attributes(vec![vec![attr("a", "1"), attr("b", "2")]]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[1].name, "b");
    }

    #[test]
    fn merge_preserves_sorted_order() {
        let result = merge_attributes(vec![
            vec![attr("a", "1"), attr("c", "3")],
            vec![attr("b", "2"), attr("d", "4")],
        ]);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "a");
        assert_eq!(result[1].name, "b");
        assert_eq!(result[2].name, "c");
        assert_eq!(result[3].name, "d");
    }

    #[test]
    fn later_list_overwrites() {
        let result = merge_attributes(vec![vec![attr("a", "first")], vec![attr("a", "second")]]);
        assert_eq!(result.len(), 1);
        assert_eq!(get_value(&result[0]), "second");
    }

    #[test]
    fn class_attributes_are_merged() {
        let result = merge_attributes(vec![vec![attr("class", "foo")], vec![attr("class", "bar")]]);
        assert_eq!(result.len(), 1);
        assert_eq!(get_value(&result[0]), "foo bar");
    }

    #[test]
    fn class_merge_trims_whitespace() {
        let result = merge_attributes(vec![
            vec![attr("class", "  foo  ")],
            vec![attr("class", "  bar  ")],
        ]);
        assert_eq!(get_value(&result[0]), "foo bar");
    }

    #[test]
    fn class_merge_handles_empty() {
        let result = merge_attributes(vec![vec![attr("class", "")], vec![attr("class", "bar")]]);
        assert_eq!(get_value(&result[0]), "bar");
    }

    #[test]
    fn mixed_attributes() {
        let result = merge_attributes(vec![
            vec![attr("class", "a"), attr("id", "x")],
            vec![attr("class", "b"), attr("id", "y")],
        ]);
        assert_eq!(result.len(), 2);
        // Should be sorted by name
        assert_eq!(result[0].name, "class");
        assert_eq!(result[1].name, "id");
        // class merged, id overwritten
        assert_eq!(get_value(&result[0]), "a b");
        assert_eq!(get_value(&result[1]), "y");
    }

    #[test]
    fn unsorted_input_still_works() {
        // Even if inputs aren't sorted, the function should handle it
        let result = merge_attributes(vec![
            vec![attr("z", "1"), attr("a", "2")],
            vec![attr("m", "3")],
        ]);
        assert_eq!(result.len(), 3);
        // Output should be sorted
        assert_eq!(result[0].name, "a");
        assert_eq!(result[1].name, "m");
        assert_eq!(result[2].name, "z");
    }

    #[test]
    fn volatile_flag_preserved_on_class_merge() {
        let mut a1 = attr("class", "foo");
        a1.volatile = true;
        let a2 = attr("class", "bar");

        let result = merge_attributes(vec![vec![a1], vec![a2]]);
        assert!(result[0].volatile);
    }
}
