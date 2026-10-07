#!/usr/bin/env node
// scripts/compare-ssg-styles.mjs -- do two SSG builds of the preview compute the same styles on every route?
//
//   node scripts/compare-ssg-styles.mjs <baseUrlA> <baseUrlB> [route-substring ...]
//
// Serve each `public/` dir statically (e.g. `python3 -m http.server 8192 -d A/public`, `... 8193 -d B/public`) and point this at
// both. It loads every component route (+ the home, docs, demos, charts, dashboard and a block-demo page) in each with
// JavaScript DISABLED (so there is no hydration noise: the cascade the static HTML + stylesheets give is what is compared),
// in light and dark, and compares the computed style of every element of `<body>` for the properties that decide how
// something looks and where it sits. Exit 1 if any route differs.
//
// What it is for: scripts/ssg-css-bundle.mjs replaces each route's own stylesheet order (render order) with ONE bundle order.
// Stylesheets of different components conflict at equal specificity (`.dx-label` vs `.dx-field-label`, `.dx-button` vs
// `.dx-sidebar-trigger`, ...), so a wrong order silently changes pixels. Compare a `--no-css-bundle` build (A) with the bundled
// build (B) whenever the bundler's ordering or the stylesheets change. Measured 2026-10-05 (release SSG of 001a5f0): 82 of 83 routes
// identical in both schemes; the one difference is the HOME page, where Field's label is 14px per the home gallery's own order
// (Label before Field) and 12.8px on the Field page and, with the bundle, everywhere -- a route-dependent cascade that already
// existed (`field/style.css`'s `.dx-field-label` and `label/style.css`'s `.dx-label` tie on specificity).
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const require = createRequire(path.join(root, "playwright", "package.json"));
const { chromium } = require("@playwright/test");

const [A, B, ...only] = process.argv.slice(2);
if (!A || !B) {
  console.error("usage: compare-ssg-styles.mjs <baseUrlA> <baseUrlB> [route-substring ...]");
  process.exit(2);
}
const PROPS = [
  "display", "position", "width", "height", "margin-top", "margin-bottom", "padding-top", "padding-left", "border-top-width",
  "border-top-left-radius", "font-size", "font-weight", "line-height", "color", "background-color", "opacity", "gap",
  "flex-direction", "align-items", "box-shadow", "font-family", "overflow-x", "visibility", "cursor", "border-top-color", "transform", "z-index",
];
const comps = fs
  .readdirSync(path.join(root, "preview", "src", "components"), { withFileTypes: true })
  .filter((d) => d.isDirectory() && fs.existsSync(path.join(root, "preview", "src", "components", d.name, "docs.md")))
  .map((d) => `/component/${d.name}/`);
const all = [...comps, "/", "/docs", "/demos", "/charts/", "/charts/line/", "/dashboard/email-client", "/component/block/sidebar/main/"];
const routes = only.length ? all.filter((r) => only.some((o) => r.includes(o))) : all;

const sandbox = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const browser = await chromium.launch(fs.existsSync(sandbox) ? { executablePath: sandbox } : {});

async function snap(base, url) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 }, javaScriptEnabled: false, reducedMotion: "reduce" });
  await ctx.route(/https:\/\/fonts\.(googleapis|gstatic)\.com\//, (r) => r.abort());
  const page = await ctx.newPage();
  await page.goto(base.replace(/\/+$/, "") + url, { waitUntil: "load" });
  // `evaluate` works with JavaScript disabled; `addStyleTag` hangs.
  await page.evaluate(() => {
    const s = document.createElement("style");
    s.textContent = "*,*::before,*::after{transition:none!important;animation:none!important}";
    document.head.appendChild(s);
  });
  const out = {};
  for (const scheme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.waitForTimeout(30);
    out[scheme] = await page.evaluate((props) => {
      const o = {};
      const walk = (el, at) => {
        const cs = getComputedStyle(el);
        o[at] = props.map((p) => cs.getPropertyValue(p));
        const n = {};
        for (const c of el.children) {
          if (c.tagName === "SCRIPT" || c.tagName === "STYLE") continue;
          const l = c.tagName + "." + (typeof c.className === "string" ? c.className.split(/\s+/).slice(0, 2).join(".") : "");
          n[l] = (n[l] || 0) + 1;
          walk(c, at + "/" + l + "#" + n[l]);
        }
      };
      walk(document.body, "body");
      return o;
    }, PROPS);
  }
  await ctx.close();
  return out;
}

let bad = 0;
for (const r of routes) {
  const a = await snap(A, r);
  const b = await snap(B, r);
  let n = 0;
  const ex = [];
  for (const s of ["light", "dark"])
    for (const k of Object.keys(a[s]))
      if (b[s][k]) {
        for (let i = 0; i < PROPS.length; i++) {
          if (a[s][k][i] !== b[s][k][i]) {
            n++;
            if (!/^(width|height)$/.test(PROPS[i]) && ex.length < 3) ex.push(`${s} ${k.split("/").slice(-2).join("/")} {${PROPS[i]}: ${a[s][k][i]} => ${b[s][k][i]}}`);
          }
        }
      }
  if (n) bad++;
  console.log(`${n ? "DIFF" : "same"} ${r} (${Object.keys(a.light).length} elements, ${n} differences) ${ex.join(" || ")}`);
}
console.log(`routes with differences: ${bad} of ${routes.length}`);
await browser.close();
process.exit(bad ? 1 : 0);
