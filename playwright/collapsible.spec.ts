import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=collapsible&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  const preview = page.locator("#component-preview-frame").first();
  await page.getByRole("button", { name: "Recent Activity" }).click();
  await expect(preview.getByText("Fixed a bug in the collapsible component")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  test("loaded (collapsed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=collapsible&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "collapsible: collapsed", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("expanded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=collapsible&`, { timeout: 20 * 60 * 1000 });
    const preview = page.locator("#component-preview-frame").first();
    await page.getByRole("button", { name: "Recent Activity" }).click();
    await expect(preview.getByText("Fixed a bug in the collapsible component")).toBeVisible();
    await expectNoAxeViolations(page, "collapsible: expanded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

/**
 * `hidden_until_found` (backlog row 144): closed content stays mounted as `hidden="until-found"`
 * so find-in-page and `#fragment` navigation can reach and open it. Driven through the
 * `until_found` variant ("Searchable activity").
 *
 * Headless Chromium's `window.find` finds `until-found` text but does NOT fire `beforematch` for
 * it (only fragment navigation does, see dev-docs/research/virtualization-survey-2026-10-04.md
 * section 1.9), so the reveal is driven two ways: a real `#fragment` navigation (the browser
 * fires `beforematch` and removes the attribute itself) and a synthetic `beforematch`. Real Ctrl+F
 * in a headed browser takes the same code path as the fragment; that part is checked by hand.
 */
test.describe("hidden_until_found", () => {
  const URL = `${BASE_URL}/component/?name=collapsible&`;
  const TEXT = "Shipped the quokka tracker inside the closed panel";

  async function load(page: Page) {
    await gotoHydrated(page, URL, { timeout: 20 * 60 * 1000 });
    const trigger = page.getByRole("button", { name: "Searchable activity" });
    const panel = page.locator(".dx-collapsible-content", { hasText: "quokka tracker" });
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    return { trigger, panel };
  }

  /** Give an element inside the panel an id the page can navigate to with a `#fragment`. */
  async function markInside(panel: Locator, id: string) {
    await panel.evaluate((el, id) => {
      (el.firstElementChild ?? el).id = id;
    }, id);
  }

  test("closed content is mounted as hidden=until-found only when opted in", async ({ page }) => {
    const { panel } = await load(page);
    await expect(panel).toHaveCount(1);
    await expect(panel).toHaveAttribute("hidden", "until-found");
    await expect(panel).toHaveAttribute("data-open", "false");
    expect(await panel.evaluate((el) => el.textContent)).toContain(TEXT);
    // Skipped by layout and paint like display:none, but still a real box for the find-in-page.
    expect(
      await panel.evaluate((el) => {
        const s = getComputedStyle(el);
        const r = el.getBoundingClientRect();
        return { cv: s.contentVisibility, display: s.display, w: r.width, h: r.height };
      }),
    ).toEqual({ cv: "hidden", display: "block", w: 0, h: 0 });

    // The default (main) demo unmounts its closed content: the text is not in the DOM at all.
    await expect(
      page.locator(".dx-collapsible-content", { hasText: "Fixed a bug in the collapsible component" }),
    ).toHaveCount(0);
  });

  test("window.find reaches closed until-found text and not unmounted text", async ({ page }) => {
    const { panel } = await load(page);
    // "tracker" occurs only inside the closed panel (the hint says "quokka").
    expect(await page.evaluate(() => window.find("tracker"))).toBe(true);
    // The unmounted default demo's text is not findable.
    expect(await page.evaluate(() => window.find("Fixed a bug in the collapsible component"))).toBe(false);
    await expect(panel).toHaveCount(1);
  });

  test("a #fragment into closed content reveals it and the collapsible opens", async ({ page }) => {
    const { trigger, panel } = await load(page);
    await markInside(panel, "quokka-target");
    await page.evaluate(() => {
      location.hash = "#quokka-target";
    });

    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await expect(panel).toHaveAttribute("data-open", "true");
    await expect(panel).not.toHaveAttribute("hidden", /.*/);
    await expect(page.locator("#quokka-target")).toBeVisible();
  });

  test("a beforematch on the content opens the collapsible", async ({ page }) => {
    const { trigger, panel } = await load(page);
    await panel.dispatchEvent("beforematch");

    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await expect(panel).toHaveAttribute("data-open", "true");
    await expect(panel).not.toHaveAttribute("hidden", /.*/);
  });

  test("a beforematch on a descendant does not open it (only the content element counts)", async ({ page }) => {
    const { trigger, panel } = await load(page);
    await panel.evaluate((el) => el.firstElementChild!.dispatchEvent(new Event("beforematch", { bubbles: true })));
    await page.waitForTimeout(300);
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await expect(panel).toHaveAttribute("hidden", "until-found");
  });

  test("closing again restores hidden=until-found", async ({ page }) => {
    const { trigger, panel } = await load(page);
    await trigger.click();
    await expect(panel).not.toHaveAttribute("hidden", /.*/);
    await expect(page.getByText(TEXT)).toBeVisible();

    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await expect(panel).toHaveAttribute("hidden", "until-found");
    await expect(page.getByText(TEXT)).toBeHidden();
    expect(await panel.evaluate((el) => el.textContent)).toContain(TEXT);
  });

  test("a reveal after the user closed it opens it again", async ({ page }) => {
    const { trigger, panel } = await load(page);
    await markInside(panel, "quokka-target");
    await trigger.click();
    await trigger.click();
    await expect(panel).toHaveAttribute("hidden", "until-found");
    await page.evaluate(() => {
      location.hash = "#quokka-target";
    });
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
  });

  test("listens exactly once, through toggles, and not at all after unmount", async ({ page }) => {
    const { trigger, panel } = await load(page);
    const cdp = await page.context().newCDPSession(page);
    const beforematchListeners = async (expression: string) => {
      const { result } = await cdp.send("Runtime.evaluate", { expression });
      const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId! });
      return listeners.filter((l) => l.type === "beforematch").length;
    };
    await panel.evaluate((el) => {
      (window as any).__panel = el;
    });
    expect(await beforematchListeners("window.__panel")).toBe(1);

    for (let i = 0; i < 3; i++) {
      await trigger.click();
      await expect(trigger).toHaveAttribute("aria-expanded", "true");
      await trigger.click();
      await expect(trigger).toHaveAttribute("aria-expanded", "false");
    }
    expect(await beforematchListeners("window.__panel")).toBe(1);

    // Client-side navigation to another component page unmounts this one.
    await page.locator('a[href="/component/kbd/?"]').first().click();
    await expect(page).toHaveURL(/\/component\/kbd\//);
    await expect(panel).toHaveCount(0);
    await expect.poll(() => page.evaluate(() => (window as any).__panel.isConnected)).toBe(false);
    expect(await beforematchListeners("window.__panel")).toBe(0);
  });
});

/**
 * The side finding fixed alongside `hidden_until_found`: `keep_mounted` closed content used to be
 * rendered with no `hidden` at all, and the styled layer's `display: contents` then showed it.
 * The preview has no `keep_mounted` demo, so this exercises the stylesheet's contract on the
 * markup the primitive now renders (`hidden=""` for `keep_mounted`, `hidden="until-found"` for
 * the opt-in); the primitive half is covered by `collapsible.rs`'s SSR unit tests.
 */
test.describe("styled content wrapper honours hidden", () => {
  test("plain hidden is display:none even though the wrapper is display:contents", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=collapsible&`, { timeout: 20 * 60 * 1000 });
    const computed = await page.evaluate(() => {
      const make = (hidden: string | null) => {
        const el = document.createElement("div");
        el.className = "dx-collapsible-content";
        if (hidden !== null) el.setAttribute("hidden", hidden);
        el.textContent = "probe";
        document.body.appendChild(el);
        const s = getComputedStyle(el);
        const out = { display: s.display, position: s.position, cv: s.contentVisibility };
        el.remove();
        return out;
      };
      return { shown: make(null), hidden: make(""), untilFound: make("until-found") };
    });
    expect(computed.shown.display).toBe("contents");
    expect(computed.hidden.display).toBe("none");
    expect(computed.untilFound).toEqual({ display: "block", position: "absolute", cv: "hidden" });
  });
});
