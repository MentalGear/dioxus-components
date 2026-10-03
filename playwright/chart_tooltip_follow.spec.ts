import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

// The chart tooltip follows the pointer the way shadcn/Recharts tooltips do
// (measured on ui.shadcn.com): x snaps to the nearest category's point, y
// tracks the pointer, 10px right of / below it, flipped to the left / above
// near the chart's right / bottom edge, kept inside the chart box, eased
// between positions but never flying in on (re)appearing. Polar charts follow
// the pointer on both axes except a pie, which anchors at the slice. A touch
// tap shows it and it stays until the next tap elsewhere; the keyboard hangs
// it off the data point; Tab-focusing the chart opens it at the first point
// (a click's own focus does not); `default_index` shows it open from the first
// render; a pointer result that lands after the tooltip closed is dropped.
//
// Motion-sensitive assertions run with `reducedMotion: "reduce"` so a
// position can be read the moment it is set (the stylesheet turns the
// transition off there -- itself asserted below); the two motion tests run
// with motion on.

const OFFSET = 10; // px, shadcn's measured gap on both axes
const TOLERANCE = 1.5; // px: half-pixel pointer rounding + border-box rounding

type Box = { x: number; y: number; width: number; height: number };

const area = `${BASE_URL}/component/?name=area_chart&`;

function chartOf(page: Page, frameSel = "#component-preview-frame"): Locator {
  return page.locator(frameSel).first().locator('.dx-chart [data-slot="chart"]').first();
}

function tooltipOf(page: Page, frameSel = "#component-preview-frame"): Locator {
  return page.locator(frameSel).first().locator('[data-slot="chart-tooltip"]').first();
}

async function box(l: Locator): Promise<Box> {
  const b = await l.boundingBox();
  if (!b) throw new Error("no bounding box");
  return b;
}

/** The chart box plus each category's hit band, measured with the chart scrolled into view. */
async function measure(page: Page, frameSel = "#component-preview-frame") {
  const chart = chartOf(page, frameSel);
  await chart.scrollIntoViewIfNeeded();
  const c = await box(chart);
  const bandLocs = page.locator(frameSel).first().locator('[data-slot="chart-hit-band"]');
  const n = await bandLocs.count();
  const bands: Box[] = [];
  const xs: number[] = [];
  for (let i = 0; i < n; i++) {
    bands.push(await box(bandLocs.nth(i)));
    // The category's own position (svg px == CSS px: the svg is drawn at its
    // measured size). Not the band's center: an area/line chart's first and
    // last points sit ON the plot's edges (Recharts' point scale), so their
    // bands, kept inside the svg, are not centered on them.
    xs.push(Number(await bandLocs.nth(i).getAttribute("data-x")));
  }
  return {
    chart: c,
    bands,
    /** A category's data-point x, relative to the chart box. */
    pointX: (i: number) => xs[i],
    plot: {
      top: bands[0].y - c.y,
      bottom: bands[0].y + bands[0].height - c.y,
    },
  };
}

/** Wait until the open tooltip sits where the follow rule says, for a pointer at (px, py) in chart coordinates. */
async function expectTooltipFollows(
  tooltip: Locator,
  chart: Box,
  pointer: { x: number; y: number },
  axes: { x: boolean; y: boolean },
  message: string,
) {
  await expect
    .poll(
      async () => {
        if ((await tooltip.getAttribute("data-state")) !== "open") return "closed";
        const t = await box(tooltip);
        const sideX = await tooltip.getAttribute("data-side-x");
        const sideY = await tooltip.getAttribute("data-side-y");
        const wantX = sideX === "left" ? pointer.x - OFFSET - t.width : pointer.x + OFFSET;
        const wantY = sideY === "top" ? pointer.y - OFFSET - t.height : pointer.y + OFFSET;
        const dx = t.x - chart.x - wantX;
        const dy = t.y - chart.y - wantY;
        return (!axes.x || Math.abs(dx) <= TOLERANCE) && (!axes.y || Math.abs(dy) <= TOLERANCE)
          ? "ok"
          : `off by dx=${dx.toFixed(1)} dy=${dy.toFixed(1)} (sides ${sideX}/${sideY})`;
      },
      { message },
    )
    .toBe("ok");
}

async function expectInside(tooltip: Locator, chart: Box, message: string) {
  await expect
    .poll(
      async () => {
        const t = await box(tooltip);
        const slack = 0.75;
        return t.x >= chart.x - slack &&
          t.y >= chart.y - slack &&
          t.x + t.width <= chart.x + chart.width + slack &&
          t.y + t.height <= chart.y + chart.height + slack
          ? "inside"
          : `outside: tooltip ${JSON.stringify(t)} chart ${JSON.stringify(chart)}`;
      },
      { message },
    )
    .toBe("inside");
}

/**
 * Page points (viewport coordinates) well inside `target`: the pointer hits it there, and still does
 * `margin` px away in every direction, so a half-pixel of coordinate rounding cannot put one outside.
 */
async function pointsOn(target: Locator, margin = 3): Promise<{ x: number; y: number }[]> {
  await target.scrollIntoViewIfNeeded();
  return target.evaluate((el, m) => {
    const r = el.getBoundingClientRect();
    const hit = (x: number, y: number) => document.elementFromPoint(x, y) === el;
    const found: { x: number; y: number }[] = [];
    const steps = 28;
    for (let i = 1; i < steps; i++) {
      for (let j = 1; j < steps; j++) {
        const x = r.left + (r.width * i) / steps;
        const y = r.top + (r.height * j) / steps;
        if (hit(x, y) && hit(x + m, y) && hit(x - m, y) && hit(x, y + m) && hit(x, y - m)) found.push({ x, y });
      }
    }
    return found;
  }, margin);
}

test.describe("Cartesian tooltip follows the pointer", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
  });

  test("x snaps to the category's point, y tracks the pointer, 10px right/below", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const px = m.pointX(1);
    for (const dy of [m.plot.top + 25, m.plot.top + 70, m.plot.top + 110]) {
      await page.mouse.move(m.chart.x + px, m.chart.y + dy);
      await expect(tooltip).toHaveAttribute("data-state", "open");
      // x is the data point's x (not the pointer's), y is the pointer's.
      await expectTooltipFollows(tooltip, m.chart, { x: px, y: dy }, { x: true, y: true }, `pointer y=${dy}`);
    }
  });

  test("x switches category at the midpoint between points and not before", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const y = m.plot.top + 40;
    const mid = (m.pointX(1) + m.pointX(2)) / 2;
    const label = tooltip.locator('[data-slot="chart-tooltip-label"]');

    await page.mouse.move(m.chart.x + m.pointX(1) - 15, m.chart.y + y);
    await expect(label).toHaveText(/./);
    const first = await label.textContent();
    await page.mouse.move(m.chart.x + mid - 2, m.chart.y + y);
    await expectTooltipFollows(tooltip, m.chart, { x: m.pointX(1), y }, { x: true, y: false }, "just before the midpoint");
    await expect(label).toHaveText(first!);

    await page.mouse.move(m.chart.x + mid + 2, m.chart.y + y);
    await expectTooltipFollows(tooltip, m.chart, { x: m.pointX(2), y }, { x: true, y: false }, "just after the midpoint");
    await expect(label).not.toHaveText(first!);
  });

  test("flips to the left of the point near the right edge, staying inside the chart", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const last = m.bands.length - 1;
    const px = m.pointX(last);
    const y = m.plot.top + 30;
    await page.mouse.move(m.chart.x + m.chart.width * 0 + px, m.chart.y + y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip).toHaveAttribute("data-side-x", "left");
    await expectTooltipFollows(tooltip, m.chart, { x: px, y }, { x: true, y: true }, "last category");
    const t = await box(tooltip);
    expect(t.x + t.width, "tooltip's right edge sits left of the point").toBeLessThan(m.chart.x + px);
    await expectInside(tooltip, m.chart, "flipped tooltip");

    // And the first category does not flip.
    await page.mouse.move(m.chart.x + m.pointX(0), m.chart.y + y);
    await expect(tooltip).toHaveAttribute("data-side-x", "right");
  });

  test("flips above the pointer near the bottom edge", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const px = m.pointX(1);
    const y = m.plot.bottom - 3;
    await page.mouse.move(m.chart.x + px, m.chart.y + y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(tooltip).toHaveAttribute("data-side-y", "top");
    await expectTooltipFollows(tooltip, m.chart, { x: px, y }, { x: true, y: true }, "near the bottom");
    await expectInside(tooltip, m.chart, "tooltip above the pointer");
  });

  test("never leaves the chart box, wherever the pointer is over the plot", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    // The plot spans the first point to the last (both ON its edges).
    const left = m.pointX(0) + 1;
    const right = m.pointX(m.bands.length - 1) - 1;
    let opened = 0;
    for (let i = 0; i <= 8; i++) {
      for (let j = 0; j <= 6; j++) {
        const x = left + ((right - left) * i) / 8;
        const y = m.plot.top + 3 + ((m.plot.bottom - m.plot.top - 6) * j) / 6;
        await page.mouse.move(m.chart.x + x, m.chart.y + y);
        await expect(tooltip).toHaveAttribute("data-state", "open");
        opened++;
        await expectInside(tooltip, m.chart, `pointer at ${x.toFixed(0)},${y.toFixed(0)}`);
      }
    }
    expect(opened).toBe(63);
  });

  test("hides over the margins and when the pointer leaves", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    await page.mouse.move(m.chart.x + m.pointX(1), m.chart.y + m.plot.top + 30);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    // Below the plot (the axis-label strip) is outside the plot: closed.
    await page.mouse.move(m.chart.x + m.pointX(1), m.chart.y + m.chart.height - 2);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
    await page.mouse.move(m.chart.x + m.pointX(1), m.chart.y + m.plot.top + 30);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await page.mouse.move(0, 0);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });

  test("keyboard: arrows hang the tooltip off the data point, flipped and clamped like the pointer's", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    await chartOf(page).focus(); // opens the first point (keyboard focus)
    await page.keyboard.press("ArrowRight"); // -> the 2nd category
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const px = m.pointX(1);
    await expect
      .poll(async () => {
        const t = await box(tooltip);
        return Math.abs(t.x - m.chart.x - (px + OFFSET)) <= TOLERANCE;
      })
      .toBe(true);
    await expectInside(tooltip, m.chart, "keyboard tooltip, 2nd category");

    await page.keyboard.press("End");
    await expect(tooltip).toHaveAttribute("data-side-x", "left");
    await expectInside(tooltip, m.chart, "keyboard tooltip, last category");

    await page.keyboard.press("Escape");
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Tooltip motion", () => {
  async function sampleTooltip(page: Page, frames: number) {
    // Records the open tooltip's left/top every animation frame.
    return page.evaluate(
      (n) =>
        new Promise<{ x: number; y: number }[]>((resolve) => {
          const tip = document.querySelector('#component-preview-frame [data-slot="chart-tooltip"]')!;
          const out: { x: number; y: number }[] = [];
          let frame = 0;
          const tick = () => {
            if (tip.getAttribute("data-state") === "open" && getComputedStyle(tip).visibility !== "hidden") {
              const r = tip.getBoundingClientRect();
              out.push({ x: r.x, y: r.y });
            }
            if (++frame < n) requestAnimationFrame(tick);
            else resolve(out);
          };
          requestAnimationFrame(tick);
        }),
      frames,
    );
  }

  test("a tooltip that (re)appears is at its target on its first frame -- no fly-in", async ({ page }) => {
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    for (const round of [0, 1]) {
      await page.mouse.move(0, 0);
      await expect(tooltip).toHaveAttribute("data-state", "closed");
      const sampled = sampleTooltip(page, 30);
      await page.mouse.move(m.chart.x + m.pointX(2), m.chart.y + m.plot.top + 60);
      const samples = await sampled;
      expect(samples.length, `round ${round}: sampled while open`).toBeGreaterThan(3);
      const final = samples[samples.length - 1];
      for (const s of samples) {
        expect(Math.abs(s.x - final.x), `round ${round}: x`).toBeLessThanOrEqual(TOLERANCE);
        expect(Math.abs(s.y - final.y), `round ${round}: y`).toBeLessThanOrEqual(TOLERANCE);
      }
    }
  });

  test("moving between categories eases rather than jumping", async ({ page }) => {
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const y = m.plot.top + 60;
    await page.mouse.move(m.chart.x + m.pointX(0), m.chart.y + y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect.poll(async () => (await box(tooltip)).x).toBeGreaterThan(0);
    await page.waitForTimeout(400); // settle the first position
    const from = (await box(tooltip)).x;
    const sampled = sampleTooltip(page, 40);
    await page.mouse.move(m.chart.x + m.pointX(2), m.chart.y + y);
    const samples = await sampled;
    const to = samples[samples.length - 1].x;
    expect(to - from, "it did move").toBeGreaterThan(20);
    const between = samples.filter((s) => s.x > from + 2 && s.x < to - 2);
    expect(between.length, "intermediate frames between the two positions").toBeGreaterThan(1);
    const duration = await tooltip.evaluate((el) => getComputedStyle(el).transitionDuration);
    expect(duration).not.toBe("0s");
  });

  test("prefers-reduced-motion turns the transition off", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    // Open it and move it twice, so motion is as enabled as it ever gets.
    await page.mouse.move(m.chart.x + m.pointX(1), m.chart.y + m.plot.top + 40);
    await page.mouse.move(m.chart.x + m.pointX(2), m.chart.y + m.plot.top + 70);
    await expect(tooltip).toHaveAttribute("data-motion", "true");
    const duration = await tooltip.evaluate((el) => getComputedStyle(el).transitionDuration);
    expect(duration).toMatch(/^0(\.0*1?)?m?s$/);
  });
});

test.describe("Horizontal bars follow the pointer along x and snap along y", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, `${BASE_URL}/component/?name=bar_chart&`, { timeout: 20 * 60 * 1000 });
  });

  test("x is the pointer's, y snaps to the row", async ({ page }) => {
    const frame = "#component-preview-frame-horizontal";
    const m = await measure(page, frame);
    const tooltip = tooltipOf(page, frame);
    const rowY = (i: number) => m.bands[i].y + m.bands[i].height / 2 - m.chart.y;
    const x0 = m.bands[0].x - m.chart.x + 40;
    await page.mouse.move(m.chart.x + x0, m.chart.y + rowY(1) - 4);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expectTooltipFollows(tooltip, m.chart, { x: x0, y: rowY(1) }, { x: true, y: true }, "row 1 at x0");
    const x1 = x0 + 60;
    await page.mouse.move(m.chart.x + x1, m.chart.y + rowY(1) + 4);
    await expectTooltipFollows(tooltip, m.chart, { x: x1, y: rowY(1) }, { x: true, y: true }, "row 1, moved right");
    await expectInside(tooltip, m.chart, "horizontal bar tooltip");
  });
});

test.describe("Pie tooltip", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, `${BASE_URL}/component/?name=pie_chart&`, { timeout: 20 * 60 * 1000 });
  });

  test("shows the slice's name, value and color -- not the series label", async ({ page }) => {
    const frame = page.locator("#component-preview-frame").first();
    const tooltip = tooltipOf(page);
    const slices = [
      ["Chrome", "275"],
      ["Safari", "200"],
      ["Firefox", "187"],
    ];
    for (let i = 0; i < slices.length; i++) {
      const arc = frame.locator(`[data-slot="chart-arc"][data-index="${i}"]`);
      const pts = await pointsOn(arc);
      expect(pts.length, `slice ${i} is hit-testable`).toBeGreaterThan(2);
      await page.mouse.move(pts[Math.floor(pts.length / 2)].x, pts[Math.floor(pts.length / 2)].y);
      await expect(tooltip).toHaveAttribute("data-state", "open");
      await expect(tooltip.locator('[data-slot="chart-tooltip-name"]')).toHaveText(slices[i][0]);
      await expect(tooltip.locator('[data-slot="chart-tooltip-value"]')).toHaveText(slices[i][1]);
      await expect(tooltip).not.toContainText("Visitors");
      const [swatch, fill] = await Promise.all([
        tooltip.locator('[data-slot="chart-swatch"]').evaluate((el) => getComputedStyle(el).backgroundColor),
        arc.evaluate((el) => getComputedStyle(el).fill),
      ]);
      expect(swatch, `slice ${i}: swatch is the slice color`).toBe(fill);
    }
  });

  test("anchors at the slice and does not follow the pointer within it", async ({ page }) => {
    const frame = page.locator("#component-preview-frame").first();
    const tooltip = tooltipOf(page);
    const arc = frame.locator('[data-slot="chart-arc"][data-index="0"]');
    const pts = await pointsOn(arc);
    const chart = await box(chartOf(page));
    const a = pts[Math.floor(pts.length * 0.25)];
    const b = pts[Math.floor(pts.length * 0.75)];
    expect(Math.hypot(a.x - b.x, a.y - b.y), "two distinct points on the slice").toBeGreaterThan(15);
    await page.mouse.move(a.x, a.y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expectInside(tooltip, chart, "pie tooltip");
    const first = await box(tooltip);
    await page.mouse.move(b.x, b.y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const second = await box(tooltip);
    expect(Math.abs(first.x - second.x)).toBeLessThanOrEqual(TOLERANCE);
    expect(Math.abs(first.y - second.y)).toBeLessThanOrEqual(TOLERANCE);

    // A different slice anchors elsewhere.
    const other = (await pointsOn(frame.locator('[data-slot="chart-arc"][data-index="3"]')))[0];
    await page.mouse.move(other.x, other.y);
    await expect
      .poll(async () => {
        const t = await box(tooltip);
        return Math.hypot(t.x - first.x, t.y - first.y) > 20;
      })
      .toBe(true);
  });

  test("closes when the pointer leaves the ring", async ({ page }) => {
    const tooltip = tooltipOf(page);
    const arc = page.locator('#component-preview-frame [data-slot="chart-arc"]').first();
    const [p] = await pointsOn(arc);
    await page.mouse.move(p.x, p.y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await page.mouse.move(0, 0);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Radar and radial tooltips follow the pointer", () => {
  test("radar: both axes follow the pointer", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, `${BASE_URL}/component/?name=radar_chart&`, { timeout: 20 * 60 * 1000 });
    const tooltip = tooltipOf(page);
    const wedge = page.locator('#component-preview-frame [data-slot="chart-hit-band"]').nth(1);
    const pts = await pointsOn(wedge);
    const chart = await box(chartOf(page)); // after pointsOn: it may have scrolled the page
    const a = pts[Math.floor(pts.length * 0.2)];
    const b = pts[Math.floor(pts.length * 0.8)];
    for (const p of [a, b]) {
      await page.mouse.move(p.x, p.y);
      await expectTooltipFollows(tooltip, chart, { x: p.x - chart.x, y: p.y - chart.y }, { x: true, y: true }, "radar pointer");
      await expectInside(tooltip, chart, "radar tooltip");
    }
  });

  test("radial: shows the ring's datum and follows the pointer", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, `${BASE_URL}/component/?name=radial_chart&`, { timeout: 20 * 60 * 1000 });
    const tooltip = tooltipOf(page);
    const arc = page.locator('#component-preview-frame [data-slot="chart-arc"][data-index="0"]');
    const pts = await pointsOn(arc);
    const chart = await box(chartOf(page)); // after pointsOn: it may have scrolled the page
    const a = pts[Math.floor(pts.length * 0.2)];
    const b = pts[Math.floor(pts.length * 0.8)];
    for (const p of [a, b]) {
      await page.mouse.move(p.x, p.y);
      await expectTooltipFollows(tooltip, chart, { x: p.x - chart.x, y: p.y - chart.y }, { x: true, y: true }, "radial pointer");
      await expectInside(tooltip, chart, "radial tooltip");
    }
    await expect(tooltip.locator('[data-slot="chart-tooltip-name"]')).toHaveText("Chrome");
  });
});

/** Records every data-state change of the first tooltip in `frameSel` from now on. */
async function recordStates(page: Page, frameSel = "#component-preview-frame") {
  await page.evaluate((sel) => {
    const tip = document.querySelector(`${sel} [data-slot="chart-tooltip"]`)!;
    (window as any).__tipStates = [];
    new MutationObserver(() => (window as any).__tipStates.push(tip.getAttribute("data-state"))).observe(tip, {
      attributes: true,
      attributeFilter: ["data-state"],
    });
  }, frameSel);
  return () => page.evaluate(() => (window as any).__tipStates as string[]);
}

test.describe("Polar tooltips do not blink while hovering", () => {
  // Regression: hover used to ride on pointerenter/leave of the arcs, whose
  // active affordance (`scale(1.05)`) moves the very hit target under the
  // pointer, so a pointer near an arc's edge flipped open/closed in a loop.
  // Hover is now resolved from coordinates against the resting geometry.
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
  });

  test("pie: sweeping a full circle across every slice boundary opens once and never closes", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=pie_chart&`, { timeout: 20 * 60 * 1000 });
    await chartOf(page).scrollIntoViewIfNeeded();
    const c = await box(chartOf(page));
    const states = await recordStates(page);
    const cx = c.x + c.width / 2;
    const cy = c.y + c.height / 2;
    const r = (Math.min(c.width, c.height) / 2) * 0.55;
    for (let i = 0; i <= 90; i++) {
      const a = (i / 90) * Math.PI * 2;
      await page.mouse.move(cx + r * Math.sin(a), cy - r * Math.cos(a));
    }
    await page.waitForTimeout(200);
    expect(await states(), "one open, never a close").toEqual(["open"]);
  });

  test("radial: wandering over one ring's edges never closes the tooltip", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=radial_chart&`, { timeout: 20 * 60 * 1000 });
    const arc = page.locator('#component-preview-frame [data-slot="chart-arc"][data-index="0"]');
    const pts = await pointsOn(arc);
    expect(pts.length).toBeGreaterThan(40);
    const states = await recordStates(page);
    for (let i = 0; i < pts.length; i += 3) await page.mouse.move(pts[i].x, pts[i].y);
    await page.waitForTimeout(200);
    expect(await states()).toEqual(["open"]);
  });
});

test.describe("First open", () => {
  test("a tooltip opened for the first time near the right edge does not move after its first frame", async ({ page }) => {
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const last = m.bands.length - 1;
    for (const i of [last - 1, last]) {
      await page.mouse.move(0, 0);
      await expect(tooltipOf(page)).toHaveAttribute("data-state", "closed");
      const sampled = page.evaluate(
        () =>
          new Promise<{ x: number; y: number }[]>((resolve) => {
            const tip = document.querySelector('#component-preview-frame [data-slot="chart-tooltip"]')!;
            const out: { x: number; y: number }[] = [];
            let n = 0;
            const tick = () => {
              if (tip.getAttribute("data-state") === "open" && getComputedStyle(tip).visibility !== "hidden") {
                const r = tip.getBoundingClientRect();
                out.push({ x: r.x, y: r.y });
              }
              if (++n < 40) requestAnimationFrame(tick);
              else resolve(out);
            };
            requestAnimationFrame(tick);
          }),
      );
      await page.mouse.move(m.chart.x + m.pointX(i), m.chart.y + m.plot.top + 50);
      const samples = await sampled;
      const final = samples[samples.length - 1];
      expect(samples.length).toBeGreaterThan(3);
      for (const smp of samples) {
        expect(Math.abs(smp.x - final.x), `category ${i}: x`).toBeLessThanOrEqual(TOLERANCE);
      }
    }
  });
});

test.describe("Narrow chart (phone)", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test("where neither side has room the tooltip pins beside its point on the roomier side, inside the chart", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    expect(m.chart.width, "the chart is narrower than ~2 tooltips").toBeLessThan(330);
    for (let i = 0; i < m.bands.length; i++) {
      await page.mouse.move(m.chart.x + m.pointX(i), m.chart.y + m.plot.top + 40);
      await expect(tooltip).toHaveAttribute("data-state", "open");
      await expectInside(tooltip, m.chart, `category ${i}`);
      const t = await box(tooltip);
      const px = m.chart.x + m.pointX(i);
      // Beside the point, not over it (a tooltip wider than either side could be pinned over it).
      const room = Math.max(px - m.chart.x, m.chart.x + m.chart.width - px) - OFFSET;
      if (room >= t.width) expect(t.x >= px || t.x + t.width <= px, `category ${i}: not over its point`).toBe(true);
    }
  });
});

test.describe("Touch", () => {
  test.use({ hasTouch: true, viewport: { width: 390, height: 844 } });

  test("a tap shows the tooltip, it stays after the finger lifts, and a tap elsewhere closes it", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const label = tooltip.locator('[data-slot="chart-tooltip-label"]');
    const y = m.chart.y + m.plot.top + 50;

    await page.touchscreen.tap(m.chart.x + m.pointX(1), y);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const first = await label.textContent();
    await expectInside(tooltip, m.chart, "tapped tooltip");
    await page.waitForTimeout(500);
    await expect(tooltip, "stays after the finger lifted").toHaveAttribute("data-state", "open");

    await page.touchscreen.tap(m.chart.x + m.pointX(3), y);
    await expect(label).not.toHaveText(first!);
    await expect(tooltip).toHaveAttribute("data-state", "open");

    // Tapping outside the chart (the card's own title) closes it.
    await page.locator("#component-preview-frame .dx-card-title").first().tap();
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });

  test("dragging a finger across the chart scrubs the tooltip between categories", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    const label = tooltip.locator('[data-slot="chart-tooltip-label"]');
    const cdp = await page.context().newCDPSession(page);
    const y = m.chart.y + m.plot.top + 50;
    const touch = (type: string, x: number) =>
      cdp.send("Input.dispatchTouchEvent", {
        type,
        touchPoints: type === "touchEnd" ? [] : [{ x, y, id: 1 }],
      });

    await touch("touchStart", m.chart.x + m.pointX(0));
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const seen = new Set<string>();
    seen.add((await label.textContent())!);
    for (let i = 1; i < m.bands.length; i++) {
      const target = m.chart.x + m.pointX(i);
      const from = m.chart.x + m.pointX(i - 1);
      for (let s = 1; s <= 4; s++) await touch("touchMove", from + ((target - from) * s) / 4);
      await expect.poll(async () => (await label.textContent())!).not.toBe([...seen].pop());
      seen.add((await label.textContent())!);
    }
    await touch("touchEnd", 0);
    expect(seen.size, "every category was visited during the drag").toBe(m.bands.length);
    await page.waitForTimeout(300);
    await expect(tooltip, "stays after lifting").toHaveAttribute("data-state", "open");
  });
});

const tooltipsGallery = `${BASE_URL}/component/?name=chart_tooltip&`;

/** Every chart_tooltip demo, as `[variant, preview-frame selector]` (`main` owns the un-suffixed frame). */
const TOOLTIP_VARIANTS = [
  "main",
  "indicator_line",
  "indicator_none",
  "label_none",
  "label_custom",
  "label_formatter",
  "formatter",
  "icons",
  "advanced",
].map((v) => [v, v === "main" ? "#component-preview-frame" : `#component-preview-frame-${v}`] as const);

test.describe("A late pointer result never reopens a closed tooltip", () => {
  test("a pointermove and a pointerleave in one task leave it closed, every time", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    await chartOf(page).scrollIntoViewIfNeeded();
    // The rect read behind a pointermove is async; a close in the same task
    // used to be overtaken by it (30/30 stuck open before the fix).
    const stuck = await page.evaluate(async () => {
      const wrapper = document.querySelector('#component-preview-frame [data-slot="chart"]')!;
      const svg = wrapper.querySelector("svg")!;
      const tip = wrapper.parentElement!.querySelector('[data-slot="chart-tooltip"]')!;
      const r = wrapper.getBoundingClientRect();
      let stuck = 0;
      for (let i = 0; i < 30; i++) {
        const o = { bubbles: true, clientX: r.x + 60 + i * 8, clientY: r.y + 60, pointerType: "mouse", pointerId: 1, isPrimary: true };
        svg.dispatchEvent(new PointerEvent("pointermove", o));
        wrapper.dispatchEvent(new PointerEvent("pointerleave", { ...o, bubbles: false, clientX: r.x - 50 }));
        await new Promise((resolve) => setTimeout(resolve, 80));
        if (tip.getAttribute("data-state") === "open") stuck++;
      }
      return stuck;
    });
    expect(stuck, "tooltip stuck open after move+leave").toBe(0);
  });

  test("a real hover afterwards still opens it", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    await page.evaluate(() => {
      const wrapper = document.querySelector('#component-preview-frame [data-slot="chart"]')!;
      const r = wrapper.getBoundingClientRect();
      const o = { bubbles: true, clientX: r.x + 80, clientY: r.y + 60, pointerType: "mouse", pointerId: 1, isPrimary: true };
      wrapper.querySelector("svg")!.dispatchEvent(new PointerEvent("pointermove", o));
      wrapper.dispatchEvent(new PointerEvent("pointerleave", { ...o, bubbles: false }));
    });
    await page.mouse.move(m.chart.x + m.pointX(1), m.chart.y + m.plot.top + 40);
    await expect(tooltipOf(page)).toHaveAttribute("data-state", "open");
  });
});

test.describe("defaultIndex: the tooltip starts open", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, tooltipsGallery, { timeout: 20 * 60 * 1000 });
  });

  test("every tooltip demo shows its tooltip, visible and inside its chart, with no interaction", async ({ page }) => {
    for (const [variant, frame] of TOOLTIP_VARIANTS) {
      const tooltip = tooltipOf(page, frame);
      await tooltip.scrollIntoViewIfNeeded();
      await expect(tooltip, `${variant}: open on load`).toHaveAttribute("data-state", "open");
      await expect(tooltip, `${variant}: placed once measured`).toHaveAttribute("data-placed", "true");
      await expect(tooltip, `${variant}: visible`).toBeVisible();
      await expectInside(tooltip, await box(chartOf(page, frame)), `${variant}: default tooltip`);
    }
  });

  test("it hangs off the default data point with the usual offset and flip rules", async ({ page }) => {
    const frame = "#component-preview-frame";
    const m = await measure(page, frame);
    const tooltip = tooltipOf(page, frame);
    await expect(tooltip).toHaveAttribute("data-placed", "true");
    // `default_index: 1` in the demos (shadcn's `defaultIndex={1}`): x is
    // that category's point + 10px.
    await expectTooltipFollows(tooltip, m.chart, { x: m.pointX(1), y: 0 }, { x: true, y: false }, "default index 1");
    await expect(tooltip.locator('[data-slot="chart-tooltip-label"]')).toHaveText(/./);
  });

  test("the first hover takes over; once it closes it stays closed", async ({ page }) => {
    const frame = "#component-preview-frame";
    const m = await measure(page, frame);
    const tooltip = tooltipOf(page, frame);
    const label = tooltip.locator('[data-slot="chart-tooltip-label"]');
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const initial = await label.textContent();
    await page.mouse.move(m.chart.x + m.pointX(4), m.chart.y + m.plot.top + 40);
    await expect(label).not.toHaveText(initial!);
    await page.mouse.move(0, 0);
    await expect(tooltip).toHaveAttribute("data-state", "closed");
    await page.waitForTimeout(300);
    await expect(tooltip, "the default is not re-applied").toHaveAttribute("data-state", "closed");
  });

  test("the server render already has it open (before the wasm loads)", async ({ request }) => {
    // Plain HTTP, no browser: what an SSG/SSR host sends before any wasm runs.
    const html = await (await request.get(`${BASE_URL}/component/chart_tooltip/`)).text();
    test.skip(!html.includes('data-slot="chart-tooltip"'), "a client-rendered dev server sends no chart markup");
    const tooltips = (html.match(/<div[^>]*>/g) ?? []).filter((tag) => tag.includes('data-slot="chart-tooltip"'));
    expect(tooltips.length, "every tooltip demo is on the page").toBeGreaterThanOrEqual(TOOLTIP_VARIANTS.length);
    expect(tooltips.filter((tag) => tag.includes('data-state="open"')).length, "and each one is open").toBe(tooltips.length);
  });
});

test.describe("Active dot on the hovered point", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
  });

  const dotsOf = (page: Page, frame: string) => page.locator(frame).first().locator('[data-slot="chart-active-dot"]');

  test("area: one r=4 dot per series at the hovered category, ringed, gone when the pointer leaves", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=area_chart&`, { timeout: 20 * 60 * 1000 });
    const frame = "#component-preview-frame-stacked"; // two series
    const m = await measure(page, frame);
    const dots = dotsOf(page, frame);
    await expect(dots).toHaveCount(0);
    await page.mouse.move(m.chart.x + m.pointX(2), m.chart.y + m.plot.top + 50);
    await expect(dots).toHaveCount(2);
    for (let s = 0; s < 2; s++) {
      const dot = dots.nth(s);
      const d = await box(dot);
      // r = 4 plus a 2px ring (stroke centred on the edge): ~10px across, at least 8.
      expect(d.width, `series ${s}: dot size`).toBeGreaterThanOrEqual(8);
      expect(d.width).toBeLessThanOrEqual(12);
      expect(Math.abs(d.x + d.width / 2 - (m.chart.x + m.pointX(2))), `series ${s}: on the category's point`).toBeLessThanOrEqual(1.5);
      const style = await dot.evaluate((el) => {
        const cs = getComputedStyle(el);
        return { fill: cs.fill, stroke: cs.stroke, strokeWidth: cs.strokeWidth, r: el.getAttribute("r") };
      });
      expect(style.r).toBe("4");
      expect(style.fill, `series ${s}: series color`).not.toMatch(/^(none|rgb\(0, 0, 0\))$/);
      expect(parseFloat(style.strokeWidth), "ring").toBeCloseTo(2, 0);
      expect(style.stroke, "ring is the card surface, not the series color").not.toBe(style.fill);
    }
    // The two series' dots sit at different heights (their own values).
    const [a, b] = [await box(dots.nth(0)), await box(dots.nth(1))];
    expect(Math.abs(a.y - b.y)).toBeGreaterThan(2);
    await page.mouse.move(0, 0);
    await expect(dots).toHaveCount(0);
  });

  test("line: a dot on the hovered point; none beside it; a chart with its own dots keeps its own", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=line_chart&`, { timeout: 20 * 60 * 1000 });
    const frame = "#component-preview-frame";
    const m = await measure(page, frame);
    const dots = dotsOf(page, frame);
    await page.mouse.move(m.chart.x + m.pointX(3), m.chart.y + m.plot.top + 50);
    await expect(dots).toHaveCount(1);
    const d = await box(dots.first());
    expect(Math.abs(d.x + d.width / 2 - (m.chart.x + m.pointX(3)))).toBeLessThanOrEqual(1.5);
    // `dots` variant draws its own dots (enlarged when active): no extra layer.
    const own = "#component-preview-frame-dots";
    const mo = await measure(page, own);
    await page.mouse.move(mo.chart.x + mo.pointX(3), mo.chart.y + mo.plot.top + 50);
    await expect(page.locator(own).locator('[data-slot="chart-dot"][data-active="true"]')).toHaveCount(1);
    await expect(dotsOf(page, own)).toHaveCount(0);
  });

  test("keyboard stepping moves the dot with the cursor", async ({ page }) => {
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
    const m = await measure(page);
    await chartOf(page).focus();
    const dot = dotsOf(page, "#component-preview-frame");
    await expect(dot).toHaveCount(1);
    await page.keyboard.press("ArrowRight");
    await expect
      .poll(async () => Math.abs((await box(dot.first())).x + (await box(dot.first())).width / 2 - (m.chart.x + m.pointX(1))))
      .toBeLessThanOrEqual(1.5);
  });
});

test.describe("Keyboard focus opens the first point", () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await gotoHydrated(page, area, { timeout: 20 * 60 * 1000 });
  });

  test("Tab into the chart shows the first point at once; Tab away closes it", async ({ page }) => {
    const m = await measure(page);
    const tooltip = tooltipOf(page);
    // Reach the chart the way a keyboard user does: Tab until it holds focus.
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    const chart = chartOf(page);
    let focused = false;
    // (the docs page's sidebar alone is ~85 tab stops ahead of the chart)
    for (let i = 0; i < 200 && !focused; i++) {
      await page.keyboard.press("Tab");
      focused = await chart.evaluate((el) => el === document.activeElement);
    }
    expect(focused, "Tab reaches the chart").toBe(true);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expectTooltipFollows(tooltip, m.chart, { x: m.pointX(0), y: 0 }, { x: true, y: false }, "first point");
    await page.keyboard.press("Tab");
    await expect(tooltip).toHaveAttribute("data-state", "closed");
  });

  test("a click's own focus does not jump to the first point", async ({ page }) => {
    const tooltip = tooltipOf(page);
    const label = tooltip.locator('[data-slot="chart-tooltip-label"]');
    // The first point's label, from the keyboard.
    await chartOf(page).focus();
    await expect(tooltip).toHaveAttribute("data-state", "open");
    const first = await label.textContent();
    await page.keyboard.press("Tab");
    await expect(tooltip).toHaveAttribute("data-state", "closed");
    // Tab moved focus on and may have scrolled the page: measure again.
    const m = await measure(page);
    await page.evaluate(() => {
      const label = document.querySelector('#component-preview-frame [data-slot="chart-tooltip"]')!;
      (window as any).__labels = [];
      new MutationObserver(() => (window as any).__labels.push(label.textContent)).observe(label, {
        childList: true,
        subtree: true,
        characterData: true,
        attributes: true,
        attributeFilter: ["data-state"],
      });
    });
    await page.mouse.click(m.chart.x + m.pointX(3), m.chart.y + m.plot.top + 50);
    await expect(tooltip).toHaveAttribute("data-state", "open");
    await expect(label).not.toHaveText(first!);
    await page.waitForTimeout(150);
    const seen = (await page.evaluate(() => (window as any).__labels as string[])).filter((t) => t.includes(first!));
    expect(seen, "the first point never flashed while the click was handled").toEqual([]);
  });
});
