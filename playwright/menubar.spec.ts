import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from "./axe";

test("pointer navigation", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=menubar&", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  const fileMenuButton = page.getByRole("menuitem", { name: "File" });
  await fileMenuButton.click();
  // Assert the menu is open
  const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
  await expect(fileMenuContent).toHaveAttribute("data-state", "open");

  // After the menu is open, hover over the Edit menu item
  const editMenuButton = page.getByRole("menuitem", { name: "Edit" });
  await editMenuButton.hover();
  // Assert the Edit menu content is open
  const editMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "Cut" }) }).last();
  await expect(editMenuContent).toHaveAttribute("data-state", "open");
  // Assert the File menu content is closed
  await expect(fileMenuContent).toHaveCount(0);

  // Click the Cut menu item
  const cutItem = editMenuContent.getByRole("menuitem", { name: "Cut" });
  await cutItem.click();
  // Assert the menu is closed after clicking a menu item
  await expect(fileMenuContent).toHaveCount(0);
});

test("keyboard navigation", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=menubar&", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole("menubar").focus();
  const fileMenuButton = page.getByRole("menuitem", { name: "File" });
  // Go right with the keyboard
  await page.keyboard.press("ArrowRight");
  // Assert the focus is on the Edit menu item
  const editMenuButton = page.getByRole("menuitem", { name: "Edit" });
  await expect(editMenuButton).toBeFocused();
  // Go left with the keyboard
  await page.keyboard.press("ArrowLeft");
  // Assert the focus is on the File menu item
  await expect(fileMenuButton).toBeFocused();
  // Open the File menu
  await page.keyboard.press("ArrowDown");
  // Assert the File menu content is open
  const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
  await expect(fileMenuContent).toHaveAttribute("data-state", "open");

  // assert the new item is focused
  const newItem = fileMenuContent.getByRole("menuitem", { name: "New" });
  await expect(newItem).toBeFocused();
  await expect(fileMenuContent.getByRole("menuitem", { name: "Open" })).toHaveAttribute("data-disabled", "true");
  await page.keyboard.press("ArrowDown");
  await expect(fileMenuContent.getByRole("menuitem", { name: "Save" })).toBeFocused();
  // Click the focused Save menu item
  await page.keyboard.press("Enter");
  // Assert the menu is closed after clicking a menu item
  await expect(fileMenuContent).toHaveCount(0);
});

test.describe("Axe automated scan", () => {
  test("loaded (menus closed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=menubar&", { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole("menuitem", { name: "File" })).toBeVisible();
    await expectNoAxeViolations(page, "menubar: loaded", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  // docs/backlog.md row 25: Menubar's role="menu" popups carry no
  // aria-labelledby/aria-label at all, so an open menu has no accessible
  // name (APG menu-and-menubar pattern requires one).
  test("File menu open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=menubar&", { timeout: 20 * 60 * 1000 });
    await page.getByRole("menuitem", { name: "File" }).click();
    const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
    await expect(fileMenuContent).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "menubar: File menu open", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
