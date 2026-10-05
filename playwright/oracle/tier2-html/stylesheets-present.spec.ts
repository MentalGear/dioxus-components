/**
 * Component stylesheets reach the page -- tier 2 (WHATWG HTML / CSSOM).
 *
 * Rule source: HTML Living Standard, "the link element" (`rel="stylesheet"`) and CSSOM's
 * `document.styleSheets` -- a stylesheet only participates in the cascade once its `<link>` is in the
 * document and loaded, and a rule that is not in `document.styleSheets` styles nothing.
 *
 * WHAT THIS GUARDS (found by execution, 2026-10-04): every component wrapper in `preview/src/components`
 * ships its own CSS with a `document::Link { rel: "stylesheet", href: asset!(..) }`. Dioxus 0.7.9
 * (`dioxus-document/src/elements/link.rs:123-137`) records the href in a de-duplication set while the
 * component RENDERS, but the web document only appends the `<link>` from an effect queued on the Link's
 * own scope (`dioxus-web/src/document.rs:160-164`) -- and `Runtime::remove_scope`
 * (`dioxus-core/src/runtime.rs:190-193`) drops a scope's queued effects. So when a subtree is torn down
 * in the same turn it first rendered (the legacy `/component/?name=X` shell renders `Navbar`, then its
 * own `use_effect` redirects to `/component/X/`), its links are NEVER inserted, yet the set still says
 * "present" -- and every later `Popover`/`Button`/`LanguageSelect` on that page is silently skipped. The
 * page then renders the component with no CSS (the theme picker's popover panel computed `border 3px /
 * radius 0` instead of `1px / 10px`; `language-select-*.css` never loaded). Row 46's lost
 * `main.css`/`dx-components-theme.css` after a redirect was the same defect.
 *
 * THE ORACLE, by property rather than by instance: on a fresh load of a sample of every route type, and
 * again with the theme picker's popover open (it mounts `Popover`/`Button` inside the header), every
 * `dx-*` class that is rendered must have its component stylesheet applied. "Its stylesheet" is read
 * from the CSS SOURCES (`preview/src/**` and `preview/assets/*.css`), not restated here, so a new
 * component is covered the day it lands: a sheet counts as applied when a class ONLY that sheet defines
 * (its fingerprint) is among the rules of `document.styleSheets`. The built asset URLs
 * (`style-dxh<hash>.css`) carry no component name, so the match is by the rules a sheet contains, not by
 * href. The four always-linked global sheets (`GlobalHead`) are not component sheets: a class they share
 * with a component sheet (`.dx-popover-content.dx-theme-picker` in `main.css`) must not mask a missing
 * component sheet, so they are left out of the ownership map.
 *
 * Run against a dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=baseline.local.config.ts oracle/tier2-html/stylesheets-present.spec.ts
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "../../base-url";

const PREVIEW = path.join(__dirname, "..", "..", "..", "preview");

/** Sheets linked once by `GlobalHead` (plus the copy-out utilities): always present, never component-owned. */
const GLOBAL_SHEETS = new Set([
  "main.css",
  "dx-components-theme.css",
  "dx-effects.css",
  "theme-presets.css",
  "dx-utilities.css",
]);

/** Every `.css` file a component or the shell links with `document::Link`, relative to `preview/`. */
function linkedSheets(): string[] {
  const out: string[] = [];
  for (const dir of ["src", "assets"]) {
    for (const rel of fs.readdirSync(path.join(PREVIEW, dir), { recursive: true }) as string[]) {
      if (!rel.endsWith(".css")) continue;
      if (dir === "assets" && GLOBAL_SHEETS.has(rel)) continue;
      out.push(path.join(dir, rel));
    }
  }
  return out;
}

/** The comma-separated selectors of every style rule in a stylesheet's source (at-rule preludes skipped). */
function selectorsOf(file: string): string[] {
  const css = fs.readFileSync(path.join(PREVIEW, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const out: string[] = [];
  for (const prelude of css.matchAll(/([^{}]+)\{/g)) {
    if (prelude[1].trim().startsWith("@")) continue;
    for (const selector of prelude[1].split(",")) out.push(selector.trim());
  }
  return out;
}

type Fingerprint = {
  /** Classes mentioned by this sheet's selectors and by no other sheet's: any one applied proves the sheet loaded. */
  marks: string[];
  /** Classes that LEAD a selector here and lead none elsewhere (`.dx-button[..]`, where `dx-button` is also a descendant elsewhere). */
  leads: string[];
};

/**
 * `defs`: class -> the component sheets with a selector LEADING with it (the sheet that styles it). `fingerprints`: how to tell, from the
 * rules of `document.styleSheets`, that each component sheet is among them.
 */
function sheetMaps(): { defs: Record<string, string[]>; fingerprints: Record<string, Fingerprint> } {
  const files = [...linkedSheets()];
  const global = [...GLOBAL_SHEETS].map((g) => path.join("assets", g)).filter((f) => fs.existsSync(path.join(PREVIEW, f)));
  const mentions = new Map<string, Set<string>>();
  const leaders = new Map<string, Set<string>>();
  const per = new Map<string, { all: Set<string>; lead: Set<string> }>();
  for (const file of [...files, ...global]) {
    const all = new Set<string>();
    const lead = new Set<string>();
    for (const selector of selectorsOf(file)) {
      [...selector.matchAll(/\.(dx-[A-Za-z0-9_-]+)/g)].forEach((m) => all.add(m[1]));
      const first = selector.match(/\.([A-Za-z_][A-Za-z0-9_-]*)/)?.[1];
      if (first?.startsWith("dx-")) lead.add(first);
    }
    per.set(file, { all, lead });
    for (const c of all) mentions.set(c, (mentions.get(c) ?? new Set()).add(file));
    for (const c of lead) leaders.set(c, (leaders.get(c) ?? new Set()).add(file));
  }
  const defs: Record<string, string[]> = {};
  const fingerprints: Record<string, Fingerprint> = {};
  for (const file of files) {
    const { all, lead } = per.get(file)!;
    fingerprints[file] = {
      marks: [...all].filter((c) => mentions.get(c)!.size === 1),
      leads: [...lead].filter((c) => leaders.get(c)!.size === 1),
    };
    // Ownership is by LEADING class (the selector's first class, which must be `dx-*`):
    // `.ec-list-toolbar .dx-select-expand-icon` styles a descendant for its own sheet, it does not
    // make that sheet the owner of `dx-select-expand-icon`.
    for (const c of lead) (defs[c] ??= []).push(file);
  }
  return { defs, fingerprints };
}

const { defs: DEFS, fingerprints: FINGERPRINTS } = sheetMaps();

/**
 * The rendered `dx-*` classes none of whose component sheets is among the page's applied stylesheets,
 * keyed by the sheet source that should have supplied them.
 */
async function missingStylesheets(page: Page): Promise<Record<string, string[]>> {
  return page.evaluate(
    ({ defs, fingerprints }) => {
      const texts: string[] = [];
      const leading = new Set<string>();
      const walk = (rules: CSSRuleList) => {
        for (const rule of rules) {
          if (rule instanceof CSSStyleRule) {
            for (const selector of rule.selectorText.split(",")) {
              const first = selector.match(/\.([A-Za-z_][A-Za-z0-9_-]*)/)?.[1];
              if (first?.startsWith("dx-")) leading.add(first);
            }
          }
          const nested = (rule as CSSGroupingRule).cssRules;
          if (nested) walk(nested);
        }
      };
      for (const sheet of document.styleSheets) {
        try {
          texts.push([...sheet.cssRules].map((r) => r.cssText).join("\n"));
          walk(sheet.cssRules);
        } catch {
          // A cross-origin sheet (the Geist font CSS) hides its rules; it defines no `dx-*` class.
        }
      }
      const applied = texts.join("\n");
      const mentioned = (cls: string) =>
        new RegExp(`\\.${cls.replace(/[-\\^$*+?.()|[\]{}]/g, "\\$&")}(?![\\w-])`).test(applied);
      const loaded = new Map<string, boolean>();
      // A sheet with no fingerprint cannot be told apart from the others: assume it is there.
      const isLoaded = (file: string) => {
        if (!loaded.has(file)) {
          const { marks, leads } = fingerprints[file] ?? { marks: [], leads: [] };
          loaded.set(file, marks.length + leads.length === 0 || marks.some(mentioned) || leads.some((c) => leading.has(c)));
        }
        return loaded.get(file)!;
      };
      const missing: Record<string, string[]> = {};
      const seen = new Set<string>();
      for (const el of document.querySelectorAll("[class]")) {
        for (const cls of el.classList) {
          if (seen.has(cls) || !defs[cls]) continue;
          seen.add(cls);
          if (!defs[cls].some(isLoaded)) (missing[defs[cls][0]] ??= []).push(cls);
        }
      }
      return missing;
    },
    { defs: DEFS, fingerprints: FINGERPRINTS },
  );
}

/** Wait for the app to have mounted (CSR) or hydrated (SSG), and for a legacy route's redirect to land. */
async function settle(page: Page, settledPath?: RegExp) {
  await page.waitForSelector('html[data-hydrated="true"]', { state: "attached", timeout: 60_000 });
  if (settledPath) await page.waitForURL(settledPath, { timeout: 30_000 });
  await page.waitForSelector("nav.dx-preview-navbar, main", { state: "attached", timeout: 30_000 });
}

const ROUTES: { url: string; settled?: RegExp }[] = [
  { url: "/" },
  { url: "/docs" },
  { url: "/demos" },
  { url: "/charts/" },
  { url: "/charts/line/" },
  // The canonical path routes, one per kind of page.
  { url: "/component/popover/" },
  { url: "/component/button/" },
  // The legacy query-string routes: they render the shell, then redirect, which is the teardown that
  // used to lose the header's stylesheets.
  { url: "/component/?name=popover&", settled: /\/component\/popover\// },
  { url: "/component/?name=select&", settled: /\/component\/select\// },
  // The block demos render inside the docs page's iframe, outside the docs layout: their own document,
  // their own de-duplication set, and their own legacy redirect shell.
  { url: "/component/block/sidebar/main/" },
  { url: "/component/block/?name=sidebar", settled: /\/component\/block\/sidebar\/main\// },
  { url: "/dashboard/email-client" },
];

test("the class -> stylesheet map is not empty (the sources were found)", () => {
  // A wrong PREVIEW path would make every route test below vacuously green.
  expect(Object.keys(DEFS).length).toBeGreaterThan(200);
  expect(DEFS["dx-popover-content"], "popover's panel class is owned by popover/style.css").toEqual([
    path.join("src", "components", "popover", "style.css"),
  ]);
  expect(DEFS["dx-language-select"], "the header language select is owned by assets/language-select.css").toEqual([
    path.join("assets", "language-select.css"),
  ]);
  for (const sheet of ["src/components/popover/style.css", "src/components/button/style.css", "assets/language-select.css"]) {
    const { marks, leads } = FINGERPRINTS[path.normalize(sheet)];
    expect(marks.length + leads.length, `${sheet} can be told apart from every other sheet`).toBeGreaterThan(0);
  }
});

for (const { url, settled } of ROUTES) {
  test(`every rendered component has its stylesheet on a fresh load of ${url}`, async ({ page }) => {
    await page.goto(`${BASE_URL}${url}`, { waitUntil: "domcontentloaded", timeout: 60_000 });
    await settle(page, settled);

    // Links are appended from effects and sheets load asynchronously: poll until settled, so only a
    // sheet that NEVER arrives fails (rather than one that is merely a frame late).
    await expect
      .poll(() => missingStylesheets(page), {
        message: `rendered dx-* classes whose stylesheet is not in document.styleSheets, by source file`,
        timeout: 10_000,
      })
      .toEqual({});

    // The header's theme picker mounts Popover + Button on demand: open it and check again, since a
    // component that first renders AFTER load is the one a de-duplicated link would have skipped.
    const trigger = page.getByRole("button", { name: "Theme", exact: true });
    if (await trigger.count()) {
      await trigger.click();
      await expect(page.locator(".dx-theme-picker")).toBeVisible();
      await expect
        .poll(() => missingStylesheets(page), {
          message: `with the theme picker open: rendered dx-* classes whose stylesheet is not applied`,
          timeout: 10_000,
        })
        .toEqual({});
    }
  });
}

test("the oracle is not vacuous: a component stylesheet missing from the page is reported", async ({ page }) => {
  // Without this, a mistake in the class -> sheet maps (wrong path, a fingerprint that matches
  // everywhere) would leave every route test above green whatever the page loaded. Take the header's
  // language select (it renders on every route) and drop ITS stylesheet, the way the lost insertion did.
  await page.goto(`${BASE_URL}/docs`, { waitUntil: "domcontentloaded", timeout: 60_000 });
  await settle(page);
  await expect.poll(() => missingStylesheets(page), { timeout: 10_000 }).toEqual({});
  const { marks } = FINGERPRINTS[path.join("assets", "language-select.css")];
  // Delete the RULES of that sheet (a whole class ONLY it mentions; `main.css` also mentions `.dx-language-select-value`),
  // wherever they are -- not the sheet's `<link>`: on a build with the site's one CSS bundle (scripts/ssg-css-bundle.mjs) the
  // language-select rules live in the same sheet as every other component's, and removing that sheet would drop them all.
  const removed = await page.evaluate((marks) => {
    const mentions = (selector: string) => marks.some((m) => new RegExp(`\\.${m}(?![\\w-])`).test(selector));
    let n = 0;
    const strip = (rules: CSSRuleList, remove: (i: number) => void) => {
      for (let i = rules.length - 1; i >= 0; i--) {
        const rule = rules[i];
        if (rule instanceof CSSStyleRule && mentions(rule.selectorText)) {
          remove(i);
          n++;
        } else if ((rule as CSSGroupingRule).cssRules) {
          strip((rule as CSSGroupingRule).cssRules, (j) => (rule as CSSGroupingRule).deleteRule(j));
        }
      }
    };
    for (const sheet of document.styleSheets) {
      try {
        strip(sheet.cssRules, (i) => sheet.deleteRule(i));
      } catch {
        // A cross-origin sheet (the Geist font CSS) hides its rules; it has no language-select rule.
      }
    }
    return n;
  }, marks);
  expect(removed, "found and deleted the language-select rules").toBeGreaterThan(0);
  const missing = await missingStylesheets(page);
  expect(Object.keys(missing)).toEqual([path.join("assets", "language-select.css")]);
  expect(missing[path.join("assets", "language-select.css")]).toContain("dx-language-select");
});

test("the theme picker's popover panel is styled on the legacy component route", async ({ page }) => {
  // The symptom the popover lane saw: on `/component/?name=...` the panel computed border 3px /
  // radius 0 (the UA `<dialog>`-ish default) instead of the themed 1px / 10px it has on `/`.
  const panel = async (url: string, settled?: RegExp) => {
    await page.goto(`${BASE_URL}${url}`, { waitUntil: "domcontentloaded", timeout: 60_000 });
    await settle(page, settled);
    await page.getByRole("button", { name: "Theme", exact: true }).click();
    const content = page.locator(".dx-theme-picker");
    await expect(content).toBeVisible();
    return content.evaluate((el) => {
      const cs = getComputedStyle(el);
      return { border: cs.borderTopWidth, radius: cs.borderTopLeftRadius };
    });
  };
  const home = await panel("/");
  const legacy = await panel("/component/?name=select&", /\/component\/select\//);
  expect(legacy).toEqual(home);
  expect(legacy.border).not.toBe("3px");
});
