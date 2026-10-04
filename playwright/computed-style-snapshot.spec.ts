import { test } from "./fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "./base-url";

/**
 * Not a conformance test — a TOOL, run on demand to capture a computed-style
 * snapshot of every themed component page (and a few chrome routes), in BOTH
 * colour schemes.
 *
 * Purpose (docs/backlog.md row 31b, row 111): a literal/ramp -> token
 * migration is meant to be value-PRESERVING. The honest check is not "does it
 * still look plausible" but "did any computed value change at all". Capture
 * before, capture after, diff: an empty diff is the proof.
 *
 * USAGE (needs a server on PLAYWRIGHT_BASE_URL, default http://127.0.0.1:8080;
 * run from `playwright/`; see dev-docs/dev-loop.md "Per-lane servers"):
 *
 *   # 1. capture "before" (any isolated build of the parent commit)
 *   DX_SNAPSHOT=1 CSS_SNAPSHOT_OUT=/abs/before.json \
 *     npx playwright test computed-style-snapshot --project=chromium -g capture
 *   # 2. capture "after" (build of the change)
 *   DX_SNAPSHOT=1 CSS_SNAPSHOT_OUT=/abs/after.json \
 *     npx playwright test computed-style-snapshot --project=chromium -g capture
 *   # 3. diff. Either plain text (one `key: value` per line, stable order):
 *   diff /abs/before.json /abs/after.json          # empty output == proof
 *   #    or the built-in compare, which names mode/page/element/property and
 *   #    FAILS on any difference (full report: <after>.diff.txt):
 *   CSS_SNAPSHOT_A=/abs/before.json CSS_SNAPSHOT_B=/abs/after.json \
 *     npx playwright test computed-style-snapshot --project=chromium -g compare
 *
 * Skipped unless DX_SNAPSHOT is set (capture) or CSS_SNAPSHOT_A + _B are set
 * (compare), so it never runs as part of the suite.
 *
 * Env:
 *   CSS_SNAPSHOT_OUT   output path (alias: DX_SNAPSHOT_OUT; default /tmp/dx-computed.json)
 *   DX_SNAPSHOT_CHROME=0   skip the non-component chrome routes (home, docs, ...)
 *   DX_SNAPSHOT_FOCUS=0    skip the `:focus-visible` pass
 *
 * OUTPUT FORMAT (version 2; NOT compatible with the pre-2026-10-03 files, which
 * were `{ page: [records] }` for one unrecorded scheme):
 *
 *   { "meta": { format, modes, properties, focusProperties, ... },
 *     "light": { "<page>": { "base": [..], "pseudo": [..], "focus": [..] }, ... },
 *     "dark":  { same } }
 *
 * `<page>` is a component name, `route:<path>` for chrome routes, with
 * `#frame<k>` appended for the k-th child iframe of that page. Pages are sorted,
 * property keys are sorted, and there is no timestamp, so two captures of an
 * unchanged build are byte-identical (animations/transitions are settled, see
 * `settleAndCapture`).
 *
 * Colour schemes: the site switches on `html[data-theme]` (cookie `dx_theme`,
 * applied by theme.rs `theme_seed`) and, absent that, on `prefers-color-scheme`
 * (dx-components-theme.css). Each mode sets BOTH — the cookie and
 * `page.emulateMedia({ colorScheme })` — and asserts both took effect.
 *
 * NOT covered (needs forced pseudo-state via CDP, not cheap/deterministic):
 * `:hover`, `:active`, `::selection`, `::-webkit-scrollbar*`. Open/checked
 * states are covered only as far as the demo renders them in its initial state.
 */

const OUT =
  process.env.CSS_SNAPSHOT_OUT ?? process.env.DX_SNAPSHOT_OUT ?? "/tmp/dx-computed.json";
const CMP_A = process.env.CSS_SNAPSHOT_A;
const CMP_B = process.env.CSS_SNAPSHOT_B;
const DO_CHROME = process.env.DX_SNAPSHOT_CHROME !== "0";
const DO_FOCUS = process.env.DX_SNAPSHOT_FOCUS !== "0";
/** Comma list of component names to capture (debugging aid; default all). */
const ONLY = process.env.DX_SNAPSHOT_ONLY?.split(",").filter(Boolean);

const MODES = ["light", "dark"] as const;
type Mode = (typeof MODES)[number];

/** Non-component routes whose chrome (`main.css`, `hero.css`, dashboard) the
 *  `dx-` selector alone misses. Captured with every classed element. */
const CHROME_ROUTES = ["/", "/docs", "/demos", "/charts/", "/dashboard/email-client"];

/** Colour-bearing computed properties (also the set re-read in `:focus-visible`
 *  and for pseudo-elements). Sorted. */
const COLOR_PROPERTIES = [
  "-webkit-tap-highlight-color",
  "-webkit-text-fill-color",
  "-webkit-text-stroke-color",
  "accent-color",
  "background-color",
  "background-image",
  "backdrop-filter",
  "border-bottom-color",
  "border-color",
  "border-image-source",
  "border-left-color",
  "border-right-color",
  "border-top-color",
  "box-shadow",
  "caret-color",
  "color",
  "color-scheme",
  "column-rule-color",
  "fill",
  "fill-opacity",
  "filter",
  "flood-color",
  "lighting-color",
  "opacity",
  "outline-color",
  "outline-offset",
  "outline-style",
  "outline-width",
  "scrollbar-color",
  "stop-color",
  "stroke",
  "stroke-opacity",
  "stroke-width",
  "text-decoration-color",
  "text-decoration-line",
  "text-emphasis-color",
  "text-shadow",
];

/** Geometry / motion properties (the original row 31b set). */
const LAYOUT_PROPERTIES = [
  "border-radius", "border-width",
  "font-size", "font-weight", "gap", "height", "line-height",
  "margin-bottom", "margin-left", "margin-right", "margin-top",
  "padding-bottom", "padding-left", "padding-right", "padding-top",
  "transition-duration", "transition-property", "transition-timing-function",
  "width", "z-index",
];

const PROPERTIES = Array.from(new Set([...COLOR_PROPERTIES, ...LAYOUT_PROPERTIES])).sort();
const FOCUS_PROPERTIES = COLOR_PROPERTIES;

type CaptureArgs = {
  selector: string;
  props: string[];
  focusProps: string[];
  mode: string;
  doFocus: boolean;
};

type Rec = Record<string, string | number>;
type Section = { base: Rec[]; pseudo: Rec[]; focus: Rec[] };

/**
 * Runs IN the page (serialised by Playwright: keep it self-contained, no
 * closure over module scope, no TS-emitted helpers such as async/await).
 * Everything after the fonts promise is synchronous, so nothing re-renders
 * between reading the base state and the focus pass.
 */
function settleAndCapture(args: CaptureArgs) {
  return document.fonts.ready.then(() => {
    const w = window as any;
    const settle = () => {
      // Make animated/transitioning values deterministic: finish transitions
      // and finite animations at their end state; park infinite animations on
      // their first frame (e.g. dx-progress-indicator re-sampled 34/54/32/54px
      // on one unchanged build before this).
      for (const a of document.getAnimations()) {
        try {
          const isTransition = w.CSSTransition && a instanceof w.CSSTransition;
          const timing = a.effect ? a.effect.getComputedTiming() : null;
          if (!isTransition && timing && timing.iterations === Infinity) {
            a.pause();
            a.currentTime = 0;
          } else {
            a.finish();
          }
        } catch (_) {
          /* ignore: nothing else to do for an animation we cannot settle */
        }
      }
    };

    const root = document.documentElement;
    const themeAttr = root.getAttribute("data-theme");
    const mediaDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    let forced = false;
    if (themeAttr !== args.mode) {
      root.setAttribute("data-theme", args.mode);
      forced = true;
    }
    settle();

    const sorted = (cs: CSSStyleDeclaration, el: Element, props: string[], extra: Rec) => {
      const rec: Rec = { ...extra, tag: el.tagName.toLowerCase(), cls: Array.from(el.classList).sort().join(" ") };
      for (const p of props) rec[p] = cs.getPropertyValue(p);
      return rec;
    };

    const els = Array.from(document.querySelectorAll(args.selector)).filter(
      (e) => !["SCRIPT", "STYLE", "LINK", "META", "TITLE", "HEAD", "NOSCRIPT"].includes(e.tagName.toUpperCase()),
    );
    const base: Rec[] = [];
    const pseudo: Rec[] = [];
    const focus: Rec[] = [];

    els.forEach((el, n) => {
      const cs = getComputedStyle(el);
      // `n` = document-order index; tag + class list are the stable identity.
      base.push(sorted(cs, el, args.props, { n }));

      for (const ps of ["::before", "::after"]) {
        const pcs = getComputedStyle(el, ps);
        const content = pcs.content;
        if (content && content !== "none" && content !== "normal") {
          pseudo.push(sorted(pcs, el, args.props, { n, pseudo: ps }));
        }
      }
      const tag = el.tagName.toLowerCase();
      if (tag === "input" || tag === "textarea") {
        pseudo.push(sorted(getComputedStyle(el, "::placeholder"), el, args.focusProps, { n, pseudo: "::placeholder" }));
      }
      if (cs.display === "list-item") {
        pseudo.push(sorted(getComputedStyle(el, "::marker"), el, args.focusProps, { n, pseudo: "::marker" }));
      }
    });

    const finishTransitions = () => {
      for (const a of document.getAnimations()) {
        try {
          if (w.CSSTransition && a instanceof w.CSSTransition) a.finish();
        } catch (_) {
          /* ignore */
        }
      }
    };
    const result = { themeAttr, mediaDark, forced, section: { base, pseudo, focus } as Section };
    if (!args.doFocus) return result;

    // The focus pass YIELDS to the event loop between elements. Run as one
    // synchronous loop it hung the renderer at 100% CPU on `calendar` (focus
    // handlers of ~200 grid cells in a row; reproduced with a plain
    // focus()/blur() loop, no snapshot code involved, and never when each
    // focus() is its own task). The base/pseudo sections above are already
    // read, so yielding here cannot change them.
    const focusable = 'a[href], button, input, textarea, select, summary, [tabindex], [contenteditable="true"]';
    const targets: [Element, number][] = [];
    els.forEach((el, n) => {
      if (el.matches(focusable) && !el.matches(":disabled")) targets.push([el, n]);
    });
    const step = (k: number): Promise<typeof result> => {
      if (k >= targets.length) return Promise.resolve(result);
      const [el, n] = targets[k];
      if (!el.isConnected) return step(k + 1);
      const h = el as HTMLElement;
      h.focus({ preventScroll: true, focusVisible: true } as FocusOptions);
      if (document.activeElement === h) {
        finishTransitions();
        focus.push(sorted(getComputedStyle(el), el, args.focusProps, { n, state: h.matches(":focus-visible") ? "focus-visible" : "focus" }));
        h.blur();
        finishTransitions();
      }
      return new Promise<void>((r) => setTimeout(r, 0)).then(() => step(k + 1));
    };
    return step(0);
  });
}

/** Sort a record's keys: identity first (n, pseudo, state, tag, cls), then properties A-Z. */
function stable(rec: Rec): Rec {
  const head = ["n", "pseudo", "state", "tag", "cls"];
  const out: Rec = {};
  for (const k of head) if (k in rec) out[k] = rec[k];
  for (const k of Object.keys(rec).sort()) if (!(k in out)) out[k] = rec[k];
  return out;
}

async function capturePage(
  page: Page,
  url: string,
  key: string,
  mode: Mode,
  selector: string,
  into: Record<string, Section>,
) {
  if (process.env.DX_SNAPSHOT_VERBOSE) console.log(`[snapshot] ${mode} ${key}`);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  // Let the wasm client render, its stylesheets attach and theme_seed run.
  await page.waitForTimeout(1500);
  const args: CaptureArgs = {
    selector,
    props: PROPERTIES,
    focusProps: FOCUS_PROPERTIES,
    mode,
    doFocus: DO_FOCUS,
  };
  const frames = [page.mainFrame(), ...page.mainFrame().childFrames()];
  for (let k = 0; k < frames.length; k++) {
    const frame = frames[k];
    if (k > 0 && (frame.url() === "about:blank" || frame.url() === "")) continue;
    const r = await frame.evaluate(settleAndCapture, args);
    if (r.mediaDark !== (mode === "dark")) {
      throw new Error(`${key}: prefers-color-scheme did not follow emulateMedia (${mode})`);
    }
    if (r.forced) {
      console.warn(`[snapshot] ${mode} ${key}${k ? `#frame${k}` : ""}: data-theme was ${r.themeAttr}, forced`);
    }
    const sec = r.section;
    into[k === 0 ? key : `${key}#frame${k}`] = {
      base: sec.base.map(stable),
      pseudo: sec.pseudo.map(stable),
      focus: sec.focus.map(stable),
    };
  }
}

test.describe("computed style snapshot", () => {
  test("capture computed styles for every themed component page", async ({ page, context }) => {
    test.skip(!process.env.DX_SNAPSHOT, "snapshot tool; set DX_SNAPSHOT=1 to run");
    test.setTimeout(20 * 60 * 1000);

    const base = path.join(__dirname, "../preview/src/components");
    const names = fs
      .readdirSync(base)
      .filter((n) => fs.existsSync(path.join(base, n, "style.css")))
      .filter((n) => !ONLY || ONLY.includes(n))
      .sort();
    const chrome = DO_CHROME ? [...CHROME_ROUTES].sort() : [];

    const all: Record<string, unknown> = {
      meta: {
        format: 2,
        modes: [...MODES],
        pages: [...names, ...chrome.map((r) => `route:${r}`)],
        componentSelector: '[class*="dx-"], [class*="dx-"] svg, [class*="dx-"] svg *',
        chromeSelector: "body [class], body svg, body svg *",
        properties: PROPERTIES,
        focusProperties: DO_FOCUS ? FOCUS_PROPERTIES : [],
      },
    };

    // Two demos advance their own state from a `setInterval(() => dioxus.send(...))`
    // ticker (progress: `Math.random() * 30` every 1s; the home music player:
    // elapsed wall-clock every 100ms), so `.dx-progress-indicator` (0/18/26px) and
    // the player's `.dx-slider-range` (116.891 vs 116.922px) differed between two
    // captures of an unchanged build. Their value depends on how many ticks ran
    // before the read, which no wait can make equal. Freeze every such ticker (the
    // demos stay at their initial value); no other preview code uses setInterval.
    await page.addInitScript(() => {
      const realSetInterval = window.setInterval.bind(window);
      (window as any).setInterval = (handler: any, timeout?: number, ...rest: any[]) =>
        typeof handler === "function" && String(handler).includes("dioxus.send(")
          ? 0
          : realSetInterval(handler, timeout, ...rest);
    });

    const host = new URL(BASE_URL).hostname;
    let total = 0;
    for (const mode of MODES) {
      // The theme is cookie-driven (theme.rs); the cookie path must be `/`
      // whatever path prefix BASE_URL carries.
      await context.clearCookies({ name: "dx_theme" });
      await context.addCookies([{ name: "dx_theme", value: mode, domain: host, path: "/" }]);
      await page.emulateMedia({ colorScheme: mode });

      const pages: Record<string, Section> = {};
      for (const name of names) {
        await capturePage(
          page,
          `${BASE_URL}/component/?name=${name}&`,
          name,
          mode,
          '[class*="dx-"], [class*="dx-"] svg, [class*="dx-"] svg *',
          pages,
        );
      }
      for (const route of chrome) {
        await capturePage(page, `${BASE_URL}${route}`, `route:${route}`, mode, "body [class], body svg, body svg *", pages);
      }
      // Stable page order regardless of insertion order.
      const sortedPages: Record<string, Section> = {};
      for (const k of Object.keys(pages).sort()) sortedPages[k] = pages[k];
      all[mode] = sortedPages;
      total += Object.values(sortedPages).reduce((n, s) => n + s.base.length, 0);
    }

    fs.writeFileSync(OUT, JSON.stringify(all, null, 1));
    console.log(`wrote ${OUT} (${names.length} component pages + ${chrome.length} routes, ${MODES.length} modes, ${total} base records)`);
  });

  test("compare two snapshots (empty diff == value-preserving)", async () => {
    test.skip(!(CMP_A && CMP_B), "set CSS_SNAPSHOT_A and CSS_SNAPSHOT_B to compare");
    const a = JSON.parse(fs.readFileSync(CMP_A!, "utf8"));
    const b = JSON.parse(fs.readFileSync(CMP_B!, "utf8"));
    const lines: string[] = [];

    if (JSON.stringify(a.meta?.properties) !== JSON.stringify(b.meta?.properties)) {
      lines.push("meta.properties differ: the two files captured different property sets");
    }

    // Identity of a record: tag|cls|pseudo|state + its occurrence count, so an
    // unrelated inserted element does not shift every later comparison.
    const index = (recs: Rec[]) => {
      const seen = new Map<string, number>();
      const m = new Map<string, Rec>();
      for (const r of recs) {
        const id = `${r.tag}.${stripHash(String(r.cls))}${r.pseudo ?? ""}${r.state ? `:${r.state}` : ""}`;
        const k = (seen.get(id) ?? 0) + 1;
        seen.set(id, k);
        m.set(`${id}#${k}`, r);
      }
      return m;
    };
    const IDENT = new Set(["n", "tag", "cls", "pseudo", "state"]);
    // Build identity, not style: `#[css_module]` appends a hash of the source path
    // to every class (`dx-top-layer-hint-e446a291`, different per worktree), and
    // chart SVG ids carry a build-specific counter (`dxc-220-gradient-mobile`).
    // Two captures of the same sources can differ in both, so strip them.
    const stripHash = (cls: string) => cls.replace(/(\bdx-[a-z0-9-]*?)-[0-9a-f]{8}\b/g, "$1");
    const norm = (v: unknown) => String(v).replace(/\bdxc-\d+-/g, "dxc-#-");

    for (const mode of MODES) {
      const pa: Record<string, Section> = a[mode] ?? {};
      const pb: Record<string, Section> = b[mode] ?? {};
      for (const pg of Array.from(new Set([...Object.keys(pa), ...Object.keys(pb)])).sort()) {
        if (!pa[pg] || !pb[pg]) {
          lines.push(`${mode} ${pg}: page only in ${pa[pg] ? "A" : "B"}`);
          continue;
        }
        for (const sec of ["base", "pseudo", "focus"] as const) {
          const ma = index(pa[pg][sec] ?? []);
          const mb = index(pb[pg][sec] ?? []);
          for (const id of Array.from(new Set([...ma.keys(), ...mb.keys()])).sort()) {
            const ra = ma.get(id);
            const rb = mb.get(id);
            if (!ra || !rb) {
              lines.push(`${mode} ${pg} ${sec} ${id}: only in ${ra ? "A" : "B"}`);
              continue;
            }
            for (const p of Object.keys(ra).filter((k) => !IDENT.has(k)).sort()) {
              if (norm(ra[p]) !== norm(rb[p])) lines.push(`${mode} ${pg} ${sec} ${id} ${p}: ${JSON.stringify(ra[p])} -> ${JSON.stringify(rb[p])}`);
            }
          }
        }
      }
    }

    const report = `${CMP_B}.diff.txt`;
    fs.writeFileSync(report, lines.join("\n") + (lines.length ? "\n" : ""));
    console.log(`${lines.length} differing value(s); full report: ${report}`);
    for (const l of lines.slice(0, 200)) console.log(l);
    if (lines.length) throw new Error(`${lines.length} computed-style difference(s) between A and B (see ${report})`);
  });
});
