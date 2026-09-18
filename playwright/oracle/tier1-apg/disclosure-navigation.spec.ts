/**
 * ORACLE: tier 1 (APG) -- Disclosure (Show/Hide) Navigation rules,
 * calibrated against W3C's own reference implementation.
 *
 * Source pattern sections, quoted verbatim from the pinned checkout of
 * `w3c/aria-practices` (commit `7e4034b262bc0d25332e330d8a582aaf34113829`,
 * see `playwright/oracle/reference/README.md`):
 *
 *   (R1) "The element that shows and hides the content has role button
 *   ... When the content is visible, the element with role button has
 *   aria-expanded set to true. When the content area is hidden, it is set
 *   to false. Optionally, the element with role button has a value
 *   specified for aria-controls that refers to the element that contains
 *   all the content that is shown or hidden."
 *   -- content/patterns/disclosure/disclosure-pattern.html,
 *      "WAI-ARIA Roles, States, and Properties"
 *
 *   (R2) "Enter: activates the disclosure control and toggles the
 *   visibility of the disclosure content. Space: activates the disclosure
 *   control and toggles the visibility of the disclosure content."
 *   -- content/patterns/disclosure/disclosure-pattern.html,
 *      "Keyboard Interaction"
 *
 *   (R3) "If a dropdown is open ... pressing Esc will close the dropdown."
 *   / "Escape: If a dropdown is open, closes it and sets focus on the
 *   button that controls that dropdown."
 *   -- content/patterns/disclosure/examples/disclosure-navigation.html,
 *      "Accessibility Features" and "Keyboard Support"
 *
 *   (R4) "Although this example uses the word 'menu' in the colloquial
 *   sense to refer to a set of navigation links, it does not use the
 *   WAI-ARIA menu role. This implementation of site navigation does not
 *   use the menu role because it does not provide the complex
 *   functionality that assistive technologies expect in a widget that has
 *   the menu role."
 *   -- content/patterns/disclosure/examples/disclosure-navigation.html,
 *      "About This Example" (the pattern's own contrast with the Menu and
 *      Menubar pattern `primitives/src/navbar.rs` implements instead)
 *
 *   (R5) "Tab / Shift+Tab: Move keyboard focus among top-level buttons,
 *   and if a dropdown is open, into and through links in the dropdown."
 *   -- content/patterns/disclosure/examples/disclosure-navigation.html,
 *      "Keyboard Support"
 *
 * Calibration shape (docs/conformance-harness.md, "Calibration";
 * mirrors `oracle/tier1-apg/focus-restore-reference.spec.ts`'s two-subject
 * layout): every rule below runs once against the pattern's own vendored
 * example page (`playwright/oracle/reference/7e4034b/`, loaded over
 * `file://` so it needs no network and cannot drift) and once against this
 * library's own `navigation_menu` primitive
 * (`http://127.0.0.1:8080/component/?name=navigation_menu&`). If the
 * REFERENCE subject ever goes red, the rule itself is wrong (or
 * w3c/aria-practices's own example regressed at the pinned commit) -- not
 * this library. If only the LIBRARY subject goes red, `navigation_menu`
 * regressed.
 *
 * See `primitives/src/navigation_menu.rs`'s module doc for why this is a
 * distinct primitive from `crate::navbar::Navbar` (a different,
 * APG-distinguished pattern -- Menu and Menubar, not Disclosure
 * Navigation), not the row-53-class duplication `crate::menu_root`'s doc
 * describes for `DropdownMenu`/`ContextMenu`/`Menubar`.
 */

import { test, expect, type Page } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";

const REFERENCE_ROOT = path.resolve(__dirname, "../reference/7e4034b/content/patterns");
const referenceUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "disclosure/examples/disclosure-navigation.html"),
).href;

const LIBRARY_URL = "http://127.0.0.1:8080/component/?name=navigation_menu&";
const LIBRARY_GOTO = { timeout: 20 * 60 * 1000 };

/**
 * The Library subject's own `NavigationMenu`, scoped by its `aria_label`
 * (`preview/src/components/navigation_menu/variants/main/mod.rs`) rather
 * than querying the whole page. The docs site's own persistent chrome
 * (header navbar, footer) renders its own "Docs" links on every route, so
 * an unscoped `page.getByRole('link', { name: 'Docs' })` resolves to more
 * than one element (confirmed live: 3 page-wide vs. 1 once scoped here).
 */
function libraryNav(page: Page) {
  return page.getByRole("navigation", { name: "Component navigation menu" });
}

test.describe("Reference: W3C's own Disclosure Navigation example", () => {
  test("R1: each trigger is a button with aria-expanded reflecting state and aria-controls naming the panel", async ({ page }) => {
    await page.goto(referenceUrl);

    const trigger = page.getByRole("button", { name: "About" });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    const controls = await trigger.getAttribute("aria-controls");
    expect(controls).toBe("id_about_menu");
    await expect(page.locator(`#${controls}`)).toHaveCount(1);

    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
  });

  test("R2: Enter and Space each toggle the trigger", async ({ page }) => {
    await page.goto(referenceUrl);

    const trigger = page.getByRole("button", { name: "About" });
    await trigger.focus();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");

    await page.keyboard.press("Enter");
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    await page.keyboard.press("Space");
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  test("R3: Escape closes the open panel and returns focus to its trigger", async ({ page }) => {
    await page.goto(referenceUrl);

    const trigger = page.getByRole("button", { name: "About" });
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    await page.getByRole("link", { name: "Overview" }).focus();
    await page.keyboard.press("Escape");

    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await expect(trigger).toBeFocused();
  });

  test("R4: no role=menu/menuitem anywhere in the widget", async ({ page }) => {
    await page.goto(referenceUrl);

    const nav = page.getByRole("navigation", { name: "Mythical University" });
    await expect(nav.locator('[role="menu"]')).toHaveCount(0);
    await expect(nav.locator('[role="menuitem"]')).toHaveCount(0);

    await page.getByRole("button", { name: "About" }).click();
    await expect(nav.locator('[role="menu"]')).toHaveCount(0);
    await expect(nav.locator('[role="menuitem"]')).toHaveCount(0);
  });

  test("R5: Tab from an open trigger enters the panel's first link", async ({ page }) => {
    await page.goto(referenceUrl);

    const trigger = page.getByRole("button", { name: "About" });
    await trigger.focus();
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    await page.keyboard.press("Tab");
    await expect(page.getByRole("link", { name: "Overview" })).toBeFocused();
  });
});

test.describe("Library: navigation_menu primitive", () => {
  test("R1: each trigger is a button with aria-expanded reflecting state and aria-controls naming the panel", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);

    const trigger = libraryNav(page).getByRole("button", { name: "Getting started" });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    const controls = await trigger.getAttribute("aria-controls");
    expect(controls).toBeTruthy();

    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await expect(page.locator(`#${controls}`)).toHaveCount(1);
  });

  test("R2: Enter and Space each toggle the trigger", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);

    const trigger = libraryNav(page).getByRole("button", { name: "Getting started" });
    await trigger.focus();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");

    await page.keyboard.press("Enter");
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    await page.keyboard.press("Space");
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  test("R3: Escape closes the open panel and returns focus to its trigger", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);

    const trigger = libraryNav(page).getByRole("button", { name: "Getting started" });
    await trigger.focus();
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    // Move focus into the panel first, as a keyboard user would, per R5.
    await page.keyboard.press("Tab");
    await page.keyboard.press("Escape");

    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await expect(trigger).toBeFocused();
  });

  test("R4: no role=menu/menuitem anywhere in the widget", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);

    const nav = libraryNav(page);
    await expect(nav.locator('[role="menu"]')).toHaveCount(0);
    await expect(nav.locator('[role="menuitem"]')).toHaveCount(0);

    await nav.getByRole("button", { name: "Getting started" }).click();
    // The disclosed panel is promoted to the top layer for *painting* on
    // the web arm (`primitives/src/navigation_menu.rs`'s module doc, "Top
    // layer"), via the native Popover API's `showPopover()` -- this never
    // reparents the element, so it stays a DOM descendant of `nav` (and
    // thus still inside this locator's scope) the whole time.
    await expect(nav.locator('[role="menu"]')).toHaveCount(0);
    await expect(nav.locator('[role="menuitem"]')).toHaveCount(0);
  });

  test("R5: Tab from an open trigger enters the panel's first link", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);

    const trigger = libraryNav(page).getByRole("button", { name: "Getting started" });
    await trigger.focus();
    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    await page.keyboard.press("Tab");
    await expect(libraryNav(page).getByRole("link", { name: "Component Library" })).toBeFocused();
  });
});
