/**
 * First-paint style delivery -- tier 2 (WHATWG HTML render-blocking stylesheets, CSSOM, Layout Instability).
 *
 * Rule sources:
 *   - HTML Living Standard, "the link element" / "render-blocking": a `<link rel=stylesheet>` in `<head>` blocks the
 *     first render; one inserted after `<body>` exists does NOT (the spec only lets render-blocking elements be added
 *     while the document "allows adding render-blocking elements", i.e. while `body` is null; `blocking=render` on a
 *     late link measured as a no-op, 2026-10-05). So a stylesheet that is discovered by RENDERING something arrives
 *     after that something has already been painted.
 *   - Layout Instability API (`layout-shift`): the reflow when such a sheet lands is a layout shift.
 *
 * THE REPORT this guards (owner, 2026-10-05): "elements on their component's page, on first load, show a white/light
 * version or placeholder before their dark mode or real style kicks in (also shifts layout), e.g. /component/field/.
 * Hydration?" Measured on the release SSG build of 001a5f0:
 *   - NOT hydration and NOT the theme. A hard load is right: the JS-disabled page and the hydrated page have
 *     0 differing computed styles over `main` in light and dark (this spec's parity tests), hydration inserts no
 *     stylesheet, CLS 0.0001, and the pre-paint script has set `data-theme` before the first paint.
 *   - IT IS component CSS arriving late with UA defaults. Every component's sheet is a `document::Link` at its render
 *     point. The SSR head carries those of what the SSR rendered, so a hard load is fine -- but the first time you click
 *     through to /component/field/ (client-side navigation), the route renders first and its 5-10 sheets are inserted
 *     into a document that has already painted: white `<input>`s (rgb(255,255,255), 2px border, radius 0),
 *     `.dx-field { display: block }`, then ~150-270 ms later (at +100 ms of CSS latency) the sheets land, the page
 *     reflows (CLS 0.02-0.08) and `transition: background-color` animates the white to the dark theme. 8 of 8 component
 *     pages sampled. The date picker's first-open glitch is the same class (the calendar sheet is inserted when the
 *     popover content first mounts: UA-grey 2px-bordered day buttons, 207 px tall, then 280 px styled).
 *
 * THE CONSTRUCTION (scripts/ssg-css-bundle.mjs, run by scripts/build-ssg.sh; eager_head.rs skips covered links): every
 * page's `<head>` carries ONE render-blocking `<link data-dx-css-bundle data-covers="...">` with every component sheet,
 * so what a route can need is a property of the document, not of what has rendered so far.
 *
 * WHAT THIS ORACLE ASSERTS (by property, over every component route -- a new component is covered the day it lands):
 *   1. every prerendered page's head has the bundle after the four global sheets, the same one on every page, complete;
 *   2. hard load: the JS-disabled computed styles of `main` equal the hydrated ones in both system colour schemes, and
 *      no stylesheet is inserted that the head/bundle did not already carry;
 *   3. client-side navigation to every component: the computed styles in the very first state the new route is in
 *      (the DOM the browser would paint) equal its settled ones, no uncovered sheet is inserted, CLS stays ~0;
 *   4. the date picker's calendar is styled the moment it first mounts;
 *   5. an explicit theme choice (cookie) is on `<html>` before the first paint and never changes afterwards;
 *   6. the detector is not blind (a page with its stylesheet removed is reported).
 *
 * RED/GREEN: tests 1, 3 and 4 are red on a build without the bundle (`scripts/build-ssg.sh --no-css-bundle`, or any
 * build older than this oracle) and green with it; test 2 is green on both (it documents that a hard load was always
 * right). Run against an SSG build, served statically (the default :8090; `dx serve` has no bundle and no
 * prerendered head):
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8090 npx playwright test --config=ssg.local.config.ts oracle/tier2-html/first-paint-styles.spec.ts
 * `FIRST_PAINT_ROUTES=field,date_picker` limits the per-route tests; `FIRST_PAINT_CSS_DELAY_MS` (default 150) is the
 * simulated latency of stylesheets fetched after load, which is what makes a late sheet observable on a fast loopback.
 * Functional, not timing: it is valid on a debug build.
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { baseUrlOr } from "../../base-url";

const BASE = baseUrlOr("http://127.0.0.1:8090");
const PREVIEW = path.join(__dirname, "..", "..", "..", "preview");
const CSS_DELAY_MS = Number(process.env.FIRST_PAINT_CSS_DELAY_MS ?? 150);

/** Every component that has a docs page (`/component/<slug>/`), from the sources so a new component is covered the day it lands. */
const ALL_COMPONENTS: string[] = fs
  .readdirSync(path.join(PREVIEW, "src", "components"), { withFileTypes: true })
  .filter((d) => d.isDirectory() && fs.existsSync(path.join(PREVIEW, "src", "components", d.name, "docs.md")))
  .map((d) => d.name)
  .sort();
/** `FIRST_PAINT_ROUTES=field,date_picker` narrows the per-route tests (the whole-site tests always cover every page). */
const COMPONENTS: string[] = (() => {
  const only = (process.env.FIRST_PAINT_ROUTES ?? "").split(",").map((s) => s.trim()).filter(Boolean);
  return only.length ? ALL_COMPONENTS.filter((c) => only.includes(c)) : ALL_COMPONENTS;
})();

/** The four sheets `GlobalHead` links on every route; the bundle must come right after them. */
const GLOBAL_SHEET_PREFIXES = ["main-", "dx-components-theme-", "dx-effects-", "theme-presets-"];

/** Computed-style properties that decide how something LOOKS and where it sits. */
const PROPS = [
  "display", "position", "width", "height", "margin-top", "margin-right", "margin-bottom", "margin-left",
  "padding-top", "padding-right", "padding-bottom", "padding-left", "border-top-width", "border-right-width",
  "border-bottom-width", "border-left-width", "border-top-left-radius", "border-top-style", "font-size", "font-weight",
  "line-height", "letter-spacing", "color", "background-color", "opacity", "gap", "flex-direction", "align-items",
  "justify-content", "box-shadow", "text-transform", "font-family", "overflow-x", "overflow-y", "visibility",
  "cursor", "outline-width", "border-top-color",
];

type Snap = Record<string, Record<string, string>>;
/** Content that sizes itself after mount (chart axes, avatar image) lands within a pixel; a stylesheet arriving late moves things by tens. */
const SUBPIXEL = 1;

/**
 * `window.__dxSnap(selector)`: the computed styles of an element's whole subtree, keyed by position + tag + first two
 * classes. Installed with `evaluate`, which works with JavaScript disabled too. Animations and transitions are switched
 * off so a value read mid-flight cannot masquerade as a difference.
 */
async function installSnap(page: Page) {
  // Not `page.addStyleTag`: it hangs with JavaScript disabled, `evaluate` does not.
  await page.evaluate((props) => {
    const off = document.createElement("style");
    off.textContent = "*,*::before,*::after{transition:none!important;animation:none!important}";
    document.head.appendChild(off);
    (window as unknown as { __dxSnap: (sel: string) => unknown }).__dxSnap = (sel: string) => {
      const out: Record<string, Record<string, string>> = {};
      const root = document.querySelector(sel);
      if (!root) return out;
      // Key = the ancestors' keys + tag.classes + the nth such sibling, so content a client render inserts next to an
      // element (a chart's axes, a select's options) does not renumber the elements that were already there.
      const label = (el: Element) => {
        const cls = typeof el.className === "string" ? el.className.trim().split(/\s+/).slice(0, 2).join(".") : "";
        return `${el.tagName.toLowerCase()}${cls ? "." + cls : ""}`;
      };
      const walk = (el: Element, at: string) => {
        const cs = getComputedStyle(el);
        const rec: Record<string, string> = {};
        for (const p of props) rec[p] = cs.getPropertyValue(p);
        // The element's own state: a differently-stated element (an avatar whose image loaded, a chart that measured itself) is
        // content a client render changed, not a style the delivery got wrong. Ids and per-render values are left out.
        rec.__sig = el
          .getAttributeNames()
          .filter((n) => !["id", "style", "for", "data-node-hydration", "aria-controls", "aria-labelledby", "aria-describedby", "popovertarget"].includes(n))
          .sort()
          .map((n) => `${n}=${el.getAttribute(n)}`)
          .join(" ");
        out[at] = rec;
        const seen: Record<string, number> = {};
        for (const c of el.children) {
          if (c.tagName === "SCRIPT" || c.tagName === "STYLE") continue;
          const l = label(c);
          walk(c, `${at}/${l}#${(seen[l] = (seen[l] ?? 0) + 1)}`);
        }
      };
      walk(root, label(root));
      return out;
    };
  }, PROPS);
}

const snapOf = (page: Page, sel: string): Promise<Snap> =>
  page.evaluate((s) => (window as unknown as { __dxSnap: (sel: string) => Snap }).__dxSnap(s), sel);

/**
 * Every computed-style difference between two snapshots of the same subtree, over the elements both have in the same state
 * (`__sig`: classes and attributes). An element only one of them has is NOT a style difference (14 of 76 pages render different content after hydration: a chart's
 * axes, an avatar's image-or-fallback, a carousel's slides -- all in space the page reserved, which the CLS bound
 * checks), so it is left to that bound instead of being reported here.
 */
function diffSnaps(a: Snap, b: Snap): string[] {
  const out: string[] = [];
  for (const k of Object.keys(a)) {
    if (!b[k] || a[k].__sig !== b[k].__sig) continue;
    for (const p of PROPS) {
      if (a[k][p] === b[k][p]) continue;
      // A box that is 342.719px tall in the static page and 342px once a chart has measured itself is not a style difference.
      if ((p === "width" || p === "height") && Math.abs(parseFloat(a[k][p]) - parseFloat(b[k][p])) <= SUBPIXEL) continue;
      out.push(`${k} { ${p}: ${a[k][p]} -> ${b[k][p]} }`);
    }
  }
  return out;
}

const summarize = (diffs: string[]) =>
  `${diffs.length} difference(s); first: ${diffs.slice(0, Number(process.env.FIRST_PAINT_DIFFS ?? 6)).join(" | ")}`;

/** Fonts come from Google: unreachable in the sandbox and a source of nondeterminism, so both loads use the fallback. */
async function noWebFonts(page: Page) {
  await page.route(/https:\/\/fonts\.(googleapis|gstatic)\.com\//, (r) => r.abort());
}

/** Delay every stylesheet fetched from now on: on loopback a late sheet lands in a few ms and the unstyled frame is invisible. */
async function delayCssFetches(page: Page) {
  let on = false;
  await page.route("**/assets/*.css", async (route) => {
    if (on) await new Promise((r) => setTimeout(r, CSS_DELAY_MS));
    await route.continue();
  });
  return { start: () => (on = true) };
}

/** Records every stylesheet `<link>` added to the document (the parser's too: subtract `__dcl`) and every layout shift, from before the page's own scripts. */
const OBSERVE_INIT = () => {
  const w = window as unknown as { __sheets: string[]; __dcl: string[]; __cls: number[]; __theme: { t: number; from: string | null; to: string | null }[]; __fcp: number | null };
  w.__sheets = [];
  w.__dcl = []; // the stylesheets of the page as served: what the parser had put in the document by DOMContentLoaded
  document.addEventListener("DOMContentLoaded", () => {
    w.__dcl = [...document.querySelectorAll("link[rel=stylesheet]")].map((l) => (l.getAttribute("href") ?? "").split("/").pop() ?? "");
  });
  w.__cls = [];
  w.__theme = [];
  w.__fcp = null;
  new MutationObserver((ms) => {
    for (const m of ms)
      for (const n of m.addedNodes)
        if (n instanceof HTMLLinkElement && n.rel === "stylesheet") w.__sheets.push((n.getAttribute("href") ?? "").split("/").pop() ?? "");
  }).observe(document, { childList: true, subtree: true });
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) w.__cls.push((e as unknown as { value: number }).value);
  }).observe({ type: "layout-shift", buffered: true });
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) if (e.name === "first-contentful-paint") w.__fcp = e.startTime;
  }).observe({ type: "paint", buffered: true });
  // `<html>` does not exist yet when an init script runs: observe the document and filter.
  new MutationObserver((ms) => {
    for (const m of ms)
      if (m.target === document.documentElement) w.__theme.push({ t: performance.now(), from: m.oldValue, to: document.documentElement.getAttribute("data-theme") });
  }).observe(document, { attributes: true, subtree: true, attributeFilter: ["data-theme"], attributeOldValue: true });
};

/** The file names of the stylesheets in `<head>` right now, and of the ones the page's bundle carries. */
async function headAndCovers(page: Page): Promise<{ head: string[]; covers: string[] }> {
  return page.evaluate(() => ({
    head: [...document.querySelectorAll("link[rel=stylesheet]")].map((l) => (l.getAttribute("href") ?? "").split("/").pop() ?? ""),
    covers: (document.querySelector("link[data-dx-css-bundle]")?.getAttribute("data-covers") ?? "").split(" ").filter(Boolean),
  }));
}

/** Same-origin stylesheet links whose sheet has not loaded yet (the aborted web-font link never will, so it is not counted). */
const pendingSheets = (page: Page) =>
  page.evaluate(() => [...document.querySelectorAll("link[rel=stylesheet]")].filter((l) => !/^(https?:)?\/\//.test(l.getAttribute("href") ?? "") && !(l as HTMLLinkElement).sheet).length);

async function hydrated(page: Page) {
  await page.waitForSelector('html[data-hydrated="true"]', { state: "attached", timeout: 90_000 });
}

// ---------------------------------------------------------------------------------------------
// 1. The raw HTML: what the browser has before any script runs.
// ---------------------------------------------------------------------------------------------

const PAGES = [
  ...ALL_COMPONENTS.map((c) => `/component/${c}/`),
  "/",
  "/docs",
  "/demos",
  "/charts/",
  "/charts/line/",
  "/dashboard/email-client",
  "/component/block/sidebar/main/",
];
const headOf = (html: string) => html.slice(0, html.indexOf("</head>"));
const sheetLinks = (head: string) => [...head.matchAll(/<link\b[^>]*>/g)].map((m) => m[0]).filter((t) => /\srel="stylesheet"/.test(t) && !/\shref="(https?:)?\/\//.test(t));
const hrefOf = (tag: string) => tag.match(/\shref="([^"]*)"/)?.[1] ?? "";

test.describe("1. every prerendered page's head carries the complete stylesheet set", () => {
  test("the route inventory is real (a wrong path would make every per-route test below vacuous)", () => {
    expect(ALL_COMPONENTS.length, "component pages found from preview/src/components/*/docs.md").toBeGreaterThan(60);
  });

  test("the pre-paint theme script precedes every stylesheet, and one bundle follows the four global sheets, identically on every page", async ({ request }) => {
    const seen = new Map<string, string>(); // `${href} ${covers}` -> a page that had it
    const problems: string[] = [];
    for (const url of PAGES) {
      const res = await request.get(`${BASE}${url}`);
      if (res.status() !== 200) {
        problems.push(`${url}: HTTP ${res.status()}`);
        continue;
      }
      const head = headOf(await res.text());
      const links = sheetLinks(head);
      const bundles = links.filter((t) => t.includes("data-dx-css-bundle"));
      if (bundles.length !== 1) {
        problems.push(`${url}: ${bundles.length} bundle link(s) in <head> (a route-specific stylesheet set: its sheets are inserted after first paint on client-side navigation)`);
        continue;
      }
      const at = links.indexOf(bundles[0]);
      const before = links.slice(0, at).map((t) => hrefOf(t).split("/").pop() ?? "");
      if (before.length !== GLOBAL_SHEET_PREFIXES.length || !GLOBAL_SHEET_PREFIXES.every((p, i) => before[i].startsWith(p))) {
        problems.push(`${url}: the bundle must come right after ${GLOBAL_SHEET_PREFIXES.join(", ")}; found ${before.join(", ")}`);
      }
      const prepaint = head.indexOf("__dxTheme");
      const firstSheet = head.search(/<link\b[^>]*\srel="stylesheet"/);
      if (prepaint < 0 || (firstSheet >= 0 && prepaint > firstSheet)) {
        problems.push(`${url}: the pre-paint theme script must run before the first stylesheet (it decides data-theme before anything paints)`);
      }
      const covers = bundles[0].match(/\sdata-covers="([^"]*)"/)?.[1] ?? "";
      const key = `${hrefOf(bundles[0])} ${covers}`;
      if (!seen.has(key)) seen.set(key, url);
    }
    expect(problems, problems.join("\n")).toEqual([]);
    expect([...seen.values()], "one bundle for the whole site: the same href and the same file list on every page").toHaveLength(1);
  });

  test("web fonts cannot shift the layout: the latin woff2 files are preloaded and the font stylesheet is display=optional", async ({ request }) => {
    // Measured (css +100 ms, files +200 ms): `display=swap` = CLS 0.117 on /component/form/, 0.048 on /carousel/; preload + optional = 0.000
    // on both, with Geist in use at first contentful paint; a stale preload URL under `optional` stays 0.000 (under `swap` it is 0.117 again).
    const problems: string[] = [];
    for (const url of ["/", "/docs", "/component/field/", "/component/form/", "/charts/"]) {
      const head = headOf(await (await request.get(`${BASE}${url}`)).text());
      const links = [...head.matchAll(/<link\b[^>]*>/g)].map((m) => m[0]);
      const preloads = links.filter((t) => /\srel="preload"/.test(t) && /\sas="font"/.test(t));
      if (preloads.length < 2) problems.push(`${url}: ${preloads.length} font preload(s), expected the Geist and Geist Mono latin files`);
      for (const t of preloads) {
        if (!/\scrossorigin/.test(t)) problems.push(`${url}: a font preload without crossorigin is fetched twice and never matched: ${t}`);
        if (!/\stype="font\/woff2"/.test(t)) problems.push(`${url}: a font preload should say type="font/woff2": ${t}`);
      }
      const css = links.find((t) => /fonts\.googleapis\.com\/css2/.test(t));
      if (!css) problems.push(`${url}: no Google font stylesheet link`);
      else if (!/display=optional/.test(css)) problems.push(`${url}: the font stylesheet must say display=optional (swap re-wraps the page when Geist arrives): ${css}`);
      const firstPreload = head.search(/<link\b[^>]*\srel="preload"[^>]*\sas="font"/);
      const firstSheet = head.search(/<link\b[^>]*\srel="stylesheet"/);
      if (firstPreload < 0 || (firstSheet >= 0 && firstPreload > firstSheet)) problems.push(`${url}: the font preloads must come before the first stylesheet so the fetch starts with the page`);
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  test("the bundle is complete: it carries a rule of every stylesheet the preview's sources ship with asset!()", async ({ request }) => {
    const head = headOf(await (await request.get(`${BASE}/component/field/`)).text());
    const bundle = sheetLinks(head).find((t) => t.includes("data-dx-css-bundle"));
    expect(bundle, "the field page's head has a bundle link").toBeTruthy();
    // The href is root-relative INCLUDING any `--base-path` prefix, and BASE already carries that prefix when the
    // site is served under one, so resolve it as a URL rather than gluing it on (that doubled the prefix: a 404 body).
    const css = await (await request.get(new URL(hrefOf(bundle!), `${BASE}/`).toString())).text();
    expect(css.length, "the bundle is not an empty file").toBeGreaterThan(50_000);
    const walk = (dir: string, ext: string): string[] =>
      fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name), ext) : e.name.endsWith(ext) ? [path.join(dir, e.name)] : []));
    const rust = walk(path.join(PREVIEW, "src"), ".rs").map((f) => fs.readFileSync(f, "utf8")).join("\n");
    const missing: string[] = [];
    // Every project stylesheet under preview/src (components/*/style.css, theme.css, dashboard CSS, ...) that some `asset!(..)` links.
    // A `#[css_module]` sheet (top_layer's fixture) hashes its class names, so no fixed class can be looked up in it.
    for (const file of walk(path.join(PREVIEW, "src"), ".css")) {
      const rel = "/" + path.relative(PREVIEW, file).split(path.sep).join("/");
      if (!rust.includes(`asset!("${rel}")`)) continue;
      const src = fs.readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
      const cls = src.match(/\.((?:dx|ec)-[A-Za-z0-9_-]+)/)?.[1];
      if (cls && !new RegExp(`\\.${cls}(?![\\w-])`).test(css)) missing.push(`${rel} (.${cls})`);
    }
    expect(missing, `stylesheets with no rule in the bundle: ${missing.join(", ")}`).toEqual([]);
  });
});

// ---------------------------------------------------------------------------------------------
// 2. A hard load: JS-disabled == hydrated, and hydration brings no stylesheet.
// ---------------------------------------------------------------------------------------------

/**
 * Routes that size or build part of themselves from a CLIENT measurement or state after mount (measured 2026-10-05 on all 76
 * routes; every other route has 0 computed-style differences between the JS-off page and the hydrated one in both schemes, and
 * between a navigation's first state and its settled one): charts take their size from a ResizeObserver (JS-off `<rect>` 46x178,
 * hydrated 70x312; a chart card 324.6 px then 342 px), a native `<select>` has no options until hydration (22 px wide, then
 * 94 px), resizable panels are 0 px then 224 px, an avatar swaps image and fallback, a toast region switches from display:none,
 * carousels and virtual lists build their items on the client, a message scroller scrolls itself. None of that is stylesheet
 * delivery, so the style-parity comparisons skip these routes; the "no uncovered stylesheet" and CLS checks still apply to them
 * (the CLS bound is what holds them to "reserved space"). A route that starts differing fails until somebody looks and adds it
 * here with a reason; the list is not a place to hide a late stylesheet (that is test 1 and the late-sheet checks).
 */
const CLIENT_MEASURED = new Set([
  "area_chart", "avatar", "bar_chart", "carousel", "chart", "chart_tooltip", "data_table", "drag_and_drop_list", "form", "item",
  "line_chart", "message_scroller", "pie_chart", "progress", "resizable", "select", "toast", "top_layer", "virtual_list",
]);

test.describe("2. hard load: the static page is the page", () => {
  test.describe.configure({ mode: "parallel" });
  for (const slug of COMPONENTS) {
    test(`/component/${slug}/: JavaScript off and hydrated compute the same styles, light and dark`, async ({ browser }) => {
      const read = async (js: boolean) => {
        const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 }, javaScriptEnabled: js, colorScheme: "light" });
        const page = await ctx.newPage();
        await noWebFonts(page);
        if (js) await page.addInitScript(OBSERVE_INIT);
        await page.goto(`${BASE}/component/${slug}/`, { waitUntil: "load" });
        if (js) {
          await hydrated(page);
          await page.waitForTimeout(500);
        }
        await installSnap(page);
        const snaps: Record<string, Snap> = {};
        for (const scheme of ["light", "dark"] as const) {
          await page.emulateMedia({ colorScheme: scheme });
          await page.waitForTimeout(50);
          snaps[scheme] = await snapOf(page, "main");
        }
        const late = js
          ? await page.evaluate(() => {
              const w = window as unknown as { __sheets: string[]; __dcl: string[] };
              return w.__sheets.filter((f) => !w.__dcl.includes(f));
            })
          : [];
        const sets = js ? await headAndCovers(page) : { head: [], covers: [] };
        const cls = js ? await page.evaluate(() => (window as unknown as { __cls: number[] }).__cls.reduce((a, b) => a + b, 0)) : 0;
        await ctx.close();
        return { snaps, late, ...sets, cls };
      };
      const off = await read(false);
      const on = await read(true);
      if (!CLIENT_MEASURED.has(slug)) {
        for (const scheme of ["light", "dark"] as const) {
          const d = diffSnaps(off.snaps[scheme], on.snaps[scheme]);
          expect(d, `${scheme}: JS-off vs hydrated: ${summarize(d)}`).toEqual([]);
        }
      }
      const uncovered = on.late.filter((f) => !on.covers.includes(f));
      expect(uncovered, `stylesheets inserted after load that the head/bundle did not carry: ${uncovered.join(", ")}`).toEqual([]);
      expect(on.cls, "layout shift of a hard load incl. hydration (no web fonts)").toBeLessThan(0.01);
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 3. Client-side navigation: the first state of a new route is its final state.
// ---------------------------------------------------------------------------------------------

test.describe("3. client-side navigation: no route is painted with late styles", () => {
  test.describe.configure({ mode: "parallel" });
  for (const slug of COMPONENTS) {
    test(`/docs -> /component/${slug}/`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      await noWebFonts(page);
      const css = await delayCssFetches(page);
      await page.addInitScript(OBSERVE_INIT);
      await page.goto(`${BASE}/docs`, { waitUntil: "domcontentloaded" });
      await hydrated(page);
      await page.waitForTimeout(500);
      await installSnap(page);
      const before = await headAndCovers(page);
      // The state the browser would paint: the moment the route's article exists, before anything else can run.
      await page.evaluate(() => {
        const w = window as unknown as { __first: unknown; __dxSnap: (s: string) => unknown; __cls: number[]; __sheets: string[] };
        w.__first = null;
        w.__cls.length = 0;
        w.__sheets.length = 0;
        const mo = new MutationObserver(() => {
          if (w.__first == null && document.querySelector("article.dx-component-page")) {
            w.__first = w.__dxSnap("article.dx-component-page");
            mo.disconnect();
          }
        });
        mo.observe(document.body, { childList: true, subtree: true });
      });
      css.start();
      await page.locator(`a.dx-sidebar-menu-button[href*="/component/${slug}/"]`).first().click();
      await page.waitForSelector("article.dx-component-page", { state: "attached", timeout: 30_000 });
      // Settled: every stylesheet link has its sheet, and a beat for styles to apply.
      await expect
        .poll(() => pendingSheets(page), { timeout: 20_000 })
        .toBe(0);
      await page.waitForTimeout(400);
      const first = (await page.evaluate(() => (window as unknown as { __first: Snap | null }).__first)) as Snap | null;
      expect(first, "the new route's article was observed").toBeTruthy();
      const settled = await snapOf(page, "article.dx-component-page");
      if (!CLIENT_MEASURED.has(slug)) {
        const d = diffSnaps(first!, settled);
        expect(d, `the first state of /component/${slug}/ differs from its settled state (what the user sees flash): ${summarize(d)}`).toEqual([]);
      }

      const late = (await page.evaluate(() => (window as unknown as { __sheets: string[] }).__sheets)).filter((f) => !before.head.includes(f) && !before.covers.includes(f));
      expect(late, `stylesheets inserted by the navigation that neither the head nor the bundle carried: ${late.join(", ")}`).toEqual([]);
      const cls = await page.evaluate(() => (window as unknown as { __cls: number[] }).__cls.reduce((a, b) => a + b, 0));
      expect(cls, "layout shift during the navigation (no web fonts)").toBeLessThan(0.01);
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 4. Late-mounting subtrees: the date picker's calendar, on its FIRST open.
// ---------------------------------------------------------------------------------------------

test.describe("4. a popover's content is styled the moment it first mounts", () => {
  for (const [slug, trigger, marker] of [
    ["date_picker", ".dx-date-picker .dx-date-picker-popover-trigger", ".dx-calendar-grid-cell"],
  ] as const) {
    test(`/component/${slug}/: first open`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      await noWebFonts(page);
      const css = await delayCssFetches(page);
      await page.goto(`${BASE}/component/${slug}/`, { waitUntil: "domcontentloaded" });
      await hydrated(page);
      await page.waitForTimeout(500);
      await installSnap(page);
      await page.evaluate((marker) => {
        const w = window as unknown as { __firstOpen: unknown; __dxSnap: (s: string) => unknown };
        w.__firstOpen = null;
        const mo = new MutationObserver(() => {
          const cell = document.querySelector(marker);
          const host = cell?.closest("[popover]");
          if (w.__firstOpen == null && host) {
            host.setAttribute("data-first-open-probe", "");
            w.__firstOpen = w.__dxSnap("[data-first-open-probe]");
            mo.disconnect();
          }
        });
        mo.observe(document.body, { childList: true, subtree: true });
      }, marker);
      css.start();
      await page.locator(trigger).first().click();
      await page.waitForSelector(marker, { state: "attached", timeout: 15_000 });
      await expect
        .poll(() => pendingSheets(page), { timeout: 20_000 })
        .toBe(0);
      await page.waitForTimeout(400);
      const first = (await page.evaluate(() => (window as unknown as { __firstOpen: Snap | null }).__firstOpen)) as Snap | null;
      expect(first, "the popover content was observed when it first mounted").toBeTruthy();
      const settled = await snapOf(page, "[data-first-open-probe]");
      const d = diffSnaps(first!, settled);
      expect(d, `the calendar's first open differs from its settled state (the glitch on the first open per page load): ${summarize(d)}`).toEqual([]);
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 5. The theme is decided before first paint.
// ---------------------------------------------------------------------------------------------

test.describe("5. an explicit theme is on <html> before first paint and never flips", () => {
  for (const [system, chosen] of [
    ["light", "dark"],
    ["dark", "light"],
  ] as const) {
    test(`system ${system}, saved choice ${chosen}`, async ({ browser }) => {
      const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 }, colorScheme: system });
      await ctx.addCookies([{ name: "dx_theme", value: chosen, url: BASE }]);
      const page = await ctx.newPage();
      await noWebFonts(page);
      await page.addInitScript(OBSERVE_INIT);
      // The colour the page is actually painted with at its first contentful paint.
      await page.addInitScript(() => {
        const w = window as unknown as { __atFcp: { theme: string | null; bg: string } | null };
        w.__atFcp = null;
        new PerformanceObserver((l) => {
          for (const e of l.getEntries())
            if (e.name === "first-contentful-paint" && !w.__atFcp) w.__atFcp = { theme: document.documentElement.getAttribute("data-theme"), bg: getComputedStyle(document.body).backgroundColor };
        }).observe({ type: "paint", buffered: true });
      });
      await page.goto(`${BASE}/component/field/`, { waitUntil: "load" });
      await hydrated(page);
      await page.waitForTimeout(300);
      const r = await page.evaluate(() => {
        const w = window as unknown as { __atFcp: { theme: string | null; bg: string } | null; __theme: { from: string | null; to: string | null }[] };
        return { atFcp: w.__atFcp, writes: w.__theme, theme: document.documentElement.getAttribute("data-theme"), bg: getComputedStyle(document.body).backgroundColor };
      });
      expect(r.atFcp?.theme, "data-theme at the first contentful paint").toBe(chosen);
      expect(r.atFcp?.bg, "the colour painted at first contentful paint is the final one").toBe(r.bg);
      const flips = r.writes.filter((w) => w.from !== w.to && w.from !== null);
      expect(flips, `data-theme changed value after being set: ${JSON.stringify(flips)}`).toEqual([]);
      expect(r.theme).toBe(chosen);
      await ctx.close();
    });
  }
});

// ---------------------------------------------------------------------------------------------
// 6. The detector is not blind.
// ---------------------------------------------------------------------------------------------

test("the oracle is not vacuous: a page whose stylesheet is removed is reported", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await noWebFonts(page);
  await page.goto(`${BASE}/component/field/`, { waitUntil: "load" });
  await hydrated(page);
  await installSnap(page);
  const styled = await snapOf(page, "main");
  // Drop every stylesheet a component contributes: the bundle when there is one, otherwise the per-route links.
  const removed = await page.evaluate(() => {
    const keep = ["main-", "dx-components-theme-", "dx-effects-", "theme-presets-"];
    let n = 0;
    for (const l of [...document.querySelectorAll("link[rel=stylesheet]")]) {
      const f = (l.getAttribute("href") ?? "").split("/").pop() ?? "";
      if (/^https?:/.test(l.getAttribute("href") ?? "") || keep.some((k) => f.startsWith(k))) continue;
      l.remove();
      n++;
    }
    return n;
  });
  expect(removed, "found component stylesheets to remove").toBeGreaterThan(0);
  const bare = await snapOf(page, "main");
  const d = diffSnaps(styled, bare);
  expect(d.length, "removing the component stylesheets must change computed styles, or the comparisons above prove nothing").toBeGreaterThan(50);
  expect(d.join("\n")).toMatch(/\.dx-input/);
});
