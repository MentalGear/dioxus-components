import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=bubble&`;
const GOTO = { timeout: 20 * 60 * 1000 };

// Scope to a demo frame. The component page also renders each demo's own source
// in a syntax-highlighted code viewer, so every visible string appears twice.
const main = (page: Page) => page.locator("#component-preview-frame").first();
const variant = (page: Page, name: string) => page.locator(`#component-preview-frame-${name}`).first();
const content = (bubble: Locator) => bubble.locator('[data-slot="bubble-content"]');

// The component's stylesheet is linked after mount: wait until it has applied
// before any test reads a computed style or a position.
async function open(page: Page) {
  await page.goto(URL, GOTO);
  await expect
    .poll(() => content(main(page).locator('[data-variant="default"]')).evaluate((el) => getComputedStyle(el).backgroundColor), {
      timeout: 60_000,
    })
    .not.toBe("rgba(0, 0, 0, 0)");
}

const VARIANTS = ["default", "secondary", "muted", "tinted", "outline", "destructive", "ghost"];

test("the main demo renders all seven variants, each bubble holding its content", async ({ page }) => {
  await open(page);
  const bubbles = main(page).locator('[data-slot="bubble"]');
  await expect(bubbles).toHaveCount(7);
  for (let i = 0; i < VARIANTS.length; i++) {
    await expect(bubbles.nth(i)).toHaveAttribute("data-variant", VARIANTS[i]);
    await expect(content(bubbles.nth(i))).toBeVisible();
  }
  // The tinted one is the end-aligned bubble in the main demo.
  await expect(bubbles.nth(3)).toHaveAttribute("data-align", "end");
});

test("a bubble sizes to its content up to 80% of the row; ghost spans the row", async ({ page }) => {
  await open(page);
  const frame = main(page);
  const widths = await frame.evaluate((root) => {
    const bubbles = Array.from(root.querySelectorAll<HTMLElement>('[data-slot="bubble"]'));
    const wrapper = bubbles[0].parentElement!;
    return {
      row: wrapper.getBoundingClientRect().width,
      bubbles: bubbles.map((b) => b.getBoundingClientRect().width),
    };
  });
  for (const w of widths.bubbles.slice(0, 6)) expect(w).toBeLessThanOrEqual(widths.row * 0.8 + 1);
  // ghost: no max-width, so it takes the full row.
  expect(widths.bubbles[6]).toBeGreaterThan(widths.row * 0.95);
});

test("alignment: an end bubble hugs the end of the row, a start bubble the start", async ({ page }) => {
  await open(page);
  const bubbles = variant(page, "alignment").locator('[data-slot="bubble"]');
  await expect(bubbles).toHaveCount(2);
  const rects = await bubbles.evaluateAll((els) => {
    const row = els[0].parentElement!.getBoundingClientRect();
    return { row: { left: row.left, right: row.right }, boxes: els.map((e) => e.getBoundingClientRect()).map((r) => ({ left: r.left, right: r.right })) };
  });
  expect(Math.abs(rects.boxes[0].left - rects.row.left)).toBeLessThan(1);
  expect(Math.abs(rects.boxes[1].right - rects.row.right)).toBeLessThan(1);
});

test("alignment mirrors in RTL (logical start/end)", async ({ page }) => {
  await open(page);
  const bubbles = variant(page, "alignment").locator('[data-slot="bubble"]');
  await expect(bubbles).toHaveCount(2);
  const rects = await bubbles.evaluateAll((els) => {
    const row = els[0].parentElement!;
    row.setAttribute("dir", "rtl");
    const r = row.getBoundingClientRect();
    return { row: { left: r.left, right: r.right }, boxes: els.map((e) => e.getBoundingClientRect()).map((b) => ({ left: b.left, right: b.right })) };
  });
  // dir=rtl: start is the right edge, end the left edge.
  expect(Math.abs(rects.boxes[0].right - rects.row.right)).toBeLessThan(1);
  expect(Math.abs(rects.boxes[1].left - rects.row.left)).toBeLessThan(1);
});

test("group stacks consecutive bubbles in a column", async ({ page }) => {
  await open(page);
  const group = variant(page, "group").locator('[data-slot="bubble-group"]').first();
  const bubbles = group.locator('[data-slot="bubble"]');
  await expect(bubbles).toHaveCount(3);
  const tops = await bubbles.evaluateAll((els) => els.map((e) => e.getBoundingClientRect().top));
  expect(tops[0]).toBeLessThan(tops[1]);
  expect(tops[1]).toBeLessThan(tops[2]);
  expect(await group.evaluate((el) => getComputedStyle(el).flexDirection)).toBe("column");
});

test("reactions: static rows are labelled images; placement follows side and align", async ({ page }) => {
  await open(page);
  const demo = variant(page, "reactions");
  const first = demo.locator('[data-slot="bubble-reactions"]').first();
  await expect(first).toHaveAttribute("role", "img");
  await expect(first).toHaveAttribute("aria-label", /Reactions: thumbs up, surprised, fire, and 8 more/);
  await expect(demo.getByRole("img", { name: /Reaction: fire/ })).toBeVisible();

  // bottom/end hangs off the bottom edge near the end; top/start rises above the top edge.
  const geo = await demo.evaluate((frame) => {
    const rows = Array.from(frame.querySelectorAll<HTMLElement>('[data-slot="bubble-reactions"]'));
    return rows.map((r) => {
      const b = r.closest<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
      const x = r.getBoundingClientRect();
      return { side: r.dataset.side, align: r.dataset.align, belowBottom: x.bottom - b.bottom, aboveTop: b.top - x.top, fromStart: x.left - b.left, fromEnd: b.right - x.right };
    });
  });
  const bottomEnd = geo[0];
  expect(bottomEnd.side).toBe("bottom");
  expect(bottomEnd.align).toBe("end");
  expect(bottomEnd.belowBottom).toBeGreaterThan(0); // overlaps and hangs below
  expect(bottomEnd.fromEnd).toBeLessThan(bottomEnd.fromStart);
  const topStart = geo[2];
  expect(topStart.side).toBe("top");
  expect(topStart.align).toBe("start");
  expect(topStart.aboveTop).toBeGreaterThan(0);
  expect(topStart.fromStart).toBeLessThan(topStart.fromEnd);
});

test("interactive reactions are real buttons with names, operable by keyboard", async ({ page }) => {
  await open(page);
  const demo = variant(page, "reactions");
  const up = demo.getByRole("button", { name: "Thumbs up" });
  const down = demo.getByRole("button", { name: "Thumbs down" });
  await expect(up).toBeVisible();
  await up.focus();
  await page.keyboard.press("Enter");
  await expect(demo.getByRole("status")).toHaveText("You agree");
  await page.keyboard.press("Tab");
  await expect(down).toBeFocused();
  await page.keyboard.press("Space");
  await expect(demo.getByRole("status")).toHaveText("You disagree");
});

test("as: a bubble can be a real button or link, focusable with a visible ring", async ({ page }) => {
  await open(page);
  const demo = variant(page, "link_button");
  const button = demo.getByRole("button", { name: "I forgot my password" });
  const link = demo.getByRole("link", { name: "Open the reset guide" });
  await expect(button).toHaveAttribute("data-slot", "bubble-content");
  await expect(link).toHaveAttribute("data-slot", "bubble-content");

  await button.click();
  await expect(demo.getByText("Sent 1 time(s)")).toBeVisible();
  await button.focus();
  await page.keyboard.press("Enter");
  await expect(demo.getByText("Sent 2 time(s)")).toBeVisible();

  await link.focus();
  await expect(link).toBeFocused();
  const ring = await link.evaluate((el) => getComputedStyle(el).boxShadow);
  expect(ring).not.toBe("none");
});

for (const scheme of ["light", "dark"] as const) {
  test.describe(`${scheme} colour roles`, () => {
    test.beforeEach(async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await open(page);
      // The emulation must reach the theme's `--light`/`--dark` toggle, or the
      // dark half of this block silently re-tests light.
      expect(await page.evaluate(() => matchMedia("(prefers-color-scheme: dark)").matches)).toBe(scheme === "dark");
    });

    // Reads each variant's resolved colours next to the role tokens they should
    // come from, normalised to sRGB bytes through a 1x1 canvas so no
    // colour-function syntax (color-mix, oklch) is parsed by hand.
    async function read(page: Page) {
      // The component's stylesheet is linked after mount: wait until it has applied.
      await expect
        .poll(() => content(main(page).locator('[data-variant="default"]')).evaluate((el) => getComputedStyle(el).backgroundColor), {
          timeout: 60_000,
        })
        .not.toBe("rgba(0, 0, 0, 0)");
      return main(page).evaluate((frame) => {
        const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
        // A translucent fill (the destructive tint) is composited over `under`,
        // the surface the bubble sits on, before it is measured.
        const rgba = (css: string, under = "#000") => {
          ctx.clearRect(0, 0, 1, 1);
          ctx.fillStyle = "#000";
          ctx.fillStyle = under;
          ctx.fillRect(0, 0, 1, 1);
          ctx.fillStyle = css;
          ctx.fillRect(0, 0, 1, 1);
          return Array.from(ctx.getImageData(0, 0, 1, 1).data).slice(0, 3);
        };
        const lum = ([r, g, b]: number[]) => {
          const f = (c: number) => ((c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
          return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
        };
        const role = (token: string) => {
          const el = document.createElement("span");
          el.style.color = `var(${token})`;
          frame.appendChild(el);
          const v = getComputedStyle(el).color;
          el.remove();
          return rgba(v);
        };
        const surface = `rgb(${role("--dx-background").join(",")})`;
        const out: Record<string, any> = {};
        frame.querySelectorAll<HTMLElement>('[data-slot="bubble"]').forEach((b) => {
          const c = b.querySelector<HTMLElement>('[data-slot="bubble-content"]')!;
          const s = getComputedStyle(c);
          const bg = rgba(s.backgroundColor, surface);
          const fg = rgba(s.color);
          const [hi, lo] = [Math.max(lum(bg), lum(fg)), Math.min(lum(bg), lum(fg))];
          out[b.dataset.variant!] = { bg, fg, border: rgba(s.borderTopColor), bgAlpha: s.backgroundColor, contrast: (hi + 0.05) / (lo + 0.05), bgLum: lum(bg) };
        });
        out.roles = {
          primary: role("--dx-primary"),
          primaryForeground: role("--dx-primary-foreground"),
          secondary: role("--dx-secondary"),
          secondaryForeground: role("--dx-secondary-foreground"),
          muted: role("--dx-muted"),
          background: role("--dx-background"),
          border: role("--dx-border"),
        };
        return out;
      });
    }

    test("default, secondary, muted and outline read the role tokens", async ({ page }) => {
      const c = await read(page);
      expect(c.default.bg).toEqual(c.roles.primary);
      expect(c.default.fg).toEqual(c.roles.primaryForeground);
      expect(c.secondary.bg).toEqual(c.roles.secondary);
      expect(c.secondary.fg).toEqual(c.roles.secondaryForeground);
      expect(c.muted.bg).toEqual(c.roles.muted);
      expect(c.outline.bg).toEqual(c.roles.background);
      expect(c.outline.border).toEqual(c.roles.border);
      // Ghost is unframed: no fill.
      expect(c.ghost.bgAlpha).toBe("rgba(0, 0, 0, 0)");
    });

    test("tinted is a light tint in light mode and a dark one in dark mode", async ({ page }) => {
      const c = await read(page);
      if (scheme === "light") expect(c.tinted.bgLum).toBeGreaterThan(0.75);
      else expect(c.tinted.bgLum).toBeLessThan(0.1);
      expect(c.tinted.contrast).toBeGreaterThanOrEqual(4.5);
    });

    test("destructive text keeps >= 4.5:1 on its tinted fill (deliberately not raw --dx-destructive)", async ({ page }) => {
      const c = await read(page);
      expect(c.destructive.contrast).toBeGreaterThanOrEqual(4.5);
      // Still reads as red: the fill leans to red over the blue/green channels.
      const [r, g, b] = c.destructive.fg;
      expect(r).toBeGreaterThan(g);
      expect(r).toBeGreaterThan(b);
    });
  });
}

test.describe("Axe automated scan", () => {
  // A bubble has no overlay/expand/select interaction -- one state to scan (the
  // page holds the main demo and every variant, so this covers all of them).
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await open(page);
    await expectNoAxeViolations(page, "bubble: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
