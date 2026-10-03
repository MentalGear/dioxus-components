/**
 * Auto-popover opening-gesture oracle (dev-docs/backlog.md row 40, 2026-10-03
 * re-audit).
 *
 * The class under test: a `popover="auto"` surface that becomes
 * `:popover-open` BETWEEN the `pointerdown` and the `pointerup` of the same
 * gesture. WHATWG HTML's light-dismiss algorithm
 * (https://html.spec.whatwg.org/multipage/popover.html#popover-light-dismiss,
 * "To light dismiss open popovers, given a PointerEvent event") is driven by
 * trusted `pointerdown`/`pointerup` events only -- never `click`:
 *
 *   - `pointerdown`: if there is no open auto popover -> return (the
 *     document's "popover pointerdown target" is NOT recorded).
 *   - `pointerup`: ancestor = topmost clicked popover of the target (null for
 *     a trigger that is not inside the popover and not declared via
 *     `popovertarget`); `sameTarget = (ancestor === pointerdown target)`;
 *     the pointerdown target is still its initial `null`, so `sameTarget` is
 *     TRUE -> every open auto popover is hidden.
 *
 * Consequences, per trigger family (primitives/src/):
 *   - open on `click`  (DropdownMenu, Select, non-modal Popover): the
 *     popover shows only after the gesture's `pointerup` has finished -> safe
 *     no matter how long the finger/mouse is held. iOS's lazy click
 *     retargeting (native-dialog.spec.ts Rule 8) is irrelevant here: the
 *     algorithm never reads `click`. This is why row 40's original worry
 *     ("an Auto popover could read the opening tap as outside") is not a
 *     real exposure for these.
 *   - open on `pointerup` (Menubar): shows after `pointerup` -> safe.
 *   - open on `pointerdown` (NavbarTrigger, navbar.rs): shows BETWEEN down
 *     and up whenever the hold outlasts Dioxus's render + effect (a few ms).
 *     `navbar.spec.ts` "mobile navigation" cannot see this: Playwright's
 *     `tap()` has zero hold, so `pointerup` is dispatched before
 *     `showPopover()` runs. A real finger holds ~50-150ms.
 *
 * The test holds a real touch for 250ms (CDP `Input.dispatchTouchEvent`,
 * since `page.touchscreen` only exposes an instantaneous `tap`) and asserts
 * that NO native `toggle` -> closed event fires on any `[popover]` during or
 * after the gesture. It records `toggle` events rather than the final
 * visible state on purpose: a compat `mouseenter` after touchend
 * (NavbarNav's `onmouseenter`) can silently re-open a just-light-dismissed
 * Navbar and mask the false dismiss.
 *
 * Expected state when this file landed: UNVERIFIED -- written in a lane that
 * could not build. If the Navbar case is RED, the construction fix is to
 * stop opening an `auto` popover from `pointerdown` (open on `click`, or
 * make `NavbarContent` `manual` + `use_outside_dismiss`); see
 * dev-docs/backlog.md row 40. The DropdownMenu case is a green control for
 * the click-open family.
 */
import { test, expect } from "../../fixtures";
import { type Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";
import { gotoHydrated } from "../../hydration";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const MOBILE_VIEWPORT = { width: 390, height: 844 };
const HOLD_MS = 250;
const SETTLE_MS = 600;

type ToggleLog = { id: string; newState: string }[];

const CASES: {
  name: string;
  url: string;
  trigger: (page: Page) => import("@playwright/test").Locator;
}[] = [
  {
    // pointerdown-open: the family the 2026-10-03 audit found exposed.
    name: "NavbarTrigger (opens on pointerdown)",
    url: `${BASE_URL}/component/?name=navbar&`,
    trigger: (page) => page.getByRole("menuitem", { name: "Inputs" }),
  },
  {
    // click-open control: must stay green regardless of hold length.
    name: "DropdownMenuTrigger (opens on click)",
    url: `${BASE_URL}/component/?name=dropdown_menu&`,
    trigger: (page) => page.getByRole("button", { name: "Open Menu" }),
  },
];

for (const kase of CASES) {
  test(`${kase.name}: a held touch (${HOLD_MS}ms) does not light-dismiss the popover it just opened`, async ({
    browser,
  }) => {
    const context = await browser.newContext({ hasTouch: true, viewport: MOBILE_VIEWPORT });
    const page = await context.newPage();
    try {
      await gotoHydrated(page, kase.url, { timeout: NAV_TIMEOUT });
      const trigger = kase.trigger(page).first();
      await trigger.scrollIntoViewIfNeeded();
      const box = await trigger.boundingBox();
      if (!box) throw new Error("trigger has no bounding box");
      const x = box.x + box.width / 2;
      const y = box.y + box.height / 2;

      // `toggle` does not bubble: capture on `document` sees every
      // `[popover]` element's transitions, including ones mounted later.
      await page.evaluate(() => {
        (window as unknown as { __toggles: ToggleLog }).__toggles = [];
        document.addEventListener(
          "toggle",
          (e) => {
            const t = e.target as HTMLElement;
            (window as unknown as { __toggles: ToggleLog }).__toggles.push({
              id: t.id,
              newState: (e as Event & { newState: string }).newState,
            });
          },
          true,
        );
      });

      const cdp = await context.newCDPSession(page);
      await cdp.send("Input.dispatchTouchEvent", {
        type: "touchStart",
        touchPoints: [{ x, y }],
      });
      await page.waitForTimeout(HOLD_MS);
      await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
      await page.waitForTimeout(SETTLE_MS);

      const toggles = await page.evaluate(
        () => (window as unknown as { __toggles: ToggleLog }).__toggles,
      );
      expect(
        toggles.some((t) => t.newState === "open"),
        `the gesture must have opened a popover at all (else this test is vacuous); toggles=${JSON.stringify(toggles)}`,
      ).toBe(true);
      expect(
        toggles.filter((t) => t.newState === "closed"),
        `no popover may be light-dismissed by the pointerup of the very gesture that opened it; toggles=${JSON.stringify(toggles)}`,
      ).toEqual([]);
    } finally {
      await context.close();
    }
  });
}
