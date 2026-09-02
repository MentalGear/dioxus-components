# Tier 1 — APG

Source: [`../../../docs/conformance-harness.md`](../../../docs/conformance-harness.md), "Tier 1 — APG" and the tier table at the top of that document.

## Rule-source policy

A rule belongs here only if it cites a **W3C WAI-ARIA Authoring Practices (APG)** pattern section — the project's own stated contract (`README.md` requires new primitives to adhere to it). Every rule file must name the specific pattern section it is drawn from at the top of the file, as `oracle-focus-restore.spec.ts` already does.

## Calibration

Every rule here runs against **two subjects**: the component under test, and the pattern's own vendored APG example page under [`../reference/`](../reference/) (see that directory's README for the pinned commit). If the reference page fails a rule, the rule is wrong, not the component — per conformance-harness.md's "Calibration" section. Do not calibrate tier 1 against an internal control (e.g. another of this library's own components); that is what the current `oracle-focus-restore.spec.ts` does today and conformance-harness.md names as the thing to upgrade away from.

## What is mined but does not run here

- `w3c/aria-at` is the most authoritative statement of what a keystroke must do, but it asserts screen-reader speech output and needs real AT (NVDA/JAWS/VoiceOver). Mine it for rules; it cannot run in Playwright.
- `web-platform-tests/wpt` (`wai-aria/`) tests the browser's accessibility-tree mapping, not component behaviour. Useful for assertion technique, not as a conformance source.

## Status

Per conformance-harness.md's "Status" section: `oracle-focus-restore.spec.ts` (focus restore on Escape, internal control), `focus-restore-reference.spec.ts` (same rule calibrated against the vendored APG reference), `keyboard-matrix.spec.ts` (per-component keyboard rows), and `menu-roles.spec.ts` (menu pattern-class role contract, calibrated against the vendored menu-button reference).
