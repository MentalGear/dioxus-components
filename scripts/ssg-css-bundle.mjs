#!/usr/bin/env node
// scripts/ssg-css-bundle.mjs -- make the stylesheet set of the SSG site a property of the DOCUMENT, not of the route.
//
//   node scripts/ssg-css-bundle.mjs <public_dir> [--manifest <.manifest.json>] [--keep-links]
//
// Run by scripts/build-ssg.sh after `dx build --ssg` (never by hand on a tree you care about: it rewrites the
// HTML in place). Idempotent per tree: a second run refuses ("already bundled") instead of stacking bundles.
//
// THE DEFECT THIS CLOSES (measured 2026-10-05, dev-docs/backlog.md "first-paint style delivery"):
// every component ships its CSS with `document::Link { rel: "stylesheet", href: asset!(..) }` at the point it
// RENDERS. For a hard load the SSR `<head>` already carries the sheets of everything the SSR rendered, as
// render-blocking `<link>`s, so the first paint is right (JS-off and hydrated screenshots are pixel-identical,
// hydration inserts no sheet, CLS 0.0001). But on a client-side navigation -- the first time you click through
// to /component/field/ -- and when a popover first opens (the calendar inside the date picker), the route or
// subtree renders FIRST and `eager_head` inserts its `<link>`s into a document that has already had its first
// paint. A stylesheet inserted after `<body>` exists is not render-blocking (HTML spec: render-blocking
// elements can only be added while the document "allows adding render-blocking elements"; `blocking=render`
// on a late link measured as a no-op), so the new content is painted with UA defaults -- white `<input>`s
// (rgb(255,255,255), 2px border, radius 0), grey UA buttons -- for one network round trip, then the sheet
// lands, `display:block` becomes `flex`, the page reflows (CLS 0.02-0.08) and `transition: background-color`
// animates the white to the dark theme. Measured on 8 of 8 component pages: 5-10 late sheets, 150-270 ms
// unstyled at +100 ms of CSS latency.
//
// THE CONSTRUCTION: one render-blocking `<link rel=stylesheet>` in EVERY page's `<head>` that carries every
// component stylesheet any route can need, so no route, popover or dialog can ever depend on a sheet that is
// discovered at render time. The set is data-driven from the build itself -- the union of the `<link
// rel=stylesheet>` of every prerendered page (plus every project-owned sheet in dx's asset manifest, for a
// sheet only a late-mounting subtree would link) -- so a new component is covered the day it lands, with no
// list to maintain. The four global sheets (`GlobalHead`: main, dx-components-theme, dx-effects,
// theme-presets) stay separate and first. A bundle is also fewer requests than the ~20 per-route sheets it
// replaces and is cached once for the whole site.
//
// What stays per component: the `document::Link`s in the wrappers. They are what a `dx components add` user
// gets and they still run here; `eager_head::create_link` skips one whose file is already in the bundle
// (`data-covers` below), so a client-side navigation inserts nothing.
//
// Cascade: the per-route sheets are ordered by RENDER order (a composer's sheet before its parts': `field` before `label`,
// so `.dx-label` (12.8px) beats `.dx-field-label` (14px), equal specificity, later wins). A bundle has ONE order, and it
// must not flip those results: bundling in first-appearance order did exactly that to Field's label (14px instead of 12.8px,
// caught by first-paint-styles.spec.ts). So the bundle is ordered to agree with as many pages' own head orders as it can
// (pairwise votes over every page, then local search); 208 of 2943 sheet pairs are ordered both ways on different pages --
// route-and-history-dependent cascade that exists today and that a single order can only resolve one way; they are printed
// (`order conflicts`) and the all-routes computed-style comparison (first-paint-styles.spec.ts, "cascade") says whether any of
// them changes a pixel. Sheets are otherwise `dx-`/`ec-`-prefixed (scripts/check-dx-class-prefix.sh) with no
// `@import`/`@charset`/`url()` (checked here: such a sheet aborts the build).
//
// `--keep-links` leaves each page's own per-component `<link>`s in the head after the bundle (they then load
// twice: harmless, wasteful). The default removes them, so every head is: fonts, the four globals, the bundle.
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

const args = process.argv.slice(2);
const positional = args.filter((a, i) => !a.startsWith("--") && args[i - 1] !== "--manifest");
if (positional.length !== 1) {
  console.error("usage: ssg-css-bundle.mjs <public_dir> [--manifest <.manifest.json>] [--keep-links]");
  process.exit(2);
}
const publicDir = path.resolve(positional[0]);
const keepLinks = args.includes("--keep-links");
const manifestPath = path.resolve(
  args.includes("--manifest") ? args[args.indexOf("--manifest") + 1] : path.join(publicDir, "..", ".manifest.json"),
);

const GLOBAL_SOURCES = new Set(["assets/main.css", "assets/dx-components-theme.css", "assets/dx-effects.css", "assets/theme-presets.css"]);
const MARK = "data-dx-css-bundle";

function die(msg) {
  console.error(`ssg-css-bundle: ${msg}`);
  process.exit(1);
}
if (!fs.existsSync(path.join(publicDir, "index.html"))) die(`not an SSG output dir (no index.html): ${publicDir}`);
if (!fs.existsSync(manifestPath)) die(`dx asset manifest not found: ${manifestPath} (pass --manifest)`);

// bundled basename -> source path relative to preview/ (e.g. "style-dxhabc.css" -> "src/components/field/style.css")
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const sourceOf = new Map();
for (const [src, entries] of Object.entries(manifest.assets ?? {})) {
  const rel = src.replace(/^.*\/preview\//, "");
  for (const e of entries) sourceOf.set(e.bundled_path, rel);
}

function walk(dir, out = []) {
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) walk(p, out);
    else if (ent.name === "index.html") out.push(p);
  }
  return out;
}
const pages = walk(publicDir).sort();

const LINK_RE = /<link\b[^>]*>/g;
const attr = (tag, name) => tag.match(new RegExp(`\\s${name}="([^"]*)"`))?.[1];
const isSheet = (tag) => attr(tag, "rel") === "stylesheet" && (attr(tag, "href") ?? "").length > 0 && !/^(https?:)?\/\//.test(attr(tag, "href"));

// --- 1. read every page's head ---------------------------------------------------------------
const parsed = [];
let alreadyBundled = 0;
for (const file of pages) {
  const html = fs.readFileSync(file, "utf8");
  const end = html.indexOf("</head>");
  if (end < 0) die(`${file}: no </head>`);
  const head = html.slice(0, end);
  if (head.includes(MARK)) {
    alreadyBundled++;
    continue;
  }
  const sheets = [...head.matchAll(LINK_RE)].filter((m) => isSheet(m[0])).map((m) => ({ tag: m[0], at: m.index, href: attr(m[0], "href") }));
  parsed.push({ file, html, end, sheets });
}
if (alreadyBundled > 0) die(`${alreadyBundled} of ${pages.length} page(s) already carry a bundle (${MARK}); refusing to stack another. Rebuild first.`);

const base = (href) => href.split("/").pop();
let prefix = null;
const order = []; // basenames of bundled sheets, first appearance (refined below)
const seen = new Set();
const pageOrders = []; // each page's own bundled sheets, in its render order
for (const pg of parsed) {
  const mine = [];
  pageOrders.push(mine);
  for (const s of pg.sheets) {
    const b = base(s.href);
    const src = sourceOf.get(b);
    if (!src) die(`${pg.file}: stylesheet ${s.href} is not in dx's asset manifest ${manifestPath} (stale manifest?)`);
    if (GLOBAL_SOURCES.has(src)) {
      const p = s.href.slice(0, s.href.lastIndexOf("/assets/"));
      if (prefix === null) prefix = p;
      else if (prefix !== p) die(`${pg.file}: inconsistent base path (${JSON.stringify(p)} vs ${JSON.stringify(prefix)})`);
      continue;
    }
    mine.push(b);
    if (!seen.has(b)) {
      seen.add(b);
      order.push(b);
    }
  }
}
if (prefix === null) die("no page links main.css: nothing to anchor the bundle after");

// Order the bundle by the pages' own orders (see "Cascade" above). votes.get(a).get(b) = pages that put a before b.
const votes = new Map(order.map((a) => [a, new Map()]));
// A component's own page (`/component/<dir>/`) is where its look is designed and tested, so its order counts 4x for the
// pairs that involve that component (the home gallery, which renders Label before Field, must not outvote the Field page).
const dirOf = (b) => sourceOf.get(b)?.match(/^src\/components\/([^/]+)\//)?.[1];
for (let p = 0; p < pageOrders.length; p++) {
  const mine = pageOrders[p];
  const own = parsed[p].file.replace(/\\/g, "/").match(/\/component\/([^/]+)\/index\.html$/)?.[1];
  for (let i = 0; i < mine.length; i++) {
    for (let j = i + 1; j < mine.length; j++) {
      const w = own && (dirOf(mine[i]) === own || dirOf(mine[j]) === own) ? 4 : 1;
      const m = votes.get(mine[i]);
      m.set(mine[j], (m.get(mine[j]) ?? 0) + w);
    }
  }
}
const want = (a, b) => votes.get(a).get(b) ?? 0; // weighted pages that want a before b
// Cost of placing `n` at position `at` of `rest`: pages that disagree with it, summed over the others.
const costAt = (n, rest, at) => {
  let c = 0;
  for (let k = 0; k < rest.length; k++) c += k < at ? want(n, rest[k]) : want(rest[k], n);
  return c;
};
for (let pass = 0, moved = true; moved && pass < 50; pass++) {
  moved = false;
  for (const n of [...order]) {
    const rest = order.filter((x) => x !== n);
    const cur = order.indexOf(n);
    let best = cur;
    let bestCost = costAt(n, rest, cur);
    for (let at = 0; at <= rest.length; at++) {
      const c = costAt(n, rest, at);
      if (c < bestCost) {
        best = at;
        bestCost = c;
      }
    }
    if (best !== cur) {
      rest.splice(best, 0, n);
      order.splice(0, order.length, ...rest);
      moved = true;
    }
  }
}
const pos = new Map(order.map((b, i) => [b, i]));
const disagree = [];
for (const [a, row] of votes) for (const [b, n] of row) if (pos.get(a) > pos.get(b)) disagree.push([a, b, n, want(b, a)]);
const conflicts = disagree.reduce((c, [, , n]) => c + n, 0);

// Project-owned sheets linked by a component that no page SSR-renders (only a late-mounting subtree would
// link them): include them too, so "reachable" never depends on a prerendered page existing.
const extra = [...sourceOf.entries()]
  .filter(([b, src]) => b.endsWith(".css") && src.startsWith("src/") && !GLOBAL_SOURCES.has(src) && !seen.has(b))
  .sort((a, b) => a[1].localeCompare(b[1]));
for (const [b] of extra) {
  seen.add(b);
  order.push(b);
}

// --- 2. build the bundle ---------------------------------------------------------------------
const chunks = [];
for (const b of order) {
  const file = path.join(publicDir, "assets", b);
  if (!fs.existsSync(file)) die(`bundled sheet missing from the output: ${file}`);
  const css = fs.readFileSync(file, "utf8").replace(/^﻿/, "");
  if (/@import\b|@charset\b/.test(css)) die(`${b} (${sourceOf.get(b)}) has @import/@charset, which cannot sit inside a concatenated sheet`);
  if (/url\(/.test(css)) die(`${b} (${sourceOf.get(b)}) has url(): a bundle would change what it resolves against; handle it first`);
  chunks.push(`/* ${sourceOf.get(b)} (${b}) */\n${css.trimEnd()}\n`);
}
const body = chunks.join("\n");
const hash = crypto.createHash("sha1").update(body).digest("hex").slice(0, 16);
const bundleName = `dx-components-all-dxh${hash}.css`;
fs.writeFileSync(path.join(publicDir, "assets", bundleName), body);
const bundleHref = `${prefix}/assets/${bundleName}`;
const covers = order.join(" ");
const bundleTag = `<link rel="stylesheet" href="${bundleHref}" ${MARK} data-covers="${covers}"/>`;

// --- 3. rewrite every page -------------------------------------------------------------------
let rewritten = 0;
let removed = 0;
for (const pg of parsed) {
  // Remove (unless --keep-links) the per-route links the bundle now carries, last to first so offsets hold.
  let head = pg.html.slice(0, pg.end);
  const edits = [];
  for (const s of pg.sheets) {
    const src = sourceOf.get(base(s.href));
    if (!GLOBAL_SOURCES.has(src) && !keepLinks) edits.push({ at: s.at, len: s.tag.length, with: "" });
  }
  // Insert the bundle right after the last global link (the four globals are adjacent in GlobalHead order).
  const globals = pg.sheets.filter((s) => GLOBAL_SOURCES.has(sourceOf.get(base(s.href))));
  if (globals.length !== GLOBAL_SOURCES.size) die(`${pg.file}: expected the ${GLOBAL_SOURCES.size} global sheets, found ${globals.length}`);
  const last = globals.reduce((a, b) => (b.at > a.at ? b : a));
  edits.push({ at: last.at + last.tag.length, len: 0, with: bundleTag });
  edits.sort((a, b) => b.at - a.at);
  for (const e of edits) {
    head = head.slice(0, e.at) + e.with + head.slice(e.at + e.len);
    if (e.with === "") removed++;
  }
  fs.writeFileSync(pg.file, head + pg.html.slice(pg.end));
  rewritten++;
}

// --- 4. self-check ---------------------------------------------------------------------------
for (const pg of parsed) {
  const html = fs.readFileSync(pg.file, "utf8");
  const head = html.slice(0, html.indexOf("</head>"));
  const n = head.split(MARK).length - 1;
  if (n !== 1) die(`${pg.file}: expected exactly one bundle link, found ${n}`);
  const links = [...head.matchAll(LINK_RE)].filter((m) => isSheet(m[0]));
  const idx = links.findIndex((m) => m[0].includes(MARK));
  const before = links.slice(0, idx).map((m) => sourceOf.get(base(attr(m[0], "href"))));
  if (!(before.length === GLOBAL_SOURCES.size && before.every((s) => GLOBAL_SOURCES.has(s)))) {
    die(`${pg.file}: the bundle must follow exactly the four global sheets, found ${JSON.stringify(before)} before it`);
  }
  if (!keepLinks && links.length !== GLOBAL_SOURCES.size + 1) die(`${pg.file}: ${links.length} local stylesheet links left, expected ${GLOBAL_SOURCES.size + 1}`);
}

const gz = (await import("node:zlib")).gzipSync(body).length;
console.log(
  `ssg-css-bundle: ${rewritten} page(s); bundle ${bundleName} = ${order.length} sheets (${extra.length} from the manifest only), ${(body.length / 1024).toFixed(0)} KB (${(gz / 1024).toFixed(0)} KB gzip); ${removed} per-route <link> removed${keepLinks ? " (--keep-links: none)" : ""}`,
);
const nameOf = (b) => dirOf(b) ?? (sourceOf.get(b) ?? b).split("/").pop();
console.log(
  `ssg-css-bundle: order conflicts: ${disagree.length} sheet pair(s) are ordered the other way on some page(s) (${conflicts} weighted page-pair votes overruled)` +
    (disagree.length
      ? "; most overruled: " +
        disagree
          .sort((x, y) => y[2] - x[2])
          .slice(0, 5)
          .map(([a, b, n, m]) => `${nameOf(a)} before ${nameOf(b)} on ${n} weighted page(s) (kept ${m})`)
          .join("; ")
      : ""),
);
console.log(`DX_CSS_BUNDLE=${bundleHref}`);
