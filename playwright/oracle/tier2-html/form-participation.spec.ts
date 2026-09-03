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
 * A parallel, *uncorrected* instance of the same root cause was found to be
 * real inside Checkbox's own primitive (`BubbleInput`, in primitives/src) --
 * that one, and the equivalent gaps in `Switch`, `RadioGroup` and `Select`,
 * are what docs/plan.md Phase 1 closes. See "Post-Phase-1 harness updates"
 * below for exactly which assertions changed as a result, and why a changed
 * assertion here is a harness correction rather than moving the goalposts:
 * each one previously asserted the *absence* of a submittable element/listener
 * (a true fact about the pre-Phase-1 code) and now asserts the presence of
 * the correct, spec-driven behaviour that replaced it -- the same rule,
 * evaluated against different (now-conforming) code.
 *
 * Post-Phase-1 harness updates (Phase 1 landed; see docs/plan.md):
 *   - Rule 1 (`RadioGroup`, `Select`): assertions were already forward-looking
 *     ("contributes name=value") and needed no change -- they went red before
 *     Phase 1 and green after, exactly as the harness is meant to.
 *   - Rule 4 (`Switch`, `RadioGroup`) and rule 6 (`Checkbox`, `RadioGroup`,
 *     `Select`): these previously asserted the *bug* (e.g. "required does
 *     NOT block submission") because that was, at the time, the true and only
 *     observable behaviour -- there was no submittable element to assert
 *     anything else about. Phase 1 flipped the underlying behaviour, so the
 *     assertions were rewritten to match (e.g. "required blocks submission
 *     and fires invalid"), following the same shape as the CALIBRATION test
 *     and the already-green Checkbox rule 4 test in the same describe block.
 *   - Rule 4 (`Select`): `Select` had no `required` prop before Phase 1.3, so
 *     there was nothing to assert; Phase 1.3 added the prop, so a new test
 *     was added here (mirroring the Checkbox/CALIBRATION shape) rather than
 *     rewriting an existing one. The fixture
 *     (`preview/src/components/form/component.rs`) was updated in the same
 *     change to set `required: true` on its "fruit-required-lib" Select,
 *     since the whole point of this fixture is exercising components per
 *     their *documented* API -- leaving `required` off a prop that now exists
 *     would just be re-testing "an absent field has no effect", per this
 *     file's own rule-4 comment from before Phase 1.3.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from "../../axe";

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

  test("Switch: required blocks submission and fires invalid", async ({ page }) => {
    // Phase 1.1 (docs/plan.md): Switch's hidden mirror <input> now forwards
    // `required: props.required` (primitives/src/switch.rs) alongside the
    // `name`/`value`/`checked`/`disabled` it already forwarded. Previously
    // this assertion recorded the opposite (required missing from the DOM
    // node, so the browser never blocked or fired `invalid` on it) -- see
    // the file header's "Post-Phase-1 harness updates".
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("opt-in-required-lib");
    expect(submitCount).toBe("0");
    const valueMissing = await page
      .locator('input[name="opt-in-required-lib"]')
      .evaluate((el: HTMLInputElement) => el.validity.valueMissing);
    expect(valueMissing).toBe(true);
  });

  test("CALIBRATION: required native radio group blocks submission and fires invalid", async ({ page }) => {
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("tier-required-native");
    expect(submitCount).toBe("0");
  });

  test("RadioGroup: required blocks submission and fires invalid", async ({ page }) => {
    // Phase 1.2 (docs/plan.md): each RadioItem now renders a hidden
    // <input type="radio"> carrying the group's `name` and `required`.
    // Previously this asserted the opposite (no <input> at all, so nothing
    // could ever fire `invalid`) -- see the file header's "Post-Phase-1
    // harness updates".
    await gotoForm(page);
    const hiddenInputs = page.locator('input[name="tier-required-lib"]');
    await expect(
      hiddenInputs,
      "one hidden radio per RadioItem in the group",
    ).toHaveCount(3);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("tier-required-lib");
    expect(submitCount).toBe("0");
    const valueMissing = await hiddenInputs
      .first()
      .evaluate((el: HTMLInputElement) => el.validity.valueMissing);
    expect(
      valueMissing,
      "per the HTML spec, a required radio button group with none checked " +
        "reports valueMissing on each of its (required) radio inputs",
    ).toBe(true);
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

  test("Select: required blocks submission on its empty placeholder value and fires invalid", async ({ page }) => {
    // Phase 1.3 (docs/plan.md) added `required` to `SelectProps`, wired to
    // the hidden native <select> from the same phase (Rule 1 above). This
    // test did not exist before Phase 1.3 -- there was no `required` prop to
    // exercise, and the fixture's own Select here had no `required` set (see
    // the file header's "Post-Phase-1 harness updates" for why adding both
    // the test and the fixture attribute is a harness upgrade, not a new
    // requirement invented for this test).
    await gotoForm(page);
    const { blocked, submitCount } = await submitRequiredAndRead(page);
    expect(blocked).toContain("fruit-required-lib");
    expect(submitCount).toBe("0");
    const valueMissing = await page
      .locator('select[name="fruit-required-lib"]')
      .evaluate((el: HTMLSelectElement) => el.validity.valueMissing);
    expect(valueMissing).toBe(true);
  });
});

/**
 * Rule 4 extension — visible feedback for a blocked submit (live-site
 * report, item 2, 2026-09-01). Rule 4 above already proves the *block*
 * itself works (submitCount stays "0"); what was missing is that
 * `required` lives on each library control's visually-hidden native mirror
 * (`aria-hidden="true"`), and Chrome refuses to focus a non-focusable
 * hidden control or show its native validation bubble on one -- confirmed
 * via the browser console on submit: "An invalid form control ... is not
 * focusable." `component.rs`'s `watch_invalid_js` bridges this onto the
 * VISIBLE control (fixture-only fix, not a primitives change -- see that
 * function's doc for why): `data-invalid="true"` on every blocked control's
 * visible counterpart, and focus on the first one, matching a real
 * browser's own reportValidity() behavior of focusing/bubble-anchoring
 * only the first invalid control.
 */
test.describe("Rule 4 extension — a blocked submit gives visible feedback on the VISIBLE control, not just its hidden mirror", () => {
  test("focuses the first invalid control's visible counterpart", async ({ page }) => {
    await gotoForm(page);
    await submitRequiredAndRead(page);
    // `chk-required-lib` is the first required control in document order.
    const active = await page.evaluate(() => document.activeElement?.id ?? null);
    expect(active).toBe("chk-required-lib");
  });

  test("marks every blocked control's visible counterpart data-invalid, not just the focused one", async ({ page }) => {
    await gotoForm(page);
    await submitRequiredAndRead(page);

    await expect(page.locator("#chk-required-lib")).toHaveAttribute("data-invalid", "true");
    await expect(page.locator("#switch-required-lib")).toHaveAttribute("data-invalid", "true");
    // The first radio in the group -- native reportValidity() itself
    // focuses/anchors on the first radio of an unsatisfied required group,
    // so this fixture's bridge follows the same convention.
    await expect(page.locator("#tier-lib-small")).toHaveAttribute("data-invalid", "true");
    await expect(
      page.getByRole("button", { name: "Fruit, required (library)" }),
    ).toHaveAttribute("data-invalid", "true");
  });

  test("clears data-invalid once the control is fixed", async ({ page }) => {
    await gotoForm(page);
    await submitRequiredAndRead(page);
    await expect(page.locator("#chk-required-lib")).toHaveAttribute("data-invalid", "true");

    await page.locator("#chk-required-lib").click();

    await expect(page.locator("#chk-required-lib")).not.toHaveAttribute("data-invalid", "true");
  });

  test("clears every data-invalid marker on form reset", async ({ page }) => {
    await gotoForm(page);
    await submitRequiredAndRead(page);
    await expect(page.locator('[data-invalid="true"]').first()).toBeVisible();

    await page.locator("#required-reset").click();

    await expect(page.locator('[data-invalid="true"]')).toHaveCount(0);
  });
});

/**
 * axe (static rules) — the required-fields form in its invalid state after
 * a blocked submit (every required control marked `data-invalid`, focus
 * moved to the first one). No component spec reaches this state; it is
 * exactly the state WCAG 3.3.1 (Error Identification) cares about, and
 * axe's `aria-valid-attr-value`/`aria-required-children`-class rules can
 * catch a wrong ARIA error-state pattern here that this file's own
 * behaviour assertions (which check `data-invalid`/focus, not ARIA
 * validity) do not.
 */
test.describe("axe: invalid state after a blocked submit", () => {
  test("blocked required-fields submit has no automatically detectable a11y issues", async ({ page }) => {
    await gotoForm(page);
    await submitRequiredAndRead(page);
    await expect(page.locator('[data-invalid="true"]').first()).toBeVisible();
    await expectNoAxeViolations(page, "form: invalid state after blocked submit", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
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

  test("Checkbox: reset restores checked-by-default", async ({ page }) => {
    // Phase 1.4 (docs/plan.md): `BubbleInput` (primitives/src/checkbox.rs)
    // now sets the mirror <input>'s default via `initial_checked` (->
    // `.defaultChecked`, the `checked` *content attribute* the HTML reset
    // algorithm reads) instead of `checked` (-> only the live `.checked`
    // IDL property). It also listens for the owning form's `reset` event to
    // resync the component's own (Dioxus-side) checked state -- the browser
    // reset only touches this hidden input's own DOM state. Previously this
    // assertion recorded the opposite (reset had no effect at all); see the
    // file header's "Post-Phase-1 harness updates".
    await gotoForm(page);
    const hidden = page.locator('input[name="terms-lib"]');
    await page.locator("#chk-lib").click(); // uncheck it
    await expect(hidden).not.toBeChecked();
    await expect(page.locator("#chk-lib")).toHaveAttribute("data-state", "unchecked");
    await page.locator("#entries-reset").click();
    await expect(hidden).toBeChecked();
    await expect(
      page.locator("#chk-lib"),
      "the visible Checkbox button's own (Dioxus) state must resync too, not " +
        "just the hidden mirror input's DOM state",
    ).toHaveAttribute("data-state", "checked");
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries, "terms-lib", "accepted")).toBe(true);
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
    // Phase 1.1/1.4 (docs/plan.md): the hidden input now sets
    // `initial_checked` and Switch listens for the owning form's `reset`
    // event to resync its own visible state (primitives/src/switch.rs).
    // Before Phase 1.1 this test's DOM-only assertion passed *by
    // coincidence* -- an absent `checked` content attribute already means
    // "unchecked" regardless of whether the fix landed, since this fixture's
    // Switch default happens to be unchecked (a `default_checked: true`
    // Switch would have failed for the identical reason Checkbox did; see
    // its rule 6 test). The `data-state` assertion below is the part that
    // could not have passed by coincidence: nothing but a working reset
    // listener flips the *visible* button back after it was toggled on.
    await gotoForm(page);
    const hidden = page.locator('input[name="notify-lib"]');
    await page.locator("#switch-lib").click(); // check it
    await expect(hidden).toBeChecked();
    await expect(page.locator("#switch-lib")).toHaveAttribute("data-state", "checked");
    await page.locator("#entries-reset").click();
    expect(await hidden.isChecked()).toBe(false);
    await expect(page.locator("#switch-lib")).toHaveAttribute("data-state", "unchecked");
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

  test("RadioGroup: reset restores none-checked", async ({ page }) => {
    // Phase 1.2 (docs/plan.md): each RadioItem's hidden radio now sets
    // `initial_checked` from the group's `default_value`, and registers a
    // `reset` listener on its owning form that resyncs the group's own
    // (Dioxus-side) selection -- there is no single form-associated element
    // "the group" can listen on, so each item's own hidden radio does it
    // (idempotently: they all set the same group value back to the same
    // default). Previously this asserted the opposite (reset had no
    // observable effect at all); see the file header's "Post-Phase-1
    // harness updates".
    await gotoForm(page);
    await page.locator("#plan-lib-pro").click();
    await expect(page.locator("#plan-lib-pro")).toHaveAttribute("data-state", "checked");
    await page.locator("#entries-reset").click();
    await expect(page.locator("#plan-lib-pro")).toHaveAttribute("data-state", "unchecked");
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasKey(entries, "plan-lib")).toBe(false);
  });

  test("CALIBRATION: native select reset restores the default option", async ({ page }) => {
    await gotoForm(page);
    await page.locator("#fruit-native").selectOption("banana");
    await page.locator("#entries-reset").click();
    await expect(page.locator("#fruit-native")).toHaveValue("apple");
  });

  test("Select: reset restores the default option", async ({ page }) => {
    // Phase 1.3 (docs/plan.md): the hidden native <select> now marks its
    // default-value <option> `initial_selected`, and Select registers a
    // `reset` listener on its owning form that resyncs its own (Dioxus-side)
    // value back to `default_value`. Previously this asserted the opposite
    // (reset had no observable effect at all); see the file header's
    // "Post-Phase-1 harness updates".
    await gotoForm(page);
    const trigger = page.getByRole("button", { name: "Fruit (library)" });
    await trigger.click();
    await page.getByLabel("Fruit options (library)").getByRole("option", { name: "Banana" }).click();
    // See the rule 1 Select test: SelectValue shows the raw value, not the label.
    await expect(trigger).toHaveText("banana");
    await page.locator("#entries-reset").click();
    await expect(trigger).toHaveText("apple");
    const entries = await submitAndRead(page, "#entries-submit", formResult(page));
    expect(hasEntry(entries, "fruit-lib", "apple")).toBe(true);
  });
});
