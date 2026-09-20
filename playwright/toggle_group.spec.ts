import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

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

    // Exact values confirmed identical before and after the `:where()` fix
    // (light: rgb(176, 176, 176); dark: rgb(62, 62, 62)).
    expect(onAndHovered, `onAtRest=${onAtRest} onAndHovered=${onAndHovered}`).toBe(onAtRest);
  });
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
