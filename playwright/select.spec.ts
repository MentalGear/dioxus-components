import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

const singleSelectTrigger = (page: Page) =>
    page.getByRole("button").filter({ hasText: /Select an option|Apple|Banana/ });

const multiSelectTrigger = (page: Page) =>
    page.getByRole("button").filter({ hasText: /Pepperoni|Mushroom|Onion/ });

test("test", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&", {
        timeout: 20 * 60 * 1000,
        waitUntil: 'networkidle'
    }); // Increase timeout to 20 minutes
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();
    // Assert the select menu is open
    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    // Assert the menu is focused
    await expect(selectMenu).toBeFocused();
    await page.keyboard.press("ArrowDown");
    const firstOption = selectMenu.getByRole("option", { name: "apple" });
    await expect(firstOption).toBeFocused();

    // Assert moving down with arrow keys moves focus to the next option
    await page.keyboard.press("ArrowDown");
    const secondOption = selectMenu.getByRole("option", { name: "banana" });
    await expect(secondOption).toBeFocused();

    // Assert moving up with arrow keys moves focus back to the previous option
    await page.keyboard.press("ArrowUp");
    await expect(firstOption).toBeFocused();

    // Assert pressing Enter selects the focused option
    await page.keyboard.press("Enter");
    // Assert the select menu is closed after selection
    await expect(selectMenu).toHaveCount(0);

    // Assert the selected value is displayed in the button
    await expect(selectTrigger).toHaveText("Apple");

    // Reopen the select menu
    await selectTrigger.click();

    // Assert typeahead functionality works
    await page.keyboard.type("Ban");
    // Assert the second option is focused after typing 'Ban'
    await expect(secondOption).toBeFocused();

    // Assert pressing Escape closes the select menu
    await page.keyboard.press("Escape");
    // Assert the select menu is closed
    await expect(selectMenu).toHaveCount(0);

    // Reopen the select menu
    await selectTrigger.click();
    // Assert the select menu is open again
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    // Click the second option to select it
    let bananaOption = selectMenu.getByRole("option", { name: "banana" });
    await bananaOption.click();
    // Assert the select menu is closed after clicking an option
    await expect(selectMenu).toHaveCount(0);
    // Assert the selected value is now 'banana'
    await expect(selectTrigger).toHaveText("Banana");
});

test("tabbing out of menu closes the select menu", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();
    // Assert the select menu is open
    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    // Assert the menu is focused
    await expect(selectMenu).toBeFocused();
    await page.keyboard.press("Tab");
    // Assert the select menu is closed
    await expect(selectMenu).toHaveCount(0);
});

test("multi-select toggles options and stays open", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&variant=multi&", {
        timeout: 20 * 60 * 1000,
    });
    const selectTrigger = multiSelectTrigger(page);
    // Default values from the demo: Pepperoni and Mushroom
    await expect(selectTrigger).toContainText("Pepperoni");
    await expect(selectTrigger).toContainText("Mushroom");

    await selectTrigger.click();
    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    const pepperoni = selectMenu.getByRole("option", { name: "Pepperoni" });
    const onion = selectMenu.getByRole("option", { name: "Onion" });

    await expect(pepperoni).toHaveAttribute("aria-selected", "true");
    await expect(onion).toHaveAttribute("aria-selected", "false");

    // Click an unselected option — it should toggle on without closing
    await onion.click();
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(onion).toHaveAttribute("aria-selected", "true");

    // Click an already-selected option — it should toggle off without closing
    await pepperoni.click();
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(pepperoni).toHaveAttribute("aria-selected", "false");

    // Escape closes the menu without selecting
    await page.keyboard.press("Escape");
    await expect(selectMenu).toHaveCount(0);
    // Trigger reflects the updated multi-selection
    await expect(selectTrigger).toContainText("Mushroom");
    await expect(selectTrigger).toContainText("Onion");
    await expect(selectTrigger).not.toContainText("Pepperoni");
});

test("mobile: multi-select tapping options keeps the dropdown open", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&variant=multi&", {
        timeout: 20 * 60 * 1000,
    });
    const selectTrigger = multiSelectTrigger(page);
    await selectTrigger.tap();

    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    const onion = selectMenu.getByRole("option", { name: "Onion" });
    await expect(onion).toHaveAttribute("aria-selected", "false");

    // Tapping the first option on mobile should toggle it without closing the menu
    await onion.tap();
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(onion).toHaveAttribute("aria-selected", "true");

    // Tapping a second option should also leave the dropdown open
    const pepperoni = selectMenu.getByRole("option", { name: "Pepperoni" });
    await pepperoni.tap();
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(pepperoni).toHaveAttribute("aria-selected", "false");
});

test("multi-select keyboard toggles and exposes aria-multiselectable", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&variant=multi&", {
        timeout: 20 * 60 * 1000,
    });
    const selectTrigger = multiSelectTrigger(page);
    await selectTrigger.click();

    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    // Listbox advertises multi-select mode for assistive tech
    await expect(selectMenu).toHaveAttribute("aria-multiselectable", "true");

    // Arrow down to focus the first option (Pepperoni — already selected by default)
    await page.keyboard.press("ArrowDown");
    const pepperoni = selectMenu.getByRole("option", { name: "Pepperoni" });
    await expect(pepperoni).toBeFocused();

    // Space toggles the focused option off without closing
    await page.keyboard.press(" ");
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(pepperoni).toHaveAttribute("aria-selected", "false");

    // Arrow down to Onion and toggle on with Enter — menu still open in multi-mode
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    const onion = selectMenu.getByRole("option", { name: "Onion" });
    await expect(onion).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(selectMenu).toHaveAttribute("data-state", "open");
    await expect(onion).toHaveAttribute("aria-selected", "true");

    await page.keyboard.press("Escape");
    await expect(selectMenu).toHaveCount(0);
    await expect(selectTrigger).toContainText("Mushroom");
    await expect(selectTrigger).toContainText("Onion");
    await expect(selectTrigger).not.toContainText("Pepperoni");
});

test("tabbing out of item closes the select menu", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();
    // Assert the select menu is open
    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    // Assert the menu is focused
    await expect(selectMenu).toBeFocused();

    // Navigate to the first option
    await page.keyboard.press("ArrowDown");
    const firstOption = selectMenu.getByRole("option", { name: "apple" });
    await expect(firstOption).toBeFocused();
    await page.keyboard.press("Tab");
    // Assert the select menu is closed
    await expect(selectMenu).toHaveCount(0);
});

test("options selected", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();
    // Assert the select menu is open
    const selectMenu = page.getByRole("listbox");
    await expect(selectMenu).toHaveAttribute("data-state", "open");

    // Assert no items have aria-selected
    const options = selectMenu.getByRole("option");
    let optionCount = await options.count();
    for (let i = 0; i < optionCount; i++) {
        await expect(options.nth(i)).not.toHaveAttribute("aria-selected", "true");
    }

    // Select the first option
    await page.keyboard.press("ArrowDown");
    const firstOption = selectMenu.getByRole("option", { name: "apple" });
    await expect(firstOption).toBeFocused();
    await page.keyboard.press("Enter");
    // Assert the select menu is closed after selection
    await expect(selectMenu).toHaveCount(0);
    // Open the select menu again
    await selectTrigger.click();
    // Assert the first option is now selected
    await expect(firstOption).toHaveAttribute("aria-selected", "true");
});

test("down arrow selects first element", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    const selectMenu = page.getByRole("listbox");
    await selectTrigger.focus();

    // Select the first option
    await page.keyboard.press("ArrowDown");
    const firstOption = selectMenu.getByRole("option", { name: "apple" });
    await expect(firstOption).toBeFocused();
});

test("up arrow selects last element", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    // Find Select a fruit...
    let selectTrigger = singleSelectTrigger(page);
    const selectMenu = page.getByRole("listbox");
    await selectTrigger.focus();

    // Select the first option
    await page.keyboard.press("ArrowUp");
    const firstOption = selectMenu.getByRole("option", { name: "other" });
    await expect(firstOption).toBeFocused();
});

test("keyboard navigation skips disabled options", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    const selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();

    const selectMenu = page.getByRole("listbox");
    const orange = selectMenu.getByRole("option").filter({ hasText: "Orange" }).first();
    const orangeade = selectMenu.getByRole("option").filter({ hasText: "Orangeade" });
    await expect(orange).toHaveAttribute("aria-disabled", "true");

    await page.keyboard.press("ArrowDown");
    await expect(selectMenu.getByRole("option", { name: "apple" })).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(selectMenu.getByRole("option", { name: "banana" })).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(orangeade).toBeFocused();
    await expect(orange).not.toBeFocused();
});

test("typeahead skips disabled options", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=select&");
    const selectTrigger = singleSelectTrigger(page);
    await selectTrigger.click();

    const selectMenu = page.getByRole("listbox");
    const orange = selectMenu.getByRole("option").filter({ hasText: "Orange" }).first();
    const orangeade = selectMenu.getByRole("option").filter({ hasText: "Orangeade" });
    await expect(orange).toHaveAttribute("aria-disabled", "true");

    await page.keyboard.type("Ora");
    await expect(orangeade).toBeFocused();
    await expect(orange).not.toBeFocused();
});

/**
 * Listbox width (docs/backlog.md row 47, 2026-09-04) -- regression guard for
 * the reported symptom: opening the styled `Select` on the home page
 * gallery rendered its listbox the full width of the viewport (1265px on a
 * 1280px-wide viewport) instead of the width of its own trigger. Root
 * cause: `.dx-select-list`'s `min-width: 100%` resolved against the
 * *viewport* once this listbox was promoted to the top layer (its
 * containing block is no longer `.dx-select`'s `position: relative` box),
 * and the `@supports (anchor-name: --a) { min-width: anchor-size(width);
 * }` rule meant to restore trigger-relative sizing never matched anything
 * served, because `#[css_module]`'s class-hashing does not scope selectors
 * written only inside an `@supports` body -- see
 * `primitives/src/top_layer.rs`'s `use_anchor_position_fallback` doc,
 * "Anchor width contract", for the full mechanism and the fix
 * (`--dx-anchor-width`, engine-published on every anchored content).
 *
 * Covers both routes the report and the reproduction named: the component
 * page (`/component/?name=select&`) and the home gallery (`/`, where the
 * report actually came from -- the gallery embeds a *different* `Select`
 * instance, "Choose a fruit", alongside its own "Select an option" ones).
 *
 * `oracle/tier2-html/top-layer.spec.ts`'s Rule 13 ("fit-content width") is
 * a *different*, narrower regression guard: it opens every anchored
 * content on that spec's own unstyled fixture page, which carries no
 * `#[css_module]` stylesheet at all -- so it cannot see a themed
 * component's CSS (this bug's actual location) and does not cover this
 * symptom. This test is the real oracle for it.
 */
test.describe("Listbox width (docs/backlog.md row 47 -- full-viewport-width regression)", () => {
    async function assertListboxTracksTriggerWidth(
        page: Page,
        trigger: ReturnType<Page["getByRole"]>,
        listbox: ReturnType<Page["getByRole"]>,
    ) {
        const triggerBox = await trigger.boundingBox();
        const listboxBox = await listbox.boundingBox();
        const viewport = page.viewportSize();
        if (!triggerBox || !listboxBox || !viewport) {
            throw new Error("expected trigger, listbox and viewport to all have a bounding box");
        }
        const debug = `trigger=${JSON.stringify(triggerBox)} listbox=${JSON.stringify(listboxBox)} viewport=${JSON.stringify(viewport)}`;

        // At least as wide as the trigger (the pre-migration contract this
        // fix restores) -- 1px slack for subpixel rounding.
        expect(listboxBox.width, debug).toBeGreaterThanOrEqual(triggerBox.width - 1);
        // Not dramatically wider than the trigger (rules out "grew to fill
        // the viewport" while still tolerating a listbox whose own option
        // text is naturally wider than a narrow trigger).
        expect(listboxBox.width, debug).toBeLessThanOrEqual(Math.max(triggerBox.width * 2, 320));
        // The actual reported symptom: nowhere near viewport width.
        expect(listboxBox.width, debug).toBeLessThan(0.6 * viewport.width);
        // Anchored near the trigger's left edge (side="bottom",
        // align="start" -- `select/components/list.rs`), not drifted to
        // the viewport's own left edge.
        expect(Math.abs(listboxBox.x - triggerBox.x), debug).toBeLessThanOrEqual(8);
    }

    test("component page: listbox tracks trigger width, not viewport width", async ({ page }) => {
        await page.setViewportSize({ width: 1280, height: 800 });
        await page.goto("http://127.0.0.1:8080/component/?name=select&", { waitUntil: 'networkidle' });
        const trigger = singleSelectTrigger(page);
        await trigger.click();
        const listbox = page.getByRole("listbox");
        await expect(listbox).toHaveAttribute("data-state", "open");
        await assertListboxTracksTriggerWidth(page, trigger, listbox);
    });

    test("home gallery: listbox tracks trigger width, not viewport width", async ({ page }) => {
        // The original report: desktop width, home page, the styled Select
        // cards ("Choose a fruit" / "Select an option").
        await page.setViewportSize({ width: 1280, height: 800 });
        await page.goto("http://127.0.0.1:8080/", { waitUntil: 'networkidle' });
        const trigger = page.getByRole("button").filter({ hasText: /^(Select an option|Choose a fruit)$/ }).first();
        await trigger.click();
        const listbox = page.getByRole("listbox");
        await expect(listbox).toHaveAttribute("data-state", "open");
        await assertListboxTracksTriggerWidth(page, trigger, listbox);
    });
});

test.describe("Axe automated scan", () => {
    test("loaded (listbox closed) has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto("http://127.0.0.1:8080/component/?name=select&", { waitUntil: 'networkidle' });
        await expectNoAxeViolations(page, "select: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });

    test("listbox open has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto("http://127.0.0.1:8080/component/?name=select&", { waitUntil: 'networkidle' });
        await singleSelectTrigger(page).click();
        await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");
        await expectNoAxeViolations(page, "select: listbox open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });
});
