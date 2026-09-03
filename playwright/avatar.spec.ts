import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=avatar&", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  let image = page.getByRole("img", { name: "User avatar" }).first();
  await expect(image).toHaveAttribute("src", "https://avatars.githubusercontent.com/u/66571940?s=96&v=4");

  await expect(page.getByLabel("Error avatar").getByText("JK")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Avatar has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=avatar&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "avatar: loaded", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
