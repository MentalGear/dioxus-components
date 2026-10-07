/**
 * Carousel: the disabled Previous/Next arrow's opacity
 * (`Carousel { nav_disabled_opacity }`, `--dx-carousel-nav-disabled-opacity`,
 * `--dx-carousel-nav-disabled-visibility`), and where keyboard focus goes
 * when the arrow that holds it becomes disabled.
 *
 * Owner ask: "carousel option to set opacity of deactivated arrow btns, e.g.
 * 0 to fade it all out if not usable e.g. at the last slide". The construction
 * under test (preview/src/components/carousel/style.css, primitives/src/carousel.rs):
 *
 *  - one CSS custom property sets the disabled opacity for BOTH arrows, with
 *    today's look (the theme's 0.5 disabled opacity) as the default;
 *  - at opacity 0 the prop also sets `--dx-carousel-nav-disabled-visibility:
 *    hidden`, and the arrow turns `visibility: hidden` only AFTER its fade
 *    (a zero-length visibility transition delayed by the fade duration), and
 *    comes back from the first frame of the fade in -- the box is kept, so
 *    nothing shifts;
 *  - focus on an arrow that becomes disabled moves to the track
 *    (`.dx-carousel-content`), instead of being dropped to <body> (Chromium's
 *    focus fixup) -- which a hidden arrow would otherwise do every time.
 *
 * SCOPING: every variant of a "Normal"-kind component renders on the same
 * page at once (see carousel.spec.ts's header), so every locator is rooted
 * in `#component-preview-frame[-<variant>]` and the carousel inside it by its
 * (exact) region name. The `hidden_arrows` demo is ONE carousel, so RTL and
 * vertical are checked on the existing `rtl`/`vertical` demos with the same
 * two inline custom properties the prop renders (see `setHiddenArrowVariables`).
 *
 * TIMING: only CSS-transition behaviour and focus are asserted, never wasm
 * latency, so this file is valid on a debug build. Positions are measured
 * relative to the carousel root, never the viewport, so an unrelated scroll
 * cannot move them.
 */

import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { expectNoAxeViolations } from "./axe";
import { gotoHydrated } from "./hydration";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

async function goto(page: Page, variant: string) {
  await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=${variant}&`, GOTO_OPTS);
}

function frame(page: Page, variant: "main" | "hidden_arrows" | "rtl" | "vertical" | "rewind"): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`);
}

/** The `hidden_arrows` demo's one carousel (`nav_disabled_opacity: 0.0`). */
function hiddenArrows(page: Page): Locator {
  return frame(page, "hidden_arrows").getByRole("region", {
    name: "Gallery with hidden arrows",
    exact: true,
  });
}

/**
 * Do to an existing carousel exactly what `nav_disabled_opacity: 0.0` does:
 * the prop is nothing but these two inline custom properties on the root
 * (primitives/src/carousel.rs `nav_disabled_vars`, unit-tested there). Used
 * for the RTL and vertical layouts so the `hidden_arrows` demo can stay one
 * carousel -- every variant of a component renders on the same page, so each
 * extra demo carousel is hydrated by every carousel test.
 */
async function setHiddenArrowVariables(root: Locator) {
  await root.evaluate((el) => {
    el.style.setProperty("--dx-carousel-nav-disabled-opacity", "0");
    el.style.setProperty("--dx-carousel-nav-disabled-visibility", "hidden");
  });
}

const prevOf = (root: Locator) => root.locator(".dx-carousel-previous");
const nextOf = (root: Locator) => root.locator(".dx-carousel-next");

/** A paint-relevant snapshot of an arrow. */
async function look(button: Locator) {
  return button.evaluate((el) => {
    const cs = getComputedStyle(el);
    return {
      opacity: parseFloat(cs.opacity),
      visibility: cs.visibility,
      disabled: (el as HTMLButtonElement).disabled,
    };
  });
}

/**
 * Where the arrows and the track sit, relative to the carousel root's own
 * top-left corner (so an unrelated page scroll cannot move any of it). The
 * arrows' boxes must be identical at every slide: `visibility: hidden` keeps
 * the box, `display: none` or a size change would not.
 */
async function layout(root: Locator) {
  return root.evaluate((el) => {
    const origin = el.getBoundingClientRect();
    const rect = (sel: string) => {
      const r = (el.querySelector(sel) as HTMLElement).getBoundingClientRect();
      return {
        x: Math.round((r.x - origin.x) * 100) / 100,
        y: Math.round((r.y - origin.y) * 100) / 100,
        w: Math.round(r.width * 100) / 100,
        h: Math.round(r.height * 100) / 100,
      };
    };
    return {
      root: { w: origin.width, h: origin.height },
      previous: rect(".dx-carousel-previous"),
      next: rect(".dx-carousel-next"),
      content: rect(".dx-carousel-content"),
    };
  });
}

/**
 * Sample `{opacity, visibility, disabled}` of the first element matching
 * `selector` inside `root` on every animation frame, from before `trigger`
 * runs until `settleMs` after it returns (a click's own latency on a debug
 * build is not a constant, so the window is anchored to the trigger's end,
 * never a fixed length).
 */
async function sampleDuring(
  root: Locator,
  selector: string,
  trigger: () => Promise<void>,
  settleMs = 800,
): Promise<{ opacity: number; visibility: string; disabled: boolean }[]> {
  const page = root.page();
  const handle = await root.elementHandle();
  if (!handle) throw new Error("sampleDuring: root resolved to no element");
  await page.evaluate(
    ({ el, selector }) => {
      const w = window as unknown as { __navSamples: unknown[]; __navStop: boolean };
      w.__navSamples = [];
      w.__navStop = false;
      const target = (el as HTMLElement).querySelector(selector) as HTMLElement;
      const tick = () => {
        if (w.__navStop) return;
        const cs = getComputedStyle(target);
        w.__navSamples.push({
          opacity: parseFloat(cs.opacity),
          visibility: cs.visibility,
          disabled: (target as HTMLButtonElement).disabled,
        });
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    },
    { el: handle, selector },
  );
  await trigger();
  await page.waitForTimeout(settleMs);
  return page.evaluate(() => {
    const w = window as unknown as {
      __navSamples: { opacity: number; visibility: string; disabled: boolean }[];
      __navStop: boolean;
    };
    w.__navStop = true;
    return w.__navSamples;
  });
}

async function activeElementInfo(page: Page) {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return {
      tag: el?.tagName ?? null,
      className: el?.className ?? "",
      label: el?.getAttribute("aria-label") ?? "",
    };
  });
}

/** Step to the last slide with Next, waiting out each smooth scroll. */
async function clickToEnd(root: Locator, steps: number) {
  const next = nextOf(root);
  for (let i = 0; i < steps; i++) {
    await next.click();
    await root.page().waitForTimeout(450);
  }
}

test.describe("Carousel: disabled arrow opacity -- the default look is unchanged", () => {
  test("with no prop, the disabled arrow keeps the theme's 0.5 and stays visible", async ({ page }) => {
    await goto(page, "main");
    const root = frame(page, "main").locator(".dx-carousel");
    const prev = prevOf(root);
    const next = nextOf(root);

    await expect(prev).toBeDisabled();
    expect(await look(prev)).toEqual({ opacity: 0.5, visibility: "visible", disabled: true });
    expect(await look(next)).toEqual({ opacity: 1, visibility: "visible", disabled: false });

    // Neither variable is declared anywhere: the stylesheet's own fallbacks
    // are what produced 0.5 / visible.
    const vars = await root.evaluate((el) => {
      const cs = getComputedStyle(el);
      return [
        cs.getPropertyValue("--dx-carousel-nav-disabled-opacity"),
        cs.getPropertyValue("--dx-carousel-nav-disabled-visibility"),
        el.getAttribute("style"),
      ];
    });
    expect(vars).toEqual(["", "", null]);
  });

  test("the disabled arrow at the last slide is also still 0.5 and visible", async ({ page }) => {
    await goto(page, "main");
    const root = frame(page, "main").locator(".dx-carousel");
    await clickToEnd(root, 4);
    await expect(nextOf(root)).toBeDisabled();
    expect(await look(nextOf(root))).toEqual({ opacity: 0.5, visibility: "visible", disabled: true });
    expect(await look(prevOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });
  });
});

test.describe("Carousel: nav_disabled_opacity 0 fades the arrow out and then hides it", () => {
  test("at the first slide Previous is disabled, transparent and hidden; Next is untouched", async ({
    page,
  }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await expect(prevOf(root)).toBeDisabled();
    expect(await look(prevOf(root))).toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(nextOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });

    // The prop is what set it: both variables, inline on the root.
    const style = await root.getAttribute("style");
    expect(style).toContain("--dx-carousel-nav-disabled-opacity:0;");
    expect(style).toContain("--dx-carousel-nav-disabled-visibility:hidden;");
  });

  test("the hidden arrow is out of the accessibility tree and cannot be reached", async ({ page }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await expect(prevOf(root)).toBeHidden();
    // Role queries skip `visibility: hidden` content.
    await expect(root.getByRole("button", { name: "Previous slide" })).toHaveCount(0);
    await expect(root.getByRole("button", { name: "Next slide" })).toHaveCount(1);

    // Hit testing: nothing at the arrow's own centre is the arrow.
    const hit = await prevOf(root).evaluate((el) => {
      const r = el.getBoundingClientRect();
      const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
      return top === el;
    });
    expect(hit).toBe(false);

    // And it is not a Tab stop: Tab from before the carousel reaches Next
    // first, never the hidden Previous.
    await nextOf(root).focus();
    await page.keyboard.press("Shift+Tab");
    expect((await activeElementInfo(page)).label).not.toBe("Previous slide");
  });

  test("walking to the last slide hides Next and un-hides Previous, with no layout shift", async ({
    page,
  }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    const first = await layout(root);

    const seen = [first];
    for (let step = 0; step < 4; step++) {
      await nextOf(root).click();
      await page.waitForTimeout(450);
      seen.push(await layout(root));
    }

    await expect(nextOf(root)).toBeDisabled();
    expect(await look(nextOf(root))).toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(prevOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });

    // Every position: the arrows, the track and the root keep one box.
    for (const [i, l] of seen.entries()) {
      expect(l, `layout at slide ${i + 1} differs from slide 1`).toEqual(first);
    }
  });

  test("the fade runs first and the hide lands at its end; coming back, the arrow is visible at once", async ({
    page,
  }) => {
    await goto(page, "hidden_arrows");
    const reducedMotion = await page.evaluate(
      () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    );
    test.skip(reducedMotion, "reduced motion collapses the fade by design; covered by its own test");

    const root = hiddenArrows(page);
    await clickToEnd(root, 3); // slide 4 of 5: Next still enabled
    await expect(nextOf(root)).toBeEnabled();

    // Disabling: opacity goes 1 -> 0 through intermediate values while the
    // arrow is still `visible`, and only the last frames are `hidden`.
    const out = await sampleDuring(root, ".dx-carousel-next", () => nextOf(root).click());
    await expect(nextOf(root)).toBeDisabled();
    const mid = out.filter((s) => s.opacity > 0.05 && s.opacity < 0.95);
    expect(mid.length, `no intermediate opacity frames: ${JSON.stringify(out)}`).toBeGreaterThan(0);
    expect(
      mid.every((s) => s.visibility === "visible"),
      `visibility flipped before the fade finished: ${JSON.stringify(mid)}`,
    ).toBe(true);
    // Never hidden while still clearly visible (no pop).
    expect(out.some((s) => s.visibility === "hidden" && s.opacity > 0.15)).toBe(false);
    expect(out[out.length - 1]).toMatchObject({ opacity: 0, visibility: "hidden" });

    await page.waitForTimeout(400);

    // Enabling: visible from the first frame of the fade in.
    const back = await sampleDuring(root, ".dx-carousel-next", () => prevOf(root).click());
    await expect(nextOf(root)).toBeEnabled();
    expect(
      back.some((s) => s.visibility === "hidden" && s.opacity > 0.05),
      `still hidden while fading back in: ${JSON.stringify(back)}`,
    ).toBe(false);
    const rising = back.filter((s) => s.opacity > 0.05 && s.opacity < 0.95);
    expect(rising.length, `no fade in: ${JSON.stringify(back)}`).toBeGreaterThan(0);
    expect(back[back.length - 1]).toMatchObject({ opacity: 1, visibility: "visible" });
  });

  test("under prefers-reduced-motion the hide is immediate too", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await clickToEnd(root, 3);

    const out = await sampleDuring(root, ".dx-carousel-next", () => nextOf(root).click());
    await expect(nextOf(root)).toBeDisabled();
    // The fade is collapsed, so once the arrow is disabled the whole transition
    // fits in a few frames; the un-overridden visibility delay would be the
    // full 200ms token (a dozen frames).
    const disabledAt = out.findIndex((s) => s.disabled);
    const hiddenAt = out.findIndex((s) => s.visibility === "hidden");
    expect(disabledAt, JSON.stringify(out)).toBeGreaterThanOrEqual(0);
    expect(hiddenAt, JSON.stringify(out)).toBeGreaterThanOrEqual(0);
    expect(hiddenAt - disabledAt, `frames from disabled to hidden: ${JSON.stringify(out)}`).toBeLessThanOrEqual(4);
    // The disabled rule carries the visibility delay, so read it off the
    // arrow now that it is disabled.
    const tDelay = await nextOf(root).evaluate((el) => getComputedStyle(el).transitionDelay);
    const parsed = tDelay.split(",").map((v) => parseFloat(v));
    expect(parsed.every((v) => v < 0.05), `transition-delay ${tDelay}`).toBe(true);
    expect(out[out.length - 1]).toMatchObject({ opacity: 0, visibility: "hidden" });
  });
});

test.describe("Carousel: hidden arrows in RTL and vertical layouts", () => {
  test("RTL: Previous sits at the physical right, is hidden at the first slide and keeps its box", async ({
    page,
  }) => {
    await goto(page, "rtl");
    const root = frame(page, "rtl").getByRole("region", {
      name: "Right-to-left slideshow demo",
      exact: true,
    });
    await setHiddenArrowVariables(root);
    await expect(root).toHaveAttribute("data-direction", "rtl");
    await expect(prevOf(root)).toBeDisabled();
    // The variables just changed under an already-disabled arrow, so it is
    // mid-fade (0.5 -> 0) for a moment: wait the transition out.
    await expect
      .poll(async () => look(prevOf(root)))
      .toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(nextOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });

    const first = await layout(root);
    // The hidden box is still where the mirrored layout puts it: Previous
    // (inline-start) right of Next (inline-end).
    expect(first.previous.x).toBeGreaterThan(first.next.x);

    for (let step = 0; step < 3; step++) {
      await nextOf(root).click();
      await page.waitForTimeout(450);
      expect(await layout(root)).toEqual(first);
    }
    await expect(nextOf(root)).toBeDisabled();
    expect(await look(nextOf(root))).toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(prevOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });
  });

  test("vertical: the up arrow is hidden at the first slide, the down arrow at the last, nothing moves", async ({
    page,
  }) => {
    await goto(page, "vertical");
    const root = frame(page, "vertical").getByRole("region", {
      name: "Vertical scrolling demo",
      exact: true,
    });
    await setHiddenArrowVariables(root);
    await expect(root).toHaveAttribute("data-orientation", "vertical");
    await expect(prevOf(root)).toBeDisabled();
    // The variables just changed under an already-disabled arrow, so it is
    // mid-fade (0.5 -> 0) for a moment: wait the transition out.
    await expect
      .poll(async () => look(prevOf(root)))
      .toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(nextOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });

    const first = await layout(root);
    // Stacked on the block axis, not the inline one.
    expect(first.previous.y).toBeLessThan(first.next.y);

    // The demo shows two slides at once, so page until Next gives out rather
    // than counting on a slide total.
    for (let step = 0; step < 6 && (await nextOf(root).isEnabled()); step++) {
      await nextOf(root).click();
      await page.waitForTimeout(450);
      expect(await layout(root)).toEqual(first);
    }
    await expect(nextOf(root)).toBeDisabled();
    expect(await look(nextOf(root))).toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    expect(await look(prevOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });
  });
});

test.describe("Carousel: the variable is the API -- CSS alone works without the prop", () => {
  test("setting the opacity variable on the root, with no prop, dims both arrows to that value", async ({
    page,
  }) => {
    await goto(page, "main");
    const root = frame(page, "main").locator(".dx-carousel");
    await root.evaluate((el) =>
      (el as HTMLElement).style.setProperty("--dx-carousel-nav-disabled-opacity", "0.2"),
    );
    // Opacity only: still visible (a screen reader can still find it disabled).
    await expect
      .poll(async () => look(prevOf(root)))
      .toEqual({ opacity: 0.2, visibility: "visible", disabled: true });

    await clickToEnd(root, 4);
    await expect
      .poll(async () => look(nextOf(root)))
      .toEqual({ opacity: 0.2, visibility: "visible", disabled: true });
  });

  test("the variables also work when set on an ancestor, and the visibility variable hides the arrow", async ({
    page,
  }) => {
    await goto(page, "main");
    const root = frame(page, "main").locator(".dx-carousel");
    await root.evaluate((el) => {
      const parent = el.parentElement as HTMLElement;
      parent.style.setProperty("--dx-carousel-nav-disabled-opacity", "0");
      parent.style.setProperty("--dx-carousel-nav-disabled-visibility", "hidden");
    });
    await expect
      .poll(async () => look(prevOf(root)))
      .toEqual({ opacity: 0, visibility: "hidden", disabled: true });
    // The enabled arrow is never touched by either variable.
    expect(await look(nextOf(root))).toEqual({ opacity: 1, visibility: "visible", disabled: false });
  });
});

test.describe("Carousel: focus when the focused arrow becomes disabled", () => {
  test("Enter on Next, paging to the last slide, moves focus to the track, never <body>", async ({
    page,
  }) => {
    // The `main` demo has the DEFAULT 0.5 look: the hand-off is not specific
    // to the hidden arrow, because Chromium already dropped focus to <body>
    // here before this change.
    await goto(page, "main");
    const root = frame(page, "main").locator(".dx-carousel");
    await nextOf(root).focus();
    for (let i = 0; i < 4; i++) {
      await page.keyboard.press("Enter");
      await page.waitForTimeout(450);
    }
    await expect(nextOf(root)).toBeDisabled();
    const active = await activeElementInfo(page);
    expect(active.tag, JSON.stringify(active)).toBe("DIV");
    expect(active.className).toContain("dx-carousel-content");
  });

  test("with a hidden arrow: clicking Next onto the last slide leaves focus on the track", async ({
    page,
  }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await clickToEnd(root, 4); // mouse clicks focus the button in Chromium
    await expect(nextOf(root)).toBeDisabled();
    await expect(nextOf(root)).toBeHidden();
    const active = await activeElementInfo(page);
    expect(active.className, JSON.stringify(active)).toContain("dx-carousel-content");
  });

  test("Space on the hidden-arrow carousel does the same, and arrow keys keep paging from the track", async ({
    page,
  }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    const slide = (n: number) => root.getByRole("group", { name: `${n} of 5` });
    await nextOf(root).focus();
    for (let i = 0; i < 4; i++) {
      await page.keyboard.press("Space");
      await page.waitForTimeout(450);
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    expect((await activeElementInfo(page)).className).toContain("dx-carousel-content");

    // The hand-off target is a working place to be: ArrowLeft pages back, and
    // Previous (enabled again) is reachable with Shift+Tab.
    await page.keyboard.press("ArrowLeft");
    await expect(slide(4)).toHaveAttribute("data-selected", "true");
    await expect(nextOf(root)).toBeEnabled();
  });

  test("the symmetric case: Enter on Previous paging back to the first slide", async ({ page }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await clickToEnd(root, 1);
    await expect(prevOf(root)).toBeEnabled();
    await prevOf(root).focus();
    await page.keyboard.press("Enter");
    await page.waitForTimeout(450);
    await expect(prevOf(root)).toBeDisabled();
    await expect(prevOf(root)).toBeHidden();
    expect((await activeElementInfo(page)).className).toContain("dx-carousel-content");
  });

  test("focus that is NOT on the arrow is left alone", async ({ page }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    // Focus Previous (enabled after one step), then page to the END with the
    // root keyboard handler: Next becomes disabled, but it never had focus.
    await clickToEnd(root, 1);
    await prevOf(root).focus();
    for (let i = 0; i < 3; i++) {
      await page.keyboard.press("ArrowRight");
      await page.waitForTimeout(450);
    }
    await expect(nextOf(root)).toBeDisabled();
    const active = await activeElementInfo(page);
    expect(active.label, JSON.stringify(active)).toBe("Previous slide");
  });

  test("a looping carousel never disables an arrow, so focus never moves", async ({ page }) => {
    await goto(page, "rewind");
    const root = page.locator("#component-preview-frame-rewind").getByRole("region", {
      name: "Rewind-loop gallery",
      exact: true,
    });
    await nextOf(root).focus();
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press("Enter");
      await page.waitForTimeout(150);
    }
    await expect(nextOf(root)).toBeEnabled();
    expect((await activeElementInfo(page)).label).toBe("Next slide");
  });
});

test.describe("Carousel: axe on the hidden-arrow demo", () => {
  test("no automatically detectable a11y issues at the first slide and at the last", async ({ page }) => {
    await goto(page, "hidden_arrows");
    const root = hiddenArrows(page);
    await expectNoAxeViolations(page, "carousel: hidden_arrows, first slide", {
      include: "#component-preview-frame-hidden_arrows",
    });
    await clickToEnd(root, 4);
    await expect(nextOf(root)).toBeHidden();
    await expectNoAxeViolations(page, "carousel: hidden_arrows, last slide", {
      include: "#component-preview-frame-hidden_arrows",
    });
  });
});
