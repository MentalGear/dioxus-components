#!/usr/bin/env node
// Extracts shadcn/ui's polar chart demos (new-york-v4 `chart-pie-*`,
// `chart-radar-*`, `chart-radial-*`) into `preview/tests/shadcn/<name>.json`,
// the fixtures `preview/src/polar_parity.rs` compares our pie/radar/radial
// demos against.
//
//   node scripts/extract-shadcn-polar-fixtures.mjs <registry-dir> [<rendered-dir>]
//
// `<registry-dir>` holds the registry items (`chart-pie-simple.json`, ... --
// what `https://ui.shadcn.com/r/styles/new-york-v4/<name>.json` serves): the
// demo's source, from which the data, the config, the card text and the
// Recharts props are read.
//
// `<rendered-dir>` (optional) holds what ui.shadcn.com actually renders for
// each demo: `<name>.json`, a JSON array of `{ w, h, vb, html }` -- the
// `outerHTML` of every `svg.recharts-surface` on
// `https://ui.shadcn.com/view/new-york-v4/<name>` (a Playwright
// `page.evaluate` over `document.querySelectorAll("svg.recharts-surface")`).
// From it the script measures the geometry Recharts drew -- every sector's
// radii and angles, the radar's rings and vertices, the label positions --
// into the fixture's `rendered` section, so the parity test holds our
// geometry to Recharts' own numbers rather than to a re-derivation of them.
//
// The fixtures are derived, never hand-typed: re-run this after refreshing
// the registry (and the rendered dumps).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, "..", "preview", "tests", "shadcn");
const src = process.argv[2] ?? process.env.SHADCN_REGISTRY_DIR;
const rendered = process.argv[3] ?? process.env.SHADCN_RENDERED_DIR;
if (!src) {
  console.error("usage: extract-shadcn-polar-fixtures.mjs <registry-dir> [<rendered-dir>]");
  process.exit(2);
}

// ---------------------------------------------------------------------------
// Source (TSX) extraction
// ---------------------------------------------------------------------------

/** The first `{ ... }`/`[ ... ]` literal after `marker`, brace-balanced. */
function literalAfter(text, marker) {
  const at = text.indexOf(marker);
  if (at < 0) return null;
  const start = text.slice(at + marker.length).search(/[[{]/) + at + marker.length;
  const open = text[start];
  const close = open === "{" ? "}" : "]";
  let depth = 0;
  for (let i = start; i < text.length; i++) {
    if (text[i] === open) depth++;
    else if (text[i] === close && --depth === 0) return text.slice(start, i + 1);
  }
  return null;
}

/** Evaluates a plain data literal (numbers, strings, nested arrays/objects). */
const evalLiteral = (lit) => (lit == null ? null : Function(`"use strict"; return (${lit});`)());

/** The attribute text of every `<Tag ...>` / `<Tag .../>` opening tag. */
function tags(tsx, tag) {
  const result = [];
  const re = new RegExp(`<${tag}(?=[\\s/>])`, "g");
  let m;
  while ((m = re.exec(tsx))) {
    // Scan to the end of the opening tag, skipping `{...}` expressions.
    let depth = 0;
    let i = m.index + m[0].length;
    for (; i < tsx.length; i++) {
      const c = tsx[i];
      if (c === "{") depth++;
      else if (c === "}") depth--;
      else if (c === ">" && depth === 0) break;
    }
    result.push(tsx.slice(m.index + m[0].length, i).replace(/\/$/, ""));
  }
  return result;
}

/** One JSX prop: a string, a number/boolean/array/object literal, `true` for a bare flag. */
function prop(attrs, name) {
  const str = attrs.match(new RegExp(`(?:^|\\s)${name}="([^"]*)"`));
  if (str) return str[1];
  const at = attrs.search(new RegExp(`(?:^|\\s)${name}=\\{`));
  if (at >= 0) {
    const lit = literalAfter(attrs.slice(at), `${name}=`);
    const braced = lit && lit.startsWith("{") ? lit.slice(1, -1).trim() : lit;
    try {
      return evalLiteral(braced);
    } catch {
      return "<expression>";
    }
  }
  if (new RegExp(`(?:^|\\s)${name}(?=\\s|$)`).test(attrs)) return true;
  return null;
}

const props = (attrs, names) => Object.fromEntries(names.map((n) => [n, prop(attrs, n)]));

function extract(name, tsx) {
  const container = tags(tsx, "ChartContainer")[0] ?? "";
  const tooltip = tags(tsx, "ChartTooltipContent")[0];
  const legend = /<ChartLegend\b/.test(tsx);
  const fixture = {
    name,
    title: tsx.match(/<CardTitle>([^<]*)<\/CardTitle>/)[1].trim(),
    description: tsx
      .match(/<CardDescription>\s*([\s\S]*?)\s*<\/CardDescription>/)[1]
      .replace(/\s+/g, " ")
      .trim(),
    footer: [...(tsx.match(/<CardFooter[\s\S]*?<\/CardFooter>/)?.[0] ?? "").matchAll(/>\s*([^<>{}]*[A-Za-z][^<>{}]*?)\s*</g)]
      .map((m) => m[1].replace(/\s+/g, " ").trim())
      .filter(Boolean),
    containerClass: prop(container, "className"),
    data: evalLiteral(literalAfter(tsx, "const chartData = ")),
    desktopData: evalLiteral(literalAfter(tsx, "const desktopData = ")),
    mobileData: evalLiteral(literalAfter(tsx, "const mobileData = ")),
    config: evalLiteral(
      literalAfter(tsx, "const chartConfig = ").replace(/icon:\s*(\w+)/g, 'icon: "$1"'),
    ),
    tooltip: tooltip == null
      ? null
      : {
          hideLabel: prop(tooltip, "hideLabel") === true,
          indicator: prop(tooltip, "indicator") ?? "dot",
          nameKey: prop(tooltip, "nameKey"),
          labelKey: prop(tooltip, "labelKey"),
        },
    legend,
    // The `<Label content=...>` center text: the tspan class sizes and the
    // first tspan's y offset (`viewBox.cy - 16` in the half gauge).
    centerLabel: /<Label\b/.test(tsx)
      ? {
          primaryClass: tsx.match(/<tspan[\s\S]*?className="([^"]*font-bold[^"]*)"/)?.[1] ?? null,
          raised: /\(viewBox\.cy \|\| 0\) - 16/.test(tsx),
        }
      : null,
  };
  if (name.startsWith("chart-pie-")) {
    fixture.pies = tags(tsx, "Pie").map((a) => ({
      ...props(a, [
        "dataKey",
        "nameKey",
        "innerRadius",
        "outerRadius",
        "startAngle",
        "endAngle",
        "paddingAngle",
        "cornerRadius",
        "labelLine",
      ]),
      label: /(?:^|\s)label(?=[\s=]|$)/.test(a) ? (/label=\{/.test(a) ? "custom" : true) : false,
      activeShape: /shape=\{/.test(a),
    }));
    fixture.labelList = tags(tsx, "LabelList").map((a) => props(a, ["dataKey", "position", "fontSize"]));
    fixture.activeIndex = Number(tsx.match(/const ACTIVE_INDEX = (\d+)/)?.[1] ?? NaN);
    fixture.halo = /outerRadius \+ 25/.test(tsx);
  } else if (name.startsWith("chart-radial-")) {
    fixture.radialChart = props(tags(tsx, "RadialBarChart")[0] ?? "", [
      "startAngle",
      "endAngle",
      "innerRadius",
      "outerRadius",
    ]);
    fixture.radialBars = tags(tsx, "RadialBar").map((a) =>
      props(a, ["dataKey", "background", "cornerRadius", "stackId"]),
    );
    fixture.polarGrid = (tags(tsx, "PolarGrid")[0] ?? null) && props(tags(tsx, "PolarGrid")[0], [
      "gridType",
      "radialLines",
      "polarRadius",
    ]);
    fixture.labelList = tags(tsx, "LabelList").map((a) => props(a, ["dataKey", "position", "fontSize"]));
  } else if (name.startsWith("chart-radar-")) {
    fixture.radarChart = props(tags(tsx, "RadarChart")[0] ?? "", ["margin"]);
    fixture.radars = tags(tsx, "Radar").map((a) => ({
      ...props(a, ["dataKey", "fillOpacity", "strokeWidth"]),
      dot: prop(a, "dot"),
    }));
    const grid = tags(tsx, "PolarGrid")[0];
    fixture.polarGrid = grid == null
      ? null
      : {
          ...props(grid, ["gridType", "radialLines", "polarRadius"]),
          fill: /fill-\(--color-/.test(grid),
        };
    const angleAxis = tags(tsx, "PolarAngleAxis")[0];
    fixture.angleAxis = angleAxis == null ? null : { customTick: /tick=\{/.test(angleAxis) };
    const radiusAxis = tags(tsx, "PolarRadiusAxis")[0];
    fixture.radiusAxis = radiusAxis == null ? null : props(radiusAxis, ["angle", "orientation"]);
  }
  return fixture;
}

// ---------------------------------------------------------------------------
// Rendered (SVG) measurement
// ---------------------------------------------------------------------------

const num = (s) => [...s.matchAll(/-?\d*\.?\d+(?:e-?\d+)?/g)].map((m) => Number(m[0]));
const round = (v, d = 2) => Math.round(v * 10 ** d) / 10 ** d;
/** Recharts degrees of a point around (cx, cy): 0 = three o'clock, ccw. */
const angleOf = ([x, y], [cx, cy]) => ((Math.atan2(-(y - cy), x - cx) * 180) / Math.PI + 360) % 360;
const dist = ([x, y], [cx, cy]) => Math.hypot(x - cx, y - cy);

/** Every `A` command of a path, with its start point and end point. */
function parsePath(d) {
  const cmds = [...d.matchAll(/([MLAZ])([^MLAZ]*)/gi)];
  let cur = null;
  const first = cmds[0] ? num(cmds[0][2]) : null;
  const arcs = [];
  let beforeLine = null;
  for (const [, c, args] of cmds) {
    const n = num(args);
    if (c === "M" || c === "L") {
      if (c === "L" && beforeLine == null) beforeLine = arcs.length;
      cur = [n[0], n[1]];
    } else if (c === "A") {
      for (let i = 0; i + 6 < n.length; i += 7) {
        const end = [n[i + 5], n[i + 6]];
        arcs.push({ r: n[i], large: n[i + 3], sweep: n[i + 4], from: cur, to: end });
        cur = end;
      }
    }
  }
  return { start: first && [first[0], first[1]], arcs, beforeLine: beforeLine ?? arcs.length };
}

/** One sector: radii (main arcs only, not corner fillets) and its ccw angles. */
function sector(d, c) {
  const { start, arcs, beforeLine } = parsePath(d);
  const main = arcs.filter((a) => Math.abs(dist(a.to, c) - a.r) < 0.5 && Math.abs(dist(a.from, c) - a.r) < 0.5);
  if (!main.length || !start) return null;
  const radii = main.map((a) => a.r);
  const outer = Math.max(...radii);
  const inner = main.some((a) => a.r < outer - 0.01) ? Math.min(...radii) : 0;
  const startDeg = angleOf(start, c);
  const lastOuter = arcs[beforeLine - 1];
  let sweepDeg = (angleOf(lastOuter.to, c) - startDeg + 360) % 360;
  if (sweepDeg < 0.01 && main.some((a) => a.large)) sweepDeg = 360;
  return { outer: round(outer), inner: round(inner), start: round(startDeg), sweep: round(sweepDeg) };
}

const attr = (tag, name) => tag.match(new RegExp(`\\s${name}="([^"]*)"`))?.[1] ?? null;

function measure(svg) {
  const [, , w, h] = svg.vb.split(/\s+/).map(Number);
  const html = svg.html;
  // The polar centre Recharts used (a legend inside the chart moves it off
  // the box centre): any mark's own `cx`/`cy`, else the box centre.
  const centred = html.match(/<(?:path|circle|line)[^>]*\scx="([\d.]+)"[^>]*\scy="([\d.]+)"/);
  const c = centred ? [Number(centred[1]), Number(centred[2])] : [w / 2, h / 2];
  const sectors = [...html.matchAll(/<path[^>]*class="recharts-sector[^"]*"[^>]*>/g)].map((m) => {
    const tag = m[0];
    return {
      background: /radial-bar-background-sector/.test(tag),
      fill: attr(tag, "fill"),
      ...sector(attr(tag, "d"), c),
    };
  });
  const rings = [...html.matchAll(/<(?:path|circle)[^>]*class="recharts-polar-grid-concentric-[^"]*"[^>]*>/g)].map(
    (m) => Number(attr(m[0], "radius") ?? attr(m[0], "r")),
  );
  const spokes = [...html.matchAll(/<g class="recharts-polar-grid-angle">([\s\S]*?)<\/g>/g)]
    .map((m) => (m[1].match(/<line/g) ?? []).length)
    .reduce((a, b) => a + b, 0);
  const polygons = [...html.matchAll(/<path[^>]*id="recharts-radar-[^"]*"[^>]*>/g)].map((m) => {
    const tag = m[0];
    const pts = [...attr(tag, "d").matchAll(/[ML]\s*(-?[\d.]+),\s*(-?[\d.]+)/g)].map((p) => [Number(p[1]), Number(p[2])]);
    return {
      fill: attr(tag, "fill"),
      fillOpacity: attr(tag, "fill-opacity") == null ? 1 : Number(attr(tag, "fill-opacity")),
      strokeWidth: attr(tag, "stroke-width") == null ? null : Number(attr(tag, "stroke-width")),
      radii: pts.slice(0, -1).map((p) => round(dist(p, c))),
    };
  });
  const dots = [...html.matchAll(/<circle[^>]*class="recharts-dot recharts-radar-dot"[^>]*>/g)].map((m) => Number(attr(m[0], "r")));
  const texts = [...html.matchAll(/<text([^>]*)>([\s\S]*?)<\/text>/g)].map((m) => {
    const tag = m[1];
    const text = m[2].replace(/<[^>]*>/g, "").trim();
    const x = Number(attr(tag, "x"));
    const y = Number(attr(tag, "y"));
    const cls = attr(tag, "class") ?? "";
    const kind = /polar-angle-axis/.test(cls)
      ? "angle"
      : /polar-radius-axis/.test(cls)
        ? "radius"
        : /pie-label/.test(cls) || /recharts-text/.test(cls) === false && /text-anchor/.test(tag) && !/tspan/.test(m[2])
          ? "label"
          : /radial-bar-label/.test(cls)
            ? "radial-label"
            : "other";
    return {
      kind,
      text,
      anchor: attr(tag, "text-anchor"),
      x: Number.isNaN(x) ? null : round(x - c[0]),
      y: Number.isNaN(y) ? null : round(y - c[1]),
    };
  });
  const lines = [...html.matchAll(/<path[^>]*class="recharts-curve recharts-pie-label-line"[^>]*>/g)].length;
  return { width: w, height: h, cx: c[0], cy: c[1], sectors, rings, spokes, polygons, dots, texts, labelLines: lines };
}

// ---------------------------------------------------------------------------

fs.mkdirSync(out, { recursive: true });
const files = fs
  .readdirSync(src)
  .filter((f) => /^chart-(pie|radar|radial)-.*\.json$/.test(f))
  .sort();
if (files.length === 0) {
  console.error(`no chart-{pie,radar,radial}-*.json in ${src}`);
  process.exit(1);
}
for (const f of files) {
  const item = JSON.parse(fs.readFileSync(path.join(src, f), "utf8"));
  if (!item.files) continue; // a rendered dump in the same directory
  const fixture = extract(item.name, item.files[0].content);
  if (rendered) {
    const dump = path.join(rendered, `${item.name}.json`);
    if (fs.existsSync(dump)) {
      const svgs = JSON.parse(fs.readFileSync(dump, "utf8"));
      fixture.rendered = svgs.length ? measure(svgs[0]) : null;
    }
  }
  fs.writeFileSync(path.join(out, `${item.name}.json`), JSON.stringify(fixture, null, 2) + "\n");
  console.log(`wrote ${item.name}.json`);
}
