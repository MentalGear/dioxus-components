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

/**
 * Hit-target coverage (2026-09-17 production report: "there is almost no
 * hit target for the input"). Root cause: the real `<input>`
 * (`primitives/src/input_otp.rs`) was already sized to the full visible row
 * (`position: absolute; inset: 0` against a `display: inline-flex` parent
 * that is itself sized by the row's own content, since the decorative
 * overlay's `display: contents` wrapper promotes `InputOtpGroup`/
 * `InputOtpSlot`/`InputOtpSeparator` to be that flex container's direct
 * items) -- so bounding-box geometry alone looked fine. The actual bug was
 * paint order plus hit-testing: that `aria-hidden` overlay renders *after*,
 * and therefore paints on top of, the real input in the same stacking
 * context, and its slot/separator boxes cover nearly the entire row, each
 * with the CSS default `pointer-events: auto`. `elementFromPoint` on a
 * running instance confirmed clicking dead-center of a visible slot
 * resolved to that slot's own decorative `<div>`, never `#otp-main` -- only
 * the few-pixel gaps *between* slots (the only screen area no box covered)
 * ever reached the real input. Fixed by giving the overlay wrapper
 * `pointer-events: none` (inherited by every descendant), making it
 * transparent to hit-testing the same way `opacity: 0` already makes the
 * real input transparent to painting.
 */
test.describe("Hit-target coverage (2026-09-17 -- almost no hit target regression)", () => {
  test("the real input's bounding box covers the full visible row", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

    const input = page.locator("#otp-main");
    await expect(input).toBeVisible();

    const row = await page.evaluate(() => {
      const overlay = document.querySelector("#otp-main + div") as HTMLElement;
      const slots = Array.from(overlay.querySelectorAll("[data-slot-index]")) as HTMLElement[];
      const separators = Array.from(overlay.querySelectorAll("div")).filter(
        (d) => d.textContent === "•" && !d.hasAttribute("data-slot-index"),
      ) as HTMLElement[];
      const rects = [...slots, ...separators].map((el) => el.getBoundingClientRect());
      return {
        left: Math.min(...rects.map((r) => r.left)),
        right: Math.max(...rects.map((r) => r.right)),
        top: Math.min(...rects.map((r) => r.top)),
        bottom: Math.max(...rects.map((r) => r.bottom)),
      };
    });
    const inputBox = await input.boundingBox();
    if (!inputBox) throw new Error("expected the real input to have a bounding box");
    const debug = `row=${JSON.stringify(row)} input=${JSON.stringify(inputBox)}`;

    // The real input's hit area must span at least the full visible row
    // (all slots + the separator), 1px slack for subpixel rounding.
    expect(inputBox.x, debug).toBeLessThanOrEqual(row.left + 1);
    expect(inputBox.y, debug).toBeLessThanOrEqual(row.top + 1);
    expect(inputBox.x + inputBox.width, debug).toBeGreaterThanOrEqual(row.right - 1);
    expect(inputBox.y + inputBox.height, debug).toBeGreaterThanOrEqual(row.bottom - 1);
  });

  test("clicking anywhere on a visible slot (not just the gaps between them) focuses the real input", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });

    const input = page.locator("#otp-main");
    await expect(input).toBeVisible();

    // Every slot, at its center and near each of its 4 corners -- the
    // reported symptom was that only the thin gaps *between* the
    // decorative slot boxes were clickable, not the boxes themselves
    // (where a user actually taps).
    const points: { label: string; x: number; y: number }[] = await page.evaluate(() => {
      const slots = Array.from(document.querySelectorAll("#otp-main + div [data-slot-index]")) as HTMLElement[];
      const pts: { label: string; x: number; y: number }[] = [];
      for (const s of slots) {
        const idx = s.getAttribute("data-slot-index");
        const r = s.getBoundingClientRect();
        pts.push({ label: `slot ${idx} center`, x: r.left + r.width / 2, y: r.top + r.height / 2 });
        pts.push({ label: `slot ${idx} top-left`, x: r.left + 2, y: r.top + 2 });
        pts.push({ label: `slot ${idx} top-right`, x: r.right - 2, y: r.top + 2 });
        pts.push({ label: `slot ${idx} bottom-left`, x: r.left + 2, y: r.bottom - 2 });
        pts.push({ label: `slot ${idx} bottom-right`, x: r.right - 2, y: r.bottom - 2 });
      }
      return pts;
    });
    expect(points.length).toBeGreaterThan(0);

    for (const p of points) {
      await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
      await page.mouse.click(p.x, p.y);
      await expect(input, `clicking ${p.label} at (${p.x}, ${p.y}) should focus the real input`).toBeFocused();
    }
  });
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
