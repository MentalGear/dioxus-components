/**
 * RadarChart -- shadcn parity gallery (14 variants:
 * $S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-*.tsx), built
 * on the shared `chart` primitive/engine ($S/chart-api.md,
 * primitives/src/chart/engine/radar.rs, primitives/src/chart/components/
 * series/radar.rs). Render + axe per variant, plus one behavioural
 * assertion per feature this lane added:
 *
 *   - one `path[data-slot="chart-radar-area"][data-series]` per series,
 *     each a closed polygon with exactly N vertices (one per category --
 *     parsed from its own `d`, not assumed);
 *   - the grid variants render the DOM shape their name promises (a
 *     `circle` ring vs a polygon `path` ring, or no grid group at all);
 *   - `lines_only` renders its series paths with zero fill opacity;
 *   - `dots` renders one `circle[data-slot="chart-dot"]` per category per
 *     series;
 *   - the hidden data table always mirrors categories x series, regardless
 *     of what the visual grid/dot/fill options are set to.
 *
 * Every variant renders inline on one page load (`?name=radar_chart&`,
 * confirmed by every other multi-variant component's own oracle spec,
 * e.g. `playwright/oracle/tier3-radix/chart.spec.ts`'s header comment):
 * the un-suffixed `#component-preview-frame` is the `main` (shadcn
 * "default") variant; every other variant gets
 * `#component-preview-frame-<variant>` (`preview/src/main.rs`'s own
 * frame-id-per-variant naming, read in full this session).
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=radar_chart&`;

// Every registered variant, in `preview/src/components/mod.rs`'s own
// `examples!` order ("main" first, matching every other component's demo
// page layout).
const VARIANTS = [
  "main",
  "dots",
  "lines_only",
  "multiple",
  "grid_circle",
  "grid_circle_fill",
  "grid_circle_no_lines",
  "grid_custom",
  "grid_fill",
  "grid_none",
  "icons",
  "label_custom",
  "legend",
  "radius",
] as const;
type Variant = (typeof VARIANTS)[number];

function frame(page: Page, variant: Variant): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`).first();
}

/** Count this radar polygon path's own vertices from its `d` attribute --
 * one `M`/`L` command per vertex, regardless of the exact coordinates
 * `radar_polygon_path` formatted them with. */
function vertexCount(d: string): number {
  const withoutClose = d.replace(/\s*Z\s*$/, "");
  const commands = withoutClose.split(/(?=[ML])/).filter((s) => s.trim().length > 0);
  return commands.length;
}

test.describe("render + axe per variant", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: renders an accessible radar chart`, async ({ page }) => {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      const svg = frame(page, variant).locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      // Every series renders its own closed polygon.
      const paths = frame(page, variant).locator('path[data-slot="chart-radar-area"]');
      await expect(paths).not.toHaveCount(0);
    });

    test(`${variant}: has no automatically detectable a11y issues`, async ({ page }) => {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      await expect(frame(page, variant).locator('svg[role="img"]').first()).toBeVisible();
      await expectNoAxeViolations(page, `radar_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});

test.describe("behavioural", () => {
  test("main: one radar path per series, each with 6 vertices (one per month)", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const paths = frame(page, "main").locator('path[data-slot="chart-radar-area"][data-series]');
    await expect(paths).toHaveCount(1); // shadcn's default demo: one "desktop" series

    const d = await paths.first().getAttribute("d");
    expect(d, "expected a non-empty path `d`").toBeTruthy();
    expect(vertexCount(d ?? "")).toBe(6); // January..June
  });

  test("main: hovering a category's hit-sector opens the tooltip with that category's content", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const tooltip = frame(page, "main").locator('[data-slot="chart-tooltip"]');
    await expect(tooltip).toHaveAttribute("data-state", "closed");

    // One hit-sector per category (6 months) -- hover the first.
    const sectors = frame(page, "main").locator('[data-slot="chart-hit-band"]');
    await expect(sectors).toHaveCount(6);
    await sectors.first().hover();
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip).toContainText("January");

    // Moving off the chart closes it again -- same mechanism as the
    // Cartesian families' own hit-bands (chart.spec.ts).
    await page.mouse.move(0, 0);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });

  test("multiple: two series, each a 6-vertex polygon", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const paths = frame(page, "multiple").locator('path[data-slot="chart-radar-area"][data-series]');
    await expect(paths).toHaveCount(2);
    for (const p of await paths.all()) {
      const d = await p.getAttribute("d");
      expect(vertexCount(d ?? "")).toBe(6);
    }
    // Distinct series -- distinct `data-series` values.
    const keys = await paths.evaluateAll((els) => els.map((el) => el.getAttribute("data-series")));
    expect(new Set(keys).size).toBe(2);
  });

  test("grid_circle: grid rings are <circle> elements", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const grid = frame(page, "grid_circle").locator('g[data-slot="chart-grid"]');
    await expect(grid).toHaveCount(1);
    const circles = grid.locator("circle");
    await expect(circles).not.toHaveCount(0);
    // A circle grid never renders a polygon-ring path.
    await expect(grid.locator("path")).toHaveCount(0);
  });

  test("grid_circle_no_lines: circle rings, no spokes", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const grid = frame(page, "grid_circle_no_lines").locator('g[data-slot="chart-grid"]');
    await expect(grid.locator("circle")).not.toHaveCount(0);
    await expect(grid.locator('[data-slot="chart-grid-spoke"]')).toHaveCount(0);
  });

  test("main (polygon grid, default): grid rings are polygon <path> elements, with spokes", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const grid = frame(page, "main").locator('g[data-slot="chart-grid"]');
    await expect(grid).toHaveCount(1);
    await expect(grid.locator("path")).not.toHaveCount(0);
    await expect(grid.locator("circle")).toHaveCount(0);
    await expect(grid.locator('[data-slot="chart-grid-spoke"]')).not.toHaveCount(0);
  });

  test("grid_none: no grid group at all", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    await expect(frame(page, "grid_none").locator('g[data-slot="chart-grid"]')).toHaveCount(0);
  });

  test("lines_only: series paths render with zero fill opacity", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const paths = frame(page, "lines_only").locator('path[data-slot="chart-radar-area"]');
    await expect(paths).not.toHaveCount(0);
    for (const p of await paths.all()) {
      const fillOpacity = await p.evaluate((el) => getComputedStyle(el).fillOpacity);
      expect(Number(fillOpacity)).toBe(0);
    }
  });

  test("dots: one dot per category per series", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const seriesPaths = frame(page, "dots").locator('path[data-slot="chart-radar-area"][data-series]');
    const seriesCount = await seriesPaths.count();
    expect(seriesCount).toBeGreaterThan(0);

    const dots = frame(page, "dots").locator('circle[data-slot="chart-dot"]');
    await expect(dots).toHaveCount(6 * seriesCount); // 6 months
  });

  for (const variant of ["main", "multiple", "dots"] as const) {
    test(`${variant}: hidden table mirrors categories x series`, async ({ page }) => {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      const table = frame(page, variant).locator('table[data-slot="chart-data"]');
      await expect(table).toHaveCount(1);

      const seriesCount = await frame(page, variant)
        .locator('path[data-slot="chart-radar-area"][data-series]')
        .count();
      const rows = table.locator("tbody tr");
      await expect(rows).toHaveCount(6); // 6 months

      const firstRowCells = rows.first().locator("td");
      await expect(firstRowCells).toHaveCount(seriesCount);
    });
  }
});
