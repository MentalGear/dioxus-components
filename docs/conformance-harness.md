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

---

## Layout

```
playwright/oracle/
  tier1-apg/           rules citing an APG pattern section
  tier2-html/          rules citing an HTML spec section
  tier3-radix/         labelled opinion
  reference/           vendored APG example pages, pinned by commit
  subjects/            adapters: route + selectors per implementation
```

Each rule file names its source at the top, as `oracle-focus-restore.spec.ts` already does. The run matrix is *rule × subject*; the output is a conformance matrix per component, which doubles as the library's accessibility scorecard.

## Status

- **Tier 1:** one rule implemented (focus restore on Escape), executed, 4 fail / 1 control passes. Not yet calibrated against an APG page — the control is internal.
- **Tier 2:** researched, not implemented. Needs a form fixture; none exists in `preview/`.
- **Tier 3:** not implemented.

---

## What the existing suites already do — and where they stop

Measured, not assumed. This decides what the harness must add versus duplicate.

### This project

| Layer | Extent |
|---|---|
| Playwright e2e | 32 spec files, **122 tests** — the primary safety net |
| Cargo unit tests | `#[cfg(test)]` in **10 of 40** files in `primitives/src` — and they are the algorithmic ones (`slider`, `calendar`, `virtualizer`, `select/text_search`, `pointer`, `date_picker`, `selection`, `color_picker`, `move_interaction`, `lib`) |
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

Keyboard interaction is genuinely well covered. The suite tests what a component *does*, and never what it must *prevent* — no scroll containment, no background inertness, and no assertion of where focus *lands* after a close, only that the thing closed. That is the exact shape of the gaps in `capability-gaps.md`, and it is why they survived 122 tests.

### Radix

`vitest` + `@testing-library/react` + **`vitest-axe`** — axe assertions inside unit tests rather than e2e. Substantial files: `select.test.tsx` 1,610 lines, `radio-group.test.tsx` 983, `checkbox.test.tsx` 829. Axe is called 7× in checkbox and 3× in radio-group, but **0× in select**.

The notable part, and it is directly relevant to tier 2: **`FormData` appears zero times across Radix's checkbox, radio-group and select tests.** Their `RadioGroup.ItemBubbleInput` block asserts that the element renders, that it is an `INPUT`, that it carries `type="radio"` and `aria-hidden`, and that props are forwarded — but never that submitting a form actually produces the entry.

So Radix verifies *the mechanism exists*, not *the outcome the mechanism is for*. The entry-list rules in this document therefore go **beyond** what the reference implementation tests. That is worth knowing in both directions: the proposal is not redundant, and it is also not something we can point at Radix to justify — it has to stand on the HTML spec, which it does.
