/**
 * `bar_chart` gallery -- shadcn/ui `chart-bar-*` demo parity
 * (dev-docs/research/chart-2026-09-19.md §1.2, $S/stage2-common.md).
 *
 * Every registered variant of a "Normal"-kind component (this one included)
 * renders on the SAME `/component/?name=bar_chart&` page load, all at once
 * -- `?variant=` has no effect on this route (confirmed live for `chart`
 * itself in `playwright/oracle/tier3-radix/chart.spec.ts`'s own header
 * comment, and re-confirmed for this route while writing this file). The
 * `main` variant's demo panel is the un-suffixed `#component-preview-frame`;
 * every other registered variant gets `#component-preview-frame-<variant>`
 * instead.
 *
 * PHASE 1 (this file, as first committed): `main` (shadcn's
 * `chart-bar-default.tsx`), `multiple`, `stacked`, `stacked_legend`,
 * `interactive` -- the variants buildable with zero primitive changes
 * beyond what the stage-1 Chart round already shipped. PHASE 2 (a follow-up
 * commit, once `s2-refactor` lands and this lane's own `BarOptions` feature
 * work is in): `horizontal`, `negative`, `mixed`, `label`, `label_custom`,
 * `active` are added here alongside their own variant folders, per
 * `$S/stage2-common.md`'s "no new feature first, feature variants in
 * follow-up commits" sequencing for this lane.
 */
import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=bar_chart&`;

function frame(page: Page, variant?: string): Locator {
  const id = variant ? `#component-preview-frame-${variant}` : "#component-preview-frame";
  return page.locator(id).first();
}

async function goto(page: Page) {
  await page.goto(URL, { timeout: 20 * 60 * 1000 });
  await page.waitForLoadState("networkidle");
  // Every variant's svg is present as soon as the page has loaded (Normal
  // components render every variant unconditionally) -- wait on the `main`
  // one specifically as the "the page is actually ready" signal, matching
  // `oracle/tier3-radix/chart.spec.ts`'s own `goto` helper.
  await frame(page).locator('svg[role="img"]').first().waitFor({ state: "attached" });
}

test.describe("main variant (shadcn chart-bar-default.tsx)", () => {
  test("renders one accessible bar per month with a hideLabel tooltip", async ({ page }) => {
    await goto(page);
    const f = frame(page);
    const svg = f.locator('svg[role="img"]').first();
    await expect(svg).toBeVisible();
    await expect(svg).toHaveAccessibleName(/.+/);

    // 6 months, one series -- 6 bars.
    await expect(f.locator('[data-slot="chart-bar"]')).toHaveCount(6);

    // hideLabel: true -- the tooltip never shows a label row.
    await f.locator('[data-slot="chart-hit-band"]').first().hover();
    const tooltip = f.locator('[data-slot="chart-tooltip"]');
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveCount(0);
    await page.mouse.move(0, 0);
  });
});

test.describe("multiple variant (shadcn chart-bar-multiple.tsx)", () => {
  test("renders two grouped (non-stacked) series per month", async ({ page }) => {
    await goto(page);
    const f = frame(page, "multiple");
    // 6 months * 2 series = 12 bars, grouped side by side -- distinct from
    // `stacked`'s same 12-bar count by each bar's own width (see below).
    const bars = f.locator('[data-slot="chart-bar"]');
    await expect(bars).toHaveCount(12);

    // DOM order is series-major (every "desktop" bar, then every "mobile"
    // bar -- `Chart` renders one `SeriesMarks` group per config series, in
    // order), so January's desktop bar is index 0 and January's mobile bar
    // is index 6 (6 months per series), not indices 0/1. Grouped bars for
    // the SAME category sit side by side (narrower each, non-overlapping x
    // ranges) -- a stacked pair instead would share the same x/width and
    // only differ in y (see the `stacked` variant's own assertion below).
    const januaryDesktop = await bars.nth(0).boundingBox();
    const januaryMobile = await bars.nth(6).boundingBox();
    expect(januaryDesktop).not.toBeNull();
    expect(januaryMobile).not.toBeNull();
    if (januaryDesktop && januaryMobile) {
      expect(Math.abs(januaryDesktop.x - januaryMobile.x)).toBeGreaterThan(1); // side by side, not stacked
      expect(januaryDesktop.x + januaryDesktop.width).toBeLessThanOrEqual(januaryMobile.x + 1); // no overlap
    }
  });
});

test.describe("stacked variant (bare, no legend)", () => {
  test("renders stacked bars for two series with no legend", async ({ page }) => {
    await goto(page);
    const f = frame(page, "stacked");
    await expect(f.locator('[data-slot="chart-bar"]')).toHaveCount(12);
    await expect(f.locator('[data-slot="chart-legend"]')).toHaveCount(0);

    // Stacked (unlike grouped): the two series' bars for the same category
    // share the same x/width and sit at different y. DOM order is
    // series-major (see the `multiple` variant's own comment above), so
    // January's desktop bar is index 0 and January's mobile bar is index 6.
    const bars = f.locator('[data-slot="chart-bar"]');
    const januaryDesktop = await bars.nth(0).boundingBox();
    const januaryMobile = await bars.nth(6).boundingBox();
    expect(januaryDesktop).not.toBeNull();
    expect(januaryMobile).not.toBeNull();
    if (januaryDesktop && januaryMobile) {
      expect(Math.abs(januaryDesktop.x - januaryMobile.x)).toBeLessThan(1);
      expect(Math.abs(januaryDesktop.width - januaryMobile.width)).toBeLessThan(1);
      expect(Math.abs(januaryDesktop.y - januaryMobile.y)).toBeGreaterThan(1); // different y (stacked)
    }
  });
});

test.describe("stacked_legend variant (shadcn chart-bar-stacked.tsx)", () => {
  test("renders stacked bars for two series with a legend", async ({ page }) => {
    await goto(page);
    const f = frame(page, "stacked_legend");
    await expect(f.locator('[data-slot="chart-bar"]')).toHaveCount(12);
    await expect(f.locator('[data-slot="chart-legend-item"]')).toHaveCount(2);
  });
});

test.describe("interactive variant (shadcn chart-bar-interactive.tsx)", () => {
  test("toggling the header buttons swaps which series draws", async ({ page }) => {
    await goto(page);
    const f = frame(page, "interactive");
    const toggles = f.locator(".dx-bar-chart-interactive-toggle");
    await expect(toggles).toHaveCount(2);

    // Desktop is active by default.
    await expect(toggles.nth(0)).toHaveAttribute("data-active", "true");
    await expect(toggles.nth(1)).toHaveAttribute("data-active", "false");
    // Both totals are real, non-empty numbers regardless of which is active.
    const totals = f.locator(".dx-bar-chart-interactive-toggle-total");
    await expect(totals.nth(0)).toHaveText(/\d/);
    await expect(totals.nth(1)).toHaveText(/\d/);

    const bars = f.locator('[data-slot="chart-bar"]');
    const barsBefore = await bars.count();
    expect(barsBefore).toBeGreaterThan(0);

    await toggles.nth(1).click();
    await expect(toggles.nth(1)).toHaveAttribute("data-active", "true");
    await expect(toggles.nth(0)).toHaveAttribute("data-active", "false");
    // Still exactly one bar per category either way (a single-series
    // config), so the count itself is unchanged even though the drawn
    // values are now the other series'.
    await expect(bars).toHaveCount(barsBefore);
  });
});

test.describe("Axe automated scan", () => {
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page);
    await expectNoAxeViolations(page, "bar_chart: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("interactive variant toggled to mobile has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page);
    await frame(page, "interactive").locator(".dx-bar-chart-interactive-toggle").nth(1).click();
    await expectNoAxeViolations(page, "bar_chart: interactive toggled", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
