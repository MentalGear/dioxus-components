# Draft issue: proposal to upstream select work back to `DioxusLabs/dioxus-components`

**Status:** drafted 2026-09-03, not filed — user decision (`docs/backlog.md` row 6).

---

**Target repo:** `DioxusLabs/dioxus-components`.

## Title

Proposal: upstream a slice of work from a fork (form participation, native `<dialog>`/top-layer overlays, an SSG cfg-axis fix, and a conformance harness)

## Body

### Context

This is a proposal from `MentalGear/dioxus-components`, a fork that has been actively developed since upstream `main`'s last commit (idle since 2026-06-29, per our own tracking — please correct us if that's stale by the time this is read). Independently of us, at least two other forks in the network (`dignifiedquire/dx-components`, `sarendipitee`) have also built substantial overlapping work in the same window — scroll lock, focus coordination, and collision detection all exist in more than one fork with no cross-pollination. We'd rather send this upstream in reviewable pieces than let it keep diverging, and we're also raising this so the network's overlapping work becomes visible to whoever else is watching this repo.

We're not proposing to dump a branch. Below is what we have, broken into independently reviewable slices, in the order we'd suggest taking them.

### What this fork has that upstream doesn't (as far as we can tell from the public `main` branch)

1. **Native form participation** — `RadioGroup`, `Switch`, `Select`, and `Checkbox` each render a hidden native input (`<input type="radio">`, a native `<select>` mirror, etc.) so their value participates in a surrounding `<form>`'s submission, `FormData`, and native validation (`required`, `checkValidity()`) without any JS-side form-state library. Verified with a form fixture built specifically to exercise this (none existed before).
2. **Native `<dialog>` + `showModal()` for modal `Dialog`/`AlertDialog`**, and **`popover`/`popover="auto"`/`popover="manual"` top-layer promotion** for non-modal overlays (`Popover`, `Tooltip`, `HoverCard`, `DropdownMenu`, `Menubar`, `Select`, `Combobox`) plus **CSS Anchor Positioning** for trigger-relative placement, with a JS-measured fallback (`use_anchor_position_fallback`) for engines without native anchor-positioning support that re-measures for the overlay's entire open lifetime (not just at open) and reads `window.visualViewport` rather than `window.inner*` so it survives an on-screen keyboard appearing after open. Together these fix real defects the plain-`div`-overlay approach has: clipping inside `overflow: hidden`/transformed ancestors, incorrect stacking against sibling overlays, and (for modals) needing a hand-rolled focus trap instead of the platform's own.
3. **A cfg-axis fix for fullstack SSG builds**: components that render different markup for "web" vs "native (Blitz)" targets were splitting on `#[cfg(target_family = "wasm")]`, which is **false** on the fullstack SSG prerender's host (non-wasm) server binary — so a deployed SSG site's server-rendered markup and its wasm-client hydration disagreed structurally, breaking event listeners page-wide after a hard page load. The fix: split on a renderer feature flag (this fork's `web` Cargo feature) instead of the target triple, since a webview-backed desktop build and a fullstack SSG server binary both need the "web markup" arm despite neither being `wasm32`. Root-caused and reproduced as a real deployed-site incident; happy to share the full writeup.
4. **A conformance harness** calibrated against WAI-ARIA APG patterns, tiered by rule source (APG pattern prose, HTML/WHATWG spec sections, and labelled-opinion "tier 3" rules), each rule citing exactly what it enforces and run against both this fork's components and vendored APG reference pages as a calibration control. Includes a dedicated hydration/deployment-parity lane that builds the actual fullstack SSG artifact and hydrates it in a real browser — the class of defect in point 3 above is specifically the kind nothing else in a normal dev-server-only test suite would ever catch.
5. **A keyboard-open contract and a menu role contract**, applied uniformly: every menu-family trigger's open keys (Enter, Space, ArrowDown, ArrowUp) now consistently request initial focus on the first item (found and fixed as a real bug: some open keys moved focus, others left it stranded on the trigger, inconsistently per component), and `DropdownMenu`/`ContextMenu`/`Menubar` now all read their `aria-haspopup`/`role` literals from one shared module (`menu_semantics`) instead of three independent hand-written copies that had already drifted (found live: `DropdownMenu` was rendering the APG **listbox** pattern's roles instead of the **menu-button** pattern's, despite having no selection semantics at all).
6. **Scroll lock, generation 4**: pure pointer/wheel/keyboard event interception (no `overflow: hidden` toggling at all, after three earlier generations were execution-falsified against real scrollbar-gutter defeats on at least one shipping browser engine — happy to share the falsification writeups, they're the kind of finding worth having on record even for an approach not taken), refcounted for nested modals, plus a permanent `scrollbar-gutter: stable` baseline that fixes a ~15px layout shift on classic (non-overlay) scrollbars.

### Proposed slice order

We'd suggest taking these roughly in the order above — form participation and the cfg-axis fix are both self-contained bug fixes with no architectural dependency on anything else here; the native-dialog/top-layer work is the largest single slice and probably wants its own review pass; the conformance harness could either come with the components it was built to check, or separately as a standalone contribution reviewers can point at any component; the keyboard/role contract and scroll lock are both small, mechanical, low-risk once the above land.

### Ask

How would you like to receive this? Options we can work with:

- **PRs, one slice at a time**, in the order above or whatever order you'd prefer — we can open the first one immediately if that's the preference.
- **A discussion thread first**, if you'd rather scope what you actually want before any code moves, especially given at least two other forks have independently built overlapping pieces of this (scroll lock and focus coordination in particular) — might be worth comparing notes across all three before committing to one shape.

Happy to answer any question about a specific mechanism above in more detail, or point at the exact commit/file for anything that would help review.

---

*Internal note (remove before filing): per-item detail and doc references live in `docs/plan.md`, `docs/backlog.md`, `docs/recommended-implementations.md`, and `docs/conformance-harness.md` in this fork — link specific sections/permalinks once a filing commit is chosen.*

## Before filing

- [ ] Re-verify upstream `main`'s last-commit date is still 2026-06-29 (or update the "idle since" claim) — this proposal's framing depends on it.
- [ ] Link specific permalinks for each of the six points above (`docs/plan.md` phase entries, `docs/recommended-implementations.md` Caveat 1, `docs/conformance-harness.md`) at the commit this issue is filed from.
- [ ] Check the upstream PR queue again for anything that has landed or changed since 2026-08-30 (this fork's last full scan) before claiming any of the six points as still-missing upstream.
- [ ] Confirm no naming/architecture collision with anything already merged upstream since this draft was written.
