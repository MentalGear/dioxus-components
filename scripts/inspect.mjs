#!/usr/bin/env node
// inspect.mjs — open one URL in a real (headless) Chromium and print enough
// to judge a change without a full Playwright spec run: an accessibility
// snapshot, a DOM outline, optionally a screenshot and/or one axe scan, with
// a few scripted interaction steps first if you need them.
//
// Uses `playwright` from playwright/node_modules directly (not the
// @playwright/test runner) -- no spec file, no test fixture, just a script.
//
// USAGE
//   node scripts/inspect.mjs <url> [options]
//
// OPTIONS
//   --variant <name>     Append `variant=<name>` to the URL's query string
//                         (this app's own `:variant` route param -- see
//                         preview/src/main.rs's `/component/block/` route).
//   --dark                Append `dark_mode=true` to the query string (this
//                         app's own `:dark_mode` route param, supported on
//                         `/`, `/docs`, `/demos`, `/component/`,
//                         `/component/block/`, `/dashboard/email-client`).
//   --viewport <WxH>     Default 1280x800.
//   --click <selector>   Click a selector. Repeatable; each --click/--press
//                         runs in the order given on the command line.
//   --press <key>        Press a key (Playwright key name, e.g. "Escape",
//                         "Tab", "ArrowDown"). Repeatable, same ordering.
//   --eval '<js>'         Run `<js>` in the page (as the body of an async
//                         function; use `return` to produce output) after
//                         all --click/--press steps, and print the result.
//   --axe                 Run one axe-core scan (same WCAG tag set as
//                         playwright/axe.ts) and print any violations.
//   --screenshot <path>   Save a full-page PNG to <path>.
//   --no-outline          Skip the DOM outline (printed by default).
//   --no-a11y             Skip the accessibility-tree snapshot (printed by
//                         default).
//   --root <selector>     Root for the a11y snapshot / DOM outline.
//                         Default: #main (this app's Dioxus mount point).
//   --timeout <ms>        First-paint wait timeout. Default 15000.
//
// READINESS SIGNAL
//   playwright/axe.ts itself defines no shared "app is ready" helper --
//   every spec in this repo waits on its own page-specific selector
//   (preview.spec.ts waits on `#hero`, component pages on a heading, etc).
//   Since inspect.mjs must work against an arbitrary URL, it generalizes
//   that pattern instead of picking one page's selector: it waits for the
//   Dioxus mount point (`#main`, from preview/index.html) to have at least
//   one child element, i.e. the app has actually painted something there,
//   not just that the HTML shell loaded. See dev-docs/dev-loop.md for the
//   measured cost of this wait against a warm server.
//
// EXAMPLES
//   node scripts/inspect.mjs http://127.0.0.1:8083/component/?name=kbd
//   node scripts/inspect.mjs http://127.0.0.1:8083/component/?name=kbd --dark --screenshot /tmp/kbd-dark.png
//   node scripts/inspect.mjs http://127.0.0.1:8083/component/?name=dialog \
//     --click 'button:has-text("Open")' --axe
//   node scripts/inspect.mjs http://127.0.0.1:8083/ \
//     --eval 'return document.querySelectorAll(".dx-component-card").length'

import { chromium } from "playwright";
import { fileURLToPath } from "node:url";
import path from "node:path";

const CHROMIUM_PATH = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"]; // mirrors playwright/axe.ts

function parseArgs(argv) {
  const opts = {
    url: null,
    variant: null,
    dark: false,
    viewport: "1280x800",
    actions: [], // ordered {type: "click"|"press", value}
    evalJs: null,
    axe: false,
    screenshot: null,
    outline: true,
    a11y: true,
    root: "#main",
    timeout: 15000,
  };
  const args = argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    switch (a) {
      case "--variant": opts.variant = args[++i]; break;
      case "--dark": opts.dark = true; break;
      case "--viewport": opts.viewport = args[++i]; break;
      case "--click": opts.actions.push({ type: "click", value: args[++i] }); break;
      case "--press": opts.actions.push({ type: "press", value: args[++i] }); break;
      case "--eval": opts.evalJs = args[++i]; break;
      case "--axe": opts.axe = true; break;
      case "--screenshot": opts.screenshot = args[++i]; break;
      case "--no-outline": opts.outline = false; break;
      case "--no-a11y": opts.a11y = false; break;
      case "--root": opts.root = args[++i]; break;
      case "--timeout": opts.timeout = Number(args[++i]); break;
      case "-h": case "--help":
        console.log(`Usage: node scripts/inspect.mjs <url> [options]\nSee the header comment in scripts/inspect.mjs for the full option list and examples.`);
        process.exit(0);
        break;
      default:
        if (!a.startsWith("--") && !opts.url) { opts.url = a; break; }
        console.error(`inspect.mjs: unknown argument: ${a}`);
        process.exit(2);
    }
  }
  if (!opts.url) {
    console.error("inspect.mjs: missing <url>. Run with --help.");
    process.exit(2);
  }
  return opts;
}

function buildUrl(opts) {
  const u = new URL(opts.url);
  if (opts.variant) u.searchParams.set("variant", opts.variant);
  if (opts.dark) u.searchParams.set("dark_mode", "true");
  return u.toString();
}

async function domOutline(page, rootSelector, { maxDepth = 6, maxNodes = 400 } = {}) {
  return page.evaluate(
    ({ rootSelector, maxDepth, maxNodes }) => {
      const root = document.querySelector(rootSelector);
      if (!root) return `(no element matched root selector "${rootSelector}")`;
      const lines = [];
      let count = 0;
      function describe(el) {
        const id = el.id ? `#${el.id}` : "";
        const cls = el.classList && el.classList.length ? `.${[...el.classList].slice(0, 2).join(".")}` : "";
        const role = el.getAttribute("role");
        const label = el.getAttribute("aria-label");
        let extra = "";
        if (role) extra += ` role=${role}`;
        if (label) extra += ` aria-label="${label}"`;
        return `${el.tagName.toLowerCase()}${id}${cls}${extra}`;
      }
      function walk(el, depth) {
        if (count >= maxNodes || depth > maxDepth) return;
        lines.push(`${"  ".repeat(depth)}${describe(el)}`);
        count++;
        for (const child of el.children) {
          if (count >= maxNodes) { lines.push(`${"  ".repeat(depth + 1)}… (truncated at ${maxNodes} nodes)`); return; }
          walk(child, depth + 1);
        }
      }
      walk(root, 0);
      return lines.join("\n");
    },
    { rootSelector, maxDepth, maxNodes },
  );
}

async function main() {
  const opts = parseArgs(process.argv);
  const targetUrl = buildUrl(opts);
  const [vw, vh] = opts.viewport.split("x").map(Number);
  const t0 = Date.now();

  const browser = await chromium.launch({ executablePath: CHROMIUM_PATH, args: ["--no-sandbox"] });
  try {
    // @axe-core/playwright's AxeBuilder requires a page created from an
    // explicit browser.newContext() (it errors "Please use
    // browser.newContext()" otherwise) -- use one unconditionally so --axe
    // works regardless of option order.
    const context = await browser.newContext({ viewport: { width: vw || 1280, height: vh || 800 } });
    const page = await context.newPage();
    console.log(`> goto ${targetUrl}`);
    await page.goto(targetUrl, { timeout: opts.timeout });

    const mount = opts.root === "#main" ? "#main" : opts.root;
    try {
      await page.waitForFunction(
        (sel) => {
          const el = document.querySelector(sel);
          return !!el && el.children.length > 0;
        },
        mount,
        { timeout: opts.timeout },
      );
    } catch {
      console.error(`! timed out waiting for "${mount}" to have rendered children (first paint) after ${opts.timeout}ms`);
    }
    const paintedAt = Date.now();
    console.log(`> first paint after ${paintedAt - t0}ms`);

    for (const action of opts.actions) {
      if (action.type === "click") {
        console.log(`> click ${action.value}`);
        await page.click(action.value, { timeout: opts.timeout });
      } else {
        console.log(`> press ${action.value}`);
        await page.keyboard.press(action.value);
      }
    }

    if (opts.evalJs) {
      console.log("--- eval result ---");
      try {
        const fn = new Function(`return (async () => { ${opts.evalJs} })()`);
        const result = await page.evaluate(fn);
        console.log(typeof result === "string" ? result : JSON.stringify(result, null, 2));
      } catch (err) {
        console.error(`eval error: ${err.message}`);
      }
    }

    if (opts.a11y) {
      console.log("--- accessibility snapshot ---");
      // Playwright 1.60 removed the legacy `page.accessibility.snapshot()`
      // API; `locator(...).ariaSnapshot()` (a YAML ARIA tree) is its
      // replacement. Fall back to the legacy API if it's ever present
      // (older Playwright), so this keeps working either way.
      if (typeof page.accessibility?.snapshot === "function") {
        const rootHandle = await page.$(opts.root);
        const snapshot = await page.accessibility.snapshot(
          rootHandle ? { root: rootHandle, interestingOnly: true } : { interestingOnly: true },
        );
        console.log(JSON.stringify(snapshot, null, 2));
      } else {
        const locator = page.locator(opts.root).first();
        console.log(await locator.ariaSnapshot());
      }
    }

    if (opts.outline) {
      console.log(`--- DOM outline (${opts.root}) ---`);
      console.log(await domOutline(page, opts.root));
    }

    if (opts.axe) {
      console.log("--- axe scan ---");
      const { default: AxeBuilder } = await import("@axe-core/playwright");
      const results = await new AxeBuilder({ page }).withTags(TAGS).analyze();
      if (results.violations.length === 0) {
        console.log("no violations");
      } else {
        for (const v of results.violations) {
          console.log(`[${v.id}] impact=${v.impact ?? "unknown"} — ${v.help}`);
          console.log(`  ${v.helpUrl}`);
          for (const node of v.nodes) {
            console.log(`  - target: ${node.target.join(" ")}`);
          }
        }
      }
    }

    if (opts.screenshot) {
      const out = path.resolve(opts.screenshot);
      await page.screenshot({ path: out, fullPage: true });
      console.log(`--- screenshot saved: ${out} ---`);
    }
  } finally {
    await browser.close();
  }
  console.log(`inspect.mjs: completed in ${Date.now() - t0}ms`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
