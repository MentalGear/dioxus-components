import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=toggle_group&");
  const b_button = page.getByRole("button", { name: "B", exact: true });
  const i_button = page.getByRole("button", { name: "I", exact: true });
  const u_button = page.getByRole("button", { name: "U", exact: true });

  // The buttons should not be selected initially
  await expect(b_button).toHaveAttribute("data-state", "off");
  await expect(i_button).toHaveAttribute("data-state", "off");
  await expect(u_button).toHaveAttribute("data-state", "off");

  // Click the "B" button and check its state
  await b_button.click();
  await expect(b_button).toHaveAttribute("data-state", "on");

  // Pressing right arrow should select the "I" button
  await page.keyboard.press("ArrowRight");
  await expect(i_button).toBeFocused();

  // Pressing enter should focus the "I" button
  await page.keyboard.press("Enter");
  await expect(i_button).toHaveAttribute("data-state", "on");

  // Pressing right two more times should bring us back to the "B" button
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(b_button).toBeFocused();
});

test.describe("Axe automated scan", () => {
  test("loaded (none selected) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=toggle_group&");
    await expectNoAxeViolations(page, "toggle_group: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("an item selected has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=toggle_group&");
    await page.getByRole("button", { name: "B", exact: true }).click();
    await expectNoAxeViolations(page, "toggle_group: B selected", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
