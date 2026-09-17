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

/**
 * Click-to-caret positioning (2026-09-17 production report: "first box has
 * normal hit target, but none of the others"). Root cause: the real
 * `<input>` is `opacity: 0` with `font-family: inherit` and no
 * `letter-spacing` tuned to the visual slot grid
 * (`preview/src/components/input_otp/style.css`'s `.dx-input-otp-input`
 * sets none) -- clicking a native text input places the caret based on
 * where the browser thinks the *actual rendered glyphs* are, and since
 * those invisible glyphs' positions were never aligned to the 40px-wide
 * slot boxes, a click's x-coordinate doesn't map to the "correct" character
 * index at all. Confirmed live before this fix: on an empty field, clicking
 * slot 0, 3, or 5 all placed the caret at index 0; on a field filled with
 * "123", clicking slot 0 landed at index 2 while slots 1/4/5 all landed at
 * index 3. Fixed by computing the clicked slot from geometry (point-in-rect
 * against each `[data-slot-index]`'s own `getBoundingClientRect()`, since
 * the slots are `pointer-events: none` and unreachable via
 * `elementFromPoint`) and explicitly calling `setSelectionRange` --
 * `primitives/src/input_otp.rs`'s `InputOtp` `onclick` handler and
 * `snap_caret_to_slot`.
 *
 * Clamping rule: a clicked slot's index is clamped to `min(slot_index,
 * value.length)`. This is not just "a" reasonable rule -- it's the only one
 * a real `<input>` can honor: `setSelectionRange` itself clamps its
 * arguments to `[0, value.length]`, so a genuinely empty field can *only*
 * ever have `selectionStart === 0`, for any slot clicked (there is no
 * character position to place a caret at for slot 3 when the value is
 * "" -- the caret has nowhere else valid to go). That is expected,
 * necessary behavior, not a regression: the tests below cover the concrete,
 * fixable case (a partially/fully filled value, where clicks used to
 * collapse to the wrong, uniform position) and separately document the
 * empty-field case landing at 0 for every slot on purpose.
 */
test.describe("Click-to-caret positioning (2026-09-17 -- second, deeper hit-target regression)", () => {
  const slotCenter = async (page: import("@playwright/test").Page, prefix: string, index: number) => {
    const box = await page.locator(`${prefix} + div [data-slot-index="${index}"]`).boundingBox();
    if (!box) throw new Error(`slot ${index} has no bounding box`);
    return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  };

  test("clicking distinct empty slots each clamp to index 0 -- the only valid caret position on an empty value", async ({
    page,
  }) => {
    // A real `<input>`'s `selectionStart` cannot exceed `value.length`
    // (`setSelectionRange` clamps its arguments), so every slot's click
    // must land at 0 here -- for an empty value, 0 is the *only* valid
    // caret position, not a leftover bug. What must differ from before the
    // fix is which slot is shown active: only slot 0 (the slot the caret
    // can actually be at), never 3 or 5.
    for (const idx of [0, 3, 5]) {
      await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
      const input = page.locator("#otp-main");
      const p = await slotCenter(page, "#otp-main", idx);
      await page.mouse.click(p.x, p.y);
      await expect(input, `clicking empty slot ${idx}`).toHaveJSProperty("selectionStart", 0);
      await expect(page.locator(`#otp-main + div [data-slot-index="0"]`)).toHaveAttribute("data-active", "true");
      if (idx !== 0) {
        await expect(page.locator(`#otp-main + div [data-slot-index="${idx}"]`)).toHaveAttribute(
          "data-active",
          "false",
        );
      }
    }
  });

  test("clicking a partially-filled input's slots lands at that slot's own index, clamped to the value's length", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    await input.click();
    await page.keyboard.type("123");

    // Before the fix, every one of these landed at either 2 (slot 0) or 3
    // (slots 1/4/5) -- clicking slot 0 never gave 0, and slots 1/4/5 were
    // indistinguishable from each other despite being different slots.
    const cases: Array<[number, number]> = [
      [0, 0],
      [1, 1],
      [2, 2],
      [3, 3], // past the last character ("3" doesn't exist) -> clamps to the end
      [4, 3], // same clamp -- distinct slot, same (correct) end position
      [5, 3],
    ];
    for (const [slotIndex, expectedSelectionStart] of cases) {
      const p = await slotCenter(page, "#otp-main", slotIndex);
      await page.mouse.click(p.x, p.y);
      await expect(input, `clicking slot ${slotIndex} on value "123"`).toHaveJSProperty(
        "selectionStart",
        expectedSelectionStart,
      );
    }
  });

  test("clicking a fully-filled input's slots lands exactly at that slot's own index, no clamping needed", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    await input.click();
    await page.keyboard.type("123456");

    for (const idx of [0, 1, 2, 3, 4, 5]) {
      const p = await slotCenter(page, "#otp-main", idx);
      await page.mouse.click(p.x, p.y);
      await expect(input, `clicking slot ${idx} on a full value`).toHaveJSProperty("selectionStart", idx);
      await expect(page.locator(`#otp-main + div [data-slot-index="${idx}"]`)).toHaveAttribute("data-active", "true");
    }
  });

  test("disabled state still blocks click-to-caret positioning", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-disabled");
    const p = await slotCenter(page, "#otp-disabled", 1);
    await page.mouse.click(p.x, p.y);
    await expect(input).not.toBeFocused();
    await expect(input).toHaveValue("12");
  });
});

/**
 * Click-then-type replace (2026-09-17 production report, found using the
 * app normally: "clicking just any field should be able to correct that
 * number. I'm in the selected field then typing a number right [and it
 * doesn't correct]"). Root cause: the previous round's click-to-caret fix
 * (`snap_caret_to_slot`, see the "Click-to-caret positioning" describe block
 * above) moved the caret with a *collapsed* selection
 * (`setSelectionRange(idx, idx)`), and native `<input>` typing *inserts* at
 * a collapsed caret rather than overwriting -- confirmed live before this
 * fix: on a full 6-digit value, clicking slot 2 and typing "9" left the
 * value completely unchanged (inserting would exceed `maxlength`, so the
 * browser silently refused the keystroke), and on a partial value "123",
 * clicking slot 1 and typing "9" produced "1923" (inserted before "2",
 * shifting "23" right) instead of correcting that one digit. Fixed by
 * selecting the clicked character (`setSelectionRange(idx, idx + 1,
 * 'forward')`) whenever one exists at that index, so typing over the
 * selection replaces it -- see `primitives/src/input_otp.rs`'s
 * `snap_caret_to_slot` doc for the full rationale, including why
 * `'forward'` was chosen for the selection direction.
 */
test.describe("Click-then-type replace (2026-09-17 -- selection-vs-collapsed-caret regression)", () => {
  const slotCenter = async (page: import("@playwright/test").Page, prefix: string, index: number) => {
    const box = await page.locator(`${prefix} + div [data-slot-index="${index}"]`).boundingBox();
    if (!box) throw new Error(`slot ${index} has no bounding box`);
    return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  };

  test("clicking a slot in a full value selects that one character, so typing replaces it instead of being blocked by maxlength", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    await input.click();
    await page.keyboard.type("123456");
    await expect(input).toHaveValue("123456");

    const p = await slotCenter(page, "#otp-main", 2);
    await page.mouse.click(p.x, p.y);
    // A one-character selection, not a collapsed caret -- this is what makes
    // the following keystroke replace instead of insert-and-get-blocked.
    await expect(input).toHaveJSProperty("selectionStart", 2);
    await expect(input).toHaveJSProperty("selectionEnd", 3);

    await page.keyboard.type("9");
    // Before the fix: stayed "123456" (maxlength silently blocked the insert).
    await expect(input).toHaveValue("129456");
  });

  test("clicking a slot in a partial value selects that one character, so typing replaces it instead of shifting the rest right", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    await input.click();
    await page.keyboard.type("123");
    await expect(input).toHaveValue("123");

    const p = await slotCenter(page, "#otp-main", 1);
    await page.mouse.click(p.x, p.y);
    await expect(input).toHaveJSProperty("selectionStart", 1);
    await expect(input).toHaveJSProperty("selectionEnd", 2);

    await page.keyboard.type("9");
    // Before the fix: became "1923" (inserted before "2" instead of
    // replacing it).
    await expect(input).toHaveValue("193");
  });

  test("clicking the last filled character replaces just that character, not an insert past maxlength", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    await input.click();
    await page.keyboard.type("12345");
    await expect(input).toHaveValue("12345");

    const p = await slotCenter(page, "#otp-main", 4);
    await page.mouse.click(p.x, p.y);
    await expect(input).toHaveJSProperty("selectionStart", 4);
    await expect(input).toHaveJSProperty("selectionEnd", 5);

    await page.keyboard.type("9");
    await expect(input).toHaveValue("12349");
  });

  test("clicking past the end of an empty value still collapses to a plain caret at 0 -- nothing exists there to select", async ({
    page,
  }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000, waitUntil: "networkidle" });
    const input = page.locator("#otp-main");
    const p = await slotCenter(page, "#otp-main", 3);
    await page.mouse.click(p.x, p.y);
    await expect(input).toHaveJSProperty("selectionStart", 0);
    await expect(input).toHaveJSProperty("selectionEnd", 0);

    await page.keyboard.type("9");
    await expect(input).toHaveValue("9");
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
