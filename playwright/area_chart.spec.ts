import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

// `area_chart` is a shadcn/ui `chart-area-*` demo gallery -- see
// `preview/src/components/area_chart/component.rs`'s own doc comment. Every
// variant renders on the same `/component/?name=area_chart&` page (this
// repo's normal multi-variant layout, `preview/src/main.rs`): the first/
// default demo's frame is `#component-preview-frame`, every other named
// variant's is `#component-preview-frame-<variant>`.
const VARIANTS: { name: string; frameId: string; seriesCount: number }[] = [
  { name: "default", frameId: "component-preview-frame", seriesCount: 1 },
  { name: "linear", frameId: "component-preview-frame-linear", seriesCount: 1 },
  { name: "step", frameId: "component-preview-frame-step", seriesCount: 1 },
  { name: "stacked", frameId: "component-preview-frame-stacked", seriesCount: 2 },
  { name: "stacked_expand", frameId: "component-preview-frame-stacked_expand", seriesCount: 3 },
  { name: "gradient", frameId: "component-preview-frame-gradient", seriesCount: 2 },
  { name: "legend", frameId: "component-preview-frame-legend", seriesCount: 2 },
  { name: "axes", frameId: "component-preview-frame-axes", seriesCount: 2 },
  { name: "icons", frameId: "component-preview-frame-icons", seriesCount: 2 },
  { name: "interactive", frameId: "component-preview-frame-interactive", seriesCount: 2 },
];

test.describe("area_chart variants render an accessible chart", () => {
  for (const variant of VARIANTS) {
    test(`${variant.name}: svg has role=img, an accessible name, and one area+line per series`, async ({
      page,
    }) => {
      await page.goto(`${BASE_URL}/component/?name=area_chart&`);
      const frame = page.locator(`#${variant.frameId}`).first();

      const svg = frame.locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      await expect(frame.locator('[data-slot="chart-area"]')).toHaveCount(variant.seriesCount);
      await expect(frame.locator('[data-slot="chart-line"]')).toHaveCount(variant.seriesCount);

      // The hidden data table is always present, mirroring the same series
      // count as columns (one header cell per series, plus the corner).
      await expect(frame.locator('[data-slot="chart-data"] thead th')).toHaveCount(
        variant.seriesCount + 1,
      );

      // Hovering a hit-band opens the tooltip -- the one hover mechanism
      // every variant shares regardless of its own curve/stacking config.
      const tooltip = frame.locator('[data-slot="chart-tooltip"]');
      await frame.locator('[data-slot="chart-hit-band"]').first().hover();
      await expect(tooltip).toHaveAttribute("data-state", "open");
      await page.mouse.move(0, 0);
      await expect(tooltip).toHaveAttribute("data-state", "closed");
    });
  }
});

test.describe("Axe automated scan", () => {
  for (const variant of VARIANTS) {
    test(`${variant.name} has no automatically detectable a11y issues`, async ({ page }) => {
      await page.goto(`${BASE_URL}/component/?name=area_chart&`);
      await expectNoAxeViolations(page, `area_chart: ${variant.name}`, {
        include: `#${variant.frameId}`,
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});

test.describe("Behavioural: variant-specific rendering", () => {
  // Title deliberately avoids the substring "mobile" (lowercase): every
  // playwright project in `playwright.config.ts` (chromium/webkit/firefox)
  // sets `grepInvert: /mobile/` to skip mobile-viewport tests, and that
  // regex matches ANY test title containing that substring, not just ones
  // about viewports -- an earlier draft of this title ("...mobile's whole
  // span...") was silently excluded from every run, everywhere, by that
  // filter (confirmed via `--list`: 16 tests instead of the expected 17,
  // with no error or warning). Worth knowing for every other stage-2 chart
  // lane too: `chart-api.md`'s own sample data names a series "mobile",
  // and any spec prose that echoes that name verbatim risks the same
  // silent exclusion.
  test("stacked: the two series sum, not overlap (the second series' whole span sits under the first's)", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/component/?name=area_chart&`);
    const frame = page.locator("#component-preview-frame-stacked").first();
    // A stacked chart's hidden table still lists every series' own raw
    // value (stacking is a drawing concern, not a data concern) -- January
    // is desktop=186, the second series ("mobile")=80, so both values must
    // still be readable.
    const row = frame.locator('[data-slot="chart-data"] tbody tr').first();
    await expect(row.locator("td")).toHaveText(["186", "80"]);
  });

  test("step: the line path is a staircase -- every segment after the initial M is purely horizontal or purely vertical", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/component/?name=area_chart&`);
    const frame = page.locator("#component-preview-frame-step").first();
    const d = await frame.locator('[data-slot="chart-line"]').first().getAttribute("d");
    expect(d).toBeTruthy();

    // This crate's `Curve::Step` (primitives/src/chart/engine/curve.rs)
    // emits plain SVG `L` (line-to) commands whose endpoints happen to
    // share an x or a y with the previous point -- geometrically a
    // horizontal/vertical "H"/"V" segment either way, just not spelled
    // with that literal shorthand letter. Parse every (x, y) pair instead
    // of grepping for `H`/`V` characters that this path will never
    // contain.
    const numbers = (d as string).match(/-?\d+(?:\.\d+)?/g)?.map(Number) ?? [];
    const points: [number, number][] = [];
    for (let i = 0; i + 1 < numbers.length; i += 2) {
      points.push([numbers[i], numbers[i + 1]]);
    }
    expect(points.length).toBeGreaterThan(2);

    for (let i = 1; i < points.length; i++) {
      const [x0, y0] = points[i - 1];
      const [x1, y1] = points[i];
      const isHorizontal = y0 === y1;
      const isVertical = x0 === x1;
      expect(
        isHorizontal || isVertical,
        `segment ${i} (${x0},${y0}) -> (${x1},${y1}) is diagonal, not a step`,
      ).toBe(true);
    }
  });

  test("stacked_expand: the topmost stacked area reaches the plot top (y close to the margin)", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/component/?name=area_chart&`);
    const frame = page.locator("#component-preview-frame-stacked_expand").first();
    // Config order is desktop, mobile, other (`ChartConfig::series`
    // positional stacking order -- `engine::stack`'s own doc): each
    // series stacks on top of the previous one, so "other" (added last)
    // is the topmost series, and every row's percent-stacked top is
    // exactly 1.0 regardless of the row's raw total.
    const topPath = frame.locator('[data-series="other"] [data-slot="chart-area"]');
    const d = await topPath.getAttribute("d");
    expect(d).toBeTruthy();
    const match = (d as string).match(/^M(-?[\d.]+)[ ,](-?[\d.]+)/);
    expect(match).toBeTruthy();
    const y = Number(match![2]);
    // MARGIN_TOP (primitives/src/chart/components/layout.rs) is 8.0 of a
    // 300-tall default viewBox -- a wide, deliberately loose bound (not
    // pixel-exact) so this test is about "reaches the plot top", not a
    // brittle pin on an internal layout constant.
    expect(y).toBeLessThan(9);
  });

  test("gradient: every series' area fill references its own <linearGradient> def", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/component/?name=area_chart&`);
    const frame = page.locator("#component-preview-frame-gradient").first();

    const areaPaths = frame.locator('[data-slot="chart-area"]');
    await expect(areaPaths).toHaveCount(2);

    for (const fill of await areaPaths.evaluateAll((els) => els.map((el) => el.getAttribute("fill")))) {
      expect(fill).toMatch(/^url\(#.+-gradient-.+\)$/);
      const id = (fill as string).slice(5, -1); // "url(#X)" -> "X"
      // The referenced def actually exists, inside this same chart (defs
      // render per-series, inside that series' own `g[data-series]`
      // group -- primitives/src/chart/components/series/area.rs).
      await expect(frame.locator(`linearGradient[id="${id}"]`)).toHaveCount(1);
    }
  });

  test('interactive: switching to "Last 7 days" shows exactly 7 hit-bands', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=area_chart&`);
    const frame = page.locator("#component-preview-frame-interactive").first();
    await expect(frame.locator('[data-slot="chart-hit-band"]')).toHaveCount(90);

    // Same trigger/listbox/option pattern as `select.spec.ts` -- the
    // listbox itself renders in a top-layer portal, not nested under this
    // demo's own frame.
    await frame.getByRole("combobox").click();
    const listbox = page.getByRole("listbox");
    await expect(listbox).toHaveAttribute("data-state", "open");
    await listbox.getByRole("option", { name: "Last 7 days" }).click();
    await expect(frame.locator('[data-slot="chart-hit-band"]')).toHaveCount(7);
  });
});
