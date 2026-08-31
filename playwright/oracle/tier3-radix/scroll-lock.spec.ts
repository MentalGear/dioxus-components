/**
 * ORACLE (tier 3 -- Radix-parity, labelled opinion; see
 * docs/conformance-harness.md "Tier 3 -- Radix-parity" and
 * playwright/oracle/tier3-radix/README.md's rule-source policy).
 *
 * Rule: while a modal surface is open, the page behind it must not scroll.
 * No W3C standard specifies this -- it is a UX convention several mature
 * libraries implement. Radix's implementation is `react-remove-scroll`
 * (https://github.com/theKashey/react-remove-scroll), which every
 * Radix Dialog/AlertDialog/DropdownMenu/etc. wraps its content in via
 * `RemoveScroll` when the surface is modal. `@radix-ui/react-dialog`'s
 * `DialogContent` passes `RemoveScroll.Root` unconditionally (gated on
 * `modal`); `@radix-ui/react-dropdown-menu` and `@radix-ui/react-context-menu`
 * take an explicit `modal` prop (default `true`) that gates the same
 * behavior on their menu content -- the shape this file's `modal` prop on
 * `DropdownMenu`/`ContextMenu` mirrors.
 *
 * Source for our own implementation:
 * `dignifiedquire/dx-components` (MIT OR Apache-2.0)
 * `primitives/src/scroll_lock.rs` @ 5af3cc292559a0e8d73c7b9a827c4ca08ef34d99,
 * adapted per docs/recommended-implementations.md §5 and implemented at
 * `primitives/src/scroll_lock.rs` in this repo -- see that file's header for
 * the full adaptation note (a Rust-side nesting-aware counter in place of
 * the dq base's JS global, reproducing sarendipitee/dx-components'
 * `overlay.rs` unlock-flash guard).
 *
 * STATUS AT WRITE TIME: RED. No scroll lock exists anywhere in this tree
 * before docs/plan.md Phase 3.2 lands -- every test below is expected to
 * fail against current `main`, and the nested-dialog case has no fixture at
 * all until Phase 3.2 adds one (see the dialog demo's "Open Nested Dialog"
 * button, `preview/src/components/dialog/variants/main/mod.rs`).
 *
 * FOLLOW-UP NOT COVERED HERE: `Select`'s open listbox is not wired to any
 * scroll lock in this pass (docs/plan.md Phase 3.2 explicitly scopes it
 * out -- neither `dignifiedquire` nor `sarendipitee` covers it either).
 * `Menubar` is correctly excluded throughout -- it is never modal.
 *
 * KNOWN GAP, both in the dq base and here (see
 * docs/recommended-implementations.md §5): no iOS momentum-scroll handling.
 * Radix delegates that to `react-remove-scroll`, which this crate has no
 * equivalent of, and this file does not test for it -- only that
 * `window.scrollY` doesn't move while locked, on a desktop Chromium
 * mouse-wheel attempt. It also does not (and, per the note on
 * `assertScrollIsLocked` below, should not) assert that a scripted
 * `window.scrollTo()` call is blocked -- `overflow: hidden` never clamps
 * that, in this implementation or Radix's.
 *
 * LAYOUT-SHIFT ASSERTIONS (below, `assertNoHorizontalShift`): scroll-gap
 * compensation is exercised too, per
 * `docs/phase4-spike-findings.md` "Round 2 -- solved by construction",
 * Construction A/C. Removing a classic (non-overlay) scrollbar's `overflow:
 * hidden` without reserving its space shifts everything horizontally by the
 * scrollbar's width the instant the lock engages -- confirmed 15px on this
 * image under a forced real scrollbar (see `playwright/xvfb.local.config.ts`).
 * That regression is invisible under this repo's default headless Chromium,
 * which renders 0-width overlay scrollbars even for a genuinely scrollable
 * page -- so `assertNoHorizontalShift` is trivially satisfied there and only
 * bites for real when run under `xvfb.local.config.ts`. Both runs matter:
 * headless proves the fix doesn't *introduce* a shift on overlay-scrollbar
 * platforms (a transient `scrollbar-gutter` toggle was tried and falsified
 * for exactly this reason -- see the findings doc), and the Xvfb run proves
 * it *removes* the shift on classic-scrollbar platforms.
 */

import { test, expect, type Page } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const goto = (page: Page, name: string) =>
  page.goto(`http://127.0.0.1:8080/component/?name=${name}&`, {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/** Sanity check: the route must actually have more content than fits in the
 * viewport, or "scrollY didn't move" would be true regardless of any lock. */
async function assertPageIsScrollable(page: Page) {
  const { scrollHeight, innerHeight } = await page.evaluate(() => ({
    scrollHeight: document.documentElement.scrollHeight,
    innerHeight: window.innerHeight,
  }));
  expect(
    scrollHeight,
    "fixture route must be taller than the viewport for this test to mean anything",
  ).toBeGreaterThan(innerHeight);
}

/**
 * Attempts to scroll the page the way a person actually would -- a mouse
 * wheel over the page -- and asserts `window.scrollY` did not move.
 *
 * Deliberately does NOT also try `window.scrollTo()`: `overflow: hidden`
 * (on `<html>`/`<body>`, which is what this lock -- and Radix's
 * `react-remove-scroll` -- set) blocks wheel-, touch-, and
 * keyboard-initiated scrolling, but does not clamp a direct, scripted
 * `scrollTo()`/`scrollTop =` call, which every browser still honors
 * regardless of overflow. Verified empirically against this page: with
 * `overflow: hidden` applied, `page.mouse.wheel()` leaves `scrollY` at 0,
 * while `window.scrollTo()` still moves it. Radix's own lock has the same
 * property (`react-remove-scroll` intercepts wheel/touch/key events; it does
 * not override `Element.scrollTo`), so asserting against a scripted
 * `scrollTo()` here would fail a correct implementation, not just ours.
 */
async function assertScrollIsLocked(page: Page) {
  const before = await page.evaluate(() => window.scrollY);
  await page.mouse.move(400, 300);
  await page.mouse.wheel(0, 800);
  const after = await page.evaluate(() => window.scrollY);
  expect(
    after,
    "page must not scroll via mouse wheel while a modal surface is locking it",
  ).toBe(before);
}

/**
 * Asserts the page can scroll again via mouse wheel (the inverse of the
 * above).
 *
 * Retries the wheel attempt rather than making just one: the unlock
 * restores `overflow` via a `document::eval` round-trip queued from the
 * closing component's unmount, which lands well after Playwright observes
 * the closed element gone from the accessibility tree (empirically, over a
 * second later is not unusual) -- not an implementation defect, just two
 * different signals settling at very different times. Each attempt resets
 * scroll position with a direct `scrollTo()` first; per the note on
 * `assertScrollIsLocked`, that call proves nothing about the lock either
 * way, it's just attempt setup.
 */
async function assertScrollIsUnlocked(page: Page) {
  const deadline = Date.now() + 8000;
  let scrollY = 0;
  do {
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.mouse.move(400, 300);
    await page.mouse.wheel(0, 800);
    await page.waitForTimeout(200);
    scrollY = await page.evaluate(() => window.scrollY);
  } while (scrollY === 0 && Date.now() < deadline);
  expect(
    scrollY,
    "page must scroll via mouse wheel again once nothing is locking it",
  ).toBeGreaterThan(0);
}

/**
 * Returns the current right edge of a `position: fixed; right: 0` marker,
 * injecting it once per page if it isn't already there.
 *
 * `position: fixed` elements are positioned against the initial containing
 * block, whose size tracks the true viewport net of whatever scrollbar the
 * platform is actually rendering right now -- unlike normal-flow content,
 * which a `padding-right` compensation recipe can reach but this can't (that
 * recipe was tried and falsified for exactly this element shape; see
 * `docs/phase4-spike-findings.md`, Round 2 Construction A). A stand-in for a
 * realistic right-aligned navbar action.
 */
async function rightEdgeMarkerRight(page: Page): Promise<number> {
  return page.evaluate(() => {
    let marker = document.getElementById("scroll-lock-shift-probe");
    if (!marker) {
      marker = document.createElement("div");
      marker.id = "scroll-lock-shift-probe";
      marker.style.cssText =
        "position:fixed; top:0; right:0; width:4px; height:4px; pointer-events:none; z-index:2147483647;";
      document.body.appendChild(marker);
    }
    return marker.getBoundingClientRect().right;
  });
}

/**
 * Asserts the right-anchored marker (see `rightEdgeMarkerRight`) has not
 * moved from `expectedRight` -- i.e. engaging or releasing the scroll lock
 * must not shift page layout horizontally. Only bites for real on a platform
 * with a non-zero scrollbar gap; see the module header's LAYOUT-SHIFT note.
 */
async function assertNoHorizontalShift(page: Page, expectedRight: number, when: string) {
  expect(await rightEdgeMarkerRight(page), `layout must not shift horizontally ${when}`).toBe(
    expectedRight,
  );
}

test.describe("Dialog locks and releases page scroll", () => {
  test("scroll is locked while open and restored after Escape", async ({ page }) => {
    await goto(page, "dialog");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    await page.getByRole("button", { name: "Show Dialog" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await assertNoHorizontalShift(page, rightBefore, "while the dialog is open");

    await assertScrollIsLocked(page);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after the dialog closes and scroll unlocks");
  });

  test("nested dialog: closing the inner one leaves the page locked; closing the outer one restores scroll", async ({
    page,
  }) => {
    await goto(page, "dialog");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    // Open the outer dialog.
    await page.getByRole("button", { name: "Show Dialog" }).click();
    const outer = page.getByRole("dialog").first();
    await expect(outer).toBeVisible();
    await assertScrollIsLocked(page);
    await assertNoHorizontalShift(page, rightBefore, "with the outer dialog open");

    // Open the nested dialog from inside the outer one's content.
    await page.getByRole("button", { name: "Open Nested Dialog" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(2);
    await assertScrollIsLocked(page);
    await assertNoHorizontalShift(page, rightBefore, "with both dialogs open");

    // Close the inner dialog only -- the outer one is still open, so the
    // page must still be locked. This is the refcount case: a naive
    // "unlock whenever any modal closes" implementation fails here.
    await page.getByRole("button", { name: "Close Nested" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(1);
    await assertScrollIsLocked(page);
    await assertNoHorizontalShift(page, rightBefore, "with only the outer dialog open again");

    // Close the outer dialog -- nothing is locking anymore. Closed via its
    // own Close button rather than Escape: this test's subject is the
    // scroll-lock refcount, not the (separately covered, and here
    // momentarily still settling right after the inner dialog's unmount)
    // Escape-priority stack in `use_global_escape_listener` (lib.rs).
    await page.getByRole("dialog").getByRole("button", { name: "Close" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after both dialogs close and scroll unlocks");
  });
});

test.describe("AlertDialog locks and releases page scroll", () => {
  test("scroll is locked while open and restored after Escape", async ({ page }) => {
    await goto(page, "alert_dialog");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    await page.getByRole("button", { name: "Show Alert Dialog" }).click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
    await assertNoHorizontalShift(page, rightBefore, "while the alert dialog is open");

    await assertScrollIsLocked(page);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("alertdialog")).toHaveCount(0);

    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after the alert dialog closes and scroll unlocks");
  });
});

test.describe("Popover locks and releases page scroll", () => {
  test("scroll is locked while open (modal by default) and restored after Escape", async ({
    page,
  }) => {
    await goto(page, "popover");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    await page.getByRole("button", { name: "Show Popover" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await assertNoHorizontalShift(page, rightBefore, "while the popover is open");

    await assertScrollIsLocked(page);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after the popover closes and scroll unlocks");
  });
});

test.describe("DropdownMenu locks and releases page scroll", () => {
  test("scroll is locked while open (modal by default) and restored after Escape", async ({
    page,
  }) => {
    await goto(page, "dropdown_menu");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.click();
    await expect(trigger).toHaveAttribute("data-state", "open");
    await assertNoHorizontalShift(page, rightBefore, "while the dropdown menu is open");

    await assertScrollIsLocked(page);

    await page.keyboard.press("Escape");
    await expect(trigger).toHaveAttribute("data-state", "closed");

    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after the dropdown menu closes and scroll unlocks");
  });
});

test.describe("ContextMenu locks and releases page scroll", () => {
  test("scroll is locked while open (modal by default) and restored after Escape", async ({
    page,
  }) => {
    await goto(page, "context_menu");
    await assertPageIsScrollable(page);
    const rightBefore = await rightEdgeMarkerRight(page);

    await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
    await expect(page.getByRole("menu")).toHaveAttribute("data-state", "open");
    await assertNoHorizontalShift(page, rightBefore, "while the context menu is open");

    await assertScrollIsLocked(page);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("menu")).toHaveCount(0);

    await assertScrollIsUnlocked(page);
    await assertNoHorizontalShift(page, rightBefore, "after the context menu closes and scroll unlocks");
  });
});
