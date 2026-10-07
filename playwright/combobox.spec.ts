import { test, expect, devices } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=combobox&`;
const variantUrl = (variant: string) =>
    `${BASE_URL}/component/?name=combobox&variant=${variant}&`;

const input = (page: Page) =>
    page.getByRole("combobox", { name: "Select framework" });

const content = (page: Page) =>
    page.locator("[role='listbox'][data-state='open']");

const list = (page: Page) =>
    page.locator("[role='listbox'][data-state='open']");

test("opens from the focused input with the keyboard", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await expect(trigger).toBeVisible();
    await trigger.focus();
    await expect(trigger).toBeFocused();

    await page.keyboard.press("ArrowDown");
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");
    await expect(list(page).getByRole("option", { name: "Next.js" })).toHaveAttribute(
        "data-highlighted",
        "true",
    );
});

test("filters and selects with the keyboard", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await expect(content(page)).toBeVisible();

    await page.keyboard.type("sve");
    const svelte = list(page).getByRole("option", { name: "SvelteKit" });
    await expect(svelte).toBeVisible();

    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("Enter");
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("SvelteKit");

    await trigger.click();
    await expect(svelte).toHaveAttribute("aria-selected", "true");

    await page.keyboard.press("Escape");
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("SvelteKit");
});

test("shows an empty state when no options match", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await page.keyboard.type("zzz");

    await expect(list(page).getByText("No framework found.")).toBeVisible();
    await expect(list(page).getByRole("option")).toHaveCount(0);
});

test("arrow keys stay on visible filtered options", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await page.keyboard.type("sve");
    await expect(trigger).toBeFocused();

    const svelte = list(page).getByRole("option", { name: "SvelteKit" });
    await expect(svelte).toBeVisible();
    await expect(svelte).not.toHaveAttribute("tabindex", /.+/);
    await expect(list(page)).not.toHaveAttribute("tabindex", /.+/);

    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");
    await expect(trigger).toBeFocused();
    await expect(trigger).toHaveAttribute("aria-activedescendant", await svelte.getAttribute("id"));

    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("ArrowUp");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");
});

test("keeps filtered options in source order", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await page.keyboard.type("s");

    const next = list(page).getByRole("option", { name: "Next.js" });
    const svelte = list(page).getByRole("option", { name: "SvelteKit" });
    const solid = list(page).getByRole("option", { name: "SolidStart" });

    await expect(next).toBeVisible();
    await expect(svelte).toBeVisible();
    await expect(solid).toBeVisible();

    const nextBox = await next.boundingBox();
    const svelteBox = await svelte.boundingBox();
    expect(nextBox).not.toBeNull();
    expect(svelteBox).not.toBeNull();
    expect(nextBox!.y).toBeLessThan(svelteBox!.y);

    await page.keyboard.press("ArrowDown");
    await expect(next).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("ArrowUp");
    await expect(next).toHaveAttribute("data-highlighted", "true");
});

test("keeps filtered options during keyboard close animation", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await page.keyboard.type("sve");

    const svelte = list(page).getByRole("option", { name: "SvelteKit" });
    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("Enter");

    const closingContent = page.locator("[role='listbox'][data-state='closed']");
    await expect(closingContent).toBeVisible();
    await expect(closingContent.getByRole("option", { name: "SvelteKit" })).toBeVisible();
    await expect(closingContent.getByRole("option")).toHaveCount(1);
    await expect(content(page)).toHaveCount(0);
});

test("clicking an option commits and closes", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await list(page).getByRole("option", { name: "Dioxus" }).click();

    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("Dioxus");
});

test("tabbing away closes the list", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = input(page);
    await trigger.click();
    await expect(content(page)).toBeVisible();

    await page.keyboard.press("Tab");
    await expect(content(page)).toHaveCount(0);
});

test("disabled options are exposed but skipped by keyboard selection", async ({ page }) => {
    await page.goto(variantUrl("disabled"), { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    await expect(page.getByRole("combobox", { name: "Disabled combobox" })).toBeDisabled();

    const trigger = page.getByRole("combobox", {
        name: "Framework with disabled option",
    });
    await trigger.click();

    const menu = list(page);
    const next = menu.getByRole("option", { name: "Next.js" });
    const svelte = menu.getByRole("option", { name: "SvelteKit" });
    const nuxt = menu.getByRole("option", { name: "Nuxt.js" });

    await expect(svelte).toHaveAttribute("aria-disabled", "true");

    await page.keyboard.press("ArrowDown");
    await expect(next).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("ArrowDown");
    await expect(svelte).toHaveAttribute("data-highlighted", "false");
    await expect(nuxt).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("ArrowUp");
    await expect(next).toHaveAttribute("data-highlighted", "true");
});

test("controlled value and controlled open stay in sync", async ({ page }) => {
    await page.goto(variantUrl("controlled"), { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = page.getByRole("combobox", { name: "Controlled framework" });
    const storedValue = page.getByTestId("combobox-controlled-value");

    await expect(trigger).toHaveValue("SvelteKit");
    await expect(storedValue).toHaveText("svelte");

    await page.getByRole("button", { name: "Set Astro" }).click();
    await expect(trigger).toHaveValue("Astro");
    await expect(storedValue).toHaveText("astro");

    // exact: true — the default substring match also catches the docs
    // sidebar's "Open navigation" button whenever the stylesheet that hides
    // it at desktop widths has not fully applied yet (the same pre-existing
    // race toast.spec.ts's own "close" lookup already guards against),
    // sending the click to it instead of this demo's "Open" button.
    await page.getByRole("button", { name: "Open", exact: true }).click();
    await expect(content(page)).toBeVisible();

    await list(page).getByRole("option", { name: "Dioxus" }).click();
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("Dioxus");
    await expect(storedValue).toHaveText("dioxus");
});

test("dynamic option removal updates filtering and keyboard selection", async ({ page }) => {
    await page.goto(variantUrl("dynamic"), { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = page.getByRole("combobox", { name: "Dynamic framework" });
    await trigger.click();
    await page.keyboard.type("s");

    await expect(list(page).getByRole("option", { name: "SvelteKit" })).toBeVisible();
    await expect(list(page).getByRole("option", { name: "SolidStart" })).toBeVisible();
    await page.keyboard.press("ArrowDown");
    await expect(list(page).getByRole("option", { name: "Next.js" })).toHaveAttribute(
        "data-highlighted",
        "true",
    );
    await page.keyboard.press("ArrowDown");
    await expect(list(page).getByRole("option", { name: "SvelteKit" })).toHaveAttribute(
        "data-highlighted",
        "true",
    );

    await page.getByRole("button", { name: "Toggle SvelteKit" }).click();
    await expect(list(page).getByRole("option", { name: "SvelteKit" })).toHaveCount(0);
    await expect(list(page).getByRole("option", { name: "SolidStart" })).toBeVisible();

    await trigger.click();
    await expect(trigger).toBeFocused();
    await page.keyboard.press("ArrowDown");
    const next = list(page).getByRole("option", { name: "Next.js" });
    await expect(next).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("Enter");
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("Next.js");
});

test("touch selection commits and closes", async ({ browser, browserName }) => {
    test.skip(browserName === "firefox", "Firefox does not support mobile contexts");

    const { defaultBrowserType: _defaultBrowserType, ...iphone } = devices["iPhone 12"];
    const context = await browser.newContext(iphone);
    try {
        const page = await context.newPage();
        await page.goto(URL, { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState('networkidle');

        const trigger = input(page);
        await trigger.tap();
        await list(page).getByRole("option", { name: "Dioxus" }).tap();

        await expect(content(page)).toHaveCount(0);
        await expect(trigger).toHaveValue("Dioxus");
    } finally {
        await context.close();
    }
});

test.describe("Axe automated scan", () => {
    test("loaded (listbox closed) has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto(URL, { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState('networkidle');
        await expectNoAxeViolations(page, "combobox: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });

    test("listbox open has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto(URL, { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState('networkidle');
        const trigger = input(page);
        await trigger.focus();
        await page.keyboard.press("ArrowDown");
        await expect(content(page)).toBeVisible();
        await expectNoAxeViolations(page, "combobox: listbox open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });
});

// ---------------------------------------------------------------------------
// Re-engaging an input that already holds a selection.
//
// Reference behaviour (APG editable combobox, Base UI Combobox, shadcn's
// Combobox which is built on it): the input's text is NEVER cleared or
// rewritten by clicking or focusing it, the caret lands where the user
// clicked, the popup lists every option (the filter ignores the selected
// label until the first edit), and the selected option is marked and
// scrolled into view. Escape / blur without a new pick restores the label.
// ---------------------------------------------------------------------------

/** The x offset (relative to the input's padding box) of the boundary before `index` in its text. */
async function caretOffsetX(trigger: Locator, index: number): Promise<number> {
    return trigger.evaluate((el: HTMLInputElement, at: number) => {
        const cs = getComputedStyle(el);
        const ctx = document.createElement("canvas").getContext("2d")!;
        ctx.font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
        return parseFloat(cs.paddingLeft) + ctx.measureText(el.value.slice(0, at)).width - el.scrollLeft;
    }, index);
}

const caret = (trigger: Locator) =>
    trigger.evaluate((el: HTMLInputElement) => [el.selectionStart, el.selectionEnd]);

/** Select "SvelteKit" through the list, leaving the input closed and holding its label. */
async function selectSvelte(page: Page) {
    const trigger = input(page);
    await trigger.click();
    await list(page).getByRole("option", { name: "SvelteKit" }).click();
    await expect(content(page)).toHaveCount(0);
    await expect(page.locator("[role='listbox']")).toHaveCount(0);
    await expect(trigger).toHaveValue("SvelteKit");
    return trigger;
}

test("clicking an input that holds a selection keeps its text and puts the caret where clicked", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    // Record every write to the input's value from here on: a re-render that flashes the text empty
    // (even for one frame) throws the caret to the end, so there must be none at all.
    await trigger.evaluate((el: HTMLInputElement) => {
        const writes: string[] = [];
        (window as any).__comboboxValueWrites = writes;
        const own = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!;
        Object.defineProperty(el, "value", {
            configurable: true,
            get() { return own.get!.call(this); },
            set(next: string) { writes.push(next); own.set!.call(this, next); },
        });
    });

    // Click on the boundary between "Svel" and "teKit".
    const box = (await trigger.boundingBox())!;
    await trigger.click({ position: { x: await caretOffsetX(trigger, 4), y: box.height / 2 } });
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    // The listbox's open animation has settled: nothing below may flip afterwards.
    await expect(list(page)).toHaveCSS("opacity", "1");
    await expect(trigger).toHaveValue("SvelteKit");
    expect(await caret(trigger)).toEqual([4, 4]);

    // Every option is listed, not just the one matching the label...
    await expect(list(page).getByRole("option")).toHaveCount(7);
    // ...and the selected one is marked (semantics and the check indicator).
    const svelte = list(page).getByRole("option", { name: "SvelteKit" });
    await expect(svelte).toHaveAttribute("aria-selected", "true");
    await expect(svelte).toHaveAttribute("data-selected", "true");
    await expect(svelte.locator(".dx-combobox-option-indicator")).toBeVisible();
    await expect(list(page).locator("[aria-selected='true']")).toHaveCount(1);

    // The text and the caret survive any follow-up render too, and the value was never rewritten.
    await page.waitForTimeout(300);
    await expect(trigger).toHaveValue("SvelteKit");
    expect(await caret(trigger)).toEqual([4, 4]);
    expect(await page.evaluate(() => (window as any).__comboboxValueWrites)).toEqual([]);
});

test("focusing an input that holds a selection and pressing ArrowDown keeps its text", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    await page.keyboard.press("ArrowDown");
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveValue("SvelteKit");
    await expect(list(page).getByRole("option")).toHaveCount(7);
    await expect(list(page).getByRole("option", { name: "SvelteKit" })).toHaveAttribute("aria-selected", "true");
});

test("filtering starts on the first edit, not on open", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    await trigger.click();
    await expect(list(page).getByRole("option")).toHaveCount(7);

    // First edit: drop the last letter. The text is now "SvelteKi", which only SvelteKit matches.
    await page.keyboard.press("End");
    await page.keyboard.press("Backspace");
    await expect(trigger).toHaveValue("SvelteKi");
    await expect(list(page).getByRole("option")).toHaveCount(1);
    await expect(list(page).getByRole("option", { name: "SvelteKit" })).toBeVisible();

    // Replace everything: filtering follows the whole input text.
    await trigger.press("ControlOrMeta+a");
    await page.keyboard.type("sol");
    await expect(trigger).toHaveValue("sol");
    await expect(list(page).getByRole("option")).toHaveCount(1);
    await expect(list(page).getByRole("option", { name: "SolidStart" })).toBeVisible();
});

test("typing into a closed input that holds a selection edits the text in place", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    // Closed, focused, caret at the end ("SvelteKit|"): a typed character is appended, not swapped for the text.
    await page.keyboard.press("End");
    await page.keyboard.type("x");
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveValue("SvelteKitx");
    expect(await caret(trigger)).toEqual([10, 10]);
    await expect(list(page).getByRole("option")).toHaveCount(0);
    await expect(list(page).getByText("No framework found.")).toBeVisible();
});

test("Escape restores the selected label after editing", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    await trigger.click();
    await trigger.press("ControlOrMeta+a");
    await page.keyboard.type("zzz");
    await expect(trigger).toHaveValue("zzz");
    await expect(list(page).getByText("No framework found.")).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("SvelteKit");
});

test("blurring without a new pick restores the selected label", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = await selectSvelte(page);

    await trigger.click();
    await trigger.press("ControlOrMeta+a");
    await page.keyboard.type("so");
    await expect(trigger).toHaveValue("so");

    await page.keyboard.press("Tab");
    await expect(content(page)).toHaveCount(0);
    await expect(trigger).toHaveValue("SvelteKit");

    // The next open starts from the full list again, with the label intact.
    await trigger.click();
    await expect(trigger).toHaveValue("SvelteKit");
    await expect(list(page).getByRole("option")).toHaveCount(7);
});

test("reopening scrolls the selected option into view", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    // Force the list to overflow: three rows tall, so the last option starts out of sight.
    await page.addStyleTag({ content: ".dx-combobox-list { max-height: 6.5rem !important; }" });

    const trigger = input(page);
    await trigger.click();
    await page.keyboard.press("End"); // open: highlights the last option
    await page.keyboard.press("Enter");
    await expect(trigger).toHaveValue("Dioxus");
    await expect(page.locator("[role='listbox']")).toHaveCount(0);

    await trigger.click();
    await expect(list(page)).toHaveCSS("opacity", "1");
    const dioxus = list(page).getByRole("option", { name: "Dioxus" });
    await expect(dioxus).toHaveAttribute("aria-selected", "true");
    await expect(trigger).toHaveValue("Dioxus");

    const within = async () => {
        const [l, o] = [await list(page).boundingBox(), await dioxus.boundingBox()];
        return !!l && !!o && o.y >= l.y - 1 && o.y + o.height <= l.y + l.height + 1;
    };
    await expect.poll(within).toBe(true);
    expect(await list(page).evaluate((el) => el.scrollTop)).toBeGreaterThan(0);

    // Keyboard navigation keeps the active option in view as well.
    await page.keyboard.press("Home");
    await expect(list(page).getByRole("option", { name: "Next.js" })).toHaveAttribute("data-highlighted", "true");
    await expect.poll(async () => {
        const [l, o] = [await list(page).boundingBox(), await list(page).getByRole("option", { name: "Next.js" }).boundingBox()];
        return !!l && !!o && o.y >= l.y - 1 && o.y + o.height <= l.y + l.height + 1;
    }).toBe(true);
});

test("an externally opened (controlled) combobox keeps the selected label", async ({ page }) => {
    await page.goto(variantUrl("controlled"), { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');

    const trigger = page.getByRole("combobox", { name: "Controlled framework" });
    await expect(trigger).toHaveValue("SvelteKit");
    await page.getByRole("button", { name: "Open", exact: true }).click();
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveValue("SvelteKit");
    await expect(list(page).getByRole("option")).toHaveCount(7);

    // ...also after a typed-and-abandoned round.
    await trigger.click();
    await trigger.press("ControlOrMeta+a");
    await page.keyboard.type("zz");
    await page.keyboard.press("Escape");
    await expect(content(page)).toHaveCount(0);
    await expect(page.locator("[role='listbox']")).toHaveCount(0);
    await page.getByRole("button", { name: "Open", exact: true }).click();
    await expect(content(page)).toBeVisible();
    await expect(trigger).toHaveValue("SvelteKit");
    await expect(list(page).getByRole("option")).toHaveCount(7);
});

// ---------------------------------------------------------------------------
// Keyboard focus ring: the input draws the same ring `Input` does.
// ---------------------------------------------------------------------------

const boxShadowOf = async (locator: Locator) => {
    // Poll past the box-shadow transition: two equal reads in a row.
    let previous = "";
    await expect.poll(async () => {
        const now = await locator.evaluate((el) => getComputedStyle(el).boxShadow);
        const settled = now === previous;
        previous = now;
        return settled;
    }).toBe(true);
    return previous;
};

test("the input shows the same visible focus ring as Input on keyboard focus", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=input&`, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const plain = page.getByRole("textbox", { name: "Enter your name" });
    await plain.focus();
    await expect(plain).toBeFocused();
    const reference = await boxShadowOf(plain);
    expect(reference).not.toBe("none");

    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState('networkidle');
    const trigger = input(page);
    const resting = await trigger.evaluate((el) => getComputedStyle(el).boxShadow);
    await trigger.focus(); // text fields always match :focus-visible
    await expect(trigger).toBeFocused();
    await expect(trigger).toHaveCSS("outline-style", "none");
    const focused = await boxShadowOf(trigger);
    expect(focused).not.toBe("none");
    expect(focused).not.toBe(resting);
    expect(focused).toBe(reference);
});
