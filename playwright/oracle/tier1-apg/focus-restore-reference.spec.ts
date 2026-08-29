/**
 * ORACLE: tier 1 (APG) — focus-restore rule, calibrated against W3C's own
 * reference implementation.
 *
 * Source pattern sections (both quoted in full in oracle-focus-restore.spec.ts,
 * repeated here because this file's whole point is to stand on its own as the
 * calibration for that rule):
 *
 *   "Escape: Closes the menu and sets focus to the menu button."
 *   — APG Menu Button pattern, keyboard interaction
 *     https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/
 *
 *   "Escape ... sets focus on the combobox [if it is not already there]."
 *   — APG Combobox pattern, keyboard interaction
 *     https://www.w3.org/WAI/ARIA/apg/patterns/combobox/
 *
 * Calibration (docs/conformance-harness.md, "Calibration" / tier1-apg/README.md):
 * `oracle-focus-restore.spec.ts` asserts this rule against this library's own
 * components, calibrated only against another of *this library's own*
 * components (Dialog) as an internal control. Per conformance-harness.md,
 * that is the weaker of the two possible references and "not yet calibrated
 * against an APG page" — this file is that upgrade. It runs the identical
 * rule against the pattern's own vendored example page
 * (playwright/oracle/reference/7e4034b/, pinned commit
 * 7e4034b262bc0d25332e330d8a582aaf34113829 of w3c/aria-practices — see that
 * directory's README for provenance), loaded over file:// so it needs no
 * network and cannot drift. If either assertion below ever goes red, the
 * *rule* is wrong (or w3c/aria-practices's own example regressed at the
 * pinned commit) — not any component in this library.
 *
 * Both pages are vendored, not fetched from w3.org, per
 * docs/conformance-harness.md's tier-1 calibration table ("Vendor the page").
 */

import { test, expect } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";

const REFERENCE_ROOT = path.resolve(__dirname, "../reference/7e4034b/content/patterns");

const menuButtonActionsUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "menu-button/examples/menu-button-actions.html"),
).href;

const comboboxSelectOnlyUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "combobox/examples/combobox-select-only.html"),
).href;

test.describe("APG Menu Button pattern — Actions Menu Button Example", () => {
  test("CALIBRATION: W3C's own example returns focus to the menu button on Escape", async ({ page }) => {
    await page.goto(menuButtonActionsUrl);

    const button = page.locator("#menubutton1");
    await expect(button).toHaveAttribute("aria-haspopup", "true");

    await button.click();
    await expect(button).toHaveAttribute("aria-expanded", "true");

    // This example moves real DOM focus onto the first menu item as soon as
    // the menu opens (see menu-button-actions.js's onButtonClick ->
    // setFocusToFirstMenuitem) -- unlike this library's DropdownMenu, which
    // needs an explicit ArrowDown to move focus off the trigger. Confirm
    // focus really did leave the button before treating Escape's return-focus
    // behaviour as meaningful, then roam once more as a keyboard user would.
    await expect(page.locator("#menu1 [role='menuitem']").first()).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("#menu1 [role='menuitem']").nth(1)).toBeFocused();

    await page.keyboard.press("Escape");

    await expect(button, "aria-expanded must return to false").toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(
      button,
      "APG's own menu-button-actions example: Escape must return focus to " +
        "the menu button",
    ).toBeFocused();
  });
});

test.describe("APG Combobox pattern — Select-Only Combobox Example", () => {
  test("CALIBRATION: W3C's own example keeps/returns focus on the combobox on Escape", async ({ page }) => {
    await page.goto(comboboxSelectOnlyUrl);

    const combo = page.locator("#combo1");
    await expect(combo).toHaveAttribute("role", "combobox");

    await combo.click();
    await expect(combo).toHaveAttribute("aria-expanded", "true");
    await expect(
      combo,
      "this pattern uses aria-activedescendant, not roving real focus, so DOM " +
        "focus is on the combobox itself the entire time the listbox is open",
    ).toBeFocused();

    // Move the active descendant, as a keyboard user would.
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Escape");

    await expect(combo, "aria-expanded must return to false").toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(
      combo,
      "APG's own select-only combobox example: Escape must leave/set focus " +
        "on the combobox",
    ).toBeFocused();
  });
});
