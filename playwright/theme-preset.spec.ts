import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/**
 * The docs-site theme picker (`preview/src/theme.rs` `ThemePicker`): shadcn's customizer as data
 * (`preview/assets/theme-presets.css`, `[data-theme-base|accent|radius]` blocks setting shadcn's bare
 * variables, which every `--dx-*` role token reads first).
 *
 * Expected colours are READ FROM THAT STYLESHEET, not restated here, so the spec follows the data and
 * proves the whole chain (cookie/attribute -> bare variable -> `--dx-*` role token -> a real button's
 * computed `background-color`) instead of re-deriving numbers.
 *
 * On phones (<= 760px, the header's own breakpoint) the header has no room for the picker, so its
 * trigger moves into the mobile navigation sheet ("Theme", at the foot of the sidebar) and opens the SAME
 * panel as a bottom Drawer instead of a Popover. The phone tests below cover that presentation: both
 * trigger placements are always rendered and CSS picks one, so they also pin the breakpoint from both
 * sides, and they open a modal (the drawer) on top of a modal (the sheet).
 *
 * Run against a dev server or an SSG build:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=baseline.local.config.ts ./theme-preset.spec.ts
 */

const PRESETS_CSS = fs.readFileSync(
  path.join(__dirname, "..", "preview", "assets", "theme-presets.css"),
  "utf8",
);

/** `{ dark, light }` of one variable in one `[data-theme-<axis>="<name>"]` block of theme-presets.css. */
function presetVar(axis: "base" | "accent", name: string, variable: string) {
  const open = PRESETS_CSS.indexOf(`[data-theme-${axis}="${name}"] {`);
  if (open < 0) throw new Error(`no ${axis} block "${name}" in theme-presets.css`);
  const block = PRESETS_CSS.slice(open, PRESETS_CSS.indexOf("}", open));
  const m = block.match(new RegExp(`--${variable}: var\\(--dark, ([^;]*?)\\) var\\(--light, ([^;]*?)\\);`));
  if (!m) throw new Error(`--${variable} is not a dark/light pair in ${axis} "${name}"`);
  return { dark: m[1], light: m[2] };
}

type RGB = [number, number, number];

/**
 * sRGB triple of any CSS colour string, through a 1x1 canvas. The asset pipeline (lightningcss) may
 * rewrite `oklch(...)` as `lab(...)` in the shipped stylesheet, and `getComputedStyle` echoes whatever
 * the author wrote, so comparing colour TEXT is brittle; comparing the pixels both strings paint is not.
 */
async function rgbOf(page: Page, css: string): Promise<RGB> {
  return page.evaluate((c) => {
    const cv = document.createElement("canvas");
    cv.width = cv.height = 1;
    const cx = cv.getContext("2d", { willReadFrequently: true })!;
    cx.fillStyle = c;
    cx.fillRect(0, 0, 1, 1);
    const d = cx.getImageData(0, 0, 1, 1).data;
    return [d[0], d[1], d[2]] as RGB;
  }, css);
}

const distance = (a: RGB, b: RGB) => Math.max(...a.map((v, i) => Math.abs(v - b[i])));

/** The demo page's own "Primary" button (not the picker's chips, which are primary buttons too). */
const primaryButton = (page: Page) => page.getByRole("button", { name: "Primary", exact: true });

const bg = (el: Locator) => el.evaluate((n) => getComputedStyle(n).backgroundColor);

/** Polls until `el`'s background paints `css` (within 2/255 per channel; transitions take 150ms). */
async function expectBackground(page: Page, el: Locator, css: string) {
  const want = await rgbOf(page, css);
  await expect
    .poll(async () => distance(await rgbOf(page, await bg(el)), want), { message: `background of ${css}` })
    .toBeLessThanOrEqual(2);
}

async function expectBackgroundNot(page: Page, el: Locator, css: string) {
  const other = await rgbOf(page, css);
  await expect
    .poll(async () => distance(await rgbOf(page, await bg(el)), other), { message: `background differs from ${css}` })
    .toBeGreaterThan(10);
}

async function openPicker(page: Page): Promise<Locator> {
  await page.getByRole("button", { name: "Theme", exact: true }).click();
  const panel = page.locator(".dx-theme-picker");
  await expect(panel).toBeVisible();
  return panel;
}

const attr = (page: Page, name: string) =>
  page.evaluate((n) => document.documentElement.getAttribute(n), name);

const cookie = async (page: Page, name: string) =>
  (await page.context().cookies()).find((c) => c.name === name)?.value;

test.use({ colorScheme: "light" });

test.beforeEach(async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/component/button/`);
  await expect(primaryButton(page)).toBeVisible();
});

test("default (no preset) is today's look: no attributes, black primary", async ({ page }) => {
  expect(await attr(page, "data-theme-base")).toBeNull();
  expect(await attr(page, "data-theme-accent")).toBeNull();
  expect(await attr(page, "data-theme-radius")).toBeNull();
  await expectBackground(page, primaryButton(page), "#000");
});

test("selecting an accent recolours a real button through --dx-primary", async ({ page }) => {
  const button = primaryButton(page);
  const token = () => button.evaluate((n) => getComputedStyle(n).getPropertyValue("--dx-primary").trim());
  const before = await token();

  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "Blue", exact: true }).click();

  const blue = presetVar("accent", "blue", "primary");
  expect(await attr(page, "data-theme-accent")).toBe("blue");
  await expectBackground(page, button, blue.light);
  const after = await token();
  expect(after).not.toBe(before);
  await expect(panel.getByRole("button", { name: "Blue", exact: true })).toHaveAttribute("aria-pressed", "true");
  expect(await cookie(page, "dx_theme_accent")).toBe("blue");
});

test("a base colour re-themes the page surfaces in both modes", async ({ page }) => {
  const zinc = presetVar("base", "zinc", "background");
  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "Zinc", exact: true }).click();
  expect(await attr(page, "data-theme-base")).toBe("zinc");

  const navbar = page.locator(".dx-preview-navbar");
  await expectBackground(page, navbar, zinc.light);

  await page.getByRole("button", { name: "Enable dark mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expectBackground(page, navbar, zinc.dark);
  // shadcn's dark background is not the library's own pure black: the preset really took over.
  await expectBackgroundNot(page, navbar, "#000");
});

test("a preset persists across client-side navigation and a hard reload", async ({ page }) => {
  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "Orange", exact: true }).click();
  await panel.getByRole("button", { name: "Stone", exact: true }).click();
  await panel.getByRole("button", { name: "0.3", exact: true }).click();
  await page.keyboard.press("Escape");

  const orange = presetVar("accent", "orange", "primary");

  // Client-side navigation (a sidebar link, no document load).
  await page.getByRole("link", { name: "calendar", exact: true }).first().click();
  await expect(page).toHaveURL(/\/component\/calendar\//);
  expect(await attr(page, "data-theme-accent")).toBe("orange");
  expect(await attr(page, "data-theme-base")).toBe("stone");
  expect(await attr(page, "data-theme-radius")).toBe("0.3");

  // Hard reload of another page: the cookie, not component state, carries it.
  await gotoHydrated(page, `${BASE_URL}/component/button/`);
  await expectBackground(page, primaryButton(page), orange.light);
  expect(await attr(page, "data-theme-radius")).toBe("0.3");
});

test("the preset is on <html> before the app boots (no flash)", async ({ page, context }) => {
  await context.addCookies([
    { name: "dx_theme_accent", value: "violet", url: BASE_URL },
    { name: "dx_theme_base", value: "slate", url: BASE_URL },
    { name: "dx_theme_radius", value: "1", url: BASE_URL },
    { name: "dx_theme", value: "dark", url: BASE_URL },
  ]);
  // Stop the wasm bundle: whatever is on <html> now came from the head script, not from the app.
  await page.route(/\.wasm(\?.*)?$/, (route) => route.abort());
  await page.goto(`${BASE_URL}/component/button/`, { waitUntil: "domcontentloaded" });
  const attrs = await page.evaluate(() => ({
    accent: document.documentElement.getAttribute("data-theme-accent"),
    base: document.documentElement.getAttribute("data-theme-base"),
    radius: document.documentElement.getAttribute("data-theme-radius"),
    mode: document.documentElement.getAttribute("data-theme"),
    hydrated: document.documentElement.getAttribute("data-hydrated"),
  }));
  expect(attrs).toEqual({ accent: "violet", base: "slate", radius: "1", mode: "dark", hydrated: null });
});

test("junk in a preset cookie is ignored, not applied", async ({ page, context }) => {
  await context.addCookies([
    { name: "dx_theme_accent", value: "blue%22%20onload%3Dx", url: BASE_URL },
    { name: "dx_theme", value: "purple", url: BASE_URL },
  ]);
  await page.goto(`${BASE_URL}/component/button/`, { waitUntil: "domcontentloaded" });
  expect(await attr(page, "data-theme-accent")).toBeNull();
  expect(await attr(page, "data-theme")).toBeNull();
});

test("light/dark toggle still flips with a preset active", async ({ page }) => {
  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "Blue", exact: true }).click();
  await page.keyboard.press("Escape");

  const blue = presetVar("accent", "blue", "primary");
  const button = primaryButton(page);
  await expectBackground(page, button, blue.light);

  await page.getByRole("button", { name: "Enable dark mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expectBackground(page, button, blue.dark);
  expect(blue.dark).not.toBe(blue.light);

  await page.getByRole("button", { name: "Enable light mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expectBackground(page, button, blue.light);
});

test("Default restores today's value, per axis and via Reset", async ({ page }) => {
  const button = primaryButton(page);

  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "Blue", exact: true }).click();
  await expectBackgroundNot(page, button, "#000");
  await panel.getByRole("button", { name: "Default accent" }).click();
  await expectBackground(page, button, "#000");
  expect(await attr(page, "data-theme-accent")).toBeNull();
  expect(await cookie(page, "dx_theme_accent")).toBeUndefined();

  await panel.getByRole("button", { name: "Red", exact: true }).click();
  await panel.getByRole("button", { name: "Slate", exact: true }).click();
  await panel.getByRole("button", { name: "1", exact: true }).click();
  await expectBackgroundNot(page, button, "#000");
  await panel.getByRole("button", { name: "Reset" }).click();
  await expectBackground(page, button, "#000");
  for (const a of ["data-theme-base", "data-theme-accent", "data-theme-radius"]) {
    expect(await attr(page, a)).toBeNull();
  }
  await expect(panel.getByRole("button", { name: "Default accent" })).toHaveAttribute("aria-pressed", "true");
  await expect(panel.getByRole("button", { name: "0.625", exact: true })).toHaveAttribute("aria-pressed", "true");
});

test("radius drives the whole corner scale; the default radius is 'no attribute'", async ({ page }) => {
  const button = primaryButton(page);
  const radiusPx = () => button.evaluate((n) => parseFloat(getComputedStyle(n).borderTopLeftRadius));
  const base = await radiusPx();

  const panel = await openPicker(page);
  await panel.getByRole("button", { name: "1", exact: true }).click();
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--radius").trim())).toBe("1rem");
  await expect.poll(radiusPx).toBeGreaterThan(base);

  await panel.getByRole("button", { name: "0", exact: true }).click();
  await expect.poll(radiusPx).toBe(0);

  await panel.getByRole("button", { name: "0.625", exact: true }).click();
  expect(await attr(page, "data-theme-radius")).toBeNull();
  await expect.poll(radiusPx).toBe(base);
});

test("every swatch is named and the panel opens from the keyboard", async ({ page }) => {
  const trigger = page.getByRole("button", { name: "Theme", exact: true });
  await trigger.focus();
  await page.keyboard.press("Enter");
  const panel = page.locator(".dx-theme-picker");
  await expect(panel).toBeVisible();
  const swatches = panel.locator(".dx-theme-swatch");
  const count = await swatches.count();
  expect(count).toBe(1 + 9 + 1 + 17);
  for (let i = 0; i < count; i++) {
    await expect(swatches.nth(i)).toHaveAccessibleName(/\S/);
  }
  await page.keyboard.press("Escape");
  await expect(panel).toBeHidden();
});

test("with no preset --dx-border-strong is still the ramp step, in both modes", async ({ page }) => {
  // The one role token that gained a `var(--border-strong, ...)` hook for presets; its fallback must be
  // exactly what it was. Painted colours, not custom-property text.
  for (const mode of ["light", "dark"]) {
    const [strong, ramp] = await page.evaluate((m) => {
      document.documentElement.setAttribute("data-theme", m);
      const paint = (v: string) => {
        const el = document.createElement("div");
        el.style.borderTop = `1px solid var(${v})`;
        document.body.appendChild(el);
        const c = getComputedStyle(el).borderTopColor;
        el.remove();
        return c;
      };
      return [paint("--dx-border-strong"), paint("--primary-color-7")];
    }, mode);
    expect(strong, mode).toBe(ramp);
  }
});

/* ------------------------------------------------------------------------------------------------
 * Phones: the trigger lives in the mobile nav sheet and opens the panel as a bottom Drawer.
 *
 * Titles say "phone", not "mobile", on purpose: the configs skip every title matching /mobile/ (their
 * launch has no touch emulation), but these describes turn `hasTouch`/`isMobile` on themselves, so they
 * can, and should, run everywhere Chromium or WebKit does.
 * ---------------------------------------------------------------------------------------------- */

const SHOTS = process.env.THEME_SHOTS_DIR; // opt-in: THEME_SHOTS_DIR=/some/dir writes mobile-*.png there

const navSheet = (page: Page) => page.locator(".dx-sidebar-sheet");
const drawer = (page: Page) => page.locator(".dx-drawer");

/** Opens the mobile navigation sheet and returns its "Theme" entry. */
async function openNavSheet(page: Page, tap = true): Promise<Locator> {
  const toggle = page.getByRole("button", { name: "Toggle Sidebar" });
  if (tap) await toggle.tap();
  else await toggle.click();
  await expect(navSheet(page)).toBeVisible();
  const entry = navSheet(page).getByRole("button", { name: "Theme", exact: true });
  await expect(entry).toBeVisible();
  return entry;
}

/** Names of the open dialogs, outermost first (document order), and whether each is truly modal. */
const openDialogs = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("dialog")]
      .filter((d) => d.open)
      .map((d) => `${d.classList.contains("dx-drawer") ? "drawer" : d.classList.contains("dx-sidebar-sheet") ? "sheet" : d.className}${d.matches(":modal") ? ":modal" : ""}`),
  );

const scrollLocked = (page: Page) => page.evaluate(() => (window as unknown as { __dxScrollLocked?: boolean }).__dxScrollLocked === true);

const focusIn = (page: Page, selector: string) =>
  page.evaluate((sel) => !!document.activeElement?.closest(sel), selector);

/** Focus on a real element of the page that is NOT inside `selector` (`<body>` = focus left the page: fine). */
const focusEscaped = (page: Page, selector: string) =>
  page.evaluate((sel) => {
    const a = document.activeElement;
    return !!a && a !== document.body && a !== document.documentElement && !a.closest(sel);
  }, selector);

const PHONES = [
  { width: 390, height: 844 },
  { width: 360, height: 740 },
];

for (const { width, height } of PHONES) {
  for (const scheme of ["light", "dark"] as const) {
    test.describe(`phone ${width}x${height} ${scheme}`, () => {
      test.use({ viewport: { width, height }, hasTouch: true, isMobile: true, colorScheme: scheme });
      test.skip(({ browserName }) => browserName === "firefox", "Firefox has no mobile emulation");

      test("the picker is not in the header; 'Theme' is in the nav sheet, on screen and tappable", async ({ page }) => {
        // The header keeps light/dark but has no picker button (hidden, so not in the a11y tree either).
        await expect(page.getByRole("button", { name: "Theme", exact: true })).toHaveCount(0);
        await expect(page.getByRole("button", { name: /Enable (dark|light) mode/ })).toBeVisible();

        const entry = await openNavSheet(page);
        await expect(entry).toHaveAttribute("aria-haspopup", "dialog");
        // The sheet slides in from the left: wait for it to land before measuring.
        await expect.poll(async () => (await entry.boundingBox())!.x).toBeGreaterThanOrEqual(0);
        const box = (await entry.boundingBox())!;
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.y).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(width);
        expect(box.y + box.height).toBeLessThanOrEqual(height);
        expect(box.height).toBeGreaterThanOrEqual(32);
        // Nothing paints over it: the point a finger would land on resolves to the entry itself.
        const hit = await entry.evaluate((el) => {
          const r = el.getBoundingClientRect();
          return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
        });
        expect(hit).toBe(true);
        if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `mobile-${width}x${height}-${scheme}-menu.png`) });
      });

      test("the drawer applies a preset at once, persists it, and returns focus to the trigger", async ({ page }) => {
        const mode = scheme;
        const entry = await openNavSheet(page);
        await entry.tap();
        const panel = drawer(page);
        await expect(panel).toBeVisible();
        await expect(page.getByRole("dialog", { name: "Theme" })).toBeVisible();
        await expect(panel.locator('.dx-theme-picker-panel[data-presentation="drawer"]')).toBeVisible();
        // Never the popover's presentation.
        await expect(page.locator(".dx-theme-picker")).toHaveCount(0);

        // A phone-sized panel: inside the viewport, and every swatch is a real touch target.
        const dbox = (await panel.boundingBox())!;
        expect(dbox.x).toBeGreaterThanOrEqual(0);
        expect(dbox.x + dbox.width).toBeLessThanOrEqual(width);
        const swatchBox = (await panel.getByRole("button", { name: "Blue", exact: true }).boundingBox())!;
        expect(Math.min(swatchBox.width, swatchBox.height)).toBeGreaterThanOrEqual(32);

        // Applies immediately, behind the open drawer.
        const blue = presetVar("accent", "blue", "primary");
        await panel.getByRole("button", { name: "Blue", exact: true }).tap();
        expect(await attr(page, "data-theme-accent")).toBe("blue");
        await expectBackground(page, primaryButton(page), blue[mode]);
        await expect(panel.getByRole("button", { name: "Blue", exact: true })).toHaveAttribute("aria-pressed", "true");
        expect(await cookie(page, "dx_theme_accent")).toBe("blue");

        const zinc = presetVar("base", "zinc", "background");
        await panel.getByRole("button", { name: "Zinc", exact: true }).tap();
        await expectBackground(page, page.locator(".dx-preview-navbar"), zinc[mode]);
        await panel.getByRole("button", { name: "0.3", exact: true }).tap();
        expect(await attr(page, "data-theme-radius")).toBe("0.3");
        if (SHOTS) {
          await page.waitForTimeout(400); // let the 150ms chip/swatch transitions settle for the picture
          await page.screenshot({ path: path.join(SHOTS, `mobile-${width}x${height}-${scheme}-drawer.png`) });
        }

        // Closing it: gone from the DOM, the sheet underneath is untouched, focus is back on the trigger.
        await panel.getByRole("button", { name: "Done" }).tap();
        await expect(drawer(page)).toHaveCount(0);
        await expect(navSheet(page)).toBeVisible();
        await expect(entry).toBeFocused();

        // It persisted: a hard reload re-applies it from the cookies, and the reopened panel reads it back.
        await gotoHydrated(page, `${BASE_URL}/component/button/`);
        await expect(primaryButton(page)).toBeVisible();
        await expectBackground(page, primaryButton(page), blue[mode]);
        expect(await attr(page, "data-theme-radius")).toBe("0.3");
        await (await openNavSheet(page)).tap();
        await expect(drawer(page).getByRole("button", { name: "Blue", exact: true })).toHaveAttribute("aria-pressed", "true");
        await expect(drawer(page).getByRole("button", { name: "Zinc", exact: true })).toHaveAttribute("aria-pressed", "true");
        await expect(drawer(page).getByRole("button", { name: "0.3", exact: true })).toHaveAttribute("aria-pressed", "true");

        // Reset puts today's look back, in the open drawer.
        await drawer(page).getByRole("button", { name: "Reset" }).tap();
        for (const a of ["data-theme-base", "data-theme-accent", "data-theme-radius"]) {
          expect(await attr(page, a)).toBeNull();
        }
        await expect(drawer(page).getByRole("button", { name: "Default accent" })).toHaveAttribute("aria-pressed", "true");
      });
    });
  }
}

test.describe("phone modal stacking (390x844)", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, colorScheme: "light" });
  test.skip(({ browserName }) => browserName === "firefox", "Firefox has no mobile emulation");

  test("the drawer opens on top of the nav sheet: one Escape, one focus trap, one scroll lock", async ({ page }) => {
    expect(await openDialogs(page)).toEqual([]);
    expect(await scrollLocked(page)).toBe(false);

    const entry = await openNavSheet(page);
    expect(await openDialogs(page)).toEqual(["sheet:modal"]);
    expect(await scrollLocked(page)).toBe(true);

    await entry.tap();
    await expect(drawer(page)).toBeVisible();
    expect(await openDialogs(page)).toEqual(["sheet:modal", "drawer:modal"]);
    expect(await scrollLocked(page)).toBe(true);
    // Focus moved into the top dialog, and Tab never reaches the sheet's links or the page underneath
    // (the first Tab past the last control leaves the document, as in any native modal dialog, and the
    // next one is back at the top of the drawer): two full cycles.
    expect(await focusIn(page, ".dx-drawer")).toBe(true);
    let reentered = false;
    for (let i = 0; i < 80; i++) {
      await page.keyboard.press("Tab");
      expect(await focusEscaped(page, ".dx-drawer"), `Tab #${i + 1} landed outside the drawer`).toBe(false);
      if (i > 36 && (await focusIn(page, ".dx-drawer"))) reentered = true;
    }
    expect(reentered).toBe(true);
    await page.keyboard.press("Shift+Tab");
    expect(await focusEscaped(page, ".dx-drawer")).toBe(false);
    // Put focus back inside for the Escape below (headless has no browser chrome to hold it).
    await page.getByRole("dialog", { name: "Theme" }).getByRole("button", { name: "Reset" }).focus();

    // Escape closes only the topmost modal: the sheet stays open, locked, and focus is back on the entry.
    await page.keyboard.press("Escape");
    await expect(drawer(page)).toHaveCount(0);
    expect(await openDialogs(page)).toEqual(["sheet:modal"]);
    expect(await scrollLocked(page)).toBe(true);
    await expect(navSheet(page)).toBeVisible();
    await expect(entry).toBeFocused();

    // A tap on the drawer's own scrim (far above the panel) closes the drawer, not the sheet beneath it.
    await entry.tap();
    await expect(drawer(page)).toBeVisible();
    await page.touchscreen.tap(340, 40);
    await expect(drawer(page)).toHaveCount(0);
    expect(await openDialogs(page)).toEqual(["sheet:modal"]);
    await expect(entry).toBeFocused();
    expect(await scrollLocked(page)).toBe(true);

    // The second Escape closes the sheet and releases the one lock.
    await page.keyboard.press("Escape");
    await expect(navSheet(page)).toHaveCount(0);
    expect(await openDialogs(page)).toEqual([]);
    expect(await scrollLocked(page)).toBe(false);
    await expect(page.getByRole("button", { name: "Toggle Sidebar" })).toBeFocused();
  });

  test("dragging the drawer down dismisses it and leaves the nav sheet open", async ({ page }) => {
    const entry = await openNavSheet(page);
    await entry.tap();
    const panel = drawer(page);
    await expect(panel).toBeVisible();
    // The gesture itself is drawer.spec.ts's business (same recipe: slow steps, a pause before release,
    // so the DISTANCE path decides, not the velocity one). What is checked here is only that it works
    // from inside the nav sheet and ends the drawer alone. Start once the slide-in has settled.
    const handle = panel.locator(".dx-drawer-handle");
    let last = -1;
    await expect
      .poll(async () => {
        const y = (await handle.boundingBox())!.y;
        const settled = Math.abs(y - last) < 0.5;
        last = y;
        return settled;
      })
      .toBe(true);
    const hb = (await handle.boundingBox())!;
    const content = (await panel.boundingBox())!;
    const x = hb.x + hb.width / 2;
    const y = hb.y + hb.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    for (let i = 1; i <= 6; i++) {
      await page.mouse.move(x, y + (content.height * 0.4 * i) / 6, { steps: 2 });
      await page.waitForTimeout(120);
    }
    await page.waitForTimeout(200);
    await page.mouse.up();
    await expect(drawer(page)).toHaveCount(0);
    expect(await openDialogs(page)).toEqual(["sheet:modal"]);
    await expect(entry).toBeFocused();
  });
});

test.describe("phone entry on every page that has a nav sheet (390x844)", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, colorScheme: "light" });
  test.skip(({ browserName }) => browserName === "firefox", "Firefox has no mobile emulation");

  // Home, the docs overview and a component page all render `DocsLayout`, so all three carry the entry.
  for (const route of ["/", "/docs/", "/component/calendar/"]) {
    test(`${route} has 'Theme' in its nav sheet and it opens the drawer`, async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}${route}`);
      const entry = await openNavSheet(page);
      await entry.tap();
      await expect(page.getByRole("dialog", { name: "Theme" })).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(drawer(page)).toHaveCount(0);
      await expect(entry).toBeFocused();
    });
  }
});

test.describe("phone keyboard (390x844, no touch)", () => {
  test.use({ viewport: { width: 390, height: 844 }, colorScheme: "light" });

  test("Enter on 'Theme' opens the drawer; the swatches are labelled groups; Escape returns focus", async ({ page }) => {
    await page.getByRole("button", { name: "Toggle Sidebar" }).focus();
    await page.keyboard.press("Enter");
    await expect(navSheet(page)).toBeVisible();
    const entry = navSheet(page).getByRole("button", { name: "Theme", exact: true });
    await entry.focus();
    await page.keyboard.press("Enter");

    const panel = drawer(page);
    await expect(panel).toBeVisible();
    await expect(page.getByRole("dialog", { name: "Theme" })).toBeVisible();
    const base = panel.getByRole("group", { name: /^Base colour/ });
    const accent = panel.getByRole("group", { name: /^Accent/ });
    const radius = panel.getByRole("group", { name: /^Radius/ });
    await expect(base.locator(".dx-theme-swatch")).toHaveCount(1 + 9);
    await expect(accent.locator(".dx-theme-swatch")).toHaveCount(1 + 17);
    await expect(radius.getByRole("button")).toHaveCount(6);
    const swatches = panel.locator(".dx-theme-swatch");
    for (let i = 0, n = await swatches.count(); i < n; i++) {
      await expect(swatches.nth(i)).toHaveAccessibleName(/\S/);
    }

    // Operable without a pointer.
    await panel.getByRole("button", { name: "Teal", exact: true }).focus();
    await page.keyboard.press("Enter");
    expect(await attr(page, "data-theme-accent")).toBe("teal");
    await expect(panel.getByRole("button", { name: "Teal", exact: true })).toHaveAttribute("aria-pressed", "true");

    await page.keyboard.press("Escape");
    await expect(drawer(page)).toHaveCount(0);
    await expect(entry).toBeFocused();
    await expect(navSheet(page)).toBeVisible();
  });
});

test.describe("the breakpoint between the two presentations", () => {
  test("desktop 1280: header trigger + popover, and no phone entry anywhere in the page", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    const trigger = page.getByRole("button", { name: "Theme", exact: true });
    await expect(trigger).toHaveCount(1);
    await expect(trigger).toBeVisible();
    expect(await trigger.evaluate((el) => !!el.closest(".dx-navbar-utilities"))).toBe(true);
    // The phone entry exists in the sidebar's markup but is `display: none`: no box, not in the a11y tree.
    await expect(page.locator(".dx-theme-sidebar-footer")).toBeHidden();
    await trigger.click();
    await expect(page.locator(".dx-theme-picker")).toBeVisible();
    await expect(page.locator('.dx-theme-picker-panel[data-presentation="popover"]')).toBeVisible();
    await expect(page.locator(".dx-drawer")).toHaveCount(0);
  });

  test("760px is a phone, 761px is not: exactly one 'Theme' button at each", async ({ page }) => {
    const header = page.locator(".dx-preview-navbar").getByRole("button", { name: "Theme", exact: true });

    await page.setViewportSize({ width: 761, height: 800 });
    await expect(header).toBeVisible();
    // The sidebar is already a sheet below 768px (its own switch, in Rust), but it must not add a second entry.
    await page.getByRole("button", { name: "Toggle Sidebar" }).click();
    await expect(navSheet(page)).toBeVisible();
    await expect(page.locator(".dx-theme-sidebar-footer")).toBeHidden();
    await expect(page.getByRole("button", { name: "Theme", exact: true })).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(navSheet(page)).toHaveCount(0);

    await page.setViewportSize({ width: 760, height: 800 });
    await expect(header).toBeHidden();
    await page.getByRole("button", { name: "Toggle Sidebar" }).click();
    await expect(navSheet(page)).toBeVisible();
    await expect(page.locator(".dx-theme-sidebar-footer")).toBeVisible();
    await expect(page.getByRole("button", { name: "Theme", exact: true })).toHaveCount(1);
  });
});
