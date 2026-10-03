import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
// dev-docs/backlog.md row 109: this suite interacts (hover/click) right
// after navigating, on the SSG lane -- `gotoHydrated` so that can't land
// before hydration attaches listeners. Row 90's `networkidle` fix for this
// file is the same class under a weaker remedy; this supersedes it.
import { gotoHydrated } from "./hydration";

// The gallery's own variant folders, `preview/src/components/pie_chart/
// variants/*` -- `main` is shadcn's `chart-pie-simple.tsx`. One-to-one with
// `preview/src/components/mod.rs`'s `pie_chart[...]` registration line and
// shadcn's own 11 `chart-pie-*.tsx` demos (dev-docs/research/
// chart-2026-09-19.md §1.2) -- each variant's own doc comment in
// `preview/src/components/pie_chart/variants/<name>/mod.rs` cites the exact
// shadcn source file it ports.
//
// Every variant renders inline on one page load (`?name=pie_chart&`, the
// same single-page-multiple-frames convention every other multi-variant
// component's own spec uses, e.g. `playwright/radar_chart.spec.ts`'s own
// header comment): the un-suffixed `#component-preview-frame` is the
// `main` variant; every other variant gets
// `#component-preview-frame-<variant>`.
const VARIANTS = [
  "main", // chart-pie-simple.tsx
  "separator_none",
  "label",
  "label_list",
  "label_custom",
  "legend",
  "donut",
  "donut_active",
  "donut_text",
  "stacked",
  "interactive",
] as const;

const URL = `${BASE_URL}/component/?name=pie_chart&`;

function frame(page: Page, variant: (typeof VARIANTS)[number]): Locator {
  const suffix = variant === "main" ? "" : `-${variant}`;
  return page.locator(`#component-preview-frame${suffix}`).first();
}

test.describe("Pie chart: renders every variant as an accessible image", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: svg[role=img] with an accessible name, at least one slice, hidden data table`, async ({
      page,
    }) => {
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      const f = frame(page, variant);

      const svg = f.locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      const slices = f.locator('[data-slot="chart-arc"]');
      await expect(slices.first()).toBeAttached();

      // The a11y contract every chart family shares (`$S/chart-api.md`):
      // a real, visually-hidden <table> lists every datum, never just the
      // SVG -- category + value + percent for a polar chart per this
      // lane's own brief. One row per CATEGORY (`table_rows`/
      // `table_rows_pie`'s own shape), not per slice -- for every
      // single-ring variant that's the same number (one slice per
      // category), but `stacked` draws `n_series` slices per category
      // (one per concentric ring), so its own row count is the slice
      // count divided by its own two series, not equal to it.
      const table = f.locator('[data-slot="chart-data"]');
      await expect(table).toBeAttached();
      const rows = table.locator("tbody tr");
      const expectedRows = variant === "stacked" ? (await slices.count()) / 2 : await slices.count();
      await expect(rows).toHaveCount(expectedRows);
    });
  }
});

test.describe("Pie chart: slice geometry", () => {
  // `chart-pie-simple`'s own 5-category dataset (chart-pie-simple.tsx,
  // ported verbatim into `variants/main/mod.rs`) -- every non-stacked
  // variant shares it, so `main` is a representative, not a special case.
  test("main: one arc per datum, counter-clockwise from three o'clock, a full turn", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    const slices = f.locator('[data-slot="chart-arc"]');
    const count = await slices.count();
    expect(count).toBe(5);

    // `pie.rs` emits `data-start-angle`/`data-end-angle` in Recharts
    // degrees (0 = three o'clock, counter-clockwise) and the radii in px,
    // so a spec asserts on the real layout without parsing `d`. These are
    // the numbers measured on shadcn's rendered chart-pie-simple.
    let previousEnd = 0;
    for (let i = 0; i < count; i++) {
      const slice = slices.nth(i);
      await expect(slice).toHaveAttribute("data-index", String(i));
      const start = parseFloat((await slice.getAttribute("data-start-angle")) ?? "NaN");
      const end = parseFloat((await slice.getAttribute("data-end-angle")) ?? "NaN");
      expect(start).toBeCloseTo(previousEnd, 2);
      expect(end).toBeGreaterThan(start);
      previousEnd = end;
    }
    expect(previousEnd).toBeCloseTo(360, 2);
    await expect(slices.first()).toHaveAttribute("data-end-angle", /^107\.0/);
    await expect(slices.first()).toHaveAttribute("data-outer-radius", "96");
  });

  test("main: a 250px square, like shadcn's aspect-square max-h-[250px]", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const box = await frame(page, "main").locator('[data-slot="chart-svg"]').boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeLessThanOrEqual(250.5);
    expect(Math.abs(box!.width - box!.height)).toBeLessThan(1);
  });

  test("donut: has a real hole (inner_radius > 0) and its arc path draws an inner ring", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "donut");
    const d = await f.locator('[data-slot="chart-arc"]').first().getAttribute("d");
    expect(d).not.toBeNull();
    // A donut slice's path draws two arcs (outer ring forward, inner ring
    // backward) -- a plain pie slice's path draws exactly one.
    expect((d!.match(/A/g) ?? []).length).toBeGreaterThanOrEqual(2);
  });

  test("stacked: two concentric pies, a disc of radius 60 and a ring from 70 to 90", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "stacked");
    // One group per series (chart-pie-stacked.tsx's two `<Pie>` elements).
    await expect(f.locator('[data-slot="chart-series"] g[data-series]')).toHaveCount(2);
    const desktop = f.locator('g[data-series="desktop"] [data-slot="chart-arc"]');
    const mobile = f.locator('g[data-series="mobile"] [data-slot="chart-arc"]');
    await expect(desktop).toHaveCount(5);
    await expect(desktop.first()).toHaveAttribute("data-outer-radius", "60");
    await expect(mobile.first()).toHaveAttribute("data-inner-radius", "70");
    await expect(mobile.first()).toHaveAttribute("data-outer-radius", "90");
  });
});

test.describe("Pie chart: hover and active-slice behaviour", () => {
  test("simple: hovering a slice sets the active index; leaving clears it", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    const first = f.locator('[data-slot="chart-arc"]').first();

    await first.hover();
    await expect(first).toHaveAttribute("data-active", "true");

    await page.mouse.move(0, 0);
    await expect(first).not.toHaveAttribute("data-active", "true");
  });

  test("donut_active: a fixed slice is active, drawn 10px further out, hole unchanged", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "donut_active");
    const active = f.locator('[data-slot="chart-arc"][data-active="true"]');
    await expect(active).toHaveCount(1);
    await expect(active).toHaveAttribute("data-index", "0");
    await expect(active).toHaveAttribute("data-outer-radius", "106");
    await expect(active).toHaveAttribute("data-inner-radius", "60");
    // Geometry, not a CSS scale: the drawn arc is not transformed.
    expect(await active.evaluate((el) => getComputedStyle(el).transform)).toBe("none");
  });

  test("interactive: the active slice has its halo ring beyond the rim", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "interactive");
    await expect(f.locator('[data-slot="chart-arc-halo"]')).toHaveCount(1);
  });

  test("interactive: choosing a month in the Select moves the active slice", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "interactive");

    const before = await f
      .locator('[data-slot="chart-arc"][data-active="true"]')
      .getAttribute("data-index");

    await f.getByRole("combobox").click();
    // The trigger's own popup renders in the top document, not the iframe
    // (this repo's overlay components render via a portal/top-layer) --
    // matched by option text, matching this repo's own Select spec
    // convention elsewhere (select.spec.ts).
    await page.getByRole("option", { name: /february/i }).click();

    await expect(f.locator('[data-slot="chart-arc"][data-active="true"]')).toHaveAttribute(
      "data-index",
      /.+/,
    );
    const after = await f
      .locator('[data-slot="chart-arc"][data-active="true"]')
      .getAttribute("data-index");
    expect(after).not.toBe(before);
  });
});

test.describe("Pie chart: donut center text", () => {
  test("donut_text: renders the two-line center label inside the hole", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "donut_text");
    const centerText = f.locator('[data-slot="chart-pie-center-text"]');
    await expect(centerText).toBeAttached();
    await expect(centerText).toContainText(/\d/); // the total, formatted
  });
});

test.describe("Pie chart: legend swatches", () => {
  test("legend: one graphics-symbol swatch per slice, each with an accessible name", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "legend");
    const swatches = f.locator('[data-slot="chart-swatch"][role="graphics-symbol"]');
    const count = await swatches.count();
    expect(count).toBeGreaterThan(1);
    for (let i = 0; i < count; i++) {
      await expect(swatches.nth(i)).toHaveAccessibleName(/.+/);
    }
  });
});

test.describe("Pie chart: labels", () => {
  test("label: every slice's value outside the rim, with a leader line", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "label");
    const labels = f.locator('[data-slot="chart-arc-label"][data-position="outside"]');
    await expect(labels).toHaveCount(await f.locator('[data-slot="chart-arc"]').count());
    await expect(labels.first()).toHaveText("275");
    await expect(f.locator('[data-slot="chart-arc-label-line"]')).toHaveCount(5);
  });

  test("label_list: each label reads the category name, not a formatted number", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "label_list");
    // chart-pie-label-list.tsx's own dataset -- the first category ported
    // into `variants/label_list/mod.rs`.
    await expect(f.locator('[data-slot="chart-arc-label"]').first()).toHaveText(/chrome/i);
  });
});

test.describe("Pie chart: load animation", () => {
  test("main: the slices sweep in once measured, and not under reduced motion", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const mask = frame(page, "main").locator('[data-slot="chart-sweep-mask"] path');
    await expect(frame(page, "main").locator('[data-slot="chart-svg"][data-animate="true"]')).toBeAttached();
    expect(await mask.evaluate((el) => getComputedStyle(el).animationName)).toBe("dx-chart-sweep");
    await page.emulateMedia({ reducedMotion: "reduce" });
    expect(await mask.evaluate((el) => getComputedStyle(el).animationName)).toBe("none");
  });
});

test.describe("Axe automated scan", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: no automatically detectable a11y issues`, async ({ page }) => {
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      await expectNoAxeViolations(page, `pie_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }

  test("simple: hovered (active slice shown) has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    await f.locator('[data-slot="chart-arc"]').first().hover();
    await expect(f.locator('[data-slot="chart-arc"]').first()).toHaveAttribute("data-active", "true");
    await expectNoAxeViolations(page, "pie_chart: simple hovered", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
