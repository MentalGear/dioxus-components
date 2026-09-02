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

## Hydration/deployment parity — orthogonal to the other three tiers

The three tiers above all ask "does this component *behave* correctly?" against a `dx serve` dev-server client. They cannot catch a defect that only exists in the **deployed build shape** — fullstack SSG, prerendered by a host (non-wasm) server binary and then hydrated by the wasm client — because none of them ever build or serve that shape. `oracle/hydration-parity.spec.ts` closes that gap. Its rule source is the WHATWG hydration-adjacent contract (server and client must render the same tree) plus Dioxus's own hydration model, not any of the three tiers' rule sources, so it is filed at `oracle/` top level rather than under `tier1-apg/`, `tier2-html/`, or `tier3-radix/`.

**Why this exists:** the 2026-09-01 production incident (`docs/recommended-implementations.md` Caveat 1) — primitives split rendered markup on `target_family = "wasm"`, which is false on the SSG server's host binary, so the deployed site's server prerender and wasm client hydration disagreed on markup structurally, breaking events page-wide on every hard-loaded page.

**Rule 4 (added 2026-09-01):** the served HTML of `/` contains no start tag with a duplicated attribute name (WHATWG HTML's duplicate-attribute parse error keeps the first occurrence; the CSR/hydrated DOM path keeps the last). This is a *second*, narrower divergence class than the structural one above — same tree shape on both lanes, but a single element's attribute *value* still disagrees. The house construction rule it enforces: an explicit attribute followed by a caller-`attributes` spread on the same element must be merged (`merge_attributes`, caller-wins), never left as two sequential, same-named attributes for the two lanes' opposite tie-break to disagree over — see `docs/recommended-implementations.md` Caveat 1's 2026-09-01 addendum for the finding, and `primitives/src/toast.rs`'s `ToastRegionRendered` doc for the fix pattern.

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
- **Tier 2:** partially implemented — `top-layer.spec.ts` (now including Rule 11, 2026-09-02 — the anchored-overlay self-overlap contract added for the iOS Safari on-screen-keyboard bug: no `dx-anchor-*` overlay's content may ever cover its own anchor, checked at open, after a simulated keyboard viewport change, and under a simulated no-anchor engine including the "conforming at open, CSS Anchor Positioning support removed mid-open" shape — cited to CSSOM View's `VisualViewport` plus this repo's own anchored-overlay placement contract; see that rule's own header doc for which of the seven `use_anchor_position_fallback` consumers this sandbox's Chromium can be driven into the critical shape for, and why the rest need a real device), `native-dialog.spec.ts`, `touch-focus-zoom.spec.ts` (2026-09-02 — text-entry `font-size` floor on touch devices: WebKit/Apple's documented auto-zoom-below-16px platform behaviour, not a W3C rule, with WCAG 1.4.4 cited as the reason `maximum-scale`/`user-scalable=no` is not an acceptable fix and shadcn's `text-base md:text-sm` `Input` cited as the consensus construction; found and fixed the same gap in `Combobox`, `Calendar`'s month/year navigation, the navbar's language `<select>`, `top_layer`'s and `form`'s raw reference controls, and one dashboard-specific CSS override beating the themed `Textarea`'s own floor), `form-participation.spec.ts` (the last one researched-but-not-yet-fixtured per its own README).
- **Tier 3:** `tier3-radix/scroll-lock.spec.ts` implemented; otherwise not implemented.
- **Hydration/deployment parity:** implemented (`oracle/hydration-parity.spec.ts`, 2026-09-01) — 4 rules (Rule 4 added same day, the attribute-override-dedup class), all run against the local SSG lane. Not wired into CI yet (`docs/backlog.md`, "SSG lane in CI").
- **Preview composition (source-level guard, not an oracle):** `scripts/check-preview-composition.sh` + `docs/preview-composition.md` (2026-09-01) — preview markup composes only themed wrappers from `crate::components::*`; a raw `dioxus_primitives::` component in a fixture or dashboard renders classless (the "collapsed library switch" incident). Its browser-visible half is covered by the existing form-participation oracle plus an SSR render test in `preview/src/components/form/component.rs`.
- **Accordion close-animation regression:** `playwright/accordion-animation.spec.ts` (2026-09-01) — samples the content height per frame during close and asserts it reaches ~0 before unmount with no mid-curve plateau (the padding-floor jank `accordion.spec.ts`'s smoothness check could not see); runs against the app route by default and against a standalone reproduction page with `ACCORDION_MODE=repro`.

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
| axe scans | **3 specs of 32** |
| `aria-hidden` | **0** |
| `overflow` / `scrollY` | **0** |

Keyboard interaction is genuinely well covered. The suite tests what a component *does*, and never what it must *prevent* — no scroll containment, no background inertness, and no assertion of where focus *lands* after a close, only that the thing closed. That is the exact shape of the gaps in `capability-gaps.md`, and it is why they survived 123 tests.

### Radix

`vitest` + `@testing-library/react` + **`vitest-axe`** — axe assertions inside unit tests rather than e2e. Substantial files: `select.test.tsx` 1,610 lines, `radio-group.test.tsx` 983, `checkbox.test.tsx` 829. Axe is called 7× in checkbox and 3× in radio-group, but **0× in select**.

The notable part, and it is directly relevant to tier 2: **`FormData` appears zero times across Radix's checkbox, radio-group and select tests.** Their `RadioGroup.ItemBubbleInput` block asserts that the element renders, that it is an `INPUT`, that it carries `type="radio"` and `aria-hidden`, and that props are forwarded — but never that submitting a form actually produces the entry.

So Radix verifies *the mechanism exists*, not *the outcome the mechanism is for*. The entry-list rules in this document therefore go **beyond** what the reference implementation tests. That is worth knowing in both directions: the proposal is not redundant, and it is also not something we can point at Radix to justify — it has to stand on the HTML spec, which it does.
