# Subjects

Source: [`../../../docs/conformance-harness.md`](../../../docs/conformance-harness.md), "Layout" ("`subjects/` adapters: route + selectors per implementation") and "Calibration" ("Every rule runs against two subjects").

## Purpose

A rule file (under `tier1-apg/`, `tier2-html/`, or `tier3-radix/`) must run unmodified against more than one subject: the component under test, plus whichever reference the tier's calibration table names (this library's own APG example page under `../reference/` for tier 1, a native HTML control in the same fixture for tier 2, a pinned Radix demo for tier 3).

To keep the rule logic itself subject-agnostic, each subject gets an **adapter** here: a small module that maps a subject name to

- the **route** (or file) to load for that subject, and
- the **selectors** needed to drive and assert on it (trigger, menu/listbox/group container, item(s), the native control to compare against, etc.)

so a rule file can do something like `for (const subject of subjects) { … }` instead of hard-coding one implementation's markup.

## Rule-source policy

This directory holds no rule sources of its own — no test here should assert a *new* behavioural claim. It only maps { subject → how to reach it in a page }. The run matrix described in conformance-harness.md is *rule × subject*; the output is a conformance matrix per component, which doubles as the library's accessibility scorecard.

## Status

Not implemented, and not blocked on anything anymore — Phase 0 item 0.4 (the form fixture) landed 2026-08-29. In practice, every oracle spec written since (`form-participation.spec.ts`, `keyboard-matrix.spec.ts`, `menu-roles.spec.ts`, `top-layer.spec.ts`, and the rest) has handled subject routing and selectors inline in the spec file itself rather than through a shared adapter here — the rule × subject matrix this directory was meant to factor out has so far stayed small enough per spec that the extra indirection hasn't paid for itself. Revisit if a rule needs to run against enough subjects that the inline approach starts duplicating real logic.
