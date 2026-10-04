import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=toggle_group&`);
  const b_button = page.getByRole("button", { name: "B", exact: true });
  const i_button = page.getByRole("button", { name: "I", exact: true });
  const u_button = page.getByRole("button", { name: "U", exact: true });

  // The buttons should not be selected initially
  await expect(b_button).toHaveAttribute("data-state", "off");
  await expect(i_button).toHaveAttribute("data-state", "off");
  await expect(u_button).toHaveAttribute("data-state", "off");

  // Click the "B" button and check its state
  await b_button.click();
  await expect(b_button).toHaveAttribute("data-state", "on");

  // Pressing right arrow should select the "I" button
  await page.keyboard.press("ArrowRight");
  await expect(i_button).toBeFocused();

  // Pressing enter should focus the "I" button
  await page.keyboard.press("Enter");
  await expect(i_button).toHaveAttribute("data-state", "on");

  // Pressing right two more times should bring us back to the "B" button
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(b_button).toBeFocused();
});

// HARDENING (docs/backlog.md row 89), not a reproduced defect -- see
// `toggle.spec.ts`'s identical test for the full writeup (same
// `:where(:hover)` construction, same "already correct via source order,
// now correct via specificity" story).
for (const dark of [false, true]) {
  test(`a pressed item keeps its "on" background while hovered (${dark ? "dark" : "light"} mode)`, async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle_group&${dark ? "dark_mode=true" : ""}`);
    const bButton = page.getByRole("button", { name: "B", exact: true });

    await bButton.click();
    await expect(bButton).toHaveAttribute("data-state", "on");
    await page.locator("body").hover({ position: { x: 0, y: 0 } });
    // Wait past the CSS transition (--dx-motion-duration-slow, 200ms)
    // before reading computed style -- a mid-transition read returns an
    // interpolated, neither-old-nor-new color.
    await page.waitForTimeout(400);
    const onAtRest = await bButton.evaluate((el) => getComputedStyle(el).backgroundColor);

    await bButton.hover();
    await page.waitForTimeout(400);
    expect(await bButton.evaluate((el) => el.matches(":hover"))).toBe(true);
    const onAndHovered = await bButton.evaluate((el) => getComputedStyle(el).backgroundColor);

    // The invariant the `:where()` fix makes durable (pressed keeps its own
    // background under the pointer), not a particular colour -- the pressed
    // colour is derived from the preset-aware role tokens.
    expect(onAndHovered, `onAtRest=${onAtRest} onAndHovered=${onAndHovered}`).toBe(onAtRest);
  });
}

// The pressed background is derived from the preset-aware role tokens (it used to be the ramp step
// `--primary-color-7`, grey under every base), so under a TINTED base it must (a) carry the tint and
// (b) still be three visibly different backgrounds: resting, hovered, pressed. The exhaustive base x
// accent x mode proof is `theme-preset-contrast.spec.ts`; this keeps the same assertion next to the
// component's own specs.
const GAP_PRESSED = 16; // per channel, 0-255
const GAP_HOVER = 6;

async function paintedRgb(page: Page, css: string): Promise<[number, number, number]> {
  return page.evaluate((c) => {
    const cx = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
    cx.canvas.width = cx.canvas.height = 1;
    cx.fillStyle = c;
    cx.fillRect(0, 0, 1, 1);
    const d = cx.getImageData(0, 0, 1, 1).data;
    return [d[0], d[1], d[2]] as [number, number, number];
  }, css);
}

const gap = (a: number[], b: number[]) => Math.max(...a.map((v, i) => Math.abs(v - b[i])));

for (const dark of [false, true]) {
  for (const base of ["slate", "gray"]) {
    test(`resting, hovered and pressed items are three different backgrounds under a ${base} base (${dark ? "dark" : "light"} mode)`, async ({ page }) => {
      // Settled first: a late (re)render of the page would drop the attribute set below.
      await gotoHydrated(page, `${BASE_URL}/component/?name=toggle_group&${dark ? "dark_mode=true" : ""}`);
      await page.evaluate((b) => {
        document.documentElement.setAttribute("data-theme-base", b);
        // Settled values, not a mid-transition read.
        document.head.insertAdjacentHTML("beforeend", "<style>*{transition:none!important}</style>");
      }, base);
      const item = (name: string) => page.getByRole("button", { name, exact: true });
      const paint = async (name: string) =>
        paintedRgb(page, await item(name).evaluate((el) => getComputedStyle(el).backgroundColor));

      // B pressed, I hovered, U resting: all three states at once.
      await item("B").click();
      await expect(item("B")).toHaveAttribute("data-state", "on");
      await item("I").hover();
      expect(await item("I").evaluate((el) => el.matches(":hover"))).toBe(true);
      const pressed = await paint("B");
      const hovered = await paint("I");
      const rest = await paint("U");

      const msg = `rest=${rest} hovered=${hovered} pressed=${pressed}`;
      expect(gap(pressed, hovered), msg).toBeGreaterThanOrEqual(GAP_PRESSED);
      expect(gap(pressed, rest), msg).toBeGreaterThanOrEqual(GAP_PRESSED);
      expect(gap(hovered, rest), msg).toBeGreaterThanOrEqual(GAP_HOVER);
      // The tint reaches the pressed state: the old ramp step was pure grey (r == g == b) under every base.
      expect(new Set(pressed).size, `pressed ${pressed} is neutral grey, not ${base}-tinted`).toBeGreaterThan(1);
    });
  }
}

test.describe("Axe automated scan", () => {
  test("loaded (none selected) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle_group&`);
    await expectNoAxeViolations(page, "toggle_group: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("an item selected has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle_group&`);
    await page.getByRole("button", { name: "B", exact: true }).click();
    await expectNoAxeViolations(page, "toggle_group: B selected", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
