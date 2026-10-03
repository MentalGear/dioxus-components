import { test, expect } from "./fixtures";
import { gotoHydrated } from "./hydration";
import { expectNoAxeViolations } from "./axe";
import { BASE_URL } from "./base-url";
import fs from "node:fs";
import path from "node:path";

// The `/charts/` gallery (`preview/src/charts_gallery.rs`): one tab per chart
// type, every chart demo as a card with a Copy button and a "View Code"
// sheet/drawer. Each tab is its own prerendered route (`/charts/<slug>/`,
// `/charts/` = the Area tab), so this runs against the SSG lane (or `dx
// serve`); start the server first and point PLAYWRIGHT_BASE_URL at it.

// The page keeps a classic-scrollbar gutter while a modal is open
// (`scrollbar-gutter: stable`, main.css), which a fixed dialog's right edge
// stops short of by the scrollbar width (15px in headless Chromium).
const SCROLLBAR_SLACK = 16;

const COMPONENTS = path.join(__dirname, "../preview/src/components");

// Minimum card counts: shadcn's own per-tab counts, which every tab of ours
// has at least (the demos behind the cards can only grow).
const TABS = [
  { slug: "area", label: "Area Charts", demo: "area_chart", min: 10, hero: true },
  { slug: "bar", label: "Bar Charts", demo: "bar_chart", min: 11, hero: true },
  { slug: "line", label: "Line Charts", demo: "line_chart", min: 10, hero: true },
  { slug: "pie", label: "Pie Charts", demo: "pie_chart", min: 11, hero: false },
  { slug: "radar", label: "Radar Charts", demo: "radar_chart", min: 14, hero: false },
  { slug: "radial", label: "Radial Charts", demo: "radial_chart", min: 6, hero: false },
  { slug: "tooltip", label: "Tooltips", demo: "chart_tooltip", min: 9, hero: false },
] as const;

for (const tab of TABS) {
  test(`/charts/${tab.slug}/ renders its cards with the toolbar chrome`, async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/charts/${tab.slug}/`);

    await expect(page.getByRole("heading", { level: 1 })).toHaveText(/Charts/);
    await expect(page.getByRole("heading", { level: 2, name: tab.label })).toBeAttached();

    const cells = page.locator(".dx-charts-cell");
    expect(await cells.count()).toBeGreaterThanOrEqual(tab.min);
    await expect(cells.first().locator('[data-slot="chart"]').first()).toBeVisible();

    // Every card: toolbar label, an icon-only Copy button, a View Code button.
    for (const cell of await cells.all()) {
      await expect(cell.locator(".dx-charts-toolbar-label")).not.toBeEmpty();
      await expect(cell.getByRole("button", { name: /^Copy code/ })).toBeVisible();
      await expect(cell.getByRole("button", { name: /^View Code/ })).toBeVisible();
      await expect(cell.locator(".dx-chart").first()).toBeAttached();
    }

    // The interactive hero card leads and spans the full width on Area/Bar/
    // Line; nowhere else is a card full-width.
    const spans = await cells.evaluateAll((els) => els.map((el) => el.getAttribute("data-span")));
    if (tab.hero) {
      await expect(cells.first()).toHaveAttribute("data-variant", "interactive");
      expect(spans[0]).toBe("full");
      expect(spans.slice(1).every((span) => span === "single")).toBe(true);
    } else {
      expect(spans.every((span) => span === "single")).toBe(true);
    }
  });
}

test("the tab row marks the current chart type with aria-current", async ({ page }) => {
  for (const tab of TABS) {
    await gotoHydrated(page, `${BASE_URL}/charts/${tab.slug}/`);
    const nav = page.getByRole("navigation", { name: "Chart types" });
    const links = nav.getByRole("link");
    await expect(links).toHaveCount(TABS.length);
    await expect(links.nth(TABS.indexOf(tab))).toHaveText(tab.label);
    await expect(nav.locator('[aria-current="page"]')).toHaveCount(1);
    await expect(nav.getByRole("link", { name: tab.label })).toHaveAttribute("aria-current", "page");
    await expect(nav.getByRole("link", { name: tab.label })).toHaveAttribute(
      "href",
      new RegExp(`/charts/${tab.slug}/\\??$`),
    );
  }
});

test("/charts/ is the Area tab, and tab links navigate between tabs", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/charts/`);
  const nav = page.getByRole("navigation", { name: "Chart types" });
  await expect(nav.getByRole("link", { name: "Area Charts" })).toHaveAttribute("aria-current", "page");
  await expect(page.locator('.dx-charts-cell[data-kind="area"]').first()).toBeVisible();

  await nav.getByRole("link", { name: "Pie Charts" }).click();
  await expect(page).toHaveURL(/\/charts\/pie\/\??$/);
  await expect(nav.getByRole("link", { name: "Pie Charts" })).toHaveAttribute("aria-current", "page");
  await expect(nav.getByRole("link", { name: "Area Charts" })).not.toHaveAttribute("aria-current", "page");
  await expect(page.locator('.dx-charts-cell[data-kind="pie"]').first()).toBeVisible();
  await expect(page.locator('.dx-charts-cell[data-kind="area"]')).toHaveCount(0);
});

test("hero buttons, top nav and the chart component pages link to the gallery", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/charts/area/`);
  await expect(page.getByRole("link", { name: "Browse charts" })).toHaveAttribute("href", "#charts");
  await expect(page.getByRole("link", { name: "Documentation" })).toHaveAttribute(
    "href",
    /\/component\/chart\/\??$/,
  );
  await expect(
    page.getByRole("navigation", { name: "Primary" }).getByRole("link", { name: "Charts" }),
  ).toHaveAttribute("href", /\/charts\/\??$/);
  await expect(
    page.getByRole("navigation", { name: "Footer" }).getByRole("link", { name: "Charts" }),
  ).toHaveAttribute("href", /\/charts\/\??$/);

  await gotoHydrated(page, `${BASE_URL}/component/area_chart/`);
  await page.getByRole("link", { name: "Browse all charts" }).click();
  await expect(page).toHaveURL(/\/charts\/\??$/);
});

test("Copy writes the variant's full source to the clipboard", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await gotoHydrated(page, `${BASE_URL}/charts/area/`);

  const cell = page.locator('.dx-charts-cell[data-variant="gradient"]');
  const copy = cell.getByRole("button", { name: /^Copy code/ });
  await copy.scrollIntoViewIfNeeded();
  await copy.click();

  const expected = fs.readFileSync(
    path.join(COMPONENTS, "area_chart/variants/gradient/mod.rs"),
    "utf8",
  );
  await expect
    .poll(() => page.evaluate(() => navigator.clipboard.readText()))
    .toBe(expected);
  // The check icon confirms the copy, then resets.
  await expect(copy).toHaveAttribute("data-copied", "true");
  await expect(copy).toHaveAttribute("data-copied", "false");
});

test("Copy shows a 'Copy code' tooltip on hover", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/charts/pie/`);
  const copy = page.locator(".dx-charts-cell").first().getByRole("button", { name: /^Copy code/ });
  await copy.hover();
  await expect(page.getByRole("tooltip", { name: "Copy code" })).toBeVisible();
});

test("View Code opens the source sheet and Escape closes it, returning focus", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.setViewportSize({ width: 1280, height: 800 });
  await gotoHydrated(page, `${BASE_URL}/charts/area/`);

  const cell = page.locator('.dx-charts-cell[data-variant="gradient"]');
  const viewCode = cell.getByRole("button", { name: /^View Code/ });
  await viewCode.scrollIntoViewIfNeeded();
  await viewCode.click();

  const sheet = page.locator('[data-slot="sheet-root"]');
  await expect(sheet).toHaveAttribute("data-state", "open");
  const dialog = page.getByRole("dialog", { name: "area_chart/variants/gradient/mod.rs" });
  await expect(dialog).toBeVisible();
  // ~700px wide, flush to the right edge (once the slide-in has settled).
  await expect
    .poll(async () => {
      const box = (await dialog.boundingBox())!;
      const vw = await page.evaluate(() => window.innerWidth);
      return [Math.round(box.width), Math.abs(Math.round(box.x + box.width) - vw) <= SCROLLBAR_SLACK];
    })
    .toEqual([700, true]);

  // Highlighted source of THIS variant.
  await expect(dialog.locator(".dx-preview-code-theme")).toContainText("Area Chart - Gradient");
  await expect(dialog.locator(".dx-preview-code-theme .dxc span").first()).toBeAttached();

  // The sheet's own copy button copies the same text.
  await dialog.getByRole("button", { name: /^Copy code/ }).click();
  const expected = fs.readFileSync(
    path.join(COMPONENTS, "area_chart/variants/gradient/mod.rs"),
    "utf8",
  );
  await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe(expected);

  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(viewCode).toBeFocused();

  // The close button closes it too.
  await viewCode.click();
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(viewCode).toBeFocused();

  // ... and so does a click on the overlay.
  await viewCode.click();
  await expect(dialog).toBeVisible();
  await page.mouse.click(40, 400);
  await expect(dialog).toHaveCount(0);
  await expect(viewCode).toBeFocused();
});

test("View Code opens a bottom drawer on phones", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await gotoHydrated(page, `${BASE_URL}/charts/pie/`);

  const cell = page.locator('.dx-charts-cell[data-variant="donut"]');
  const viewCode = cell.getByRole("button", { name: /^View Code/ });
  await viewCode.scrollIntoViewIfNeeded();
  await viewCode.click();

  const drawer = page.locator('[data-slot="drawer-root"]');
  await expect(drawer).toHaveAttribute("data-state", "open");
  await expect(page.locator('[data-slot="sheet-root"]')).toHaveCount(0);
  const dialog = page.getByRole("dialog", { name: "pie_chart/variants/donut/mod.rs" });
  await expect(dialog).toBeVisible();
  // Anchored to the bottom edge, full width (once the slide-in has settled).
  await expect
    .poll(async () => {
      const box = (await dialog.boundingBox())!;
      const vw = await page.evaluate(() => window.innerWidth);
      return [Math.abs(Math.round(box.width) - vw) <= SCROLLBAR_SLACK, Math.round(box.y + box.height)];
    })
    .toEqual([true, 844]);

  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(viewCode).toBeFocused();
});

test("the code block scrolls horizontally inside the scroll-locked drawer", async ({ page }) => {
  // Long source lines must scroll sideways, not clip: the modal scroll lock
  // used to treat any purely horizontal wheel/touch gesture as a background
  // scroll and cancel it.
  await page.setViewportSize({ width: 390, height: 844 });
  await gotoHydrated(page, `${BASE_URL}/charts/pie/`);
  const viewCode = page
    .locator('.dx-charts-cell[data-variant="donut"]')
    .getByRole("button", { name: /^View Code/ });
  await viewCode.scrollIntoViewIfNeeded();
  await viewCode.click();
  const body = page.locator(".dx-charts-code-body");
  await expect(body).toBeVisible();
  // Let the slide-in settle so the pointer lands on the body, not mid-travel.
  const dialog = page.getByRole("dialog", { name: "pie_chart/variants/donut/mod.rs" });
  await expect
    .poll(async () => Math.round(((await dialog.boundingBox())!).y + ((await dialog.boundingBox())!).height))
    .toBe(844);
  expect(await body.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(true);
  const box = (await body.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(200, 0);
  await expect.poll(() => body.evaluate((el) => el.scrollLeft)).toBeGreaterThan(0);
});

test("no horizontal page overflow; the tab row scrolls inside itself", async ({ page }) => {
  for (const width of [1440, 1024, 768, 390, 320]) {
    await page.setViewportSize({ width, height: 900 });
    await gotoHydrated(page, `${BASE_URL}/charts/area/`);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow, `page overflows horizontally at ${width}px`).toBeLessThanOrEqual(0);
    const columns = await page.evaluate(
      () =>
        getComputedStyle(document.querySelector(".dx-charts-grid")!).gridTemplateColumns.split(" ")
          .length,
    );
    expect(columns).toBe(width >= 1024 ? 3 : width >= 768 ? 2 : 1);
  }
  const tabs = page.locator(".dx-charts-tabs");
  const scrolls = await tabs.evaluate((el) => el.scrollWidth > el.clientWidth);
  expect(scrolls).toBe(true);
});

test.describe("Axe automated scan", () => {
  test("a gallery tab has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/charts/area/`);
    await expectNoAxeViolations(page, "charts gallery: area");
  });

  test("the pie tab has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/charts/pie/`);
    await expectNoAxeViolations(page, "charts gallery: pie");
  });

  test("the open code sheet has no automatically detectable a11y issues", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await gotoHydrated(page, `${BASE_URL}/charts/area/`);
    await page
      .locator('.dx-charts-cell[data-variant="gradient"]')
      .getByRole("button", { name: /^View Code/ })
      .click();
    await expect(page.locator('[data-slot="sheet-root"]')).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "charts gallery: code sheet open");
  });
});
