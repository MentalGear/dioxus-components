import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=chart&`);
  const frame = page.locator("#component-preview-frame").first();

  // the chart renders as a single accessible image. An attribute selector,
  // not getByRole("img") -- this page's Select trigger also renders a bare
  // decorative chevron <svg> with no explicit role, which Chromium's own
  // accessibility tree still computes an implicit "img" role for, so
  // getByRole("img").first() picks up that unrelated icon instead (found by
  // running this spec for real: it resolved to
  // `svg.dx-select-expand-icon`, not the chart).
  const svg = frame.locator('svg[role="img"]').first();
  await expect(svg).toBeVisible();
  await expect(svg).toHaveAccessibleName(/.+/);

  // hovering a data point's hit-band shows the tooltip
  const tooltip = frame.locator('[data-slot="chart-tooltip"]');
  await frame.locator('[data-slot="chart-hit-band"]').first().hover();
  await expect(tooltip).toHaveAttribute("data-state", "open");
  await expect(tooltip).toBeVisible();

  // moving the mouse off the chart hides it again
  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveAttribute("data-state", "closed");
});

test.describe("Axe automated scan", () => {
  test("loaded (tooltip closed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=chart&`);
    await expectNoAxeViolations(page, "chart: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tooltip open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=chart&`);
    const frame = page.locator("#component-preview-frame").first();
    await frame.locator('[data-slot="chart-hit-band"]').first().hover();
    await expect(frame.locator('[data-slot="chart-tooltip"]')).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "chart: tooltip open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
