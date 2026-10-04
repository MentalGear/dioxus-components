import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/**
 * Contrast sweep over the docs-site theme picker (`preview/assets/theme-presets.css`): every base x every
 * accent x light/dark, measured from COMPUTED colours on the real components, so a preset that makes a
 * component unreadable cannot ship unnoticed.
 *
 * What it asserts, per combination:
 *   - text vs its own effective background >= 4.5:1 (WCAG AA, normal text; every one of these is 12-14px
 *     text, none is "large") for the Badge variants, the Button variants (rest AND hover), the link-variant
 *     Button, the Toggle / ToggleGroup states, and the docs site's own accent ink (SHOWCASE eyebrow, the `$`
 *     in the CLI chip, a component page's eyebrow);
 *   - a pressed toggle's background differs from its hovered and its resting background.
 *
 * Method. The base/accent names are READ FROM `theme-presets.css` (so a new preset is swept automatically),
 * plus "none" for each axis (the library's own look). Each combination is applied by setting
 * `data-theme-base` / `data-theme-accent` / `data-theme` on <html> (what the picker and the pre-paint snippet
 * set), transitions are switched off so `getComputedStyle` reads the settled value, and the colour pair is
 * resolved through a 1x1 canvas: the background is composited layer by layer from <html> down to the element
 * (a `color-mix(..., transparent)` fill is translucent, so the page behind it is part of the answer).
 *
 * Run against a dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=baseline.local.config.ts ./theme-preset-contrast.spec.ts
 */

const AA = 4.5;
/**
 * Smallest per-channel difference (0-255) that counts as "a different background". The pressed state is the
 * one that has to be unmistakable, so it gets a wide margin; hover vs rest is the ramp's own faint hover
 * surface (#f8f8f8 on white is 7 apart) and only has to be a visible step.
 */
const DISTINCT_PRESSED = 16;
const DISTINCT_HOVER = 6;

const PRESETS_CSS = fs.readFileSync(
  path.join(__dirname, "..", "preview", "assets", "theme-presets.css"),
  "utf8",
);

function presetNames(axis: "base" | "accent"): string[] {
  const names = [...PRESETS_CSS.matchAll(new RegExp(`\\[data-theme-${axis}="([a-z]+)"\\] \\{`, "g"))].map((m) => m[1]);
  return [...new Set(names)];
}

const BASES = presetNames("base");
const ACCENTS = presetNames("accent");

type RGB = [number, number, number];
type Probe = {
  /** Label used in failure messages. */
  name: string;
  /** CSS selector of the element whose text colour and background are measured. */
  selector: string;
  /** If set, the first match whose trimmed text equals this (several demo buttons share a selector). */
  text?: string;
};
type Row = {
  mode: "light" | "dark";
  base: string;
  accent: string;
  probe: string;
  ratio: number;
  fg: RGB;
  bg: RGB;
};

/**
 * Applies every base x accent x mode combination in turn and measures each probe. One `evaluate` per call:
 * attribute writes plus a forced style read are synchronous once transitions are off, so a whole sweep costs
 * milliseconds and nothing can repaint mid-measure.
 */
async function sweep(page: Page, probes: Probe[], mode: "light" | "dark"): Promise<Row[]> {
  return page.evaluate(
    ({ probes, mode, bases, accents }) => {
      const kill = document.createElement("style");
      kill.textContent = "*,*::before,*::after{transition:none!important;animation:none!important}";
      document.head.appendChild(kill);

      const cv = document.createElement("canvas");
      cv.width = cv.height = 1;
      const cx = cv.getContext("2d", { willReadFrequently: true })!;
      const rgbaOver = (under: RGB | null, css: string): RGB => {
        cx.clearRect(0, 0, 1, 1);
        if (under) {
          cx.fillStyle = `rgb(${under[0]} ${under[1]} ${under[2]})`;
          cx.fillRect(0, 0, 1, 1);
        }
        cx.fillStyle = css;
        cx.fillRect(0, 0, 1, 1);
        const d = cx.getImageData(0, 0, 1, 1).data;
        return [d[0], d[1], d[2]];
      };
      const lum = ([r, g, b]: RGB) => {
        const f = (v: number) => {
          const c = v / 255;
          return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        };
        return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
      };
      const ratio = (a: RGB, b: RGB) => {
        const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
        return (hi + 0.05) / (lo + 0.05);
      };
      const find = (p: { selector: string; text?: string }) =>
        [...document.querySelectorAll(p.selector)].find((el) => !p.text || el.textContent?.trim() === p.text) ?? null;

      const html = document.documentElement;
      html.setAttribute("data-theme", mode);
      const out: Row[] = [];
      for (const base of ["none", ...bases]) {
        for (const accent of ["none", ...accents]) {
          if (base === "none") html.removeAttribute("data-theme-base");
          else html.setAttribute("data-theme-base", base);
          if (accent === "none") html.removeAttribute("data-theme-accent");
          else html.setAttribute("data-theme-accent", accent);
          for (const p of probes) {
            const el = find(p);
            if (!el) throw new Error(`probe "${p.name}" matched nothing (${p.selector}${p.text ? ` / "${p.text}"` : ""})`);
            // Effective background: composite every ancestor's background-color from <html> down.
            const chain: Element[] = [];
            for (let n: Element | null = el; n; n = n.parentElement) chain.unshift(n);
            let bg: RGB = [255, 255, 255];
            for (const n of chain) bg = rgbaOver(bg, getComputedStyle(n).backgroundColor);
            const fg = rgbaOver(bg, getComputedStyle(el).color);
            out.push({ mode, base, accent, probe: p.name, ratio: ratio(fg, bg), fg, bg });
          }
        }
      }
      return out;
    },
    { probes, mode, bases: BASES, accents: ACCENTS },
  );
}

const rgb = (c: RGB) => `rgb(${c.join(",")})`;
const combo = (r: Row) => `base=${r.base} accent=${r.accent}`;

/** One line per probe that has any combination under `min`, worst case first. Empty when everything passes. */
function failures(rows: Row[], min = AA): string[] {
  const byProbe = new Map<string, Row[]>();
  for (const r of rows) byProbe.set(r.probe, [...(byProbe.get(r.probe) ?? []), r]);
  const lines: string[] = [];
  for (const [probe, list] of byProbe) {
    const bad = list.filter((r) => r.ratio < min);
    if (!bad.length) continue;
    const worst = bad.reduce((a, b) => (b.ratio < a.ratio ? b : a));
    lines.push(
      `${probe} [${worst.mode}]: ${bad.length}/${list.length} combinations < ${min}:1; worst ${worst.ratio.toFixed(2)}:1 ` +
        `at ${combo(worst)} (text ${rgb(worst.fg)} on ${rgb(worst.bg)})`,
    );
  }
  return lines;
}

/** Surfaces the worst ratio per probe even on a pass, so the sweep's margin is visible in the run log. */
function logWorst(title: string, rows: Row[]) {
  const worst = new Map<string, Row>();
  for (const r of rows) if (!worst.has(r.probe) || r.ratio < worst.get(r.probe)!.ratio) worst.set(r.probe, r);
  console.log(
    `[contrast] ${title}: ` +
      [...worst.values()].map((r) => `${r.probe} ${r.ratio.toFixed(2)}:1 (${combo(r)})`).join("; "),
  );
}

const open = async (page: Page, route: string, ready: string) => {
  await gotoHydrated(page, `${BASE_URL}${route}`);
  await expect(page.locator(ready).first()).toBeVisible();
};

test("the sweep covers every base and accent the stylesheet defines", () => {
  expect(BASES).toHaveLength(9);
  expect(ACCENTS).toHaveLength(17);
});

for (const mode of ["light", "dark"] as const) {
  test.describe(`${mode} mode`, () => {
    test("Badge variants: text >= 4.5:1 under every preset", async ({ page }) => {
      await open(page, "/component/badge/", ".dx-badge");
      const rows = await sweep(
        page,
        [
          ...["primary", "secondary", "destructive", "outline"].map((v) => ({
            name: `badge ${v}`,
            selector: `.dx-badge[data-style="${v}"]`,
            text: v[0].toUpperCase() + v.slice(1),
          })),
          // The demo's "Verified" badge: an inline `--dx-ring-color` fill with white ink, which was 3.60:1
          // in dark mode before its fill was clamped.
          { name: "badge verified", selector: ".dx-badge", text: "Verified" },
        ],
        mode,
      );
      logWorst(`badge ${mode}`, rows);
      expect(failures(rows)).toEqual([]);
    });

    test("Button variants at rest: text >= 4.5:1 under every preset", async ({ page }) => {
      await open(page, "/component/button/", ".dx-button");
      const rows = await sweep(
        page,
        [
          ["default", "Primary"],
          ["secondary", "Secondary"],
          ["outline", "Outline"],
          ["ghost", "Ghost"],
          ["link", "Link"],
          ["destructive", "Destructive"],
        ].map(([v, text]) => ({ name: `button ${v}`, selector: ".dx-button", text })),
        mode,
      );
      logWorst(`button rest ${mode}`, rows);
      expect(failures(rows)).toEqual([]);
    });

    for (const [variant, text] of [
      ["default", "Primary"],
      ["secondary", "Secondary"],
      ["outline", "Outline"],
      ["ghost", "Ghost"],
      ["destructive", "Destructive"],
    ]) {
      const hoverContrast = async ({ page }: { page: Page }) => {
        await open(page, "/component/button/", ".dx-button");
        const button = page.locator(".dx-button", { hasText: new RegExp(`^${text}$`) }).first();
        await button.hover();
        expect(await button.evaluate((el) => el.matches(":hover"))).toBe(true);
        const rows = await sweep(page, [{ name: `button ${variant} hover`, selector: ".dx-button", text }], mode);
        logWorst(`button ${variant} hover ${mode}`, rows);
        expect(failures(rows)).toEqual([]);
      };
      test(`Button ${variant} while hovered: text >= 4.5:1 under every preset`, hoverContrast);
    }

    test("Button default hover under an accent moves the fill away from the text, visibly", async ({ page }) => {
      // Nova's `hover:bg-primary/80` lets the page show through and so moves the fill TOWARD the text
      // colour, which is fine for the library's own black button and for a base colour alone (both keep
      // it, see the contrast sweep above) but not for an accent's mid-tone fill that clears 4.5:1 by a
      // hair at rest. Under an accent the hover must go the other way: darker under light text, lighter
      // under dark text. Contrast can then only rise, and the step has to be one a pointer user can see.
      await open(page, "/component/button/", ".dx-button");
      const button = page.locator(".dx-button", { hasText: /^Primary$/ }).first();
      const probe: Probe[] = [{ name: "button default", selector: ".dx-button", text: "Primary" }];
      await page.mouse.move(0, 0);
      const rest = await sweep(page, probe, mode);
      await button.hover();
      expect(await button.evaluate((el) => el.matches(":hover"))).toBe(true);
      const hover = await sweep(page, probe, mode);

      const lum = ([r, g, b]: RGB) => {
        const f = (v: number) => {
          const c = v / 255;
          return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        };
        return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
      };
      const wrong: string[] = [];
      let smallest = { gap: Infinity, at: "" };
      let worstRatio = { ratio: Infinity, at: "" };
      rest.forEach((r, i) => {
        if (r.accent === "none") return; // no accent: Nova's `/80`, asserted by the sweep above
        const h = hover[i];
        const lightInk = lum(r.fg) > lum(r.bg);
        const movedAway = lightInk ? lum(h.bg) < lum(r.bg) : lum(h.bg) > lum(r.bg);
        const gap = Math.max(...r.bg.map((v, k) => Math.abs(v - h.bg[k])));
        if (gap < smallest.gap) smallest = { gap, at: `${combo(r)} [${mode}]` };
        if (h.ratio < worstRatio.ratio) worstRatio = { ratio: h.ratio, at: `${combo(r)} [${mode}]` };
        if (!movedAway) wrong.push(`${combo(r)} [${mode}]: ${lightInk ? "light" : "dark"} ink, fill ${rgb(r.bg)} -> ${rgb(h.bg)} moved toward it`);
        if (gap < DISTINCT_HOVER) wrong.push(`${combo(r)} [${mode}]: hover ${rgb(h.bg)} vs rest ${rgb(r.bg)} differ by only ${gap}/255`);
        if (h.ratio < r.ratio - 0.001) wrong.push(`${combo(r)} [${mode}]: hover contrast ${h.ratio.toFixed(2)} fell below rest ${r.ratio.toFixed(2)}`);
      });
      console.log(
        `[contrast] button default hover (accent) ${mode}: smallest rest->hover step ${smallest.gap}/255 (${smallest.at}); ` +
          `worst hover contrast ${worstRatio.ratio.toFixed(2)}:1 (${worstRatio.at})`,
      );
      expect(wrong.slice(0, 12)).toEqual([]);
      // And with no accent chosen the library's own look is untouched: Nova's `/80` still changes the fill.
      const none = rest.filter((r) => r.accent === "none" && r.base === "none");
      none.forEach((r) => {
        const h = hover[rest.indexOf(r)];
        expect(h.bg, `no preset [${mode}]: hover differs from rest`).not.toEqual(r.bg);
      });
    });

    test("Toggle: off / hovered / pressed text >= 4.5:1, and the three backgrounds differ", async ({ page }) => {
      await open(page, "/component/toggle/", ".dx-toggle");
      const toggle = page.locator(".dx-toggle", { hasText: /^B$/ });
      const probe: Probe[] = [{ name: "toggle", selector: ".dx-toggle", text: "B" }];
      const away = () => page.mouse.move(0, 0);

      await away();
      await expect(toggle).toHaveAttribute("data-state", "off");
      const off = await sweep(page, [{ ...probe[0], name: "toggle off" }], mode);
      await toggle.hover();
      const hover = await sweep(page, [{ ...probe[0], name: "toggle hover" }], mode);
      await toggle.click();
      await expect(toggle).toHaveAttribute("data-state", "on");
      await away();
      const on = await sweep(page, [{ ...probe[0], name: "toggle on" }], mode);
      await toggle.hover();
      const onHover = await sweep(page, [{ ...probe[0], name: "toggle on+hover" }], mode);

      const all = [...off, ...hover, ...on, ...onHover];
      logWorst(`toggle ${mode}`, all);
      expect(failures(all)).toEqual([]);
      expect(distinctnessFailures(off, hover, on)).toEqual([]);
      // A pressed toggle keeps its pressed background under the pointer (row 89), in every preset.
      expect(sameBackground(on, onHover)).toEqual([]);
    });

    test("ToggleGroup: off / hovered / pressed text >= 4.5:1, and the three backgrounds differ", async ({ page }) => {
      await open(page, "/component/toggle_group/", ".dx-toggle-group-item");
      const item = (t: string) => page.locator(".dx-toggle-group-item", { hasText: new RegExp(`^${t}$`) }).first();
      const probe = (name: string, text: string): Probe => ({ name, selector: ".dx-toggle-group-item", text });

      await item("B").click();
      await expect(item("B")).toHaveAttribute("data-state", "on");
      await item("I").hover();
      expect(await item("I").evaluate((el) => el.matches(":hover"))).toBe(true);
      // B is pressed, I is hovered, U is neither: all three states in one frame, so one sweep.
      const rows = await sweep(
        page,
        [probe("group on", "B"), probe("group hover", "I"), probe("group off", "U")],
        mode,
      );
      logWorst(`toggle group ${mode}`, rows);
      expect(failures(rows)).toEqual([]);
      const pick = (name: string) => rows.filter((r) => r.probe === name);
      expect(distinctnessFailures(pick("group off"), pick("group hover"), pick("group on"))).toEqual([]);
    });
  });
}

/** Combinations in which two of the three state backgrounds are too close (see `DISTINCT_*`). */
function distinctnessFailures(off: Row[], hover: Row[], on: Row[]): string[] {
  const d = (a: RGB, b: RGB) => Math.max(...a.map((v, i) => Math.abs(v - b[i])));
  const bad: string[] = [];
  const smallest: Record<string, { gap: number; at: string }> = {};
  off.forEach((o, i) => {
    const pairs: [string, RGB, RGB, number][] = [
      ["on/hover", on[i].bg, hover[i].bg, DISTINCT_PRESSED],
      ["on/off", on[i].bg, o.bg, DISTINCT_PRESSED],
      ["hover/off", hover[i].bg, o.bg, DISTINCT_HOVER],
    ];
    for (const [label, a, b, min] of pairs) {
      const gap = d(a, b);
      if (!smallest[label] || gap < smallest[label].gap) smallest[label] = { gap, at: `${combo(o)} [${o.mode}]` };
      if (gap < min) bad.push(`${label} backgrounds ${rgb(a)} vs ${rgb(b)} (gap ${gap} < ${min}) at ${combo(o)} [${o.mode}]`);
    }
  });
  console.log(
    "[contrast] toggle state backgrounds, smallest gap /255: " +
      Object.entries(smallest)
        .map(([k, v]) => `${k} ${v.gap} (${v.at})`)
        .join("; "),
  );
  return bad.slice(0, 12);
}

function sameBackground(a: Row[], b: Row[]): string[] {
  const bad: string[] = [];
  a.forEach((r, i) => {
    if (r.bg.some((v, k) => Math.abs(v - b[i].bg[k]) > 1)) bad.push(`${combo(r)} [${r.mode}]: ${rgb(r.bg)} -> ${rgb(b[i].bg)}`);
  });
  return bad.slice(0, 12);
}

/**
 * The docs site's own accent ink (`--highlight-color-tertiary`: the SHOWCASE eyebrow, the `$` in the CLI chip,
 * prose links) follows an accent preset, stays readable on every surface it sits on, and is exactly today's
 * blue when no accent is chosen.
 */
test.describe("docs-site highlight ink", () => {
  for (const mode of ["light", "dark"] as const) {
    test(`home page accents: text >= 4.5:1 under every preset (${mode})`, async ({ page }) => {
      await open(page, "/", ".dx-hero-prompt");
      const rows = await sweep(
        page,
        [
          { name: "eyebrow SHOWCASE", selector: ".dx-section-eyebrow", text: "Showcase" },
          { name: "eyebrow CATALOG", selector: ".dx-section-eyebrow", text: "Catalog" },
          { name: "cli chip $", selector: ".dx-hero-prompt", text: "$" },
        ],
        mode,
      );
      logWorst(`home ${mode}`, rows);
      expect(failures(rows)).toEqual([]);
    });

  }

  test("an accent preset recolours the highlight; no accent keeps today's blue", async ({ page }) => {
    await open(page, "/", ".dx-hero-prompt");
    const ink = () =>
      page.evaluate(() => {
        const el = document.querySelector(".dx-hero-prompt")!;
        const cv = document.createElement("canvas");
        cv.width = cv.height = 1;
        const cx = cv.getContext("2d", { willReadFrequently: true })!;
        cx.fillStyle = getComputedStyle(el).color;
        cx.fillRect(0, 0, 1, 1);
        return Array.from(cx.getImageData(0, 0, 1, 1).data.slice(0, 3));
      });
    const set = (attrs: Record<string, string | null>) =>
      page.evaluate((a) => {
        document.head.insertAdjacentHTML("beforeend", "<style>*{transition:none!important}</style>");
        for (const [k, v] of Object.entries(a)) v === null ? document.documentElement.removeAttribute(k) : document.documentElement.setAttribute(k, v);
      }, attrs);

    for (const [mode, today] of [["light", [0, 101, 255]], ["dark", [77, 148, 255]]] as const) {
      await set({ "data-theme": mode, "data-theme-accent": null, "data-theme-base": null });
      expect(await ink(), `${mode}: no preset`).toEqual(today);
      // A base colour alone is not an accent choice: the docs highlight stays blue.
      await set({ "data-theme-base": "zinc" });
      expect(await ink(), `${mode}: base only`).toEqual(today);
      await set({ "data-theme-accent": "orange" });
      const orange = await ink();
      expect(orange, `${mode}: orange accent`).not.toEqual(today);
      expect(orange[0], `${mode}: orange is warm (red channel above blue)`).toBeGreaterThan(orange[2]);
      await set({ "data-theme-accent": "violet", "data-theme-base": null });
      const violet = await ink();
      expect(violet, `${mode}: violet differs from orange`).not.toEqual(orange);
    }
  });
});
