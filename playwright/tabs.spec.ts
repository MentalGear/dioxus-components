import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=tabs&`);
  let activeTab = page.locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
    .filter({ hasText: /^Tab \d Content$/ });
  let tab1Button = page.getByRole("tab", { name: "Tab 1" });
  let tab2Button = page.getByRole("tab", { name: "Tab 2" });
  let tab3Button = page.getByRole("tab", { name: "Tab 3" });
  // Clicking the right arrow should focus the next tab trigger
  await tab1Button.click();
  await page.keyboard.press("ArrowRight");
  await expect(tab2Button).toBeFocused();

  // Clicking enter should activate the focused tab
  await page.keyboard.press("Enter");
  await expect(activeTab).toContainText("Tab 2 Content");

  // Clicking right twice more should bring us back to the first tab
  await page.keyboard.press("ArrowRight");
  await expect(tab3Button).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(tab1Button).toBeFocused();

  // Clicking each tab should activate it
  await tab3Button.click();
  await expect(activeTab).toContainText("Tab 3 Content");
  await tab2Button.click();
  await expect(activeTab).toContainText("Tab 2 Content");
  await tab1Button.click();
  await expect(activeTab).toContainText("Tab 1 Content");
});

test.describe("active tab keeps its color under the pointer (calendar-state lane sweep)", () => {
  // Same bug class as calendar/style.css's day hover (see calendar.spec.ts):
  // `.dx-tabs-trigger:hover` used to carry higher specificity than
  // `.dx-tabs-trigger[data-state="active"]`, so clicking a tab -- which
  // leaves the pointer resting on it -- silently dimmed its text to the
  // plain hover tint instead of the active color. Runs in both themes: this
  // one (unlike calendar's) reproduced in light mode too, since
  // --secondary-color-1 and --secondary-color-3 differ in both.
  for (const dark of [false, true]) {
    test(`clicking a tab keeps its active text color while still hovered (${dark ? "dark" : "light"} mode)`, async ({ page }) => {
      await page.goto(`${BASE_URL}/component/?name=tabs&${dark ? "dark_mode=true" : ""}`);
      const tab2Button = page.getByRole("tab", { name: "Tab 2" });
      const tab3Button = page.getByRole("tab", { name: "Tab 3" });

      // Tab colours now transition (Nova tabs, row 111), so every read waits for the
      // trigger's running animations to finish: a read taken mid-fade is an interpolated
      // colour, not either end state.
      const settledColor = (tab: typeof tab2Button) =>
        tab.evaluate(async (el) => {
          await Promise.all(el.getAnimations().map((a) => a.finished.catch(() => undefined)));
          return getComputedStyle(el).color;
        });

      // Click Tab 2 -- Playwright's .click() leaves the pointer on it.
      await tab2Button.click();
      await expect(tab2Button).toHaveAttribute("data-state", "active");
      expect(await tab2Button.evaluate((el) => el.matches(":hover"))).toBe(true);
      const activeAndHoveredColor = await settledColor(tab2Button);

      // Move the pointer off and read the "true" active color, plus the resting color of a
      // tab that stayed inactive throughout (Tab 3).
      await page.locator("body").hover({ position: { x: 0, y: 0 } });
      const activeAtRestColor = await settledColor(tab2Button);
      const inactiveAtRestColor = await settledColor(tab3Button);

      expect(activeAndHoveredColor).toBe(activeAtRestColor); // active color survives hover
      // Hover and active both resolve to `foreground` now, so "not the plain hover tint" can
      // no longer be observed; what must still hold is that the active tab is not rendered
      // in the inactive (muted) colour.
      expect(activeAndHoveredColor).not.toBe(inactiveAtRestColor);
    });
  }
});

test.describe("Axe automated scan", () => {
  test("loaded (tab 1 active) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=tabs&`);
    await expectNoAxeViolations(page, "tabs: tab 1 active", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tab 2 selected has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=tabs&`);
    await page.getByRole("tab", { name: "Tab 2" }).click();
    // Scoped with the same `.filter(...)` the file's own "test" test uses
    // above -- this page's "Variants" section renders a second, unrelated
    // Tabs instance with its own active tabpanel, so the bare selector
    // resolves to two elements (a Playwright strict-mode violation).
    await expect(
      page
        .locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
        .filter({ hasText: /^Tab \d Content$/ }),
    ).toContainText("Tab 2 Content");
    await expectNoAxeViolations(page, "tabs: tab 2 selected", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

/**
 * `hidden_until_found` (backlog row 144), driven through the `until_found` variant: Overview
 * (active), Specs and Reviews, inactive panels mounted as `hidden="until-found"`. A reveal by the
 * browser activates that panel's tab.
 *
 * Headless Chromium's `window.find` finds `until-found` text but does not fire `beforematch` for
 * it (only fragment navigation does), so the reveal is driven by a real `#fragment` navigation and
 * by a synthetic `beforematch`; real Ctrl+F takes the same path as the fragment and is checked by
 * hand in a headed browser.
 */
test.describe("hidden_until_found", () => {
  const URL = `${BASE_URL}/component/?name=tabs&`;
  const PANEL_TEXT: Record<string, string> = {
    Overview: "A small amphibian",
    Specs: "Regenerates limbs",
    Reviews: "Rated four stars",
  };

  async function load(page: Page) {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const tab = (name: string) => page.getByRole("tab", { name, exact: true });
    // The docs page wraps every demo in its own Tabs (Preview/Code), whose active panel also
    // contains this text: scope to the innermost Tabs root, the demo's own.
    const root = page.locator(".dx-tabs", { has: tab("Overview") }).last();
    const panelOf = (name: string) => root.locator(':scope > [role="tabpanel"]', { hasText: PANEL_TEXT[name] });
    await expect(tab("Overview")).toHaveAttribute("aria-selected", "true");
    return { tab, panelOf };
  }

  /** Give the content inside a panel an id the page can navigate to with a `#fragment`. */
  async function markInside(panel: Locator, id: string) {
    await panel.evaluate((el, id) => {
      el.firstElementChild!.id = id;
    }, id);
  }

  async function expectActive(tab: (n: string) => Locator, panelOf: (n: string) => Locator, active: string) {
    for (const name of Object.keys(PANEL_TEXT)) {
      const isActive = name === active;
      await expect(tab(name)).toHaveAttribute("aria-selected", String(isActive));
      await expect(tab(name)).toHaveAttribute("data-state", isActive ? "active" : "inactive");
      await expect(panelOf(name)).toHaveAttribute("data-state", isActive ? "active" : "inactive");
      if (isActive) await expect(panelOf(name)).not.toHaveAttribute("hidden", /.*/);
      else await expect(panelOf(name)).toHaveAttribute("hidden", "until-found");
    }
  }

  test("inactive panels are mounted as hidden=until-found only when opted in", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    await expectActive(tab, panelOf, "Overview");
    for (const name of ["Specs", "Reviews"]) {
      await expect(panelOf(name)).toHaveCount(1);
      expect(await panelOf(name).evaluate((el) => el.textContent)).toContain(PANEL_TEXT[name]);
    }

    // The default demo unmounts inactive panels: an empty, plain `hidden` element, no content.
    await expect(page.locator('[role="tabpanel"]', { hasText: "Tab 2 Content" })).toHaveCount(0);
    await expect(
      page.locator('[role="tabpanel"][data-state="inactive"][hidden=""]').filter({ hasNotText: /\S/ }).first(),
    ).toBeAttached();
    // And find-in-page reaches the opted-in text but not the unmounted text.
    expect(await page.evaluate(() => window.find("axolotl"))).toBe(true);
    expect(await page.evaluate(() => window.find("Tab 2 Content"))).toBe(false);
  });

  test("an inactive until-found panel is skipped but stays a real box taking no space", async ({ page }) => {
    const { panelOf } = await load(page);
    for (const name of ["Specs", "Reviews"]) {
      expect(
        await panelOf(name).evaluate((el) => {
          const s = getComputedStyle(el);
          const r = el.getBoundingClientRect();
          return {
            cv: s.contentVisibility,
            displayNone: s.display === "none",
            position: s.position,
            padding: s.paddingTop,
            border: s.borderTopWidth,
            w: r.width,
            h: r.height,
          };
        }),
      ).toEqual({ cv: "hidden", displayNone: false, position: "absolute", padding: "0px", border: "0px", w: 0, h: 0 });
    }
  });

  test("a #fragment into an inactive panel reveals it and activates its tab", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    await markInside(panelOf("Specs"), "axolotl-target");
    await page.evaluate(() => {
      location.hash = "#axolotl-target";
    });

    await expectActive(tab, panelOf, "Specs");
    await expect(page.locator("#axolotl-target")).toBeVisible();
    await expect(tab("Specs")).toHaveAttribute("tabindex", "0");
    await expect(tab("Overview")).toHaveAttribute("tabindex", "-1");
  });

  test("a beforematch on an inactive panel activates its tab", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    await panelOf("Reviews").dispatchEvent("beforematch");
    await expectActive(tab, panelOf, "Reviews");
  });

  test("a beforematch on the already-active panel changes nothing", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    await panelOf("Overview").dispatchEvent("beforematch");
    await page.waitForTimeout(300);
    await expectActive(tab, panelOf, "Overview");
  });

  test("clicking tabs afterwards still swaps panels and restores hidden=until-found", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    await panelOf("Specs").dispatchEvent("beforematch");
    await expectActive(tab, panelOf, "Specs");

    await tab("Overview").click();
    await expectActive(tab, panelOf, "Overview");
    await tab("Reviews").click();
    await expectActive(tab, panelOf, "Reviews");
  });

  test("listens exactly once per panel, through switches, and not at all after unmount", async ({ page }) => {
    const { tab, panelOf } = await load(page);
    const cdp = await page.context().newCDPSession(page);
    const beforematchListeners = async (expression: string) => {
      const { result } = await cdp.send("Runtime.evaluate", { expression });
      const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId! });
      return listeners.filter((l) => l.type === "beforematch").length;
    };
    await panelOf("Specs").evaluate((el) => {
      (window as any).__panel = el;
    });
    expect(await beforematchListeners("window.__panel")).toBe(1);

    for (let i = 0; i < 3; i++) {
      await tab("Specs").click();
      await expect(tab("Specs")).toHaveAttribute("aria-selected", "true");
      await tab("Overview").click();
      await expect(tab("Overview")).toHaveAttribute("aria-selected", "true");
    }
    expect(await beforematchListeners("window.__panel")).toBe(1);

    await page.locator('a[href="/component/kbd/?"]').first().click();
    await expect(page).toHaveURL(/\/component\/kbd\//);
    await expect.poll(() => page.evaluate(() => (window as any).__panel.isConnected)).toBe(false);
    expect(await beforematchListeners("window.__panel")).toBe(0);
  });
});

// The `Ghost` variant (`data-variant="ghost"`, the DEMO/CODE tabs on every component page) is shadcn's `line`
// variant: a transparent list, and the active tab carries a 2px underline that moves with the selection.
// Before this nothing styled `ghost`, so the active tab differed from the others by a shade of text colour.
test.describe("ghost variant is a line: the active tab carries the underline", () => {
  const underline = (tab: Locator) =>
    tab.evaluate((el) => {
      const after = getComputedStyle(el, "::after");
      return { opacity: Number(after.opacity), height: after.height, content: after.content };
    });

  test("the underline follows the selected tab and the list has no fill", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=button&`, { timeout: 20 * 60 * 1000 });
    const list = page.locator('[data-variant="ghost"] .dx-tabs-list').first();
    const demo = list.getByRole("tab", { name: "DEMO" });
    const code = list.getByRole("tab", { name: "CODE" });
    await expect(demo).toHaveAttribute("aria-selected", "true");

    await expect(list).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(demo).toHaveCSS("box-shadow", "none");
    await expect.poll(() => underline(demo)).toMatchObject({ opacity: 1, height: "2px" });
    await expect.poll(() => underline(code)).toMatchObject({ opacity: 0, height: "2px" });

    await code.click();
    await expect(code).toHaveAttribute("aria-selected", "true");
    await expect.poll(() => underline(code)).toMatchObject({ opacity: 1 });
    await expect.poll(() => underline(demo)).toMatchObject({ opacity: 0 });
  });
});
