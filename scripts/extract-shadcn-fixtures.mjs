#!/usr/bin/env node
// Extracts the data and the chart-relevant props of shadcn/ui's area, line
// and bar chart registry demos (new-york-v4) into
// `preview/tests/shadcn/{area,line,bar}-<variant>.json`, the fixtures
// `preview/src/chart_parity.rs` compares our gallery demos against.
//
//   node scripts/extract-shadcn-fixtures.mjs <registry-dir> [<probe-dir>]
//
// `<registry-dir>` holds `chart-{area,line,bar}-*.json` registry items (the
// files `https://ui.shadcn.com/r/styles/new-york-v4/<name>.json` serves). The
// fixtures are derived, never hand-typed: re-run this after refreshing the
// registry. Only facts our demos must reproduce are extracted -- the data,
// the series config (order = JSX order, which is Recharts' stacking/draw
// order), and the props that change geometry or behavior.
//
// With `<probe-dir>` it also writes `preview/tests/shadcn/geometry.json`: the
// SVG geometry shadcn's own pages render (gridlines, line/area curve paths,
// bar rects, dots -- at the 369px card width), from DOM probes of
// `https://ui.shadcn.com/view/new-york-v4/<chart>` saved as
// `sp-{area,line,bar,tip}.json` (each `{ <chart>: { svg, grid, curves, bars,
// dots } }`, attribute values exactly as Recharts wrote them).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, "..", "preview", "tests", "shadcn");
const src = process.argv[2] ?? process.env.SHADCN_REGISTRY_DIR;
const probes = process.argv[3];
if (!src) {
  console.error("usage: extract-shadcn-fixtures.mjs <registry-dir>");
  process.exit(2);
}

/** shadcn demo name -> our gallery variant folder, where it is not just the
 * name's suffix with `-` -> `_` (`chart-bar-stacked` is shadcn's "Stacked +
 * Legend"; our plain `stacked` variant is the same chart without one). */
const VARIANT = { "chart-bar-stacked": "stacked_legend" };
const variantOf = (name) => {
  const suffix = name.split("-").slice(2).join("-");
  return VARIANT[name] ?? (suffix === "default" ? "main" : suffix.replaceAll("-", "_"));
};

/** Index just past the brace/bracket/paren group opening at `start`. */
function skipGroup(text, start) {
  const pairs = { "{": "}", "[": "]", "(": ")" };
  const stack = [pairs[text[start]]];
  let i = start + 1;
  let quote = null;
  for (; i < text.length && stack.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === "\\") i++;
      else if (c === quote) quote = null;
    } else if (c === '"' || c === "'" || c === "`") quote = c;
    else if (pairs[c]) stack.push(pairs[c]);
    else if (c === stack[stack.length - 1]) stack.pop();
  }
  return i;
}

/** The first `{...}`/`[...]` literal after `marker`, brace-balanced. */
function literalAfter(text, marker) {
  const at = text.indexOf(marker);
  if (at < 0) return null;
  const start = text.slice(at + marker.length).search(/[[{]/) + at + marker.length;
  return text.slice(start, skipGroup(text, start));
}

/** Evaluates a plain data literal (numbers, strings, nested arrays/objects). */
const evalLiteral = (lit) => Function(`"use strict"; return (${lit});`)();

/** Every `<Name ...>` opening tag's attribute text (JSX-brace aware). */
function tags(tsx, name) {
  const found = [];
  const re = new RegExp(`<${name}(?=[\\s/>])`, "g");
  let m;
  while ((m = re.exec(tsx))) {
    let i = m.index + m[0].length;
    const begin = i;
    while (i < tsx.length) {
      const c = tsx[i];
      if (c === "{") i = skipGroup(tsx, i);
      else if (c === '"') i = tsx.indexOf('"', i + 1) + 1;
      else if (c === ">" || (c === "/" && tsx[i + 1] === ">")) break;
      else i++;
    }
    found.push(tsx.slice(begin, i));
  }
  return found;
}

/** Parse one JSX attribute list into `{ name: value }`: strings, evaluated
 * literals, `true` for a bare flag, or `"<expr>"` for a non-literal. */
function attrs(text) {
  const out = {};
  let i = 0;
  while (i < text.length) {
    const m = /[A-Za-z_][\w-]*/.exec(text.slice(i));
    if (!m) break;
    const name = m[0];
    i += m.index + name.length;
    if (text[i] !== "=") {
      out[name] = true;
      continue;
    }
    i++;
    if (text[i] === '"') {
      const end = text.indexOf('"', i + 1);
      out[name] = text.slice(i + 1, end);
      i = end + 1;
    } else if (text[i] === "{") {
      const end = skipGroup(text, i);
      const expr = text.slice(i + 1, end - 1).trim();
      try {
        out[name] = evalLiteral(expr);
      } catch {
        out[name] = "<expr>";
      }
      i = end;
    }
  }
  return out;
}

const first = (tsx, name) => {
  const t = tags(tsx, name);
  return t.length ? attrs(t[0]) : null;
};

/** The chart's root element (`AreaChart`/`LineChart`/`BarChart`). */
const KINDS = { area: "AreaChart", line: "LineChart", bar: "BarChart" };
const SERIES = { area: "Area", line: "Line", bar: "Bar" };

function extract(name, tsx) {
  const kind = name.split("-")[1];
  const root = first(tsx, KINDS[kind]);
  const series = tags(tsx, SERIES[kind]).map((t) => {
    const a = attrs(t);
    const dot = a.dot === undefined ? null : a.dot === "<expr>" ? "custom" : a.dot;
    return {
      dataKey: a.dataKey ?? null,
      type: a.type ?? null,
      fill: typeof a.fill === "string" ? a.fill : null,
      fillOpacity: a.fillOpacity ?? null,
      stackId: a.stackId ?? null,
      radius: a.radius ?? null,
      strokeWidth: a.strokeWidth ?? null,
      dot,
      activeDot: a.activeDot ?? null,
    };
  });
  const grid = first(tsx, "CartesianGrid");
  const axis = (n) => {
    const a = first(tsx, n);
    if (!a) return null;
    return {
      dataKey: a.dataKey ?? null,
      type: a.type ?? null,
      hide: a.hide === true,
      tickMargin: a.tickMargin ?? null,
      minTickGap: a.minTickGap ?? null,
      tickCount: a.tickCount ?? null,
    };
  };
  const tooltipTag = first(tsx, "ChartTooltip");
  const content = first(tsx, "ChartTooltipContent") ?? {};
  const ACTIVE = tsx.match(/const ACTIVE_INDEX = (\d+)/);
  return {
    name,
    variant: variantOf(name),
    title: tsx.match(/<CardTitle>([^<]*)<\/CardTitle>/)[1].trim(),
    data: evalLiteral(literalAfter(tsx, "const chartData = ")),
    config: evalLiteral(
      literalAfter(tsx, "const chartConfig = ")
        // `icon: TrendingUp` is an identifier, not data: keep its name.
        .replace(/icon:\s*(\w+)/g, 'icon: "$1"'),
    ),
    chart: {
      kind,
      margin: root.margin ?? null,
      layout: root.layout ?? null,
      stackOffset: root.stackOffset ?? null,
    },
    series,
    grid: grid ? { vertical: grid.vertical !== false, horizontal: grid.horizontal !== false } : null,
    xAxis: axis("XAxis"),
    yAxis: axis("YAxis"),
    tooltip: {
      cursor: tooltipTag?.cursor !== false,
      indicator: content.indicator ?? "dot",
      hideLabel: content.hideLabel === true,
      hideIndicator: content.hideIndicator === true,
      nameKey: content.nameKey ?? null,
      labelKey: content.labelKey ?? null,
      labelFormatter: content.labelFormatter !== undefined,
    },
    legend: /<ChartLegend\b/.test(tsx),
    gradient: /<linearGradient\b/.test(tsx),
    cells: /<Cell\b/.test(tsx),
    labelLists: tags(tsx, "LabelList").map((t) => {
      const a = attrs(t);
      return { dataKey: a.dataKey ?? null, position: a.position ?? null, offset: a.offset ?? null };
    }),
    activeIndex: ACTIVE ? Number(ACTIVE[1]) : null,
  };
}

fs.mkdirSync(out, { recursive: true });
const names = fs
  .readdirSync(src)
  .filter((f) => /^chart-(area|line|bar)-.*\.json$/.test(f))
  .sort();
for (const file of names) {
  const item = JSON.parse(fs.readFileSync(path.join(src, file), "utf8"));
  const fixture = extract(item.name, item.files[0].content);
  const target = path.join(out, `${fixture.chart.kind}-${fixture.variant}.json`);
  fs.writeFileSync(target, JSON.stringify(fixture, null, 2) + "\n");
  console.log(`${item.name} -> ${path.relative(path.join(here, ".."), target)}`);
}

if (probes) {
  const geometry = {};
  // `sp-tip.json` (the tooltip probes) also holds `chart-area-default`.
  for (const file of ["area", "line", "bar", "tip"]) {
    const probe = JSON.parse(fs.readFileSync(path.join(probes, `sp-${file}.json`), "utf8"));
    for (const [name, p] of Object.entries(probe)) {
      // The 369px cards only: the 1286px heroes are a different size.
      if (!/^chart-(area|line|bar)-/.test(name) || /interactive/.test(name)) continue;
      geometry[name] = {
        svg: p.svg,
        grid: p.grid,
        curves: p.curves.map((c) => ({ cls: c.cls, d: c.d })),
        bars: p.bars.map((b) => ({ x: +b.x, y: +b.y, w: +b.w, h: +b.h, d: b.d })),
        dots: p.dots.map((d) => [d[0], d[1], Number(d[2])]),
      };
    }
  }
  const target = path.join(out, "geometry.json");
  fs.writeFileSync(target, JSON.stringify(geometry, null, 1) + "\n");
  console.log(`probes -> ${path.relative(path.join(here, ".."), target)}`);
}
