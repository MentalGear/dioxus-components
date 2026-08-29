/**
 * ORACLE: tier 2 (HTML) — form participation.
 *
 * Source: docs/conformance-harness.md, "Tier 2 — HTML forms", drawing on:
 *   - WHATWG HTML §form-control-infrastructure
 *     https://html.spec.whatwg.org/multipage/form-control-infrastructure.html
 *     (submittable elements, the `name` attribute, the constraint validation
 *     API: `willValidate`, `validity.valueMissing`)
 *   - WHATWG HTML §forms, "Constructing the entry list"
 *     https://html.spec.whatwg.org/multipage/forms.html#constructing-the-entry-list
 *     (which controls contribute, and what they contribute)
 *   - WHATWG HTML §forms, "resetting a form"
 *     https://html.spec.whatwg.org/multipage/forms.html#resetting-a-form
 *
 * Fixture: preview/src/components/form/component.rs (`FormFixture`), served at
 * /component/?name=form. Every library control there is paired with a native
 * reference control sharing a parallel `name`, inside a real <form>. Submitting
 * builds `new FormData(form)` in-browser (the actual algorithm, not a Dioxus-side
 * approximation of it) and renders it as `name=value` lines; a capturing
 * `invalid` listener on the required-fields form records which controls the
 * browser itself refused to submit through.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): every rule below
 * runs against the native reference control first (prefixed CALIBRATION: in
 * the test name) and the library control second. A CALIBRATION failure means
 * the *test* is wrong, not the component, and must be fixed before anything
 * else — see "Harness adjustment" below for the one fixture bug this
 * process actually found and fixed.
 *
 * Rules implemented (numbering per conformance-harness.md's tier-2 checklist):
 *   1. A named control contributes `name=value` to FormData on submit.
 *   2. An unchecked/unselected control contributes nothing — not an empty string.
 *   3. A disabled control is barred from constraint validation and excluded
 *      from the entry list.
 *   4. `required` with no value blocks submission and sets `validity.valueMissing`.
 *   6. Form reset restores the initial value.
 *
 * Rules deliberately NOT implemented here, and why:
 *   5. checkValidity()/reportValidity()/willValidate "behave per spec" as a
 *      general API-surface contract. Rules 3 and 4 below already exercise
 *      `willValidate` (rule 3) and `validity.valueMissing` (rule 4) on the
 *      exact controls that matter; a separate rule 5 suite would mostly
 *      re-assert the same wiring under a different name rather than reach
 *      new component behaviour. Left for a future pass if a control ever
 *      needs `checkValidity()`/`reportValidity()` called programmatically
 *      (e.g. a "Validate" button), which this fixture has no reason to add yet.
 *   7. <label> association (focus + activation). This is a real, distinct
 *      rule (every control here already has a paired <label for>), but it
 *      needs its own click/focus assertions rather than entry-list ones, and
 *      mixing "which element received focus" assertions into this file would
 *      blur it with the tier-1 focus-restore oracle. Left for a follow-up file.
 *   8. The `form` attribute (a control rendered outside the <form>). The
 *      fixture has no such control today; adding one is a fixture change with
 *      no bearing on the defects rules 1-4/6 already surface. Left for later.
 *
 * Harness adjustment made during calibration (fixture, not primitive):
 *   `preview/src/components/form/component.rs`'s native `#chk-native` and
 *   `#chk-disabled-native` reference inputs originally used the rsx attribute
 *   `checked: true`. Per dioxus-interpreter-js's `set_attribute.ts`, the rsx
 *   name `checked` maps to the live `.checked` IDL *property*, while
 *   `initial_checked` maps to `.defaultChecked` (the `checked` *content
 *   attribute*). The HTML "reset algorithm" for checkboxes reads the content
 *   attribute, not the live property — so with `checked: true`, resetting the
 *   fixture's own native calibration control did NOT restore it to checked,
 *   which would have made rule 6's CALIBRATION assertion fail on a plain
 *   native `<input>` (a contradiction: native checkboxes correctly restore on
 *   reset in every real browser). That is squarely "the test [fixture] is
 *   wrong, not the component" per conformance-harness.md — fixed by switching
 *   those two inputs to `initial_checked: true`. This does not touch
 *   primitives/src or any component behaviour.
 *
 * A parallel, *uncorrected* instance of the same root cause turned out to be
 * real inside Checkbox's own primitive (`BubbleInput`, in primitives/src,
 * intentionally left untouched) — see the rule 6 section below for what that
 * produces.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const gotoForm = (page: Page) =>
  page.goto("http://127.0.0.1:8080/component/?name=form&", {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/** Splits a `<pre>` entry-list dump into non-empty `name=value` lines. */
function lines(text: string | null): string[] {
  return (text ?? "").split("\n").map((l) => l.trim()).filter(Boolean);
}

/** True if some line is exactly `key=value`. */
function hasEntry(entries: string[], key: string, value: string): boolean {
  return entries.includes(`${key}=${value}`);
}

/** True if any line's key (before the first `=`) matches. */
function hasKey(entries: string[], key: string): boolean {
  return entries.some((l) => l.slice(0, l.indexOf("=")) === key);
}

/** Clicks a submit button and waits for its `<pre>` result to update, via
 * the fixture's own `data-submit-count` bump — never a timeout. */
async function submitAndRead(
  page: Page,
  submitSelector: string,
  result: Locator,
): Promise<string[]> {
  const before = parseInt((await result.getAttribute("data-submit-count")) ?? "0", 10);
  await page.locator(submitSelector).click();
  await expect(result).toHaveAttribute("data-submit-count", String(before + 1));
  return lines(await result.textContent());
}

/** Submits the required-fields form and waits for the capturing `invalid`
 * listener to report (data-invalid-count bump), returning the blocked
 * control names plus whether the form actually got through. */
async function submitRequiredAndRead(page: Page): Promise<{
  blocked: string[];
  submitCount: string | null;
}> {
  const report = page.locator("#invalid-report");
  const result = page.locator("#required-result");
  const before = parseInt((await report.getAttribute("data-invalid-count")) ?? "0", 10);
  await page.locator("#required-submit").click();
  await expect(report).toHaveAttribute("data-invalid-count", String(before + 1));
  return {
    blocked: lines(await report.textContent()),
    submitCount: await result.getAttribute("data-submit-count"),
  };
}

const formResult = (page: Page) => page.locator("#form-result");

test.describe("Rule 1 — a named control contributes name=value to FormData on submit", () => {
  test("CALIBRATION: native checkbox, checked by default, contributes its value", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(entries, "native checkbox defaults to checked").toContainEqual("terms-native=accepted");
  });

  test("Checkbox: checked by default, contributes its value", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(entries).toContainEqual("terms-lib=accepted");
  });

  test("CALIBRATION: native radio, once checked, contributes name=value", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#plan-native-pro").click();
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries, "plan-native", "pro")).toBe(true);
  });

  test("RadioGroup: once an item is checked, contributes name=value", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#plan-lib-pro").click();
    await expect(page.locator("#plan-lib-pro")).toHaveAttribute("data-state", "checked");
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(
      hasEntry(entries, "plan-lib", "pro"),
      "RadioGroup declares `name` but RadioItem never renders a submittable " +
        "element (no <input>, no BubbleInput) — see capability-gaps.md.",
    ).toBe(true);
  });

  test("CALIBRATION: native select contributes its selected option's value", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries, "fruit-native", "apple")).toBe(true);
    await page.locator("#fruit-native").selectOption("banana");
    const entries2 = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries2, "fruit-native", "banana")).toBe(true);
  });

  test("Select: contributes its selected option's value", async ({ page }) => {
    await gotoForm(page);
    const trigger = page.getByRole("button", { name: "Fruit (library)" });
    await trigger.click();
    await page.getByLabel("Fruit options (library)").getByRole("option", { name: "Banana" }).click();
    // SelectValue displays the selected option's underlying value (the raw
    // `T`, lowercase "banana" per this fixture), not its rendered label.
    await expect(trigger).toHaveText("banana");
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(
      hasKey(entries, "fruit-lib"),
      "Select declares `name` but never renders a submittable element for it " +
        "(no <select>, no hidden input) — see capability-gaps.md.",
    ).toBe(true);
  });
});

test.describe("Rule 2 — an unchecked/unselected control contributes nothing (not an empty string)", () => {
  // Select is not exercised here: a native <select> always has a currently
  // selected option (the spec defaults it to the first, absent an explicit
  // `selected`), so it can never be literally "unselected" the way a checkbox
  // or radio group can — it always contributes *something*. Rule 1 above
  // already covers whether that something is correct; a "contributes
  // nothing" assertion would not describe any reachable <select> state.

  test("CALIBRATION: unchecked native checkbox contributes nothing", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#chk-native").click(); // uncheck the checked-by-default control
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "terms-native")).toBe(false);
  });

  test("Checkbox: unchecked contributes nothing", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#chk-lib").click(); // uncheck the checked-by-default control
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "terms-lib")).toBe(false);
  });

  test("CALIBRATION: native switch-style checkbox, unchecked by default, contributes nothing", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "notify-native")).toBe(false);
  });

  test("Switch: unchecked by default, contributes nothing", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "notify-lib")).toBe(false);
  });

  test("CALIBRATION: native radio group, none checked by default, contributes nothing", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "plan-native")).toBe(false);
  });

  test("RadioGroup: none checked by default, contributes nothing", async ({ page }) => {
    await gotoForm(page);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    // NOTE: this passes today, but not for the reason the rule intends — see
    // "RadioGroup: once an item is checked, contributes name=value" (rule 1)
    // above, which is RED. RadioGroup contributes nothing in *every* state,
    // checked or not, because it never renders a submittable element at all;
    // an absent entry here is not evidence the "unchecked" state specifically
    // is handled, only that nothing is. Kept as a real (if weak) assertion:
    // it is still a spec-true fact about the current DOM, and would catch a
    // regression where an unselected item started leaking a stray entry.
    expect(hasKey(entries, "plan-lib")).toBe(false);
  });
});

test.describe("Rule 3 — a disabled control is barred from constraint validation and excluded from the entry list", () => {
  test("CALIBRATION: disabled+checked native checkbox is excluded and unvalidatable", async ({ page }) => {
    await gotoForm(page);
    const willValidate = await page
      .locator("#chk-disabled-native")
      .evaluate((el: HTMLInputElement) => el.willValidate);
    expect(willValidate, "a disabled control is never a candidate for constraint validation").toBe(false);

    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(
      hasKey(entries, "promo-native"),
      "disabled controls are excluded from the entry list regardless of checkedness",
    ).toBe(false);
  });

  test("Checkbox: disabled+checked is excluded and unvalidatable", async ({ page }) => {
    await gotoForm(page);
    const hidden = page.locator('input[name="promo-lib"]');
    await expect(hidden).toBeChecked(); // sanity: the fixture really did check it
    const willValidate = await hidden.evaluate((el: HTMLInputElement) => el.willValidate);
    expect(willValidate).toBe(false);

    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "promo-lib")).toBe(false);
  });
});

test.describe("Rule 4 — required with no value blocks submission and sets validity.valueMissing", () => {
  // A single submit of the untouched required-fields form leaves every
  // required control in the form unsatisfied at once, so one submission
  // exercises every control below in the same `invalid` batch (see
  // component.rs's watch_invalid_js: a capturing listener, batched via
  // microtask into one write per attempt). Each test reloads first, per the
  // task's "reset state between tests via page reload".

  test("CALIBRATION: required native checkbox blocks submission and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("terms-required-native");
    expect(submitCount, "a blocked submission must not reach the entry-list handler").toBe("0");
    const valueMissing = await page
      .locator("#chk-required-native")
      .evaluate((el: HTMLInputElement) => el.validity.valueMissing);
    expect(valueMissing).toBe(true);
  });

  test("Checkbox: required blocks submission and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("terms-required-lib");
    expect(submitCount).toBe("0");
    const valueMissing = await page
      .locator('input[name="terms-required-lib"]')
      .evaluate((el: HTMLInputElement) => el.validity.valueMissing);
    expect(valueMissing).toBe(true);
  });

  test("CALIBRATION: required native switch-style checkbox blocks submission and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("opt-in-required-native");
    expect(submitCount).toBe("0");
  });

  test("Switch: required does NOT block submission (required is not forwarded to the hidden input)", async ({ page }) => {
    await gotoForm(page);
    const { blocked } = await submitRequiredAndRead(page);
    const hiddenRequired = await page
      .locator('input[name="opt-in-required-lib"]')
      .evaluate((el: HTMLInputElement) => el.required);
    expect(
      hiddenRequired,
      "Switch's hidden mirror <input> never receives `required: props.required` " +
        "(see primitives/src/switch.rs) even though the component forwards " +
        "`name`, `value`, `checked` and `disabled` to it — required is the one " +
        "prop dropped on the floor.",
    ).toBe(false);
    expect(
      blocked,
      "with `required` missing on the actual DOM node, the browser never " +
        "considers this control invalid, so it never appears in the capturing " +
        "`invalid` listener's report",
    ).not.toContain("opt-in-required-lib");
  });

  test("CALIBRATION: required native radio group blocks submission and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("tier-required-native");
    expect(submitCount).toBe("0");
  });

  test("RadioGroup: required does NOT block submission (no submittable element exists to be invalid)", async ({ page }) => {
    await gotoForm(page);
    const count = await page.locator('input[name="tier-required-lib"]').count();
    expect(count, "RadioGroup renders no <input> at all for this name").toBe(0);
    const { blocked } = await submitRequiredAndRead(page);
    expect(
      blocked,
      "no element exists with this name, so nothing can ever fire `invalid` " +
        "for it or block submission on its account",
    ).not.toContain("tier-required-lib");
  });

  test("CALIBRATION: required native select blocks submission on its empty placeholder value and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("fruit-required-native");
    expect(submitCount).toBe("0");
    const valueMissing = await page
      .locator("#fruit-required-native")
      .evaluate((el: HTMLSelectElement) => el.validity.valueMissing);
    expect(valueMissing).toBe(true);
  });

  // Select has no library-side assertion here: `Select::<String>` has no
  // `required` prop at all yet (docs/plan.md Phase 1.3), so there is nothing
  // to submit against — the fixture's own comment says as much. Testing "does
  // an absent prop fail to block" would just be testing that Rust doesn't
  // have a field it was never given.
});

test.describe("Rule 6 — form reset restores the initial value", () => {
  test("CALIBRATION: native checkbox reset restores checked-by-default", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#chk-native").click(); // uncheck it
    await expect(page.locator("#chk-native")).not.toBeChecked();
    await page.locator("#entries-reset").click();
    await expect(page.locator("#chk-native")).toBeChecked();
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries, "terms-native", "accepted")).toBe(true);
  });

  test("Checkbox: reset does NOT restore checked-by-default (BubbleInput has no persisted default)", async ({ page }) => {
    await gotoForm(page);
    const hidden = page.locator('input[name="terms-lib"]');
    await page.locator("#chk-lib").click(); // uncheck it
    await expect(hidden).not.toBeChecked();
    await page.locator("#entries-reset").click();
    expect(
      await hidden.isChecked(),
      "primitives/src/checkbox.rs's BubbleInput sets the mirror <input>'s " +
        "default via the rsx attribute `checked: default_checked != Unchecked`. " +
        "Per dioxus-interpreter-js's set_attribute.ts, the rsx name `checked` " +
        "always maps to the live `.checked` IDL property, never to the " +
        "`checked` content attribute (`initial_checked`/`.defaultChecked` is " +
        "the one that does). The HTML reset algorithm restores checkedness " +
        "from the *content attribute*, which BubbleInput never sets — so " +
        "resetting always reverts to unchecked, regardless of `default_checked`. " +
        "This is a genuine primitive defect (not touched, per task scope), and " +
        "it is why this rule is RED here even though rules 1-4 are GREEN for " +
        "Checkbox: the same fix (`initial_checked` instead of `checked`) that " +
        "this harness applied to its own native reference control (see file " +
        "header) has not been applied inside the primitive itself.",
    ).toBe(true);
  });

  test("CALIBRATION: native switch-style checkbox reset restores unchecked-by-default", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#switch-native").click(); // check it
    await expect(page.locator("#switch-native")).toBeChecked();
    await page.locator("#entries-reset").click();
    await expect(page.locator("#switch-native")).not.toBeChecked();
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "notify-native")).toBe(false);
  });

  test("Switch: reset restores unchecked-by-default", async ({ page }) => {
    await gotoForm(page);
    const hidden = page.locator('input[name="notify-lib"]');
    await page.locator("#switch-lib").click(); // check it
    await expect(hidden).toBeChecked();
    await page.locator("#entries-reset").click();
    // NOTE: unlike Checkbox, this passes -- but only because Form A's
    // default for Switch happens to be *unchecked*, and an absent `checked`
    // content attribute already means "unchecked" regardless of whether
    // Switch's own hidden-input binding (`checked: checked`, in
    // primitives/src/switch.rs) ever sets that attribute. It coincides with
    // correct behaviour here; it would NOT prove a `default_checked: true`
    // Switch resets correctly, for the identical structural reason Checkbox
    // fails above. Treat this as "not disproven by this fixture", not as a
    // clean bill of health.
    expect(await hidden.isChecked()).toBe(false);
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "notify-lib")).toBe(false);
  });

  test("CALIBRATION: native radio group reset restores none-checked", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#plan-native-pro").click();
    await expect(page.locator("#plan-native-pro")).toBeChecked();
    await page.locator("#entries-reset").click();
    await expect(page.locator("#plan-native-pro")).not.toBeChecked();
  });

  test("RadioGroup: reset does NOT restore none-checked (no <input>, nothing observes the reset event)", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#plan-lib-pro").click();
    await expect(page.locator("#plan-lib-pro")).toHaveAttribute("data-state", "checked");
    await page.locator("#entries-reset").click();
    expect(
      await page.locator("#plan-lib-pro").getAttribute("data-state"),
      "RadioGroup's selection lives purely in a Dioxus signal with no " +
        "underlying form-associated element, so the native `reset` event " +
        "(which only resets real form controls) has nothing to act on.",
    ).toBe("checked");
  });

  test("CALIBRATION: native select reset restores the default option", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#fruit-native").selectOption("banana");
    await page.locator("#entries-reset").click();
    await expect(page.locator("#fruit-native")).toHaveValue("apple");
  });

  test("Select: reset does NOT restore the default option (no <select>, nothing observes the reset event)", async ({ page }) => {
    await gotoForm(page);
    const trigger = page.getByRole("button", { name: "Fruit (library)" });
    await trigger.click();
    await page.getByLabel("Fruit options (library)").getByRole("option", { name: "Banana" }).click();
    // See the rule 1 Select test: SelectValue shows the raw value, not the label.
    await expect(trigger).toHaveText("banana");
    await page.locator("#entries-reset").click();
    expect(
      await trigger.textContent(),
      "Select's value lives purely in a Dioxus signal with no underlying " +
        "form-associated element for the native `reset` event to act on.",
    ).toBe("banana");
  });
});
