/**
 * The "Extra" badge: components that are not in shadcn/ui's catalog carry it
 * on the component page header, in their sidebar entry and on their homepage
 * card; shadcn components carry none.
 *
 * ORACLE: the list itself. `CATALOG` in preview/src/components/mod.rs is the
 * single source of truth for which component is an extra, and this spec READS
 * it (the `("name", Origin::Shadcn|Extra),` rows) instead of keeping a second
 * copy, so a new row is tested without an edit here and the badge cannot drift
 * from the list. (`scripts/check-component-catalog.sh` keeps the list complete
 * against the directories; this file keeps the UI faithful to the list.)
 *
 * What is asserted, beyond "the badge is there":
 *  - the text is real ("Extra") and carries the tooltip explanation as `title`;
 *  - it is a SIBLING of the page `h1` and the card `h3`, so those headings keep
 *    the component's own name as their accessible name;
 *  - nothing overflows at 390px (page header, mobile sidebar sheet);
 *  - axe is clean with the badge in place, light and dark.
 */
import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

const TITLE = "Not part of shadcn/ui — a dioxus-components addition";

// --- the list, read from Rust --------------------------------------------------------------
const MOD_RS = fs.readFileSync(path.join(__dirname, "..", "preview", "src", "components", "mod.rs"), "utf8");
const CATALOG_BLOCK = MOD_RS.split("pub const CATALOG")[1]?.split("\n];")[0] ?? "";
const ROWS = [...CATALOG_BLOCK.matchAll(/^\s*\("([a-z0-9_]+)", Origin::(Shadcn|Extra)\),/gm)].map((m) => ({
  name: m[1],
  extra: m[2] === "Extra",
}));
const EXTRAS = ROWS.filter((r) => r.extra).map((r) => r.name);
const SHADCN = ROWS.filter((r) => !r.extra).map((r) => r.name);
/** Not rendered on the homepage gallery (`ComponentGallery` filters the oracle fixture out). */
const NOT_IN_GALLERY = new Set(["top_layer"]);

const display = (name: string) => name.replace(/_/g, " ");
const componentUrl = (name: string) => `${BASE_URL}/component/?name=${name}&`;

/** The docs sidebar's entry for a component (`Route::component` -> `/component/<name>/`). */
const sidebarEntry = (page: Page, name: string) => page.locator(`a[data-sidebar="menu-button"][href*="/component/${name}/"]`);

/** The homepage renders every demo (75 cards); wait until the gallery is really there. */
async function openHome(page: Page) {
  await gotoHydrated(page, `${BASE_URL}/`, { timeout: 20 * 60 * 1000 });
  await expect(page.locator("article.dx-component-card")).not.toHaveCount(0, { timeout: 120_000 });
}

async function openComponent(page: Page, name: string) {
  await page.goto(componentUrl(name), { timeout: 20 * 60 * 1000 });
  // The legacy `?name=` route redirects to the canonical path route once mounted.
  await expect(page.locator(".dx-component-page-header h1")).toHaveText(display(name), { timeout: 60_000 });
}

test("the list parses: both kinds are present and the extras are the known eight", () => {
  expect(ROWS.length).toBeGreaterThan(60);
  expect(SHADCN).toContain("button");
  // The chart gallery pages are shadcn's own `chart` sections, not additions.
  expect(SHADCN).toEqual(expect.arrayContaining(["chart", "area_chart", "chart_tooltip"]));
  expect(new Set(ROWS.map((r) => r.name)).size).toBe(ROWS.length);
  expect(EXTRAS.length).toBeGreaterThan(0);
});

test.describe("component page header", () => {
  test("an extra shows the badge next to its name, with the tooltip", async ({ page }) => {
    await openComponent(page, "toolbar");
    const header = page.locator(".dx-component-page-header");
    const badge = header.locator(".dx-extra-badge");
    await expect(badge).toHaveCount(1);
    await expect(badge).toBeVisible();
    await expect(badge).toHaveText("Extra");
    await expect(badge).toHaveAttribute("title", TITLE);
    // Real text in the accessibility tree, not an image or pseudo-element.
    await expect(header.getByText("Extra", { exact: true })).toBeVisible();
    // The `h1` keeps the component's own name: the badge is its sibling, not a child.
    const h1 = header.getByRole("heading", { level: 1 });
    await expect(h1).toHaveText("toolbar");
    await expect(h1.locator(".dx-extra-badge")).toHaveCount(0);
    // "Next to": same row, directly after the name.
    const [h, b] = await Promise.all([h1.boundingBox(), badge.boundingBox()]);
    expect(h && b, "both boxes laid out").toBeTruthy();
    expect(b!.x).toBeGreaterThanOrEqual(h!.x + h!.width - 1);
    expect(Math.abs(b!.y + b!.height / 2 - (h!.y + h!.height / 2))).toBeLessThan(h!.height / 2);
  });

  test("a shadcn component and a chart gallery page show no badge", async ({ page }) => {
    for (const name of ["button", "area_chart"]) {
      await openComponent(page, name);
      await expect(page.locator(".dx-component-page-header .dx-extra-badge"), name).toHaveCount(0);
      await expect(page.locator(".dx-component-page-header").getByText("Extra", { exact: true }), name).toHaveCount(0);
    }
  });

  test("every extra's page shows it; a sample of shadcn pages does not", async ({ page }) => {
    test.setTimeout(5 * 60 * 1000);
    // The sidebar and gallery tests below compare the whole list; the header is the third render site.
    for (const name of EXTRAS) {
      await openComponent(page, name);
      await expect(page.locator(".dx-component-page-header .dx-extra-badge"), name).toHaveCount(1);
    }
    for (const name of ["accordion", "message_scroller", "navigation_menu", "chart"]) {
      await openComponent(page, name);
      await expect(page.locator(".dx-component-page-header .dx-extra-badge"), name).toHaveCount(0);
    }
  });
});

test.describe("sidebar", () => {
  test("an extra's entry carries the badge; a shadcn entry does not", async ({ page }) => {
    await openComponent(page, "toolbar");
    const extra = sidebarEntry(page, "toolbar");
    await expect(extra).toHaveCount(1);
    await expect(extra.locator(".dx-extra-badge")).toHaveText("Extra");
    await expect(extra.locator(".dx-extra-badge")).toHaveAttribute("title", TITLE);
    // The badge is part of the link, so the link's name says so.
    await expect(extra).toHaveAccessibleName(/^toolbar\s+Extra$/);
    const shadcn = sidebarEntry(page, "button");
    await expect(shadcn).toHaveCount(1);
    await expect(shadcn.locator(".dx-extra-badge")).toHaveCount(0);
    await expect(shadcn).toHaveAccessibleName("button");
  });

  test("the sidebar badges are exactly the extras", async ({ page }) => {
    await openComponent(page, "button");
    const sidebar = page.locator('[data-slot="sidebar"]:not([data-mobile="true"])');
    // One entry per component, and a badge in precisely the extras' entries.
    const withBadge = await sidebar
      .locator('a[data-sidebar="menu-button"]')
      .evaluateAll((links) =>
        links
          .filter((a) => a.querySelector(".dx-extra-badge"))
          .map((a) => /\/component\/([a-z0-9_]+)\//.exec(a.getAttribute("href") ?? "")?.[1]),
      );
    expect(withBadge.sort()).toEqual([...EXTRAS].sort());
  });

  test("the badge stays inside its link, at the row's end", async ({ page }) => {
    await openComponent(page, "drag_and_drop_list"); // the longest extra name
    const link = sidebarEntry(page, "drag_and_drop_list");
    const badge = link.locator(".dx-extra-badge");
    await expect(badge).toBeVisible();
    const [l, b] = await Promise.all([link.boundingBox(), badge.boundingBox()]);
    expect(l && b).toBeTruthy();
    expect(b!.x).toBeGreaterThanOrEqual(l!.x);
    expect(b!.x + b!.width).toBeLessThanOrEqual(l!.x + l!.width + 0.5);
    // Not wrapped into the name's line: the badge sits right of the text.
    const text = await link.evaluate((a) => {
      const range = document.createRange();
      range.selectNodeContents(a.firstChild!);
      const r = range.getBoundingClientRect();
      return { right: r.right, top: r.top, bottom: r.bottom };
    });
    expect(b!.x).toBeGreaterThanOrEqual(text.right - 1);
    expect(b!.y).toBeLessThan(text.bottom);
  });
});

test.describe("homepage gallery card", () => {
  test("an extra's card has the badge beside its title; shadcn cards have none", async ({ page }) => {
    await openHome(page);
    const card = (name: string) =>
      page.locator("article.dx-component-card").filter({ has: page.getByRole("heading", { name: display(name), exact: true }) });

    const extra = card("toolbar");
    await expect(extra).toHaveCount(1);
    const badge = extra.locator(".dx-component-card-title-row .dx-extra-badge");
    await expect(badge).toHaveText("Extra");
    await expect(badge).toHaveAttribute("title", TITLE);
    // The `h3` keeps the component's name (a heading query by that exact name found the card).
    await expect(extra.getByRole("heading", { level: 3 }).locator(".dx-extra-badge")).toHaveCount(0);

    await expect(card("button")).toHaveCount(1);
    await expect(card("button").locator(".dx-extra-badge")).toHaveCount(0);
  });

  test("the cards with a badge are exactly the extras the gallery shows", async ({ page }) => {
    await openHome(page);
    const titles = await page.locator("article.dx-component-card").evaluateAll((cards) =>
      cards
        .filter((c) => c.querySelector(".dx-extra-badge"))
        .map((c) => (c.querySelector("h3")?.textContent ?? "").trim()),
    );
    expect(titles.sort()).toEqual(EXTRAS.filter((n) => !NOT_IN_GALLERY.has(n)).map(display).sort());
  });
});

test.describe("phone width (390px)", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  const noHorizontalScroll = (page: Page) =>
    page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth);

  test("the longest extra's page header wraps without overflowing", async ({ page }) => {
    await openComponent(page, "drag_and_drop_list");
    const badge = page.locator(".dx-component-page-header .dx-extra-badge");
    await expect(badge).toBeVisible();
    const box = await badge.boundingBox();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(390);
    expect(await noHorizontalScroll(page), "no horizontal page scroll").toBe(true);
  });

  test("the phone navigation sheet shows the badge inside its link", async ({ page }) => {
    await openComponent(page, "drag_and_drop_list");
    await page.getByRole("button", { name: "Toggle Sidebar" }).click();
    const sheet = page.locator(".dx-sidebar-sheet");
    await expect(sheet).toBeVisible();
    const link = sheet.locator('a[data-sidebar="menu-button"][href*="/component/drag_and_drop_list/"]');
    const badge = link.locator(".dx-extra-badge");
    await badge.scrollIntoViewIfNeeded();
    await expect(badge).toBeVisible();
    const [l, b] = await Promise.all([link.boundingBox(), badge.boundingBox()]);
    expect(b!.x).toBeGreaterThanOrEqual(l!.x);
    expect(b!.x + b!.width).toBeLessThanOrEqual(l!.x + l!.width + 0.5);
    expect(l!.x + l!.width).toBeLessThanOrEqual(390);
  });

  test("the homepage cards fit", async ({ page }) => {
    await openHome(page);
    const row = page
      .locator("article.dx-component-card")
      .filter({ has: page.getByRole("heading", { name: "drag and drop list", exact: true }) })
      .locator(".dx-component-card-title-row");
    await expect(row.locator(".dx-extra-badge")).toBeVisible();
    const box = await row.boundingBox();
    expect(box!.x + box!.width).toBeLessThanOrEqual(390);
    expect(await noHorizontalScroll(page), "no horizontal page scroll").toBe(true);
  });
});

test.describe("Axe automated scan", () => {
  for (const scheme of ["light", "dark"] as const) {
    test(`an extra's page with the badge (${scheme}) has no automatically detectable a11y issues`, async ({ page }) => {
      await openComponent(page, "toolbar");
      await page.evaluate((s) => document.documentElement.setAttribute("data-theme", s), scheme);
      // The page header badge and, in the sidebar, the active entry's badge.
      await expect(page.locator(".dx-extra-badge")).not.toHaveCount(0);
      await expectNoAxeViolations(page, `component-catalog: toolbar page, ${scheme}`, {
        excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
      });
    });
  }

  test("the homepage card titles with the badge have no automatically detectable a11y issues", async ({ page }) => {
    await openHome(page);
    await expect(page.locator(".dx-component-card-title-row .dx-extra-badge").first()).toBeVisible();
    await expectNoAxeViolations(page, "component-catalog: homepage card titles", {
      include: ".dx-component-card-title-row",
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
