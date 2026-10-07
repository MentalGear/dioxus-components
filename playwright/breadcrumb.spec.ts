import { test, expect } from "./fixtures";
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

// `BreadcrumbEllipsis` is a `<span>` placed inside a `BreadcrumbItem` (shadcn's shape). It used to render an `<li>`,
// so the demo nested an `<li>` in an `<li>` (backlog row 160). Axe does not flag that nesting, so it is asserted directly.
test("the ellipsis is a span inside its item: no li nested in an li", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=breadcrumb&`, { timeout: 20 * 60 * 1000 });
  const ellipsis = page.locator(".dx-breadcrumb-ellipsis").first();
  await expect(ellipsis).toBeVisible();
  expect(await ellipsis.evaluate((el) => el.tagName)).toBe("SPAN");
  await expect(ellipsis).toHaveAttribute("aria-hidden", "true");
  expect(await ellipsis.evaluate((el) => el.parentElement!.tagName)).toBe("LI");
  expect(await page.locator("#component-preview-frame li li").count()).toBe(0);
});
