import { test, expect } from "./fixtures";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/**
 * The site's name and icon (backlog row 161). The name is `SITE_NAME` in `preview/src/main.rs` and the
 * static shell's `<title>` in `preview/index.html` (a unit test pins the two together); the icons are
 * `document::Link`s built from `asset!()`, so they carry the build's `--base-path`.
 */
const PAGES = [
  ["home", "/"],
  ["a component page", "/component/button/"],
  ["the docs page", "/docs"],
  ["a charts page", "/charts/area/"],
] as const;

for (const [label, path] of PAGES) {
  test(`${label} is titled with the site name and links its icons`, async ({ page, request }) => {
    await gotoHydrated(page, `${BASE_URL}${path}`);
    // The charts routes set `<Kind> Charts – shadcn-dioxus`; every other page keeps the shell's title.
    await expect(page).toHaveTitle(/shadcn-dioxus/);
    await expect(page).not.toHaveTitle(/dioxus \| /);

    const icons = await page.locator('head link[rel="icon"], head link[rel="apple-touch-icon"]').evaluateAll(
      (els) => els.map((e) => ({ rel: e.getAttribute("rel"), type: e.getAttribute("type"), href: (e as HTMLLinkElement).href })),
    );
    expect(icons.map((i) => i.type ?? i.rel).sort()).toEqual(["apple-touch-icon", "image/png", "image/svg+xml"]);
    for (const icon of icons) {
      const res = await request.get(icon.href);
      expect(res.status(), icon.href).toBe(200);
    }

    const svg = icons.find((i) => i.type === "image/svg+xml")!;
    const body = await (await request.get(svg.href)).text();
    expect(body, "the SVG follows the colour scheme itself").toContain("prefers-color-scheme");

    await expect(page.locator('head meta[property="og:site_name"]')).toHaveAttribute("content", "shadcn-dioxus");
    await expect(page.locator('head meta[name="description"]')).toHaveAttribute("content", /\S{10}/);
    await expect(page.locator('head meta[property="og:image"]')).toHaveAttribute("content", /^https:\/\/.+\.png$/);
  });
}

test("the header brand reads the site name", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/`);
  await expect(page.locator(".dx-navbar-brand")).toHaveText("shadcn-dioxus");
});
