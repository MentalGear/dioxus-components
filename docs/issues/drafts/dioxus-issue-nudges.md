# Draft: two comment nudges for existing Dioxus issues

**Status:** drafted 2026-09-03, not filed. Both issue numbers below (**#2962**, **#4319**) are carried over from `docs/backlog.md` row 14's existing text, which predates this drafting session. **Neither number is independently corroborated anywhere in this repo's own tracking**: `complaints.md` (this repo's running list of Dioxus framework friction) documents the underlying *behavior* each comment below describes (the ancestor-traversal gap, the need for a typed dialog-close/open API) but cites no GitHub issue numbers at all, and no source file in `primitives/src/` references either number. Per this task's own instruction: **do not treat #2962/#4319 as verified — confirm both numbers against Dioxus's actual issue tracker before posting either comment**, and if either is wrong, search `DioxusLabs/dioxus`'s issues for the correct one (search terms below) rather than guessing a replacement.

---

**Target repo:** `DioxusLabs/dioxus`.

## Comment 1 — issue **#2962** (`onclose` event) — number unverified, confirm before posting

**Suggested search if #2962 turns out wrong:** issues mentioning `<dialog>`, `onclose`, or the native `close`/`cancel` DOM events in the context of Dioxus's event system.

> We hit exactly this gap building native `<dialog>` support for our modal `Dialog`/`AlertDialog` components (`MentalGear/dioxus-components`, a fork). Since there's no typed `onclose` (or `oncancel`) event exposed on `dialog` elements, we worked around it with a small `document::eval` bridge: `use_dialog_close_sync` (`primitives/src/lib.rs`) opens an eval channel, adds a native `close` event listener to the `<dialog>` element by id from the JS side, and forwards a signal back into Rust via `dioxus.send`/`await dioxus.recv()` whenever that native event fires — closing the loop so a `showModal()`-driven dialog dismissed by the browser itself (Escape's default `cancel` action, a `method="dialog"` form submission, or any other close not initiated by our own code) doesn't leave our `open` signal stranded at `true`. It works, but it's more moving parts than a first-class `onclose`/`oncancel` prop on `dialog` elements would need: an eval channel, a cleanup-on-unmount `send`, and a listener registered/torn down by hand.
>
> A typed `onclose`/`oncancel` handler on `dialog` elements (mirroring how other native events already get typed Dioxus handlers) would let us delete this bridge entirely and just write `dialog { onclose: move |_| set_open.call(false) }`. Happy to share the exact construction (`use_dialog_close_sync`, `use_dialog_open_driver`) if it's useful as a concrete "here's what a user has to build without this" reference, or to test against an experimental implementation if one exists.

## Comment 2 — issue **#4319** (dialog methods) — number unverified, confirm before posting

**Suggested search if #4319 turns out wrong:** issues about calling native `HTMLDialogElement` methods (`showModal()`, `close()`, `show()`) from Dioxus, or about binding a `dialog` element's `open` attribute declaratively alongside imperative modal control.

> Same area as the `onclose` gap (issue #2962, linked above): calling `showModal()`/`close()` on a `<dialog>` element currently has to go through a `document::eval` bridge rather than a typed method call. Our construction (`use_dialog_open_driver`, `primitives/src/lib.rs`) drives this from a Rust `open` signal: an effect re-runs `document::eval` with a small inline script that looks the element up by id, checks its `.open` DOM property, and calls `showModal()`/`close()` only when that property disagrees with the signal (calling either method redundantly throws `InvalidStateError`, so the guard is load-bearing, not defensive-only).
>
> One trap worth flagging concretely, since it cost us real debugging time and might be useful signal for whatever a typed API's design ends up being: we found that binding a `dialog` element's `open` attribute **declaratively** (`dialog { open: some_signal }`) in the same build as also calling `showModal()` imperatively **doesn't** produce a runtime error or a warning — it silently makes `showModal()` a no-op, because Dioxus commits the declarative `open` attribute during render, before any effect driving `showModal()` gets a chance to run, so by the time an imperative call checks `dialog.open` it's already `true` and the call is skipped. If a typed method API (`dialog_ref.show_modal()`, or similar) ships, it'd be worth either documenting this interaction explicitly or making the two mechanisms mutually exclusive at the type level, since the failure mode (dialog looks "open" per its attribute, but `showModal()`'s actual modal behavior — top layer, focus trap, `::backdrop` — never engaged) is easy to miss in testing and only shows up as "the dialog doesn't trap focus" rather than a hard error.
>
> A typed `dialog_ref.show_modal()`/`.close()` (or a `use_dialog_ref`-style hook returning handles to both) would let us drop the eval bridge the same way a typed `onclose` would for the paired issue.

## Before filing

- [ ] **Verify #2962 and #4319 actually exist and are about these topics** on `DioxusLabs/dioxus`'s issue tracker — neither number is corroborated in this repo's own docs or code, only carried over from `docs/backlog.md` row 14's pre-existing text. If either number is wrong, search using the terms above and substitute the correct issue (or file new ones if none exist).
- [ ] Re-verify against the latest Dioxus release — either gap may already be closed.
- [ ] Confirm both issues are still open (not already closed/fixed) before posting either comment.
- [ ] Link a permalink to `primitives/src/lib.rs`'s `use_dialog_close_sync`/`use_dialog_open_driver` at the commit these comments are posted from (both currently `#[cfg(feature = "web")]`-gated private functions in that file, not part of the public API).
