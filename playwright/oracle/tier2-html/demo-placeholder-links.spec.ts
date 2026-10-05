/**
 * ORACLE: tier 2 (HTML) -- a demo's placeholder link (`href="#"`) goes nowhere.
 *
 * Source: WHATWG HTML Living Standard, "navigate to a fragment"
 *   https://html.spec.whatwg.org/multipage/browsing-the-web.html#navigate-fragid
 * and, for the empty fragment, the indicated part of the document
 *   https://html.spec.whatwg.org/multipage/browsing-the-web.html#the-indicated-part-of-the-document
 * Following `<a href="#">` navigates to the fragment "" of the current URL: the URL gains a trailing `#`
 * (a new history entry) and, because the empty fragment indicates the top of the document, the viewport is
 * scrolled to the top -- smoothly, under the site's global `scroll-behavior: smooth`.
 *
 * WHAT THIS GUARDS (audit 2026-10-05, item 6): 17 demos use `href: "#"` as a stand-in for a destination
 * (Pagination x5, Item x3, Breadcrumb x2, Marker x2, Bubble, Attachment, Card, and the top_layer fixture
 * x2). Measured on `/component/pagination/`: a click changed the URL to `...?#` and `scrollY` 120 -> 0.
 * The same class as the submit buttons `DemoForm` closed (`demo-forms-no-navigation.spec.ts`): a demo must
 * not be able to move the page.
 *
 * The construction is ONE capture-phase click listener in `preview/index.html`'s existing delegated
 * same-page-anchor handler, which `preventDefault()`s `a[href="#"]` and nothing else (propagation is left
 * alone, so a demo's own `onclick` still runs). It lives in the document shell, not in a Rust handler, so it
 * also covers a prerendered page BEFORE the wasm bundle has hydrated it, and a block-demo iframe, and a new
 * `href: "#"` needs no ceremony.
 *
 * ORACLE, by property rather than by instance: the pages are READ from the demo sources (every component
 * directory whose `.rs` files write `href: "#"` outside a comment, so a new placeholder link is covered the
 * day it lands) plus the homepage, which renders every demo. On each, every visible `main a[href="#"]` is
 * clicked and activated with Enter; the URL (including its hash) must not change and the page must not jump to
 * the top (`scrollY` to 0, where an empty fragment points).
 *
 *   1. (hydrated) as above.
 *   2. (before hydration) the wasm bundle is blocked and the served markup is clicked. Needs prerendered
 *      HTML, so it skips on a client-rendered `dx serve` and is meant for the SSG build.
 *   3. (calibration) the detector sees a real fragment navigation: a link to `#main-content` DOES change the
 *      URL's hash in the same harness, so a green above is not the detector being blind.
 *
 * Run against a dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts oracle/tier2-html/demo-placeholder-links.spec.ts
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "../../base-url";
import { gotoHydrated } from "../../hydration";

const COMPONENTS_DIR = path.join(__dirname, "..", "..", "..", "preview", "src", "components");

/** Every `.rs` file under `dir`. */
function rustFiles(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    return e.isDirectory() ? rustFiles(p) : e.name.endsWith(".rs") ? [p] : [];
  });
}

/** Component directories whose Rust source has a `href: "#"` outside a `//` comment. */
const WITH_PLACEHOLDER = fs
  .readdirSync(COMPONENTS_DIR, { withFileTypes: true })
  .filter((e) => e.isDirectory())
  .map((e) => e.name)
  .filter((name) =>
    rustFiles(path.join(COMPONENTS_DIR, name)).some((f) =>
      fs
        .readFileSync(f, "utf8")
        .split("\n")
        .some((line) => /href:\s*"#"/.test(line) && !/^\s*\/\//.test(line)),
    ),
  );

const PAGES = [{ id: "home", path: "/" }, ...WITH_PLACEHOLDER.map((name) => ({ id: name, path: `/component/${name}/` }))];
const NAV_TIMEOUT = 5 * 60 * 1000;

const LINKS = 'main a[href="#"]';

/**
 * Where to find a page's placeholder links. Most pages render them in place. A link inside an overlay's
 * content exists only while that overlay is open and closes it when activated, so such a page lists one
 * scenario per overlay: `open` is run before EACH activation.
 */
interface Scenario {
  link: string;
  open?: (page: Page) => Promise<void>;
}
const isOverlay = (scenario: Scenario) => scenario.open !== undefined;
const SCENARIOS: Record<string, Scenario[]> = {
  top_layer: ["clip", "edge-bottom"].map((which) => ({
    link: `#${which}-navigation-menu-content a[href="#"]`,
    open: async (page: Page) => {
      if (!(await page.locator(`#${which}-navigation-menu-content a[href="#"]`).isVisible())) {
        await page.locator(`#${which}-navigation-menu-trigger`).click();
      }
      await expect(page.locator(`#${which}-navigation-menu-content a[href="#"]`)).toBeVisible();
    },
  })),
};

test("the pages parse from the demo sources", () => {
  expect(WITH_PLACEHOLDER).toEqual(expect.arrayContaining(["pagination", "item", "breadcrumb", "marker", "bubble", "attachment", "card"]));
});

type Mark = { url: string; scrollY: number };
const mark = (page: Page): Promise<Mark> => page.evaluate(() => ({ url: location.href, scrollY: Math.round(scrollY) }));

/** Activate every visible placeholder link of `scenario`, by click and by Enter, and report what moved. */
async function activateAll(page: Page, scenario: Scenario): Promise<{ count: number; moved: string[] }> {
  await scenario.open?.(page);
  const total = await page.locator(scenario.link).count();
  const moved: string[] = [];
  let count = 0;
  for (let i = 0; i < total; i++) {
    for (const via of ["click", "enter"] as const) {
      await scenario.open?.(page);
      const link = page.locator(scenario.link).nth(i);
      if (!(await link.isVisible())) continue;
      if (!isOverlay(scenario)) {
        // (An open overlay closes when the page scrolls, so its links are activated where they are.)
        await link.scrollIntoViewIfNeeded();
        // Scroll the page down a little, keeping the link on screen below the sticky nav (so Playwright's own
        // pre-click scroll has nothing to do), so a jump to the top would be a visible scroll change.
        await link.evaluate((el) => {
          const room = Math.min(120, el.getBoundingClientRect().top - 90, document.documentElement.scrollHeight - innerHeight - scrollY);
          if (room > 0) scrollBy(0, room);
        });
        if (!(await link.isVisible())) continue;
      }
      const before = await mark(page);
      const label = `${via} on link ${i + 1} (${(await link.innerText({ timeout: 2000 }).catch(() => "")).trim().slice(0, 24) || "no text"})`;
      if (via === "click") await link.click({ timeout: 5000 });
      else {
        await link.focus({ timeout: 5000 });
        await page.keyboard.press("Enter");
      }
      await page.waitForTimeout(250);
      const after = await mark(page);
      if (after.url !== before.url) moved.push(`${label}: URL ${before.url} -> ${after.url}`);
      // `href="#"` indicates the top of the document, so the failure is a jump to exactly 0. (Not "any scroll
      // change": the homepage's `content-visibility` cards re-lay-out as they render and shift the offset.)
      if (before.scrollY > 1 && after.scrollY <= 1) moved.push(`${label}: scrollY ${before.scrollY} -> ${after.scrollY} (jumped to the top)`);
      count++;
    }
  }
  return { count, moved };
}

for (const { id, path: route } of PAGES) {
  test(`${id}: activating a placeholder link does not change the URL or jump to the top`, async ({ page }) => {
    test.setTimeout(NAV_TIMEOUT);
    await gotoHydrated(page, BASE_URL + route, { timeout: NAV_TIMEOUT });
    let activated = 0;
    const moved: string[] = [];
    for (const scenario of SCENARIOS[id] ?? [{ link: LINKS }]) {
      const result = await activateAll(page, scenario);
      activated += result.count;
      moved.push(...result.moved);
    }
    expect(activated, "at least one placeholder link was on screen to activate").toBeGreaterThan(0);
    expect(moved, "a placeholder link moved the page").toEqual([]);
  });
}

test("before hydration (wasm blocked): a placeholder link in the served HTML goes nowhere", async ({ page }) => {
  test.setTimeout(NAV_TIMEOUT);
  await page.route(/\.wasm(\?.*)?$/, (r) => r.abort());
  await page.goto(`${BASE_URL}/component/pagination/`, { waitUntil: "load", timeout: NAV_TIMEOUT });
  const prerendered = await page.locator(LINKS).count();
  test.skip(prerendered === 0, "client-rendered build (dx serve): there is no prerendered HTML to click before hydration");
  const { count, moved } = await activateAll(page, { link: LINKS });
  expect(count).toBeGreaterThan(0);
  expect(moved, "a placeholder link moved the un-hydrated page").toEqual([]);
});

test("calibration: a fragment link to a real id DOES change the URL in this harness", async ({ page }) => {
  test.setTimeout(NAV_TIMEOUT);
  await gotoHydrated(page, `${BASE_URL}/component/pagination/`, { timeout: NAV_TIMEOUT });
  await page.evaluate(() => {
    const a = document.createElement("a");
    a.id = "calibration-link";
    a.href = "#main-content";
    a.textContent = "calibration";
    document.querySelector("main")!.prepend(a);
  });
  await page.locator("#calibration-link").click();
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("#main-content");
});
