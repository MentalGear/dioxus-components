import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import { BASE_URL } from "./base-url";

/**
 * The shared modal-overlay switch (`preview/assets/dx-components-theme.css`, "Modal overlay switch"):
 * `data-dx-overlay="off"` on a native `<dialog>` removes its `::backdrop` scrim, and ONE theme rule
 * does it for every component that carries the attribute (Dialog, AlertDialog, Sheet, Drawer,
 * CommandDialog, the modal Popover).
 *
 * This is the theme-level half of the contract and deliberately does not depend on any component's own
 * `overlay` prop: it opens the real component, sets the attribute by hand and reads the browser's own
 * resolved `::backdrop`. That is what proves the rule beats each component's own `::backdrop` rule
 * (`.dx-dialog[open]::backdrop` and friends are (0,2,1) and up; the theme rule is `:where()` (0,0,1),
 * so it holds only because it is `!important`). Each component's prop -> attribute wiring is covered by
 * that component's own spec (`popover.spec.ts` for Popover).
 *
 * The scrim is read from the resolved style, never from a screenshot: `getComputedStyle(dialog,
 * "::backdrop")` is what the compositor paints.
 */

interface Modal {
  name: string;
  url: string;
  open: (page: Page) => Promise<unknown>;
}

const MODALS: Modal[] = [
  { name: "Dialog", url: "dialog", open: (p) => p.getByRole("button", { name: "Show Dialog", exact: true }).click() },
  {
    name: "AlertDialog",
    url: "alert_dialog",
    open: (p) => p.getByRole("button", { name: "Show Alert Dialog", exact: true }).click(),
  },
  { name: "Sheet", url: "sheet", open: (p) => p.getByRole("button", { name: "Right", exact: true }).click() },
  { name: "Drawer", url: "drawer", open: (p) => p.getByRole("button", { name: "Move Goal", exact: true }).click() },
  {
    name: "CommandDialog",
    url: "command",
    open: (p) => p.getByRole("button", { name: "Open Command Palette" }).first().click(),
  },
  // The modal Popover opens undimmed by default; its switch is exercised below on the `overlay` variant.
  {
    name: "Popover (overlay: true)",
    url: "popover&variant=overlay",
    open: (p) => p.getByRole("button", { name: "Open with overlay", exact: true }).click(),
  },
];

/** `{ alpha, blur }` of a dialog's `::backdrop` as the browser resolves it right now. */
const scrim = (dialog: Locator) =>
  dialog.evaluate((el) => {
    const cs = getComputedStyle(el, "::backdrop");
    const c = /rgba?\(([^)]+)\)/.exec(cs.backgroundColor);
    const parts = c ? c[1].split(/[ ,/]+/).filter(Boolean) : [];
    const alpha = !c ? 0 : parts.length > 3 ? Number(parts[3]) : 1;
    const blur = /blur\(([\d.]+)px\)/.exec(cs.backdropFilter);
    return { alpha: +alpha.toFixed(3), blur: blur ? Number(blur[1]) : 0, filter: cs.backdropFilter };
  });

for (const modal of MODALS) {
  test(`${modal.name}: data-dx-overlay="off" removes the scrim and removing it brings it back`, async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=${modal.url}&`);
    await modal.open(page);
    const dialog = page.locator("dialog[open]");
    await expect(dialog).toBeVisible();

    // The precondition that makes this test able to fail: the shared scrim (black/10, blur 4px) is
    // painted before the switch is touched. The fade is 200ms, so poll for it.
    await expect.poll(() => scrim(dialog), { message: "the shared scrim must be painted first" }).toMatchObject({
      alpha: 0.1,
      blur: 4,
    });

    await dialog.evaluate((el) => el.setAttribute("data-dx-overlay", "off"));
    await expect
      .poll(() => scrim(dialog), { message: "the switch must remove the component's own scrim" })
      .toEqual({ alpha: 0, blur: 0, filter: "none" });

    // Still a real modal: the switch removes paint, nothing else.
    expect(await dialog.evaluate((el) => (el as HTMLDialogElement).matches(":modal"))).toBe(true);

    await dialog.evaluate((el) => el.removeAttribute("data-dx-overlay"));
    await expect.poll(() => scrim(dialog), { message: "dropping the switch must restore the scrim" }).toMatchObject({
      alpha: 0.1,
      blur: 4,
    });
  });
}

test.describe("the rule itself", () => {
  test.beforeEach(async ({ page }) => {
    // Any page that loads the theme; the dialog page is the one whose stylesheet is under test below.
    await page.goto(`${BASE_URL}/component/?name=dialog&`);
    await expect(page.getByRole("button", { name: "Show Dialog", exact: true })).toBeVisible();
  });

  test("a bare <dialog> carrying the attribute loses the UA's default scrim too", async ({ page }) => {
    // No component class, no stylesheet of ours on it: only the browser's own `dialog::backdrop`
    // (`rgb(0 0 0 / 10%)`) is in play, and the attribute alone must beat it.
    const result = await page.evaluate(() => {
      const read = (el: HTMLElement) => {
        const cs = getComputedStyle(el, "::backdrop");
        return { bg: cs.backgroundColor, filter: cs.backdropFilter };
      };
      const make = (off: boolean) => {
        const d = document.createElement("dialog");
        if (off) d.setAttribute("data-dx-overlay", "off");
        document.body.append(d);
        d.showModal();
        const r = read(d);
        d.close();
        d.remove();
        return r;
      };
      return { plain: make(false), off: make(true) };
    });
    // The control: without the attribute the UA scrim is there (so the assertion below can fail).
    expect(result.plain.bg).toBe("rgba(0, 0, 0, 0.1)");
    expect(result.off.bg).toBe("rgba(0, 0, 0, 0)");
    expect(result.off.filter).toBe("none");
  });

  test("it beats an author ::backdrop rule of any specificity, in any source order", async ({ page }) => {
    const result = await page.evaluate(() => {
      const style = document.createElement("style");
      // (1,3,1) with !important absent: far above the theme rule's (0,0,1); added last, so also later in source order.
      style.textContent = `dialog#probe.mine[open][data-dx-overlay]::backdrop {
        background: rgb(255 0 0 / 50%); backdrop-filter: blur(9px); }`;
      document.head.append(style);
      const d = document.createElement("dialog");
      d.id = "probe";
      d.className = "mine";
      document.body.append(d);
      d.showModal();
      const read = () => {
        const cs = getComputedStyle(d, "::backdrop");
        return { bg: cs.backgroundColor, filter: cs.backdropFilter };
      };
      // Control: the rule above needs `[data-dx-overlay]`, so set the attribute to a value that is NOT
      // "off". The author rule applies, proving it really does out-rank a `:where()` rule.
      d.setAttribute("data-dx-overlay", "on");
      const control = read();
      d.setAttribute("data-dx-overlay", "off");
      const off = read();
      d.close();
      d.remove();
      style.remove();
      return { control, off };
    });
    expect(result.control.bg).toBe("rgba(255, 0, 0, 0.5)");
    expect(result.control.filter).toBe("blur(9px)");
    expect(result.off.bg).toBe("rgba(0, 0, 0, 0)");
    expect(result.off.filter).toBe("none");
  });
});
