# Capability gaps in `dioxus-primitives`, and which forks closed them

**Scan date:** 2026-08-29 · **Baseline:** `bf007c1` (upstream `main`, unchanged since 2026-06-29)
**Verification level:** static, **except focus restore, which has now been executed against the running app** (see below). Other claims are backed by reading the current source on `main` and in fork refs, not by running them.

Companion to [`adopt-fork-fixes.md`](./adopt-fork-fixes.md), which covers *bug fixes*. This document covers *missing capabilities* — behaviour a mature headless component library is expected to have, that upstream does not implement, and that at least one fork does.

---

## TL;DR

Three findings outrank everything in the fix report:

1. **`RadioGroup` and `Select` accept `name` and `required` props, document them as "for form submission", and silently ignore them.** A developer following the documented API ships a form that omits the field. Two forks fixed `RadioGroup` independently; the patch is single-file.
2. **No overlay does collision detection.** Placement is static CSS keyed off `data-side`. A `Popover`, `Select`, `Tooltip` or `DropdownMenu` near a viewport edge renders off-screen, and `ContextMenu` opens at raw click coordinates with no clamping.
3. **Closing a `DropdownMenu`, `ContextMenu`, `Menubar` or `Select` never returns focus to the trigger.** Dialog and Popover restore focus correctly; the menu family never got it. **Confirmed by execution** — four failing conformance tests against a passing Dialog control ([details](#confirmed-by-execution--and-the-mechanism-was-not-what-i-predicted)).

**One fork is the source for nearly all of it.** `dignifiedquire/dx-components` is a systematic Radix-parity rewrite whose accessibility modules are deliberately standalone files — `scroll_lock.rs` (58 lines), `aria_hidden.rs` (91), `typeahead.rs` (78), `direction.rs` (83) — each depending only on helpers upstream already has.

---

## The layer map

Worth stating, because it determines where a fix can even go:

```
DioxusLabs/dioxus  (framework, main is on 0.8.0-alpha.1)
        ↓
dioxus-primitives  ← primitives/ IN THIS REPO. All accessibility behaviour lives here.
        ↓
preview/           (styled showcase layer)
```

There is no separate primitives repository — `primitives/Cargo.toml` declares `name = "dioxus-primitives"` and it is a workspace member here. `DioxusLabs/dioxus-primitives` and `DioxusLabs/primitives` do not exist.

**The framework cannot help with any gap below.** Checked against `DioxusLabs/dioxus@main`: no body/scroll API (`scroll_lock`, `set_body_style`, `body().style` — no hits in `packages/document` or `packages/web`), and no portal support (`use_portal`/`PortalIn`/`PortalOut` — no hits anywhere), so `complaints.md`'s "Need Portals" entry is still open. `document::eval` (`packages/document/src/lib.rs:29`) remains the only escape hatch, which is why `primitives/src` reaches for it in 14 places and why both forks implement scroll lock in JavaScript. Fixing these upstream in the framework would mean designing new public API — a different project, not on the critical path.

---

## Priority 1 — form controls that lie

Severity comes from the API *promising* the behaviour. A missing feature is an inconvenience; a documented prop that does nothing is a trap that surfaces in production.

| Component | Props declared | Hidden native input? | Verdict |
|---|---|---|---|
| `Checkbox` | `name`, `value`, `required` (`checkbox.rs:70-80`) | **Yes** — `BubbleInput` renders a real `<input type="checkbox">` (`checkbox.rs:262-296`) | Correct — the reference implementation |
| `Switch` | `name`, `value`, `required` (`switch.rs:21-27`) | Yes (`switch.rs:121-130`), but **`required` is never forwarded** — only `aria_required` on the button (`switch.rs:92`) | Partial: no native constraint validation |
| `RadioGroup` | `name` (`radio_group.rs:93`, doc: *"The name attribute for form submission"*), `required` (`:97`) | **No.** `props.name` appears nowhere else in the file. Items are `button role="radio"` with no backing input | **Silent failure** |
| `Select` / `SelectMulti` | `name` (`select.rs:52,103`, doc: *"Name of the select for form submission"*); no `required` prop exists | **No.** `props.name` is never referenced in the render body | **Silent failure — the most flagrant** |
| `Slider`, `ToggleGroup`, `Combobox` | none | No | Absent but honest |

**Available fixes.** `RadioGroup` was fixed independently in two forks, in the same shape — a per-item hidden `<input type="radio">` carrying `name`/`value`/`checked`/`required`/`disabled`, which also gives correct native group semantics: `dignifiedquire@radio_group.rs:267-279` and `sarendipitee@radio_group.rs:338-348`. Both are single-file and dependency-free — **adoptable as-is**, modulo `ReadSignal` vs `bool` prop-type differences.

`Select` was fixed only by `dignifiedquire` (`select.rs:158-186`), rendering a hidden native `<select>` with mirrored `<option>` children — Radix's "BubbleSelect" pattern, and the more correct fix since a real `<select required>` gets full browser validation UI. **Adoptable with rework**: it needs a `required` prop added to `SelectProps`/`SelectMultiProps` (upstream has none), and upstream's `Select<T>` is generic while the fork stringifies via `text_value`, so value-type handling needs review.

`Switch`'s missing `required` is a one-line fix.

---

## Priority 2 — accessibility behaviour

### Focus restore on close — widest blast radius

**Present** for `Dialog`, `AlertDialog` and modal `Popover`: the vendored `focus-trap.ts` captures `document.activeElement` in its constructor and calls `.focus()` on `remove()`, wired at `dialog.rs:245-249`, `alert_dialog.rs:195-199`, `popover.rs:234-238`.

**Absent** for `DropdownMenu`, `ContextMenu`, `Menubar` and `Select` — none of them call `createFocusTrap`. They use the roving-tabindex machinery in `collection.rs`, which moves *real DOM focus* onto items as the user arrow-keys. On close that focused node unmounts with the content, and nothing hands focus back: searching those files for restore-focus/`activeElement` returns nothing. Per browser behaviour focus falls to `<body>`, so a keyboard user must Tab from the top of the page to get back.

`dignifiedquire` closes this with `use_refocus_on_close_unless` (`lib.rs:236-254`, wired at `dropdown_menu.rs:748`), matching Radix's `onCloseAutoFocus` semantics — restore focus to the trigger *unless* the close was caused by interacting outside. **Adoptable with rework**: the helper is ~15 lines and drops onto upstream's existing `trigger_id` fields, but the `interacted_outside` signal must be threaded through the dismiss path, which upstream's `use_outside_dismiss` doesn't currently expose. A cruder interim fix — focus `trigger_id` at each `set_open.call(false)` site — captures most of the value.

*Impact: high. Keyboard and screen-reader users, on four of the library's most-used components.*

#### Confirmed by execution — and the mechanism was not what I predicted

`playwright/oracle-focus-restore.spec.ts` encodes the APG rule and was run against the app built from this commit (`dx run --web`, Chromium). **Four fail, and the control passes:**

| Component | Focus after Escape | |
|---|---|---|
| `DropdownMenu` | `<div role="option">` — "Edit" | ✗ |
| `Menubar` | `<div role="menuitem">` — "New" | ✗ |
| `ContextMenu` | `<body>` | ✗ |
| `Select` | trigger reported `inactive` | ✗ |
| `Dialog` (control) | its trigger | ✓ |

The control matters: same harness, same page, same keypress, and `Dialog` restores focus correctly — so the four failures are behaviour, not a broken rig.

**Correction to the static analysis above.** I predicted focus falls to `<body>` in all four cases. That holds only for `ContextMenu`. In `DropdownMenu` and `Menubar`, focus *remains on the menu item of the menu that just closed* — which is worse than `<body>`: the user's next Tab continues from a position inside dismissed content that is no longer on screen. So this is not one bug with one shared fix; at minimum the menu family needs focus moved off the item as well as returned to the trigger.

To reproduce, with the preview running on :8080:

```bash
cd playwright && npx playwright test oracle-focus-restore --project=chromium
```

### `aria-hidden` on background content — a WCAG-class defect

**Absent.** The `aria_hidden` occurrences upstream (`dialog.rs:142`, `popover.rs:292`) mark the dialog's *own content* hidden while closed — they do nothing about the page behind an open modal. The backdrop blocks pointer events but not assistive technology, so a screen-reader user can navigate straight out of an open dialog and operate background controls that a sighted user cannot reach.

`dignifiedquire/aria_hidden.rs` (91 lines) ports the `aria-hidden` npm package's `hideOthers()`: walk from the overlay to `<body>`, marking every sibling `aria-hidden="true"`, each tagged `data-dxc-aria-hidden="<overlay-id>"` so nested overlays don't unhide each other's elements.

**Adoptable with rework — and note the catch:** the fork wires it into `popover.rs:333` only. `Dialog` and `AlertDialog`, the components users actually reach for when they mean "modal", never got it. Adopting this means finishing work the fork itself left undone.

*Impact: high for screen-reader users; WCAG 1.3.1 / 4.1.2 class.*

### Body scroll lock

**Absent** — no `document.body.style.overflow` write anywhere in `primitives/src`. Open a `Dialog` or `Sheet` on a long page and the background scrolls behind it.

`dignifiedquire/scroll_lock.rs` (58 lines) is the better of the two implementations and is **adoptable as-is**: it refcounts via a `window.__dxScrollLockCount` global so nested modals work, and — unlike `sarendipitee`'s — it saves and restores the *original* `overflow` value rather than blanking it, so it won't clobber an app-set style. Its only dependency, `use_effect_cleanup`, already exists upstream at `lib.rs:136` with an identical signature, and `DialogContext` already carries the `open: Memo<bool>` and `is_modal: ReadSignal<bool>` the hook wants. The fork calls it from five components (`dialog.rs:312`, `alert_dialog.rs:204`, `popover.rs:332`, `dropdown_menu.rs:409`, `context_menu.rs:410`), one line each.

`sarendipitee`'s equivalent lives inside a 999-line `OverlayManager` and is **not portable** without that architecture. Neither implementation handles iOS momentum scroll or scrollbar-gap compensation — both explicitly disclaim it.

### Nested-modal focus scope

**Absent.** The vendored `FocusTrap` has no `pause()`/`unpause()`, so a dialog opened from a dialog leaves two traps independently intercepting Tab against their own node lists. `dignifiedquire/focus_scope.rs` (743 lines) is a full Radix `focus-scope` port with a `thread_local!` scope stack. **Adoptable with rework** — it replaces the entire `focus-trap.js` mechanism and every call site, so this is an integration, not a file copy.

*Impact: medium — real, but only in the nested case.*

### Typeahead, and RTL

**Typeahead** is *present and sophisticated* in `select/` — a Levenshtein matcher that is keyboard-layout aware, with a configurable timeout (`select/context.rs`, `select/text_search.rs`). It is **absent** from `listbox.rs`, `combobox/`, `menubar.rs`, `dropdown_menu.rs` and `context_menu.rs`. `dignifiedquire/typeahead.rs` (78 lines) is a deliberately simpler prefix matcher for menus — matching Radix's actual menu behaviour — depending only on `dioxus_sdk_time::sleep`, already an upstream dependency. **Adoptable as-is** plus per-menu wiring.

**RTL is entirely absent**: no `dir`/`is_rtl`/`use_direction` anywhere, and arrow-key handling is hardcoded LTR (`menubar.rs:287-288`, `tabs.rs:326-327`, `toolbar.rs:209-215`). `dignifiedquire/direction.rs` (83 lines) is **adoptable as-is**; the direction-aware key flip it pairs with lives in a 708-line roving-focus rewrite, but the *concept* is a small patch to existing `collection.rs` handlers — port the idea, not the file.

**Roving tabindex** was checked and is **not** a gap: all six composite widgets route through the shared `collection.rs`, which centrally guarantees a single tab stop.

---

## Priority 3 — overlay positioning

### Collision detection / auto-flip / shift

**Absent, entirely.** `ContentSide` and `ContentAlign` (`lib.rs:284-325`) are enums whose only method is `as_str()`; `PopoverContent` (`popover.rs:229-296`) stamps `data-side`/`data-align` on a div and stops there. Actual placement is static CSS:

```css
.dx-popover-content[data-side="top"] { position: absolute; bottom: 100%; left: 50%; margin-bottom: 8px; }
```

Nothing measures the viewport. Searching `primitives/` for `getBoundingClientRect|flip|collision|floating-ui|autoUpdate|middleware` returns nothing relevant. `ContextMenu` is worse: it places at raw click coordinates (`context_menu.rs:495-497`) with no clamping, so a right-click near the screen edge opens a partly off-screen menu.

Two forks solved it, differently:

- **`sarendipitee/floating.rs`** (269 lines) — one `use_position()` hook wrapping the external `floating-ui-dioxus`/`-dom`/`-utils` crates (0.6.0, from crates.io). Offset + Flip + Shift + `auto_update`, gated `#[cfg(target_family = "wasm")]` with a native fallback that reproduces today's CSS-only behaviour. Reuses upstream's own `ContentSide`/`ContentAlign` names.
- **`dignifiedquire`** — an in-repo `floating-ui/` crate (3,292 lines across 18 files) plus `popper.rs` (1,158 lines): a genuine port of `@floating-ui/core` + `/dom` with flip, shift, offset, size, arrow and hide middleware, `collision_padding` and sticky behaviour. Confirmed **separable** from that fork's Tailwind layer — `primitives/` there has zero styling coupling, and the shadcn work lives in a third crate.

**Recommendation if this is taken on: `sarendipitee`'s wrapper.** ~270 lines behind one hook beats 3,300 lines to vendor and maintain, and it keeps upstream's public enum names. The trade-offs to accept: a new third-party dependency family, wasm-only behaviour (desktop/liveview keep today's static CSS — not a regression, but not a universal fix), and `ContextMenu` still needing its own clamp since it isn't anchor-based.

*Impact: high. Anyone placing an overlay near a viewport edge — the bottom of a scrollable form, the right edge of a toolbar, any small mobile viewport.*

### Top-layer rendering

**Absent.** Every overlay is an in-flow `div`; there is no `popover=` attribute, no native `<dialog>`, no `showModal` in `primitives/`. `portal.rs` exists (72 lines) but is private (`lib.rs:40`) and used only by `toast.rs` — which is why the one-line `pub mod portal` in the fix report showed up at all. Consequence: a popover inside a scroll container or under a `transform`ed ancestor gets clipped.

`dignifiedquire/top_layer.rs` (189 lines) wraps the native primitives — `popover="auto"`/`"manual"` and `<dialog>` + `showModal()`. **Adoptable with rework**; the blockers are browser-support assumptions and the fact that it touches every overlay's render tree.

**Historical note worth knowing:** upstream *used* to render `Dialog` as a native `<dialog>` with `showModal()`. Commit `dd87bf05` (2026-03-07), "Rewrite Dialog and AlertDialog primitives to match Radix", deliberately replaced it with the `div role="dialog"` pattern. Several stale fork branches still contain `showModal()` — that is leftover pre-rewrite code, **not** fork innovation.

### CSS anchor positioning

Absent upstream **and in all 111 fork refs**. Every fork that solved positioning chose JS measurement, presumably for browser support. Nothing to adopt; no demonstrated demand.

---

## Summary

| Gap | Upstream | Best source | Portability | Impact |
|---|---|---|---|---|
| `RadioGroup` form submission | Silent failure | `dignifiedquire` or `sarendipitee` `radio_group.rs` | As-is | High |
| `Select` form submission | Silent failure | `dignifiedquire` `select.rs:158-186` | With rework | High |
| `Switch` `required` | Partial | `dignifiedquire` `switch.rs:128` | As-is (1 line) | Medium |
| Focus restore (menus, select) | Absent | `dignifiedquire` `lib.rs:236-254` | With rework | High |
| `aria-hidden` background | Absent | `dignifiedquire` `aria_hidden.rs` | With rework (needs wiring the fork lacks) | High |
| Body scroll lock | Absent | `dignifiedquire` `scroll_lock.rs` | **As-is** | High |
| Collision detection | Absent | `sarendipitee` `floating.rs` | With rework | High |
| Top-layer rendering | Absent | `dignifiedquire` `top_layer.rs` | With rework | Med-high |
| Nested-modal focus scope | Absent | `dignifiedquire` `focus_scope.rs` | With rework (replaces focus-trap) | Medium |
| Typeahead (menus) | Absent | `dignifiedquire` `typeahead.rs` | As-is | Medium |
| RTL | Absent | `dignifiedquire` `direction.rs` | As-is + concept port | Medium |
| Roving tabindex | Consistent | — | No gap | — |
| CSS anchor positioning | Absent everywhere | — | Nothing to adopt | Low |
