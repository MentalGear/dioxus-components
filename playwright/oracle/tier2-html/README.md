# Tier 2 — HTML forms

Source: [`../../../docs/conformance-harness.md`](../../../docs/conformance-harness.md), "Tier 2 — HTML forms".

## Rule-source policy

A rule belongs here only if it cites a section of the **WHATWG HTML Living Standard** (or, for the messaging layer, a numbered WCAG success criterion). The normative sections in play:

| Section | Covers |
|---|---|
| [`form-control-infrastructure`](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html) | form owner, the `name` attribute, submittable elements, the constraint validation API |
| [`forms`](https://html.spec.whatwg.org/multipage/forms.html) | the form submission algorithm, constructing the entry list |
| [`form-elements`](https://html.spec.whatwg.org/multipage/form-elements.html) | `<select>`, `<option>`, `<textarea>` |

Every rule file must name the specific section (and, where applicable, algorithm step or WCAG success criterion) it is drawn from at the top of the file.

## Calibration

Tier 2's reference is the strongest of the three tiers: each rule runs against our component **and** a native control (`<input>`, `<select>`, …) in the same fixture. A native `<input type="radio" name="x">` must contribute `x=value` to a `FormData` by specification — no vendoring, no network, no third party, and it cannot drift.

## The rule checklist (from conformance-harness.md)

1. A named control contributes `name=value` to `FormData` on submit.
2. An unchecked/unselected control contributes nothing — not an empty string.
3. A disabled control is barred from constraint validation and excluded from the entry list.
4. `required` with no value blocks submission and sets `validity.valueMissing`.
5. `checkValidity()` / `reportValidity()` / `willValidate` behave per spec.
6. Form reset restores the initial value.
7. A `<label>` association focuses and activates the control.
8. The `form` attribute associates a control rendered outside the `<form>` element.

Rules 1–4 alone would have caught the `RadioGroup` and `Select` defects described in `capability-gaps.md`.

## What is mined but does not run here

`web-platform-tests/wpt`, path `html/semantics/forms/`, is an authoritative rule inventory and a proven assertion technique (build a form, submit it, inspect the resulting entry list) — but it tests the *browser's* implementation, not ours. Do not point WPT files at our components; do not plan on running WPT as this suite.

## Status

Per conformance-harness.md: rules 1–4 and 6 implemented in `form-participation.spec.ts` (2026-08-29), calibrated against the form fixture in `preview/src/components/form/component.rs` (Phase 0 item 0.4, landed). Rules 5, 7, and 8 are deliberately not implemented yet — see that spec file's own header for why each is deferred rather than missing by oversight. `top-layer.spec.ts`, `native-dialog.spec.ts`, `touch-focus-zoom.spec.ts`, and `global-stylesheet.spec.ts` also live in this tier; see `conformance-harness.md`'s "Status" section for the full current inventory.
