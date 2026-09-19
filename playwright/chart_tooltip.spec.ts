/**
 * STATUS AT WRITE TIME: partially RED, by construction (mirrors this
 * repo's own precedent for writing a spec ahead of the implementation it
 * covers, e.g. `playwright/oracle/tier3-radix/chart.spec.ts`'s own "STATUS
 * AT WRITE TIME: RED" header). Four of the nine variants below (`main`,
 * `indicator_none`, `label_none`, `label_formatter`) are registered and
 * pass today; the other five (`indicator_line`, `label_custom`,
 * `formatter`, `icons`, `advanced`) need `ChartTooltipProps::{indicator,
 * label_key, formatter}` and `ChartSeries.icon`, which land only after
 * `s2-refactor` lands and this lane cherry-picks it (`$S/stage2-lanes.md`)
 * -- until then their routes 404 and every test below for them fails at
 * `goto`, not at a real assertion. Run this file for real once all nine
 * variants are registered in `preview/src/components/mod.rs`.
 */
import { test, expect, type Page, type Locator } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

// `chart_tooltip`'s nine gallery variants -- one per shadcn
// `chart-tooltip-*.tsx` demo (`$S/refs/ui/apps/v4/registry/new-york-v4/
// charts/chart-tooltip-*.tsx`; `preview/src/components/chart_tooltip/
// docs.md` table). "main" is the un-suffixed default variant every
// component page renders first; every other name below is a named variant
// registered in `preview/src/components/mod.rs`'s `examples!` list.
const VARIANTS = [
  "main",
  "indicator_line",
  "indicator_none",
  "label_none",
  "label_custom",
  "label_formatter",
  "formatter",
  "icons",
  "advanced",
] as const;

const URL = `${BASE_URL}/component/?name=chart_tooltip&`;

// Same convention as `playwright/oracle/tier3-radix/chart.spec.ts`'s own
// `frameOf`: the un-suffixed `#component-preview-frame` belongs to the
// `main` variant; every other variant's demo panel is
// `#component-preview-frame-<variant>`.
function frameOf(page: Page, variant: (typeof VARIANTS)[number]): Locator {
  const suffix = variant === "main" ? "" : `-${variant}`;
  return page.locator(`#component-preview-frame${suffix}`).first();
}

async function goto(page: Page): Promise<void> {
  await page.goto(URL, { timeout: 20 * 60 * 1000 });
}

/** Hovers the first data point's hit-band and waits for the tooltip to open. */
async function openTooltip(frame: Locator): Promise<void> {
  await frame.locator('[data-slot="chart-hit-band"]').first().hover();
  await expect(frame.locator('[data-slot="chart-tooltip"]')).toHaveAttribute("data-state", "open");
}

test.describe("render: every variant shows a labelled chart and opens its tooltip on hover", () => {
  for (const variant of VARIANTS) {
    test(variant, async ({ page }) => {
      await goto(page);
      const frame = frameOf(page, variant);
      const svg = frame.locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);
      await openTooltip(frame);
      // Every demo's fixture is 2 series (running/swimming) -- one row each.
      await expect(frame.locator('[data-slot="chart-tooltip-item"]')).toHaveCount(2);
    });
  }
});

test.describe("Axe automated scan: every variant, tooltip open", () => {
  for (const variant of VARIANTS) {
    test(variant, async ({ page }) => {
      await goto(page);
      const frame = frameOf(page, variant);
      await openTooltip(frame);
      await expectNoAxeViolations(page, `chart_tooltip ${variant}: tooltip open`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }
});

test.describe("indicator_line: a line-shaped swatch on every row", () => {
  test("the tooltip and every row swatch carry data-indicator=line", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "indicator_line");
    await openTooltip(frame);
    const tooltip = frame.locator('[data-slot="chart-tooltip"]');
    await expect(tooltip).toHaveAttribute("data-indicator", "line");
    const swatches = tooltip.locator('[data-slot="chart-swatch"]');
    await expect(swatches).toHaveCount(2);
    const count = await swatches.count();
    for (let i = 0; i < count; i++) {
      await expect(swatches.nth(i)).toHaveAttribute("data-indicator", "line");
    }
  });
});

test.describe("indicator_none: no swatch at all", () => {
  test("no chart-swatch element renders, but the row's name and value still do", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "indicator_none");
    await openTooltip(frame);
    const tooltip = frame.locator('[data-slot="chart-tooltip"]');
    await expect(tooltip.locator('[data-slot="chart-swatch"]')).toHaveCount(0);
    await expect(tooltip.locator('[data-slot="chart-tooltip-name"]')).toHaveCount(2);
    await expect(tooltip.locator('[data-slot="chart-tooltip-value"]')).toHaveCount(2);
  });
});

test.describe("label_none: no label row element", () => {
  test("no chart-tooltip-label element renders", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "label_none");
    await openTooltip(frame);
    await expect(frame.locator('[data-slot="chart-tooltip"] [data-slot="chart-tooltip-label"]')).toHaveCount(0);
  });
});

test.describe("label_custom: a fixed label instead of the per-datum date", () => {
  test("the label row shows the configured heading, not a date", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "label_custom");
    await openTooltip(frame);
    await expect(frame.locator('[data-slot="chart-tooltip-label"]')).toHaveText("Activities");
  });
});

test.describe("label_formatter: the date is spelled out in full", () => {
  test("the label row shows a full month name, not the raw ISO date", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "label_formatter");
    await openTooltip(frame);
    const label = frame.locator('[data-slot="chart-tooltip-label"]');
    await expect(label).toContainText(
      /January|February|March|April|May|June|July|August|September|October|November|December/,
    );
    await expect(label).not.toContainText("2024-07");
  });
});

test.describe("formatter: shows the custom-formatted value text", () => {
  test("rows render the custom kcal-suffixed value instead of the plain number", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "formatter");
    await openTooltip(frame);
    await expect(frame.locator('[data-slot="chart-tooltip"]')).toContainText("kcal");
  });
});

test.describe("icons: an svg icon inside every row", () => {
  test("each row shows an icon instead of the default swatch", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "icons");
    await openTooltip(frame);
    const items = frame.locator('[data-slot="chart-tooltip"] [data-slot="chart-tooltip-item"]');
    const count = await items.count();
    expect(count).toBe(2);
    for (let i = 0; i < count; i++) {
      await expect(items.nth(i).locator('[data-slot="chart-icon"] svg')).toHaveCount(1);
    }
  });
});

test.describe("advanced: a computed total row after the last series", () => {
  test("the total row renders with a summed, kcal-suffixed value", async ({ page }) => {
    await goto(page);
    const frame = frameOf(page, "advanced");
    await openTooltip(frame);
    const total = frame.locator('[data-slot="chart-tooltip"] [data-slot="chart-tooltip-total"]');
    await expect(total).toHaveCount(1);
    await expect(total).toContainText("kcal");
  });
});
