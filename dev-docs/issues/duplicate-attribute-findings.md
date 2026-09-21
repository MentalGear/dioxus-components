<!-- Produced by lane `attr-guard` (dev-docs/backlog.md row 93) alongside
     scripts/check-attr-spread-collision.sh, its baseline files, and the syn/dioxus-rsx
     analyzer at scripts/attr-spread-collision-checker/. This is the "summary the owner
     can act on" that lane's brief asked for -- an action-oriented breakdown of the
     ~800-entry debt register the gate ships with, not a repeat of the mechanism
     investigation (that's dev-docs/issues/duplicate-attribute-root-cause.md) or the
     ratchet's own mechanics (that's the .sh script's header comment). -->

# Duplicate-attribute guard: findings

`scripts/check-attr-spread-collision.sh` now runs as a pre-commit gate (joining the
seven guards `CLAUDE.md` lists). It does not fix anything -- this lane's brief was
detection only -- so everything below is debt the gate now prevents from *growing*, not
debt it has paid down. The two baseline files it ratchets against are the actual list:

- `scripts/check-attr-spread-collision.baseline.tsv` -- 687 sites (the primary defect).
- `scripts/check-attr-spread-collision.component-forward.baseline.tsv` -- 105 sites (a
  second, related defect this lane found while building the first check; see below).

## The primary defect, in one line

A component renders a literal attribute beside a raw `..spread`, and its props struct
has no typed field claiming that attribute's name. SSR emits both; the browser's HTML
parser keeps the component's own literal (first-wins) and hydration never corrects it,
so a caller's override silently does nothing in production while appearing to work in a
plain `dx serve --web` dev session. Full mechanism trace:
`dev-docs/issues/duplicate-attribute-root-cause.md`. Discriminator: a typed field (e.g.
`PopoverRootProps::id`, reconciled via `use_id_or`) resolves the value to one literal
before it ever reaches the catch-all `attributes: Vec<Attribute>`, so that shape is
correctly not reported.

## Headline numbers vs. the prototype

| | prototype (`duplicate-attribute-guard-prototype.py`) | this gate's analyzer (`syn` + `dioxus-rsx`) |
|---|---|---|
| Findings | 623 (as documented); **624** reproduced against this lane's own base commit `c133aa7` -- six carousel-feature commits landed between the investigation's base (`6b4bd27`) and this lane's, one of which added exactly one new site | **687** |
| Files | 66 | 70 |
| Method | indentation/line heuristic | real parse: `syn::parse_file` + `dioxus_rsx::CallBody` (the actual grammar `rsx!` invocations compile against) |

687 > 624, as expected: a real parse sees element bodies a line-oriented heuristic
cannot. It is not simply "624 plus the new stuff" -- building this analyzer surfaced and
fixed two real resolution bugs (one shared with the prototype, one new to this port) that
also *removed* some of the prototype's own findings as false positives. Net effect across
both directions:

- **+83 gained**, from three real blind spots the prototype's own header already
  disclosed or that this port newly found:
  - Single-line element bodies, the prototype's own disclosed, named gap. Confirmed
    concretely: `primitives/src/alert_dialog.rs:353` and `primitives/src/toast.rs`'s
    `ToastTitle`/`ToastDescription` -- exactly the five `id` instances
    `dev-docs/backlog.md` row 93 named as real but automation-invisible -- are now
    caught.
  - Shorthand attributes (`id,` instead of `id: id,`) have no colon, so the prototype's
    colon-requiring regex never matched them at all. `dioxus_rsx::Attribute::parse`
    handles both forms uniformly, so this port does too (this is *how*
    `toast.rs`'s `id,` above is actually caught).
  - 4 files the prototype listed zero findings for at all
    (`data_table`, `drawer`, `kbd`, `sheet` under `preview/src/components/`) turned out
    to have only single-line-shaped sites.
- **-20 lost (correctly)**, real false positives in the prototype's own final,
  "0 unresolved" run, now excluded because this port resolves typed fields exactly
  rather than by regex:
  - **14 sites** where the prototype's field-detection genuinely missed a typed field
    that is there in the source (`CalendarProps::dir`, confirmed by direct read;
    4 more `dir` sites in `select`/`tabs`) -- a prototype bug, not a gate bug.
  - Generic Props types (`ComboboxOptionProps<T>`, `CommandItemProps<T>`,
    `SelectOptionProps<T>`, `TagOptionProps<T>`, ~16 components total across
    `primitives/src` and `preview/src/components`): the prototype's
    `OLD_STYLE_PROPS_RE` regex required the *entire* type text to be a bare identifier,
    so `props: FooProps<T>` never matched and silently fell through to treating the
    literal parameter name `props` as the only typed field -- a real false-positive
    class this port found empirically (spot-checking `component-attributes-forward`
    surfaced `ComboboxOption`'s genuinely-typed `id` field being flagged) and fixed by
    construction: the analyzer now takes the base type identifier regardless of generic
    arguments, since `syn` already hands us the real parsed type. This class was **not
    fully disclosed** in the prototype's own limitations list; finding and fixing it is
    this port's own contribution, not something it was told to look for.

Zero "unresolved enclosing fn" cases, matching the prototype's own final run.

## Split: what a typed field can fix vs. what it structurally cannot

| Bucket | Count | What's needed |
|---|---|---|
| `no typed field/param of this name` (bare Rust idents: `role`, `class`, `aria_label`, ...) | 448 | A typed prop field, the `use_id_or` pattern generalized (see `primitives/src/popover.rs`), **or** `merge_attributes` at the site (see `preview/src/components/label/component.rs` post-`5fc1439`, or `combobox/components/option.rs`'s own `attributes!`/`merge_attributes` pairing) |
| `always (string-literal name, cannot be a typed field)` (`"data-state"`, `"aria-hidden"`, ...) | 239 | **Cannot** be a typed field -- Rust field/parameter names cannot contain `-`. Needs `merge_attributes` at the site, or a future API-level construction (option (b) in the root-cause doc's recommendation) |

Top names in the never-typeable bucket: `data-state` 49, `data-disabled` 48,
`data-slot` 30, `data-direction` 23, `data-orientation` 16, `data-align` 10,
`data-side` 8, plus one-off `data-*`/`aria-*` string-literal spellings of names that
elsewhere in the codebase are *also* written as the bare-ident sugar (e.g. `aria-hidden`
× 3 as a literal vs. `aria_hidden` × 9 as a bare ident) -- both spellings are valid rsx
and both are real instances; only the bare-ident spelling is fixable by a typed field.

Top names in the fixable bucket: `role` 93, `class` 56, `tabindex` 41, `type` 28,
`aria_label` 22, `style` 19, `aria_labelledby` 13, `popover` 12, `aria_disabled` 12,
`id` 10 (the five row-93-named instances plus five more), `aria_hidden` 9, `disabled` 8,
`aria_orientation`/`aria_modal`/`aria_describedby`/`aria_controls` 8 each, `dir` 7.

`role` and `class` alone are 149 of the 687 -- almost entirely the same shape repeated
across themed wrapper components in `preview/src/components/**`, each independently
writing `class: "dx-whatever"` beside its own forwarded spread.

## Heaviest files (primary check)

`calendar.rs` 31, `color_picker.rs` 29, `popover.rs` 28 (its own `id` is correctly *not*
among them -- typed, per `f2be1d7`), `context_menu.rs` 26, `slider.rs`/`date_picker.rs`/
`command.rs` 23 each, `dropdown_menu.rs` 22, `carousel.rs` 21, `resizable.rs`/`navbar.rs`
18 each, `tag_group.rs`/`menubar.rs` 17 each, `toast.rs`/`navigation_menu.rs` 16 each.
All in `primitives/src/`; `preview/src/components/**` sites are individually smaller
(2-6 each) but far more numerous (spread across ~55 files), which is why the file-count
total (70) is dominated by preview wrappers even though the single heaviest files are
primitives.

## A second, related class this lane found while building the first check

The lane brief flagged a known gap the prototype could not detect: the `5fc1439`-shaped
variant, where a component invocation sets an ad-hoc `#[props(extends =
GlobalAttributes)]` key directly (e.g. `class: "dx-label"`) *and separately* forwards a
whole `attributes: expr` field to the *same child* -- not a same-element `..spread`, but
the identical VNode-layer mechanism one call-site hop over (both funnel into the child's
one `attributes: Vec<Attribute>` field). This lane chose to **handle it**, not just
disclose it: the same analyzer also flags this shape (`kind ==
component-attributes-forward` in its output), conservatively -- only when (a) the
ad-hoc key's name is a real `GlobalAttributes` ident (mechanically extracted, see below;
this deliberately misses ad-hoc keys from a narrower `#[props(extends = button)]`-style
group, e.g. `disabled` on `PopoverTrigger` -- a known, disclosed precision boundary, not
a bug) and (b) the target component's own typed fields are resolved and confirmed not to
already claim that name (the same discriminator as the primary check, applied one hop
over).

**This was not expected to find much** -- the one known instance (`preview/src/components/
label/component.rs`) was already fixed by `5fc1439` before this lane started. It found
**105 currently-live sites across 27 files instead**, the single largest being
`preview/src/components/calendar/component.rs` (16). Concrete, hand-verified example
(`preview/src/components/accordion/component.rs:11-19`):

```rust
accordion::Accordion {
    class: "dx-accordion",       // ad-hoc GlobalAttributes key
    ...
    attributes: props.attributes, // separately forwarded, unmerged
    {props.children}
}
```

`AccordionProps` (`primitives/src/accordion.rs`) has no typed `class` field -- only
`#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>` -- so any caller of
preview's `Accordion` who passes their own `class` gets it appended alongside
`"dx-accordion"` in the same `Vec`, the exact `5fc1439` shape, just not yet exercised by
a demo page. All four of `accordion/component.rs`'s wrapper functions
(`Accordion`/`AccordionItem`/`AccordionTrigger`/`AccordionContent`) have this shape.
Spot-checked eight sites across five files by hand (accordion ×4, combobox's `id`
exclusion, color_picker's `aria_label`/`aria_expanded`, date_picker's primitive-to-
primitive `aria_label`); all confirmed real, zero false positives found in the sample.
Because this check's own precision has far less validation history than the primary
one (it is new, not a hardened port of a prototype others already iterated on), treat
the 105 as a strong lead, not a fully audited count the way the primary 687 now is.

By name: `class` 94, `aria_label` 7, `tabindex`/`style`/`aria_sort`/`aria_expanded` 1
each. This is reported and ratcheted **separately** from the primary 687 (its own
baseline file, its own section here) -- it is a different discovery from a heuristic the
prototype never attempted, not part of the "687 vs. 623" comparison above.

## The style-shorthand and global-attribute ident lists

Both `dev-docs/issues/duplicate-attribute-style-shorthand-idents.txt` (407 idents,
excluded from the primary check -- these fold into one `style="...;"` string on SSR, a
different, non-erroring code path) and the new
`dev-docs/issues/duplicate-attribute-global-attribute-idents.txt` (80 idents, used only
by the `component-attributes-forward` check above) are mechanically regenerated, not
hand-copied, by `scripts/attr-spread-collision-checker/regenerate-attribute-idents.py`
from this lockfile's own `dioxus-html/src/attribute_groups.rs` (verified: regenerating
the style list reproduces the existing 407-line file byte-for-byte). Re-run that script
after any `dioxus-html` version bump.

## What this still does not find (disclosed, not silently inherited)

- An `rsx! { .. }` invocation written lexically *inside* another `rsx! { .. }`'s own
  token stream (as opposed to a plain nested element/component, which the analyzer does
  walk structurally). Believed rare-to-nonexistent in idiomatic Dioxus code.
- The `component-attributes-forward` check's own stated conservatism above
  (`GlobalAttributes`-only, target-must-resolve).
- Anything where the collision spans two different, unrelated attributes rather than
  the same name appearing twice.
- A full, exhaustive hand-verification of all 792 findings. Spot-checked roughly two
  dozen across both checks, both authoring styles, multiple files, and every reason
  bucket; the remainder rely on the analyzer's correctness, not individual confirmation.

## What to do with this

Per `CLAUDE.md`'s own construction guidance, this is a class, not 792 unrelated
instances: the fixable bucket (448 + the `component-attributes-forward` 105 = 553) wants
either a typed field (best when the name is a stable, meaningful identity like `id`) or
the `merge_attributes`/`attributes!` construction `combobox`/`label` already use (best
for a default-plus-override like `class`); the never-typeable 239 need
`merge_attributes` at the site or the API-level construction the root-cause doc's option
(b) describes, since no per-name fix can ever reach them. None of this is fixed by this
lane (detection only, per its brief); this document is the hand-off.
