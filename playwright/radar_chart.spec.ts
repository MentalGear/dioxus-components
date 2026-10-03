/**
 * RadarChart -- shadcn parity gallery (14 variants:
 * $S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-*.tsx), built
 * on the shared `chart` primitive/engine ($S/chart-api.md,
 * primitives/src/chart/engine/radar.rs, primitives/src/chart/components/
 * series/radar.rs). Render + axe per variant, plus one behavioural
 * assertion per feature this lane added:
 *
 *   - one `g[data-series] path[data-slot="chart-radar-area"]` per series
 *     (`data-series` lives on the wrapping `<g>`, not the `<path>` itself --
 *     the same convention `area`/`line` document, e.g.
 *     `primitives/src/chart/components/series/area.rs`'s own module doc),
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

import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
// dev-docs/backlog.md row 109: this suite interacts (hover/click) right
// after navigating, on the SSG lane -- `gotoHydrated` so that can't land
// before hydration attaches listeners.
import { gotoHydrated } from "./hydration";

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
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      const svg = frame(page, variant).locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      // Every series renders its own closed polygon.
      const paths = frame(page, variant).locator('path[data-slot="chart-radar-area"]');
      await expect(paths).not.toHaveCount(0);
    });

    test(`${variant}: has no automatically detectable a11y issues`, async ({ page }) => {
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      await expect(frame(page, variant).locator('svg[role="img"]').first()).toBeVisible();
      await expectNoAxeViolations(page, `radar_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});

test.describe("behavioural", () => {
  test("main: one radar path per series, each with 6 vertices (one per month)", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const paths = frame(page, "main").locator('g[data-series] path[data-slot="chart-radar-area"]');
    await expect(paths).toHaveCount(1); // shadcn's default demo: one "desktop" series

    const d = await paths.first().getAttribute("d");
    expect(d, "expected a non-empty path `d`").toBeTruthy();
    expect(vertexCount(d ?? "")).toBe(6); // January..June
  });

  test("main: hovering a category's hit-sector opens the tooltip with that category's content", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
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
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const paths = frame(page, "multiple").locator('g[data-series] path[data-slot="chart-radar-area"]');
    await expect(paths).toHaveCount(2);
    for (const p of await paths.all()) {
      const d = await p.getAttribute("d");
      expect(vertexCount(d ?? "")).toBe(6);
    }
    // Distinct series -- distinct `data-series` values. `data-series` lives
    // on each path's own parent `<g>` (the selector above, and this file's
    // own header comment), not the `<path>` itself, so read it off the
    // closest ancestor that carries it rather than the path element.
    const keys = await paths.evaluateAll((els) =>
      els.map((el) => el.closest("[data-series]")?.getAttribute("data-series")),
    );
    expect(new Set(keys).size).toBe(2);
  });

  test("grid_circle: grid rings are <circle> elements", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const grid = frame(page, "grid_circle").locator('g[data-slot="chart-grid"]');
    await expect(grid).toHaveCount(1);
    const circles = grid.locator("circle");
    await expect(circles).not.toHaveCount(0);
    // A circle grid never renders a polygon-ring path.
    await expect(grid.locator("path")).toHaveCount(0);
  });

  test("grid_circle_no_lines: circle rings, no spokes", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const grid = frame(page, "grid_circle_no_lines").locator('g[data-slot="chart-grid"]');
    await expect(grid.locator("circle")).not.toHaveCount(0);
    await expect(grid.locator('[data-slot="chart-grid-spoke"]')).toHaveCount(0);
  });

  test("main (polygon grid, default): grid rings are polygon <path> elements, with spokes", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const grid = frame(page, "main").locator('g[data-slot="chart-grid"]');
    await expect(grid).toHaveCount(1);
    await expect(grid.locator("path")).not.toHaveCount(0);
    await expect(grid.locator("circle")).toHaveCount(0);
    await expect(grid.locator('[data-slot="chart-grid-spoke"]')).not.toHaveCount(0);
  });

  test("grid_none: no grid group at all", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    await expect(frame(page, "grid_none").locator('g[data-slot="chart-grid"]')).toHaveCount(0);
  });

  test("lines_only: outlines only (no fill, 2px stroke) over rings without spokes", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "lines_only");
    const paths = f.locator('path[data-slot="chart-radar-area"]');
    await expect(paths).toHaveCount(2);
    for (const p of await paths.all()) {
      const style = await p.evaluate((el) => {
        const cs = getComputedStyle(el);
        return { fillOpacity: cs.fillOpacity, strokeWidth: cs.strokeWidth };
      });
      expect(Number(style.fillOpacity)).toBe(0);
      expect(style.strokeWidth).toBe("2px");
    }
    await expect(f.locator('[data-slot="chart-grid-spoke"]')).toHaveCount(0);
  });

  test("main: Recharts' five radius ticks (0..320), no outline, labels anchored away from the centre", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    const rings = f.locator('[data-slot="chart-grid-ring"]');
    await expect(rings).toHaveCount(5);
    await expect(rings.last()).toHaveAttribute("data-radius", "96");
    const area = f.locator('path[data-slot="chart-radar-area"]');
    expect(await area.evaluate((el) => getComputedStyle(el).stroke)).toBe("none");
    const labels = f.locator('[data-axis="angle"] text');
    await expect(labels.nth(0)).toHaveAttribute("text-anchor", "middle");
    await expect(labels.nth(1)).toHaveAttribute("text-anchor", "start");
    await expect(labels.nth(4)).toHaveAttribute("text-anchor", "end");
  });

  test("multiple: the first series translucent, the second opaque", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const paths = frame(page, "multiple").locator('path[data-slot="chart-radar-area"]');
    await expect(paths.nth(0)).toHaveAttribute("fill-opacity", "0.6");
    await expect(paths.nth(1)).toHaveAttribute("fill-opacity", "1");
  });

  test("radius: the radius axis' tick values along the 60-degree spoke", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const ticks = frame(page, "radius").locator('[data-axis="radius"] text');
    await expect(ticks).toHaveText(["0", "80", "160", "240", "320"]);
  });

  test("label_custom: two-line ticks, values over the month", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const first = frame(page, "label_custom").locator('[data-axis="angle"] text').first();
    await expect(first).toHaveText("186/80January");
  });

  test("dots: one dot per category per series", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const seriesPaths = frame(page, "dots").locator('g[data-series] path[data-slot="chart-radar-area"]');
    const seriesCount = await seriesPaths.count();
    expect(seriesCount).toBeGreaterThan(0);

    const dots = frame(page, "dots").locator('circle[data-slot="chart-dot"]');
    await expect(dots).toHaveCount(6 * seriesCount); // 6 months
    await expect(dots.first()).toHaveAttribute("r", "4");
  });

  for (const variant of ["main", "multiple", "dots"] as const) {
    test(`${variant}: hidden table mirrors categories x series`, async ({ page }) => {
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      const table = frame(page, variant).locator('table[data-slot="chart-data"]');
      await expect(table).toHaveCount(1);

      const seriesCount = await frame(page, variant)
        .locator('g[data-series] path[data-slot="chart-radar-area"]')
        .count();
      const rows = table.locator("tbody tr");
      await expect(rows).toHaveCount(6); // 6 months

      const firstRowCells = rows.first().locator("td");
      await expect(firstRowCells).toHaveCount(seriesCount);
    });
  }
});
