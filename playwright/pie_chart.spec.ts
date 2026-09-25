import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

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
      await page.goto(URL);
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
  test("main: one arc per datum, slices sum to a full turn within tolerance", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "main");
    const slices = f.locator('[data-slot="chart-arc"]');
    const count = await slices.count();
    expect(count).toBeGreaterThan(1);

    // `pie.rs` emits `data-start-angle`/`data-end-angle` (radians, this
    // crate's `engine::polar` convention) on every arc specifically so a
    // spec can assert on the real layout numbers without reconstructing
    // them from the `d` path string.
    let total = 0;
    let previousEnd: number | null = null;
    for (let i = 0; i < count; i++) {
      const slice = slices.nth(i);
      await expect(slice).toHaveAttribute("data-index", String(i));
      const start = parseFloat((await slice.getAttribute("data-start-angle")) ?? "NaN");
      const end = parseFloat((await slice.getAttribute("data-end-angle")) ?? "NaN");
      expect(Number.isNaN(start)).toBe(false);
      expect(Number.isNaN(end)).toBe(false);
      if (previousEnd !== null) {
        expect(start).toBeCloseTo(previousEnd, 2);
      }
      previousEnd = end;
      total += end - start;
    }
    expect(total).toBeCloseTo(2 * Math.PI, 2);
  });

  test("donut: has a real hole (inner_radius > 0) and its arc path draws an inner ring", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "donut");
    const d = await f.locator('[data-slot="chart-arc"]').first().getAttribute("d");
    expect(d).not.toBeNull();
    // A donut slice's path draws two arcs (outer ring forward, inner ring
    // backward) -- a plain pie slice's path draws exactly one.
    expect((d!.match(/A/g) ?? []).length).toBeGreaterThanOrEqual(2);
  });

  test("stacked: renders two concentric rings (two series) from the same category axis", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "stacked");
    const seriesGroups = await f.locator('[data-slot="chart-arc"]').evaluateAll((nodes) =>
      Array.from(new Set(nodes.map((n) => n.getAttribute("data-series")))),
    );
    // Two rings -> two distinct dataKey-equivalent groupings (desktop,
    // mobile), matching chart-pie-stacked.tsx's own two `<Pie>` elements.
    expect(seriesGroups.filter((s) => s !== null).length).toBeGreaterThanOrEqual(2);
  });
});

test.describe("Pie chart: hover and active-slice behaviour", () => {
  test("simple: hovering a slice sets the active index; leaving clears it", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "main");
    const first = f.locator('[data-slot="chart-arc"]').first();

    await first.hover();
    await expect(first).toHaveAttribute("data-active", "true");

    await page.mouse.move(0, 0);
    await expect(first).not.toHaveAttribute("data-active", "true");
  });

  test("donut_active: a fixed slice is active without any hover", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "donut_active");
    const active = f.locator('[data-slot="chart-arc"][data-active="true"]');
    await expect(active).toHaveCount(1);
    await expect(active).toHaveAttribute("data-index", "0");
  });

  test("interactive: choosing a month in the Select moves the active slice", async ({ page }) => {
    await page.goto(URL);
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
    await page.goto(URL);
    const f = frame(page, "donut_text");
    const centerText = f.locator('[data-slot="chart-pie-center-text"]');
    await expect(centerText).toBeAttached();
    await expect(centerText).toContainText(/\d/); // the total, formatted
  });
});

test.describe("Pie chart: legend swatches", () => {
  test("legend: one graphics-symbol swatch per slice, each with an accessible name", async ({ page }) => {
    await page.goto(URL);
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
  test("label: a text label is drawn for every slice", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "label");
    const labels = f.locator('[data-slot="chart-arc-label"]');
    await expect(labels).toHaveCount(await f.locator('[data-slot="chart-arc"]').count());
  });

  test("label_list: each label reads the category name, not a formatted number", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "label_list");
    // chart-pie-label-list.tsx's own dataset -- the first category ported
    // into `variants/label_list/mod.rs`.
    await expect(f.locator('[data-slot="chart-arc-label"]').first()).toHaveText(/chrome/i);
  });
});

test.describe("Axe automated scan", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: no automatically detectable a11y issues`, async ({ page }) => {
      await page.goto(URL);
      await expectNoAxeViolations(page, `pie_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }

  test("simple: hovered (active slice shown) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(URL);
    const f = frame(page, "main");
    await f.locator('[data-slot="chart-arc"]').first().hover();
    await expect(f.locator('[data-slot="chart-arc"]').first()).toHaveAttribute("data-active", "true");
    await expectNoAxeViolations(page, "pie_chart: simple hovered", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
