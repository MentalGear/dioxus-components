import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { startFadeSampling, assertFadesOutThenUnmounts } from "./assert-fade-out";

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
  // Native-dialog engine migration (two-engine overlay architecture
  // completion): the modal `Popover` is now a real `<dialog>` +
  // `showModal()` on the web arm, so this cycle goes through Chromium's own
  // focus trap rather than the vendored `FocusTrap` -- same documented shape
  // as `dialog.spec.ts`'s identical comment (docs/phase4-spike-findings.md
  // experiment 4a): Chromium's native trap parks focus on `<body>` for
  // exactly one Tab stop after the last focusable element before wrapping
  // to the first (invisible to the user, does not let focus escape the
  // dialog) -- a harness correction for the new trap's documented shape,
  // not a behavior change under test.
  await page.keyboard.press("Tab");
  await expect
    .poll(() => page.evaluate(() => document.activeElement === document.body))
    .toBe(true);
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
  // final open fires. The middle "closed" toggle is Escape, not a second
  // click on the trigger button: native-dialog engine migration (two-engine
  // overlay architecture completion) -- the modal `Popover` is now a real
  // `<dialog>` + `showModal()` on the web arm, and the trigger sits *outside*
  // that dialog, so it is correctly background-inert (same rule
  // `oracle/tier2-html/native-dialog.spec.ts` Rule 2 already covers for
  // `Dialog`) and a raw click on it while open never reaches its handler at
  // all -- Escape reaches the dialog's own `cancel`/`close` handling
  // (synced back to `open` by `use_dialog_close_sync`) the same way a user
  // closing it would, and still races `use_animated_open`'s exit cycle the
  // exact same way a second trigger click used to.
  await popoverButton.click();
  await page.keyboard.press("Escape");
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
  // external interruption, not a re-open. Escape, not a second trigger
  // click -- see the identical comment in the "rapid open/close/open" test
  // above for why the (now correctly background-inert) trigger can no
  // longer be clicked while this modal `Popover` is open.
  await page.keyboard.press("Escape");
  await page.waitForTimeout(30);
  await page.evaluate(() => {
    const el = document.querySelector('[role="dialog"]');
    el?.getAnimations().forEach((a) => a.cancel());
  });

  // No further open/close cycle follows. The element must still eventually
  // unmount rather than leak in the DOM in its closed-but-mounted form.
  await expect(domNode).toHaveCount(0, { timeout: 2000 });
});

test.describe("Axe automated scan", () => {
  test("loaded (popover closed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=popover&");
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole("button", { name: "Show Popover" })).toBeVisible();
    await expectNoAxeViolations(page, "popover: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=popover&");
    await page.getByRole("button", { name: "Show Popover" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await expectNoAxeViolations(page, "popover: open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// dev-docs/backlog.md rows 19 and 7: on the web (popover) arm,
// `use_popover_sync` used to call `hidePopover()` the instant `open` went
// false, and the UA's `[popover]:not(:popover-open) { display: none }`
// rule then hid the content before `dx-popover-fade-out`
// (`../preview/src/components/popover/style.css`, which already had this
// animation defined -- it just never got to play) ever got a chance to
// run -- RED before both fixes landed (reasoned through, not executed,
// since this lane cannot run Playwright: `display: none` from the very
// first sample, so `assertFadesOutThenUnmounts` would fail both the "no
// display:none while mounted" invariant and the "a few frames of
// decreasing opacity" check immediately). GREEN once
// `primitives/src/top_layer.rs`'s `use_popover_shown_while_mounted`
// (row 19) keeps the popover shown through the animation that CSS already
// defined (row 7).
//
// Targets the `non_modal` variant (`preview/src/components/popover/
// variants/non_modal/mod.rs`, `is_modal: false`), not this file's own
// `?name=popover&` *main*-variant demo above: that demo's `PopoverRoot`
// never sets `is_modal`, so it defaults to `true`
// (`primitives/src/popover.rs`) and renders as a real `<dialog>` +
// `showModal()` -- a completely different code path
// (`use_dialog_open_driver`/`use_dialog_close_sync`, no `popover`
// attribute at all) that neither row 19 nor row 7 touches.
//
// This is *not* the `top_layer` oracle fixture's own `#stack-popover-*`
// instance (an earlier version of this test used it, and that was wrong):
// that fixture composes the raw `dioxus_primitives::popover` primitive
// directly, on purpose (`scripts/check-preview-composition.sh`'s
// documented exemption for it, for native-vs-library positioning/
// interaction comparisons -- see that fixture's own header comment), so it
// never loads `../preview/src/components/popover/style.css` and has no
// close animation defined at all: `getAnimations()` is empty, so
// `use_animated_open` correctly unmounts immediately -- a fixture-
// composition gap, not a defect in rows 19/7's fix. `main`/`non_modal`
// (this file's route) go through the themed `PopoverRoot` wrapper
// (`preview/src/components/popover/component.rs`), which *does* load that
// stylesheet via its own `document::Link`, the same way every other
// themed component page does -- the same reason Tooltip's/HoverCard's
// close-fade tests above (already on themed routes) don't have this
// problem.
//
// Both `main` and `non_modal` render on this one page (`ComponentDemo`'s
// "Normal" component layout puts every variant on the same route, not
// behind a `?variant=` query param -- confirmed by reading
// `preview/src/main.rs`'s `ComponentHighlight`/`ComponentVariantHighlight`;
// `select.spec.ts`'s `?variant=multi&` etc. disambiguate by each variant's
// own unique visible text for the same reason, not the URL), so this test
// locates its trigger by the non-modal demo's own distinct label ("Open
// popover", vs. the main/modal demo's "Show Popover") rather than by
// `#component-preview-frame`, which both variants' `Tabs` share -- see
// `preview/src/main.rs`'s `ComponentVariantHighlight`, which hard-codes
// that id onto every variant's own "Demo" tab panel.
//
// The close is triggered via `.evaluate(el => el.click())`, not
// Playwright's `.click()` (same as this file's own "test" above, for a
// related reason): a real `.click()` dispatches a genuine `pointerdown`
// first, which native `popover="auto"` light dismiss reacts to
// synchronously -- closing the popover (and applying `display: none`)
// *before* Rust's own `open` signal (and this content's close animation)
// ever gets involved, exactly the "native close bypasses the exit
// animation" limit `use_popover_shown_while_mounted`'s own doc describes
// as accepted and out of scope. A JS-level `.click()` call dispatches only
// a `click` event (no `pointerdown`), so it never engages light dismiss at
// all -- only this trigger's own `onclick` handler
// (`ctx.set_open.call(!(ctx.open)())`, `primitives/src/popover.rs`) runs,
// a genuinely script-driven toggle, exactly the path row 19 fixes.
test.describe("Close-fade animation, non-modal arm (docs/backlog.md rows 19, 7)", () => {
  test("content fades out (opacity -> 0, still popover-open) before unmounting", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=popover&variant=non_modal&", {
      timeout: 20 * 60 * 1000,
    });
    const trigger = page.getByRole("button", { name: "Open popover" });
    const content = page.getByRole("dialog");

    await expect(trigger).toBeVisible();
    await trigger.evaluate((el) => (el as HTMLElement).click());
    await expect(content).toBeVisible();
    const contentId = await content.getAttribute("id");
    if (!contentId) throw new Error("non-modal popover content has no id to sample");

    // Start sampling before triggering the close, so the first frames
    // (still data-state="open") are never missed -- see
    // assert-fade-out.ts's `startFadeSampling` doc.
    const framesPromise = startFadeSampling(page, contentId);
    await trigger.evaluate((el) => (el as HTMLElement).click());
    const samples = await framesPromise;

    assertFadesOutThenUnmounts(samples);
  });
});
