import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=empty&", { timeout: 20 * 60 * 1000 });
  // Scope to the demo frame. The component page also renders this component's
  // own source in a syntax-highlighted code viewer, so every visible string
  // here appears a second time as a highlighted token span.
  const demo = page.locator("#component-preview-frame").first();
  await expect(demo.getByText("No messages yet")).toBeVisible();
  await expect(demo.getByText("You don't have any messages.")).toBeVisible();
  await expect(demo.getByRole("button", { name: "New message" })).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Empty has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=empty&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "empty: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
