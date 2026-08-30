The Form fixture assembles a real `<form>` around this library's form
controls -- `Checkbox`, `Switch`, `RadioGroup`, and `Select` -- each set up
with its documented `name` (and `required`, where it exists) exactly as an
app would use it. Every row places the library control beside a native
reference control sharing a parallel `name`, per the tier 2 (HTML)
calibration rule in `docs/conformance-harness.md`: native controls are the
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
submittable element for it -- see `docs/plan.md` Phase 1. Their rows
contribute nothing to `FormData` today and never block submission. `Select`
additionally has no `required` prop yet, so its row in the required-blocking
form only has a native reference to test against. This fixture sets those
props anyway, per each component's documented API, so the same fixture goes
red today and green once Phase 1 lands.
