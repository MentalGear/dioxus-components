import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=avatar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  // The demo avatars are bundled geometric SVGs (preview/assets/avatars), not
  // hot-linked photos: the src is the hashed local asset and it must actually load.
  const image = page.getByRole("img", { name: "Avery Lin" }).first();
  await expect(image).toHaveAttribute("src", /\/avery-lin[^/]*\.svg$/);
  await expect(page.getByLabel("Basic avatar")).toHaveAttribute("data-state", "loaded");

  // A broken image falls back to the initials.
  await expect(page.getByLabel("Error avatar").getByText("JR")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Avatar has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=avatar&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "avatar: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
