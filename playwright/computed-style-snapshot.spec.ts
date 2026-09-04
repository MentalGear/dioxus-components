import { test } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

/**
 * Not a conformance test — a TOOL, run on demand to capture a computed-style
 * snapshot of every themed component page.
 *
 * Purpose (docs/backlog.md row 31b): the literal -> token migration is meant
 * to be value-PRESERVING, because row 31a derived its scales from the very
 * literals row 31b replaces. So the honest check is not "does it still look
 * plausible" but "did any computed value change at all". Capture before,
 * capture after, diff: an empty diff is the proof.
 *
 * Skipped unless DX_SNAPSHOT is set, so it never runs as part of the suite.
 */

const OUT = process.env.DX_SNAPSHOT_OUT ?? "/tmp/dx-computed.json";

const PROPERTIES = [
  "font-size", "font-weight", "line-height",
  "padding-top", "padding-right", "padding-bottom", "padding-left",
  "margin-top", "margin-right", "margin-bottom", "margin-left",
  "border-radius", "box-shadow", "border-width", "border-color",
  "transition-duration", "transition-property", "transition-timing-function",
  "z-index", "gap", "width", "height", "color", "background-color",
];

test.skip(!process.env.DX_SNAPSHOT, "snapshot tool; set DX_SNAPSHOT=1 to run");

test("capture computed styles for every themed component page", async ({ page }) => {
  test.setTimeout(20 * 60 * 1000);
  const base = path.join(__dirname, "../preview/src/components");
  const names = fs
    .readdirSync(base)
    .filter((n) => fs.existsSync(path.join(base, n, "style.css")))
    .sort();

  const all: Record<string, unknown> = {};
  for (const name of names) {
    await page.goto(`http://127.0.0.1:8080/component/?name=${name}&`, {
      waitUntil: "domcontentloaded",
    });
    // Let the wasm client render and its stylesheets attach.
    await page.waitForTimeout(1500);
    all[name] = await page.evaluate((props) => {
      const out: Array<Record<string, string>> = [];
      const els = Array.from(document.querySelectorAll('[class*="dx-"]'));
      for (const el of els) {
        const cs = getComputedStyle(el);
        const rec: Record<string, string> = {
          // Identity: tag + full class list + document order, all stable
          // across a pure CSS-value change.
          tag: el.tagName.toLowerCase(),
          cls: Array.from(el.classList).sort().join(" "),
        };
        for (const p of props) rec[p] = cs.getPropertyValue(p);
        out.push(rec);
      }
      return out;
    }, PROPERTIES);
  }
  fs.writeFileSync(OUT, JSON.stringify(all, null, 1));
  console.log(`wrote ${OUT} (${names.length} pages)`);
});
