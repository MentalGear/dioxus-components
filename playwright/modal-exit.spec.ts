/**
 * The exit of every native-`<dialog>` modal, run over the consumers DERIVED from the source
 * (`modal-consumers.ts`): Dialog, AlertDialog, Sheet (all four sides), Drawer (bottom and
 * top) and CommandDialog -- every component that opens a dialog through the shared
 * `use_dialog_open_driver`. The claims live in `assert-modal-exit.ts` (a closing modal stays
 * a modal in the top layer at one size and place until its exit has played; owner report
 * 2026-10-05) and `assert-backdrop-fade.ts` (one scrim, faded in and out for the same
 * length); this file only applies them to the whole list, so adding a modal component
 * without registering it here fails the first test below instead of silently skipping both.
 *
 * Captured on the HOME PAGE, inside the component's `.dx-component-card`: the regression is
 * only visible there (a card is `content-visibility: auto`, so layout and paint contained,
 * and a dialog that leaves the top layer is sized and clipped by it). Dev-server note: this
 * is a debug build, so frame times are coarse; every assertion is either a per-frame invariant
 * or measured in browser events, never a frame count (backlog row 115).
 */
import { test, expect } from "./fixtures";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";
import { assertSharedBackdrop, captureBackdrop, resolveOverlayMs } from "./assert-backdrop-fade";
import { assertModalExit, captureModalExit, homeCard } from "./assert-modal-exit";
import {
  CONSUMERS,
  EXEMPT,
  deriveThemedConsumers,
  derivePrimitiveOpeners,
  type Closer,
} from "./modal-consumers";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

test("the modal consumers this spec covers are exactly the ones the source composes", () => {
  const derived = deriveThemedConsumers();
  const covered = new Set([...CONSUMERS.map((c) => c.component), ...Object.keys(EXEMPT)]);
  const missing = derived.filter((n) => !covered.has(n));
  const stale = [...covered].filter((n) => !derived.includes(n));
  expect(
    missing,
    `component(s) compose DialogRoot/AlertDialogRoot/Drawer/CommandDialog but have no entry in playwright/modal-consumers.ts ` +
      `(add one so the exit and scrim assertions run on it, or an EXEMPT reason): ${missing.join(", ")}`,
  ).toEqual([]);
  expect(stale, `playwright/modal-consumers.ts lists component(s) that no longer compose a modal: ${stale.join(", ")}`).toEqual([]);

  // The primitive layer: the only native-dialog openers are the shared driver's callers (the modal
  // Popover's web arm used to carry a deferred-close driver of its own; it calls the shared one now,
  // so the deferred close has one implementation), and the only file that calls showModal() itself is
  // the driver's (lib.rs).
  const { viaDriver, ownShowModal } = derivePrimitiveOpeners();
  expect(viaDriver, "primitives calling use_dialog_open_driver").toEqual(["alert_dialog.rs", "dialog.rs", "popover.rs"]);
  expect(ownShowModal, "primitives calling .showModal() themselves").toEqual(["lib.rs"]);
});

const closerLabel = (c: Closer) => (typeof c === "string" ? c : `button "${c.button}"`);

for (const consumer of CONSUMERS) {
  for (const variant of consumer.variants) {
    for (const closer of variant.closers) {
      test(`${consumer.component} (${variant.name}) closing via ${closerLabel(closer)} stays a modal, at one size, until its exit has played`, async ({
        page,
      }) => {
        await gotoHydrated(page, `${BASE_URL}/`, GOTO_OPTS);
        const card = await homeCard(page, consumer.component);
        const trigger = card.getByRole("button", { name: variant.trigger, exact: true });
        const dialog = page.locator(consumer.dialogSelector);
        const close = async () => {
          if (closer === "escape") await page.keyboard.press("Escape");
          else if (closer === "backdrop") await page.mouse.click(2, 2);
          else await dialog.getByRole("button", { name: closer.button, exact: true }).first().click();
        };
        const capture = await captureModalExit(page, consumer.dialogSelector, () => trigger.click(), close);
        assertModalExit(capture, `${consumer.component}/${variant.name}/${closerLabel(closer)}`);
      });
    }
  }

  test(`${consumer.component} paints the shared scrim and fades it in and out`, async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=${consumer.component}&`, GOTO_OPTS);
    const trigger = page.getByRole("button", { name: consumer.variants[0].trigger, exact: true });
    await expect(trigger).toBeVisible();
    const capture = await captureBackdrop(
      page,
      () => trigger.click(),
      () => page.keyboard.press("Escape"),
    );
    assertSharedBackdrop(capture, await resolveOverlayMs(page), consumer.component);
  });
}

// The `overlay` prop (Dialog/AlertDialog/Sheet/Drawer/CommandDialog, default true): `overlay: false` puts
// `data-dx-overlay="off"` on the `<dialog>` (and on the root wrapper), the theme's one rule
// (`dx-components-theme.css`, "Modal overlay switch"; `overlay-switch.spec.ts` covers the rule itself) removes
// the `::backdrop` scrim, and everything else about the modal stays. Each component's `overlay` variant demo
// renders it with the prop off.
for (const consumer of CONSUMERS) {
  test(`${consumer.component}: overlay: false marks the dialog, paints no scrim, and stays a modal`, async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=${consumer.component}&`, GOTO_OPTS);
    await page.getByRole("button", { name: consumer.overlayOffTrigger, exact: true }).click();
    const dialog = page.locator(`${consumer.dialogSelector}[open]`);
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute("data-dx-overlay", "off");
    // The root wrapper carries it too: it is the scrim painter wherever there is no top layer.
    await expect(dialog.locator("xpath=..")).toHaveAttribute("data-dx-overlay", "off");
    await expect
      .poll(() =>
        dialog.evaluate((el) => {
          const cs = getComputedStyle(el, "::backdrop");
          const m = /rgba?\(([^)]+)\)/.exec(cs.backgroundColor);
          const parts = m ? m[1].split(/[ ,/]+/).filter(Boolean) : [];
          return { alpha: !m ? 0 : parts.length > 3 ? Number(parts[3]) : 1, filter: cs.backdropFilter };
        }),
      )
      .toEqual({ alpha: 0, filter: "none" });
    expect(await dialog.evaluate((el) => (el as HTMLDialogElement).matches(":modal"))).toBe(true);
    // Escape still dismisses it through the same deferred close.
    await page.keyboard.press("Escape");
    await expect(page.locator(consumer.dialogSelector)).toHaveCount(0);
  });

  test(`${consumer.component}: the default (overlay: true) carries no data-dx-overlay and paints the shared scrim`, async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=${consumer.component}&`, GOTO_OPTS);
    await page.getByRole("button", { name: consumer.variants[0].trigger, exact: true }).click();
    const dialog = page.locator(`${consumer.dialogSelector}[open]`);
    await expect(dialog).toBeVisible();
    await expect(dialog).not.toHaveAttribute("data-dx-overlay", /.*/);
    await expect
      .poll(() => dialog.evaluate((el) => getComputedStyle(el, "::backdrop").backgroundColor))
      .not.toMatch(/^(rgba\(0, 0, 0, 0\)|transparent)$/);
  });
}

test("Dialog's non-web/non-modal wrapper scrim follows overlay too (a plain div has no ::backdrop for the theme rule to reach)", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/component/?name=dialog&`, GOTO_OPTS);
  // A non-modal dialog is a plain `div` in the page: no top layer, no `::backdrop`. Its root wrapper is the scrim.
  const wrapperScrim = () =>
    page.locator(".dx-dialog-backdrop:not(:has(> dialog))").evaluate((el) => {
      const cs = getComputedStyle(el);
      const m = /rgba?\(([^)]+)\)/.exec(cs.backgroundColor);
      const parts = m ? m[1].split(/[ ,/]+/).filter(Boolean) : [];
      return { alpha: !m ? 0 : parts.length > 3 ? Number(parts[3]) : 1, filter: cs.backdropFilter };
    });

  await page.getByRole("button", { name: "Non-modal dialog", exact: true }).click();
  await expect(page.locator(".dx-dialog-backdrop:not(:has(> dialog))")).toBeVisible();
  await expect.poll(wrapperScrim, { message: "overlay: true -- the wrapper paints the shared scrim" }).toMatchObject({ alpha: 0.1 });
  await page.keyboard.press("Escape");
  await expect(page.locator(".dx-dialog-backdrop")).toHaveCount(0);

  await page.getByRole("button", { name: "Non-modal dialog without overlay", exact: true }).click();
  await expect(page.locator(".dx-dialog-backdrop:not(:has(> dialog))")).toHaveAttribute("data-dx-overlay", "off");
  await expect.poll(wrapperScrim, { message: "overlay: false -- the wrapper paints nothing" }).toEqual({ alpha: 0, filter: "none" });
});
