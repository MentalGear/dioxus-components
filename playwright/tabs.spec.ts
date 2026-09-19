import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=tabs&`);
  let activeTab = page.locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
    .filter({ hasText: /^Tab \d Content$/ });
  let tab1Button = page.getByRole("tab", { name: "Tab 1" });
  let tab2Button = page.getByRole("tab", { name: "Tab 2" });
  let tab3Button = page.getByRole("tab", { name: "Tab 3" });
  // Clicking the right arrow should focus the next tab trigger
  await tab1Button.click();
  await page.keyboard.press("ArrowRight");
  await expect(tab2Button).toBeFocused();

  // Clicking enter should activate the focused tab
  await page.keyboard.press("Enter");
  await expect(activeTab).toContainText("Tab 2 Content");

  // Clicking right twice more should bring us back to the first tab
  await page.keyboard.press("ArrowRight");
  await expect(tab3Button).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(tab1Button).toBeFocused();

  // Clicking each tab should activate it
  await tab3Button.click();
  await expect(activeTab).toContainText("Tab 3 Content");
  await tab2Button.click();
  await expect(activeTab).toContainText("Tab 2 Content");
  await tab1Button.click();
  await expect(activeTab).toContainText("Tab 1 Content");
});

test.describe("active tab keeps its color under the pointer (calendar-state lane sweep)", () => {
  // Same bug class as calendar/style.css's day hover (see calendar.spec.ts):
  // `.dx-tabs-trigger:hover` used to carry higher specificity than
  // `.dx-tabs-trigger[data-state="active"]`, so clicking a tab -- which
  // leaves the pointer resting on it -- silently dimmed its text to the
  // plain hover tint instead of the active color. Runs in both themes: this
  // one (unlike calendar's) reproduced in light mode too, since
  // --secondary-color-1 and --secondary-color-3 differ in both.
  for (const dark of [false, true]) {
    test(`clicking a tab keeps its active text color while still hovered (${dark ? "dark" : "light"} mode)`, async ({ page }) => {
      await page.goto(`${BASE_URL}/component/?name=tabs&${dark ? "dark_mode=true" : ""}`);
      const tab2Button = page.getByRole("tab", { name: "Tab 2" });
      const tab3Button = page.getByRole("tab", { name: "Tab 3" });

      // The plain hover color, from a tab that stays inactive throughout
      // (Tab 1 is active by default, so it cannot supply an "unselected
      // hover" baseline).
      await tab3Button.hover();
      const plainHoverColor = await tab3Button.evaluate((el) => getComputedStyle(el).color);

      // Click Tab 2 -- Playwright's .click() leaves the pointer on it.
      await tab2Button.click();
      await expect(tab2Button).toHaveAttribute("data-state", "active");
      expect(await tab2Button.evaluate((el) => el.matches(":hover"))).toBe(true);
      const activeAndHoveredColor = await tab2Button.evaluate((el) => getComputedStyle(el).color);

      // Move the pointer off and read the "true" active color.
      await page.locator("body").hover({ position: { x: 0, y: 0 } });
      const activeAtRestColor = await tab2Button.evaluate((el) => getComputedStyle(el).color);

      expect(activeAndHoveredColor).toBe(activeAtRestColor); // active color survives hover
      expect(activeAndHoveredColor).not.toBe(plainHoverColor); // not just the plain hover tint
    });
  }
});

test.describe("Axe automated scan", () => {
  test("loaded (tab 1 active) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=tabs&`);
    await expectNoAxeViolations(page, "tabs: tab 1 active", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tab 2 selected has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=tabs&`);
    await page.getByRole("tab", { name: "Tab 2" }).click();
    // Scoped with the same `.filter(...)` the file's own "test" test uses
    // above -- this page's "Variants" section renders a second, unrelated
    // Tabs instance with its own active tabpanel, so the bare selector
    // resolves to two elements (a Playwright strict-mode violation).
    await expect(
      page
        .locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
        .filter({ hasText: /^Tab \d Content$/ }),
    ).toContainText("Tab 2 Content");
    await expectNoAxeViolations(page, "tabs: tab 2 selected", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
