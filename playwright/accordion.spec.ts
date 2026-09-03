import { test, expect, type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from "./axe";

const URL = "http://127.0.0.1:8080/component/?name=accordion&";
const LOAD_TIMEOUT = 20 * 60 * 1000;

async function loadAccordion(page: Page) {
  await page.goto(URL, { timeout: LOAD_TIMEOUT, waitUntil: 'networkidle' });
  const accordionItems = page.locator("[data-open]").filter({ has: page.getByRole("button") });
  await expect(accordionItems.first()).toHaveAttribute("data-disabled", "false", {
    timeout: 30000,
  });
  return accordionItems;
}

async function clickOpen(button: Locator, item: Locator) {
  await expect(button).toBeEnabled();
  await button.click();
  await expect(item).toHaveAttribute("data-open", "true");
}

test("test", async ({ page }) => {
  const accordionItems = await loadAccordion(page);
  const buttons = accordionItems.getByRole("button");
  const firstAccordionItem = accordionItems.first();
  await clickOpen(buttons.first(), firstAccordionItem);

  const secondAccordionItem = accordionItems.nth(1);
  await clickOpen(buttons.nth(1), secondAccordionItem);
  await expect(firstAccordionItem).toHaveAttribute("data-open", "false");
});

test("keyboard navigation skips disabled items", async ({ page }) => {
  const accordionItems = await loadAccordion(page);
  const buttons = accordionItems.getByRole("button");

  await expect(accordionItems.nth(2)).toHaveAttribute("data-disabled", "true");
  await expect(buttons.nth(2)).toBeDisabled();

  await buttons.nth(1).focus();
  await page.keyboard.press("ArrowDown");
  await expect(buttons.nth(3)).toBeFocused();

  await page.keyboard.press("ArrowUp");
  await expect(buttons.nth(1)).toBeFocused();
});

/**
 * Regression coverage for a plain CSS smoothness bug, not a conformance rule
 * (accordion open/close smoothness is not specified by APG/HTML, and the
 * mechanism this asserts on -- `grid-template-rows` -- is not a Radix or
 * bits-ui *behaviour* either, so this does not belong under `oracle/`'s
 * tiered rule-source policy; it is a straightforward "does not snap"
 * regression test, most at home here).
 *
 * `AccordionContent` only mounts once an item starts opening
 * (`use_animated_open`), so a plain `transition` on `grid-template-rows`
 * never has an earlier frame to interpolate from: the brand-new element's
 * very first paint already has `data-open="true"`, and the panel snaps
 * open instantly (verified against HEAD before this fix -- see the
 * "before" frame series in the accompanying investigation). The close
 * direction happened to look fine because there the element already exists
 * when `data-open` flips to `false`, so its transition does have a
 * prior frame.
 *
 * The fix (style.css) swaps the `transition` for a pair of `@keyframes`
 * animations, one per direction: a CSS *animation* always plays its `from`
 * keyframe on the frame it is applied, even to a freshly-mounted element,
 * unlike a `transition`. This is the same reason bits-ui
 * (https://www.bits-ui.com/docs/components/accordion) and Radix drive their
 * accordion content with `@keyframes` (`accordion-down`/`accordion-up`,
 * animating `height: 0 -> var(--bits-accordion-content-height)`) rather than
 * a transition -- consulted here only as a tie-breaker on *mechanism*, not
 * vendored, per this repo's tier-3 rule-source policy.
 */
async function sampleHeightFrames(page: Page, contentId: string, act: () => Promise<void>) {
  const framesPromise = page.evaluate((id) => {
    return new Promise<Array<{ exists: boolean; h: number | null }>>((resolve) => {
      const frames: Array<{ exists: boolean; h: number | null }> = [];
      let n = 0;
      function tick() {
        const el = document.getElementById(id);
        frames.push({ exists: !!el, h: el ? el.getBoundingClientRect().height : null });
        n++;
        if (n < 40) {
          requestAnimationFrame(tick);
        } else {
          resolve(frames);
        }
      }
      requestAnimationFrame(tick);
    });
  }, contentId);
  await act();
  return framesPromise;
}

function assertSmoothTransition(frames: Array<{ exists: boolean; h: number | null }>) {
  const heights = frames.filter((f) => f.exists && f.h !== null).map((f) => f.h as number);
  expect(heights.length).toBeGreaterThan(3);

  const start = heights[0];
  const end = heights[heights.length - 1];
  const delta = Math.abs(end - start);
  expect(delta).toBeGreaterThan(0);

  // At least 3 frames strictly between the start and end values (allowing
  // small tolerance) -- i.e. the transition actually interpolates instead
  // of snapping straight from start to end.
  const tolerance = Math.max(1, delta * 0.02);
  const strictlyIntermediate = heights.filter((h) => {
    const distFromStart = Math.abs(h - start);
    const distFromEnd = Math.abs(h - end);
    return distFromStart > tolerance && distFromEnd > tolerance;
  });
  expect(strictlyIntermediate.length).toBeGreaterThanOrEqual(3);

  // No single-frame jump should cover more than half of the total delta --
  // that would be a snap rather than an animation, at either the start or
  // the end of the transition.
  for (let i = 1; i < heights.length; i++) {
    const step = Math.abs(heights[i] - heights[i - 1]);
    expect(step).toBeLessThanOrEqual(delta * 0.5 + tolerance);
  }

  // Monotonic within tolerance (allow tiny easing overshoot/rounding noise).
  const increasing = end >= start;
  for (let i = 1; i < heights.length; i++) {
    if (increasing) {
      expect(heights[i]).toBeGreaterThanOrEqual(heights[i - 1] - tolerance);
    } else {
      expect(heights[i]).toBeLessThanOrEqual(heights[i - 1] + tolerance);
    }
  }
}

test("open and close animate the content height smoothly, without snapping", async ({ page }) => {
  const accordionItems = await loadAccordion(page);
  const buttons = accordionItems.getByRole("button");
  const firstButton = buttons.first();
  const contentId = await firstButton.getAttribute("aria-controls");
  expect(contentId).toBeTruthy();

  const openFrames = await sampleHeightFrames(page, contentId!, () => firstButton.click());
  assertSmoothTransition(openFrames);

  // Let the open animation fully settle before measuring the close.
  await page.waitForTimeout(500);

  const closeFrames = await sampleHeightFrames(page, contentId!, () => firstButton.click());
  assertSmoothTransition(closeFrames);
});

test.describe("Axe automated scan", () => {
  test("loaded (all items closed) has no automatically detectable a11y issues", async ({ page }) => {
    await loadAccordion(page);
    await expectNoAxeViolations(page, "accordion: loaded", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test("first item expanded has no automatically detectable a11y issues", async ({ page }) => {
    const accordionItems = await loadAccordion(page);
    const buttons = accordionItems.getByRole("button");
    await clickOpen(buttons.first(), accordionItems.first());
    await expectNoAxeViolations(page, "accordion: first item expanded", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
