# Fork-network research

What exists in the 81 forks of `DioxusLabs/dioxus-components` that upstream does not have, what is worth taking, and how to take it.

**Baseline:** `bf007c1` — upstream `main`, unchanged since 2026-06-29. This repo tracked it exactly at scan time (2026-08-29).

| Document | Answers |
|---|---|
| [`adopt-fork-fixes-results.md`](./adopt-fork-fixes-results.md) | Which **bug fixes** exist in forks and were never merged. Four are still live on `main`; ~11 conditional; ~45 rejected with reasons. |
| [`capability-gaps.md`](./capability-gaps.md) | Which **capabilities** upstream is missing — form submission, focus restore, scroll lock, `aria-hidden`, collision detection, typeahead, RTL — and which fork closed each. |
| [`lifting-from-forks.md`](./lifting-from-forks.md) | **How to port** any of it: licensing, the three lift shapes, name mapping, the traps, and per-item recipes. |
| [`conformance-harness.md`](./conformance-harness.md) | The **harness design**: three labelled rule tiers (APG / HTML / Radix-opinion), verified sources for each, and how each tier calibrates itself against a known-correct reference. |
| `../playwright/oracle-focus-restore.spec.ts` | The **executable oracle**: APG conformance tests that turn a claim into a reproducible result. |

## The short version

Three findings outrank everything else, all confirmed against the current source:

1. **`RadioGroup` and `Select` accept `name`/`required`, document them as being for form submission, and silently ignore them.** A developer following the documented API ships a broken form. `Checkbox` does it correctly and shows the pattern.
2. **No overlay does collision detection.** Placement is static CSS keyed off `data-side`, so anything near a viewport edge renders off-screen.
3. **Closing a `DropdownMenu`, `ContextMenu`, `Menubar` or `Select` never returns focus to the trigger** — confirmed by execution, four failing tests against a passing `Dialog` control.

Beyond those: no body scroll lock, no `aria-hidden` on background content while a modal is open (a WCAG-class defect), no typeahead outside `select/`, no RTL.

One fork, `dignifiedquire/dx-components`, is the source for most of it, and its accessibility modules are standalone files by design. It is **not** a superset of upstream, though — it lacks `color_picker`, `tag_group`, all virtualization, and the current `selectable`/`selection` machinery — so forking it wholesale trades one set of gaps for another.

## Confidence

Read this before acting on any of it.

- **Executed:** focus restore. The oracle ran against the app built from this commit — four failures, one passing control. It also *corrected* the static analysis: focus falls to `<body>` only in `ContextMenu`; `DropdownMenu` and `Menubar` keep focus on the closed menu's item.
- **Verified statically:** everything else. Claims are backed by reading current source and fork refs, and every cherry-pick result in `adopt-fork-fixes-results.md` §8 was executed. Nothing else was compiled or run.
- **Point-in-time:** PR states and fork contents were read on the scan date. Re-check before acting.
- **Reviewed:** the fix report went through three adversarial review rounds, which caught four factual errors and two broken commands. The capability documents have not had equivalent review.

## Where the layers are

```
DioxusLabs/dioxus  (framework — main is on 0.8.0-alpha.1)
        ↓
dioxus-primitives  ← primitives/ IN THIS REPO. All accessibility behaviour lives here.
        ↓
preview/           (styled showcase)
```

There is no separate primitives repository. The framework offers no body/scroll API and no portal support, so `document::eval` is the only mechanism available for scroll lock and `aria-hidden` — which is why both forks implement them in JavaScript.
