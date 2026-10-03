#!/usr/bin/env node
// Extracts the data and props of shadcn/ui's `chart-tooltip-*` registry demos
// (new-york-v4) into `preview/tests/shadcn/chart-tooltip-<name>.json`, the
// fixtures `preview/src/chart_tooltip_parity.rs` compares our demos against.
//
//   node scripts/extract-shadcn-tooltip-fixtures.mjs <registry-dir>
//
// `<registry-dir>` holds `chart-tooltip-*.json` registry items (the files
// `https://ui.shadcn.com/r/styles/new-york-v4/<name>.json` serves). The
// fixtures are derived, never hand-typed: re-run this after refreshing the
// registry. Only the facts our demos must reproduce are extracted.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, "..", "preview", "tests", "shadcn");
const src = process.argv[2] ?? process.env.SHADCN_REGISTRY_DIR;
if (!src) {
  console.error("usage: extract-shadcn-tooltip-fixtures.mjs <registry-dir>");
  process.exit(2);
}

/** The first `{ ... }`/`[ ... ]` literal after `marker`, brace-balanced. */
function literalAfter(text, marker) {
  const at = text.indexOf(marker);
  if (at < 0) return null;
  let i = text.search(/[[{]/, at + marker.length);
  const start = text.slice(at + marker.length).search(/[[{]/) + at + marker.length;
  const open = text[start];
  const close = open === "{" ? "}" : "]";
  let depth = 0;
  for (i = start; i < text.length; i++) {
    if (text[i] === open) depth++;
    else if (text[i] === close && --depth === 0) return text.slice(start, i + 1);
  }
  return null;
}

/** Evaluates a plain data literal (numbers, strings, nested arrays/objects). */
const evalLiteral = (lit) => Function(`"use strict"; return (${lit});`)();

function extract(name, tsx) {
  const data = evalLiteral(literalAfter(tsx, "const chartData = "));
  const config = evalLiteral(
    literalAfter(tsx, "const chartConfig = ")
      // `icon: Footprints` is an identifier, not data: keep its name as a string.
      .replace(/icon:\s*(\w+)/g, 'icon: "$1"'),
  );
  const bars = [...tsx.matchAll(/<Bar\s+([\s\S]*?)\/>/g)].map((m) => {
    const attrs = m[1];
    const get = (k) => attrs.match(new RegExp(`${k}="([^"]*)"`))?.[1] ?? null;
    const radius = attrs.match(/radius=\{(\[[^\]]*\])\}/)?.[1];
    return {
      dataKey: get("dataKey"),
      stackId: get("stackId"),
      fill: get("fill"),
      radius: radius ? JSON.parse(radius) : null,
    };
  });
  const tooltip = tsx.slice(tsx.indexOf("<ChartTooltip"), tsx.indexOf("</BarChart>"));
  // Everything from `<ChartTooltipContent` on, and just its own props (the text up to its
  // `formatter={` render function, whose markup has attributes of its own).
  const content = tooltip.slice(tooltip.indexOf("<ChartTooltipContent") + "<ChartTooltipContent".length);
  const props = content.slice(0, content.search(/formatter=\{|\/>/));
  const flag = (k) => new RegExp(`\\b${k}\\b(?!=)`).test(props);
  const xAxis = tsx.match(/<XAxis([\s\S]*?)\/>/)?.[1] ?? "";
  return {
    name,
    title: tsx.match(/<CardTitle>([^<]*)<\/CardTitle>/)[1].trim(),
    description: tsx
      .match(/<CardDescription>\s*([\s\S]*?)\s*<\/CardDescription>/)[1]
      .replace(/\s+/g, " ")
      .trim(),
    data,
    config,
    xAxis: {
      dataKey: xAxis.match(/dataKey="([^"]*)"/)?.[1] ?? null,
      tickLine: !/tickLine=\{false\}/.test(xAxis),
      axisLine: !/axisLine=\{false\}/.test(xAxis),
      tickMargin: Number(xAxis.match(/tickMargin=\{(\d+)\}/)?.[1] ?? 0),
      weekdayTicks: /weekday:\s*"short"/.test(xAxis),
    },
    hasYAxis: /<YAxis/.test(tsx),
    hasGrid: /<CartesianGrid/.test(tsx),
    bars,
    cursor: !/cursor=\{false\}/.test(tooltip),
    defaultIndex: Number(tooltip.match(/defaultIndex=\{(\d+)\}/)?.[1] ?? NaN),
    tooltip: {
      indicator: props.match(/indicator="(\w+)"/)?.[1] ?? "dot",
      hideLabel: flag("hideLabel"),
      hideIndicator: flag("hideIndicator"),
      labelKey: props.match(/labelKey="(\w+)"/)?.[1] ?? null,
      labelFormatter: /labelFormatter=/.test(props),
      formatter: /formatter=\{/.test(content),
      className: props.match(/className="([^"]*)"/)?.[1] ?? null,
      // The `formatter` demos' own row markup the way shadcn writes it.
      unit: /kcal/.test(content) ? "kcal" : null,
      totalRow: /Total/.test(content),
      // The `formatter` draws its own `h-2.5 w-2.5` colored square in each row.
      customSwatch: /--color-bg/.test(content),
    },
  };
}

fs.mkdirSync(out, { recursive: true });
const files = fs.readdirSync(src).filter((f) => /^chart-tooltip-.*\.json$/.test(f)).sort();
if (files.length === 0) {
  console.error(`no chart-tooltip-*.json in ${src}`);
  process.exit(1);
}
for (const f of files) {
  const item = JSON.parse(fs.readFileSync(path.join(src, f), "utf8"));
  const fixture = extract(item.name, item.files[0].content);
  fs.writeFileSync(path.join(out, `${item.name}.json`), JSON.stringify(fixture, null, 2) + "\n");
  console.log(`wrote ${item.name}.json`);
}
