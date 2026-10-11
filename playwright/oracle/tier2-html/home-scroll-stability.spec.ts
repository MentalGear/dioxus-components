/**
 * ORACLE: tier 2 (HTML) -- the overview page (`/`) keeps its height while it is scrolled for the first time.
 * A count, not a latency (load-proof, valid on a debug build); the timing numbers for the same page live in
 * the scroll profile (dev-docs/backlog.md row 174).
 *
 * Source: CSS Containment Module Level 2 -- `content-visibility: auto` skips the rendering of an
 * off-screen element, which is laid out at its `contain-intrinsic-size` until it has been rendered once
 * (`auto` then remembers the real size). The estimate is the CONTENT-box size, so a card's own padding
 * is on top of it:
 *   https://www.w3.org/TR/css-contain-2/#content-visibility
 *   https://www.w3.org/TR/css-sizing-4/#intrinsic-size-override
 *
 * The defect (row 174, cause 4): `.dx-component-card` estimated `auto 380px` while adding 3rem of padding
 * top and bottom, i.e. 476 px against a measured mean of 384 px (median 256 px). 58 of 75 cards were more
 * than 100 px off, so the document shrank 38.5k -> 32.0k px (-17%) during the first scroll: the scrollbar
 * thumb jumped, 64 height steps over 30 px, the largest 1,544 px in one frame. The estimate is now the
 * measured mean less the padding (`preview/assets/main.css`, next to the card rules); this fails when the
 * first scroll changes the document height by 10% or more, which is what re-measuring is for when the
 * gallery changes.
 *
 * Chromium only (it relies on Chromium's `content-visibility: auto` laying cards out at their estimate):
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts \
 *     ./oracle/tier2-html/home-scroll-stability.spec.ts
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

test.describe("overview page scroll height", () => {
  test.skip(({ browserName }) => browserName !== "chromium", "content-visibility estimates are measured in Chromium");

  async function openHome(page: Page) {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/`, { timeout: 20 * 60 * 1000 });
    await expect(page.locator(".dx-component-card").first()).toBeVisible({ timeout: 5 * 60 * 1000 });
    await page.waitForFunction(() => document.querySelectorAll(".dx-component-card").length >= 60, undefined, { timeout: 120_000 });
    await page.waitForTimeout(2500); // hydration and the first measured renders settle
    await page.mouse.move(640, 400);
  }

  /** Wheel down in 400 px ticks until the bottom, returning the document height at each tick. */
  async function scrollToBottom(page: Page) {
    const heights: number[] = [];
    for (let i = 0; i < 400; i++) {
      const s = await page.evaluate(() => ({ y: scrollY, h: document.scrollingElement!.scrollHeight, vh: innerHeight }));
      heights.push(s.h);
      if (s.y + s.vh >= s.h - 2) break;
      await page.mouse.wheel(0, 400);
      await page.waitForTimeout(40);
    }
    await page.waitForTimeout(400);
    return heights;
  }

  test("the first scroll down the page changes the document height by less than 10%", async ({ page }) => {
    await openHome(page);
    const heights = await scrollToBottom(page);
    const start = heights[0];
    const end = await page.evaluate(() => document.scrollingElement!.scrollHeight);
    const change = Math.abs(end - start) / start;
    test.info().annotations.push({ type: "document height", description: `${start} -> ${end} px (${(change * 100).toFixed(1)}%)` });
    expect(change, `document height ${start} -> ${end} px over the first scroll (was 38.5k -> 32.0k, -17%, with the content-box estimate that ignored the 3rem card padding)`).toBeLessThan(0.1);
  });
});
