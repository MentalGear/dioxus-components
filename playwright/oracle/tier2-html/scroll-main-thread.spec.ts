/**
 * ORACLE: tier 2 (HTML) -- scroll smoothness of the overview page (`/`), by COUNTS.
 *
 * Source: dev-docs/research/scroll-jank-2026-10-04.md (the investigation, `scripts/measure-scroll.mjs`
 * the harness, section 5 the constructions this file guards). Rule sources, per this tier's policy of
 * citing a standard rather than an opinion:
 *   - DOM Standard, `AddEventListenerOptions.passive` ("a passive listener will never call
 *     `preventDefault()`") and its "default passive" rule for window/document/body wheel + touch
 *     listeners; UI Events, `wheel` is cancelable -- so a NON-passive wheel/touch listener forces the
 *     user agent to wait for the handler before it may scroll:
 *       https://dom.spec.whatwg.org/#dom-addeventlisteneroptions-passive
 *       https://w3c.github.io/uievents/#event-type-wheel
 *   - Pointer Events, `touch-action` (what actually keeps a drag from panning the page, with no
 *     `touchstart` handler involved):
 *       https://w3c.github.io/pointerevents/#the-touch-action-css-property
 *   - HTML Standard, timers (`setInterval` repeats until `clearInterval`, for the life of the
 *     page, whatever script created it):
 *       https://html.spec.whatwg.org/multipage/timers-initialization.html#dom-setinterval
 *   - CSS Containment Level 2, `content-visibility: auto` (off-screen content is skipped: no style,
 *     layout or paint; it stays focusable and, in Chromium/Firefox, searchable by find-in-page --
 *     not in Safari 18-25, WebKit bug 283846, fixed in Safari 26):
 *       https://www.w3.org/TR/css-contain-2/#content-visibility
 *
 * What it asserts (all counts -- load-proof, valid on a debug build; a latency threshold belongs in
 * `main-thread.spec.ts`, which is skipped on debug):
 *   1. Page timers do not grow with the page's age. A shim counts every `setInterval` (live =
 *      created - cleared) and every timer callback that fires; over an idle window the live-interval
 *      count is constant and the wakeups per second do not climb. At HEAD a fresh 100 ms interval was
 *      created about every 1.2 s and never cleared (128 live after ~100 s, 1,326 wakeups/s at 155 s).
 *   2. The music-player block's readout advances and its play/pause works. Both were dead at HEAD
 *      (stuck at 1:24, always "playing") because the block's hooks ran in the PARENT's scope and every
 *      re-render allocated fresh ones -- the same bug that leaked the intervals (see `MasonryCard`).
 *   3. `#main` (Dioxus' event root) has no NON-passive `wheel` / `touchstart` / `touchmove` listener,
 *      asked of Chromium itself (`DOMDebugger.getEventListeners`). At HEAD, `Input`'s unconditional
 *      `onwheel` and three `ontouchstart: prevent_default` made all of them blocking.
 *   4. A real-touch drag that starts on the slider thumb / resizable handle / color-area thumb moves the
 *      control and does NOT scroll the page or select text, with no touchstart handler to say so
 *      (`touch-action: none` does it) -- the guard for removing those three handlers.
 *   5. The demo cards are skipped off screen (`content-visibility: auto` on `.dx-component-card` and
 *      the non-popout `.dx-widget-card`; popout cards stay `visible`, containment would clip their
 *      dropdown): >= 70% of the gallery cards are skipped at the top AND at the bottom of the page
 *      (`checkVisibility({ contentVisibilityAuto: true })`), and the idle page relayouts almost never
 *      (`Performance.getMetrics` LayoutCount, ceiling below). A scripted scroll's layout/recalc counts
 *      are REPORTED (annotation + console), not capped: they do not separate HEAD from the fix (see the
 *      comment at the scroll in test 5). Test 6: the same for the component pages' stacked variants.
 *
 * Chromium only (CDP), like `main-thread.spec.ts`. Run against the dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts \
 *     ./oracle/tier2-html/scroll-main-thread.spec.ts
 *
 * Red proof (2026-10-04), against the release build of HEAD (`/home/user/tgt/tip/dx/preview/release/web/public`
 * on :8090): tests 1, 2, 3, 5 and 6 fail (live setIntervals 4 -> 10 -> 15 over 12 s idle, 59 then 111 timer
 * fires/s; the player readout stuck; `#main` blocking on `wheel` + `touchstart`; no card or variant skipped).
 * All 8 pass against the fixed dev build on :8083. The three touch tests pass on both -- they guard the
 * removal of the `ontouchstart` handlers, not a HEAD defect. (The resizable one was red on the first fixed
 * build, before `main.css` exempted the handle from its `touch-action: manipulation !important` catch-all.)
 * Numbers: the research note's section 7.
 */
import { test, expect } from "../../fixtures";
import type { CDPSession, Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

/** Installed before any page script. Everything is kept in `window.__scrollProbe`. */
function installTimerShim() {
  const P = {
    intervalsCreated: 0,
    intervalsCleared: 0,
    liveIntervals: new Set<number>(),
    timerFires: 0,
    timeoutsCreated: 0,
  };
  (window as unknown as { __scrollProbe: typeof P }).__scrollProbe = P;
  const origSetInterval = window.setInterval.bind(window);
  const origClearInterval = window.clearInterval.bind(window);
  const origSetTimeout = window.setTimeout.bind(window);
  window.setInterval = ((fn: TimerHandler, ms?: number, ...rest: unknown[]) => {
    P.intervalsCreated++;
    const wrapped =
      typeof fn === "function"
        ? (...a: unknown[]) => {
            P.timerFires++;
            return (fn as (...x: unknown[]) => unknown)(...a);
          }
        : fn;
    const id = origSetInterval(wrapped as TimerHandler, ms, ...rest);
    P.liveIntervals.add(id);
    return id;
  }) as typeof window.setInterval;
  window.clearInterval = ((id?: number) => {
    if (id !== undefined && P.liveIntervals.delete(id)) P.intervalsCleared++;
    return origClearInterval(id);
  }) as typeof window.clearInterval;
  window.setTimeout = ((fn: TimerHandler, ms?: number, ...rest: unknown[]) => {
    P.timeoutsCreated++;
    const wrapped =
      typeof fn === "function"
        ? (...a: unknown[]) => {
            P.timerFires++;
            return (fn as (...x: unknown[]) => unknown)(...a);
          }
        : fn;
    return origSetTimeout(wrapped as TimerHandler, ms, ...rest);
  }) as typeof window.setTimeout;
}

async function probe(page: Page) {
  return page.evaluate(() => {
    const P = (window as unknown as { __scrollProbe: { liveIntervals: Set<number>; intervalsCreated: number; timerFires: number; timeoutsCreated: number } }).__scrollProbe;
    return { live: P.liveIntervals.size, created: P.intervalsCreated, fires: P.timerFires, timeouts: P.timeoutsCreated };
  });
}

async function metrics(cdp: CDPSession) {
  const { metrics } = await cdp.send("Performance.getMetrics");
  const get = (n: string) => metrics.find((m) => m.name === n)?.value ?? 0;
  return { layouts: get("LayoutCount"), recalcs: get("RecalcStyleCount") };
}

async function openHome(page: Page) {
  await page.addInitScript(installTimerShim);
  await page.goto(`${BASE_URL}/`, { timeout: 20 * 60 * 1000 });
  // Hydrated and fully rendered: the masonry blocks and every gallery card are mounted.
  await expect(page.locator(".dx-component-card").first()).toBeVisible({ timeout: 5 * 60 * 1000 });
  await expect(page.locator(".dx-widget-card").first()).toBeVisible();
  await page.waitForFunction(() => document.querySelectorAll(".dx-component-card").length >= 60, undefined, { timeout: 120_000 });
}

/** The music-player block's elapsed-time label (the left of its two `m:ss` spans). */
async function playerLabel(page: Page): Promise<string> {
  return page.evaluate(() => {
    const card = [...document.querySelectorAll(".dx-widget-card")].find((c) => c.textContent?.includes("Midnight City"));
    const spans = [...(card?.querySelectorAll("span") ?? [])].map((s) => s.textContent ?? "").filter((t) => /^\d+:\d\d$/.test(t));
    return spans[0] ?? "";
  });
}
const seconds = (label: string) => {
  const [m, s] = label.split(":").map(Number);
  return m * 60 + s;
};

test.describe("scroll smoothness of / (counts)", () => {
  test("1. page timers do not grow with the page's age", async ({ page }) => {
    await openHome(page);
    await page.waitForTimeout(3000); // settle: first renders, entrance animations
    const a = await probe(page);
    await page.waitForTimeout(6000);
    const b = await probe(page);
    await page.waitForTimeout(6000);
    const c = await probe(page);

    console.log(`[timers] live ${a.live}->${b.live}->${c.live}, created ${a.created}->${c.created}, fires/s ${((b.fires - a.fires) / 6).toFixed(1)} then ${((c.fires - b.fires) / 6).toFixed(1)}, setTimeout calls ${a.timeouts}->${c.timeouts}`);
    // Live intervals: constant (HEAD: +1 about every 1.2 s, never cleared).
    expect(c.live, `live setIntervals over 12 s idle: ${a.live} -> ${b.live} -> ${c.live}`).toBe(a.live);
    expect(c.created, `setIntervals created over 12 s idle: ${a.created} -> ${c.created}`).toBe(a.created);

    // Timer wakeups per second: bounded, and not climbing with age (HEAD: ~58/s at 8 s, ~200/s at 25 s).
    const firstHalf = (b.fires - a.fires) / 6;
    const secondHalf = (c.fires - b.fires) / 6;
    expect(firstHalf, `timer fires/s 3-9 s: ${firstHalf.toFixed(1)}`).toBeLessThanOrEqual(12);
    expect(secondHalf, `timer fires/s 9-15 s: ${secondHalf.toFixed(1)}`).toBeLessThanOrEqual(Math.max(12, firstHalf * 1.5));
  });

  test("2. the player block's readout advances and its play/pause works", async ({ page }) => {
    await openHome(page);
    const start = await playerLabel(page);
    expect(start, "the player block is on the page").toMatch(/^\d+:\d\d$/);
    // HEAD: stuck at 1:24 -- every re-render allocated a fresh signal at 84 s.
    await expect.poll(async () => seconds(await playerLabel(page)) - seconds(start), { timeout: 15_000, intervals: [500] }).toBeGreaterThanOrEqual(3);

    const card = page.locator(".dx-widget-card", { hasText: "Midnight City" });
    await card.getByRole("button", { name: "Play or pause" }).click();
    const paused = seconds(await playerLabel(page));
    await page.waitForTimeout(3500);
    expect(seconds(await playerLabel(page)), "paused: the readout holds still").toBeLessThanOrEqual(paused + 1);
    await card.getByRole("button", { name: "Play or pause" }).click();
    await expect.poll(async () => seconds(await playerLabel(page)) - paused, { timeout: 15_000, intervals: [500] }).toBeGreaterThanOrEqual(2);
  });

  test("3. #main has no non-passive wheel / touch listener", async ({ page }) => {
    await openHome(page);
    await page.waitForTimeout(1500);
    const cdp = await page.context().newCDPSession(page);
    const { result } = await cdp.send("Runtime.evaluate", { expression: "document.querySelector('#main')" });
    expect(result.objectId, "#main exists (Dioxus' event root)").toBeTruthy();
    const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId! });
    const blocking = listeners
      .filter((l) => ["wheel", "mousewheel", "touchstart", "touchmove"].includes(l.type) && !l.passive)
      .map((l) => `${l.type}${l.useCapture ? " (capture)" : ""}`);
    expect(blocking, "non-passive wheel/touch listeners on #main (each makes every scroll tick wait for the main thread)").toEqual([]);
  });

  test("5. off-screen demo cards are skipped, and nothing relayouts the idle page", async ({ page }) => {
    await openHome(page);
    const cv = await page.evaluate(() => {
      const of = (sel: string) => [...new Set([...document.querySelectorAll(sel)].map((e) => getComputedStyle(e).contentVisibility))];
      return {
        component: of(".dx-component-card"),
        widget: of(".dx-widget-card:not(.dx-widget-card-popout)"),
        popout: of(".dx-widget-card-popout"),
      };
    });
    expect(cv.component, ".dx-component-card").toEqual(["auto"]);
    expect(cv.widget, ".dx-widget-card (non-popout)").toEqual(["auto"]);
    expect(cv.popout, ".dx-widget-card-popout keeps `visible` (paint containment would clip its dropdown)").toEqual(["visible"]);

    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Performance.enable");

    // At the top of the page, only the first viewport's cards take part in rendering; everything else is
    // skipped (no style/layout/paint, their animations stop costing frames). HEAD: 0 of 70 skipped.
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.waitForTimeout(1500);
    const skippedAtTop = await page.evaluate(
      () => [...document.querySelectorAll(".dx-component-card-preview")].filter((e) => !(e as HTMLElement).checkVisibility({ contentVisibilityAuto: true })).length,
    );
    const total = await page.locator(".dx-component-card").count();
    expect(skippedAtTop, `of ${total} gallery cards, ${skippedAtTop} are skipped at the top of the page`).toBeGreaterThanOrEqual(Math.floor(total * 0.7));

    // Idle at the top: a ticking demo or a pulse in an off-screen card must not keep re-laying-out the page.
    // HEAD (release): ~28 layouts/s (the leaked 100 ms intervals); cards un-skipped: ~43/s on a debug build.
    const m0 = await metrics(cdp);
    await page.waitForTimeout(4000);
    const m1 = await metrics(cdp);
    const idleLayouts = m1.layouts - m0.layouts;
    expect(idleLayouts, `layouts over 4 s idle at the top (${idleLayouts}); HEAD ~110`).toBeLessThanOrEqual(24);

    // A scripted top-to-bottom scroll (real wheel input). Counts are reported, not capped: on any build the
    // scroll window still holds ~1 layout per card entering/leaving view and the on-screen Progress demo's
    // 250 ms `width` transition, which is O(card) now (the cost that moved is PrePaint/Layerize per frame,
    // which is time, not a count; see the research note and `scripts/measure-scroll.mjs`).
    await page.mouse.move(40, 400);
    const before = await metrics(cdp);
    const atBottom = () => page.evaluate(() => scrollY >= document.scrollingElement!.scrollHeight - innerHeight - 4);
    let ticks = 0;
    for (; ticks < 1200 && !(ticks % 8 === 0 && (await atBottom())); ticks++) {
      await page.mouse.wheel(0, 200);
      await page.waitForTimeout(16);
    }
    await page.waitForTimeout(800); // let the last smooth-scroll animation land
    const after = await metrics(cdp);
    const pageHeight = await page.evaluate(() => document.scrollingElement!.scrollHeight - innerHeight);
    expect(await page.evaluate(() => scrollY), "the scripted scroll reached the bottom").toBeGreaterThan(pageHeight - 400);
    const skippedAtBottom = await page.evaluate(
      () => [...document.querySelectorAll(".dx-component-card-preview")].filter((e) => !(e as HTMLElement).checkVisibility({ contentVisibilityAuto: true })).length,
    );
    expect(skippedAtBottom, `${skippedAtBottom} of ${total} cards are skipped at the bottom of the page`).toBeGreaterThanOrEqual(Math.floor(total * 0.7));
    const summary = `idle layouts/4s ${idleLayouts}; scroll: layouts ${after.layouts - before.layouts}, style recalcs ${after.recalcs - before.recalcs}, ${ticks} wheel ticks, page ${pageHeight}px; skipped cards top/bottom ${skippedAtTop}/${skippedAtBottom} of ${total}`;
    test.info().annotations.push({ type: "scroll-counts", description: summary });
    console.log(`[scroll-counts] ${summary}`);
  });

  test("6. component-page variants are NOT skipped (their demos measure themselves on mount)", async ({ page }) => {
    // `.dx-component-variant`: the stacked "Variants" demos under a component's main demo (the chart pages
    // have up to 13 live demos each). They are deliberately outside the `content-visibility: auto` rule
    // (main.css): a skipped subtree has no layout box, so a carousel or a chart measured 0 at mount and
    // never recovered -- carousel-virtual-wheel / carousel / chart_tooltip `default_index` went red
    // (round-2 verification, 2026-10-04). The home cards DO keep the skip (tests 1-5).
    await page.goto(`${BASE_URL}/component/area_chart/?`, { timeout: 20 * 60 * 1000 });
    await page.waitForFunction(() => document.querySelectorAll(".dx-component-variant").length >= 5, undefined, { timeout: 5 * 60 * 1000 });
    await page.waitForTimeout(1500);
    const r = await page.evaluate(() => {
      const v = [...document.querySelectorAll(".dx-component-variant")] as HTMLElement[];
      return {
        n: v.length,
        cv: [...new Set(v.map((e) => getComputedStyle(e).contentVisibility))],
        skipped: v.filter((e) => !(e.firstElementChild as HTMLElement).checkVisibility({ contentVisibilityAuto: true })).length,
      };
    });
    expect(r.cv, ".dx-component-variant").toEqual(["visible"]);
    expect(r.skipped, `of ${r.n} variants, ${r.skipped} are skipped`).toBe(0);
  });
});

test.describe("a touch drag does not need a touchstart handler", () => {
  test.use({ hasTouch: true });

  /** Real touch input (CDP), a drag that is mostly VERTICAL so an un-prevented pan would scroll the page. */
  async function touchDrag(page: Page, cdp: CDPSession, x: number, y: number, dx: number, dy: number) {
    const send = (type: string, pts: { x: number; y: number }[]) =>
      cdp.send("Input.dispatchTouchEvent", { type, touchPoints: pts.map((p, i) => ({ ...p, id: i + 1 })) });
    await send("touchStart", [{ x, y }]);
    for (let s = 1; s <= 10; s++) await send("touchMove", [{ x: x + (dx * s) / 10, y: y + (dy * s) / 10 }]);
    await send("touchEnd", []);
    await page.waitForTimeout(150);
  }

  async function prepare(page: Page, url: string, { scrollable = true } = {}) {
    await page.goto(`${BASE_URL}${url}`, { timeout: 20 * 60 * 1000 });
    // Make the page scrollable by plenty, so "it did not scroll" is a real statement. (Not for the
    // color picker: its popover scroll-locks the page anyway, and a taller page moves where the
    // anchored popover, and so the thumb, lands.)
    if (scrollable) {
      await page.evaluate(() => {
        document.body.style.minHeight = "6000px";
        window.scrollTo(0, 0);
      });
    }
    return await page.context().newCDPSession(page);
  }

  const noScroll = async (page: Page) => {
    expect(await page.evaluate(() => scrollY), "the page did not pan").toBe(0);
    expect(await page.evaluate(() => getSelection()?.toString() ?? ""), "nothing got selected").toBe("");
  };

  test("slider thumb", async ({ page }) => {
    const cdp = await prepare(page, "/component/?name=slider&");
    const thumb = page.getByRole("slider").first();
    await thumb.scrollIntoViewIfNeeded();
    await page.evaluate(() => window.scrollTo(0, 0));
    const box = (await thumb.boundingBox())!;
    const before = Number(await thumb.getAttribute("aria-valuenow"));
    await touchDrag(page, cdp, box.x + box.width / 2, box.y + box.height / 2, 60, 140);
    expect(Number(await thumb.getAttribute("aria-valuenow")), "the drag moved the value").not.toBe(before);
    await noScroll(page);
  });

  test("resizable handle", async ({ page }) => {
    const cdp = await prepare(page, "/component/?name=resizable&");
    const handle = page.getByRole("separator", { name: "Sidebar" });
    await handle.scrollIntoViewIfNeeded();
    await page.evaluate(() => window.scrollTo(0, 0));
    // Same geometry as resizable.spec.ts's real-touch case: a finger on the grip's edge (coarse hit area).
    const box = (await handle.boundingBox())!;
    const grip = (await handle.locator(".dx-resizable-handle-grip").boundingBox())!;
    const left = page.locator(`#${await handle.getAttribute("aria-controls")}`); // the handle's primary pane
    const w0 = (await left.boundingBox())!.width;
    await touchDrag(page, cdp, box.x + box.width / 2 - 5, grip.y + grip.height / 2, 50, 140);
    expect(Math.abs((await left.boundingBox())!.width - w0), "the drag resized the panel").toBeGreaterThan(20);
    await noScroll(page);
  });

  test("color area thumb", async ({ page }) => {
    const cdp = await prepare(page, "/component/?name=color_picker&", { scrollable: false });
    await page.getByRole("button", { name: /Color picker/i }).first().click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    // The popover scroll-locks the page while open (a window `touchmove` preventDefault), so a "did not
    // pan" check would pass for that reason alone: here the drag moving the color and selecting nothing
    // is the evidence; the scroll half is proven by the slider and the resizable handle.
    const thumb = dialog.locator(".dx-color-picker-area-thumb").first();
    await expect(thumb).toBeVisible();
    const box = (await thumb.boundingBox())!;
    const hex = page.locator("#color_field");
    const before = await hex.inputValue();
    await touchDrag(page, cdp, box.x + box.width / 2, box.y + box.height / 2, 30, 100);
    expect(await hex.inputValue(), "the drag moved the color").not.toBe(before);
    expect(await page.evaluate(() => getSelection()?.toString() ?? ""), "nothing got selected").toBe("");
  });
});
