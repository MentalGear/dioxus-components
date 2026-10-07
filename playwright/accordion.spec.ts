import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

const URL = `${BASE_URL}/component/?name=accordion&`;
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
    await expectNoAxeViolations(page, "accordion: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("first item expanded has no automatically detectable a11y issues", async ({ page }) => {
    const accordionItems = await loadAccordion(page);
    const buttons = accordionItems.getByRole("button");
    await clickOpen(buttons.first(), accordionItems.first());
    await expectNoAxeViolations(page, "accordion: first item expanded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

/**
 * `hidden_until_found` (backlog row 144), driven through the `until_found` variant: three items
 * (Shipping open by default, Returns, Warranty), `allow_multiple_open: false`. Closed content stays
 * mounted as `hidden="until-found"`; a reveal by the browser opens that item and closes the other.
 *
 * Headless Chromium's `window.find` finds `until-found` text but does not fire `beforematch` for
 * it (only fragment navigation does), so the reveal is driven by a real `#fragment` navigation and
 * by a synthetic `beforematch`; real Ctrl+F takes the same path as the fragment and is checked by
 * hand in a headed browser.
 */
test.describe("hidden_until_found", () => {
  async function load(page: Page) {
    await gotoHydrated(page, URL, { timeout: LOAD_TIMEOUT });
    const trigger = (name: string) => page.getByRole("button", { name, exact: true });
    const panelOf = (name: string) =>
      page.locator(".dx-accordion-content", { has: page.locator("p"), hasText: PANEL_TEXT[name] });
    await expect(trigger("Shipping")).toHaveAttribute("aria-expanded", "true");
    // Default-open Shipping finishes its open animation (and `use_animated_open`'s effect) first.
    await expect(panelOf("Shipping")).not.toHaveAttribute("hidden", /.*/);
    return { trigger, panelOf };
  }

  const PANEL_TEXT: Record<string, string> = {
    Shipping: "Orders leave the warehouse",
    Returns: "narwhal plush",
    Warranty: "two year limited warranty",
  };

  /** Give the paragraph inside a panel an id the page can navigate to with a `#fragment`. */
  async function markInside(panel: Locator, id: string) {
    await panel.evaluate((el, id) => {
      el.querySelector("p")!.id = id;
    }, id);
  }

  async function expectOpenItem(
    page: Page,
    trigger: (name: string) => Locator,
    panelOf: (name: string) => Locator,
    open: string,
  ) {
    for (const name of Object.keys(PANEL_TEXT)) {
      await expect(trigger(name)).toHaveAttribute("aria-expanded", String(name === open));
      if (name === open) {
        await expect(panelOf(name)).not.toHaveAttribute("hidden", /.*/);
        await expect(panelOf(name)).toHaveAttribute("data-open", "true");
      } else {
        // Closed again: its close animation finishes, then it is re-hidden but stays mounted.
        await expect(panelOf(name)).toHaveAttribute("hidden", "until-found", { timeout: 5000 });
        await expect(panelOf(name)).toHaveAttribute("data-open", "false");
      }
    }
  }

  test("closed items stay mounted as hidden=until-found only when opted in", async ({ page }) => {
    const { panelOf } = await load(page);
    for (const name of ["Returns", "Warranty"]) {
      await expect(panelOf(name)).toHaveCount(1);
      await expect(panelOf(name)).toHaveAttribute("hidden", "until-found");
      expect(await panelOf(name).evaluate((el) => getComputedStyle(el).contentVisibility)).toBe("hidden");
    }
    // The default demo's four closed items render no content element at all: only the opt-in
    // demo's three exist.
    await expect(page.locator(".dx-accordion-content")).toHaveCount(3);
    // And find-in-page reaches the opted-in text but not the unmounted text.
    expect(await page.evaluate(() => window.find("narwhal"))).toBe(true);
    expect(await page.evaluate(() => window.find("lorem ipsum lorem"))).toBe(false);
  });

  test("a closed until-found panel at rest runs no animation", async ({ page }) => {
    await load(page);
    // `data-open="false"` is also how a closed panel looks at rest; the close animation must be
    // keyed on `data-closing`, or every closed panel would play it on page load.
    expect(
      await page.evaluate(() =>
        document
          .getAnimations()
          .map((a) => (a as CSSAnimation).animationName)
          .filter((n) => n?.startsWith("dx-accordion-close")),
      ),
    ).toEqual([]);
  });

  test("a #fragment into a closed item reveals it and opens that item, closing the other", async ({ page }) => {
    const { trigger, panelOf } = await load(page);
    await markInside(panelOf("Returns"), "narwhal-target");
    await page.evaluate(() => {
      location.hash = "#narwhal-target";
    });

    await expectOpenItem(page, trigger, panelOf, "Returns");
    await expect(page.locator("#narwhal-target")).toBeVisible();
  });

  test("a beforematch on a closed item opens it and respects single-open mode", async ({ page }) => {
    const { trigger, panelOf } = await load(page);
    await panelOf("Warranty").dispatchEvent("beforematch");
    await expectOpenItem(page, trigger, panelOf, "Warranty");
  });

  test("a reveal is not animated: the item is at full height from its first frame", async ({ page }) => {
    const { panelOf } = await load(page);
    const returns = panelOf("Returns");
    await markInside(returns, "narwhal-target");
    const contentId = await returns.getAttribute("id");
    expect(contentId).toBeTruthy();

    const frames = await page.evaluate(async (id) => {
      const el = document.getElementById(id)!;
      const out: Array<{ h: number; hidden: boolean; animation: string }> = [];
      const done = new Promise<void>((resolve) => {
        let n = 0;
        const tick = () => {
          out.push({
            h: el.getBoundingClientRect().height,
            hidden: el.hasAttribute("hidden"),
            animation: getComputedStyle(el).animationName,
          });
          if (++n < 45) requestAnimationFrame(tick);
          else resolve();
        };
        requestAnimationFrame(tick);
      });
      location.hash = "#narwhal-target";
      await done;
      return out;
    }, contentId!);

    const visible = frames.filter((f) => !f.hidden);
    expect(visible.length).toBeGreaterThan(10);
    const settled = visible[visible.length - 1].h;
    expect(settled).toBeGreaterThan(20);
    // Never collapses and re-expands (the fight with the open animation): every frame from the
    // first visible one is already at full height, and none runs the open animation.
    for (const f of visible) {
      expect(f.h).toBeGreaterThanOrEqual(settled - 1);
      expect(f.animation).toBe("none");
    }
    await expect(returns).toHaveAttribute("data-revealed", "true");
  });

  test("opening by click still animates, closing animates and then re-hides", async ({ page }) => {
    const { trigger, panelOf } = await load(page);
    const returns = panelOf("Returns");
    const contentId = (await returns.getAttribute("id"))!;

    const openFrames = await sampleHeightFrames(page, contentId, () => trigger("Returns").click());
    assertSmoothTransition(openFrames);
    await expect(returns).not.toHaveAttribute("data-revealed", /.*/);
    await page.waitForTimeout(500);

    const closeFrames = await sampleHeightFrames(page, contentId, () => trigger("Returns").click());
    assertSmoothTransition(closeFrames);
    expect(closeFrames[closeFrames.length - 1].h).toBe(0);
    await expect(returns).toHaveAttribute("hidden", "until-found");
    await expect(trigger("Returns")).toHaveAttribute("aria-expanded", "false");
  });

  test("an item revealed by a match closes and re-opens as an ordinary animated item afterwards", async ({ page }) => {
    const { trigger, panelOf } = await load(page);
    const returns = panelOf("Returns");
    await returns.dispatchEvent("beforematch");
    await expect(returns).toHaveAttribute("data-revealed", "true");
    await page.waitForTimeout(500);

    await trigger("Returns").click();
    await expect(returns).toHaveAttribute("hidden", "until-found", { timeout: 5000 });
    await expect(returns).not.toHaveAttribute("data-revealed", /.*/);

    const contentId = (await returns.getAttribute("id"))!;
    const openFrames = await sampleHeightFrames(page, contentId, () => trigger("Returns").click());
    assertSmoothTransition(openFrames);
  });

  test("listens exactly once per item, through toggles, and not at all after unmount", async ({ page }) => {
    const { trigger, panelOf } = await load(page);
    const cdp = await page.context().newCDPSession(page);
    const beforematchListeners = async (expression: string) => {
      const { result } = await cdp.send("Runtime.evaluate", { expression });
      const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId! });
      return listeners.filter((l) => l.type === "beforematch").length;
    };
    await panelOf("Warranty").evaluate((el) => {
      (window as any).__panel = el;
    });
    expect(await beforematchListeners("window.__panel")).toBe(1);

    for (let i = 0; i < 3; i++) {
      await trigger("Warranty").click();
      await expect(trigger("Warranty")).toHaveAttribute("aria-expanded", "true");
      await trigger("Warranty").click();
      await expect(trigger("Warranty")).toHaveAttribute("aria-expanded", "false");
    }
    expect(await beforematchListeners("window.__panel")).toBe(1);

    await page.locator('a[href="/component/kbd/?"]').first().click();
    await expect(page).toHaveURL(/\/component\/kbd\//);
    await expect.poll(() => page.evaluate(() => (window as any).__panel.isConnected)).toBe(false);
    expect(await beforematchListeners("window.__panel")).toBe(0);
  });
});
