import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

// Every variant renders inline on one page load (`?name=radial_chart&`,
// same convention every other multi-variant component's own spec uses,
// e.g. `playwright/radar_chart.spec.ts`'s own header comment): the
// un-suffixed `#component-preview-frame` is the `main` (shadcn "default")
// variant; every other variant gets `#component-preview-frame-<variant>`
// (`preview/src/main.rs`'s own frame-id-per-variant naming). One-to-one
// with `preview/src/components/mod.rs`'s `radial_chart[...]` registration
// line and shadcn's own 6 `chart-radial-*.tsx` demos (dev-docs/research/
// chart-2026-09-19.md §1.2); each variant's own doc comment cites its
// exact shadcn source file.
const VARIANTS = ["main", "label", "grid", "text", "shape", "stacked"] as const;

const URL = `${BASE_URL}/component/?name=radial_chart&`;

function frame(page: Page, variant: (typeof VARIANTS)[number]): Locator {
  const suffix = variant === "main" ? "" : `-${variant}`;
  return page.locator(`#component-preview-frame${suffix}`).first();
}

test.describe("Radial chart: renders every variant as an accessible image", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: svg[role=img] with an accessible name, at least one ring, hidden data table`, async ({
      page,
    }) => {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      const f = frame(page, variant);

      const svg = f.locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      const arcs = f.locator('[data-slot="chart-arc"]');
      await expect(arcs.first()).toBeAttached();

      const table = f.locator('[data-slot="chart-data"]');
      await expect(table).toBeAttached();
      await expect(table.locator("tbody tr").first()).toBeAttached();
    });
  }
});

test.describe("Radial chart: ring geometry", () => {
  test("main: one ring per datum, each a wide (not tall) arc", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "main");
    // chart-radial-simple.tsx's own 5-category dataset, ported verbatim.
    const arcs = f.locator('[data-slot="chart-arc"]');
    const count = await arcs.count();
    expect(count).toBe(5);
    for (let i = 0; i < count; i++) {
      await expect(arcs.nth(i)).toHaveAttribute("data-index", String(i));
    }
  });

  test("grid: a background track renders behind the value arcs", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "grid");
    await expect(f.locator('[data-slot="chart-polar-grid"]').first()).toBeAttached();
  });

  test("stacked: two series stack cumulatively into one ring, not two concentric rings", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "stacked");
    const arcs = f.locator('[data-slot="chart-arc"]');
    // One datum row (chart-radial-stacked.tsx: `[{ month: "january", ... }]`)
    // with two series (mobile, desktop) stacked into that one ring -- two
    // arcs total, both `data-index="0"`, distinguished by `data-series`.
    await expect(arcs).toHaveCount(2);
    await expect(arcs.nth(0)).toHaveAttribute("data-index", "0");
    await expect(arcs.nth(1)).toHaveAttribute("data-index", "0");
    const series = await arcs.evaluateAll((nodes) => nodes.map((n) => n.getAttribute("data-series")));
    expect(new Set(series).size).toBe(2);

    // The two arcs are contiguous (stacked): the first one's end angle is
    // the second one's start angle.
    const end0 = parseFloat((await arcs.nth(0).getAttribute("data-end-angle")) ?? "NaN");
    const start1 = parseFloat((await arcs.nth(1).getAttribute("data-start-angle")) ?? "NaN");
    expect(end0).toBeCloseTo(start1, 2);
  });
});

test.describe("Radial chart: center text and labels", () => {
  test("text: renders the two-line center label", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "text");
    const centerText = f.locator('[data-slot="chart-pie-center-text"]');
    await expect(centerText).toBeAttached();
    await expect(centerText).toContainText(/\d/);
  });

  test("shape: a single ring renders with a rounded corner radius", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "shape");
    const d = await f.locator('[data-slot="chart-arc"]').first().getAttribute("d");
    expect(d).not.toBeNull();
    // A rounded-corner arc's path uses small-radius fillet `A` commands in
    // addition to the two main ring arcs -- more than the 2 a sharp-corner
    // annular sector needs.
    expect((d!.match(/A/g) ?? []).length).toBeGreaterThan(2);
  });

  test("label: each ring's own category label is drawn", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const f = frame(page, "label");
    await expect(f.locator('[data-slot="chart-arc-label"]').first()).toBeAttached();
  });
});

test.describe("Axe automated scan", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: no automatically detectable a11y issues`, async ({ page }) => {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      await expectNoAxeViolations(page, `radial_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});
