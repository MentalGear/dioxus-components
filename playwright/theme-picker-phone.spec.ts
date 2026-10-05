import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import * as path from "path";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/**
 * Backlog row 142: a phone has to be able to reach the theme picker on EVERY page that has the site
 * header, not only on the ones with a navigation sheet to host it.
 *
 * `theme::ThemePickerSidebarFooter` (the phone entry the picker has always had) lives at the foot of the
 * docs sidebar's sheet, so a route with no sidebar (`/demos`, the `/charts/` gallery, the "not found"
 * pages) had no picker at <= 760px: the header's popover trigger is hidden there and there was no sheet.
 * The header's own `theme::ThemePicker` now renders `ThemePickerPhoneTrigger` when (and only when) the
 * route has no `SidebarCtx`, so this spec is a SWEEP over every route type: whichever entry a page has,
 * it must have exactly one "Theme" entry reachable at phone width, that entry must open the SAME drawer,
 * and the header must still fit.
 *
 * The preset/cookie plumbing itself (every swatch, persistence across reloads, focus return from a drawer
 * stacked on the nav sheet) is `theme-preset.spec.ts`'s job; this spec only proves the routes that spec
 * does not visit reach it too, and that desktop is unchanged on them.
 *
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=baseline.local.config.ts ./theme-picker-phone.spec.ts
 */

/** Opt-in: `THEME_SHOTS_DIR=/some/dir` writes a screenshot of each phone route there. */
const SHOTS = process.env.THEME_SHOTS_DIR;

/** Routes whose header has no navigation sheet: the picker's phone entry is the header's own button. */
const HEADER_ENTRY_ROUTES = [
  { name: "demos", url: "/demos" },
  { name: "charts", url: "/charts/" },
  { name: "charts tab", url: "/charts/bar/" },
  { name: "charts not found", url: "/charts/no-such-chart/" },
  { name: "component not found", url: "/component/no-such-component/" },
];

/** Routes with the docs sidebar: the entry is the "Theme" footer of the navigation sheet (unchanged). */
const SHEET_ENTRY_ROUTES = [
  { name: "home", url: "/" },
  { name: "docs", url: "/docs" },
  { name: "component page", url: "/component/button/" },
];

const PHONES = [
  { width: 390, height: 844 },
  { width: 360, height: 740 },
];

const themeButtons = (page: Page) => page.getByRole("button", { name: "Theme", exact: true });
const headerThemeButton = (page: Page) => page.locator(".dx-preview-navbar").getByRole("button", { name: "Theme", exact: true });
const drawer = (page: Page) => page.locator(".dx-drawer");
const navSheet = (page: Page) => page.locator(".dx-sidebar-sheet");

const attr = (page: Page, name: string) => page.evaluate((n) => document.documentElement.getAttribute(n), name);
const cookie = async (page: Page, name: string) => (await page.context().cookies()).find((c) => c.name === name)?.value;

/** `--dx-primary` as the header resolves it: the token every accent preset rewrites. */
const primaryToken = (page: Page) =>
  page.locator(".dx-preview-navbar").evaluate((n) => getComputedStyle(n).getPropertyValue("--dx-primary").trim());

/** The page is no wider than the viewport: nothing the header (or anything else) adds makes it scroll sideways. */
async function expectNoHorizontalOverflow(page: Page, width: number) {
  const m = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    header: document.querySelector(".dx-preview-navbar .dx-navbar-inner")!.scrollWidth,
    headerBox: document.querySelector(".dx-preview-navbar .dx-navbar-inner")!.getBoundingClientRect().width,
  }));
  expect(m.innerWidth).toBe(width);
  expect(m.scrollWidth).toBeLessThanOrEqual(m.innerWidth);
  expect(m.header).toBeLessThanOrEqual(Math.ceil(m.headerBox));
}

/**
 * Every visible control in the header sits inside the viewport and none paints over another (the failure
 * the picker's first phone attempt had: one more 24px icon drew over "Charts" and the GitHub link).
 * Items nested in another item (the language select's flag inside its trigger) are not "another item".
 */
async function expectHeaderItemsClear(page: Page, width: number, tolerance = 1) {
  const items = await page.evaluate(() => {
    const els = [...document.querySelectorAll<HTMLElement>(".dx-preview-navbar a, .dx-preview-navbar button, .dx-preview-navbar select")];
    return els
      .map((el) => {
        const r = el.getBoundingClientRect();
        return { label: el.getAttribute("aria-label") || el.textContent?.trim() || el.tagName, x: r.x, y: r.y, w: r.width, h: r.height, el };
      })
      .filter((i) => i.w > 0 && i.h > 0)
      .filter((i) => !els.some((o) => o !== i.el && o.contains(i.el) && o.getBoundingClientRect().width > 0))
      .filter((i) => !els.some((o) => o !== i.el && i.el.contains(o) && o.getBoundingClientRect().width > 0))
      .map(({ el: _el, ...rest }) => rest);
  });
  expect(items.length).toBeGreaterThanOrEqual(5);
  for (const i of items) {
    expect(i.x, `${i.label} starts inside the viewport`).toBeGreaterThanOrEqual(0);
    expect(i.x + i.w, `${i.label} ends inside the viewport`).toBeLessThanOrEqual(width);
  }
  for (let a = 0; a < items.length; a++) {
    for (let b = a + 1; b < items.length; b++) {
      const A = items[a];
      const B = items[b];
      const overlapX = Math.min(A.x + A.w, B.x + B.w) - Math.max(A.x, B.x);
      const overlapY = Math.min(A.y + A.h, B.y + B.h) - Math.max(A.y, B.y);
      expect(overlapX > tolerance && overlapY > tolerance, `header items "${A.label}" and "${B.label}" overlap by ${overlapX}px`).toBe(false);
    }
  }
}

/** The drawer is a phone-sized sheet: inside the viewport, swatches are real touch targets. */
async function expectPhoneSizedDrawer(page: Page, width: number, height: number) {
  const panel = drawer(page);
  await expect(panel).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Theme" })).toBeVisible();
  await expect(panel.locator('.dx-theme-picker-panel[data-presentation="drawer"]')).toBeVisible();
  await expect(page.locator(".dx-theme-picker")).toHaveCount(0); // never the popover's presentation
  // The drawer slides up from the bottom edge: measure its bottom once it has landed.
  await expect
    .poll(async () => {
      const b = (await panel.boundingBox())!;
      return b.y + b.height;
    })
    .toBeLessThanOrEqual(height + 1);
  const box = (await panel.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(width);
  const swatch = (await panel.getByRole("button", { name: "Blue", exact: true }).boundingBox())!;
  expect(Math.min(swatch.width, swatch.height)).toBeGreaterThanOrEqual(32);
}

test.use({ colorScheme: "light" });

for (const { width, height } of PHONES) {
  test.describe(`phone ${width}x${height}`, () => {
    test.use({ viewport: { width, height }, hasTouch: true, isMobile: true });
    test.skip(({ browserName }) => browserName === "firefox", "Firefox has no mobile emulation");

    for (const route of HEADER_ENTRY_ROUTES) {
      test(`${route.name} (${route.url}): the header has the one "Theme" entry, it fits, and it opens the drawer`, async ({ page }) => {
        await gotoHydrated(page, `${BASE_URL}${route.url}`);

        // Exactly one entry in the whole page, and it is in the header (no sheet exists to hold another).
        await expect(themeButtons(page)).toHaveCount(1);
        const trigger = headerThemeButton(page);
        await expect(trigger).toBeVisible();
        await expect(trigger).toHaveAttribute("aria-haspopup", "dialog");
        expect(await trigger.evaluate((el) => !!el.closest(".dx-navbar-utilities"))).toBe(true);
        // Not the popover's trigger: that one is hidden at this width, this is the phone entry.
        await expect(page.locator(".dx-theme-picker-trigger")).toBeHidden();
        await expect(trigger).toHaveClass(/dx-theme-phone-trigger/);

        // It fits: on screen, a real touch target, nothing under it, nothing it paints over, no sideways scroll.
        const box = (await trigger.boundingBox())!;
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(width);
        expect(Math.min(box.width, box.height)).toBeGreaterThanOrEqual(24);
        const hit = await trigger.evaluate((el) => {
          const r = el.getBoundingClientRect();
          return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
        });
        expect(hit).toBe(true);
        await expectNoHorizontalOverflow(page, width);
        await expectHeaderItemsClear(page, width);
        if (SHOTS) await page.screenshot({ path: path.join(SHOTS, `phone-${width}x${height}-${route.name.replace(/ /g, "-")}.png`) });

        // It opens the same drawer ...
        await trigger.tap();
        await expectPhoneSizedDrawer(page, width, height);
        if (SHOTS) {
          await page.waitForTimeout(400);
          await page.screenshot({ path: path.join(SHOTS, `phone-${width}x${height}-${route.name.replace(/ /g, "-")}-drawer.png`) });
        }
        // ... the page behind it did not grow a scrollbar of its own, and the drawer is the topmost modal.
        await expectNoHorizontalOverflow(page, width);

        // ... closes it, and puts focus back on the button that opened it.
        await drawer(page).getByRole("button", { name: "Done" }).tap();
        await expect(drawer(page)).toHaveCount(0);
        await expect(trigger).toBeFocused();

        // Reopened by keyboard, dismissed by Escape.
        await trigger.focus();
        await page.keyboard.press("Enter");
        await expect(drawer(page)).toBeVisible();
        // Wait for the focus trap to land (a person never presses Escape before the drawer has focus).
        await expect.poll(() => page.evaluate(() => !!document.activeElement?.closest(".dx-drawer"))).toBe(true);
        await page.keyboard.press("Escape");
        await expect(drawer(page)).toHaveCount(0);
        await expect(trigger).toBeFocused();
      });
    }

    for (const route of SHEET_ENTRY_ROUTES) {
      test(`${route.name} (${route.url}): unchanged, the entry is the navigation sheet's "Theme", not the header's`, async ({ page }) => {
        await gotoHydrated(page, `${BASE_URL}${route.url}`);
        await expect(themeButtons(page)).toHaveCount(0);
        await expect(page.locator(".dx-theme-phone-trigger")).toHaveCount(0); // not even rendered
        await page.getByRole("button", { name: "Toggle Sidebar" }).tap();
        await expect(navSheet(page)).toBeVisible();
        const entry = navSheet(page).getByRole("button", { name: "Theme", exact: true });
        await expect(entry).toBeVisible();
        await expect(themeButtons(page)).toHaveCount(1);
      });
    }

    test("/demos: a preset and the light/dark toggle both apply to the page, from the header's entry", async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}/demos`);

      // Light/dark: still in the header beside the new entry, still switches the page.
      expect(await attr(page, "data-theme")).toBeNull();
      const toDark = page.getByRole("button", { name: "Enable dark mode" });
      await expect(toDark).toBeVisible();
      const lightBackground = await page.locator(".dx-preview-navbar").evaluate((n) => getComputedStyle(n).backgroundColor);
      await toDark.tap();
      expect(await attr(page, "data-theme")).toBe("dark");
      await expect
        .poll(() => page.locator(".dx-preview-navbar").evaluate((n) => getComputedStyle(n).backgroundColor))
        .not.toBe(lightBackground);
      await page.getByRole("button", { name: "Enable light mode" }).tap();
      expect(await attr(page, "data-theme")).toBe("light");

      // A preset: attribute at once, the `--dx-primary` token the page's controls read follows, the cookie
      // is written, and the open panel shows it selected.
      const before = await primaryToken(page);
      await headerThemeButton(page).tap();
      const panel = drawer(page);
      await expect(panel).toBeVisible();
      await panel.getByRole("button", { name: "Blue", exact: true }).tap();
      expect(await attr(page, "data-theme-accent")).toBe("blue");
      await expect.poll(() => primaryToken(page)).not.toBe(before);
      await expect(panel.getByRole("button", { name: "Blue", exact: true })).toHaveAttribute("aria-pressed", "true");
      expect(await cookie(page, "dx_theme_accent")).toBe("blue");
      await panel.getByRole("button", { name: "0.3", exact: true }).tap();
      expect(await attr(page, "data-theme-radius")).toBe("0.3");

      // Reset clears all three (the same Reset the sheet's drawer has).
      await panel.getByRole("button", { name: "Reset", exact: true }).tap();
      expect(await attr(page, "data-theme-accent")).toBeNull();
      expect(await attr(page, "data-theme-radius")).toBeNull();
      await expect.poll(() => primaryToken(page)).toBe(before);
      await panel.getByRole("button", { name: "Blue", exact: true }).tap();
      await panel.getByRole("button", { name: "Done" }).tap();
      await expect(drawer(page)).toHaveCount(0);

      // It persists, and a hard reload of a DIFFERENT no-sidebar route re-applies it and reads it back.
      await gotoHydrated(page, `${BASE_URL}/charts/`);
      expect(await attr(page, "data-theme-accent")).toBe("blue");
      await headerThemeButton(page).tap();
      await expect(drawer(page).getByRole("button", { name: "Blue", exact: true })).toHaveAttribute("aria-pressed", "true");
    });

    test("/charts/: a preset recolours the charts page too", async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}/charts/`);
      const before = await primaryToken(page);
      await headerThemeButton(page).tap();
      await drawer(page).getByRole("button", { name: "Rose", exact: true }).tap();
      expect(await attr(page, "data-theme-accent")).toBe("rose");
      await expect.poll(() => primaryToken(page)).not.toBe(before);
      await drawer(page).getByRole("button", { name: "Done" }).tap();
      await expect(drawer(page)).toHaveCount(0);
      await expectNoHorizontalOverflow(page, width);
      await expectHeaderItemsClear(page, width);
    });
  });
}

test.describe("desktop is unchanged on the routes that had no phone entry", () => {
  for (const route of HEADER_ENTRY_ROUTES.slice(0, 2)) {
    test(`1280 ${route.url}: the popover trigger is the one "Theme" button; the phone entry is rendered but hidden`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 800 });
      await gotoHydrated(page, `${BASE_URL}${route.url}`);

      await expect(themeButtons(page)).toHaveCount(1);
      const trigger = headerThemeButton(page);
      await expect(trigger).toBeVisible();
      await expect(trigger).toHaveClass(/dx-theme-picker-trigger/);
      expect(await trigger.evaluate((el) => !!el.closest(".dx-navbar-utilities"))).toBe(true);
      // Present in the markup (CSS, not Rust, picks the entry) but with no box and not in the a11y tree.
      await expect(page.locator(".dx-theme-phone-trigger")).toHaveCount(1);
      await expect(page.locator(".dx-theme-phone-trigger")).toBeHidden();

      await trigger.click();
      await expect(page.locator(".dx-theme-picker")).toBeVisible();
      await expect(page.locator('.dx-theme-picker-panel[data-presentation="popover"]')).toBeVisible();
      await expect(page.locator(".dx-drawer")).toHaveCount(0);
      await page.getByRole("button", { name: "Teal", exact: true }).click();
      expect(await attr(page, "data-theme-accent")).toBe("teal");
    });
  }

  test("760px is a phone, 761px is not, on /demos: exactly one 'Theme' button at each", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/demos`);

    await page.setViewportSize({ width: 761, height: 800 });
    await expect(themeButtons(page)).toHaveCount(1);
    await expect(headerThemeButton(page)).toHaveClass(/dx-theme-picker-trigger/);
    await expect(page.locator(".dx-theme-phone-trigger")).toBeHidden();

    await page.setViewportSize({ width: 760, height: 800 });
    await expect(themeButtons(page)).toHaveCount(1);
    await expect(headerThemeButton(page)).toHaveClass(/dx-theme-phone-trigger/);
    await expect(page.locator(".dx-theme-picker-trigger")).toBeHidden();
  });
});

/**
 * The header is one `nowrap` row at phone width, so every page's row has to fit its own items: a docs page
 * spends room on the sidebar trigger, a page without a sheet on the picker's own button, and at 390px a docs
 * page's "Charts" link drew ~4px over the GitHub mark (~16px at 320px) while this spec only swept the
 * header-entry routes at 390/360. Every route type x every phone width, down to 320px (the narrowest phone), with
 * ZERO tolerance: items may touch their neighbours' gaps but never their boxes.
 */
const ALL_ROUTES = [...SHEET_ENTRY_ROUTES, { name: "demos", url: "/demos" }, { name: "charts", url: "/charts/" }];

for (const { width, height } of [...PHONES, { width: 320, height: 568 }]) {
  test.describe(`the header row fits at ${width}px`, () => {
    test.use({ viewport: { width, height }, hasTouch: true, isMobile: true });
    test.skip(({ browserName }) => browserName === "firefox", "Firefox has no mobile emulation");

    for (const route of ALL_ROUTES) {
      test(`${route.name} (${route.url}): no header item overlaps another and nothing scrolls sideways`, async ({ page }) => {
        await gotoHydrated(page, `${BASE_URL}${route.url}`);
        await expectNoHorizontalOverflow(page, width);
        await expectHeaderItemsClear(page, width, 0);
      });
    }
  });
}
