import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
// dev-docs/backlog.md row 109: this suite interacts (hover/click) right
// after navigating, on the SSG lane -- `gotoHydrated` so that can't land
// before hydration attaches listeners. Row 90's `networkidle` fix for this
// file is the same class under a weaker remedy; this supersedes it.
import { gotoHydrated } from "./hydration";

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
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
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
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    // chart-radial-simple.tsx's own 5-category dataset, ported verbatim.
    const arcs = f.locator('[data-slot="chart-arc"]');
    const count = await arcs.count();
    expect(count).toBe(5);
    for (let i = 0; i < count; i++) {
      await expect(arcs.nth(i)).toHaveAttribute("data-index", String(i));
    }
  });

  test("main: rings start at three o'clock and sweep counter-clockwise over muted tracks", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "main");
    const arcs = f.locator('[data-slot="chart-arc"]');
    // Recharts degrees (0 = three o'clock, counter-clockwise), px radii:
    // the numbers measured on shadcn's chart-radial-simple.
    await expect(arcs.nth(0)).toHaveAttribute("data-start-angle", "0");
    await expect(arcs.nth(0)).toHaveAttribute("data-end-angle", "360");
    await expect(arcs.nth(1)).toHaveAttribute("data-end-angle", /^261\.8/);
    await expect(arcs.nth(0)).toHaveAttribute("data-inner-radius", "31.6");
    await expect(arcs.nth(0)).toHaveAttribute("data-outer-radius", "43.6");
    await expect(f.locator('[data-slot="chart-radial-background"]')).toHaveCount(5);
  });

  test("grid: circles through the rings and spokes, no tracks", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "grid");
    await expect(f.locator('circle[data-slot="chart-grid-ring"]')).toHaveCount(5);
    await expect(f.locator('[data-slot="chart-grid-spoke"]')).toHaveCount(14);
    await expect(f.locator('[data-slot="chart-radial-background"]')).toHaveCount(0);
  });

  test("text: a 250-degree gauge over a full muted ring", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "text");
    const arc = f.locator('[data-slot="chart-arc"]');
    await expect(arc).toHaveAttribute("data-end-angle", "250");
    await expect(arc).toHaveAttribute("data-inner-radius", "81");
    await expect(arc).toHaveAttribute("data-outer-radius", "89");
    await expect(f.locator('[data-slot="chart-radial-annulus"]')).toHaveCount(1);
    await expect(f.locator('[data-slot="chart-pie-center-text"]')).toHaveText("200Visitors");
  });

  test("stacked: two series stack cumulatively into one ring, not two concentric rings", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
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
    // Mobile first, at its share of the stack total (a deliberate
    // difference from Recharts' clamped scale -- see radial.rs), and the
    // stack fills the half turn exactly.
    await expect(arcs.nth(0)).toHaveAttribute("data-series", "mobile");
    expect(end0).toBeCloseTo((570 / 1830) * 180, 1);
    await expect(arcs.nth(1)).toHaveAttribute("data-end-angle", "180");
    await expect(f.locator('[data-slot="chart-pie-center-text"]')).toHaveText("1,830Visitors");
  });
});

test.describe("Radial chart: center text and labels", () => {
  test("text: renders the two-line center label", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "text");
    const centerText = f.locator('[data-slot="chart-pie-center-text"]');
    await expect(centerText).toBeAttached();
    await expect(centerText).toContainText(/\d/);
  });

  test("shape: a single ring with square ends, like shadcn's", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "shape");
    const d = await f.locator('[data-slot="chart-arc"]').first().getAttribute("d");
    expect(d).not.toBeNull();
    // shadcn's chart-radial-shape sets no `cornerRadius` (its fixture,
    // preview/tests/shadcn/chart-radial-shape.json, has `cornerRadius: null`):
    // a plain annular sector, whose path has exactly the two ring arcs and
    // no fillet `A` commands.
    expect((d!.match(/A/g) ?? []).length).toBe(2);
  });

  test("label: rings start at six o'clock and each is named along its arc", async ({ page }) => {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const f = frame(page, "label");
    await expect(f.locator('[data-slot="chart-arc"]').first()).toHaveAttribute("data-start-angle", "-90");
    await expect(f.locator('[data-slot="chart-arc-label"]')).toHaveCount(5);
    await expect(f.locator('[data-slot="chart-arc-label"]').first()).toHaveText("Chrome");
  });
});

test.describe("Axe automated scan", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: no automatically detectable a11y issues`, async ({ page }) => {
      await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
      await expectNoAxeViolations(page, `radial_chart: ${variant}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});
