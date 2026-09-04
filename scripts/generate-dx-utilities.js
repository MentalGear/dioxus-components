#!/usr/bin/env node
//
// generate-dx-utilities.js
//
// Generates `preview/assets/dx-utilities.css` -- a small, static set of
// `dx-`-prefixed utility classes (spacing, gap, flex/grid basics, text
// sizes, radius, shadow) -- FROM the tokens defined in
// `preview/assets/dx-components-theme.css` (docs/backlog.md row 31a).
//
// Why generated rather than hand-written: a hand-written utility sheet
// drifts from the token file the moment either one is edited alone (the
// exact failure mode row 31a is trying to avoid -- see the backlog row's
// own wording, "produced by a script from the token file so it cannot
// drift"). This script instead reads the token file's own custom-property
// NAMES at generation time and emits one class per token found, each
// referencing that token with `var(--dx-...)` rather than a copied
// literal value -- so a class's rendered value always tracks whatever the
// token currently resolves to, in either theme.
//
// What this generates (and nothing else):
//   - Spacing: padding/margin, all sides + axis shorthands (x/y), off the
//     `--dx-space-*` scale, plus a hand-written `0` step (no token needed
//     for zero).
//   - Gap: `gap`/`row-gap`/`column-gap`, same `--dx-space-*` scale.
//   - Flex/grid basics: `display`, `flex-direction`, `flex-wrap`,
//     `align-items`, `justify-content`, and the three common `flex`
//     shorthands. These are NOT token-driven (there is no "flex-basics
//     scale" to survey) -- they're a small fixed keyword set, listed
//     explicitly in this script rather than derived from the CSS file.
//   - Text sizes: `font-size` off the `--dx-text-*` scale.
//   - Radius: `border-radius` off the `--dx-radius-*` scale.
//   - Shadow: `box-shadow` off the `--dx-shadow-*` scale, plus a
//     hand-written `none` step.
//
// What this deliberately does NOT generate (docs/backlog.md row 31a is
// explicit about this): no variants. No `hover:`, `md:`, `data-[state]:`,
// no responsive/state prefixes of any kind, and no JIT/on-demand
// scanning of markup for class usage -- every class below is emitted
// unconditionally, every run, which is what makes `--check` below
// meaningful. Variants stay hand-written in component `style.css` rules,
// same as before this round.
//
// Each generated rule is intentionally atomic (one selector, one
// declaration) -- composing e.g. `dx-flex dx-flex-row dx-items-center`
// in markup, rather than one class setting three properties. Two
// reasons: it keeps every rule immune to `stylelint-config-idiomatic-
// order`'s `order/properties-order` rule (nothing to order when there is
// only one declaration), and it is the standard utility-CSS shape this
// kind of sheet is expected to have.
//
// Usage:
//   node scripts/generate-dx-utilities.js          Write/overwrite the file.
//   node scripts/generate-dx-utilities.js --check   Exit 1 if the committed
//                                                    file is stale (does not
//                                                    write).
//
// Idempotent: running it twice in a row with an unchanged token file
// produces byte-identical output (stable insertion-order iteration, no
// timestamps, no randomness).

"use strict";

const fs = require("fs");
const path = require("path");

const REPO_ROOT = path.resolve(__dirname, "..");
const TOKEN_FILE = path.join(
  REPO_ROOT,
  "preview/assets/dx-components-theme.css",
);
const OUTPUT_FILE = path.join(REPO_ROOT, "preview/assets/dx-utilities.css");

// ---------------------------------------------------------------------------
// 1. Read the token names out of the theme file.
// ---------------------------------------------------------------------------
//
// Matches `--dx-space-2:`, `--dx-text-xs:`, `--dx-radius-lg:`,
// `--dx-shadow-md:` and so on, anywhere in the file (there is exactly one
// `:root` block that defines these, so no need to scope the regex to it).
// Other `--dx-*` tokens this file also defines (`--dx-opacity-hover`,
// `--dx-ring`, `--dx-motion-*`, `--dx-breakpoint-*`, `--dx-z-*`) simply
// don't match this prefix set and are correctly left out -- none of them
// are part of the four scales row 31a asks this generator to cover.
function readTokenSteps(prefix) {
  const css = fs.readFileSync(TOKEN_FILE, "utf8");
  const re = new RegExp(`--dx-${prefix}-([a-z0-9]+)\\s*:`, "g");
  const steps = [];
  const seen = new Set();
  let m;
  while ((m = re.exec(css))) {
    if (!seen.has(m[1])) {
      seen.add(m[1]);
      steps.push(m[1]);
    }
  }
  if (steps.length === 0) {
    throw new Error(
      `No --dx-${prefix}-* tokens found in ${TOKEN_FILE} -- has the token file moved or been renamed?`,
    );
  }
  return steps;
}

const spaceSteps = readTokenSteps("space"); // e.g. ["1","2","3",...,"32"]
const textSteps = readTokenSteps("text"); // e.g. ["xs","sm","base",...]
const radiusSteps = readTokenSteps("radius"); // e.g. ["xs","sm",...,"full"]
const shadowSteps = readTokenSteps("shadow"); // e.g. ["sm","md",...,"2xl"]

// ---------------------------------------------------------------------------
// 2. Build the rules.
// ---------------------------------------------------------------------------

const rules = []; // { comment?, selector, prop, value }[]

function section(title) {
  rules.push({ sectionTitle: title });
}

// --- Spacing: padding / margin --------------------------------------------
section("Spacing -- padding");
const paddingSides = [
  ["p", "padding"],
  ["pt", "padding-top"],
  ["pr", "padding-right"],
  ["pb", "padding-bottom"],
  ["pl", "padding-left"],
];
for (const [cls, prop] of paddingSides) {
  rules.push({ selector: `dx-${cls}-0`, prop, value: "0" });
  for (const step of spaceSteps) {
    rules.push({
      selector: `dx-${cls}-${step}`,
      prop,
      value: `var(--dx-space-${step})`,
    });
  }
}
rules.push({ selector: "dx-px-0", prop: "padding-inline", value: "0" });
rules.push({ selector: "dx-py-0", prop: "padding-block", value: "0" });
for (const step of spaceSteps) {
  rules.push({
    selector: `dx-px-${step}`,
    prop: "padding-inline",
    value: `var(--dx-space-${step})`,
  });
  rules.push({
    selector: `dx-py-${step}`,
    prop: "padding-block",
    value: `var(--dx-space-${step})`,
  });
}

section("Spacing -- margin");
const marginSides = [
  ["m", "margin"],
  ["mt", "margin-top"],
  ["mr", "margin-right"],
  ["mb", "margin-bottom"],
  ["ml", "margin-left"],
];
for (const [cls, prop] of marginSides) {
  rules.push({ selector: `dx-${cls}-0`, prop, value: "0" });
  for (const step of spaceSteps) {
    rules.push({
      selector: `dx-${cls}-${step}`,
      prop,
      value: `var(--dx-space-${step})`,
    });
  }
}
rules.push({ selector: "dx-mx-0", prop: "margin-inline", value: "0" });
rules.push({ selector: "dx-my-0", prop: "margin-block", value: "0" });
for (const step of spaceSteps) {
  rules.push({
    selector: `dx-mx-${step}`,
    prop: "margin-inline",
    value: `var(--dx-space-${step})`,
  });
  rules.push({
    selector: `dx-my-${step}`,
    prop: "margin-block",
    value: `var(--dx-space-${step})`,
  });
}

// --- Gap --------------------------------------------------------------
section("Gap");
rules.push({ selector: "dx-gap-0", prop: "gap", value: "0" });
rules.push({ selector: "dx-gap-x-0", prop: "column-gap", value: "0" });
rules.push({ selector: "dx-gap-y-0", prop: "row-gap", value: "0" });
for (const step of spaceSteps) {
  rules.push({
    selector: `dx-gap-${step}`,
    prop: "gap",
    value: `var(--dx-space-${step})`,
  });
  rules.push({
    selector: `dx-gap-x-${step}`,
    prop: "column-gap",
    value: `var(--dx-space-${step})`,
  });
  rules.push({
    selector: `dx-gap-y-${step}`,
    prop: "row-gap",
    value: `var(--dx-space-${step})`,
  });
}

// --- Flex/grid basics --------------------------------------------------
// Not token-driven -- see the file header comment. Small, fixed, atomic.
section("Flex/grid basics (fixed keyword set, not token-driven)");
const displayUtils = [
  ["dx-block", "display", "block"],
  ["dx-inline-block", "display", "inline-block"],
  ["dx-flex", "display", "flex"],
  ["dx-inline-flex", "display", "inline-flex"],
  ["dx-grid", "display", "grid"],
  ["dx-inline-grid", "display", "inline-grid"],
  ["dx-hidden", "display", "none"],
];
const flexUtils = [
  ["dx-flex-row", "flex-direction", "row"],
  ["dx-flex-row-reverse", "flex-direction", "row-reverse"],
  ["dx-flex-col", "flex-direction", "column"],
  ["dx-flex-col-reverse", "flex-direction", "column-reverse"],
  ["dx-flex-wrap", "flex-wrap", "wrap"],
  ["dx-flex-nowrap", "flex-wrap", "nowrap"],
  ["dx-items-start", "align-items", "flex-start"],
  ["dx-items-center", "align-items", "center"],
  ["dx-items-end", "align-items", "flex-end"],
  ["dx-items-stretch", "align-items", "stretch"],
  ["dx-justify-start", "justify-content", "flex-start"],
  ["dx-justify-center", "justify-content", "center"],
  ["dx-justify-end", "justify-content", "flex-end"],
  ["dx-justify-between", "justify-content", "space-between"],
  ["dx-flex-1", "flex", "1 1 0%"],
  ["dx-flex-auto", "flex", "1 1 auto"],
  ["dx-flex-none", "flex", "none"],
];
for (const [selector, prop, value] of [...displayUtils, ...flexUtils]) {
  rules.push({ selector, prop, value });
}

// --- Text sizes ----------------------------------------------------------
section("Text sizes");
for (const step of textSteps) {
  rules.push({
    selector: `dx-text-${step}`,
    prop: "font-size",
    value: `var(--dx-text-${step})`,
  });
}

// --- Radius ----------------------------------------------------------------
section("Radius");
for (const step of radiusSteps) {
  rules.push({
    selector: `dx-radius-${step}`,
    prop: "border-radius",
    value: `var(--dx-radius-${step})`,
  });
}

// --- Shadow ------------------------------------------------------------
section("Shadow");
rules.push({ selector: "dx-shadow-none", prop: "box-shadow", value: "none" });
for (const step of shadowSteps) {
  rules.push({
    selector: `dx-shadow-${step}`,
    prop: "box-shadow",
    value: `var(--dx-shadow-${step})`,
  });
}

// ---------------------------------------------------------------------------
// 3. Render.
// ---------------------------------------------------------------------------

const HEADER = `/* GENERATED FILE -- DO NOT EDIT BY HAND.
 *
 * Produced by \`node scripts/generate-dx-utilities.js\` from the token
 * declarations in \`preview/assets/dx-components-theme.css\`
 * (docs/backlog.md row 31a). Re-run that script to regenerate after
 * changing the token file; \`node scripts/generate-dx-utilities.js
 * --check\` fails if this file is stale.
 *
 * A small, static set of \`dx-\`-prefixed utility classes -- spacing,
 * gap, flex/grid basics, text sizes, radius, shadow -- for point-of-use
 * composition (docs/demo layouts, consumer tweaks) without pulling in a
 * JIT engine. This file is entirely optional: no themed component
 * depends on it, and it ships separately from
 * \`dx-components-theme.css\` so importing the theme never pulls these
 * classes in unasked.
 *
 * No variants live here on purpose (no \`hover:\`, \`md:\`,
 * \`data-[state]:\`, ...) -- those stay hand-written in each component's
 * own \`style.css\`. Every class below is a single selector with a
 * single declaration; compose several in markup
 * (\`class="dx-flex dx-items-center dx-gap-2"\`) rather than expecting
 * one class to do several things.
 *
 * Import this file only if you use these classes -- it is not pulled in
 * by \`dx-components-theme.css\` itself. If a consumer's \`dx components
 * add\` copy doesn't want it, don't add the \`<link>\`.
 */
`;

function renderRule(rule) {
  return `\n.${rule.selector} {\n  ${rule.prop}: ${rule.value};\n}\n`;
}

let out = HEADER;
for (const rule of rules) {
  if (rule.sectionTitle) {
    out += `\n/* ${rule.sectionTitle} */\n`;
    continue;
  }
  out += renderRule(rule);
}

// ---------------------------------------------------------------------------
// 4. Write, or check.
// ---------------------------------------------------------------------------

const checkMode = process.argv.includes("--check");

if (checkMode) {
  const existing = fs.existsSync(OUTPUT_FILE)
    ? fs.readFileSync(OUTPUT_FILE, "utf8")
    : null;
  if (existing === out) {
    console.log(
      `OK: ${path.relative(REPO_ROOT, OUTPUT_FILE)} is up to date (${rules.filter((r) => !r.sectionTitle).length} rules).`,
    );
    process.exit(0);
  }
  console.error(
    existing === null
      ? `STALE: ${path.relative(REPO_ROOT, OUTPUT_FILE)} does not exist yet.`
      : `STALE: ${path.relative(REPO_ROOT, OUTPUT_FILE)} does not match what the token file generates.`,
  );
  console.error(
    "Run `node scripts/generate-dx-utilities.js` and commit the result.",
  );
  process.exit(1);
} else {
  fs.writeFileSync(OUTPUT_FILE, out);
  console.log(
    `Wrote ${path.relative(REPO_ROOT, OUTPUT_FILE)} (${rules.filter((r) => !r.sectionTitle).length} rules, from ${spaceSteps.length} spacing + ${textSteps.length} text + ${radiusSteps.length} radius + ${shadowSteps.length} shadow tokens).`,
  );
}
