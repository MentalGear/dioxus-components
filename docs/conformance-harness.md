# Conformance harness — rule sources and calibration

What the a11y/behaviour harness tests against, where each rule comes from, and how the harness proves itself correct before it judges a component.

Design principle: **a failing test must be traceable to a rule someone else wrote.** If the rule's only author is us, it is an opinion, and it must be labelled as one.

---

## Three tiers, never blurred

| Tier | Source | Status | Why it binds |
|---|---|---|---|
| **1. APG** | W3C WAI-ARIA Authoring Practices | Standard | The project's **own stated contract** — `README.md` requires new primitives to adhere to it |
| **2. HTML** | WHATWG HTML Living Standard + WPT | Standard | Form submission and constraint validation are plain HTML semantics |
| **3. Radix-parity** | Radix UI behaviour | **Opinion** | Upstream never committed to it; useful where the standards are silent |

Keeping tier 3 separate is not pedantry. Upstream's README names shadcn for styling and APG for behaviour, and never mentions Radix. If Radix behaviours are filed as "conformance", an upstream maintainer can fairly reject the whole suite. Labelled as a second tier, they are a proposal.

## Calibration — the part that makes it trustworthy

Every rule runs against **two subjects**: the component under test, and a reference implementation that is known-correct. If the reference fails, the *test* is wrong, not the component.

| Tier | Reference subject | Cost |
|---|---|---|
| 1. APG | The pattern's own APG example page | Vendor the page — see below |
| 2. HTML | **The browser itself** — a native `<input>`/`<select>` in the same fixture | Free, and it cannot drift |
| 3. Radix | Radix demo, pinned | Avoid in CI |

Tier 2's reference is the strongest of the three: a native `<input type="radio" name="x">` must contribute `x=value` to a `FormData` by specification. No vendoring, no network, no third party. The current `oracle-focus-restore.spec.ts` uses a weaker, internal control — `Dialog`, chosen by us because it already worked. Upgrading tier 1 to an APG-page reference removes us from the loop entirely.

---

## Tier 1 — APG

Reachability verified (HTTP 200): `patterns/menu-button/examples/menu-button-actions/`, `patterns/combobox/examples/combobox-select-only/`, `patterns/radio/`.

Source repo: `w3c/aria-practices`. The example pages are self-contained static HTML/CSS/JS under a permissive W3C licence, so **vendor them** into `playwright/oracle/reference/` and pin the commit — CI must not depend on `w3.org` being up.

Two related suites, and what they are actually good for:

- **`w3c/aria-at`** — the most authoritative statement of what a keystroke must do, but it asserts *screen-reader speech output* and needs real AT (NVDA/JAWS/VoiceOver) driving a browser. Mine it for rules; it cannot run in Playwright.
- **`web-platform-tests/wpt`, `wai-aria/`** — tests the browser's accessibility-tree mapping, not component behaviour. Useful for assertion technique, not conformance.

## Tier 2 — HTML forms

This is where the most severe defect in `capability-gaps.md` sits: `RadioGroup` and `Select` declare `name`/`required`, document them as being for form submission, and never reference them in their render bodies.

### Normative text — all verified reachable (HTTP 200)

| Section | Covers |
|---|---|
| [`form-control-infrastructure`](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html) | form owner, the `name` attribute, **submittable elements**, the constraint validation API (`willValidate`, `checkValidity()`, `reportValidity()`, `validity`) |
| [`forms`](https://html.spec.whatwg.org/multipage/forms.html) | the form submission algorithm, **constructing the entry list** |
| [`form-elements`](https://html.spec.whatwg.org/multipage/form-elements.html) | `<select>`, `<option>`, `<textarea>` |

The decisive concept is **submittable element**. Only `button`, `input`, `select`, `textarea` (and form-associated custom elements) can contribute to the entry list. A `div role="radio"` cannot — no matter what ARIA it carries. That is why the hidden-native-input pattern (`Checkbox`'s `BubbleInput`, Radix's "BubbleSelect") is not a hack but the only conforming approach available here: Dioxus renders plain DOM, not custom elements, so `ElementInternals.setFormValue()` — the modern alternative — is not on the table.

### Executable reference — WPT

`web-platform-tests/wpt`, path `html/semantics/forms/`. Sparse-clone it rather than the whole repo (32 MB vs. gigabytes):

```bash
git clone --filter=blob:none --no-checkout --depth 1 \
  https://github.com/web-platform-tests/wpt.git
cd wpt && git sparse-checkout init --cone
git sparse-checkout set html/semantics/forms && git checkout
```

Most relevant directories, with test counts as cloned:

| Path | Tests | Use |
|---|---|---|
| `form-submission-0/` | 33 | **`constructing-form-data-set.html`** is the entry-list reference |
| `constraints/` | 44 | `form-validation-checkValidity`, `-reportValidity`, `-validity-valueMissing`, … |
| `resetting-a-form/` | 6 | form reset restores initial values |
| `the-label-element/` | 14 | label→control association and activation |
| `the-select-element/` | 327 | `<select>` semantics |
| `the-input-element/` | 177 | `<input>` semantics |

**Important limit:** WPT tests the *browser's* implementation of the spec. These files cannot be pointed at our components — they assert on native elements. Their value is (a) an authoritative rule inventory and (b) proven technique: build a form, submit it, inspect the resulting entry list. Do not plan on running WPT as our suite.

### The rule checklist this yields

Each rule runs against our component *and* a native control in the same fixture:

1. A named control contributes `name=value` to `FormData` on submit.
2. An unchecked/unselected control contributes **nothing** — not an empty string.
3. A disabled control is barred from constraint validation and excluded from the entry list.
4. `required` with no value blocks submission and sets `validity.valueMissing`.
5. `checkValidity()` / `reportValidity()` / `willValidate` behave per spec.
6. Form reset restores the initial value.
7. A `<label>` association focuses and activates the control.
8. The `form` attribute associates a control rendered outside the `<form>` element.

Rules 1–4 alone would have caught the `RadioGroup` and `Select` defects.

### WCAG, for the messaging layer

Constraint validation covers whether a form *blocks*; WCAG covers whether a person can *recover*. Relevant and reachable: **3.3.1 Error Identification** (verified 200), 3.3.3 Error Suggestion, and 4.1.2 Name, Role, Value. These matter once validation exists — an error that is only conveyed by a red border fails 1.4.1/3.3.1 regardless of whether the form submits correctly.

## Tier 3 — Radix-parity, labelled as opinion

Behaviours no standard specifies, which mature libraries nonetheless implement: body scroll lock while a modal is open, `onCloseAutoFocus` semantics (restore focus to the trigger *unless* dismissal came from outside interaction), `aria-hidden` on background content, collision-aware repositioning.

File these as `tier3/` with the Radix source named in each test. They are proposals, and should read that way in a PR.

When a Radix behaviour looks idiosyncratic, reach for **bits-ui** (or another mature headless library) as an ad-hoc tie-breaker — read, never vendor — to distinguish a Radix quirk from a headless-library consensus. A second implementation is a second opinion, not a rule source, so it can strengthen a tier-3 proposal's framing ("de-facto consensus") but never becomes a standing oracle; policy details in `playwright/oracle/tier3-radix/README.md`.

---

## axe (static rules) — orthogonal to the three tiers, like hydration parity

Source: `axe-core`'s own rule set (`playwright/axe.ts`, wired via `@axe-core/playwright`), tagged to the WCAG success criteria it implements (`wcag2a`/`wcag2aa`/`wcag21a`/`wcag21aa`) plus its curated `best-practice` rules. This is not a fourth tier of the pattern-conformance ladder above — it does not cite an APG pattern or an HTML section the way tiers 1-2 do, and it is not a Radix behaviour the way tier 3 is. It is a different *class* of check entirely, filed here for the same reason hydration parity is: it needs its own paragraph, not a slot in the tiered ladder.

**What it covers that the behaviour oracles above do not.** Tiers 1-3 test *behaviour*: does Escape move focus where APG says, does a required control block form submission, does an anchored overlay track its trigger. axe tests *static* accessibility rules on whatever DOM exists at the moment it runs: valid ARIA role/attribute combinations, accessible names, unique landmarks, label association, contrast, list/table structure, heading order. A component can pass every keyboard/behaviour oracle in this repo and still fail axe — a duplicate landmark, an invalid `aria-required` on a `button`, an icon-only button with no discernible text. Row 25's ContextMenu/Menubar accessible-name gap is exactly this class of defect (though, found by execution while building this round: axe-core carries no rule that checks a bare `role="menu"` container for an accessible name at all — the row 25 fix landed here by construction, not because a scan went red for it; the axe-detectable analogue this round *did* catch by execution is `SelectList`'s own `role="listbox"`, which axe's `aria-input-field-name` rule does check).

**The helper (`playwright/axe.ts`), `expectNoAxeViolations(page, label, opts?)`:** runs `new AxeBuilder({ page }).withTags([...])`, scoped to `opts.include` if given, and fails with a table (rule id, impact, help URL, every offending node's target + trimmed html) if anything is found. `opts.exclude` disables specific rules, but each exclusion requires a non-empty `reason` — the call throws otherwise. This is enforced in code, not just convention: an excluded rule is a real, unfixed defect going invisible to every future run unless the reason is written down and auditable in the diff. A rule may be excluded only for a documented false positive at that specific scan site, or (`CONTRAST_TRACKED_ELSEWHERE`, exported from `axe.ts`) for the one sitewide, already-tracked, out-of-round-scope exception described below — never to force green on a real, locally-discovered issue.

**The two-state convention.** Every component spec that reaches an open/expanded/selected state scans twice: once after the page loads (closed/at-rest markup), once after the spec's own existing "open the overlay" step (open markup) — reusing whatever locator/interaction the spec already has, never a new fixture. A component with no such state (a static list, an always-visible control) scans once, at load. Applied this round to 30 previously-uncovered component specs (`playwright/*.spec.ts`) plus the 3 pre-existing ones refactored onto the shared helper (`preview.spec.ts`, `drag_and_drop_list.spec.ts`, `tag_group.spec.ts`), and to 4 tier-2 oracle specs (below) — **37 of 45 specs now scan (33 top-level component specs + 4 oracle specs), 72 scan-call sites total, up from 3 specs / 4 calls.** The 2 top-level specs left out (`accordion-animation.spec.ts`, `oracle-focus-restore.spec.ts`) exercise the same component states `accordion.spec.ts`/the individual component specs already scan directly — adding axe there would rescan the identical DOM for no new coverage.

Those 4 tier-2 oracle fixtures gained scans at states no component spec reaches: `oracle/tier2-html/top-layer.spec.ts` (every one of the 8 library overlay types open on the top-layer fixture, reusing Rule 1's own open steps), `oracle/tier2-html/native-dialog.spec.ts` (outer + nested `<dialog>` both open at once), `oracle/tier2-html/form-participation.spec.ts` (the required-fields form in its invalid state after a blocked submit — WCAG 3.3.1's own state), and `oracle/tier2-html/touch-focus-zoom.spec.ts` (the dashboard email client, default and compose-open — a route no component spec visits at all).

**`CONTRAST_TRACKED_ELSEWHERE` — why `color-contrast` needed a sitewide exclusion.** Running the round's full new coverage with `color-contrast` enabled surfaced it on very nearly every scan, at every state, on every route — not a per-component defect, but the shared chrome present everywhere (the docs sidebar's nav links/section headings, the site footer) already failing the same 4.5:1 threshold on every page. That is a real, already-diagnosed, already-filed gap (`docs/backlog.md` rows 31/32 — design tokens and the styling engine, not yet landed, both explicitly named as owning contrast), not a per-scan false positive, and explicitly out of this round's own remit ("contrast failures in the theme... do NOT fix"). Left enabled, it would not surface anything new past the first finding — every one of dozens of scan sites would independently rediscover the identical footer/sidebar nodes, drowning any real, component-specific finding in repetition. `CONTRAST_TRACKED_ELSEWHERE`, exported from `axe.ts`, is passed explicitly by every scan site that would otherwise hit it (auditable per this file's own exclusion-with-reason rule) rather than 60-odd copies of the same reason string. The three pre-existing exclusions predate this rule entirely and are grandfathered by this round's own instruction to preserve their coverage unchanged.

**Findings from this round's red-first pass**, fixed by construction (small, unambiguous, `#[css_module`-free tag/attribute fixes — none needed a `cfg(target_family` axis):
- **Row 25 landed**: `ContextMenu`'s and `Menubar`'s `role="menu"` popups now carry `aria-labelledby` from their trigger's id, mirroring `DropdownMenuContent`'s existing pattern (`primitives/src/context_menu.rs`, `primitives/src/menubar.rs` — the latter needed a new `trigger_id` field threaded through `MenubarMenuContext`, since no such id existed to label from before).
- `SelectList`'s `role="listbox"` had no accessible name either (same defect class as row 25, and *this* one axe's `aria-input-field-name` rule does catch) — labelled from `SelectTrigger`'s id the same way, but only when the caller hasn't already supplied `aria-label`/`aria-labelledby` themselves: `SelectList`'s own doc example demonstrates a caller override (`SelectList { aria_label: "Select Demo", ... }`), and ARIA's own precedence rules mean an unconditional `aria-labelledby` would have silently shadowed that documented, still-supported override (`primitives/src/select/components/list.rs`).
- `Navbar`'s own component-demo page duplicated the site's top-level chrome's `aria-label="Components"` on its own `nav`, an `axe` `landmark-unique` violation isolated to that one demo page — renamed to `"Example navigation"` (`preview/src/components/navbar/variants/main/mod.rs`).
- `SheetContentClose`'s icon-only close button had no accessible name (`button-name`) — added `aria-label: "Close"`, matching how `DialogClose`/`AlertDialogClose` are already labelled per-call-site (`preview/src/components/sheet/component.rs`).
- The dashboard email client's per-message `Avatar`s and the sidebar's own user avatar rendered `role="img"` with no name (`role-img-alt`) — `ImageAvatar`'s `alt` prop was threaded only to the inner `<img>`, never to the outer `role="img"` element that actually needs it; now defaults the outer element's `aria-label` from `alt` too, caller-overridable (`preview/src/components/avatar/component.rs`).
- The dashboard email client nested a second `<main>` (`landmark-main-is-top-level`, `landmark-no-duplicate-main`, `landmark-unique`) inside `SidebarInset`'s own `<main>` — changed to a plain `div`; `SidebarInset` is already the page's semantic main region (`preview/src/dashboard/views/email_client/mod.rs`).
- Two demo pages used a heading tag (`h4`/`h3`) purely for visual weight on ephemeral or per-row content with no real place in the page's heading outline (`heading-order`) — `Tooltip`'s rich-content demo and the virtual list's 2000-row-repeated card title, both changed to styled `p`s (`preview/src/components/tooltip/variants/main/mod.rs`, `preview/src/components/virtual_list/variants/main/mod.rs`).
- `color-contrast`, sitewide (`docs/backlog.md` row 39): one CSS custom property, `--secondary-color-5` (the theme's "muted secondary text" token, used across ~28 component stylesheets plus the docs sidebar and site footer), measured 3.74:1 on white; bumped `#848484` -> `#707070` (same hue, clears 4.5:1 everywhere it's paired with in this app) fixed effectively the whole surface at once. Three smaller, same-class findings alongside it: a site-CSS accent color (`--highlight-color-tertiary`, homepage/docs eyebrows) at 3.76:1, darkened the same way; one component's own literal inline badge color at 4.16:1, darkened ~10%; and two components (`drag_and_drop_list`'s task-code demo, `Calendar`'s unavailable-day style) using `--secondary-color-6` — a token whose *light* value is meant for dark-surface use — as light-mode text at 1.54:1, corrected to `--secondary-color-5`.
- `color-contrast` on a *disabled* item, in three components (`DropdownMenuItem`, `MenubarItem`, `NavbarItem`): each renders a non-natively-disabled element (`div`/`a`, not a `<button disabled>`) with a `data-disabled` styling hook (muted color + 50% opacity) but no `aria-disabled` — so axe (and any real screen reader) read it as a perfectly normal, active, low-contrast item, not an exempt disabled one. `ContextMenuItem` already set `aria-disabled` correctly; the other three now do too (`primitives/src/dropdown_menu.rs`, `primitives/src/menubar.rs`, `primitives/src/navbar.rs`). `RadioItem`/`TabTrigger`/`ToolbarButton`'s identical-looking `data-disabled` sites were checked and are unaffected — each is a real `<button disabled>`, which already carries native disabled semantics.
- `ContextMenuContent`/`MenubarContent`/`DropdownMenuContent`'s `aria-labelledby` (this round's row 25 fix, and `DropdownMenuContent`'s pre-existing one) all moved from a bare literal attribute set alongside a raw `..attributes` spread to a proper `merge_attributes` input, built only when applicable (a shared `has_own_accessible_name` helper, promoted from `SelectList` into `lib.rs` so all four call sites use one definition) — the literal-plus-spread shape is the exact duplicate-attribute hazard `docs/conformance-harness.md` hydration-parity Rule 4 already documents, reachable here if a caller's own attribute list ever carried a same-named, empty-valued `aria-label`/`aria-labelledby` (confirmed to occur in practice: this crate's own themed `Select` wrapper threads `aria_label` through unconditionally, `Some("")`/absent or not).

**Findings recorded but not fixed** (structural, third-party-vendored, or out-of-round-scope; filed as fresh backlog rows — see `docs/backlog.md` rows 36-39):
- `SelectTrigger`'s caller-overridable `id` is never resynced into the shared context signal `SelectList`'s new `aria-labelledby` reads from, so a caller who sets their own `id` on `SelectTrigger` (as the `top_layer` oracle fixture does) gets a dangling `aria-labelledby` reference — needs the same id-sync plumbing `content_id` already has (row 36).
- The dashboard email client's message rows render `role="button"` on an element that also contains further-interactive controls (star/flag toggles) — `nested-interactive`, a real structural question about how row-click and per-row controls should coexist (row 37).
- The `/component/block/?name=X&variant=...` route family has no page-level `<h1>`/`<main>` of its own when visited directly (`page-has-heading-one`, `region`) — by design for iframe-embedded use (see `sidebar.spec.ts`'s "preview page renders block" test), but a real gap when visited directly, as `sidebar.spec.ts`'s own axe tests and the dashboard's embedded sidebar do (row 38).
- `color-contrast`, one remaining combination after the sitewide token fix above: the vendored, build-time-generated `github-light` syntax-highlighting theme's comment-token color measures 4.39:1 — a third-party theme this repo does not author or own the palette of. Scoped out with a region exclusion (`EXCLUDE_VENDORED_CODE_HIGHLIGHT`, `playwright/axe.ts`), not a rule-wide `disableRules`, so a component's own contrast defect anywhere else on the same page still fails the scan (row 39).

---

## Hydration/deployment parity — orthogonal to the other three tiers

The three tiers above all ask "does this component *behave* correctly?" against a `dx serve` dev-server client. They cannot catch a defect that only exists in the **deployed build shape** — fullstack SSG, prerendered by a host (non-wasm) server binary and then hydrated by the wasm client — because none of them ever build or serve that shape. `oracle/hydration-parity.spec.ts` closes that gap. Its rule source is the WHATWG hydration-adjacent contract (server and client must render the same tree) plus Dioxus's own hydration model, not any of the three tiers' rule sources, so it is filed at `oracle/` top level rather than under `tier1-apg/`, `tier2-html/`, or `tier3-radix/`.

**Why this exists:** the 2026-09-01 production incident (`docs/recommended-implementations.md` Caveat 1) — primitives split rendered markup on `target_family = "wasm"`, which is false on the SSG server's host binary, so the deployed site's server prerender and wasm client hydration disagreed on markup structurally, breaking events page-wide on every hard-loaded page. `scripts/check-cfg-axis.sh` (wired into `main.yml`) is the source-level guard against a recurrence of that specific cfg predicate; `oracle/hydration-parity.spec.ts` is the black-box oracle for the deployed symptom itself, and the two are complementary rather than redundant — see `docs/backlog.md` row 22 for the gap neither one closes (the SSG lane is not yet wired into CI at all).

**Rule 4 (added 2026-09-01):** the served HTML of `/` contains no start tag with a duplicated attribute name (WHATWG HTML's duplicate-attribute parse error keeps the first occurrence; the CSR/hydrated DOM path keeps the last). This is a *second*, narrower divergence class than the structural one above — same tree shape on both lanes, but a single element's attribute *value* still disagrees. The house construction rule it enforces: an explicit attribute followed by a caller-`attributes` spread on the same element must be merged (`merge_attributes`, caller-wins), never left as two sequential, same-named attributes for the two lanes' opposite tie-break to disagree over — see `docs/recommended-implementations.md` Caveat 1's 2026-09-01 addendum for the finding, and `primitives/src/toast.rs`'s `ToastRegionRendered` doc for the fix pattern. For the `style` attribute specifically on every anchored-content component (`TooltipContent`, `HoverCardContent`, `DropdownMenuContent`, `MenubarContent`, `PopoverContent`'s modal/non-modal arms, `SelectList`, `ComboboxList`), the construction is `top_layer::anchored_content_attributes` (`primitives/src/top_layer.rs`), which folds the component's own `position-anchor` binding together with a caller's `style` (shorthand or plain) into the one attribute `merge_attributes` then carries.

**Rule 4b's subject changed 2026-09-03** (`docs/backlog.md` row 43): originally the top-layer oracle fixture's `ToastProvider` region, whose caller `aria_label: "Top-layer fixture notifications"` override of `ToastRegionRendered`'s own default was the concrete regression guard for Rule 4's `merge_attributes` construction. That fixture was removed from the `/` home-page gallery grid this same round (`preview/src/main.rs`'s `ComponentGallery` now excludes `top_layer` — see "Preview composition" below and the item-2 write-up in `docs/backlog.md`), so Rule 4b's subject moved to the `avatar` component's own always-mounted gallery card: its "Error State" example renders `ImageAvatar { alt: "Invalid image", aria_label: "Error avatar", ... }`, and `ImageAvatar` (`preview/src/components/avatar/component.rs`) computes its own default accessible name from `alt` (the row-34 axe fix) merged with the caller's `attributes` — structurally the same "explicit default + caller override of the same attribute name, resolved by `merge_attributes`" shape, one layer up (a preview-level themed-wrapper default rather than a primitive's own). The rule still asserts the served, effective value is the caller's override ("Error avatar"), not the wrapper's own alt-derived default ("Invalid image").

**The SSG lane — how to build and run it locally:**

```bash
# 1. Build the fullstack SSG artifact (mirrors CI's web.yml: ssg: true,
#    features: fullstack — omit --base-path to serve at "/" locally).
cd preview
#    --force-sequential: dx otherwise races its own client and server
#    sub-builds, and the client's index.html write can clobber the
#    server's prerendered output (observed 2026-09-01).
dx build --ssg --features fullstack --platform web --force-sequential true

# 2. Snapshot, then serve the static output plainly (no dx dev server).
#    Site dir: target/dx/preview/debug/web/public (workspace-root
#    relative). Snapshot it: this is the SAME directory `dx run --web`
#    serves for the CSR lane, and the SSG build leaves prerendered route
#    pages (component/, dashboard/, docs/, demos/) in it -- a later CSR
#    client then mounts on top of that prerendered markup and every
#    chrome element appears twice (duplicate navbar/footer/toast region,
#    axe `landmark-unique`, scroll-lock and native-dialog reds). Delete
#    those route directories before going back to the CSR lane.
cp -r ../target/dx/preview/debug/web/public /tmp/ssg-site
python3 -m http.server 8090 -d /tmp/ssg-site

# 3. Run the hydration-parity oracle (and/or any other spec) against it.
cd ../playwright
npx playwright test --config=ssg.local.config.ts oracle/hydration-parity.spec.ts
```

`playwright/ssg.local.config.ts` (cloned from `baseline.local.config.ts`) has no `webServer` entry, so it never starts or waits on a dev server — start the static server yourself first. It does not change the base URL for the rest of this repo's specs, which hardcode `http://127.0.0.1:8080`: to run one of *those* against the SSG lane, also serve the same site directory on port 8080 (a second `http.server` process over the same directory is harmless). `oracle/hydration-parity.spec.ts` itself hardcodes port 8090.

A quick manual check that needs no Playwright at all — the fastest way to tell whether a build landed on the web arm or the native arm:

```bash
grep -o 'dx-toast[^>]*' target/dx/preview/debug/web/public/index.html
# web arm (correct):    ...dx-toast-container-XXXXXXXX" ... popover="manual" ...
# native arm (the bug): ...dx-toast-container-XXXXXXXX" data-node-hydration="..." (no popover)
```

---

## Layout

```
playwright/oracle/
  tier1-apg/                 rules citing an APG pattern section
  tier2-html/                rules citing an HTML spec section
  tier3-radix/                labelled opinion
  hydration-parity.spec.ts   SSG-server/wasm-client markup parity (its own axis, no tier)
  reference/                 vendored APG example pages, pinned by commit
  subjects/                  adapters: route + selectors per implementation
```

Each rule file names its source at the top, as `oracle-focus-restore.spec.ts` already does. The run matrix is *rule × subject*; the output is a conformance matrix per component, which doubles as the library's accessibility scorecard.

## Status

- **Tier 1:** `oracle-focus-restore.spec.ts` (focus restore on Escape, internal control), `focus-restore-reference.spec.ts` (same rule calibrated against the vendored menu-button/combobox APG reference pages), `keyboard-matrix.spec.ts` (per-component keyboard rows against APG prose), and `menu-roles.spec.ts` (2026-09-02) — the menu/menu-button pattern's role contract (popup `role="menu"`, items `role="menuitem"`, trigger `aria-haspopup`), calibrated against the vendored `menu-button-actions.html` reference and run against `DropdownMenu`, `ContextMenu`, and `Menubar`'s submenus; caught and fixed `DropdownMenu`'s inherited listbox/option roles (`docs/backlog.md` row 24).
- **Tier 2:** partially implemented — `top-layer.spec.ts` (now including Rule 11, 2026-09-02 — the anchored-overlay self-overlap contract added for the iOS Safari on-screen-keyboard bug: no `dx-anchor-*` overlay's content may ever cover its own anchor, checked at open, after a simulated keyboard viewport change, and under a simulated no-anchor engine including the "conforming at open, CSS Anchor Positioning support removed mid-open" shape — cited to CSSOM View's `VisualViewport` plus this repo's own anchored-overlay placement contract; see that rule's own header doc for which of the seven `use_anchor_position_fallback` consumers this sandbox's Chromium can be driven into the critical shape for, and why the rest need a real device; and now Rule 12, 2026-09-03 — the inline-axis shift contract added for a user device report (a `side="top"`/`"bottom"`, `align`-only placement's horizontal position was never checked against the viewport on either engine): every anchored overlay's content stays within the viewport horizontally under the same no-anchor-engine simulation Rule 11 uses, checked against the reported `ColorPicker` case and two `top-layer.spec.ts` fixture cases that reproduced with over 190px of overflow pre-fix; `Navbar` also joined Rule 1/5/11 this round, having never been migrated onto the top-layer engine at all — see `docs/recommended-implementations.md`'s 2026-09-03 addendum for both; and now Rule 13, 2026-09-04 — every anchored content on this file's own fixture is fit-content wide and sits near its trigger, guarding the `top_layer::anchored_content_attributes` duplicated-`style` failure mode. **Rule 13 runs on this fixture's unstyled markup only** (no `#[css_module]` stylesheet at all), so it cannot see a themed component's own CSS and is not the oracle for a themed width bug — the "Select listbox renders full viewport width" report (`docs/backlog.md` row 47) lived entirely in `preview/src/components/select/style.css`'s `min-width: 100%`, invisible to this fixture; `select.spec.ts`'s own listbox-width cases are that bug's real regression guard, on the actual themed pages; and now Rule 14, 2026-09-04 — `SelectTrigger`/`DropdownMenuTrigger`/`ContextMenuTrigger`/`MenubarTrigger`/`NavbarTrigger` must each publish whatever `id` actually renders (caller override or generated) into the shared `trigger_id` context signal its content's `aria-labelledby` reads from, guarding the id-desync defect `docs/backlog.md` rows 36/41a fixed by construction (`use_id_or`, mirroring `DialogTitle`'s pattern) — **verified by execution, Chromium only**: passed on first run, along with the previously-RED "Select listbox open" axe test in this same file), `native-dialog.spec.ts` (now including Rule 8, 2026-09-03 — the opening-gesture false positive: a modal overlay's backdrop-dismiss listener must not be attached until the frame after it opens, so the tap that opened it can never be read as a dismiss; a black-box touch-emulated regression guard plus a construction-level check, since this sandbox's Chromium cannot reproduce the underlying iOS Safari click-retargeting quirk itself — see that rule's own header doc), `touch-focus-zoom.spec.ts` (2026-09-02 — text-entry `font-size` floor on touch devices: WebKit/Apple's documented auto-zoom-below-16px platform behaviour, not a W3C rule, with WCAG 1.4.4 cited as the reason `maximum-scale`/`user-scalable=no` is not an acceptable fix and shadcn's `text-base md:text-sm` `Input` cited as the consensus construction; found and fixed the same gap in `Combobox`, `Calendar`'s month/year navigation, the navbar's language `<select>`, `top_layer`'s and `form`'s raw reference controls, and one dashboard-specific CSS override beating the themed `Textarea`'s own floor), `form-participation.spec.ts` (the last one researched-but-not-yet-fixtured per its own README).
- **Tier 3:** `tier3-radix/scroll-lock.spec.ts` implemented; otherwise not implemented.
- **Hydration/deployment parity:** implemented (`oracle/hydration-parity.spec.ts`, 2026-09-01) — 4 rules (Rule 4 added same day, the attribute-override-dedup class), all run against the local SSG lane. Not wired into CI yet (`docs/backlog.md`, "SSG lane in CI").
- **Global stylesheet applied (tier 2, `oracle/tier2-html/global-stylesheet.spec.ts`, 2026-09-02):** on every route, `main.css` must be linked, present in `document.styleSheets`, and the source of `body`'s font-family. Found by execution: `main.css` opened with an `@import` of the Geist fonts, and a stylesheet whose `@import` has not loaded is never applied — so in any environment where the font CDN is slow or unreachable (this sandbox's proxy, for one) the entire app stylesheet was inert on every route, on both lanes, for the whole life of the harness, and no spec noticed because component behaviour lives in per-component `css_module` files. The fonts now load from `<link>`s in the shared `GlobalHead` component (`preview/src/main.rs`) and `main.css` carries no `@import`.
- **Touch focus-zoom floor (tier 2, `oracle/tier2-html/touch-focus-zoom.spec.ts`, 2026-09-02):** under a coarse-pointer context (iPhone descriptor on Chromium — WebKit is not available locally), every text-entry element on every route, including overlay-gated ones, computes `font-size >= 16px` (WebKit's documented auto-zoom threshold; WCAG 1.4.4 rules out `maximum-scale`; shadcn's `text-base md:text-sm` is the consensus construction). Floors live at the component layer (copied-out components) and as an app-wide `!important` backstop in `main.css`.
- **Preview composition (source-level guard, not an oracle):** `scripts/check-preview-composition.sh` + `docs/preview-composition.md` (2026-09-01) — preview markup composes only themed wrappers from `crate::components::*`; a raw `dioxus_primitives::` component in a fixture or dashboard renders classless (the "collapsed library switch" incident). Its browser-visible half is covered by the existing form-participation oracle plus an SSR render test in `preview/src/components/form/component.rs`.
- **Accordion close-animation regression:** `playwright/accordion-animation.spec.ts` (2026-09-01) — samples the content height per frame during close and asserts it reaches ~0 before unmount with no mid-curve plateau (the padding-floor jank `accordion.spec.ts`'s smoothness check could not see); runs against the app route by default and against a standalone reproduction page with `ACCORDION_MODE=repro`. **Harness quirk, found 2026-09-04 on this sandbox's first execution:** the spec sets a 20-minute `page.goto` timeout to tolerate a cold wasm compile, but the outer 5-minute per-test timeout in `playwright.config.ts` fires first and defeats it — only bites the first test run against a cold dev server (5/5 clean once the server is warm).
- **DatePicker anchoring + smoke:** `playwright/date-picker.spec.ts` (2026-09-01) — regression test for a live-site report ("the calendar doesn't anchor to its trigger"), root-caused to `DatePickerPopover` never forwarding `is_modal` and `date_picker/style.css` missing the `@supports (anchor-name: --a)` block every other non-modal overlay carries; plus a minimal smoke suite. Partially closes `docs/backlog.md` row 20 — segment typing/arrows/backspace and the range picker are still not covered.
- **axe (static rules), 2026-09-03 (`docs/backlog.md` row 34):** implemented across 37 of 45 specs (33 top-level component specs + 4 oracle specs, 72 scan-call sites, up from 3 specs / 4 calls) — see "axe (static rules)" above for the full account, the shared `playwright/axe.ts` helper, and this round's findings.
- **Touch double-tap-zoom suppression (tier 2, `oracle/tier2-html/touch-double-tap.spec.ts`, 2026-09-03, `docs/backlog.md` row 42):** under the same coarse-pointer iPhone 13 Chromium context `touch-focus-zoom.spec.ts` calibrates, every interactive element on every route (`button`, `[role="button"]`, `a[href]`, `input`, `select`, `textarea`, `summary`, `[tabindex]:not([tabindex="-1"])`, `[role="menuitem"/"option"/"tab"/"switch"/"checkbox"/"radio"/"slider"]`) computes `touch-action: manipulation` (or the spec-equivalent `pan-x pan-y`) — the Pointer Events spec's own documented mechanism for suppressing double-tap-to-zoom without disabling one-finger panning (`touch-action: none` would additionally break scrolling and is treated as a hard failure, never an acceptable fix). Found and fixed a user-reported gap ("if the user is quickly tapping buttons, the double-tap zoom is triggered"): **56 of 57 tests red before this round** (`computed touch-action: auto` on every matched element everywhere — nothing in the app declared the property at all, so even a component page with no interactive markup of its own failed via shared chrome: the footer's links, the DEMO/CODE tabs, the per-code-block copy button), all green after. Two layers, same construction as the font-size floor: an `!important` app-wide rule in `main.css`, plus the same property on every themed component's own interactive root class (`preview/src/components/*/style.css`) so a `dx components add`-copied component still carries it. `slider`/`color_picker`'s thumbs (pointer-drag controls) were checked specially and confirmed unaffected — `manipulation` still permits the pointer drag and pan/pinch, only the double-tap gesture is suppressed; their pre-existing, out-of-scope `touch-action: none` on the *track* containers (not the thumbs) was left untouched.
- **Top-layer ink colour (tier 2, `oracle/tier2-html/top-layer-ink.spec.ts`, 2026-09-04, `docs/backlog.md` row 50):** every `[popover]`/`<dialog>` element, and every `button`/`input`/`select`/`textarea` nested inside one, must resolve a `color` that traces to the app's own ink token rather than the UA's `CanvasText` default — cited to the WHATWG HTML rendering chapter's popover/`<dialog>` UA rules plus CSS Cascading and Inheritance Level 4's cascade algorithm (a directly-cascaded UA declaration on the element beats an inherited author one, at any specificity). 15 surfaces × light/dark, asserting against the token actually read from the page rather than a hardcoded rgb literal so the rule survives a theme retune; deliberately keeps the cases where a component declares its own `color` (Tooltip; Select/Combobox's `--secondary-color-1`) as a construction proof the shared `:where()`-specificity fix does not override them. Red-first: 20 failed / 10 passed before the fix in `primitives/src/top_layer.rs`'s `ensure_top_layer_ink_styles`, 30/30 after.
- **`dx-` class migration and literal-migration tooling (`docs/backlog.md` rows 31b/32, 2026-09-04):** two mechanical-check scripts joined `check-preview-composition.sh`/`check-cfg-axis.sh` as source-level guards — `scripts/check-dx-class-prefix.sh` (every themed component's shipped `style.css` may only define `dx-<component>` or `dx-<component>-…` classes; a component still on `#[css_module]` is a warning rather than a failure, so the exemption expires automatically the moment that component's hashing is dropped) and `scripts/check-css-literals.sh` (a themed stylesheet may not hard-code a value that exactly matches a row-31a design token, in the token's own unit; near-unit matches like an equivalent `px` value are reported as informational notes, never failures, since normalising them is a rendering decision, not a mechanical one). Two more files are TOOLS, not oracles — they carry no rule citation and are not part of the default `npx playwright test` run: `playwright/dx-class-migration.spec.ts` (two generic per-component assertions — some stylesheet reachable from the page defines `.dx-<component>`, and no element anywhere carries the `dx-…-<8 hex>` hashed shape `#[css_module]` used to append; caught `sidebar`'s stylesheet failing to reach its iframe-embedded demo on first run, a silent-delivery-failure shape no role/name-selecting spec elsewhere in the suite could see) and `playwright/computed-style-snapshot.spec.ts` (skipped unless `DX_SNAPSHOT` is set; captures 24 computed properties across every `dx-` element on all 46 component pages, so row 31b's literal→token migration could be proven value-preserving by diffing a before/after snapshot rather than by eye — the one real diff found, `dx-progress-indicator`'s animated width, was confirmed to vary just as much on a single unchanged build).

---

## What the existing suites already do — and where they stop

Measured, not assumed. This decides what the harness must add versus duplicate.

### This project

| Layer | Extent |
|---|---|
| Playwright e2e | 32 spec files, **123 tests** — the primary safety net |
| Cargo unit tests | `#[cfg(test)]` in **10 of 63** `.rs` files under `primitives/src` (8 of the 40 top-level ones) — and they are the algorithmic ones (`slider`, `calendar`, `virtualizer`, `select/text_search`, `pointer`, `date_picker`, `selection`, `color_picker`, `move_interaction`, `lib`) |
| CI (`main.yml`) | `cargo check --workspace --all-features`, `cargo test --workspace`, `cargo fmt --check`, docs with `-Dwarnings`, and `clippy … -D warnings` |
| Other workflows | `playwright.yml`, `stylelint.yml`, `web.yml`, `pages.yml`, `all_components.yml` |
| `test-harness/` | A separate Dioxus app for manual exercising — not an automated suite |

The split is sensible: unit-test the algorithms in Rust, interaction-test the components in a real browser. There is no component-level unit test of DOM/ARIA output, because Rust has no jsdom equivalent — which is what `hovinen`'s `dioxus-test` branch was exploring.

Where the e2e suite stops is precise and revealing:

| Assertion | Count |
|---|---|
| `keyboard.press` | 167 |
| `toBeFocused` | 84 |
| `aria-selected` | 13 |
| `role=` | 11 |
| axe scans | **3 specs of 32** (superseded 2026-09-03: 37 of 45, 72 scan-call sites — see "axe (static rules)" above) |
| `aria-hidden` | **0** |
| `overflow` / `scrollY` | **0** |

Keyboard interaction is genuinely well covered. The suite tests what a component *does*, and never what it must *prevent* — no scroll containment, no background inertness, and no assertion of where focus *lands* after a close, only that the thing closed. That is the exact shape of the gaps in `capability-gaps.md`, and it is why they survived 123 tests.

### Radix

`vitest` + `@testing-library/react` + **`vitest-axe`** — axe assertions inside unit tests rather than e2e. Substantial files: `select.test.tsx` 1,610 lines, `radio-group.test.tsx` 983, `checkbox.test.tsx` 829. Axe is called 7× in checkbox and 3× in radio-group, but **0× in select**.

The notable part, and it is directly relevant to tier 2: **`FormData` appears zero times across Radix's checkbox, radio-group and select tests.** Their `RadioGroup.ItemBubbleInput` block asserts that the element renders, that it is an `INPUT`, that it carries `type="radio"` and `aria-hidden`, and that props are forwarded — but never that submitting a form actually produces the entry.

So Radix verifies *the mechanism exists*, not *the outcome the mechanism is for*. The entry-list rules in this document therefore go **beyond** what the reference implementation tests. That is worth knowing in both directions: the proposal is not redundant, and it is also not something we can point at Radix to justify — it has to stand on the HTML spec, which it does.
