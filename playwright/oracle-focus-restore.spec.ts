/**
 * ORACLE: focus restoration on close.
 *
 * These tests are a *specification oracle*, not regression tests for current
 * behaviour. Each assertion encodes a rule from the W3C WAI-ARIA Authoring
 * Practices Guide (APG) — the standard this library's README commits to
 * ("It adheres to the WAI-ARIA Authoring Practices for accessibility").
 *
 * A failure here is therefore a conformance gap, not a preference.
 *
 * The rule under test, common to every pattern below:
 *
 *   "Escape: Closes the menu and sets focus to the menu button."
 *   — APG Menu Button pattern, keyboard interaction
 *     https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/
 *
 *   The same return-focus requirement appears in the Menubar pattern and in
 *   the Select-Only Combobox pattern ("Escape ... sets focus on the combobox").
 *     https://www.w3.org/WAI/ARIA/apg/patterns/menubar/
 *     https://www.w3.org/WAI/ARIA/apg/patterns/combobox/
 *
 * Why it matters: these components move real DOM focus onto their items while
 * open (roving tabindex). If nothing returns focus to the trigger on close,
 * the focused node unmounts with the content and focus falls to <body> — a
 * keyboard user must Tab from the top of the document to get back to where
 * they were.
 *
 * Existing coverage note: the current suite asserts `data-state="closed"`
 * after Escape but never asserts *where focus went*, which is why this class
 * of defect is invisible to it today.
 */

import { test, expect, type Page } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const open = (page: Page, name: string) =>
  page.goto(`http://127.0.0.1:8080/component/?name=${name}&`, {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/** Reports where focus actually landed, so a failure names the culprit. */
async function focusReport(page: Page) {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el) return "null";
    return `<${el.tagName.toLowerCase()}${el.id ? ` id="${el.id}"` : ""}${
      el.getAttribute("role") ? ` role="${el.getAttribute("role")}"` : ""
    }> text=${JSON.stringify((el.textContent || "").trim().slice(0, 40))}`;
  });
}

test("DropdownMenu returns focus to its trigger on Escape", async ({ page }) => {
  await open(page, "dropdown_menu");

  const trigger = page.getByRole("button", { name: "Open Menu" });
  await trigger.click();
  await expect(trigger).toHaveAttribute("data-state", "open");

  // Move focus onto an item, as a keyboard user would.
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Escape");
  await expect(trigger).toHaveAttribute("data-state", "closed");

  expect(
    await focusReport(page),
    "APG menu-button: Escape must return focus to the menu button",
  ).toContain("Open Menu");
  await expect(trigger).toBeFocused();
});

test("Select returns focus to its trigger on Escape", async ({ page }) => {
  await open(page, "select");

  const trigger = page
    .getByRole("button")
    .filter({ hasText: /Select an option|Apple|Banana/ });
  await trigger.click();
  await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Escape");

  expect(
    await focusReport(page),
    "APG combobox: Escape must return focus to the combobox/trigger",
  ).not.toBe("<body> text=\"\"");
  await expect(trigger).toBeFocused();
});

test("Menubar returns focus to its menu item on Escape", async ({ page }) => {
  await open(page, "menubar");

  const fileMenu = page.getByRole("menuitem", { name: "File" });
  await fileMenu.click();
  const fileContent = page
    .getByRole("menu")
    .filter({ has: page.getByRole("menuitem", { name: "New" }) })
    .last();
  await expect(fileContent).toHaveAttribute("data-state", "open");

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Escape");

  expect(
    await focusReport(page),
    "APG menubar: Escape must return focus to the parent menubar item",
  ).toContain("File");
  await expect(fileMenu).toBeFocused();
});

test("ContextMenu returns focus to its trigger on Escape", async ({ page }) => {
  await open(page, "context_menu");

  const trigger = page.getByRole("button", { name: "right click here" });
  await trigger.click({ button: "right" });
  await expect(page.getByRole("menu")).toHaveAttribute("data-state", "open");

  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);

  // APG's context-menu guidance follows the menu-button rule: dismissing
  // returns focus to the element that owns the menu.
  expect(
    await focusReport(page),
    "Escape must return focus to the context-menu trigger, not <body>",
  ).toContain("right click here");
  await expect(trigger).toBeFocused();
});

/**
 * CONTROL: Dialog already implements focus restore, via the vendored
 * focus-trap (FocusTrap captures document.activeElement and restores it in
 * remove()). This test should PASS on current main.
 *
 * It exists to prove the oracle itself is sound: if this fails too, the
 * harness is wrong, not the components.
 */
test("CONTROL: Dialog returns focus to its trigger on close", async ({ page }) => {
  await open(page, "dialog");

  const trigger = page.getByRole("button", { name: /Show Dialog|Open Dialog/i }).first();
  await trigger.click();
  await page.keyboard.press("Escape");

  await expect(trigger).toBeFocused();
});
