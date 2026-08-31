/**
 * ORACLE: tier 2 (HTML) — top layer (`popover` attribute).
 *
 * Source: docs/plan.md Phase 4.4, drawing on the WHATWG HTML Living
 * Standard's popover chapter:
 *   - The `popover` attribute, its `auto`/`manual` states, and the implied
 *     top-layer promotion:
 *     https://html.spec.whatwg.org/multipage/popover.html#the-popover-attribute
 *   - Light dismiss (Escape, outside pointerdown, closing sibling `auto`
 *     popovers) for `popover="auto"`:
 *     https://html.spec.whatwg.org/multipage/popover.html#popover-light-dismiss
 *   - `showPopover()`/`hidePopover()` and the `toggle` event fired on every
 *     state change (browser- or script-driven alike):
 *     https://html.spec.whatwg.org/multipage/popover.html#dom-showpopover
 *     https://html.spec.whatwg.org/multipage/popover.html#dom-hidepopover
 *   - The top layer itself (a rendering surface above the document that
 *     escapes ancestor overflow/transform/stacking contexts):
 *     https://html.spec.whatwg.org/multipage/rendering.html#the-top-layer
 *
 * Fixture: preview/src/components/top_layer/component.rs (`TopLayerFixture`),
 * served at /component/?name=top_layer. Every library control there is
 * paired with a native reference control — a plain `<div popover="auto">`
 * shown by a `<button popovertarget>` — the browser's own implementation of
 * the identical WHATWG HTML feature, with zero Dioxus involvement.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): every rule below
 * runs against the native reference first (prefixed CALIBRATION: in the
 * test name) and the library component second. A CALIBRATION failure means
 * the *test* is wrong, not the component.
 *
 * Per-component dismissal semantics (docs/plan.md Phase 4.4's own
 * requirement to "document the choice per component"):
 *   - `Tooltip`/`HoverCard` render with `popover="manual"` on the web arm.
 *     Both already own their entire open/close lifecycle through
 *     hover/focus (and, for Tooltip, an explicit Escape handler on the
 *     trigger) — MDN's own naming ("hover card") does not imply the light
 *     dismiss a click-triggered popover has, and `auto`'s outside-pointerdown
 *     dismissal would race that existing lifecycle rather than usefully
 *     extend it. So rules 2 ("light dismiss") and 3 ("Escape") below apply
 *     only to the non-modal `Popover` arm (`popover="auto"`) and its native
 *     reference — see primitives/src/top_layer.rs's `PopoverKind` doc for
 *     the full reasoning.
 *   - The non-modal `Popover` arm renders as `<dialog popover="auto">`,
 *     getting light dismiss for free from the platform; this file's rules
 *     2-3 are exactly what checks that the WHATWG-mandated browser behaviour
 *     (`toggle` event) still lands back in the Rust `open` signal, per
 *     primitives/src/top_layer.rs's `use_popover_sync` (the same defect
 *     class documented for `<dialog>`'s old one-way binding in
 *     docs/recommended-implementations.md Caveat 1).
 *   - Rule 1 ("clipping escape") and rule 4 ("stacking") are top-layer
 *     properties every promoted element gets regardless of its
 *     `popover="auto"|"manual"` kind, so all three components (Tooltip,
 *     HoverCard, Popover) are checked for rule 1; rule 4 uses `Popover`
 *     (the click-triggered case is the easiest to drive deterministically).
 *
 * Rules implemented (see this session's report for the full red/green
 * ledger per rule per component, both before and after the Phase 4.4
 * implementation landed):
 *   1. Clipping escape — component content inside an ancestor with
 *      `overflow: hidden` + `transform` + a height shorter than the
 *      content renders fully visible outside the clip.
 *   2. Light dismiss for `popover="auto"` — click outside closes it AND
 *      Rust state (`data-state` on the `PopoverRoot`) syncs.
 *   3. Escape closes `popover="auto"` AND Rust state syncs.
 *   4. Top-layer stacking — a popover opened after a high-z-index sibling
 *      renders above it.
 */

import { test, expect, type Page } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const gotoFixture = (page: Page) =>
  page.goto("http://127.0.0.1:8080/component/?name=top_layer&", {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/**
 * The real discriminating test for "does this content escape the clip,"
 * not just "is its geometric box taller than the clip." A normal (non-top-
 * layer) DOM child's `getBoundingClientRect()` reports its full laid-out
 * size regardless of an ancestor's `overflow: hidden` -- clipping only
 * affects *paint*, not layout geometry -- so a box-size comparison alone
 * cannot tell a genuinely-escaped element from one that is simply present
 * but invisible past the clip. `document.elementFromPoint()` at a
 * coordinate inside the content's own box but outside the clip ancestor's
 * box resolves to whatever is actually *painted* there: the content itself
 * if it escaped (top layer), or whatever sits behind/around the clip box
 * if it did not (the pre-Phase-4.4 behaviour this rule was written to
 * catch).
 *
 * Candidate probe points are the content box's four corners (inset 2px)
 * plus its center, not just "just past the ancestor's edge": a top-layer
 * element's landing spot varies by engine/positioning support (an
 * anchor-positioned popover lands right next to its trigger, which sits
 * inside the clip ancestor; the native `<div popover>` reference here has
 * no anchor-positioning applied at all, so the UA default
 * `position: fixed; inset: 0; margin: auto` centers it in the viewport --
 * nowhere near the clip ancestor). Whichever candidate point falls outside
 * the ancestor's box is used; if content and ancestor don't overlap at
 * all, every candidate qualifies.
 */
async function escapesClip(
  page: Page,
  contentSelector: string,
  ancestorSelector: string,
): Promise<{ escapes: boolean; reason?: string; x?: number; y?: number; hit?: string | null }> {
  return page.evaluate(
    ({ contentSelector, ancestorSelector }) => {
      const content = document.querySelector(contentSelector) as HTMLElement | null;
      const ancestor = document.querySelector(ancestorSelector) as HTMLElement | null;
      if (!content || !ancestor) {
        return { escapes: false, reason: `missing element(s): content=${!!content} ancestor=${!!ancestor}` };
      }
      const c = content.getBoundingClientRect();
      const a = ancestor.getBoundingClientRect();
      const inset = 2;
      const candidates: [number, number][] = [
        [c.left + inset, c.top + inset],
        [c.right - inset, c.top + inset],
        [c.left + inset, c.bottom - inset],
        [c.right - inset, c.bottom - inset],
        [c.left + c.width / 2, c.top + c.height / 2],
      ];
      const outsideAncestor = (x: number, y: number) =>
        x < a.left || x > a.right || y < a.top || y > a.bottom;
      const probe = candidates.find(([x, y]) => outsideAncestor(x, y));
      if (!probe) {
        return {
          escapes: false,
          reason: `every candidate point in content's box (${JSON.stringify(c)}) falls inside the ancestor's box (${JSON.stringify(a)}) -- cannot distinguish escape from containment this way`,
        };
      }
      const [x, y] = probe;
      const hit = document.elementFromPoint(x, y);
      const escapes = hit === content || content.contains(hit);
      return {
        escapes,
        x,
        y,
        hit: hit ? `${hit.tagName}#${hit.id || "(no id)"}` : null,
      };
    },
    { contentSelector, ancestorSelector },
  );
}

test.describe("Rule 1 — clipping escape (an ancestor with overflow:hidden + transform + a height shorter than the content must not clip it)", () => {
  test("CALIBRATION: native <div popover=auto> escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-native-trigger").click();
    await expect(page.locator("#clip-native-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-native-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("Tooltip content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-tooltip-trigger").hover();
    await expect(page.locator("#clip-tooltip-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-tooltip-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("HoverCard content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-hovercard-trigger").hover();
    await expect(page.locator("#clip-hovercard-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-hovercard-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("Popover (non-modal) content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-popover-trigger").click();
    await expect(page.locator("#clip-popover-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-popover-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });
});

test.describe("Rule 2 — light dismiss for popover=auto (click outside closes it, and Rust state syncs)", () => {
  test("CALIBRATION: native <div popover=auto> closes on outside click", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();
    await page.locator("#outside-click-target").click();
    await expect(page.locator("#stack-native-content")).toBeHidden();
  });

  test("Popover (non-modal): click outside closes it and syncs data-state", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "open");

    await page.locator("#outside-click-target").click();

    // The browser's own light dismiss hides the element; the Rust signal
    // must independently learn about it (primitives/src/top_layer.rs
    // `use_popover_sync`'s `toggle` listener) -- assert both, since a
    // stranded signal (the exact defect class recommended-implementations.md
    // Caveat 1 documents for <dialog>) would leave data-state="open" even
    // though the content is gone.
    await expect(page.locator("#stack-popover-content")).toBeHidden();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Rule 3 — Escape closes popover=auto, and Rust state syncs", () => {
  test("CALIBRATION: native <div popover=auto> closes on Escape", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator("#stack-native-content")).toBeHidden();
  });

  test("Popover (non-modal): Escape closes it and syncs data-state", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "open");

    await page.keyboard.press("Escape");

    await expect(page.locator("#stack-popover-content")).toBeHidden();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Rule 4 — top-layer stacking (a popover opened after a high-z-index sibling renders above it)", () => {
  test("CALIBRATION: native <div popover=auto> renders above the high-z-index sibling", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();

    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    const hit = await page.evaluate(() => {
      const sibling = document.getElementById("stack-sibling")!;
      const r = sibling.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return el ? `${el.tagName}#${el.id || "(no id)"}` : null;
    });
    expect(hit).toBe("DIV#stack-native-content");
  });

  test("Popover (non-modal) renders above the high-z-index sibling", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();

    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    const hit = await page.evaluate(() => {
      const sibling = document.getElementById("stack-sibling")!;
      const r = sibling.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return el ? `${el.tagName}#${el.id || "(no id)"}` : null;
    });
    expect(hit).toBe("DIALOG#stack-popover-content");
  });
});
