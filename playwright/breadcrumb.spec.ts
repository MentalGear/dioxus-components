import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=breadcrumb&`, { timeout: 20 * 60 * 1000 });
  const nav = page.getByRole("navigation", { name: "breadcrumb" });
  await expect(nav).toBeVisible();
  await expect(nav.getByRole("link", { name: "Home" })).toBeVisible();
  await expect(nav.getByRole("link", { name: "Components" })).toBeVisible();
  // The current page is not a link -- it is marked aria-current="page".
  const current = nav.getByText("Breadcrumb", { exact: true });
  await expect(current).toHaveAttribute("aria-current", "page");
});

test.describe("Axe automated scan", () => {
  // Breadcrumb has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=breadcrumb&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "breadcrumb: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
