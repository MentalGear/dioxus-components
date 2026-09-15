import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

const URL = "http://127.0.0.1:8080/component/?name=input_otp&";

test("typing fills the boxes and the real input's value matches", async ({ page }) => {
  await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

  const input = page.locator("#otp-main");
  await expect(input).toBeVisible();

  await input.click();
  await page.keyboard.type("123456");

  // The one real input is the source of truth.
  await expect(input).toHaveValue("123456");

  // Every visual box mirrors the matching character.
  const digits = "123456";
  for (let i = 0; i < digits.length; i++) {
    await expect(page.locator(`#otp-main + div [data-slot-index="${i}"]`)).toHaveText(digits[i]);
  }

  // `on_complete`/`on_value_change` wiring reaches the demo's own display.
  await expect(page.locator("#input-otp-value")).toContainText("Value: 123456");
});

test("paste fills all slots at once", async ({ page }) => {
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

  const input = page.locator("#otp-main");
  await input.click();
  await page.evaluate((text) => navigator.clipboard.writeText(text), "654321");
  await input.press("Control+V");

  await expect(input).toHaveValue("654321");
  const digits = "654321";
  for (let i = 0; i < digits.length; i++) {
    await expect(page.locator(`#otp-main + div [data-slot-index="${i}"]`)).toHaveText(digits[i]);
  }
});

test("Backspace/Delete/ArrowLeft/ArrowRight move the active slot", async ({ page }) => {
  await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

  const input = page.locator("#otp-main");
  const slot = (i: number) => page.locator(`#otp-main + div [data-slot-index="${i}"]`);

  await input.click();
  await page.keyboard.type("123456");
  // Typing advances the active slot -- after a full 6-digit value the caret
  // has nowhere left to go, so the last slot stays active.
  await expect(slot(5)).toHaveAttribute("data-active", "true");

  await page.keyboard.press("Home");
  await expect(slot(0)).toHaveAttribute("data-active", "true");

  await page.keyboard.press("ArrowRight");
  await expect(slot(1)).toHaveAttribute("data-active", "true");
  await expect(slot(0)).toHaveAttribute("data-active", "false");

  // Backspace deletes the character before the caret (index 0, "1") and
  // moves the caret back with it.
  await page.keyboard.press("Backspace");
  await expect(input).toHaveValue("23456");
  await expect(slot(0)).toHaveAttribute("data-active", "true");

  // Delete removes the character at the caret ("2") without moving it.
  await page.keyboard.press("Delete");
  await expect(input).toHaveValue("3456");
  await expect(slot(0)).toHaveAttribute("data-active", "true");
  await expect(slot(0)).toHaveText("3");

  await page.keyboard.press("End");
  await expect(slot(4)).toHaveAttribute("data-active", "true");

  await page.keyboard.press("ArrowLeft");
  await expect(slot(3)).toHaveAttribute("data-active", "true");
  await expect(slot(4)).toHaveAttribute("data-active", "false");
});

test("disabled state blocks input", async ({ page }) => {
  await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

  const input = page.locator("#otp-disabled");
  await expect(input).toBeDisabled();
  await expect(input).toHaveValue("12");

  // A disabled input cannot receive focus, so a click/keyboard attempt is a
  // no-op -- the value and the boxes it drives stay exactly as they were.
  await input.click({ force: true });
  await page.keyboard.type("99");
  await expect(input).toHaveValue("12");
  await expect(page.locator(`#otp-disabled + div [data-slot-index="0"]`)).toHaveText("1");
  await expect(page.locator(`#otp-disabled + div [data-slot-index="1"]`)).toHaveText("2");
});

test.describe("Axe automated scan", () => {
  // Input OTP has no overlay/expand/select interaction -- like Input/Input
  // Group, one state to scan (docs/conformance-harness.md).
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    await expect(page.locator("#otp-main")).toBeVisible();
    await expectNoAxeViolations(page, "input_otp: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
