import { test, expect } from "@playwright/test";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=popover&");
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  await expect(popoverButton).toBeVisible();
  await popoverButton.click();
  // pressing the first input should be focused
  const confirm = page.getByRole("button", { name: "Confirm" });
  const cancel = page.getByRole("button", { name: "Cancel" });
  await expect(confirm).toBeFocused();
  // pressing tab again should focus the cancel button
  await page.keyboard.press("Tab");
  await expect(cancel).toBeFocused();
  // pressing tab again should focus the confirm button again
  await page.keyboard.press("Tab");
  await expect(confirm).toBeFocused();
  // pressing enter should close the popover
  await page.keyboard.press("Enter");
  // the item should show deleted under component-preview-frame
  await expect(page.locator("#component-preview-frame")).toContainText(
    "Item deleted!",
  );

  // Open the popover again
  await popoverButton.click();
  // pressing escape should close the popover
  await page.keyboard.press("Escape");
});

test("popover dismisses when clicking outside", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=popover&");
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  await popoverButton.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  // Click far outside the popover (corner of the document) — should dismiss.
  await page.mouse.click(2, 2);
  await expect(dialog).toHaveCount(0);
});

test("popover stays open when clicking non-focusable content inside it", async ({ page }) => {
  // Regression: `use_outside_dismiss` served pointerdown and focusin with one
  // shared handler. Clicking a non-focusable region inside the popover (e.g.
  // this demo's "Delete Item?" heading) blurs the currently-focused control,
  // and the browser moves focus to the nearest focusable *ancestor* -- which
  // is outside the popover's root while still containing it. The shared
  // handler read that as focus leaving and closed the popover the user just
  // clicked into.
  await page.goto("http://127.0.0.1:8080/component/?name=popover&");
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  await popoverButton.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  await dialog.getByText("Delete Item?").click();

  // The popover must still be open -- clicking its own non-focusable content
  // is not an outside dismiss.
  await expect(dialog).toBeVisible();
});

test("rapid open/close/open settles on the correct final state", async ({ page }) => {
  // Regression for `use_animated_open`'s unmount race: `open` is captured by
  // value when the close effect spawns its async task. If the user reopens
  // before that task's animation settles, a naive fix can let the stale
  // task's `show_in_dom.set(false)` run after the fresh reopen already set
  // `show_in_dom.set(true)` -- the popover vanishes although it is open.
  await page.goto("http://127.0.0.1:8080/component/?name=popover&");
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  const dialog = page.getByRole("dialog");

  // Toggle open, closed, open again in rapid succession -- no waits, so the
  // close animation from the middle toggle is still in flight when the
  // final open fires.
  await popoverButton.click();
  await popoverButton.click();
  await popoverButton.click();

  // Final state must be open, and must *stay* open -- not flicker closed
  // once the stale close task's animation would have settled.
  await expect(dialog).toBeVisible();
  await page.waitForTimeout(600);
  await expect(dialog).toBeVisible();
});

test("an animation cancelled with no successor cycle still unmounts", async ({ page }) => {
  // Regression for the residual leak in the upstream unmount-race fix: it
  // declines to write `show_in_dom` when the close animation's promise
  // rejects, reasoning that a rejection means a newer cycle is already in
  // flight to take over. That reasoning doesn't hold when the animation is
  // cancelled by something *other* than a newer open/close cycle (e.g. a
  // script cancelling it directly) -- with no successor cycle to flip
  // `show_in_dom`, the closed (but still `opacity: 0`) node stays mounted
  // forever. The generation counter must apply the stale cycle's own result
  // in that case, since no newer generation exists to own it.
  await page.goto("http://127.0.0.1:8080/component/?name=popover&");
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  const dialog = page.getByRole("dialog");

  await popoverButton.click();
  await expect(dialog).toBeVisible();

  // Raw DOM presence, not `getByRole` -- the content sets `aria-hidden="true"`
  // synchronously as soon as `open` flips false, which removes it from the
  // accessibility tree (and so from `getByRole('dialog')`) well before the
  // animation/unmount race this test is about is resolved one way or the
  // other. Only a literal DOM query reveals whether the node actually leaks.
  const domNode = page.locator('[role="dialog"]');

  // Start closing, then cancel its CSS animation directly -- simulating an
  // external interruption, not a re-open.
  await popoverButton.click();
  await page.waitForTimeout(30);
  await page.evaluate(() => {
    const el = document.querySelector('[role="dialog"]');
    el?.getAnimations().forEach((a) => a.cancel());
  });

  // No further open/close cycle follows. The element must still eventually
  // unmount rather than leak in the DOM in its closed-but-mounted form.
  await expect(domNode).toHaveCount(0, { timeout: 2000 });
});
