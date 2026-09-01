# Preview composition rule

`preview/` (the docs/demo site) has one architectural rule that
`scripts/check-preview-composition.sh` enforces automatically:

> Preview markup composes ONLY themed wrappers from `crate::components::*`.
> It never reaches for a raw `dioxus_primitives::` component directly.

## Why

Every component this library ships has two layers:

- `primitives/src/<name>.rs` (or `primitives/src/<name>/`) -- the unstyled,
  accessible primitive. Its elements carry no fixed size or visual by
  default; a consumer's own CSS is expected to give it one.
- `preview/src/components/<name>/component.rs` -- the *themed wrapper*. It
  imports the raw primitive, attaches the theme's `css_module` class (e.g.
  `.dx-switch` in `preview/src/components/switch/style.css`), and is what
  every `dx components add <name>` install actually ships.

Composing the raw primitive anywhere else in the preview app skips that
theme class. For most primitives that "just" produces unstyled markup, but
for a few it produces something that looks broken rather than merely
plain, because the primitive relies on the theme's CSS for basic layout,
not just color/spacing:

- `Switch`'s `<button>` gets no `width`/`height` at all without
  `.dx-switch { all: unset; width: 2rem; height: 1.15rem; ... }` -- it
  collapses to an empty button's intrinsic (near-zero) size.
- `Checkbox`'s indicator only renders its children while checked; without
  `.dx-checkbox`/`.dx-checkbox-indicator` giving the button a fixed box, it
  visibly grows for the checkmark glyph when checked and collapses back to
  nothing when unchecked.
- `RadioItem`'s entire visual is a `.dx-radio-item::before` pseudo-element;
  without that class the button renders no visual at all, in any state.

Both incidents below were confirmed by construction (execution-verified,
not just plausible) before being fixed:

- **Checkbox/RadioGroup** (see the header comment in
  `preview/src/components/form/component.rs`): the form fixture imported
  `dioxus_primitives::checkbox::{Checkbox, CheckboxIndicator}` and
  `dioxus_primitives::radio_group::{RadioGroup, RadioItem}` directly instead
  of `crate::components::{checkbox, radio_group}`.
- **Switch/Select** (2026-09-01, live-site report: "Notifications (library
  switch)" and "Opt-in, required (library switch)" render visually
  collapsed on the form fixture): the same fixture also imported
  `dioxus_primitives::switch::{Switch, SwitchThumb}` and
  `dioxus_primitives::select::{Select, SelectList, SelectOption,
  SelectTrigger, SelectValue}` directly. The dashboard email client
  (`preview/src/dashboard/views/email_client/{list_pane,read_pane}.rs`) had
  the same defect, importing the whole `dioxus_primitives::select` module
  under an alias (`primitive_select`) instead of
  `crate::components::select::*`.

## The rule, precisely

- A raw `dioxus_primitives::` import is legitimate in exactly two places:
  1. Inside the themed wrapper itself
     (`preview/src/components/<name>/component.rs`) -- that file *is* the
     layer responsible for pulling in the raw primitive and attaching the
     theme class. This applies to the wrapper's own matching primitive; two
     named exceptions (`form` and `top_layer`) are not themed wrappers at
     all and are called out explicitly in the script instead of relying on
     this blanket exemption -- see the script's comments for why each is
     handled the way it is.
  2. Anywhere else in `preview/src`, for a small, explicit allowlist of
     *non-markup* items -- hooks (`use_toast`) and plain value types/enums
     (`CheckboxState`, `ToastOptions`, `Color`, `DateRange`, `ContentSide`,
     `ScrollDirection`) that render nothing themselves, so there is no
     theme for a wrapper to attach in the first place.
- Everything else -- a component/markup type, or a module alias that pulls
  in a whole family of such components (`dioxus_primitives::select as
  primitive_select`) -- must come from `crate::components::*` instead.

See `scripts/check-preview-composition.sh` for the enforced, up-to-date
version of the allowlist and the exact file-exemption logic (including the
reasoning for the `form`/`top_layer` special cases), and its own header
comment for the full rationale.

## Running the check

```sh
scripts/check-preview-composition.sh
```

Exits non-zero and prints every offending line if the rule is violated.
It has no external dependencies beyond `bash`/`grep`, so it's cheap enough
to run on every change to `preview/src`.
