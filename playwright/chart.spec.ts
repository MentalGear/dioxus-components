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

// Regression: user report "the examples for the chart demo page display the
// labels over the legend" (2026-09-19). Root cause, confirmed by measuring
// real rendered boxes (not just reading the CSS): `ChartTooltip` positions
// itself with `left`/`top` PERCENTAGES of `Chart`'s own logical width/height
// -- i.e. of the svg's own box -- but before the fix, `.dx-chart` (a flex
// column holding the svg wrapper, the tooltip, AND the legend as siblings)
// was the tooltip's containing block, so those percentages actually
// resolved against the COMBINED svg+legend box instead. With
// `ChartLegend`'s `vertical_align: Top` (the `bar` demo variant, the ONLY
// one affected in practice -- the default/bottom alignment's tooltip only
// ever extends UPWARD, away from a legend below it, so it was never at
// risk), this placed the tooltip's anchor -- and often its entire body --
// inside the legend's own row. Confirmed live: hovering the `bar` demo's
// 2nd bar rendered the tooltip directly over the legend, fully obscuring
// "Desktop" (`$S/chart-legend-overlap/before-bar-crop-light.png`).
//
// Two things were true and are both asserted below, independently:
//  1. The STATIC x-axis label group and the plot svg's own box never
//     overlapped the legend to begin with, on unmodified `main`, at any
//     viewport this lane tested (measured, not assumed -- the initial bug
//     report's own best guess at the mechanism, which this suite's naming
//     preserves, turned out not to be it). These assertions stay green
//     before and after the fix -- a real invariant worth locking in, not a
//     manufactured regression.
//  2. The DYNAMIC open tooltip DID overlap the legend, for the `bar`
//     variant specifically, before the fix -- red on unmodified `main`
//     ($S/chart-legend-overlap/full-sweep-light.txt: bar chart 0,
//     hit-band 1, overlap w=152 h=16), green after.
test.describe("Legend never overlaps the chart's axis labels, svg box, or open tooltip", () => {
  function intersects(
    a: { x: number; y: number; width: number; height: number } | null,
    b: { x: number; y: number; width: number; height: number } | null,
  ): boolean {
    if (!a || !b) return false;
    return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
  }

  // Every variant that has a legend, and how many independent `.dx-chart`
  // roots its frame renders. `stacked` renders two (area, then bar),
  // sharing one legend each; every other variant renders one. `line` (a
  // single series) has no `ChartLegend` at all -- nothing to check there.
  const VARIANTS: { frame: string; chartCount: number }[] = [
    { frame: "#component-preview-frame", chartCount: 1 },
    { frame: "#component-preview-frame-bar", chartCount: 1 },
    { frame: "#component-preview-frame-stacked", chartCount: 2 },
  ];

  for (const { frame: frameSel, chartCount } of VARIANTS) {
    test(`${frameSel}: x-axis labels and the svg box don't overlap the legend`, async ({ page }) => {
      await page.goto(`${BASE_URL}/component/?name=chart&`);
      const charts = page.locator(frameSel).first().locator(".dx-chart");
      await expect(charts).toHaveCount(chartCount);
      for (let i = 0; i < chartCount; i++) {
        const chart = charts.nth(i);
        const legendBox = await chart.locator('[data-slot="chart-legend"]').first().boundingBox();
        const axisBox = await chart
          .locator('[data-slot="chart-axis"][data-axis="x"]')
          .first()
          .boundingBox();
        const svgBox = await chart.locator('svg[role="img"]').first().boundingBox();
        expect(intersects(axisBox, legendBox), `chart ${i}: x-axis labels vs legend`).toBe(false);
        expect(intersects(svgBox, legendBox), `chart ${i}: svg box vs legend`).toBe(false);
      }
    });

    test(`${frameSel}: the open tooltip doesn't overlap the legend, for every data point`, async ({ page }) => {
      await page.goto(`${BASE_URL}/component/?name=chart&`);
      const charts = page.locator(frameSel).first().locator(".dx-chart");
      for (let i = 0; i < chartCount; i++) {
        const chart = charts.nth(i);
        const legend = chart.locator('[data-slot="chart-legend"]').first();
        const tooltip = chart.locator('[data-slot="chart-tooltip"]').first();
        const hitBands = chart.locator('[data-slot="chart-hit-band"]');
        const count = await hitBands.count();
        // Every hit-band for the small, fixed demos (<=6 here); a stride
        // for `main`'s 90-point dataset keeps this fast while still
        // covering its full value range (always including index 0).
        const step = Math.max(1, Math.floor(count / 10));
        for (let b = 0; b < count; b += step) {
          await hitBands.nth(b).hover();
          await expect(tooltip).toHaveAttribute("data-state", "open");
          // `legendBox` is re-measured on EVERY iteration, right alongside
          // `tooltipBox` -- not hoisted above the loop. `boundingBox()`
          // returns VIEWPORT-relative coordinates, and `.hover()` force-
          // scrolls its target into view as part of Playwright's own
          // actionability protocol while a bare `boundingBox()` does not;
          // hoisting `legendBox` above the loop measured it against the
          // page's initial (pre-scroll) position while `tooltipBox` was
          // measured after `.hover()` had already scrolled the page --
          // two boxes in two different scroll states are not comparable,
          // and this produced a real, confirmed false pass in this exact
          // test (`$S/chart-legend-overlap/spec-before-fix-buggy-test.log`)
          // even against the unfixed CSS.
          const legendBox = await legend.boundingBox();
          const tooltipBox = await tooltip.boundingBox();
          expect(intersects(tooltipBox, legendBox), `chart ${i}, hit-band ${b}: tooltip vs legend`).toBe(
            false,
          );
        }
        await page.mouse.move(0, 0);
      }
    });
  }
});
