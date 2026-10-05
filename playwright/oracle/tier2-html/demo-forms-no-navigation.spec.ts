/**
 * ORACLE: tier 2 (HTML) -- a demo's submit never navigates or reloads the page.
 *
 * Source: WHATWG HTML Living Standard, the form submission algorithm
 *   https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-submission-algorithm
 * Activating a submit button (or implicit submission: Enter in a text field) fires `submit` and, unless that
 * event is cancelled, NAVIGATES to the form's action URL with the entry list as the query string -- a full
 * page load. The algorithm's `method=dialog` branch ("If form does not have an ancestor dialog element, then
 * return") is the one case that never navigates, and is what `DemoForm` relies on before hydration.
 *
 * WHAT THIS GUARDS (owner report 2026-10-05: "submit buttons in the demo / docs pages should not trigger /
 * reload the page"): the card's login form was a bare `<form>` with no `onsubmit`, so "Login" and Enter in its
 * email field reloaded `/component/card/` as `/component/card/?`. Dioxus does not prevent that by itself --
 * dioxus-web-0.7.9/src/dom.rs:113-116 calls `preventDefault` only when a Rust handler cleared
 * `Event::default_action_enabled()` -- and, independent of any handler, the SSG pages are plain HTML until the
 * wasm bundle has hydrated them. The construction is `DemoForm` (preview/src/components/form/component.rs),
 * enforced statically by scripts/check-demo-forms.sh; this is the behavioural half.
 *
 * ORACLE, by property rather than by instance: for EVERY route the preview serves (the component catalog read
 * from `CATALOG`/`examples!` in preview/src/components/mod.rs, plus the app's own pages, so a new component is
 * covered the day it lands), every submit control that is on screen is clicked and Enter is pressed in the
 * first text field of every form; nothing may navigate. "Navigate" is any of: a main-frame navigation
 * request, a `framenavigated`, a URL change, or a JS marker set on `window` before the action being gone
 * afterwards (a reload).
 *
 *   1. (hydrated, every route) as above, on a hydrated page. Routes are entered by clicking the docs sidebar's
 *      own links (a real user path, and ~50x cheaper than a fresh wasm boot per route); the app's own pages and
 *      the sidebar block demos are loaded fresh. `novalidate` is forced on each form first, so constraint
 *      validation cannot hide a submission that would have navigated. The dashboard's compose modal is opened
 *      first -- its form is not in the DOM until then.
 *   2. (before hydration) the SERVED markup of every route that has a `<form`, loaded with scripting OFF and,
 *      separately, with the wasm bundle blocked (JS on, no Dioxus listeners): the same clicks and Enter
 *      presses must not navigate. Needs prerendered HTML, so it skips on a client-rendered `dx serve` and is
 *      meant for the SSG build (playwright/ssg.local.config.ts; `SSG_SITE_DIR` is not needed, the routes come
 *      from the catalog). This is the half no Rust handler can ever provide.
 *   3. (calibration) the detector itself: a raw `<form>` (no `method=dialog`, no handler) DOES navigate in the
 *      same harness, scripting on and off -- so a green above is not the detector failing to see -- and a raw
 *      `<form method="dialog">` outside a `<dialog>` does NOT, which is the platform rule `DemoForm` builds on.
 *
 * Run against a dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts oracle/tier2-html/demo-forms-no-navigation.spec.ts
 */
import { test, expect } from "../../fixtures";
import type { Locator, Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "../../base-url";
import { gotoHydrated } from "../../hydration";

// --- the routes, read from Rust ------------------------------------------------------------------
const MOD_RS = fs.readFileSync(path.join(__dirname, "..", "..", "..", "preview", "src", "components", "mod.rs"), "utf8");
const CATALOG_BLOCK = MOD_RS.split("pub const CATALOG")[1]?.split("\n];")[0] ?? "";
const COMPONENTS = [...CATALOG_BLOCK.matchAll(/^\s*\("([a-z0-9_]+)", Origin::(?:Shadcn|Extra)\),/gm)].map((m) => m[1]);
/** `name(block)[variant, ...]` rows of `examples!`: rendered at `/component/block/<name>/<variant>/`. */
const BLOCKS = [...(MOD_RS.split("examples!(\n")[1] ?? "").matchAll(/^\s*([a-z0-9_]+)\(block\)(?:\[([^\]]*)\])?/gm)].flatMap((m) =>
  ["main", ...(m[2] ?? "").split(",").map((v) => v.trim()).filter(Boolean)].map((variant) => `/component/block/${m[1]}/${variant}/`),
);
/** The app's own pages: not under `/component/<name>/`, but several embed demos (`/` and `/demos` render the galleries). */
const APP_PAGES = ["/", "/demos", "/docs", "/charts/", "/dashboard/email-client"];
const COMPONENT_ROUTES = COMPONENTS.map((name) => `/component/${name}/`);
const ALL_ROUTES = [...APP_PAGES, ...BLOCKS, ...COMPONENT_ROUTES];

/** Controls that submit: a submit/image button or a default-type `<button>`, inside a form or tied to one by `form=`. */
const SUBMITTERS =
  'form :is(button:not([type]), button[type="submit"], input[type="submit"], input[type="image"]):not([disabled]), ' +
  ':is(button:not([type]), button[type="submit"]):not([disabled])[form]';
/** Fields whose Enter key triggers implicit submission (searched inside one form). */
const TEXT_FIELD =
  "input:is(:not([type]), [type=text], [type=email], [type=password], [type=search], [type=tel], [type=url], [type=number]):not([disabled]):not([readonly])";

/** Routes that only have a form after something is opened. */
const REVEAL: Record<string, (page: Page) => Promise<void>> = {
  "/dashboard/email-client": async (page) => {
    await page.getByRole("button", { name: /compose|new message/i }).first().click();
    await expect(page.locator("#ec-compose-to")).toBeVisible();
  },
};

// --- the detector --------------------------------------------------------------------------------
type Tracker = { navRequests: string[]; frameNavs: string[]; reset(): void };

/** Records main-frame navigations from now on. `reset()` after the page has settled, before each probe. */
function track(page: Page): Tracker {
  const t: Tracker = {
    navRequests: [],
    frameNavs: [],
    reset() {
      t.navRequests.length = 0;
      t.frameNavs.length = 0;
    },
  };
  page.on("request", (r) => {
    if (r.isNavigationRequest() && r.frame() === page.mainFrame()) t.navRequests.push(r.url());
  });
  page.on("framenavigated", (f) => {
    if (f === page.mainFrame()) t.frameNavs.push(f.url());
  });
  return t;
}

type Outcome = { where: string; control: string; via: "click" | "enter"; navigated: boolean; detail: string };

/** Run `act`, give a native navigation time to start, and say whether the page navigated. */
async function probe(page: Page, tracker: Tracker, where: string, control: string, via: "click" | "enter", act: () => Promise<void>, scripted: boolean): Promise<Outcome> {
  const before = page.url();
  const mark = `mark-${Math.random().toString(36).slice(2)}`;
  if (scripted) await page.evaluate((m) => ((window as unknown as { __demoFormMark: string }).__demoFormMark = m), mark);
  tracker.reset();
  await act();
  // A native submission starts a navigation request within the same task as the (uncancelled) `submit`
  // event; the settle only has to outlast that, and any client-side redirect the page might add.
  await page.waitForTimeout(400);
  const reasons: string[] = [];
  if (tracker.navRequests.length) reasons.push(`navigation request ${tracker.navRequests.join(", ")}`);
  if (tracker.frameNavs.length) reasons.push(`framenavigated ${tracker.frameNavs.join(", ")}`);
  if (page.url() !== before) reasons.push(`URL ${before} -> ${page.url()}`);
  if (scripted) {
    const after = await page
      .evaluate(() => (window as unknown as { __demoFormMark?: string }).__demoFormMark)
      .catch(() => "context destroyed");
    if (after !== mark) reasons.push(`window marker lost (the document was replaced): ${after}`);
  }
  return { where, control, via, navigated: reasons.length > 0, detail: reasons.join("; ") };
}

/** A short label for a control, built from locator reads only (they work with scripting disabled too). */
async function describe(loc: Locator): Promise<string> {
  const id = await loc.getAttribute("id", { timeout: 1_000 }).catch(() => null);
  const text = ((await loc.textContent({ timeout: 1_000 }).catch(() => "")) ?? "").trim().slice(0, 24);
  const formId = await loc.locator("xpath=ancestor::form[1]").getAttribute("id", { timeout: 500 }).catch(() => null);
  const tag = await loc.evaluate((el) => el.tagName.toLowerCase()).catch(() => "control");
  return `${tag}${id ? `#${id}` : ""}${text ? ` "${text}"` : ""}${formId ? ` in form#${formId}` : ""}`;
}

/** Click every visible submit control and press Enter in the first visible text field of every form. */
async function exerciseForms(page: Page, tracker: Tracker, where: string, scripted: boolean): Promise<{ outcomes: Outcome[]; forms: number; controls: number }> {
  const outcomes: Outcome[] = [];
  const forms = await page.locator("form").count();
  if (scripted) {
    // Validation must not be able to hide a submission that would have navigated.
    await page.evaluate(() => document.querySelectorAll("form").forEach((f) => (f.noValidate = true)));
  }
  const submitters = page.locator(SUBMITTERS);
  const n = await submitters.count();
  let controls = 0;
  for (let i = 0; i < n; i++) {
    const control = submitters.nth(i);
    if (!(await control.isVisible())) continue;
    controls++;
    const label = await describe(control);
    outcomes.push(await probe(page, tracker, where, label, "click", async () => {
      try {
        await control.click({ timeout: 5_000 });
      } catch {
        // Something (a toast, a sticky header) covers it: a script click runs the same activation behaviour.
        await control.evaluate((el) => (el as HTMLElement).click());
      }
    }, scripted));
    if (outcomes[outcomes.length - 1].navigated) return { outcomes, forms, controls }; // the document is gone: stop here
  }
  // Implicit submission: Enter in a text field. One field per form is enough to trigger it.
  for (let f = 0; f < forms; f++) {
    const fields = page.locator("form").nth(f).locator(TEXT_FIELD);
    let field: Locator | null = null;
    for (let k = 0; k < (await fields.count()); k++) {
      if (await fields.nth(k).isVisible()) {
        field = fields.nth(k);
        break;
      }
    }
    if (!field) continue;
    const target = field;
    outcomes.push(await probe(page, tracker, where, await describe(target), "enter", async () => {
      await target.focus();
      await page.keyboard.press("Enter");
    }, scripted));
    if (outcomes[outcomes.length - 1].navigated) break;
  }
  return { outcomes, forms, controls };
}

function report(failures: Outcome[]): string {
  return failures.map((f) => `  ${f.where}: ${f.via === "click" ? "clicking" : "Enter in"} ${f.control} navigated -- ${f.detail}`).join("\n");
}

/** Enter a route the way a visitor would: the sidebar link when there is one, otherwise a fresh load. */
async function enter(page: Page, route: string, first: boolean): Promise<void> {
  const component = route.match(/^\/component\/([a-z0-9_]+)\/$/)?.[1];
  const link = component ? page.locator(`a[href*="/component/${component}/"]`).first() : null;
  if (!first && link && (await link.count()) > 0) {
    await link.evaluate((a) => (a as HTMLAnchorElement).click());
    await page.waitForFunction((n) => location.pathname.includes(`/component/${n}/`), component!, { timeout: 30_000 });
    // If the link was not intercepted the browser followed it: that is a fresh load, so wait for hydration.
    await page.waitForSelector('html[data-hydrated="true"]', { state: "attached", timeout: 5 * 60 * 1000 });
    await page.locator(".dx-component-page-header").first().waitFor({ state: "attached", timeout: 30_000 }).catch(() => {});
  } else {
    await gotoHydrated(page, BASE_URL + route, { timeout: 20 * 60 * 1000 });
  }
  await page.waitForTimeout(500); // the demos mount in the render after the route change
  await REVEAL[route]?.(page);
  await page.waitForTimeout(300);
}

test.describe("demo forms never navigate", () => {
  test("the route list parses: catalog components, block demos and the app pages", () => {
    expect(COMPONENTS.length).toBeGreaterThan(60);
    expect(COMPONENTS).toEqual(expect.arrayContaining(["card", "form", "top_layer", "button"]));
    expect(BLOCKS.length).toBeGreaterThan(0);
    expect(new Set(ALL_ROUTES).size).toBe(ALL_ROUTES.length);
  });

  test("hydrated: every route's submit buttons and Enter-in-field never navigate or reload", async ({ page }) => {
    test.setTimeout(30 * 60 * 1000);
    const tracker = track(page);
    const failures: Outcome[] = [];
    const exercised: string[] = [];
    let forms = 0;
    let controls = 0;
    let first = true;
    for (const route of ALL_ROUTES) {
      await enter(page, route, first);
      first = false;
      tracker.reset();
      const r = await exerciseForms(page, tracker, route, true);
      forms += r.forms;
      controls += r.controls;
      if (r.forms > 0) exercised.push(`${route} (${r.forms} form${r.forms > 1 ? "s" : ""}, ${r.controls} submit control${r.controls === 1 ? "" : "s"})`);
      const bad = r.outcomes.filter((o) => o.navigated);
      failures.push(...bad);
      if (bad.length) {
        // The document was replaced: start the next route from a fresh, hydrated page.
        await gotoHydrated(page, BASE_URL + "/", { timeout: 20 * 60 * 1000 });
        first = true;
      }
    }
    console.log(`demo-forms: ${exercised.length} routes with forms; ${forms} forms, ${controls} visible submit controls exercised:\n  ${exercised.join("\n  ")}`);
    // Not vacuous: the preview does ship forms (the card login, the form fixture's two, the compose modal).
    expect(forms, "no <form> was found on any route -- the walk is not reaching the demos").toBeGreaterThanOrEqual(4);
    expect(controls).toBeGreaterThanOrEqual(4);
    expect(failures, `a submit navigated (reloaded) the page; render the form with DemoForm:\n${report(failures)}`).toEqual([]);
  });

  test.describe("before hydration (served markup)", () => {
    /** Routes whose served HTML already has a `<form`: empty on a client-rendered dev server. */
    let withForms: string[] = [];
    test.beforeAll(async ({ playwright }) => {
      const ctx = await playwright.request.newContext();
      try {
        for (const route of ALL_ROUTES) {
          const res = await ctx.get(BASE_URL + route, { timeout: 60_000 }).catch(() => null);
          if (res?.ok() && /<form[\s>]/i.test(await res.text())) withForms.push(route);
        }
      } finally {
        await ctx.dispose();
      }
    });

    for (const variant of ["scripting disabled", "wasm blocked"] as const) {
      test(`a submit on the served markup cannot navigate (${variant})`, async ({ browser }) => {
        test.setTimeout(10 * 60 * 1000);
        test.skip(withForms.length === 0, "the target serves no prerendered <form> (client-rendered dx serve): run this against an SSG build");
        const context = await browser.newContext(variant === "scripting disabled" ? { javaScriptEnabled: false } : {});
        try {
          // The same forms, minus the browser's own validation, which would otherwise swallow the submit.
          await context.route("**/*", async (route) => {
            const req = route.request();
            if (!req.isNavigationRequest() || req.method() !== "GET") return route.continue();
            const res = await route.fetch();
            if (!(res.headers()["content-type"] ?? "").includes("text/html")) return route.fulfill({ response: res });
            return route.fulfill({ response: res, body: (await res.text()).replace(/<form\b/gi, "<form novalidate") });
          });
          if (variant === "wasm blocked") await context.route("**/*.wasm", (r) => r.abort());
          const failures: Outcome[] = [];
          let controls = 0;
          for (const route of withForms) {
            const page = await context.newPage();
            const tracker = track(page);
            await page.goto(BASE_URL + route, { waitUntil: "domcontentloaded", timeout: 120_000 });
            tracker.reset();
            const r = await exerciseForms(page, tracker, route, variant !== "scripting disabled");
            controls += r.controls;
            failures.push(...r.outcomes.filter((o) => o.navigated));
            await page.close();
          }
          console.log(`demo-forms (${variant}): ${withForms.length} routes with served forms, ${controls} submit controls exercised`);
          expect(controls, "no visible submit control found on the served markup").toBeGreaterThan(0);
          expect(failures, `a submit on the prerendered page navigated before hydration; DemoForm's method="dialog" must be in the served <form>:\n${report(failures)}`).toEqual([]);
        } finally {
          await context.close();
        }
      });
    }
  });

  test.describe("calibration: the detector sees a native navigation, and the platform rule holds", () => {
    const REF = (attrs: string) => `<!doctype html><title>ref</title><form ${attrs}><input name="q" value="x"><button id="go" type="submit">go</button></form>`;
    const URL_OF = (name: string) => `${BASE_URL}/__demo-forms-calibration-${name}`;

    for (const scripting of [true, false]) {
      test(`CALIBRATION: a raw <form> navigates, a method="dialog" one outside a <dialog> does not (scripting ${scripting ? "on" : "off"})`, async ({ browser }) => {
        const context = await browser.newContext({ javaScriptEnabled: scripting });
        try {
          const results: Record<string, boolean> = {};
          for (const [name, attrs] of [["raw", ""], ["dialog", 'method="dialog"']] as const) {
            const page = await context.newPage();
            await page.route(`${URL_OF(name)}`, (r) => r.fulfill({ contentType: "text/html", body: REF(attrs) }));
            const tracker = track(page);
            await page.goto(URL_OF(name));
            tracker.reset();
            const out = await probe(page, tracker, name, "#go", "click", () => page.locator("#go").click(), scripting);
            results[name] = out.navigated;
            await page.close();
          }
          expect(results.raw, "the detector must see a native form navigate").toBe(true);
          expect(results.dialog, `a dialog-method form outside a <dialog> must never navigate (HTML form submission algorithm)`).toBe(false);
        } finally {
          await context.close();
        }
      });
    }
  });
});
