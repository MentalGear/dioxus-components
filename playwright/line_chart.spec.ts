import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=line_chart&`;

/**
 * Variant name -> its own preview frame id, per `preview/src/main.rs`'s own
 * convention (`ComponentDemo`'s `frame_id`): the `main` variant keeps the
 * bare `component-preview-frame` id every single-variant component page
 * already uses; every other variant on the same page gets
 * `component-preview-frame-<name>` appended, so they can never collide
 * (the same fix `popover`'s `non_modal` variant needed).
 *
 * Kept as an explicit map (not derived from a shared list) so a variant
 * added in a follow-up commit (`dots_colors`, `dots_custom`, `label`,
 * `label_custom`, once the primitive-side dot/label work in
 * `components/series/line.rs` lands) is a one-line addition here, visible
 * in the diff.
 */
const FRAME_IDS: Record<string, string> = {
  main: "component-preview-frame",
  linear: "component-preview-frame-linear",
  step: "component-preview-frame-step",
  multiple: "component-preview-frame-multiple",
  dots: "component-preview-frame-dots",
  interactive: "component-preview-frame-interactive",
};

function frame(page: Page, variant: keyof typeof FRAME_IDS) {
  return page.locator(`#${FRAME_IDS[variant]}`).first();
}

test.describe("Line chart", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(URL);
  });

  for (const variant of Object.keys(FRAME_IDS) as (keyof typeof FRAME_IDS)[]) {
    test(`${variant}: renders as a single accessible image with the hidden data table`, async ({
      page,
    }) => {
      const scope = frame(page, variant);
      // Attribute selector, not getByRole("img") -- this page's other
      // controls (the interactive variant's toggle buttons, the Select on
      // sibling chart pages) can render decorative icon-shaped <svg>s some
      // engines compute an implicit "img" role for too (the exact false
      // match `chart.spec.ts`'s own comment documents hitting).
      const svg = scope.locator('svg[role="img"]').first();
      await expect(svg).toBeVisible();
      await expect(svg).toHaveAccessibleName(/.+/);

      // The a11y contract every chart carries regardless of curve/dots/
      // series count: a real, always-present hidden table mirroring the
      // same data (`$S/chart-api.md`).
      await expect(scope.locator('table[data-slot="chart-data"]')).toBeAttached();
    });
  }

  test("dots: renders one chart-dot per data point", async ({ page }) => {
    // 6 months, one configured series (see that variant's own doc comment
    // for why only `desktop` is configured) -- one dot per datum.
    await expect(frame(page, "dots").locator('[data-slot="chart-dot"]')).toHaveCount(6);
  });

  test("multiple: renders one chart-line per configured series", async ({ page }) => {
    await expect(frame(page, "multiple").locator('[data-slot="chart-line"]')).toHaveCount(2);
  });

  test("linear and step: still render exactly one chart-line for their single series", async ({
    page,
  }) => {
    await expect(frame(page, "linear").locator('[data-slot="chart-line"]')).toHaveCount(1);
    await expect(frame(page, "step").locator('[data-slot="chart-line"]')).toHaveCount(1);
  });

  test("interactive: defaults to Desktop, draws one line, and switches series on click", async ({
    page,
  }) => {
    const scope = frame(page, "interactive");
    const desktopToggle = scope.getByRole("button", { name: /Desktop/ });
    const mobileToggle = scope.getByRole("button", { name: /Mobile/ });

    await expect(desktopToggle).toHaveAttribute("data-active", "true");
    await expect(mobileToggle).toHaveAttribute("data-active", "false");
    // Only the active series is ever configured/drawn (see the variant's
    // own doc comment) -- one line, never two, in either state.
    await expect(scope.locator('[data-slot="chart-line"]')).toHaveCount(1);
    await expect(scope.locator('svg[role="img"]')).toHaveAccessibleName(/Desktop/);

    await mobileToggle.click();

    await expect(mobileToggle).toHaveAttribute("data-active", "true");
    await expect(desktopToggle).toHaveAttribute("data-active", "false");
    await expect(scope.locator('[data-slot="chart-line"]')).toHaveCount(1);
    await expect(scope.locator('svg[role="img"]')).toHaveAccessibleName(/Mobile/);
  });

  test("hovering a hit-band opens the tooltip, moving away closes it", async ({ page }) => {
    const scope = frame(page, "multiple");
    const tooltip = scope.locator('[data-slot="chart-tooltip"]');

    await scope.locator('[data-slot="chart-hit-band"]').first().hover();
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip).toBeVisible();

    await page.mouse.move(0, 0);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });

  // Deferred to the follow-up commit that lands `components/series/
  // line.rs`'s active-dot enlargement (`$S/stage2-common.md`'s ownership
  // table: "active-dot enlargement on hover via `data-active`"): a
  // behavioural assertion that hovering/focusing a data point sets
  // `data-active="true"` on that point's own `[data-slot="chart-dot"]`
  // (and only that one) belongs here once that primitive feature exists.
  // Tracked in `$S/stage2-lanes.md`'s "s2-line" entry, not silently
  // dropped.

  test.describe("Axe automated scan", () => {
    for (const variant of Object.keys(FRAME_IDS) as (keyof typeof FRAME_IDS)[]) {
      test(`${variant}: has no automatically detectable a11y issues`, async ({ page }) => {
        // Scoped to this one variant's own frame -- `include` already
        // excludes the page's other variants and the vendored code-viewer
        // tab, so no `excludeRegions` is needed here (unlike the whole-page
        // scan below).
        await expectNoAxeViolations(page, `line_chart: ${variant}`, {
          include: [`#${FRAME_IDS[variant]}`],
        });
      });
    }

    test("interactive: Mobile selected has no automatically detectable a11y issues", async ({
      page,
    }) => {
      const scope = frame(page, "interactive");
      await scope.getByRole("button", { name: /Mobile/ }).click();
      await expect(scope.getByRole("button", { name: /Mobile/ })).toHaveAttribute(
        "data-active",
        "true",
      );
      await expectNoAxeViolations(page, "line_chart: interactive (Mobile selected)", {
        include: [`#${FRAME_IDS.interactive}`],
      });
    });

    test("multiple: tooltip open has no automatically detectable a11y issues", async ({
      page,
    }) => {
      const scope = frame(page, "multiple");
      await scope.locator('[data-slot="chart-hit-band"]').first().hover();
      await expect(scope.locator('[data-slot="chart-tooltip"]')).toHaveAttribute(
        "data-state",
        "open",
      );
      await expectNoAxeViolations(page, "line_chart: multiple (tooltip open)", {
        include: [`#${FRAME_IDS.multiple}`],
      });
    });

    test("whole page (every variant loaded) has no automatically detectable a11y issues", async ({
      page,
    }) => {
      await expectNoAxeViolations(page, "line_chart: whole page loaded", {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  });
});
