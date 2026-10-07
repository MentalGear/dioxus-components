import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=native_select&`, { timeout: 20 * 60 * 1000 });
  const select = page.locator("#component-preview-frame").first().getByRole("combobox");
  await expect(select).toHaveValue("apple");
  await select.selectOption("banana");
  await expect(select).toHaveValue("banana");
  await expect(page.getByText("Selected: banana")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Native Select has no overlay/expand/select interaction -- it's a plain
  // native <select>, one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=native_select&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "native_select: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// Native controls follow the mode (`color-scheme`, `preview/assets/dx-components-theme.css`).
//
// Before this, the theme's `--dark`/`--light` toggle reached only the colours WE declare: a native
// `<select>`'s popup, the page's scrollbars and every other UA-drawn part stayed light in dark mode, and
// the Native Select's own translucent dark fill (`--dx-input` at 30%) landed on a LIGHT popup, leaving
// near-white option text on a near-white list. The root now carries `color-scheme`, so the UA draws its
// parts in the active scheme, and the options carry the popover's opaque surface and ink.
//
// Three ways to be dark are covered, because they are three code paths: the cookie the docs-site theme
// toggle writes (`dx_theme` -> `<html data-theme>`, set before first paint), the OS preference with no
// choice made (`prefers-color-scheme`, the no-JS path), and a cookie that overrides the OS.
const SELECT = "#component-preview-frame .dx-native-select";
const ROUTE = `${BASE_URL}/component/?name=native_select&`;

type Mode = "light" | "dark";

async function openThemed(page: Page, opts: { os: Mode; cookie?: Mode; base?: string }) {
  await page.emulateMedia({ colorScheme: opts.os });
  const cookies = [];
  if (opts.cookie) cookies.push({ name: "dx_theme", value: opts.cookie, url: BASE_URL });
  if (opts.base) cookies.push({ name: "dx_theme_base", value: opts.base, url: BASE_URL });
  if (cookies.length) await page.context().addCookies(cookies);
  await page.goto(ROUTE, { timeout: 20 * 60 * 1000 });
  await expect(page.locator(SELECT).first()).toBeVisible();
}

/**
 * Everything the assertions compare, read in one round trip. Each "expected" is a probe element styled with
 * the TOKEN expression the stylesheet is supposed to resolve to, so the check follows the tokens (presets
 * included) instead of restating colour numbers. `ua` is the UA's own system colours on an unstyled element:
 * `Canvas`/`CanvasText` are the page colours the browser picks for the USED color scheme, so they say which
 * scheme is really in force whatever the computed `color-scheme` string is (`light dark` is a legal value).
 */
async function probe(page: Page) {
  return page.evaluate((selector) => {
    const select = document.querySelector(selector) as HTMLSelectElement;
    const option = select.querySelector("option") as HTMLOptionElement;
    const scoped = document.createElement("div");
    scoped.style.cssText = "position:absolute;visibility:hidden";
    select.parentElement!.append(scoped);
    const resolve = (prop: "backgroundColor" | "color", css: string) => {
      scoped.style[prop] = "";
      scoped.style[prop] = css;
      return getComputedStyle(scoped)[prop];
    };
    const lum = (rgb: string) => {
      const m = /rgba?\(([^)]+)\)/.exec(rgb);
      if (!m) throw new Error(`not an rgb colour: ${rgb}`);
      const [r, g, b] = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
      return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
    };
    const root = getComputedStyle(document.documentElement);
    const own = getComputedStyle(select);
    const opt = getComputedStyle(option);
    const ua = { canvas: resolve("backgroundColor", "Canvas"), text: resolve("color", "CanvasText") };
    // An <optgroup>/<option> a caller never styled: the library's rule must still reach it.
    const out = {
      dataTheme: document.documentElement.dataset.theme ?? null,
      rootScheme: root.colorScheme,
      selectScheme: own.colorScheme,
      uaCanvasLum: lum(ua.canvas),
      uaTextLum: lum(ua.text),
      select: {
        bg: own.backgroundColor,
        color: own.color,
        expectBgLight: resolve("backgroundColor", "var(--dx-background)"),
        expectBgDark: resolve("backgroundColor", "color-mix(in oklab, var(--dx-input) 30%, transparent)"),
        expectColor: resolve("color", "var(--dx-foreground)"),
      },
      option: {
        bg: opt.backgroundColor,
        color: opt.color,
        expectBg: resolve("backgroundColor", "var(--dx-popover)"),
        expectColor: resolve("color", "var(--dx-popover-foreground)"),
      },
    };
    scoped.remove();
    return out;
  }, SELECT);
}

test.describe("Dark mode: native parts follow the theme", () => {
  test("a dark choice (cookie) on a light OS: color-scheme is dark and the UA draws dark", async ({ page }) => {
    // The cookie is the explicit choice, so it must win over the OS preference.
    await openThemed(page, { os: "light", cookie: "dark" });
    const r = await probe(page);
    expect(r.dataTheme).toBe("dark");
    expect(r.rootScheme).toBe("dark");
    expect(r.selectScheme).toBe("dark"); // inherited from the root, not set per component
    expect(r.uaCanvasLum, "the UA's own Canvas must be dark").toBeLessThan(0.2);
    expect(r.uaTextLum, "and its CanvasText light").toBeGreaterThan(0.8);
  });

  test("the select and its options read the role tokens in dark", async ({ page }) => {
    await openThemed(page, { os: "light", cookie: "dark" });
    const r = await probe(page);
    expect(r.select.bg).toBe(r.select.expectBgDark);
    expect(r.select.color).toBe(r.select.expectColor);
    // The popup is painted from the options: opaque popover surface and ink, so it never inherits the
    // select's translucent fill (which, over a light popup, was near-white text on near-white).
    expect(r.option.bg).toBe(r.option.expectBg);
    expect(r.option.color).toBe(r.option.expectColor);
    expect(r.option.bg).not.toBe("rgba(0, 0, 0, 0)");
  });

  test("a light choice (cookie) on a dark OS: color-scheme is light and the UA draws light", async ({ page }) => {
    await openThemed(page, { os: "dark", cookie: "light" });
    const r = await probe(page);
    expect(r.dataTheme).toBe("light");
    expect(r.rootScheme).toBe("light");
    expect(r.selectScheme).toBe("light");
    expect(r.uaCanvasLum).toBeGreaterThan(0.8);
    expect(r.uaTextLum).toBeLessThan(0.2);
    expect(r.select.bg).toBe(r.select.expectBgLight);
    expect(r.select.color).toBe(r.select.expectColor);
    expect(r.option.bg).toBe(r.option.expectBg);
    expect(r.option.color).toBe(r.option.expectColor);
  });

  for (const os of ["dark", "light"] as const) {
    test(`no choice made: color-scheme is "light dark" and the UA follows the ${os} OS preference`, async ({ page }) => {
      // The no-JS path: nothing sets `data-theme`, so the root declares both schemes and the browser
      // picks by `prefers-color-scheme`, the very media feature the token blocks key on.
      await openThemed(page, { os });
      const r = await probe(page);
      expect(r.dataTheme).toBeNull();
      expect(r.rootScheme).toBe("light dark");
      if (os === "dark") {
        expect(r.uaCanvasLum).toBeLessThan(0.2);
        expect(r.option.bg).toBe(r.option.expectBg);
        expect(r.select.bg).toBe(r.select.expectBgDark);
      } else {
        expect(r.uaCanvasLum).toBeGreaterThan(0.8);
        expect(r.select.bg).toBe(r.select.expectBgLight);
      }
    });
  }

  test("under a theme preset the options still read the (tinted) popover role, in dark", async ({ page }) => {
    await openThemed(page, { os: "light", cookie: "dark", base: "zinc" });
    const r = await probe(page);
    expect(r.rootScheme).toBe("dark");
    expect(r.option.bg).toBe(r.option.expectBg);
    expect(r.option.color).toBe(r.option.expectColor);
    expect(r.select.bg).toBe(r.select.expectBgDark);
  });
});
