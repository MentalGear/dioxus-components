The Form fixture assembles a real `<form>` around this library's form
controls -- `Checkbox`, `Switch`, `RadioGroup`, and `Select` -- each set up
with its documented `name` (and `required`, where it exists) exactly as an
app would use it. Every row places the library control beside a native
reference control sharing a parallel `name`, per the tier 2 (HTML)
calibration rule: native controls are the
ground truth for `FormData` and constraint validation, so if a rule fails on
them, the test is wrong, not the component.

## Two forms

- **Entry list** (`#entries-form`) -- submitting prevents navigation, builds
  `new FormData(form)`, and renders one `name=value` line per entry, in
  insertion order, into `#form-result`. A `data-submit-count` attribute on
  that element increments on every submit, so a test can await the update
  deterministically instead of racing the DOM write.
- **Required blocking** (`#form-required`) -- every library control that has
  a documented `required` prop sets it. Because the `invalid` event does not
  bubble, a single capturing listener on the form records which controls the
  browser's constraint validation blocked submission on into
  `#invalid-report`, with a matching `data-invalid-count` attribute. A
  submit that clears every required control renders its own entry list into
  `#required-result`.

## `DemoForm`: every preview form, and why

Every `<form>` in the preview app -- this fixture's two, the card's login form, the dashboard's compose
modal -- is a `DemoForm` (`crate::components::form::DemoForm`), never a bare `form { .. }`. A bare form with
no `onsubmit` is a real HTML form: clicking its submit button, or pressing Enter in one of its fields,
navigates to the form's action URL with the entries as a query string, which reloads the page
(`/component/card/` becomes `/component/card/?`). Dioxus does not prevent that on its own, and on the
prerendered pages a submit before the wasm bundle has hydrated cannot be stopped by any Rust handler.

`DemoForm` renders `method="dialog"` (a dialog-method form outside a `<dialog>` never navigates, per the
HTML Standard's form submission algorithm, so it holds before hydration, with scripting off and under any
CSP) and an `onsubmit` that calls `prevent_default()`. Its props extend the global attributes only, so a
caller cannot pass `action` or `method` and undo it. Constraint validation and the `submit` event behave as
for any form. By default it shows a polite "Submitted. This is a demo, so nothing was sent." line under the
fields after a submit, so the demo still visibly submits; pass `show_status: false` when the page reports the
result itself, and `onsubmit:` to react to the submit.

`scripts/check-demo-forms.sh` fails the gate run on any other `form {` in `preview/src`, and
`playwright/oracle/tier2-html/demo-forms-no-navigation.spec.ts` clicks every submit control and presses Enter
in every form on every route, hydrated and before hydration, asserting nothing navigates. The one exemption is
the native `<dialog>` reference in the top-layer fixture, where submitting is meant to close the dialog.
The library has no form primitive; if one is added, native submission or a server function should be an opt-in
prop there, never the preview's default.

## Component Structure

```rust
Checkbox { name: "terms-lib", value: "accepted", required: true,
    CheckboxIndicator { "✓" }
}
Switch { name: "opt-in-lib", value: "subscribed", required: true,
    SwitchThumb {}
}
RadioGroup { name: "tier-lib", required: true,
    RadioItem { value: "small".to_string(), index: 0usize, "Small" }
    RadioItem { value: "medium".to_string(), index: 1usize, "Medium" }
    RadioItem { value: "large".to_string(), index: 2usize, "Large" }
}
Select::<String> { name: "fruit-lib",
    SelectTrigger { SelectValue {} }
    SelectList {
        SelectOption::<String> { index: 0usize, value: "apple", "Apple" }
    }
}
```

## Known gap this fixture is built to expose

`RadioGroup` and `Select` document `name` (and `RadioGroup` also documents
`required`) as being for form submission, but neither currently renders a
submittable element for it yet. Their rows
contribute nothing to `FormData` today and never block submission. `Select`
additionally has no `required` prop yet, so its row in the required-blocking
form only has a native reference to test against. This fixture sets those
props anyway, per each component's documented API, so the same fixture goes
red today and green once that lands.
