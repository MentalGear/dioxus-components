/**
 * ORACLE: tier 2 (HTML) — native `<dialog>` modality for `Dialog`/`AlertDialog`.
 *
 * Source: docs/plan.md Phase 4.2, drawing on the WHATWG HTML Living
 * Standard's `<dialog>` chapter and the W3C ARIA Authoring Practices Guide
 * (APG) dialog-modal pattern for the keyboard rules:
 *   - The `<dialog>` element, `showModal()`, top-layer promotion, and the
 *     background-inert behaviour a modal dialog imposes:
 *     https://html.spec.whatwg.org/multipage/interactive-elements.html#the-dialog-element
 *   - `close`/`cancel` events, and the requirement that `close()` restore
 *     focus to "the previously focused element":
 *     https://html.spec.whatwg.org/multipage/interactive-elements.html#dom-dialog-close
 *   - APG dialog (modal) keyboard interaction — Escape closes the dialog and
 *     returns focus to the triggering element; Tab/Shift+Tab are trapped
 *     inside the dialog:
 *     https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/
 *
 * Fixture: preview/src/components/top_layer/component.rs (`TopLayerFixture`,
 * extended additively for this file — the "Native <dialog> ..." sections),
 * served at /component/?name=top_layer, plus the existing Dialog demo
 * (/component/?name=dialog, its nested-dialog fixture from Phase 3.2) for
 * rules 3-5, which need no new markup.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): rule 1 runs
 * against a native reference first (prefixed CALIBRATION:) — a plain
 * `<dialog>` opened by one `showModal()` call, with no library state or
 * logic involved beyond that (see the fixture's doc comment for why the
 * trigger cannot be fully declarative the way `popovertarget` is). A
 * CALIBRATION failure means the test is wrong, not the component.
 *
 * STATUS AT WRITE TIME (this session, against pre-Phase-4.2 `main` —
 * `Dialog`/`AlertDialog` still a CSS-positioned `div` + vendored
 * `FocusTrap`): rules 1-2 RED (a plain div is clipped by ordinary CSS and
 * has no platform-level inertness; the JS focus trap only contains Tab, it
 * does not touch pointer/programmatic-focus inertness); rules 3-4 GREEN
 * already (the vendored trap + `use_global_escape_listener` already sync
 * `open` on Escape and restore focus on close); rule 5 GREEN already (the
 * Phase 3.2 nested-dialog fixture's close order and focus return already
 * work under the JS trap). Full per-rule ledger in this session's report.
 *
 * Historical defect this whole file guards against
 * (docs/recommended-implementations.md Caveat 1,
 * docs/phase4-spike-findings.md experiment 1): upstream's first `<dialog>`
 * (`b3f6de53`) drove `showModal()`/`close()` one-way from the `open` signal
 * with no `close`/`cancel` sync, so a native Escape stranded the signal at
 * `true` and the dialog could never reopen. Rule 3's 5-cycle reopen loop is
 * this exact regression test, at the oracle layer rather than a spike.
 */

import { test, expect, type Page } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const gotoTopLayer = (page: Page) =>
  page.goto("http://127.0.0.1:8080/component/?name=top_layer&", {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

const gotoDialog = (page: Page) =>
  page.goto("http://127.0.0.1:8080/component/?name=dialog&", {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/** Reports where focus actually landed, so a failure names the culprit. */
async function focusReport(page: Page) {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (!el) return "null";
    return `<${el.tagName.toLowerCase()}${el.id ? ` id="${el.id}"` : ""}> text=${JSON.stringify(
      (el.textContent || "").trim().slice(0, 40),
    )}`;
  });
}

/**
 * Same technique as playwright/oracle/tier2-html/top-layer.spec.ts's
 * `escapesClip` — a normal DOM child's `getBoundingClientRect()` reports its
 * full laid-out size regardless of an `overflow: hidden` ancestor (clipping
 * only affects paint), so the real discriminator is what
 * `document.elementFromPoint()` actually paints at a point inside the
 * content's box but outside the clip ancestor's box.
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
          reason: `every candidate point in content's box (${JSON.stringify(c)}) falls inside the ancestor's box (${JSON.stringify(a)})`,
        };
      }
      const [x, y] = probe;
      const hit = document.elementFromPoint(x, y);
      const escapes = hit === content || content.contains(hit);
      return { escapes, x, y, hit: hit ? `${hit.tagName}#${hit.id || "(no id)"}` : null };
    },
    { contentSelector, ancestorSelector },
  );
}

test.describe("Rule 1 — clipping escape (a modal <dialog> inside overflow:hidden+transform must not be clipped)", () => {
  test("CALIBRATION: native <dialog> + showModal() escapes the clip", async ({ page }) => {
    await gotoTopLayer(page);
    await page.locator("#dialog-clip-native-trigger").click();
    await expect(page.locator("#dialog-clip-native-content")).toBeVisible();
    const result = await escapesClip(page, "#dialog-clip-native-content", "#dialog-clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("Dialog content escapes the clip", async ({ page }) => {
    await gotoTopLayer(page);
    await page.locator("#dialog-clip-trigger").click();
    await expect(page.locator("#dialog-clip-content")).toBeVisible();
    const result = await escapesClip(page, "#dialog-clip-content", "#dialog-clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });
});

test.describe("Rule 2 — background inertness (a real click must not reach a background handler; programmatic focus() on a background control must not move focus)", () => {
  test("background button click does not fire, and focus() on a background input does not move focus", async ({
    page,
  }) => {
    await gotoTopLayer(page);

    const bgButton = page.locator("#dialog-inert-bg-button");
    const bgCount = page.locator("#dialog-inert-bg-count");
    await expect(bgCount).toHaveText("0");

    await page.locator("#dialog-inert-trigger").click();
    await expect(page.locator("#dialog-inert-content")).toBeVisible();

    // A real mouse click at the background button's coordinates, bypassing
    // Playwright's actionability checks (which would themselves refuse to
    // click an occluded element) — this is the point: does the click reach
    // the button's own handler at all.
    const box = await bgButton.boundingBox();
    expect(box, "background button must still have a layout box").not.toBeNull();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    }
    await expect(bgCount, "a real click on a background control must not reach its handler while a modal dialog is open").toHaveText("0");

    // Programmatic focus() on a background input must not move
    // document.activeElement while the modal dialog is open.
    const movedFocus = await page.evaluate(() => {
      const input = document.getElementById("dialog-inert-bg-input") as HTMLElement | null;
      input?.focus();
      return document.activeElement === input;
    });
    expect(movedFocus, "focus() on a background control must not move focus out of an open modal dialog").toBe(false);

    await page.locator("#dialog-inert-close").click();
  });
});

test.describe("Rule 3 — Escape closes the dialog, the Rust `open` signal syncs, and reopening works repeatedly (5 cycles)", () => {
  test("5 open → Escape → reopen cycles never strand the signal", async ({ page }) => {
    await gotoDialog(page);
    const trigger = page.getByRole("button", { name: "Show Dialog" });
    const dialogEl = page.getByRole("dialog");

    for (let cycle = 0; cycle < 5; cycle++) {
      await trigger.click();
      await expect(dialogEl, `cycle ${cycle}: dialog must open`).toBeVisible();
      // The browser's own `.open` DOM property, read directly — independent
      // of Playwright's role-based visibility heuristics.
      const openBefore = await dialogEl.evaluate((el) => (el as HTMLDialogElement).open);
      expect(openBefore, `cycle ${cycle}: dialog.open must be true once shown`).toBe(true);

      await page.keyboard.press("Escape");
      await expect(dialogEl, `cycle ${cycle}: Escape must close the dialog`).toHaveCount(0);

      // The historical defect (docs/recommended-implementations.md Caveat
      // 1): if the `open` signal stranded at `true`, the *next* trigger
      // click would compute `!true` and the dialog would never reopen. A
      // clean re-open on every cycle is itself the sync proof.
    }
  });
});

test.describe("Rule 4 — focus restore to the trigger on close (control; must stay green through the native-dialog swap)", () => {
  test("Escape returns focus to the trigger that opened the dialog", async ({ page }) => {
    await gotoDialog(page);
    const trigger = page.getByRole("button", { name: "Show Dialog" });
    await trigger.click();
    await expect(page.getByRole("dialog")).toBeVisible();

    await page.keyboard.press("Escape");

    expect(await focusReport(page), "focus must return to the trigger, not <body>").not.toBe(
      "<body> text=\"\"",
    );
    await expect(trigger).toBeFocused();
  });
});

test.describe("Rule 5 — nested dialogs: close order and focus return", () => {
  test("closing the inner dialog returns focus inside the outer one; closing the outer one returns focus to the page trigger", async ({
    page,
  }) => {
    await gotoDialog(page);
    const pageTrigger = page.getByRole("button", { name: "Show Dialog" });
    await pageTrigger.click();

    const outer = page.getByRole("dialog").first();
    await expect(outer).toBeVisible();

    const openNested = outer.getByRole("button", { name: "Open Nested Dialog" });
    await openNested.click();
    await expect(page.getByRole("dialog")).toHaveCount(2);

    // Close the inner dialog via Escape -- per `use_global_escape_listener`'s
    // stack (lib.rs), only the top-most (inner) listener fires.
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(1);
    await expect(outer, "the outer dialog must still be open").toBeVisible();
    // Native `close()`'s own focus-restore (WHATWG HTML) returns focus to
    // whatever had it before the inner dialog's showModal() -- the button
    // that opened it, still inside the (still-open) outer dialog.
    await expect(openNested, "focus must return to the inner dialog's own trigger, inside the outer dialog").toBeFocused();

    // Close the outer dialog too -- focus must return all the way to the
    // original page-level trigger.
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(pageTrigger, "focus must return to the page trigger once every dialog is closed").toBeFocused();
  });
});
