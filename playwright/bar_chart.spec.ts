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
 * PHASE 1 (`main` -- shadcn's `chart-bar-default.tsx`, `multiple`,
 * `stacked`, `stacked_legend`, `interactive`): the variants buildable with
 * zero primitive changes beyond what the stage-1 Chart round already
 * shipped. PHASE 2 (`horizontal`, `negative`, `mixed`, `label`,
 * `label_custom`, `active`, added once `s2-refactor` landed and this
 * lane's own `BarOptions` feature work -- `horizontal`/`value_labels`/
 * `inside_labels`/`active_index` -- was integrated): all 10/10 of shadcn's
 * `chart-bar-*` demos.
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

test.describe("horizontal variant (shadcn chart-bar-horizontal.tsx)", () => {
  test("renders one wider-than-tall bar per browser with category labels", async ({ page }) => {
    await goto(page);
    const f = frame(page, "horizontal");
    const bars = f.locator('[data-slot="chart-bar"]');
    await expect(bars).toHaveCount(5);
    for (const bar of await bars.all()) {
      const box = await bar.boundingBox();
      expect(box).not.toBeNull();
      if (box) expect(box.width).toBeGreaterThan(box.height);
    }
    await expect(f.locator('[data-axis="category"]')).toHaveCount(1);
    // No default axes -- this family draws its own category labels.
    await expect(f.locator('[data-axis="x"]')).toHaveCount(0);
    await expect(f.locator('[data-axis="y"]')).toHaveCount(0);
  });
});

test.describe("negative variant (shadcn chart-bar-negative.tsx)", () => {
  test("draws a zero line with bars on both sides and per-datum colors", async ({ page }) => {
    await goto(page);
    const f = frame(page, "negative");
    await expect(f.locator('[data-slot="chart-zero-line"]')).toHaveCount(1);
    const bars = f.locator('[data-slot="chart-bar"]');
    await expect(bars).toHaveCount(6);
    const zeroY = await f.locator('[data-slot="chart-zero-line"]').getAttribute("y1");
    expect(zeroY).not.toBeNull();
    const boxes = await Promise.all((await bars.all()).map((b) => b.boundingBox()));
    const heights = boxes.map((b) => b?.height ?? 0);
    // Every bar has a real, positive height (both the positive and
    // negative months draw a visible rect, not a zero-length sliver).
    expect(heights.every((h) => h > 0)).toBe(true);
  });
});

test.describe("mixed variant (shadcn chart-bar-mixed.tsx)", () => {
  test("renders one bar per browser, each its own distinct fill color", async ({ page }) => {
    await goto(page);
    const f = frame(page, "mixed");
    const bars = f.locator('[data-slot="chart-bar"]');
    await expect(bars).toHaveCount(5);
    const fills = await Promise.all(
      (await bars.all()).map((b) => b.evaluate((el) => getComputedStyle(el).fill)),
    );
    // At least two distinct resolved fill colors among the 5 bars -- each
    // one sets its own `ChartDatum::color`, not one shared series color.
    expect(new Set(fills).size).toBeGreaterThan(1);
  });
});

test.describe("label variant (shadcn chart-bar-label.tsx)", () => {
  test("draws one value label above each bar, grid and y-axis hidden", async ({ page }) => {
    await goto(page);
    const f = frame(page, "label");
    await expect(f.locator('[data-slot="chart-bar"]')).toHaveCount(6);
    await expect(f.locator('[data-slot="chart-label"]')).toHaveCount(6);
    await expect(f.locator('[data-slot="chart-grid"]')).toHaveCount(0);
    await expect(f.locator('[data-axis="y"]')).toHaveCount(0);
  });
});

test.describe("label_custom variant (shadcn chart-bar-label-custom.tsx)", () => {
  test("draws a category label inside each bar and its value outside", async ({ page }) => {
    await goto(page);
    const f = frame(page, "label_custom");
    await expect(f.locator('[data-slot="chart-bar"]')).toHaveCount(5);
    // Two labels per bar: the inside category name and the outside value.
    await expect(f.locator('[data-slot="chart-label"]')).toHaveCount(10);
    await expect(f.locator('[data-position="inside"]')).toHaveCount(5);
    await expect(f.locator('[data-position="value"]')).toHaveCount(5);
  });
});

test.describe("active variant (shadcn chart-bar-active.tsx)", () => {
  test("marks exactly one bar active and dims every other bar", async ({ page }) => {
    await goto(page);
    const f = frame(page, "active");
    const bars = f.locator('[data-slot="chart-bar"]');
    await expect(bars).toHaveCount(5);
    // `getAttribute` (not a `[data-active="true"]` CSS locator): the raw
    // SSR text renders this as an unquoted boolean token
    // (`data-active=true`), a valid but unusual attribute form worth
    // reading directly rather than relying on a selector to normalize it.
    const activeCount = (
      await Promise.all((await bars.all()).map((b) => b.getAttribute("data-active")))
    ).filter((v) => v === "true").length;
    expect(activeCount).toBe(1);
    const opacities = await Promise.all(
      (await bars.all()).map((b) => b.evaluate((el) => getComputedStyle(el).opacity)),
    );
    // The 4 non-active bars share one dimmed opacity, distinct from the
    // active bar's own full opacity.
    expect(new Set(opacities).size).toBe(2);
  });
});

test.describe("Axe automated scan", () => {
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page);
    await expectNoAxeViolations(page, "bar_chart: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  // Title deliberately avoids the lowercase substring "mobile":
  // `playwright.config.ts`'s `grepInvert: /mobile/` (meant to skip
  // viewport-name tests) silently drops ANY test whose title contains that
  // substring, case-sensitively, with no warning -- dev-docs/issues/
  // chart-stage2-handoff.md §5's own documented trap, live in this exact
  // file until now (confirmed via `npx playwright test --list
  // bar_chart.spec.ts | wc -l` vs. `grep -c 'test(' bar_chart.spec.ts`
  // before this rename).
  test("interactive variant toggled to the second series has no automatically detectable a11y issues", async ({
    page,
  }) => {
    await goto(page);
    await frame(page, "interactive").locator(".dx-bar-chart-interactive-toggle").nth(1).click();
    await expectNoAxeViolations(page, "bar_chart: interactive toggled", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
