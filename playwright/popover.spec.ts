import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { sampleCloseFade, assertFadesOutThenUnmounts } from "./assert-fade-out";
import { BASE_URL } from "./base-url";
import { assertSharedBackdrop, captureBackdrop, resolveOverlayMs } from "./assert-backdrop-fade";

test("opens a form of labelled fields, traps focus, Escape closes", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=popover&`);
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  await expect(popoverButton).toBeVisible();
  await popoverButton.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  // showModal() focuses the first field. shadcn's own demo content: four
  // Label + Input rows under a "Dimensions" header.
  const width = dialog.getByRole("textbox", { name: "Width", exact: true });
  await expect(width).toBeFocused();
  // Tab walks the other three fields in order.
  for (const name of ["Max. width", "Height", "Max. height"]) {
    await page.keyboard.press("Tab");
    await expect(dialog.getByRole("textbox", { name, exact: true })).toBeFocused();
  }
  // Native-dialog engine migration (two-engine overlay architecture
  // completion): the modal `Popover` is a real `<dialog>` + `showModal()` on
  // the web arm, so this cycle goes through Chromium's own focus trap rather
  // than the vendored `FocusTrap` -- same documented shape as
  // `dialog.spec.ts`'s identical comment (docs/phase4-spike-findings.md
  // experiment 4a): Chromium's native trap parks focus on `<body>` for
  // exactly one Tab stop after the last focusable element before wrapping to
  // the first (invisible to the user, does not let focus escape the dialog)
  // -- a harness correction for the new trap's documented shape, not a
  // behavior change under test.
  await page.keyboard.press("Tab");
  await expect
    .poll(() => page.evaluate(() => document.activeElement === document.body))
    .toBe(true);
  await page.keyboard.press("Tab");
  await expect(width).toBeFocused();

  // Escape closes it and gives focus back to the trigger.
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(popoverButton).toBeFocused();

  // And it reopens: the signal is not stranded by the native close.
  await popoverButton.click();
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
});

test("the trigger announces a dialog popup and mirrors the open state in aria-expanded", async ({ page }) => {
  // Radix/shadcn Popover trigger semantics: `aria-haspopup="dialog"` plus `aria-expanded`. Found missing
  // 2026-10-04 (the themed `[aria-expanded="true"]` trigger rule had nothing to match). Located by
  // attribute and text, not role: while the modal one is open its trigger is inert, so `getByRole`
  // would no longer find it to read the attribute back.
  await page.goto(`${BASE_URL}/component/?name=popover&`);
  const dialog = page.getByRole("dialog");
  // The modal variant (showModal()) and the non-modal one (popover="auto"); Escape closes both.
  for (const text of ["Show Popover", "Open popover"]) {
    const trigger = page.locator('button[aria-haspopup="dialog"]', { hasText: text });
    await expect(trigger).toHaveCount(1);
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
    await trigger.click();
    await expect(dialog).toBeVisible();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(trigger).toHaveAttribute("aria-expanded", "false");
  }
});

test("the content is labelled form fields in a dialog, not a menu", async ({ page }) => {
  // The previous demo was a "Delete Item?" heading over two full-width outline
  // buttons: a confirm/cancel stack that read as a menu. A popover holds
  // arbitrary content; this one holds shadcn's "Dimensions" form, and nothing in
  // it has a menu role because nothing in it is a menu.
  await page.goto(`${BASE_URL}/component/?name=popover&`);
  const trigger = page.getByRole("button", { name: "Show Popover" });
  await trigger.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  await expect(dialog.getByRole("heading", { name: "Dimensions" })).toBeVisible();
  await expect(dialog.getByText("Set the dimensions for the layer.")).toBeVisible();
  await expect(dialog.getByRole("menu")).toHaveCount(0);
  await expect(dialog.getByRole("menuitem")).toHaveCount(0);
  await expect(dialog.getByRole("button")).toHaveCount(0);
  await expect(dialog.getByRole("textbox")).toHaveCount(4);
  for (const [name, value] of [
    ["Width", "100%"],
    ["Max. width", "300px"],
    ["Height", "25px"],
    ["Max. height", "none"],
  ]) {
    // Found by its <label>, so each field is genuinely associated with one.
    await expect(dialog.getByRole("textbox", { name, exact: true })).toHaveValue(value);
  }
});

test("the trigger is Nova's default outline button", async ({ page }) => {
  // It used to be a card-coloured 18px-padded box of its own, unlike the
  // `Button { data-style: "outline" }` every other overlay demo opens from.
  await page.goto(`${BASE_URL}/component/?name=popover&`);
  const trigger = page.getByRole("button", { name: "Show Popover" });
  await expect(trigger).toBeVisible();
  const box = await trigger.evaluate((el) => {
    const cs = getComputedStyle(el);
    return {
      height: el.getBoundingClientRect().height,
      border: cs.borderTopWidth,
      radius: cs.borderTopLeftRadius,
      weight: cs.fontWeight,
      size: cs.fontSize,
    };
  });
  expect(box).toEqual({ height: 32, border: "1px", radius: "10px", weight: "500", size: "14px" });
});

test("popover dismisses when clicking outside", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=popover&`);
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
  // this demo's "Dimensions" heading) blurs the currently-focused control,
  // and the browser moves focus to the nearest focusable *ancestor* -- which
  // is outside the popover's root while still containing it. The shared
  // handler read that as focus leaving and closed the popover the user just
  // clicked into.
  await page.goto(`${BASE_URL}/component/?name=popover&`);
  const popoverButton = page.getByRole("button", { name: "Show Popover" });
  await popoverButton.click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();

  await dialog.getByRole("heading", { name: "Dimensions" }).click();

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
  await page.goto(`${BASE_URL}/component/?name=popover&`);
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
  await page.goto(`${BASE_URL}/component/?name=popover&`);
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
    await page.goto(`${BASE_URL}/component/?name=popover&`);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole("button", { name: "Show Popover" })).toBeVisible();
    await expectNoAxeViolations(page, "popover: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&`);
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
// (`use_popover_modal_driver`/`use_dialog_close_sync`, no `popover`
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
    await page.goto(`${BASE_URL}/component/?name=popover&variant=non_modal&`, {
      timeout: 20 * 60 * 1000,
    });
    const trigger = page.getByRole("button", { name: "Open popover" });
    const content = page.getByRole("dialog");

    await expect(trigger).toBeVisible();
    await trigger.evaluate((el) => (el as HTMLElement).click());
    await expect(content).toBeVisible();
    const contentId = await content.getAttribute("id");
    if (!contentId) throw new Error("non-modal popover content has no id to sample");

    // `sampleCloseFade` settles the OPEN fade-in first, starts sampling before
    // the close trigger so the first frames are never missed, fires the
    // trigger and returns the capture -- see assert-fade-out.ts.
    const capture = await sampleCloseFade(page, contentId, () =>
      trigger.evaluate((el) => (el as HTMLElement).click()),
    );

    assertFadesOutThenUnmounts(capture);
  });
});

// The scrim. The modal Popover is a `<dialog>` + `showModal()` with no wrapper of
// its own, so before the shared-scrim fix its whole scrim was the UA's default
// `::backdrop` (`rgba(0 0 0 / 10%)`, no transition): it appeared in one frame and
// vanished in one frame. It now paints the same scrim as Dialog/AlertDialog/Sheet/
// Drawer and fades it in and out for the same length (`assert-backdrop-fade.ts` owns
// the claim; the other four specs run the identical assertion).
//
// But a popover dims nothing by default: shadcn's has no overlay, so `PopoverRoot`'s
// `overlay` prop defaults to false, which puts `data-dx-overlay="off"` on the dialog
// and the theme's one `dialog[data-dx-overlay="off"]::backdrop` rule removes the scrim
// (`dx-components-theme.css`; `overlay-switch.spec.ts` covers that rule on every modal
// component). Modality is a separate axis (`is_modal`) and is unaffected: the main demo
// is modal and still undimmed, the `overlay` variant is modal AND dimmed.
const TRANSPARENT = /^(rgba\(0, 0, 0, 0\)|transparent)$/;

test.describe("Scrim", () => {
  test("a modal popover dims nothing by default (overlay defaults to false) but is still modal", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&`);
    const trigger = page.getByRole("button", { name: "Show Popover", exact: true });
    await expect(trigger).toBeVisible();
    const capture = await captureBackdrop(
      page,
      () => trigger.click(),
      () => page.keyboard.press("Escape"),
    );
    // Not one frame of scrim while opening, open or closing, and no blur either.
    const peak = Math.max(...capture.samples.map((sample) => sample[2]));
    expect(peak, "the ::backdrop must stay fully transparent for the whole open/close cycle").toBeLessThan(0.001);
    expect(capture.settled.backgroundColor).toMatch(TRANSPARENT);
    expect(capture.settled.backdropFilter).toBe("none");
  });

  test("the default carries data-dx-overlay=off on a dialog that is still a real modal", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&`);
    await page.getByRole("button", { name: "Show Popover", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute("data-dx-overlay", "off");
    expect(await dialog.evaluate((el) => (el as HTMLDialogElement).matches(":modal"))).toBe(true);
    await expect(dialog).toHaveAttribute("aria-modal", "true");
  });

  test("overlay: true paints the shared scrim and fades it in and out", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&variant=overlay&`);
    const trigger = page.getByRole("button", { name: "Open with overlay", exact: true });
    await expect(trigger).toBeVisible();
    const capture = await captureBackdrop(
      page,
      () => trigger.click(),
      () => page.keyboard.press("Escape"),
    );
    assertSharedBackdrop(capture, await resolveOverlayMs(page), "popover (modal, overlay: true)");
  });

  test("overlay: true sets nothing on the dialog (the scrim shows by absence of the switch)", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&variant=overlay&`);
    await page.getByRole("button", { name: "Open with overlay", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    expect(await dialog.getAttribute("data-dx-overlay")).toBeNull();
    expect(await dialog.evaluate((el) => (el as HTMLDialogElement).matches(":modal"))).toBe(true);
  });

  test("modal popover content fades out too, for as long as the scrim", async ({ page }) => {
    // The panel's own exit. Escape closes the native dialog at once, and the
    // content has to keep fading (not snap off) while its scrim fades behind it.
    await page.goto(`${BASE_URL}/component/?name=popover&`);
    const trigger = page.getByRole("button", { name: "Show Popover", exact: true });
    const content = page.getByRole("dialog");
    await trigger.click();
    await expect(content).toBeVisible();
    const contentId = await content.getAttribute("id");
    if (!contentId) throw new Error("modal popover content has no id to sample");
    const capture = await sampleCloseFade(page, contentId, () => page.keyboard.press("Escape"));
    assertFadesOutThenUnmounts(capture);
  });

  test("non-modal popover paints no scrim at all (shadcn's popover has no overlay)", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=popover&variant=non_modal&`);
    const trigger = page.getByRole("button", { name: "Open popover", exact: true });
    await expect(trigger).toBeVisible();
    await trigger.evaluate((el) => (el as HTMLElement).click());
    const content = page.getByRole("dialog");
    await expect(content).toBeVisible();
    const backdrop = await content.evaluate((el) => {
      const cs = getComputedStyle(el, "::backdrop");
      return { background: cs.backgroundColor, filter: cs.backdropFilter };
    });
    expect(backdrop.background).toMatch(TRANSPARENT);
    expect(backdrop.filter).toBe("none");
  });
});
