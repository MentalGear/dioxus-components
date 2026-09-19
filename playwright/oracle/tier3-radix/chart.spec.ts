/**
 * ORACLE (tier 3 -- Radix-parity, labelled opinion; see
 * dev-docs/conformance-harness.md "Tier 3 -- Radix-parity" and
 * playwright/oracle/tier3-radix/README.md's rule-source policy).
 *
 * Chart has no Radix primitive to calibrate against at all -- Radix ships
 * no chart component, and neither does any fork this repo's own research
 * already surveyed (dev-docs/research/chart-2026-09-19.md §2.4). This file
 * lives in `tier3-radix/` (the folder name is kept, per that research
 * doc's own §6.5 decision (b): a new `chart.spec.ts` whose header names its
 * real opinion source rather than renaming the folder) with two different
 * opinion sources instead of a Radix demo:
 *
 *   - Recharts' `accessibilityLayer`, read from source at
 *     `recharts/recharts` commit 86ad3632ff3f83a742001ae6fc079dd27961a2a4
 *     (2026-09-18, MIT) -- specifically its keyboard contract
 *     (`src/state/keyboardEventsMiddleware.ts`: focus activates a data
 *     point, ArrowRight/ArrowLeft step it direction-aware, Home/End jump to
 *     the ends, Escape/blur clears it). R5 below is calibrated against this
 *     reading, not against a live Recharts demo (no local checkout is
 *     wired into this Playwright run) -- same "reasoned from source, not
 *     dual-run" status `dev-docs/conformance-harness.md`'s calibration
 *     table already accepts for a component with no runnable reference.
 *   - The W3C WAI-ARIA Graphics Module 1.0 (https://www.w3.org/TR/graphics-aria-1.0/,
 *     Recommendation, 2018-10-02) for the `img`/`graphics-symbol` roles R1
 *     and R3 assert -- a standards *vocabulary*, not a behavior pattern, so
 *     it argably belongs nearer tier 1/2 than tier 3; kept here rather than
 *     split across folders because every rule in this file is calibrated
 *     against the same single subject (`Chart` itself, §6.5's option (b))
 *     and splitting one component's oracle across two tiers for a single
 *     role-vocabulary citation would cost more clarity than it buys. R8 (no
 *     `role="application"`) is this repo's own construction, reasoned from
 *     both sources' silence/downside -- not itself cited to either.
 *
 * Calibration: no second, known-correct subject exists to dual-run these
 * rules against (no Radix demo, no vendored APG example page -- neither
 * applies here, per the research doc's §2.2/§2.4). Every rule below is
 * therefore internal-opinion, asserted only against this repo's own
 * `Chart`, the same status `oracle-focus-restore.spec.ts` originally
 * carried before an external reference existed
 * (dev-docs/conformance-harness.md's own calibration table).
 *
 * Fixture: `?name=chart&variant=bar` throughout -- three series (desktop/
 * mobile/tablet), a legend, a tooltip, and a small, fixed six-row dataset
 * (`preview/src/components/chart/variants/bar/mod.rs`), which keeps every
 * index-based assertion below (Home/End, "row 0"/"row 5") exact and
 * independent of the `main` variant's interactive range picker.
 *
 * STATUS AT WRITE TIME: RED. `Chart`/`ChartContainer`/`ChartTooltip`/
 * `ChartLegend` do not exist in `dioxus_primitives` yet (this lane is
 * written concurrently with, and ahead of, the chart-primitive lane
 * landing them -- $S/chart-lanes.md has the hand-off) and the `chart[bar,
 * line, stacked]` demo registration this file's fixture depends on is not
 * yet wired into `preview/src/components/mod.rs`'s compiled module tree.
 * Every assertion below is written directly against the binding contract
 * ($S/chart-api.md), not against a running server -- expected to fail
 * end-to-end (the route itself 404s/doesn't render) until both land, then
 * to be run for real and fixed up against whatever the real DOM turns out
 * to need (dev-docs/dx-serve-hot-reload.md; "iterate visually later" per
 * this lane's own brief).
 */

import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "../../axe";
import { BASE_URL } from "../../base-url";

const URL = `${BASE_URL}/component/?name=chart&variant=bar&`;
const DARK_URL = `${BASE_URL}/component/?name=chart&variant=bar&dark_mode=true&`;

function frameOf(page: Page) {
  return page.locator("#component-preview-frame").first();
}

async function goto(page: Page, url: string = URL) {
  await page.goto(url, { timeout: 20 * 60 * 1000 });
  await frameOf(page).locator("svg[role=\"img\"]").first().waitFor({ state: "attached" });
}

test.describe("R1 -- svg root: role=img, non-empty accessible name, <title> (WAI-ARIA Graphics Module 1.0, img role)", () => {
  test("the chart svg has role=img, a non-empty aria-label, and a <title>", async ({ page }) => {
    await goto(page);
    const svg = frameOf(page).locator('svg[role="img"]').first();
    await expect(svg).toBeVisible();

    const ariaLabel = await svg.getAttribute("aria-label");
    expect(ariaLabel, "svg[role=img] must carry a non-empty aria-label").toBeTruthy();
    expect(ariaLabel!.trim().length).toBeGreaterThan(0);

    const title = svg.locator("> title").first();
    await expect(title).toHaveCount(1);
    const titleText = await title.textContent();
    expect(titleText?.trim().length ?? 0).toBeGreaterThan(0);
  });
});

test.describe("R2 -- hidden data table: one row per datum, one cell per series, values match the tooltip", () => {
  test("the hidden table has the right shape and its values agree with the tooltip's", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page);
    const table = frame.locator('table[data-slot="chart-data"]');
    await expect(table).toHaveCount(1);

    // 6 months in the `bar` variant's fixture, 3 series.
    const rows = table.locator("tbody tr");
    await expect(rows).toHaveCount(6);
    const firstRowCells = rows.first().locator("td");
    await expect(firstRowCells).toHaveCount(3);

    const tableValues = await firstRowCells.allTextContents();

    // Hovering the first datum's hit-band must show the SAME values the
    // table's first row carries, series for series.
    await frame.locator('[data-slot="chart-hit-band"][data-index="0"]').hover();
    const tooltip = frame.locator('[data-slot="chart-tooltip"]');
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const tooltipValues = await tooltip.locator('[data-slot="chart-tooltip-value"]').allTextContents();

    expect(tooltipValues.length).toBe(tableValues.length);
    for (let i = 0; i < tableValues.length; i++) {
      const tableNum = parseFloat(tableValues[i].replace(/[^0-9.\-]/g, ""));
      const tooltipNum = parseFloat(tooltipValues[i].replace(/[^0-9.\-]/g, ""));
      expect(Number.isNaN(tableNum), `table cell ${i} ("${tableValues[i]}") must be numeric or the "—" no-value marker`).toBe(false);
      expect(tooltipNum, `table cell ${i} ("${tableValues[i]}") vs tooltip value ${i} ("${tooltipValues[i]}")`).toBeCloseTo(tableNum, 1);
    }
  });
});

test.describe("R3 -- legend swatches: role=graphics-symbol + aria-label (WAI-ARIA Graphics Module 1.0, graphics-symbol role)", () => {
  test("every legend swatch is a labelled graphics-symbol", async ({ page }) => {
    await goto(page);
    const items = frameOf(page).locator('[data-slot="chart-legend-item"]');
    await expect(items).toHaveCount(3); // desktop, mobile, tablet

    const count = await items.count();
    for (let i = 0; i < count; i++) {
      const swatch = items.nth(i).locator('[data-slot="chart-swatch"]');
      await expect(swatch).toHaveAttribute("role", "graphics-symbol");
      const label = await swatch.getAttribute("aria-label");
      expect(label, `legend swatch ${i} must have a non-empty aria-label`).toBeTruthy();
    }
  });
});

test.describe("R4 -- hovering a hit-band opens the tooltip with that datum's label and values", () => {
  test("the tooltip's label matches the hovered datum's row in the hidden table", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page);
    const table = frame.locator('table[data-slot="chart-data"]');
    const rowHeaders = table.locator("tbody tr th[scope=\"row\"]");

    for (const index of [0, 2, 5]) {
      const expectedLabel = (await rowHeaders.nth(index).textContent())?.trim();
      await frame.locator(`[data-slot="chart-hit-band"][data-index="${index}"]`).hover();
      const tooltip = frame.locator('[data-slot="chart-tooltip"]');
      await expect(tooltip).toHaveAttribute("data-state", "open");
      const tooltipLabel = (await tooltip.locator('[data-slot="chart-tooltip-label"]').textContent())?.trim();
      expect(tooltipLabel, `hit-band ${index}`).toBe(expectedLabel);
      // 3 series in this fixture -- every row's tooltip must show all 3.
      await expect(tooltip.locator('[data-slot="chart-tooltip-item"]')).toHaveCount(3);
    }
  });
});

test.describe("R5 -- keyboard: ArrowRight/ArrowLeft step the active index, Home/End jump, Escape closes (Recharts keyboardEventsMiddleware.ts, tier-3 opinion)", () => {
  // No `rtl` demo variant exists for `chart` (unlike e.g. `calendar[...,
  // rtl]`) at MVP -- per this lane's brief ("otherwise assert LTR only and
  // note it"), the ArrowRight/ArrowLeft direction-swap under `dir="rtl"`
  // Recharts applies (`selectChartDirection`) is NOT covered here. Add a
  // `chart[..., rtl]` demo variant and a dedicated case here (mirroring
  // `oracle/tier3-radix/rtl.spec.ts`'s own per-component cases) if/when one
  // is added.
  test("LTR: ArrowRight/ArrowLeft/Home/End/Escape drive the active index and the tooltip", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page);
    const container = frame.locator("[data-chart]");
    const tooltip = frame.locator('[data-slot="chart-tooltip"]');
    const table = frame.locator('table[data-slot="chart-data"]');
    const rowHeaders = table.locator("tbody tr th[scope=\"row\"]");
    const firstLabel = (await rowHeaders.nth(0).textContent())?.trim();
    const secondLabel = (await rowHeaders.nth(1).textContent())?.trim();
    const lastLabel = (await rowHeaders.nth(5).textContent())?.trim();

    // Contract shape of the container itself, not just its behavior.
    await expect(container).toHaveAttribute("tabindex", "0");
    await expect(container).toHaveAttribute("role", "group");
    await expect(container).toHaveAttribute("aria-roledescription", "chart");

    await container.focus();
    await page.keyboard.press("ArrowRight");
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(firstLabel!);

    await page.keyboard.press("ArrowRight");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(secondLabel!);

    await page.keyboard.press("ArrowLeft");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(firstLabel!);

    await page.keyboard.press("End");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(lastLabel!);

    await page.keyboard.press("Home");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(firstLabel!);

    await page.keyboard.press("Escape");
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });
});

test.describe("R6 -- every series' computed color is non-transparent, and distinct, in light AND dark mode (--dx-chart-* tokens)", () => {
  async function seriesFillColors(page: Page): Promise<string[]> {
    const groups = frameOf(page).locator('[data-slot="chart-series"]');
    const count = await groups.count();
    const colors: string[] = [];
    for (let i = 0; i < count; i++) {
      const mark = groups.nth(i).locator('[data-slot="chart-bar"]').first();
      const fill = await mark.evaluate((el) => getComputedStyle(el).fill);
      colors.push(fill);
    }
    return colors;
  }

  function assertOpaqueAndDistinct(colors: string[]) {
    expect(colors.length).toBe(3); // desktop, mobile, tablet
    for (const c of colors) {
      expect(c, `computed fill "${c}" must not be none/transparent`).not.toBe("none");
      expect(c).not.toBe("transparent");
      // fully-transparent black is what an unset/failed color read looks
      // like as `rgba(0, 0, 0, 0)` -- a real token always has alpha 1 here.
      expect(c).not.toMatch(/rgba\(0,\s*0,\s*0,\s*0\)/);
    }
    expect(new Set(colors).size, `all ${colors.length} series must resolve to distinct colors: ${colors.join(", ")}`).toBe(colors.length);
  }

  test("light mode", async ({ page }) => {
    await goto(page, URL);
    assertOpaqueAndDistinct(await seriesFillColors(page));
  });

  test("dark mode", async ({ page }) => {
    await goto(page, DARK_URL);
    assertOpaqueAndDistinct(await seriesFillColors(page));
  });
});

test.describe("R7 -- axe clean at rest and with the tooltip open", () => {
  test("at rest", async ({ page }) => {
    await goto(page);
    await expectNoAxeViolations(page, "chart oracle: at rest", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tooltip open", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page);
    await frame.locator('[data-slot="chart-hit-band"][data-index="0"]').hover();
    await expect(frame.locator('[data-slot="chart-tooltip"]')).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "chart oracle: tooltip open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

test.describe("R8 -- no role=application anywhere (editorial: a heavy-handed ARIA escape hatch neither Recharts' own upstream standard-body citation nor the Graphics Module calls for; dev-docs/research/chart-2026-09-19.md §2.1/§2.5)", () => {
  test("the component page renders no role=application element", async ({ page }) => {
    await goto(page);
    await expect(page.locator('[role="application"]')).toHaveCount(0);
  });
});
