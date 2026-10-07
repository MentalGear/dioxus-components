/**
 * ORACLE: tier 2 (HTML) -- JS listeners are owned by the scope that added them.
 *
 * Source: dev-docs/backlog.md rows 149 and 159 (`scroll_lock.rs` left non-passive `wheel`/`touchmove`
 * listeners on `window` for the rest of the visit after the first modal had opened; the same shape --
 * an `addEventListener` inside a `document::eval` script that nothing removes -- was in the form
 * fixture, the sidebar, `pointer.rs` and the focus trap). Rule sources, per this tier's policy of citing
 * a standard rather than an opinion:
 *   - DOM Standard, `AddEventListenerOptions.passive` and `removeEventListener`: a registration lives until
 *     it is removed, whatever script made it:
 *       https://dom.spec.whatwg.org/#dom-eventtarget-addeventlistener
 *   - the same standard's "default passive" rule for window/document/body wheel + touch listeners, and
 *     UI Events (`wheel` is cancelable): a NON-passive wheel/touch listener makes the user agent wait for
 *     the handler before it may scroll:
 *       https://dom.spec.whatwg.org/#default-passive-value
 *       https://w3c.github.io/uievents/#event-type-wheel
 *
 * What it asserts. Chromium itself is asked what `window`, `document`, `<html>` and `<body>` hold
 * (CDP `DOMDebugger.getEventListeners`; Playwright cannot ask the page, and a JS shim would only see the
 * calls it wraps):
 *   1. INVENTORY: for every modal, popover and menu consumer, and for entering and leaving the routes that
 *      mount the form fixture and the sidebar, the multiset of (type, capture, passive) on those four
 *      targets after the surface has been opened and closed again (twice, so a second cycle cannot stack on
 *      the first) equals the one taken before the first open. Nothing may be left behind, and nothing
 *      lazily installed "for the rest of the session" either, which is why the baseline is taken BEFORE the
 *      first open and there is no warm-up cycle.
 *   2. NO BLOCKING LISTENER RESIDUE: after any of the cycles above no non-passive `wheel` / `mousewheel` /
 *      `touchstart` / `touchmove` listener is on those targets (row 149's symptom stated on its own, so a
 *      baseline that already carried one cannot hide it).
 *   3. SCROLL LOCK TIMING (the Dialog, with its exit slowed to 1.2 s so the window is deterministic):
 *      while the dialog is still animating out (`dialog[open]` still there) the page does not scroll and the
 *      wheel is cancelled; the lock lets go at the dialog's real `close()` -- not before it, and before the
 *      dialog unmounts (`use_animated_open`'s ~250 ms hold) -- after which a wheel scrolls the page again.
 *   4. NO WIDTH SHIFT: under a forced classic scrollbar (`--hide-scrollbars` dropped: the default headless
 *      Chromium has 0-width overlay scrollbars, which hide this class of bug), `documentElement.clientWidth`
 *      is the same on every frame from before the open, through the open, the exit and the close.
 *
 * Chromium only (CDP), like `main-thread.spec.ts`. Run against the dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=baseline.local.config.ts \
 *     ./oracle/tier2-html/listener-inventory.spec.ts
 *
 * Red proof (2026-10-05), against the tree before the helper (`scroll_lock.rs` installing its four listeners
 * at the first lock and keeping them; `form` adding two `document` listeners per visit; `pointer.rs`
 * installing four on `window` on first use): see dev-docs/backlog.md row 159.
 */
import { test, expect } from "../../fixtures";
import type { CDPSession, Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";
import { gotoHydrated } from "../../hydration";

type Listener = { type: string; capture: boolean; passive: boolean };

const TARGETS = ["window", "document", "document.documentElement", "document.body"] as const;
const BLOCKING_TYPES = new Set(["wheel", "mousewheel", "touchstart", "touchmove", "DOMMouseScroll"]);
const GOTO = { timeout: 20 * 60 * 1000 };

/** Source-text markers of Playwright's own injected script, whose capture listeners (hit-target checks) pollute `window`. */
const PLAYWRIGHT_MARKERS = ["_setupHitTargetInterceptors", "__playwright_global_listeners_check__", "__pwUtility"];
const playwrightScripts = new WeakMap<CDPSession, Map<string, boolean>>();

async function isPlaywrightScript(cdp: CDPSession, scriptId: string): Promise<boolean> {
  let cache = playwrightScripts.get(cdp);
  if (!cache) playwrightScripts.set(cdp, (cache = new Map()));
  const known = cache.get(scriptId);
  if (known !== undefined) return known;
  let verdict = false;
  try {
    const { scriptSource } = await cdp.send("Debugger.getScriptSource", { scriptId });
    verdict = PLAYWRIGHT_MARKERS.some((m) => scriptSource.includes(m));
  } catch {
    // The script is gone: not one of ours to judge, count it.
  }
  cache.set(scriptId, verdict);
  return verdict;
}

/** The page's listeners on `expression`, minus Playwright's own. */
async function listenersOf(cdp: CDPSession, expression: string): Promise<Listener[]> {
  const { result } = await cdp.send("Runtime.evaluate", { expression });
  if (!result.objectId) return [];
  const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId });
  await cdp.send("Runtime.releaseObject", { objectId: result.objectId });
  const mine: Listener[] = [];
  for (const l of listeners) {
    if (await isPlaywrightScript(cdp, l.scriptId)) continue;
    mine.push({ type: l.type, capture: l.useCapture, passive: l.passive });
  }
  return mine;
}

const keyOf = (target: string, l: Listener) => `${target}  ${l.type}${l.capture ? "  capture" : ""}${l.passive ? "  passive" : "  NON-PASSIVE"}`;

/** `{ "window  wheel  capture  NON-PASSIVE": 1, ... }` for the four global targets. */
async function inventory(cdp: CDPSession): Promise<Record<string, number>> {
  const out: Record<string, number> = {};
  for (const target of TARGETS) {
    for (const l of await listenersOf(cdp, target)) {
      const k = keyOf(target, l);
      out[k] = (out[k] ?? 0) + 1;
    }
  }
  return out;
}

/** What `listen()` (primitives/src/js_listener.rs) believes is live: a second opinion, for diagnosis. */
const helperLive = (page: Page) =>
  page.evaluate(() => {
    const reg = (window as unknown as { __dxJsListeners?: { live: Map<number, string> } }).__dxJsListeners;
    return reg ? [...reg.live.values()].sort() : [];
  });

function diff(before: Record<string, number>, after: Record<string, number>): string[] {
  const out: string[] = [];
  for (const k of new Set([...Object.keys(before), ...Object.keys(after)])) {
    const d = (after[k] ?? 0) - (before[k] ?? 0);
    if (d !== 0) out.push(`${d > 0 ? "+" : ""}${d}  ${k}`);
  }
  return out.sort();
}

const blockingIn = (inv: Record<string, number>) =>
  Object.keys(inv).filter((k) => {
    const type = k.split("  ")[1];
    return BLOCKING_TYPES.has(type) && k.endsWith("NON-PASSIVE");
  });

/** Time for the longest exit (`DIALOG_EXIT_TIMEOUT_MS` is 1500) and the unmount hold, plus slack. */
const SETTLE_MS = 2200;

async function openCdp(page: Page): Promise<CDPSession> {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Debugger.enable");
  return cdp;
}

type Scenario = {
  name: string;
  /** Path under BASE_URL, with the trailing `&`/`?` the other specs use. */
  url: string;
  open: (page: Page) => Promise<void>;
  close: (page: Page) => Promise<void>;
};

const escape = (page: Page) => page.keyboard.press("Escape");
const click = (name: string | RegExp, role: "button" | "menuitem" | "combobox" = "button") => async (page: Page) => {
  await page.getByRole(role, { name, exact: typeof name === "string" }).first().click();
};

/**
 * One scenario per overlay family. The modal consumers mirror `modal-consumers.ts`'s `CONSUMERS`
 * (dialog, alert_dialog, sheet, drawer, command); the rest are the popover and menu families.
 */
const SCENARIOS: Scenario[] = [
  { name: "dialog", url: "/component/?name=dialog&", open: click("Show Dialog"), close: escape },
  { name: "alert dialog", url: "/component/?name=alert_dialog&", open: click("Show Alert Dialog"), close: escape },
  { name: "sheet", url: "/component/?name=sheet&", open: click("Right"), close: escape },
  { name: "drawer", url: "/component/?name=drawer&", open: click("Move Goal"), close: escape },
  { name: "command palette", url: "/component/?name=command&", open: click("Open Command Palette"), close: escape },
  { name: "popover (modal arm)", url: "/component/?name=popover&", open: click("Show Popover"), close: escape },
  {
    // `pointer.rs`: the window pointer bridge exists only while a pointer is held down on a slider.
    name: "slider drag (the pointer bridge on window)",
    url: "/component/?name=slider&",
    open: async (page) => {
      const thumb = page.getByRole("slider", { name: "Demo Slider" });
      const box = (await thumb.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2, { steps: 5 });
    },
    close: async (page) => {
      await page.mouse.up();
    },
  },
  { name: "dropdown menu", url: "/component/?name=dropdown_menu&", open: click("Open Menu"), close: escape },
  {
    name: "context menu",
    url: "/component/?name=context_menu&",
    open: async (page) => {
      await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
      await expect(page.getByRole("menu")).toBeVisible();
    },
    close: escape,
  },
  { name: "menubar", url: "/component/?name=menubar&", open: click("File", "menuitem"), close: escape },
  {
    name: "navigation menu",
    url: "/component/?name=navigation_menu&",
    open: async (page) => {
      await page.getByRole("navigation", { name: "Component navigation menu" }).getByRole("button", { name: "Getting started" }).click();
    },
    close: escape,
  },
  {
    name: "select",
    url: "/component/?name=select&",
    open: async (page) => {
      await page.getByRole("combobox").first().click();
    },
    close: escape,
  },
  {
    name: "combobox",
    url: "/component/?name=combobox&",
    open: async (page) => {
      await page.getByRole("combobox", { name: "Select framework" }).click();
    },
    close: escape,
  },
  { name: "date picker", url: "/component/?name=date_picker&", open: click("Show Calendar"), close: escape },
  {
    name: "tooltip",
    url: "/component/?name=tooltip&",
    open: async (page) => {
      await page.getByText("Rich content").first().hover();
      await expect(page.getByRole("tooltip")).toBeVisible();
    },
    close: async (page) => {
      await page.mouse.move(0, 0);
    },
  },
  {
    name: "hover card",
    url: "/component/?name=hover_card&",
    open: async (page) => {
      await page.getByRole("button", { name: "Dioxus" }).hover();
    },
    close: async (page) => {
      await page.mouse.move(0, 0);
    },
  },
];

test.describe("listener inventory: every overlay returns window/document to its baseline", () => {
  for (const scenario of SCENARIOS) {
    test(`${scenario.name}: open and close twice leaves no listener behind`, async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}${scenario.url}`, GOTO);
      // The page's own steady state: let anything the load schedules settle, then measure.
      await page.waitForTimeout(800);
      const cdp = await openCdp(page);
      const before = await inventory(cdp);
      const helperBefore = await helperLive(page);

      for (let cycle = 0; cycle < 2; cycle++) {
        await scenario.open(page);
        await page.waitForTimeout(500);
        await scenario.close(page);
        await page.waitForTimeout(SETTLE_MS);
      }

      const after = await inventory(cdp);
      expect(diff(before, after), `listeners left on window/document/<html>/<body> by "${scenario.name}"`).toEqual([]);
      expect(blockingIn(after), "non-passive wheel/touch listener on a global target after the surface closed").toEqual([]);
      expect(await helperLive(page), "registrations `listen()` still believes live").toEqual(helperBefore);
    });
  }

  // The home page mounts the form fixture (its demo card) and the sidebar; `/charts/` has neither.
  for (const route of [
    { name: "the form fixture (document click/keyup, form invalid/reset)", from: "/charts/?", to: "/?", mounted: "#form-required" },
    { name: "the sidebar provider (window resize + keydown)", from: "/charts/?", to: "/docs?", mounted: '[data-slot="sidebar-wrapper"]' },
  ]) {
    test(`route: entering and leaving ${route.name} leaves no listener behind`, async ({ page }) => {
      test.setTimeout(300_000);
      await gotoHydrated(page, `${BASE_URL}${route.from}`, GOTO);
      await page.waitForTimeout(800);
      const cdp = await openCdp(page);
      const before = await inventory(cdp);
      const helperBefore = await helperLive(page);

      for (let visit = 0; visit < 2; visit++) {
        await page.locator(`a[href="${route.to}"]`).first().click();
        await expect(page.locator(route.mounted).first()).toBeVisible({ timeout: 60_000 });
        await page.waitForTimeout(500);
        await page.locator(`a[href="${route.from}"]`).first().click();
        await expect(page.locator(route.mounted)).toHaveCount(0, { timeout: 60_000 });
        await page.waitForTimeout(500);
      }

      expect(diff(before, await inventory(cdp)), `listeners left by visiting ${route.to}`).toEqual([]);
      expect(await helperLive(page), "registrations `listen()` still believes live").toEqual(helperBefore);
    });
  }
});

test("self-check: the detector sees a listener that nothing removes (and the blocking-listener rule fires on it)", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/component/?name=dialog&`, GOTO);
  await page.waitForTimeout(500);
  const cdp = await openCdp(page);
  const before = await inventory(cdp);
  // Row 149's exact shape: a capturing, non-passive wheel listener on window that outlives its surface.
  await page.evaluate(() => window.addEventListener("wheel", () => {}, { capture: true, passive: false }));
  const after = await inventory(cdp);
  expect(diff(before, after)).toEqual(["+1  window  wheel  capture  NON-PASSIVE"]);
  expect(blockingIn(after)).toEqual(["window  wheel  capture  NON-PASSIVE"]);
});

test.describe("scroll lock (row 149)", () => {
  /** Slows the Dialog's exit so "mid-exit" is a window of a second, not of a frame. */
  async function slowExit(page: Page) {
    await page.addStyleTag({ content: ":root { --dx-overlay-duration: 1200ms !important; }" });
  }

  /** Records `close()` calls, the lock flag and wheel outcomes, with one clock. */
  async function installTimeline(page: Page) {
    await page.evaluate(() => {
      const w = window as unknown as { __tl: [number, string][]; __tlStart: number };
      w.__tl = [];
      w.__tlStart = performance.now();
      const log = (what: string) => w.__tl.push([Math.round(performance.now() - w.__tlStart), what]);
      const close = HTMLDialogElement.prototype.close;
      HTMLDialogElement.prototype.close = function (...args) {
        log("dialog.close()");
        return close.apply(this, args);
      };
      // Bubble-phase, passive: runs after the lock's capture listener, so `defaultPrevented` is its verdict.
      window.addEventListener(
        "wheel",
        (e) => log(`wheel ${e.defaultPrevented ? "cancelled" : "free"} dialogOpen=${!!document.querySelector("dialog[open]")}`),
        { passive: true },
      );
      const flag = window as unknown as { __dxScrollLocked?: boolean };
      let v = flag.__dxScrollLocked;
      Object.defineProperty(window, "__dxScrollLocked", {
        configurable: true,
        get: () => v,
        set: (x) => {
          if (x !== v) log(`lock=${x} dialogConnected=${!!document.querySelector("dialog.dx-dialog")} dialogOpen=${!!document.querySelector("dialog[open]")}`);
          v = x;
        },
      });
    });
  }

  const timeline = (page: Page) => page.evaluate(() => (window as unknown as { __tl: [number, string][] }).__tl);

  test("the lock holds through the exit and lets go at the dialog's real close()", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=dialog&`, GOTO);
    await slowExit(page);
    await installTimeline(page);
    const trigger = page.getByRole("button", { name: "Show Dialog", exact: true }).first();
    await trigger.click();
    await expect(page.locator("dialog[open]")).toBeVisible();
    await page.waitForTimeout(400);

    await page.keyboard.press("Escape");
    // Mid-exit: the dialog is still open (and modal) for ~1.2 s.
    await page.waitForTimeout(350);
    expect(await page.locator("dialog[open]").count(), "the exit should still be playing").toBe(1);
    const yBefore = await page.evaluate(() => scrollY);
    await page.mouse.move(640, 400);
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(150);
    expect(await page.evaluate(() => scrollY), "the page must not scroll while the dialog is still animating out").toBe(yBefore);

    await expect(page.locator("dialog[open]")).toHaveCount(0, { timeout: 10_000 });
    await expect.poll(() => page.evaluate(() => (window as unknown as { __dxScrollLocked?: boolean }).__dxScrollLocked)).toBe(false);
    await page.waitForTimeout(600);
    await page.mouse.wheel(0, 400);
    await expect.poll(() => page.evaluate(() => scrollY), { message: "after the release a wheel scrolls the page again" }).toBeGreaterThan(yBefore);

    const tl = await timeline(page);
    const at = (prefix: string) => tl.find(([, what]) => what.startsWith(prefix))?.[0];
    const closedAt = at("dialog.close()");
    const releasedAt = at("lock=false");
    expect(closedAt, JSON.stringify(tl)).toBeDefined();
    expect(releasedAt, JSON.stringify(tl)).toBeDefined();
    // Never before the real close ...
    expect(releasedAt!, `lock released before close(): ${JSON.stringify(tl)}`).toBeGreaterThanOrEqual(closedAt!);
    // ... and from the close, not from the unmount that follows it by the ~250 ms hold: the dialog node
    // is still in the document when the flag drops.
    const release = tl.find(([, what]) => what.startsWith("lock=false"))![1];
    expect(release, `released at unmount, not at close(): ${JSON.stringify(tl)}`).toContain("dialogConnected=true");
    const midExitWheel = tl.find(([, what]) => what.startsWith("wheel"))!;
    expect(midExitWheel[1], JSON.stringify(tl)).toBe("wheel cancelled dialogOpen=true");
    expect(tl.filter(([, what]) => what.startsWith("wheel")).at(-1)![1], JSON.stringify(tl)).toMatch(/^wheel free dialogOpen=false/);
  });

  // The default headless Chromium has 0-width overlay scrollbars even for a tall page, which hides a lock
  // that changes the root scroller's width. Launching with `--hide-scrollbars` dropped gives the 15 px of most
  // users' Windows/Linux browsers. (`test.use({ launchOptions })` cannot be scoped to a describe, so this
  // test launches its own browser, with the project's executable.)
  test("under a forced classic scrollbar, documentElement.clientWidth is the same on every frame of open, exit and close", async ({ playwright }, testInfo) => {
    const executablePath = (testInfo.project.use as { launchOptions?: { executablePath?: string } }).launchOptions?.executablePath;
    const browser = await playwright.chromium.launch({ executablePath, args: ["--no-sandbox"], ignoreDefaultArgs: ["--hide-scrollbars"] });
    try {
      const page = await (await browser.newContext({ viewport: { width: 1280, height: 800 } })).newPage();
      await gotoHydrated(page, `${BASE_URL}/component/?name=dialog&`, GOTO);
      await slowExit(page);
      const gutter = await page.evaluate(() => innerWidth - document.documentElement.clientWidth);
      expect(gutter, "this launch must render a classic scrollbar, or the test proves nothing").toBeGreaterThan(0);
      await page.evaluate(() => {
        const w = window as unknown as { __widths: number[]; __raf: number };
        w.__widths = [];
        const f = () => {
          w.__widths.push(document.documentElement.clientWidth);
          w.__raf = requestAnimationFrame(f);
        };
        f();
      });
      await page.getByRole("button", { name: "Show Dialog", exact: true }).first().click();
      await expect(page.locator("dialog[open]")).toBeVisible();
      await page.waitForTimeout(400);
      await page.keyboard.press("Escape");
      await expect(page.locator("dialog[open]")).toHaveCount(0, { timeout: 10_000 });
      await page.waitForTimeout(800);
      const widths = await page.evaluate(() => {
        const w = window as unknown as { __widths: number[]; __raf: number };
        cancelAnimationFrame(w.__raf);
        return w.__widths;
      });
      expect(widths.length, "frames sampled").toBeGreaterThan(30);
      expect([...new Set(widths)], `clientWidth over ${widths.length} frames (gutter ${gutter}px)`).toHaveLength(1);
    } finally {
      await browser.close();
    }
  });
});
