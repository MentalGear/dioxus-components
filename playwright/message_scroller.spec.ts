/**
 * Rule sources (docs/conformance-harness.md tiers):
 *
 * - Tier 3 "opinion". No APG pattern covers a chat transcript; the references
 *   are the ARIA `log` role, WCAG 2.1.1 (the scroll region is keyboard
 *   operable) and shadcn/ui's own `@shadcn/react` `message-scroller` suites
 *   (`packages/react/src/message-scroller/message-scroller.browser.test.tsx`,
 *   17 real-Chromium cases). The ones that matter most are ported here against
 *   the real component: opening position, follow-the-stream, release on user
 *   intent, jump-to-latest, turn anchoring, prepend preservation.
 * - The `content-visibility` virtualization cases (a 2,000-row transcript) are
 *   this library's own acceptance for `Virtualization::ContentVisibility`: the
 *   work stays bounded, and find-in-page, the accessibility tree and the live
 *   edge still behave as if every row were rendered.
 *
 * No assertion here is a millisecond threshold, except the one marked release-only
 * ("scrolling a 2,000-row transcript costs the main thread little per frame"), which is
 * skipped unless the build under test is a known release build (CLAUDE.md: timing specs
 * only on release; `PW_BUILD_PROFILE=release` or `SSG_SITE_DIR` names it). The bounds
 * everywhere else are pixel distances and row counts. Pixel bounds absorb one frame of
 * lag between a row growing and the controller's rAF-coalesced follow.
 */
import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL, buildProfile } from "./base-url";
import { gotoHydrated } from "./hydration";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };
// The demos' `MessageScrollerContent` start padding (`--dx-space-4`).
const CONTENT_PADDING = 16;
const URL = `${BASE_URL}/component/?name=message_scroller&`;

type Variant = "main" | "last_anchor" | "load_history" | "long";

const frameOf = (page: Page, variant: Variant) =>
  page
    .locator(variant === "main" ? "#component-preview-frame" : `#component-preview-frame-${variant}`)
    .first();
const viewportOf = (page: Page, variant: Variant = "main") =>
  frameOf(page, variant).locator("[data-message-scroller-viewport]");
const jumpButton = (page: Page, variant: Variant = "main", direction = "end") =>
  frameOf(page, variant).locator(`[data-message-scroller-button][data-direction="${direction}"]`);

/** Open the page and wait for the transcript's opening position to be applied. */
async function open(page: Page, variant: Variant = "main") {
  await gotoHydrated(page, URL, GOTO_OPTS);
  const viewport = viewportOf(page, variant);
  await viewport.scrollIntoViewIfNeeded();
  await expect(viewport).not.toHaveAttribute("data-pending-scroll");
  return viewport;
}

const dist = (vp: Locator) =>
  vp.evaluate((el) => Math.round(el.scrollHeight - el.scrollTop - el.clientHeight));
const scrollTop = (vp: Locator) => vp.evaluate((el) => Math.round(el.scrollTop));
const offsetOf = (vp: Locator, id: string) =>
  vp.evaluate((el, id) => {
    const row = el.querySelector(`[data-message-id="${id}"]`) as HTMLElement;
    return Math.round(row.getBoundingClientRect().top - el.getBoundingClientRect().top);
  }, id);

/** Resolves once scrollTop has been the same for a few frames. */
const stable = (vp: Locator) =>
  vp.evaluate(
    (el) =>
      new Promise<number>((resolve) => {
        let last = el.scrollTop;
        let frames = 0;
        const tick = () => {
          if (Math.abs(el.scrollTop - last) < 0.5) frames++;
          else frames = 0;
          last = el.scrollTop;
          if (frames >= 8) resolve(Math.round(el.scrollTop));
          else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      }),
  );

/** Wheel over the viewport (a real wheel event, not a scrollTop write). */
async function wheel(page: Page, vp: Locator, deltaY: number) {
  const box = (await vp.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, deltaY);
  return stable(vp);
}

/** Record the distance to the end every frame until `stop()` is called. */
async function sampleDistance(vp: Locator) {
  await vp.evaluate((el) => {
    const w = window as unknown as { __msDist: number[]; __msStop: boolean };
    w.__msDist = [];
    w.__msStop = false;
    const tick = () => {
      w.__msDist.push(Math.round(el.scrollHeight - el.scrollTop - el.clientHeight));
      if (!w.__msStop) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  return async () =>
    vp.evaluate(() => {
      const w = window as unknown as { __msDist: number[]; __msStop: boolean };
      w.__msStop = true;
      return w.__msDist;
    });
}

const streamButton = (page: Page) => frameOf(page, "main").getByRole("button", { name: "Stream a reply" });

test.describe("opening position", () => {
  test("opens at the end, with the viewport hidden until it is applied (no flash)", async ({ page }) => {
    // Sample every frame from the first one: the viewport must never be visible
    // AND scrollable AND still at the top. (Before its stylesheet applies it is
    // not a scroll container, so that state cannot occur there either.)
    await page.addInitScript(() => {
      const w = window as unknown as { __msFlash: { samples: number; flashes: number } };
      w.__msFlash = { samples: 0, flashes: 0 };
      const tick = () => {
        const vp = document.querySelector("[data-message-scroller-viewport]") as HTMLElement | null;
        if (vp) {
          w.__msFlash.samples++;
          const scrollable = vp.scrollHeight > vp.clientHeight + 1;
          const visible = getComputedStyle(vp).visibility !== "hidden";
          if (visible && scrollable && vp.scrollTop < 1) w.__msFlash.flashes++;
        }
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    });
    const vp = await open(page);
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    const flash = await page.evaluate(
      () => (window as unknown as { __msFlash: { samples: number; flashes: number } }).__msFlash,
    );
    expect(flash.samples).toBeGreaterThan(0);
    expect(flash.flashes).toBe(0);
    // The guard attribute is gone from both the root and the viewport.
    await expect(frameOf(page, "main").locator("[data-pending-scroll]")).toHaveCount(0);
  });

  test("last-anchor opens on the last user message, not the end", async ({ page }) => {
    const vp = await open(page, "last_anchor");
    // The last user turn is `m6`; it sits at the reading line (the previous-row
    // peek plus the content padding), and its overflowing reply is below it.
    await expect.poll(() => offsetOf(vp, "m6")).toBeGreaterThanOrEqual(50);
    expect(await offsetOf(vp, "m6")).toBeLessThanOrEqual(110);
    expect(await dist(vp)).toBeGreaterThan(20);
    await expect(jumpButton(page, "last_anchor")).toHaveAttribute("data-active", "true");
  });
});

test.describe("follow the stream", () => {
  test("follows at the live edge while a reply streams", async ({ page }) => {
    const vp = await open(page);
    const stop = await sampleDistance(vp);
    await streamButton(page).click();
    // The button is disabled while the simulated stream runs, enabled when done.
    await expect(streamButton(page)).toBeDisabled();
    await expect(streamButton(page)).toBeEnabled({ timeout: 60_000 });
    const samples = await stop();
    expect(samples.length).toBeGreaterThan(30);
    // Without following, the ~600px reply would open a gap of hundreds of px.
    expect(Math.max(...samples)).toBeLessThanOrEqual(100);
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    // Following publishes `end: false`, so the jump button stays out of the way.
    await expect(jumpButton(page)).toHaveAttribute("data-active", "false");
    await expect(jumpButton(page)).toHaveJSProperty("inert", true);
  });

  test("stops following once the reader scrolls up (wheel)", async ({ page }) => {
    const vp = await open(page);
    await streamButton(page).click();
    await expect(frameOf(page, "main").locator('[data-message-id^="m"]').last()).toContainText("Streaming makes");
    const releasedAt = await wheel(page, vp, -500);
    expect(await dist(vp)).toBeGreaterThan(100);
    await expect(streamButton(page)).toBeEnabled({ timeout: 60_000 });
    // The reply kept growing below the reader without moving them.
    expect(Math.abs((await scrollTop(vp)) - releasedAt)).toBeLessThanOrEqual(1);
    expect(await dist(vp)).toBeGreaterThan(100);
    await expect(jumpButton(page)).toHaveAttribute("data-active", "true");
    await expect(jumpButton(page)).toHaveJSProperty("inert", false);
  });

  test("stops following when the scroll position is dragged up (scrollbar-style move, no wheel/key)", async ({ page }) => {
    const vp = await open(page);
    await streamButton(page).click();
    await expect(frameOf(page, "main").locator('[data-message-id^="m"]').last()).toContainText("Streaming makes");
    // Move the thumb up a little every frame, as a scrollbar drag does.
    await vp.evaluate(async (el) => {
      for (let i = 0; i < 40; i++) {
        el.scrollTop = Math.max(0, el.scrollTop - 25);
        await new Promise((r) => requestAnimationFrame(() => r(null)));
      }
    });
    await expect(streamButton(page)).toBeEnabled({ timeout: 60_000 });
    expect(await dist(vp)).toBeGreaterThan(100);
  });

  test("a row appended while the reader is released does not move them", async ({ page }) => {
    const vp = await open(page);
    const at = await wheel(page, vp, -500);
    await frameOf(page, "main").getByRole("button", { name: "Add message" }).click();
    await expect(frameOf(page, "main").getByText(/^Message \d+$/).last()).toBeVisible();
    expect(Math.abs((await stable(vp)) - at)).toBeLessThanOrEqual(1);
  });

  test("a wheel down at the live edge does not release following", async ({ page }) => {
    const vp = await open(page);
    await wheel(page, vp, 400); // already at the end: nothing can move
    await frameOf(page, "main").getByRole("button", { name: "Add message" }).click();
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
  });

  test("with auto-scroll off a streamed reply never moves the reader", async ({ page }) => {
    const vp = await open(page);
    await frameOf(page, "main").getByRole("switch", { name: "Auto-scroll" }).click();
    const at = await scrollTop(vp);
    await streamButton(page).click();
    await expect(streamButton(page)).toBeEnabled({ timeout: 60_000 });
    expect(Math.abs((await scrollTop(vp)) - at)).toBeLessThanOrEqual(1);
    expect(await dist(vp)).toBeGreaterThan(100);
  });
});

test.describe("jump to latest", () => {
  test("is inert at rest and not reachable by Tab", async ({ page }) => {
    const vp = await open(page);
    const button = jumpButton(page);
    await expect(button).toHaveAttribute("data-active", "false");
    await expect(button).toHaveJSProperty("inert", true);
    await expect(button).toHaveAttribute("tabindex", "-1");
    await vp.focus();
    await page.keyboard.press("Tab");
    await expect(button).not.toBeFocused();
  });

  test("returns the reader to the end and resumes following", async ({ page }) => {
    const vp = await open(page);
    await wheel(page, vp, -600);
    const button = jumpButton(page);
    await expect(button).toHaveAttribute("data-active", "true");
    await button.click();
    await expect.poll(() => dist(vp), { timeout: 10_000 }).toBeLessThanOrEqual(1);
    await expect(button).toHaveAttribute("data-active", "false");
    // Following is re-engaged: an appended row stays in view.
    await frameOf(page, "main").getByRole("button", { name: "Add message" }).click();
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
  });

  test("is operable from the keyboard (PageUp, Tab, Enter) and hands focus back to the viewport", async ({ page }) => {
    const vp = await open(page);
    const button = jumpButton(page);
    await vp.focus();
    await page.keyboard.press("PageUp");
    await expect.poll(() => dist(vp)).toBeGreaterThan(30);
    await expect(button).toHaveAttribute("data-active", "true");
    await page.keyboard.press("Tab");
    await expect(button).toBeFocused();
    await page.keyboard.press("Enter");
    await expect.poll(() => dist(vp), { timeout: 10_000 }).toBeLessThanOrEqual(1);
    await expect(button).toHaveJSProperty("inert", true);
    // The button went inert while focused: focus must not fall to <body>.
    await expect(vp).toBeFocused();
  });

  test("scroll keys release following", async ({ page }) => {
    const vp = await open(page);
    await vp.focus();
    await page.keyboard.press("Home");
    await expect.poll(() => scrollTop(vp)).toBeLessThanOrEqual(1);
    await frameOf(page, "main").getByRole("button", { name: "Add message" }).click();
    await expect(frameOf(page, "main").getByText(/^Message \d+$/).last()).toBeAttached();
    expect(await scrollTop(vp)).toBeLessThanOrEqual(1);
  });

  test("the scrollability signal reaches Rust only as `none` / `start` / `start end`", async ({ page }) => {
    const vp = await open(page);
    const status = frameOf(page, "main").getByTestId("scroll-status");
    // At the end and not scrolled up from a position that can scroll up: start only.
    await expect(status).toHaveText("start");
    await wheel(page, vp, -3000);
    await expect(status).toHaveText("end");
    await wheel(page, vp, 300);
    await expect(status).toHaveText("start end");
    await expect(vp).toHaveAttribute("data-scrollable", "start end");
  });
});

test.describe("anchoring a new turn", () => {
  test("a sent message lands near the top, then following takes over the long reply", async ({ page }) => {
    const vp = await open(page);
    await frameOf(page, "main").getByRole("button", { name: "Send a message" }).click();
    // The new user turn (id 12) is a scroll anchor: it moves near the top, with
    // the previous-row peek (64px) plus the content padding (16px) above it.
    await expect.poll(() => offsetOf(vp, "m12"), { timeout: 10_000 }).toBeGreaterThanOrEqual(50);
    expect(await offsetOf(vp, "m12")).toBeLessThanOrEqual(110);
    await expect(streamButton(page)).toBeEnabled({ timeout: 60_000 });
    // The reply is taller than the viewport, so following took over from the
    // anchor and the turn scrolled out of the top.
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    expect(await offsetOf(vp, "m12")).toBeLessThan(0);
  });
});

test.describe("loading earlier messages", () => {
  test("prepending rows does not move the row being read", async ({ page }) => {
    const vp = await open(page, "load_history");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    await vp.evaluate((el) => {
      el.scrollTop = Math.round(el.scrollHeight / 2);
    });
    await stable(vp);
    const rows = vp.locator("[data-message-id]");
    const before = await rows.count();
    const id = await vp.evaluate((el) => {
      const top = el.getBoundingClientRect().top;
      for (const row of Array.from(el.querySelectorAll("[data-message-id]"))) {
        if (row.getBoundingClientRect().bottom > top + 8) return (row as HTMLElement).dataset.messageId!;
      }
      return "";
    });
    const offsetBefore = await offsetOf(vp, id);
    await frameOf(page, "load_history").getByRole("button", { name: "Load earlier messages" }).click();
    await expect(rows).toHaveCount(before + 6);
    await stable(vp);
    expect(Math.abs((await offsetOf(vp, id)) - offsetBefore)).toBeLessThanOrEqual(1);
  });

  test("the start button jumps to the oldest message", async ({ page }) => {
    const vp = await open(page, "load_history");
    const start = jumpButton(page, "load_history", "start");
    await expect(start).toHaveAttribute("data-active", "true");
    await start.click();
    await expect.poll(() => scrollTop(vp), { timeout: 10_000 }).toBeLessThanOrEqual(1);
    await expect(start).toHaveAttribute("data-active", "false");
  });
});

test.describe("virtualization: content-visibility", () => {
  const ROWS = 2000;

  /** Rows that are actually rendered, i.e. not inside a chunk (or row) that `content-visibility` skips. */
  const renderedRows = (vp: Locator) =>
    vp.evaluate(
      (el) =>
        Array.from(el.querySelectorAll<HTMLElement>("[data-message-id]")).filter((row) =>
          row.checkVisibility({ contentVisibilityAuto: true }),
        ).length,
    );

  test("keeps every row in the DOM but only renders a bounded number of them", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    expect(await vp.locator("[data-message-id]").count()).toBe(ROWS);
    // Rows are skipped in chunks of 20 (one skippable element per row costs the browser
    // main-thread time on every scroll frame in proportion to the row count): 2,000 rows are 100
    // chunks.
    await expect(vp.locator("[data-message-scroller-chunk]")).toHaveCount(ROWS / 20);
    // Only the rows in view plus the live-edge chunk are rendered. A generous cap: the point is
    // "bounded", not an exact figure.
    expect(await renderedRows(vp)).toBeLessThan(80);
    // The live edge (the last 8 rows) is held out of skipping by the controller: the one
    // chunk that holds them.
    await expect(vp.locator("[data-keep-rendered]")).toHaveCount(1);
    await expect(
      vp.locator(`[data-message-scroller-chunk][data-keep-rendered] [data-message-id="m${ROWS - 1}"]`),
    ).toHaveCount(1);
    await expect(vp.locator("[data-message-scroller-content]")).toHaveAttribute(
      "data-virtualize",
      "content-visibility",
    );
  });

  test("off-screen rows stay in the accessibility tree and findable (find-in-page)", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    // Playwright cannot drive native Ctrl+F, so assert what it relies on: the
    // row is in the DOM, its text is in the accessibility tree, and the
    // browser's own `window.find` locates it.
    const row = vp.locator('[data-message-id="m777"]');
    await expect(row).toBeAttached();
    const tree = await vp.locator("[data-message-scroller-content]").ariaSnapshot();
    expect(tree).toContain("Message 777.");
    expect(tree).toContain("Message 1.");
    // `window.find` selects the match and reveals the skipped row.
    const found = await page.evaluate(() => (window as unknown as { find: (s: string) => boolean }).find("Message 777."));
    expect(found).toBe(true);
  });

  test("follow lands exactly at the end while rows are appended", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    const add = frameOf(page, "long").getByRole("button", { name: "Add message" });
    for (let i = 0; i < 4; i++) {
      await add.click();
      await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    }
    await expect(frameOf(page, "long").getByText(`${ROWS + 4} messages in the DOM`)).toBeVisible();
  });

  test("scroll-up release and jump-to-latest still work on a long transcript", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    const at = await wheel(page, vp, -800);
    await frameOf(page, "long").getByRole("button", { name: "Add message" }).click();
    await expect(frameOf(page, "long").getByText(/messages in the DOM/)).toContainText(String(ROWS + 1));
    expect(Math.abs((await stable(vp)) - at)).toBeLessThanOrEqual(1);
    await expect(jumpButton(page, "long")).toHaveAttribute("data-active", "true");
    await jumpButton(page, "long").click();
    await expect.poll(() => dist(vp), { timeout: 10_000 }).toBeLessThanOrEqual(1);
  });

  test("jumping to a far-away message lands it at the top edge", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    await frameOf(page, "long").getByRole("button", { name: "Jump to message 1000" }).click();
    // The row lands on the content's start padding (16px). Its neighbours render
    // at their real size as they arrive, which moves it; the controller re-aims.
    await expect.poll(async () => Math.abs((await offsetOf(vp, "m1000")) - CONTENT_PADDING), { timeout: 10_000 }).toBeLessThanOrEqual(3);
    expect(Math.abs((await stable(vp)) - (await vp.evaluate((el) => Math.round(el.scrollTop))))).toBeLessThanOrEqual(1);
  });

  test("the row holding focus is never skipped", async ({ page }) => {
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    await frameOf(page, "long").getByRole("button", { name: "Jump to message 1000" }).click();
    await expect.poll(async () => Math.abs((await offsetOf(vp, "m1000")) - CONTENT_PADDING), { timeout: 10_000 }).toBeLessThanOrEqual(3);
    // Rows hold no focusable child here, so focus the row itself.
    await vp.evaluate((el) => {
      const row = el.querySelector('[data-message-id="m1000"]') as HTMLElement;
      row.tabIndex = -1;
      row.focus();
    });
    // The unit that is kept is the chunk holding the row.
    await expect(
      vp.locator('[data-message-scroller-chunk]:has([data-message-id="m1000"])'),
    ).toHaveAttribute("data-keep-rendered", "");
  });

  test("scrolling a 2,000-row transcript costs the main thread little per frame (release only)", async ({ page }) => {
    // A timing threshold: meaningful only on a release build (debug builds measure 5-10x slower).
    test.skip(
      buildProfile() !== "release",
      "per-frame main-thread time is only measured on a release build (PW_BUILD_PROFILE=release)",
    );
    const vp = await open(page, "long");
    await expect.poll(() => dist(vp)).toBeLessThanOrEqual(1);
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Performance.enable");
    const taskSeconds = async () => {
      const { metrics } = await cdp.send("Performance.getMetrics");
      return metrics.find((m) => m.name === "TaskDuration")!.value;
    };
    await page.waitForTimeout(500);
    const before = await taskSeconds();
    // From the live edge up through 10,000px of transcript, one 40px step per frame.
    const frames = await vp.evaluate(async (el) => {
      const start = el.scrollTop;
      let frames = 0;
      for (let y = 40; y <= 10_000; y += 40) {
        el.scrollTop = start - y;
        frames++;
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
      return frames;
    });
    const msPerFrame = ((await taskSeconds()) - before) * 1000 / frames;
    console.log(`message_scroller long: ${msPerFrame.toFixed(1)} ms of main-thread time per scroll frame`);
    // One skippable element per row costs ~20 ms per frame at 2,000 rows (measured, release,
    // headless Chromium on a 4-core VM); one per chunk of 20 rows ~1-2 ms. 8 ms is far from both.
    expect(msPerFrame).toBeLessThan(8);
  });
});

test.describe("accessibility", () => {
  test("ships the log / region defaults", async ({ page }) => {
    const vp = await open(page);
    await expect(vp).toHaveAttribute("role", "region");
    await expect(vp).toHaveAttribute("tabindex", "0");
    const content = frameOf(page, "main").locator("[data-message-scroller-content]");
    await expect(content).toHaveAttribute("role", "log");
    await expect(content).toHaveAttribute("aria-relevant", "additions");
    const spacer = content.locator("[data-message-scroller-spacer]");
    await expect(spacer).toHaveAttribute("aria-hidden", "true");
    await expect(spacer).toBeHidden();
  });

  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await open(page);
    await expectNoAxeViolations(page, "message_scroller: loaded", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });

  test("with the jump button visible has no automatically detectable a11y issues", async ({ page }) => {
    const vp = await open(page);
    await wheel(page, vp, -600);
    await expect(jumpButton(page)).toHaveAttribute("data-active", "true");
    await expectNoAxeViolations(page, "message_scroller: jump button visible", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
