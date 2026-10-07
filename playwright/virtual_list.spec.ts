import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL, buildProfile } from "./base-url";
import { gotoHydrated } from "./hydration";

// Helper to run scrollHeight stability test with configurable tolerance
async function testScrollHeightStability(
  page: import("@playwright/test").Page,
  tolerancePx: number
) {
  await page.goto(`${BASE_URL}/component/?name=virtual_list&`, {
    timeout: 20 * 60 * 1000,
  });

  const container = page.locator(".dx-virtual-list-container").first();
  await expect(container).toBeVisible({ timeout: 30000 });

  // Wait for initial render
  await page.waitForTimeout(500);

  // Get initial state
  const initialState = await container.evaluate((el) => ({
    scrollHeight: el.scrollHeight,
    clientHeight: el.clientHeight,
  }));

  const maxScroll = initialState.scrollHeight - initialState.clientHeight;
  const steps = 20;
  const stepSize = maxScroll / steps;

  const measurements: Array<{ scrollTop: number; scrollHeight: number }> = [];

  // Simulate continuous scrolling (like dragging the scrollbar)
  // by setting scrollTop in rapid succession
  for (let i = 1; i <= steps; i++) {
    const targetScroll = Math.round(stepSize * i);

    await container.evaluate((el, scroll) => {
      el.scrollTop = scroll;
    }, targetScroll);

    // Wait for scroll event to propagate through Rust and re-render
    await page.waitForTimeout(100);

    const state = await container.evaluate((el) => ({
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
    }));

    measurements.push(state);
  }

  // Analyze scrollHeight stability during the "drag"
  // Exclude last measurement since scrollend fires ~150ms after last scroll,
  // which correctly unfreezes the height - we only care about stability DURING scroll
  const duringScrollMeasurements = measurements.slice(0, -1);
  const scrollHeights = duringScrollMeasurements.map((m) => m.scrollHeight);
  const minHeight = Math.min(...scrollHeights);
  const maxHeight = Math.max(...scrollHeights);
  const heightVariance = maxHeight - minHeight;

  console.log(`scrollHeight range: ${minHeight} - ${maxHeight} (variance: ${heightVariance}px)`);
  console.log(`Measurements:`, measurements.map((m, i) =>
    `step ${i + 1}: scrollTop=${m.scrollTop}, scrollHeight=${m.scrollHeight}`
  ).join('\n'));

  // The bug: if scrollHeight changes during scrolling, the scrollbar thumb
  // position (scrollTop / scrollHeight) changes even though the user's mouse
  // hasn't moved proportionally. This causes the thumb to drift from the cursor.
  //
  // For a stable scrollbar, scrollHeight should not change during active scrolling.
  expect(
    heightVariance,
    `scrollHeight changed by ${heightVariance}px during scroll (tolerance: ${tolerancePx}px) - this causes scrollbar thumb to drift from mouse cursor`
  ).toBeLessThan(tolerancePx);

  return { heightVariance, measurements };
}

// Test with adaptive estimation (no estimate_size provided).
// The demo uses 6 repeating item sizes, so adaptive estimation learns quickly.
// Allow small margin since first few items may have slightly off estimates.
test("scrollHeight stable with adaptive estimation", async ({ page }) => {
  // Allow up to 200px variance - adaptive estimation may have small drift
  // as it learns item sizes, but should still be much better than 500px+ without fix
  const { heightVariance } = await testScrollHeightStability(page, 200);

  // Bonus: with repeating sizes, adaptive estimation should actually achieve near-zero variance
  console.log(`Adaptive estimation achieved ${heightVariance}px variance`);
});

// Stricter test - should achieve 0px variance with the stable_total_size fix
test("scrollHeight remains stable during continuous scroll", async ({ page }) => {
  await testScrollHeightStability(page, 100);
});

// Test with random_heights variant which has highly variable item sizes
// This reproduces production failure where adaptive estimation struggles
test("scrollHeight stable with random heights variant", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/block/?name=virtual_list&variant=random_heights`, {
    timeout: 20 * 60 * 1000,
  });

  const container = page.locator(".dx-virtual-list-container").first();
  await expect(container).toBeVisible({ timeout: 30000 });

  // Verify variant is loaded by checking for variant-specific content
  const firstCard = page.getByRole("heading", { level: 3 }).first();
  const cardText = await firstCard.textContent();
  console.log("First card text:", cardText);
  // Random heights variant shows "X repeats" in the heading
  expect(cardText, "Variant should show 'repeats' count").toContain("repeats");

  // Start scrolling immediately - don't wait for measurements
  const initialState = await container.evaluate((el) => ({
    scrollHeight: el.scrollHeight,
    clientHeight: el.clientHeight,
  }));

  // Scroll through first portion of list
  const maxScroll = (initialState.scrollHeight - initialState.clientHeight) * 0.3;
  const steps = 15;
  const stepSize = maxScroll / steps;

  const measurements: Array<{ scrollTop: number; scrollHeight: number }> = [];

  for (let i = 1; i <= steps; i++) {
    const targetScroll = Math.round(stepSize * i);

    await container.evaluate((el, scroll) => {
      el.scrollTop = scroll;
    }, targetScroll);

    await page.waitForTimeout(100);

    const state = await container.evaluate((el) => ({
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
    }));

    measurements.push(state);
  }

  // Analyze stability
  const duringScrollMeasurements = measurements.slice(0, -1);
  const scrollHeights = duringScrollMeasurements.map((m) => m.scrollHeight);
  const minHeight = Math.min(...scrollHeights);
  const maxHeight = Math.max(...scrollHeights);
  const heightVariance = maxHeight - minHeight;

  console.log(`Random heights - scrollHeight range: ${minHeight} - ${maxHeight} (variance: ${heightVariance}px)`);
  console.log(`Measurements:`, measurements.map((m, i) =>
    `step ${i + 1}: scrollTop=${m.scrollTop}, scrollHeight=${m.scrollHeight}`
  ).join('\n'));

  // With random heights and poor early estimates, we intentionally DON'T freeze
  // because a bad frozen value causes worse UX (sudden jumps) than gradual drift.
  // The adaptive estimation will improve as more items are measured, and future
  // scrolls will be stable once enough items are measured (≥20 or ≥10%).
  //
  // This test documents the expected drift behavior - it should improve over time
  // as the estimate converges. For better UX, users should provide estimate_size.
  console.log(`Note: ${heightVariance}px drift is expected with poor early estimates`);

  // Just verify we're not seeing catastrophic variance (e.g., 50000px+)
  expect(
    heightVariance,
    `scrollHeight variance ${heightVariance}px is unexpectedly high`
  ).toBeLessThan(15000);
});

test("virtual list virtualizes rows and updates on scroll", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=virtual_list&`, { timeout: 20 * 60 * 1000 });

  // The `main` demo pins the windowed mode; the page also carries a content-visibility demo
  // that keeps every row in the DOM, so scope to the windowed lists.
  const cards = page.locator('.dx-virtual-list-container[data-virtualize="windowed"]').first().getByRole("listitem");
  await expect(cards.first()).toBeVisible({ timeout: 30000 });

  const initialCount = await cards.count();

  expect(initialCount).toBeGreaterThan(0);
  expect(initialCount).toBeLessThan(2000);

  // Retry scrolling + assertion: re-apply scroll on each retry since WASM
  // re-renders may reset scrollTop, especially on slower engines (WebKit).
  await expect(async () => {
    await page.evaluate(() => {
      document.querySelectorAll('[role="list"]').forEach((c) => {
        if (c.scrollHeight > c.clientHeight + 1) {
          c.scrollTop = 6000;
        }
      });
      window.scrollTo(0, 6000);
    });
    await page.waitForTimeout(300);
    const headings = await page.getByRole("heading", { level: 3 }).allTextContents();
    // After scrolling to offset 6000, at least some items with index > 30
    // should be visible, proving the virtual list responded to the scroll.
    const hasScrolledContent = headings.some((h) => {
      const match = h.match(/#(\d+)/);
      return match != null && parseInt(match[1]) > 30;
    });
    expect(hasScrolledContent).toBe(true);
  }).toPass({ timeout: 15000 });
});

test("resize churn while scrolling does not panic (holds no borrow across resize_item)", async ({ page }) => {
  // Regression for a signal-borrow bug inferred (not directly observed) in
  // `primitives/src/virtual_list.rs`'s onresize handler: it historically held
  // a `measurements.peek()` guard for the full duration of a call to
  // `resize_item`, which writes `item_size_cache` -- a dependency of that
  // same memo. A Rust-level unit test attempting the same shape against the
  // primitives' Store/Memo machinery directly (see
  // `virtual::virtualizer::tests::test_resize_item_while_measurements_memo_peek_held`)
  // did not reproduce a panic, so this is the browser-level fallback oracle:
  // drive real ResizeObserver churn (viewport-width changes reflow every
  // visible card, firing `onresize` for each) concurrently with scrolling
  // (which also touches the same memo), and assert the console never carries
  // a Rust panic or a borrow-checker message.
  const consoleMessages: string[] = [];
  page.on("console", (msg) => consoleMessages.push(msg.text()));
  page.on("pageerror", (err) => consoleMessages.push(`pageerror: ${err.message}`));

  await page.goto(`${BASE_URL}/component/block/?name=virtual_list&variant=random_heights`, {
    timeout: 20 * 60 * 1000,
  });

  const container = page.locator(".dx-virtual-list-container").first();
  await expect(container).toBeVisible({ timeout: 30000 });
  await page.waitForTimeout(500);

  const initialState = await container.evaluate((el) => ({
    scrollHeight: el.scrollHeight,
    clientHeight: el.clientHeight,
  }));
  const maxScroll = Math.max(0, initialState.scrollHeight - initialState.clientHeight);

  const widths = [1280, 800, 1280, 640, 1024, 900, 1280];
  for (let i = 0; i < widths.length; i++) {
    await page.setViewportSize({ width: widths[i], height: 800 });
    // Scroll while the resize-triggered remeasurement is still in flight, so
    // the scroll-driven memo read and the resize-driven cache write race.
    const target = Math.round((maxScroll * (i + 1)) / widths.length);
    await container.evaluate((el, scroll) => {
      el.scrollTop = scroll;
    }, target);
    await page.waitForTimeout(80);
  }
  await page.waitForTimeout(400);

  const suspicious = consoleMessages.filter(
    (m) => /panicked/i.test(m) || /already borrowed/i.test(m) || /BorrowMutError/i.test(m)
  );
  expect(suspicious, `Suspicious console output during resize churn:\n${suspicious.join("\n")}`).toEqual([]);
});

test.describe("Axe automated scan", () => {
  // Virtual list has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=virtual_list&`, { timeout: 20 * 60 * 1000 });
    await expect(page.locator(".dx-virtual-list-container").first()).toBeVisible({ timeout: 30000 });
    // The page carries every variant, so this scans the content-visibility list (1,000 rows in
    // the DOM) and the 100,000-row windowed list too. (The bare `/component/block/` pages have
    // no `<main>`, which the scan waits for.)
    await expectNoAxeViolations(page, "virtual_list: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// ---------------------------------------------------------------------------
// Modes (backlog row 143): `ContentVisibility` keeps every row in the DOM and lets the browser
// skip the off-screen ones; `Windowed` mounts only a window. Each has its own demo variant
// with a live "rows in the DOM / Rust renders of the list" readout (`data-vl-rows`,
// `data-vl-renders`); the windowed one also has buttons that change the row count (the default
// `Auto` mode switches modes at 1,000 rows) and unmount the list.
// ---------------------------------------------------------------------------

const CV_URL = `${BASE_URL}/component/block/?name=virtual_list&variant=content_visibility`;
const WINDOWED_URL = `${BASE_URL}/component/block/?name=virtual_list&variant=windowed`;

const cvList = (page: Page) => page.locator('[data-vl-demo="content-visibility"] [role="list"]');
const windowedList = (page: Page) => page.locator('[data-vl-demo="windowed"] [role="list"]');
const readout = (page: Page, demo: string, field: "rows" | "renders" | "mode") =>
  page.locator(`[data-vl-demo="${demo}"] [data-vl-${field}]`);
const numberIn = async (locator: Locator) => Number((await locator.textContent())?.trim());

/**
 * Records, from before any script runs, which `window` `resize` listeners and which `scroll`
 * listeners on `[role="list"]` elements are live. Playwright cannot ask the page what listeners
 * it holds, so the shim wraps add/removeEventListener (keyed by listener identity; capture
 * flags are not distinguished, which is fine for the list's own `{ passive: true }` pair).
 */
async function trackListeners(page: Page) {
  await page.addInitScript(() => {
    const resize = new Set<unknown>();
    const listScroll = new Set<unknown>();
    (window as unknown as Record<string, unknown>).__dxListeners = { resize, listScroll };
    const add = EventTarget.prototype.addEventListener;
    const remove = EventTarget.prototype.removeEventListener;
    const watched = (target: EventTarget, type: string) =>
      target === window && type === "resize"
        ? resize
        : type === "scroll" && target instanceof Element && target.getAttribute("role") === "list"
          ? listScroll
          : null;
    EventTarget.prototype.addEventListener = function (type: string, listener: unknown, options?: unknown) {
      if (listener) watched(this, type)?.add(listener);
      return add.call(this, type, listener as EventListener, options as AddEventListenerOptions);
    };
    EventTarget.prototype.removeEventListener = function (type: string, listener: unknown, options?: unknown) {
      if (listener) watched(this, type)?.delete(listener);
      return remove.call(this, type, listener as EventListener, options as EventListenerOptions);
    };
  });
}

const liveListeners = (page: Page) =>
  page.evaluate(() => {
    const l = (window as unknown as { __dxListeners: { resize: Set<unknown>; listScroll: Set<unknown> } }).__dxListeners;
    return { resize: l.resize.size, listScroll: l.listScroll.size };
  });

/** Scroll a container by `total` px in `step` px increments, one per frame, then let it settle. */
async function scrollAndCount(list: Locator, total: number, step: number) {
  return list.evaluate(
    async (el, [totalPx, stepPx]) => {
      let mutations = 0;
      const observer = new MutationObserver((records) => (mutations += records.length));
      observer.observe(el, { subtree: true, childList: true, attributes: true, characterData: true });
      let scrollEvents = 0;
      const onScroll = () => scrollEvents++;
      el.addEventListener("scroll", onScroll, { passive: true });
      const start = el.scrollTop;
      for (let y = stepPx; y <= totalPx; y += stepPx) {
        el.scrollTop = start + y;
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      // The scroll-end debounce is 600ms, and a correction for rows measured above the
      // viewport can start one more scroll after it.
      await new Promise((resolve) => setTimeout(resolve, 2000));
      el.removeEventListener("scroll", onScroll);
      mutations += observer.takeRecords().length;
      observer.disconnect();
      return { mutations, scrollEvents, moved: el.scrollTop - start };
    },
    [total, step],
  );
}

/**
 * Rows that are actually rendered, i.e. not inside a chunk that `content-visibility` skips.
 * (A skipped chunk's own box is laid out; it is its *contents*, the rows, that are skipped.)
 */
const renderedRows = (list: Locator) =>
  list.evaluate(
    (el) =>
      Array.from(el.querySelectorAll<HTMLElement>('[role="listitem"]')).filter((row) =>
        row.checkVisibility({ contentVisibilityAuto: true }),
      ).length,
  );

/** Whether row `index` of a content-visibility list is rendered (not skipped). */
const rowRendered = (list: Locator, index: number) =>
  list
    .locator(`[data-virtual-index="${index}"]`)
    .evaluate((row) => row.checkVisibility({ contentVisibilityAuto: true }));

test.describe("content-visibility mode", () => {
  // Safari 18-25 supports `content-visibility: auto` only partially (skipped content is not
  // found by find-in-page, WebKit bug 283846), and this suite has not been run on a WebKit
  // that has the fix, so these Chromium/Firefox assertions are not claimed for WebKit.
  test.beforeEach(({ browserName }) => {
    test.skip(browserName === "webkit", "content-visibility find-in-page / relevance rules unverified on Playwright's WebKit");
  });

  test("keeps every row in the DOM and renders only a bounded number of them", async ({ page }) => {
    await gotoHydrated(page, CV_URL);
    const list = cvList(page);
    await expect(list).toHaveAttribute("data-virtualize", "content-visibility");
    // Every row is present (server-rendered, hydrated), so find-in-page and the accessibility
    // tree can reach them.
    await expect(list.getByRole("listitem")).toHaveCount(1000);
    await expect(readout(page, "content-visibility", "rows")).toHaveText("1000");
    await expect(readout(page, "content-visibility", "mode")).toHaveText("content-visibility");
    // ...but the browser skips all but the rows in view: "bounded", not an exact figure.
    const rendered = await renderedRows(list);
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(80);
    // Rows are skipped in chunks of 20 (one skippable element per row costs the browser O(rows)
    // on every scroll frame; per chunk it is O(rows / 20)): 1,000 rows are 50 chunks, each
    // carrying the skip rule and the sum of its rows' height guesses.
    const chunks = list.locator(":scope > div");
    await expect(chunks).toHaveCount(50);
    for (const chunk of [0, 49]) {
      const style = (await chunks.nth(chunk).getAttribute("style")) ?? "";
      expect(style).toContain("content-visibility: auto");
      expect(style).toMatch(/contain-intrinsic-block-size: auto \d+px/);
      await expect(chunks.nth(chunk).getByRole("listitem")).toHaveCount(20);
    }
    // The rows carry no skip rule of their own.
    expect((await list.locator('[data-virtual-index="0"]').getAttribute("style")) ?? "").toBe("");
  });

  test("off-screen rows are in the accessibility tree and found by find-in-page", async ({ page }) => {
    await gotoHydrated(page, CV_URL);
    const list = cvList(page);
    const row = list.locator('[data-virtual-index="776"]');
    await expect(row).toBeAttached();
    await expect(row).toContainText("Row 777 of 1000");
    // Not rendered yet: it is far below the fold.
    expect(await rowRendered(list, 776)).toBe(false);
    const tree = await list.ariaSnapshot();
    expect(tree).toContain("Row 777 of 1000");
    expect(tree).toContain("Row 1 of 1000");
    // Playwright cannot drive native Ctrl+F; `window.find` is the same search. It finds text
    // in skipped content and un-skips the row it selects.
    expect(await page.evaluate(() => (window as unknown as { find: (s: string) => boolean }).find("Row 777 of 1000"))).toBe(true);
    await expect.poll(() => rowRendered(list, 776)).toBe(true);
  });

  test("scrolling costs Rust and the DOM nothing", async ({ page }) => {
    const panics: string[] = [];
    page.on("console", (msg) => /panicked/i.test(msg.text()) && panics.push(msg.text()));
    await trackListeners(page);
    await gotoHydrated(page, CV_URL);
    const list = cvList(page);
    await expect(list.getByRole("listitem")).toHaveCount(1000);
    // Let the post-hydration work settle, then count from a quiet baseline.
    await page.waitForTimeout(500);
    const rendersBefore = await numberIn(readout(page, "content-visibility", "renders"));
    const { mutations, scrollEvents, moved } = await scrollAndCount(list, 20_000, 400);
    // The scroll really happened...
    expect(scrollEvents).toBeGreaterThan(20);
    expect(moved).toBeGreaterThan(19_000);
    // ...and nothing in Rust or the DOM noticed: no list re-render, no mutation of the list's
    // subtree, no scroll listener on the container at all.
    expect(await numberIn(readout(page, "content-visibility", "renders"))).toBe(rendersBefore);
    expect(mutations).toBe(0);
    expect((await liveListeners(page)).listScroll).toBe(0);
    expect(await renderedRows(list)).toBeLessThan(80);
    expect(panics).toEqual([]);
  });

  test("the chunk holding focus or an end of the selection is never skipped", async ({ page }) => {
    await gotoHydrated(page, CV_URL);
    const list = cvList(page);
    const rendered = (index: number) => rowRendered(list, index);
    expect(await rendered(700)).toBe(false);
    // Focus without scrolling: the row is far off-screen, and only focus keeps it rendered
    // (the browser's own "relevant to the user" rule, no controller involved).
    await list.locator('[data-virtual-index="700"]').evaluate((el) => {
      el.tabIndex = -1;
      el.focus({ preventScroll: true });
    });
    await expect.poll(() => rendered(700)).toBe(true);
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());
    await expect.poll(() => rendered(700)).toBe(false);
    // The same for both ends of a selection, a couple of chunks apart.
    await list.evaluate((el) => {
      const text = (i: number) => el.querySelector(`[data-virtual-index="${i}"] p`)!.firstChild!;
      getSelection()!.setBaseAndExtent(text(790), 2, text(830), 4);
    });
    // Rows 790 and 830 are in different chunks (39 and 41); both are rendered.
    await expect.poll(() => rendered(790)).toBe(true);
    await expect.poll(() => rendered(830)).toBe(true);
  });
});

test.describe("content-visibility mode: per-frame cost (release only)", () => {
  test("a scroll costs the main thread about what a windowed list does, per frame", async ({ page, browserName }) => {
    // A timing threshold: meaningful only on a release build, and CDP metrics are Chromium's.
    test.skip(browserName !== "chromium", "CDP Performance.getMetrics");
    test.skip(
      buildProfile() !== "release",
      "per-frame main-thread time is only measured on a release build (PW_BUILD_PROFILE=release)",
    );
    await gotoHydrated(page, CV_URL);
    const list = cvList(page);
    await expect(list.getByRole("listitem")).toHaveCount(1000);
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Performance.enable");
    const taskSeconds = async () => {
      const { metrics } = await cdp.send("Performance.getMetrics");
      return metrics.find((m) => m.name === "TaskDuration")!.value;
    };
    await page.waitForTimeout(500);
    const before = await taskSeconds();
    const { scrollEvents } = await scrollAndCount(list, 10_000, 40);
    const msPerFrame = (((await taskSeconds()) - before) * 1000) / scrollEvents;
    console.log(`content-visibility 1000 rows: ${msPerFrame.toFixed(1)} ms of main-thread time per scroll frame`);
    // One skippable element per row costs 7.7 ms per frame at these 1,000 rows (31 ms at 5,000;
    // measured, release, headless Chromium, 4-core VM); one per chunk of 20 rows ~1.1 ms, and a
    // windowed list ~1.4 ms. 4 ms separates the two with room for a slower machine.
    expect(msPerFrame).toBeLessThan(4);
  });
});

test.describe("windowed mode", () => {
  test("mounts a window over a full-height canvas, and find-in-page cannot reach the rest", async ({ page }) => {
    await gotoHydrated(page, WINDOWED_URL);
    const list = windowedList(page);
    await expect(list).toHaveAttribute("data-virtualize", "windowed");
    await expect(list.getByRole("listitem").first()).toBeVisible();
    const mounted = await list.getByRole("listitem").count();
    expect(mounted).toBeGreaterThan(5);
    expect(mounted).toBeLessThan(80);
    await expect(readout(page, "windowed", "rows")).toHaveText(String(mounted));
    await expect(readout(page, "windowed", "mode")).toHaveText("windowed");
    // 100,000 rows at ~100px: the scrollbar is the full list.
    expect(await list.evaluate((el) => el.scrollHeight)).toBeGreaterThan(5_000_000);
    // The trade-off, stated as a test: text in an unmounted row does not exist for Ctrl+F.
    const find = (text: string) => page.evaluate((t) => (window as unknown as { find: (s: string) => boolean }).find(t), text);
    expect(await find("Row 1 of 100000")).toBe(true);
    expect(await find("Row 77777 of 100000")).toBe(false);
  });

  test("a scroll re-renders Rust once per few rows, not once per scroll event", async ({ page }) => {
    const panics: string[] = [];
    page.on("console", (msg) => /panicked/i.test(msg.text()) && panics.push(msg.text()));
    await gotoHydrated(page, WINDOWED_URL);
    const list = windowedList(page);
    await expect(list.getByRole("listitem").first()).toBeVisible();
    // Get past the first measurements, then count from a quiet baseline.
    await scrollAndCount(list, 2_000, 100);
    const rendersBefore = await numberIn(readout(page, "windowed", "renders"));
    const { mutations, scrollEvents, moved } = await scrollAndCount(list, 1_000, 25);
    const rendersDuring = (await numberIn(readout(page, "windowed", "renders"))) - rendersBefore;
    console.log(`1000px in 25px steps: ${scrollEvents} scroll events, ${rendersDuring} Rust renders, ${mutations} DOM mutations`);
    expect(moved).toBeGreaterThan(900);
    expect(scrollEvents).toBeGreaterThanOrEqual(20);
    // Rows are ~110px and `buffer` is 12, so the window is re-centred every ~6 rows (~660px):
    // a couple of renders for 1000px (plus the one that releases the frozen canvas height when
    // the scroll ends), against one per scroll event before.
    expect(rendersDuring).toBeGreaterThanOrEqual(1);
    expect(rendersDuring).toBeLessThanOrEqual(6);
    expect(rendersDuring).toBeLessThan(scrollEvents / 3);
    // Each render changes a handful of rows, not the whole window.
    expect(mutations).toBeLessThan(150);
    expect(panics).toEqual([]);
  });

  test("unmounting removes the scroll bridge's window resize listener", async ({ page }) => {
    await trackListeners(page);
    await gotoHydrated(page, WINDOWED_URL);
    const list = windowedList(page);
    await expect(list.getByRole("listitem").first()).toBeVisible();
    const mounted = await liveListeners(page);
    expect(mounted.listScroll).toBe(1);

    const toggle = page.getByRole("button", { name: /(Unmount|Mount) the list/ });
    // Each mount adds exactly one resize listener and each unmount removes it: a leak would
    // grow the count by one per cycle, which is how a page that mounts lists leaks.
    for (let cycle = 0; cycle < 3; cycle++) {
      await toggle.click();
      await expect(list).toHaveCount(0);
      await expect.poll(async () => (await liveListeners(page)).resize).toBe(mounted.resize - 1);
      await toggle.click();
      await expect(list.getByRole("listitem").first()).toBeVisible();
      await expect.poll(async () => (await liveListeners(page)).resize).toBe(mounted.resize);
    }
  });

  test("the default mode follows the row count (1,000 is the edge) and drops the windowed bridge when it switches", async ({ page }) => {
    await trackListeners(page);
    await gotoHydrated(page, WINDOWED_URL);
    const list = windowedList(page);
    await expect(list).toHaveAttribute("data-virtualize", "windowed");
    await expect(list.getByRole("listitem").first()).toBeVisible();
    await expect(readout(page, "windowed", "mode")).toHaveText("windowed");
    const windowedListeners = await liveListeners(page);
    const switchTo = (count: number) => page.getByRole("button", { name: `${count} rows`, exact: true }).click();

    // 5,000 rows and one row past the threshold are both windowed: the cost of keeping every row
    // grows with the count (mounting 5,000 is a ~150 ms task), a window mounts ~40 whatever it is.
    for (const count of [5000, 1001]) {
      await switchTo(count);
      await expect(list).toHaveAttribute("data-virtualize", "windowed");
      expect(await list.getByRole("listitem").count()).toBeLessThan(80);
    }

    // ...and 1,000 rows keep every row in the DOM instead.
    await switchTo(1000);
    await expect(list).toHaveAttribute("data-virtualize", "content-visibility");
    await expect(list.getByRole("listitem")).toHaveCount(1000);
    await expect(readout(page, "windowed", "mode")).toHaveText("content-visibility");
    await expect(readout(page, "windowed", "rows")).toHaveText("1000");
    await expect(list.locator('[data-virtual-index="999"]')).toHaveAttribute("aria-setsize", "1000");
    // The windowed body is gone, and took its listeners with it.
    await expect.poll(() => liveListeners(page)).toEqual({ resize: windowedListeners.resize - 1, listScroll: 0 });

    // And back to a long list.
    await switchTo(100000);
    await expect(list).toHaveAttribute("data-virtualize", "windowed");
    expect(await list.getByRole("listitem").count()).toBeLessThan(80);
    await expect.poll(() => liveListeners(page)).toEqual(windowedListeners);
  });
});

test.describe("both modes", () => {
  for (const [name, url, list, size] of [
    ["content-visibility", CV_URL, cvList, 1000],
    ["windowed", WINDOWED_URL, windowedList, 100000],
  ] as const) {
    test(`${name}: same roles and positional ARIA on the container and every row`, async ({ page }) => {
      await gotoHydrated(page, url);
      const container = list(page);
      await expect(container).toHaveAttribute("role", "list");
      await expect(container).toHaveAttribute("tabindex", "0");
      const first = container.getByRole("listitem").first();
      await expect(first).toHaveAttribute("aria-setsize", String(size));
      await expect(first).toHaveAttribute("aria-posinset", "1");
      await expect(first).toHaveAttribute("data-virtual-index", "0");
      // Every mounted row, whichever mode mounted it, announces its position of the full set.
      const rows = await container.getByRole("listitem").evaluateAll((els) =>
        els.map((el) => [el.getAttribute("data-virtual-index"), el.getAttribute("aria-posinset"), el.getAttribute("aria-setsize")]),
      );
      for (const [index, posinset, setsize] of rows) {
        expect(Number(posinset)).toBe(Number(index) + 1);
        expect(setsize).toBe(String(size));
      }
    });
  }
});
