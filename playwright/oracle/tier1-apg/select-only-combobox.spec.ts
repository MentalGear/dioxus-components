/**
 * ORACLE: tier 1 (APG) -- Select trigger's combobox role, calibrated against
 * W3C's own reference implementation (docs/backlog.md row 8).
 *
 * Source pattern: W3C ARIA Authoring Practices Guide, Combobox pattern,
 * "Select-Only Combobox Example"
 * https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/
 * Prose source (quoted below): the pinned checkout of `w3c/aria-practices`
 * at commit 7e4034b262bc0d25332e330d8a582aaf34113829 --
 * `content/patterns/combobox/combobox-pattern.html`. The vendored,
 * executable example page this file calibrates against (per
 * conformance-harness.md's "Calibration" table, "Vendor the page") is
 * `playwright/oracle/reference/7e4034b/content/patterns/combobox/examples/combobox-select-only.html`,
 * the same file `focus-restore-reference.spec.ts` already vendors and
 * navigates over `file://`.
 *
 * Two-subject shape, per tier1-apg/README.md's "Calibration" section and
 * following `focus-restore-reference.spec.ts`'s own construction: every rule
 * that has a reference-subject equivalent runs against BOTH the vendored
 * example page and this library's `Select`. If the reference page itself
 * fails a rule, the rule is wrong, not the component.
 *
 * Rules (quoted verbatim from the sources above):
 *
 *   R1 -- combobox-pattern.html, "WAI-ARIA Roles, States, and Properties":
 *     "The element that serves as an input and displays the combobox value
 *     has role combobox."
 *     "The combobox element has aria-controls set to a value that refers to
 *     the element that serves as the popup. Note that aria-controls only
 *     needs to be set when the popup is visible. However, it is valid to
 *     reference an element that is not visible."
 *     "... Note that elements with role combobox have an implicit
 *     aria-haspopup value of listbox."
 *     "When the combobox popup is not visible, the element with role
 *     combobox has aria-expanded set to false. When the popup element is
 *     visible, aria-expanded is set to true."
 *   Checked against BOTH subjects: the vendored example's `#combo1`/
 *   `#listbox1`, and this library's `Select` trigger/listbox.
 *
 *   R2 -- combobox-pattern.html, "About This Pattern":
 *     "Comboboxes and listboxes can be marked as required with
 *     aria-required="true", and they have an accessible name that is
 *     distinct from their value."
 *   Single-subject (this library only): the vendored select-only example
 *   ("Favorite Fruit") has no required-field variant -- confirmed by reading
 *   both its static markup and `examples/js/select-only.js` directly (no
 *   `aria-required` in either) -- so there is nothing to calibrate this rule
 *   against on the reference side. Subject: the `form` conformance fixture
 *   route (`/component/?name=form&`, `preview/src/components/form/
 *   component.rs`), which has a "Fruit, required (library)" `Select`.
 *
 *   R3 -- axe-core, `aria-allowed-attr`/`aria-allowed-role`/
 *   `aria-required-attr` (via `playwright/axe.ts`'s `expectNoAxeViolations`,
 *   which runs the full wcag2a/wcag2aa/wcag21a/wcag21aa/best-practice rule
 *   set, a superset of these three): role="combobox" on a <button> element,
 *   plus aria-haspopup/aria-expanded/aria-controls/aria-autocomplete/
 *   aria-required on a combobox-role element, must all still resolve to a
 *   conforming ARIA-in-HTML combination. Single-subject (this library
 *   only) -- axe is a static DOM scan, not a behavioural comparison, so
 *   there is no meaningful "run axe against the reference page" analogue
 *   here the way R1's behavioural check has one. Scanned at both closed and
 *   open, per axe.ts's own "two-state convention". This duplicates (does
 *   not replace) `select.spec.ts`'s pre-existing "Axe automated scan"
 *   describe block, which already scans this same route/states and now
 *   benefits from the same role change; kept here too since this file's own
 *   mandate is to hold role-conformance rules R1-R3 as a self-contained set
 *   next to the sources they're drawn from.
 *
 * NOT covered by role="combobox"/aria-autocomplete="none": this library
 * deliberately keeps its existing real-roving-DOM-focus model rather than
 * adopting the reference example's `aria-activedescendant` technique (task
 * brief for this row: "keep this repo's existing focus model, do NOT switch
 * to aria-activedescendant") -- see keyboard-matrix.spec.ts's Select
 * section for the (unchanged by this row) behavioural rows that already
 * document that divergence. `aria-autocomplete="none"` itself is NOT part
 * of R1's dual-subject check: the vendored reference page does not set it
 * anywhere in its static markup or `select-only.js` (confirmed by reading
 * both) even though combobox-pattern.html's prose says every combobox
 * "has aria-autocomplete set to a value that corresponds to its autocomplete
 * behavior" -- this library sets it to match Radix's trigger regardless
 * (see `primitives/src/select/components/trigger.rs`'s own comment), but
 * that is a Radix-parity choice, not something asserted here as an
 * APG-sourced, dual-subject-calibrated rule.
 */

import { test, expect, type Page } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "../../axe";
import { BASE_URL } from "../../base-url";

const REFERENCE_ROOT = path.resolve(__dirname, "../reference/7e4034b/content/patterns");
const comboboxSelectOnlyUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "combobox/examples/combobox-select-only.html"),
).href;

const BASE = `${BASE_URL}/component/?name=`;
const goto = (page: Page, name: string) =>
  page.goto(`${BASE}${name}&`, { waitUntil: "networkidle", timeout: 20 * 60 * 1000 });

// Same role/locator convention as select.spec.ts's `singleSelectTrigger` --
// docs/backlog.md row 8 gave the trigger role="combobox".
const librarySelectTrigger = (page: Page) =>
  page.getByRole("combobox").filter({ hasText: /Select an option|Apple|Banana/ });

test.describe("R1 -- combobox role/aria-haspopup/aria-expanded/aria-controls contract", () => {
  test("CALIBRATION: W3C's own select-only combobox example carries the role contract", async ({ page }) => {
    await page.goto(comboboxSelectOnlyUrl);
    const combo = page.locator("#combo1");
    const listbox = page.locator("#listbox1");

    await expect(combo).toHaveAttribute("role", "combobox");
    await expect(combo).toHaveAttribute("aria-haspopup", "listbox");
    await expect(combo).toHaveAttribute("aria-expanded", "false");
    await expect(listbox).toHaveAttribute("role", "listbox");

    const controls = await combo.getAttribute("aria-controls");
    const listboxId = await listbox.getAttribute("id");
    expect(
      controls,
      "aria-controls must reference the listbox's own id",
    ).toBe(listboxId);

    await combo.click();
    await expect(combo).toHaveAttribute("aria-expanded", "true");
  });

  test("Select trigger carries the same role contract", async ({ page }) => {
    await goto(page, "select");
    const trigger = librarySelectTrigger(page);

    await expect(trigger).toHaveAttribute("role", "combobox");
    await expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
    await expect(trigger).toHaveAttribute("aria-expanded", "false");

    // "aria-controls only needs to be set when the popup is visible.
    // However, it is valid to reference an element that is not visible" --
    // so this must already be non-empty while closed, even though
    // `SelectList` only mounts the actual listbox element once open.
    const controls = await trigger.getAttribute("aria-controls");
    expect(
      controls,
      "trigger must carry a non-empty aria-controls before the listbox ever opens",
    ).toBeTruthy();

    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    const listbox = page.getByRole("listbox");
    await expect(listbox).toBeVisible();
    const listboxId = await listbox.getAttribute("id");
    expect(
      controls,
      "the aria-controls value captured while closed must reference the now-rendered listbox's own id",
    ).toBe(listboxId);
  });
});

test.describe("R2 -- a required Select exposes aria-required on the combobox trigger", () => {
  test("a required Select exposes aria-required=\"true\"", async ({ page }) => {
    await goto(page, "form");
    const trigger = page.getByRole("combobox", { name: "Fruit, required (library)" });
    await expect(trigger).toHaveAttribute("aria-required", "true");
  });

  test("a non-required Select exposes aria-required=\"false\"", async ({ page }) => {
    await goto(page, "form");
    const trigger = page.getByRole("combobox", { name: "Fruit (library)" });
    await expect(trigger).toHaveAttribute("aria-required", "false");
  });

  test.skip(
    "CALIBRATION: no vendored reference to calibrate against -- the select-only combobox example (\"Favorite Fruit\") has no required-field variant (confirmed by reading combobox-select-only.html and select-only.js directly; aria-required appears in neither)",
    () => {},
  );
});

test.describe("R3 -- axe: role=combobox trigger stays a11y-clean closed and open", () => {
  test("closed: no automatically detectable a11y issues (aria-allowed-attr/aria-allowed-role/aria-required-attr and the rest of the tag set)", async ({ page }) => {
    await goto(page, "select");
    await expectNoAxeViolations(page, "select-only-combobox: closed", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });

  test("open: no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "select");
    await librarySelectTrigger(page).click();
    await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "select-only-combobox: open", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
