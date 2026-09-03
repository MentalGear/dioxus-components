import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

const BASE_URL = "http://127.0.0.1:8080";
const SIDEBAR_RENDER_TIMEOUT = 30 * 1000;

async function gotoSidebarBlock(page: Page) {
  await page.goto(`${BASE_URL}/component/block/?name=sidebar&variant=main&`, {
    timeout: 20 * 60 * 1000,
    waitUntil: 'load'
  });

  await expect(page.locator('[data-slot="sidebar-wrapper"]')).toBeVisible({
    timeout: SIDEBAR_RENDER_TIMEOUT,
  });
}

test("sidebar: preview page renders block", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=sidebar&`, {
    timeout: 20 * 60 * 1000,
    waitUntil: 'load'
  });
  const iframe = page.locator("iframe").first();
  await expect(iframe).toBeVisible({ timeout: SIDEBAR_RENDER_TIMEOUT });
  await expect(iframe).toHaveAttribute(
    "src",
    /component\/block\/\?name=sidebar&variant=main/,
    { timeout: SIDEBAR_RENDER_TIMEOUT },
  );

  await expect(
    page.frameLocator("iframe").first().locator('[data-slot="sidebar-wrapper"]'),
  ).toBeVisible({ timeout: SIDEBAR_RENDER_TIMEOUT });
});

test.describe("sidebar: block route", () => {
  test("desktop: toggles via button and Ctrl+B", async ({ page }) => {
    await gotoSidebarBlock(page);

    const sidebar = page.locator('[data-slot="sidebar"]:not([data-mobile="true"])');
    await expect(sidebar).toHaveAttribute("data-state", "expanded");
    const trigger = page.locator('[data-slot="sidebar-trigger"]');
    await expect(trigger).toHaveAccessibleName("Toggle Sidebar");

    // Toggle via button.
    await trigger.click();
    await expect(sidebar).toHaveAttribute("data-state", "collapsed");
    await trigger.click();
    await expect(sidebar).toHaveAttribute("data-state", "expanded");

    // Toggle via keyboard shortcut (⌘/Ctrl+B).
    await page.keyboard.press("Control+b");
    await expect(sidebar).toHaveAttribute("data-state", "collapsed");
    await page.keyboard.press("Control+b");
    await expect(sidebar).toHaveAttribute("data-state", "expanded");
  });

  test("desktop: side switch updates data-side", async ({ page }) => {
    await gotoSidebarBlock(page);

    const sidebar = page.locator('[data-slot="sidebar"]:not([data-mobile="true"])');
    await expect(sidebar).toHaveAttribute("data-side", "left");

    await page.getByRole("button", { name: "Right" }).click();
    await expect(sidebar).toHaveAttribute("data-side", "right");
    await page.getByRole("button", { name: "Left" }).click();
    await expect(sidebar).toHaveAttribute("data-side", "left");
  });

  test("desktop: icon collapse shows tooltip on focus and preserves accessible names", async ({
    page,
  }) => {
    await gotoSidebarBlock(page);

    const sidebar = page.locator('[data-slot="sidebar"]:not([data-mobile="true"])');
    const trigger = page.locator('[data-slot="sidebar-trigger"]');

    await page.getByRole("button", { name: "Icon" }).click();
    await trigger.click();

    await expect(sidebar).toHaveAttribute("data-state", "collapsed");
    await expect(sidebar).toHaveAttribute("data-collapsible", "icon");

    // In icon-collapsed mode, tooltips should appear on keyboard focus.
    const playground = page
      .locator('[data-sidebar="menu-button"]')
      .filter({ hasText: "Playground" })
      .first();

    await playground.focus();

    const tooltip = page.getByRole("tooltip");
    await expect(tooltip).toBeVisible();
    await expect(tooltip).toContainText("Playground");

    // Even when labels are visually hidden in icon mode, the control should still have an accessible name.
    await expect(playground).toHaveAccessibleName("Playground");
  });

  test("mobile: opens as a sheet and closes with Escape (focus restored)", async ({
    page,
  }) => {
    await gotoSidebarBlock(page);

    const trigger = page.locator('[data-slot="sidebar-trigger"]');
    await trigger.tap();

    const sheet = page.locator('[data-slot="sheet-root"]');
    await expect(sheet).toHaveAttribute("data-state", "open");
    await page.keyboard.press("Escape");
    await expect(sheet).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });
});

// KNOWN RED, filed rather than fixed (docs/backlog.md): the `/component/
// block/?name=sidebar&variant=main&` route (this file's `gotoSidebarBlock`)
// is a bare block fixture with no page-level `<h1>`/`<main>` wrapper of its
// own -- by design, it is normally embedded in an `<iframe>` from the main
// preview page (see the "preview page renders block" test above), where
// that absence is a non-issue (the iframe's own document is a separate
// accessibility-tree root). Visiting it directly, as both tests below do,
// surfaces `page-has-heading-one`/`region` for exactly that reason -- a
// product decision on whether block routes should carry a lightweight
// semantic wrapper for direct-visit accessibility, not a Sidebar defect.
test.describe("Axe automated scan", () => {
  test("loaded (expanded) has no automatically detectable a11y issues", async ({ page }) => {
    await gotoSidebarBlock(page);
    await expectNoAxeViolations(page, "sidebar: expanded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("collapsed has no automatically detectable a11y issues", async ({ page }) => {
    await gotoSidebarBlock(page);
    const sidebar = page.locator('[data-slot="sidebar"]:not([data-mobile="true"])');
    const trigger = page.locator('[data-slot="sidebar-trigger"]');
    await trigger.click();
    await expect(sidebar).toHaveAttribute("data-state", "collapsed");
    await expectNoAxeViolations(page, "sidebar: collapsed", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
