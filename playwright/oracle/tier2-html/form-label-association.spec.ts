/**
 * ORACLE: tier 2 (HTML) — <label> association.
 *
 * Source: docs/conformance-harness.md, "Tier 2 — HTML forms", rule checklist
 * item 7 ("A <label> association focuses and activates the control"), citing
 * WHATWG HTML's the-label-element:
 *   https://html.spec.whatwg.org/multipage/forms.html#the-label-element
 * ("if a labeled control is not disabled, its label's activation behavior
 * focuses the labeled control and fires a click event at it" — a browser
 * platform behavior, not something this library implements itself; the
 * point of this oracle is to prove the library's own controls actually
 * expose a *labelable, focusable, click-activatable* element at the `id` a
 * caller's `<label for>` points at, the same way a native control does).
 *
 * This file was split out of `form-participation.spec.ts` deliberately (see
 * that file's own header, rule 7's deferral note): a focus/activation
 * assertion is a different shape from that file's entry-list assertions, and
 * mixing the two would blur this oracle with the tier-1 focus-restore one.
 *
 * Fixture: preview/src/components/form/component.rs (`FormFixture`), same as
 * `form-participation.spec.ts` — every row there already pairs a
 * `label[for]` with a native reference control AND (for every control
 * *except* Select, see "Known fixture gap" below) the library control, so no
 * fixture change was needed for this rule.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): every rule below
 * runs against the native reference control first (prefixed CALIBRATION: in
 * the test name) and the library control second. A CALIBRATION failure would
 * mean the *test* is wrong, not the component.
 *
 * Method: click the label's own TEXT node, never the label element's whole
 * bounding box and never the control itself — for RadioGroup (and the native
 * radio reference), the `<label>` both wraps its control AND carries a
 * matching `for` (belt-and-braces markup already in the fixture), so a plain
 * `label.click()` could land on either the control or the text depending on
 * hit-testing geometry and would not distinguish "the label's activation
 * behavior worked" from "I directly clicked the 20px control that happens to
 * sit under the label's bounding-box center." Scoping the click to the
 * label's own trailing text (`label.getByText(...)`) makes every assertion
 * here actually exercise `for`/wrapping-based association, not a coincidence
 * of layout. Checkbox/Switch's label and control are markup *siblings* (no
 * nesting), so this is only strictly necessary for RadioGroup, but the same
 * technique is used everywhere below for one consistent method across every
 * rule 7 case.
 *
 * Known fixture gap, found while writing this file (recorded, not fixed
 * here — a `FormFixture` composition decision, not a rule-7 defect in any
 * primitive): the library `Select`'s field
 * (`preview/src/components/form/component.rs`, "Fruit (library select)")
 * uses a plain `span` for its caption, not a `label[for]`, unlike every
 * other control pair in this fixture (including Select's own native
 * reference, which does use `label[for="fruit-native"]`) — it relies on
 * `Select`'s `trigger_aria_label` prop for its accessible name instead.
 * `form-participation.spec.ts`'s own rule-7 deferral note says "every
 * control here already has a paired `<label for>`", which is accurate for
 * six of seven control families but not this one. Select's CALIBRATION case
 * is still run below (proving the rule/technique itself against a native
 * `<select>`); there is no library-subject counterpart to run it against in
 * this fixture, so none is asserted — retarget this once (if ever) `Select`
 * gains a real `<label for>` wired to a caller-visible field caption, rather
 * than adding a second, competing accessible-name mechanism here.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import { BASE_URL } from "../../base-url";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const gotoForm = (page: Page) =>
  page.goto(`${BASE_URL}/component/?name=form&`, {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/** The label's own text node, scoped under its `for`, never the whole label
 * box and never the control it may also wrap -- see the file header's
 * "Method" note. */
function labelText(page: Page, forId: string, text: string): Locator {
  return page.locator(`label[for="${forId}"]`).getByText(text, { exact: true });
}

test.describe("Rule 7 — <label> association focuses and activates the control", () => {
  test("CALIBRATION: native checkbox — clicking its label focuses and toggles it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#chk-native");
    const before = await control.isChecked();
    await labelText(page, "chk-native", "Accept terms (native)").click();
    await expect(control).toBeFocused();
    expect(await control.isChecked()).toBe(!before);
  });

  test("Checkbox: clicking its label focuses and toggles it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#chk-lib");
    const before = await control.getAttribute("aria-checked");
    await labelText(page, "chk-lib", "Accept terms (library)").click();
    await expect(control).toBeFocused();
    await expect(control).toHaveAttribute("aria-checked", before === "true" ? "false" : "true");
  });

  test("CALIBRATION: native switch-shaped checkbox — clicking its label focuses and toggles it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#switch-native");
    const before = await control.isChecked();
    await labelText(page, "switch-native", "Notifications (native reference)").click();
    await expect(control).toBeFocused();
    expect(await control.isChecked()).toBe(!before);
  });

  test("Switch: clicking its label focuses and toggles it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#switch-lib");
    const before = await control.getAttribute("aria-checked");
    await labelText(page, "switch-lib", "Notifications (library switch)").click();
    await expect(control).toBeFocused();
    await expect(control).toHaveAttribute("aria-checked", before === "true" ? "false" : "true");
  });

  test("CALIBRATION: native radio — clicking an unchecked item's label focuses and checks it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#plan-native-pro");
    await expect(control).not.toBeChecked();
    await labelText(page, "plan-native-pro", "Pro").click();
    await expect(control).toBeFocused();
    await expect(control).toBeChecked();
  });

  test("RadioGroup: clicking an unchecked item's label focuses and checks it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#plan-lib-pro");
    await expect(control).toHaveAttribute("aria-checked", "false");
    await labelText(page, "plan-lib-pro", "Pro").click();
    await expect(control).toBeFocused();
    await expect(control).toHaveAttribute("aria-checked", "true");
    // Roving tabindex / single-selection semantics, checked here as a
    // by-product of the same click rather than a separate rule: activating
    // one radio in the group deselects the others.
    await expect(page.locator("#plan-lib-starter")).toHaveAttribute("aria-checked", "false");
  });

  test("CALIBRATION: native <select> — clicking its label focuses it", async ({ page }) => {
    await gotoForm(page);
    const control = page.locator("#fruit-native");
    await labelText(page, "fruit-native", "Fruit (native reference)").click();
    await expect(control).toBeFocused();
  });
});
