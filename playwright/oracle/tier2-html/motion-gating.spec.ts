/**
 * ORACLE: tier 2 (HTML) -- motion nobody can see does not run, in the components themselves.
 *
 * Source: the owner's brief (2026-10-10): "animations running in the background are not about cards,
 * but the actual components and their animations". Rule sources:
 *   - Web Animations: `Animation.playState` is `"paused"` once `animation-play-state: paused` applies,
 *     and a paused CSS animation keeps its phase and resumes from it:
 *       https://www.w3.org/TR/css-animations-1/#animation-play-state
 *       https://www.w3.org/TR/web-animations-1/#play-states
 *   - Intersection Observer: an element outside the root (plus `rootMargin`) is not intersecting:
 *       https://www.w3.org/TR/intersection-observer/
 *   - Page Visibility: `visibilitychange` on `document` when the tab is hidden:
 *       https://www.w3.org/TR/page-visibility-2/
 *
 * The construction under test is `primitives/src/activity.rs` (one shared IntersectionObserver +
 * `visibilitychange` + `contentvisibilityautostatechange`), which components subscribe to:
 *   1. CSS LOOPS: a spinner / skeleton far outside the viewport reports `playState: "paused"` and
 *      `data-dx-motion="paused"`, and runs again (same animation object, phase kept) once scrolled in.
 *   2. CAROUSEL AUTOPLAY: no slide change while the carousel is off-screen or the tab is hidden; on
 *      return a FRESH countdown (no instant jump), then it advances again.
 *   3. TICKER: the progress demo's 1 Hz tick makes zero timer callbacks while off-screen.
 *   4. TOAST: auto-dismiss is NOT affected by scrolling, but is dropped while the tab is hidden and
 *      restarts in full on return (a timer that carries meaning).
 *   5. CHART ENTRANCE: a chart below the fold has NOT animated (`data-animate` absent) before it is
 *      actually in view (>= 40% visible, no pre-roll margin) and animates once when it is.
 *   6. LISTENERS: the registry's two document listeners exist only while something is subscribed, and
 *      the inventory is back to its baseline after the subscribers unmount.
 *
 * "Tab hidden" is simulated by overriding `document.visibilityState` and dispatching `visibilitychange`
 * (a headless page cannot be backgrounded), which is exactly what the registry reads.
 *
 * Counts and states only (load-proof, valid on a debug build). Chromium only (CDP for the inventory):
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts \
 *     ./oracle/tier2-html/motion-gating.spec.ts
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

const NAV = { timeout: 20 * 60 * 1000 };

test.describe("motion gating", () => {
  test.skip(({ browserName }) => browserName !== "chromium", "CDP + checkVisibility are Chromium-only");

  /** Pushes `selector` far outside the viewport without touching the DOM Dioxus owns. */
  async function park(page: Page, selector: string) {
    await page.evaluate((sel) => {
      const style = document.createElement("style");
      style.id = "zz-park";
      style.textContent = `${sel} { transform: translateY(9000px) !important; }`;
      document.head.appendChild(style);
    }, selector);
  }
  async function unpark(page: Page) {
    await page.evaluate(() => document.getElementById("zz-park")?.remove());
  }
  async function setHidden(page: Page, hidden: boolean) {
    await page.evaluate((h) => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => (h ? "hidden" : "visible") });
      Object.defineProperty(document, "hidden", { configurable: true, get: () => h });
      document.dispatchEvent(new Event("visibilitychange"));
    }, hidden);
  }

  test("a spinner and a skeleton far off-screen are paused, and run again once scrolled in", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=spinner&`, NAV);
    const spinner = page.locator(".dx-spinner").first();
    await expect(spinner).toBeVisible({ timeout: 5 * 60 * 1000 });
    const state = () =>
      spinner.evaluate((el) => ({
        attr: el.getAttribute("data-dx-motion"),
        states: el.getAnimations({ subtree: true }).map((a) => a.playState),
      }));
    // In view: running, not paused.
    await expect.poll(async () => (await state()).states.includes("running")).toBe(true);
    expect((await state()).attr).toBeNull();

    await park(page, ".dx-spinner");
    await expect.poll(async () => (await state()).attr, { timeout: 5000 }).toBe("paused");
    expect((await state()).states.every((s) => s === "paused")).toBe(true);

    await unpark(page);
    await expect.poll(async () => (await state()).attr, { timeout: 5000 }).toBeNull();
    expect((await state()).states.includes("running")).toBe(true);

    // Same for the skeleton (a different component, same construction).
    await page.goto(`${BASE_URL}/component/?name=skeleton&`, NAV);
    const skeleton = page.locator(".dx-skeleton").first();
    await expect(skeleton).toBeVisible({ timeout: 5 * 60 * 1000 });
    await park(page, ".dx-skeleton");
    await expect.poll(() => skeleton.getAttribute("data-dx-motion"), { timeout: 5000 }).toBe("paused");
    expect(await skeleton.evaluate((el) => el.getAnimations().map((a) => a.playState))).toEqual(
      expect.arrayContaining(["paused"]),
    );
    await unpark(page);
    await expect.poll(() => skeleton.getAttribute("data-dx-motion"), { timeout: 5000 }).toBeNull();
  });

  test("a hidden tab pauses a visible spinner", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=spinner&`, NAV);
    const spinner = page.locator(".dx-spinner").first();
    await expect(spinner).toBeVisible({ timeout: 5 * 60 * 1000 });
    await expect.poll(() => spinner.getAttribute("data-dx-motion")).toBeNull();
    await setHidden(page, true);
    await expect.poll(() => spinner.getAttribute("data-dx-motion"), { timeout: 5000 }).toBe("paused");
    await setHidden(page, false);
    await expect.poll(() => spinner.getAttribute("data-dx-motion"), { timeout: 5000 }).toBeNull();
  });

  test("carousel autoplay: no slide change off-screen or in a hidden tab, then a fresh countdown", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=carousel&variant=autoplay&`, NAV);
    const root = '[aria-label="Autoplay gallery"]';
    const label = () =>
      page
        .locator(`${root} [role="group"][aria-roledescription="slide"][data-selected="true"]`)
        .first()
        .getAttribute("aria-label");
    await expect.poll(label, { timeout: 5 * 60 * 1000 }).not.toBeNull();
    // The variant sits below the fold of the stacked page, where its autoplay is (correctly) stopped.
    expect(await page.locator(root).evaluate((el) => el.getBoundingClientRect().top)).toBeGreaterThan(800);
    await page.evaluate((sel) => document.querySelector(sel)?.scrollIntoView({ block: "center", behavior: "instant" }), root);
    // delay_ms is 1200 in this demo: it advances on its own while visible.
    const first = await label();
    await expect.poll(label, { timeout: 4000 }).not.toBe(first);

    // Off-screen: nothing changes for three delays.
    await park(page, root);
    await expect.poll(() => page.evaluate(() => !!(window as any).__dxActivity)).toBe(true);
    await page.waitForTimeout(500); // observer report + the task being dropped
    const parked = await label();
    await page.waitForTimeout(3800);
    expect(await label(), "no slide change while off-screen").toBe(parked);

    // Back in view: fresh countdown, so no change within the first ~700 ms, then it advances.
    await unpark(page);
    await page.waitForTimeout(700);
    expect(await label(), "no instant jump on return").toBe(parked);
    await expect.poll(label, { timeout: 4000 }).not.toBe(parked);

    // Hidden tab: same.
    await setHidden(page, true);
    await page.waitForTimeout(500);
    const hiddenAt = await label();
    await page.waitForTimeout(3800);
    expect(await label(), "no slide change in a hidden tab").toBe(hiddenAt);
    await setHidden(page, false);
    await page.waitForTimeout(700);
    expect(await label(), "no instant jump on return").toBe(hiddenAt);
    await expect.poll(label, { timeout: 4000 }).not.toBe(hiddenAt);
  });

  test("a ticker demo makes zero 1 s timer callbacks while off-screen", async ({ page }) => {
    await page.addInitScript(() => {
      const w = window as any;
      w.__ticks = 0;
      const real = window.setTimeout.bind(window);
      (window as any).setTimeout = (fn: any, ms?: number, ...rest: any[]) => {
        if (typeof ms === "number" && ms >= 900 && ms <= 1100 && typeof fn === "function") {
          return real(
            (...a: any[]) => {
              w.__ticks++;
              return fn(...a);
            },
            ms,
            ...rest,
          );
        }
        return real(fn, ms, ...rest);
      };
    });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=progress&`, NAV);
    const bar = page.getByRole("progressbar", { name: "Progressbar Demo" }).first();
    await expect(bar).toBeVisible({ timeout: 5 * 60 * 1000 });
    const value = () => bar.getAttribute("aria-valuenow");
    const v0 = await value();
    await expect.poll(value, { timeout: 4000 }).not.toBe(v0); // it ticks while visible

    await park(page, "[data-dx-motion-key]"); // the demo's wrapper carries the key, not the bar
    await page.waitForTimeout(800);
    const ticks = await page.evaluate(() => (window as any).__ticks as number);
    const parkedValue = await value();
    await page.waitForTimeout(3500);
    expect(await page.evaluate(() => (window as any).__ticks as number), "no 1 s timer callbacks off-screen").toBe(ticks);
    expect(await value(), "no state change off-screen").toBe(parkedValue);

    await unpark(page);
    await expect.poll(value, { timeout: 5000 }).not.toBe(parkedValue);
  });

  test("toast auto-dismiss ignores scrolling but not a hidden tab", async ({ page }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=toast&`, NAV);
    // The demo's own toast lasts 60 s: use the provider's default duration (5 s) through the
    // top-layer fixture instead if it exists, else shorten with the page-level API.
    const trigger = page.getByRole("button", { name: "Info (60s)" });
    await expect(trigger).toBeVisible({ timeout: 5 * 60 * 1000 });
    // 60 s is too long to wait for; speed the toast's own sleep up by scaling setTimeout for the
    // toast's duration only (the Rust task sleeps through `setTimeout(60000)`).
    await page.evaluate(() => {
      const real = window.setTimeout.bind(window);
      (window as any).setTimeout = (fn: any, ms?: number, ...rest: any[]) =>
        real(fn, ms === 60000 ? 4000 : ms, ...rest);
    });
    const toasts = page.locator('[role="alertdialog"]');

    // 1. Scrolling does not shorten or extend it: it is gone ~4 s after it appeared.
    await trigger.click();
    await expect(toasts).toHaveCount(1);
    const t0 = Date.now();
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    await expect(toasts).toHaveCount(0, { timeout: 8000 });
    const elapsed = Date.now() - t0;
    expect(elapsed, "dismissed on schedule although the page was scrolled").toBeGreaterThan(3000);
    expect(elapsed).toBeLessThan(6500);

    // 2. A hidden tab holds it; on return the countdown starts over.
    await page.evaluate(() => window.scrollTo(0, 0));
    await trigger.click();
    await expect(toasts).toHaveCount(1);
    await page.waitForTimeout(1000);
    await setHidden(page, true);
    await page.waitForTimeout(6000); // longer than the whole duration
    await expect(toasts, "still there after a hidden tab outlasted its duration").toHaveCount(1);
    await setHidden(page, false);
    const back = Date.now();
    await page.waitForTimeout(2500);
    await expect(toasts, "fresh countdown: not yet gone 2.5 s after return").toHaveCount(1);
    await expect(toasts).toHaveCount(0, { timeout: 6000 });
    expect(Date.now() - back).toBeGreaterThan(3000);
  });

  test("a chart below the fold has not animated; it animates once when it is actually in view", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=bar_chart&`, NAV);
    const svgs = page.locator('[data-slot="chart-svg"]');
    await expect(svgs.first()).toBeAttached({ timeout: 5 * 60 * 1000 });
    await page.waitForTimeout(2500); // measured renders settle
    const count = await svgs.count();
    expect(count, "the page stacks several charts").toBeGreaterThan(2);

    // Pick the chart furthest down; it is below the fold at the top of the page.
    const target = count - 1;
    const geometry = () =>
      svgs.nth(target).evaluate((el) => {
        const r = el.getBoundingClientRect();
        return { top: r.top + scrollY, height: r.height, vh: innerHeight };
      });
    await page.evaluate(() => window.scrollTo(0, 0));
    const g = await geometry();
    expect(g.top, "the last chart starts below the first screen").toBeGreaterThan(g.vh);
    const animated = () => svgs.nth(target).getAttribute("data-animate");
    expect(await animated(), "not animated before it is in view").toBeNull();

    // 10% visible: inside what a pre-roll margin would have started, but the entrance must wait.
    await page.evaluate((y) => window.scrollTo(0, y), g.top - g.vh + g.height * 0.1);
    await page.waitForTimeout(800);
    expect(await animated(), "10% visible is not 'seen'").toBeNull();

    // 70% visible: now it plays, once.
    await page.evaluate((y) => window.scrollTo(0, y), g.top - g.vh + g.height * 0.7);
    await expect.poll(animated, { timeout: 5000 }).toBe("true");

    // And never again: scrolling away and back does not reset it.
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.waitForTimeout(500);
    expect(await animated()).toBe("true");
  });

  test("a hidden tab holds a chart's entrance until the tab is visible", async ({ page }) => {
    // The first chart sits 605 px down the page and is 342 px tall. The entrance needs 40% of it on
    // screen (ENTER_RATIO), so the default 1280x720 viewport shows only 34% of it and rightly never
    // enters; 800 px shows 57%. Pin the viewport like the sibling chart test does.
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.addInitScript(() => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
      Object.defineProperty(document, "hidden", { configurable: true, get: () => true });
    });
    await page.goto(`${BASE_URL}/component/?name=bar_chart&`, NAV);
    const first = page.locator('[data-slot="chart-svg"]').first();
    await expect(first).toBeAttached({ timeout: 5 * 60 * 1000 });
    await page.waitForTimeout(2500);
    expect(await first.getAttribute("data-animate"), "no entrance while the tab is hidden").toBeNull();
    await setHidden(page, false);
    await expect.poll(() => first.getAttribute("data-animate"), { timeout: 5000 }).toBe("true");
  });

  test("reduced motion stops every loop and keeps a still 'loading' shape", async ({ page }) => {
    // One rule in dx-components-theme.css (`[data-dx-motion-key]` hosts) stops the loops of the
    // components that call `use_motion`; the sheets of the others carry their own block. Row 176.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(`${BASE_URL}/component/?name=spinner&`, NAV);
    const spinner = page.locator(".dx-spinner").first();
    await expect(spinner).toBeVisible({ timeout: 5 * 60 * 1000 });
    await page.waitForTimeout(500);
    expect(
      await spinner.evaluate((el) => ({
        running: el.getAnimations({ subtree: true }).length,
        name: getComputedStyle(el.querySelector("svg")!).animationName,
        iconBox: el.querySelector("svg")!.getBoundingClientRect().width,
      })),
      "a spinner under reduced motion is a still ring, still drawn",
    ).toEqual({ running: 0, name: "none", iconBox: 16 });

    // Skeleton, indeterminate progress and a loading avatar: build each as the components do (the
    // host carries `data-dx-motion-key`, `use_motion`'s marker) on the page that loads its sheet.
    const cases = [
      { page: "skeleton", html: '<div id="zz" class="dx-skeleton" data-dx-motion-key="zz1" style="height:1rem;width:5rem"></div>', target: "#zz" },
      {
        page: "progress",
        html: '<div id="zz" class="dx-progress" data-state="indeterminate" data-dx-motion-key="zz2"><div class="dx-progress-indicator"></div></div>',
        target: "#zz .dx-progress-indicator",
      },
      { page: "avatar", html: '<div id="zz" class="dx-avatar" data-state="loading" data-dx-motion-key="zz3" style="height:2rem;width:2rem"></div>', target: "#zz" },
    ];
    for (const c of cases) {
      await page.goto(`${BASE_URL}/component/?name=${c.page}&`, NAV);
      await expect(page.locator("main").first()).toBeVisible({ timeout: 5 * 60 * 1000 });
      await page.waitForTimeout(800);
      const r = await page.evaluate(({ html, target }) => {
        const host = document.createElement("div");
        host.innerHTML = html;
        document.body.appendChild(host);
        const el = document.querySelector(target)!;
        const out = {
          name: getComputedStyle(el).animationName,
          running: el.getAnimations().length,
          fill: el.getBoundingClientRect().width / (el.parentElement!.getBoundingClientRect().width || 1),
        };
        host.remove();
        return out;
      }, c);
      expect(r.name, `${c.page}: no animation under reduced motion`).toBe("none");
      expect(r.running, `${c.page}: nothing running`).toBe(0);
      if (c.page === "progress") {
        expect(r.fill, "a still indeterminate bar fills the track (it must not read as 50%)").toBeGreaterThan(0.99);
      }
    }
  });

  test("the listener inventory returns to its baseline after the subscribers unmount", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=button&`, NAV);
    await expect(page.locator("main").first()).toBeVisible({ timeout: 5 * 60 * 1000 });
    await page.waitForTimeout(1500);
    const client = await page.context().newCDPSession(page);
    const inventory = async () => {
      const { result } = await client.send("Runtime.evaluate", { expression: "document" });
      const { listeners } = await client.send("DOMDebugger.getEventListeners", { objectId: result.objectId! });
      return listeners
        .map((l) => l.type)
        .filter((t) => t === "visibilitychange" || t === "contentvisibilityautostatechange")
        .sort();
    };
    const baseline = await inventory();

    // In-app navigation (no reload) to a page whose components subscribe, and back.
    await page.goto(`${BASE_URL}/component/?name=spinner&`, NAV);
    await expect(page.locator(".dx-spinner").first()).toBeVisible({ timeout: 5 * 60 * 1000 });
    await expect.poll(inventory).toEqual(["contentvisibilityautostatechange", "visibilitychange"]);
    await expect.poll(() => page.evaluate(() => (window as any).__dxActivity?.subs.size ?? 0)).toBeGreaterThan(0);

    // Unmount without a reload: swap the route through the SPA router.
    // The link's href carries the deployment's base path (`/shadcn-dioxus/...` on the Pages build).
    const basePath = new URL(BASE_URL).pathname.replace(/\/$/, "");
    await page.evaluate((prefix) => {
      const link = [...document.querySelectorAll("a")].find((a) => {
        const href = a.getAttribute("href") ?? "";
        return href.startsWith(`${prefix}/component/button/`);
      });
      (link as HTMLElement | undefined)?.click();
    }, basePath);
    await expect(page.locator(".dx-spinner")).toHaveCount(0, { timeout: 30_000 });
    await expect.poll(() => page.evaluate(() => !!(window as any).__dxActivity), { timeout: 10_000 }).toBe(false);
    expect(await inventory()).toEqual(baseline);
  });
});
