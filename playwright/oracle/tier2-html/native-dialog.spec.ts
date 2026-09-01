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

/**
 * Rule 6 — modal `Popover` on the native-dialog engine (two-engine overlay
 * architecture completion, docs/plan.md). `PopoverModalContent`'s web arm
 * moves from the vendored `FocusTrap` (a plain `div`, no platform-level
 * inertness) onto the exact same `<dialog>` + `showModal()` +
 * open-driver/close-sync/backdrop-dismiss trio `Dialog`'s web modal arm
 * already uses (Rules 1-4 above) -- so this rule mirrors those almost
 * exactly, against `#name=top_layer`'s new "Native <dialog> modal Popover"
 * section instead of the Dialog demo.
 *
 * STATUS AT WRITE TIME (this session, against the pre-migration modal
 * `PopoverModalContent` -- a `div` + vendored `FocusTrap`, identical shape
 * to pre-Phase-4.2 `Dialog`): inertness (6b) RED, for the identical reason
 * Rule 2 was RED for `Dialog` -- the JS trap only contains Tab, it does not
 * touch pointer/programmatic-focus inertness, so a background click/focus()
 * reaches straight through. Clipping escape (6a), Escape+resync (6c), and
 * focus restore (6d) were already GREEN pre-migration (the vendored trap +
 * `use_global_escape_listener` already synced `open` on Escape and restored
 * focus, and the `div` was never clipped in this fixture's specific
 * up-to-60px case the way a *taller* clip would have caught -- see Rule 1's
 * own pre-4.2 ledger for the general form of this same clipping defect).
 * Trigger-anchored placement (6e) is the one rule that is GREEN *both*
 * before and after this migration, and must *stay* green through the swap
 * -- that is this rule's regression guard, not a new capability: pre-
 * migration, the modal arm was a plain DOM child positioned by ordinary
 * `position: absolute` inside `PopoverRoot`'s `position: relative` wrapper
 * (no top-layer promotion, no anchor-positioning needed at all); this
 * migration's `<dialog>` is promoted straight into the top layer by
 * `showModal()` regardless, which is exactly the containing-block problem
 * `docs/plan.md` Phase 4.4 solved for `popover="auto"` content (see
 * `top_layer.rs`'s module doc) -- so the web modal arm now carries the same
 * `dx-anchor-popover`/`position-anchor` wiring the non-modal arm already
 * uses, and `top_layer.rs`'s shared, engine-injected anchor-positioning
 * stylesheet now matches `:modal` alongside `[popover]` in every selector
 * (including the `margin: 0; inset: auto;` UA reset for `dialog:modal`'s own
 * `position: fixed; inset: 0; margin: auto;` centering trap) so the same
 * `anchor()` rules apply to a `showModal()` dialog too.
 */

test.describe("Rule 6 — modal Popover (two-engine completion)", () => {
  test("6a. clipping escape: modal Popover content escapes an overflow:hidden ancestor", async ({ page }) => {
    await gotoTopLayer(page);
    await page.locator("#popover-modal-clip-trigger").click();
    await expect(page.locator("#popover-modal-clip-content")).toBeVisible();
    const result = await escapesClip(page, "#popover-modal-clip-content", "#popover-modal-clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("6b. background inertness: focus() on a background control must not move focus; a real click must not reach a background handler", async ({
    page,
  }) => {
    await gotoTopLayer(page);

    const bgButton = page.locator("#popover-modal-inert-bg-button");
    const bgCount = page.locator("#popover-modal-inert-bg-count");
    await expect(bgCount).toHaveText("0");

    await page.locator("#popover-modal-inert-trigger").click();
    await expect(page.locator("#popover-modal-inert-content")).toBeVisible();

    // Programmatic focus() first, while the Popover is unambiguously still
    // open -- unlike Rule 2's `AlertDialog`-based fixture (chosen there
    // specifically because `Dialog`'s own backdrop-click dismiss would
    // otherwise confound the probe), every modal `Popover` *does* carry
    // `use_dialog_backdrop_dismiss` (this migration wires the same trio
    // `Dialog`'s web modal arm uses), so a real click at these background
    // coordinates is a legitimate "outside" click to the `<dialog>`'s own
    // full-viewport `::backdrop` and correctly dismisses the Popover as a
    // *separate*, by-design behavior -- not a failure of inertness, but a
    // reason this test cannot chain the click before the focus() check
    // (a `.focus()` call is not a `click` event, so it never triggers that
    // dismiss and is the unconfounded probe here).
    const movedFocus = await page.evaluate(() => {
      const input = document.getElementById("popover-modal-inert-bg-input") as HTMLElement | null;
      input?.focus();
      return document.activeElement === input;
    });
    expect(movedFocus, "focus() on a background control must not move focus out of an open modal Popover").toBe(false);

    // The real click below both proves the background button's own handler
    // is unreachable *and* (correctly, separately) dismisses the Popover via
    // its own backdrop-dismiss -- see the comment above. Only `bgCount`
    // staying "0" is asserted; this click is not expected to leave the
    // Popover open afterward.
    const box = await bgButton.boundingBox();
    expect(box, "background button must still have a layout box").not.toBeNull();
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    }
    await expect(bgCount, "a real click on a background control must not reach its handler while a modal Popover is open").toHaveText("0");
  });

  test("6c. Escape closes the Popover, the Rust `open` signal syncs, and reopening works repeatedly (5 cycles)", async ({ page }) => {
    await gotoTopLayer(page);
    const trigger = page.locator("#popover-modal-anchor-trigger");
    const contentEl = page.locator("#popover-modal-anchor-content");

    for (let cycle = 0; cycle < 5; cycle++) {
      await trigger.click();
      await expect(contentEl, `cycle ${cycle}: Popover must open`).toBeVisible();
      const openBefore = await contentEl.evaluate((el) => (el as HTMLDialogElement).open);
      expect(openBefore, `cycle ${cycle}: dialog.open must be true once shown`).toBe(true);

      await page.keyboard.press("Escape");
      await expect(contentEl, `cycle ${cycle}: Escape must close the Popover`).toBeHidden();

      // Same historical-defect guard as Rule 3: a stranded `open` signal
      // would make the *next* trigger click compute `!true` and the
      // Popover would never reopen (`PopoverTrigger`'s own
      // `set_open.call(!(ctx.open)())` toggle). A clean re-open every
      // cycle is itself the sync proof.
    }
  });

  test("6d. focus restore to the trigger on close", async ({ page }) => {
    await gotoTopLayer(page);
    const trigger = page.locator("#popover-modal-anchor-trigger");
    await trigger.click();
    await expect(page.locator("#popover-modal-anchor-content")).toBeVisible();

    await page.keyboard.press("Escape");

    expect(await focusReport(page), "focus must return to the trigger, not <body>").not.toBe(
      "<body> text=\"\"",
    );
    await expect(trigger).toBeFocused();
  });

  test("6e. trigger-anchored placement: a modal Popover renders next to its trigger, not viewport-centered (regression guard -- must stay green through the native-dialog swap)", async ({
    page,
  }) => {
    await gotoTopLayer(page);
    // Pinned near the top-left viewport corner (see the fixture's own
    // wrapping `style`) -- far enough from the viewport center that a
    // viewport-centered `showModal()` (the UA default this migration must
    // override) would measurably disagree with a trigger-anchored one.
    const trigger = page.locator("#popover-modal-anchor-trigger");
    const content = page.locator("#popover-modal-anchor-content");
    await trigger.click();
    await expect(content).toBeVisible();

    const measurement = await page.evaluate(() => {
      const t = document.getElementById("popover-modal-anchor-trigger")!.getBoundingClientRect();
      const c = document.getElementById("popover-modal-anchor-content")!.getBoundingClientRect();
      return {
        // The default `side="bottom"`/`align="center"` contract: content
        // top should sit a small, fixed gap below the trigger's bottom
        // edge, horizontally centered on it -- see
        // `preview/src/components/popover/style.css`'s
        // `[data-side="bottom"]` rule (8px `margin-top`, `transform:
        // translateX(-50%)`) and `top_layer.rs`'s `:modal`-matched
        // `anchor()` counterpart for the CSS-anchor-positioning path.
        offsetTop: c.top - t.bottom,
        offsetCenterX: c.left + c.width / 2 - (t.left + t.width / 2),
        contentTop: c.top,
        contentLeft: c.left,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
      };
    });

    const debug = JSON.stringify(measurement);
    // Trigger-anchored: content sits a small gap below the trigger,
    // centered on it -- generous tolerance (rounding, border/padding)
    // around the ~8-16px gap this component's CSS declares.
    expect(Math.abs(measurement.offsetTop), debug).toBeLessThan(40);
    expect(Math.abs(measurement.offsetCenterX), debug).toBeLessThan(20);

    // Not viewport-centered: the UA default this migration must override
    // (`dialog:modal { position: fixed; inset: 0; margin: auto; }`) would
    // place the content's center within a few pixels of the *viewport's*
    // center regardless of the trigger -- which, given the trigger is
    // pinned at (40, 40), is nowhere near where a trigger-anchored render
    // lands. Asserting the content's top-left corner is far from the
    // viewport center directly rules out that centering trap having won.
    const viewportCenterY = measurement.viewportHeight / 2;
    expect(
      Math.abs(measurement.contentTop - viewportCenterY),
      `${debug} -- content must not be viewport-centered`,
    ).toBeGreaterThan(150);
  });
});
