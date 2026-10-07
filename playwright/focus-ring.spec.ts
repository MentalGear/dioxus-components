import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/*
 * Every text-entry control draws the one keyboard-focus ring `Input` draws:
 * `--dx-ring-field` (`preview/assets/dx-components-theme.css`) -- the
 * ring-coloured 1px edge plus the shared `--dx-ring-focus` halo. Each of these
 * controls resets `outline`, so a rule that forgets the ring leaves keyboard
 * focus invisible -- which is how the combobox input and the textarea shipped,
 * and why the ring is now one token instead of a literal repeated per control.
 *
 * This is the class guard: ONE list of every text-entry control (its page, the
 * element that takes focus, the element that draws the ring), one generated test
 * per entry. A new text-entry control is a new row here. The reference is Input's
 * own computed ring, anchored to the token below, so a retuned token moves every
 * control together and a control that stops using it fails its own row.
 *
 * Not in the list on purpose: the Textarea `outline` variant, which draws its
 * edge as a real border and takes `Button`'s construction (border in the ring
 * colour, halo around it) -- `textarea.spec.ts` asserts that pattern for it.
 */

const NAV = { timeout: 20 * 60 * 1000 };

const page_ = (name: string, variant?: string) =>
    `${BASE_URL}/component/?name=${name}&${variant ? `variant=${variant}&` : ""}`;

/** The computed `box-shadow`, read once it has stopped changing (the ring transitions in). */
async function settledBoxShadow(locator: Locator): Promise<string> {
    let previous = "";
    await expect
        .poll(async () => {
            const now = await locator.evaluate((el) => getComputedStyle(el).boxShadow);
            const settled = now === previous;
            previous = now;
            return settled;
        })
        .toBe(true);
    return previous;
}

/** What `--dx-ring-field` resolves to as a computed `box-shadow`, via a probe element. */
async function ringFieldToken(page: Page): Promise<string> {
    return page.evaluate(() => {
        const probe = document.createElement("i");
        probe.style.boxShadow = "var(--dx-ring-field)";
        document.body.append(probe);
        const value = getComputedStyle(probe).boxShadow;
        probe.remove();
        return value;
    });
}

/** What `Input` draws on keyboard focus: the reference every text field's ring must equal. */
async function inputFocusRing(page: Page): Promise<string> {
    await gotoHydrated(page, page_("input"), NAV);
    const input = page.getByRole("textbox", { name: "Enter your name" });
    await expect(input).toBeVisible();
    await input.focus();
    await expect(input).toBeFocused();
    const ring = await settledBoxShadow(input);
    expect(ring).not.toBe("none");
    return ring;
}

type Field = {
    /** Test title: the control, in words. */
    name: string;
    url: string;
    /** Get the page into the state where the control is on screen (e.g. open a dialog). */
    open?: (page: Page) => Promise<void>;
    /** The element that takes keyboard focus. */
    focus: (page: Page) => Locator;
    /** The element that draws the ring; the focused element itself when omitted. */
    ring?: (page: Page) => Locator;
    /** The element whose resting `box-shadow` the ring must differ from; `ring` when omitted. */
    resting?: (page: Page) => Locator;
    /** The focused control is stripped bare and the ring owner draws the one ring. */
    ringIsOnAnotherElement?: boolean;
};

const FIELDS: Field[] = [
    {
        name: "Input",
        url: page_("input"),
        focus: (page) => page.getByRole("textbox", { name: "Enter your name" }),
    },
    {
        name: "Textarea (default variant)",
        url: page_("textarea", "main"),
        focus: (page) => page.locator("#default"),
    },
    {
        name: "Textarea (fade variant)",
        url: page_("textarea", "fade"),
        focus: (page) => page.locator("#fade"),
    },
    {
        name: "Textarea (ghost variant)",
        url: page_("textarea", "ghost"),
        focus: (page) => page.locator("#ghost"),
    },
    {
        name: "Select trigger",
        url: page_("select"),
        focus: (page) => page.getByRole("combobox").filter({ hasText: /Select an option|Apple|Banana/ }),
    },
    {
        name: "Native Select",
        url: page_("native_select"),
        focus: (page) => page.locator("#component-preview-frame .dx-native-select").first(),
    },
    {
        name: "Combobox input",
        url: page_("combobox"),
        focus: (page) => page.getByRole("combobox", { name: "Select framework" }),
    },
    {
        // The wrapper draws the field chrome; the inner control is stripped bare.
        name: "Input Group (ring on the group)",
        url: page_("input_group"),
        focus: (page) => page.getByPlaceholder("Search..."),
        ring: (page) => page.locator(".dx-input-group", { has: page.getByPlaceholder("Search...") }),
        ringIsOnAnotherElement: true,
    },
    {
        // The real input is invisible; the active slot is where focus shows.
        name: "Input OTP (ring on the active slot)",
        url: page_("input_otp"),
        focus: (page) => page.locator("#otp-main"),
        ring: (page) => page.locator("#otp-main + div .dx-input-otp-slot[data-active='true']"),
        resting: (page) => page.locator("#otp-main + div .dx-input-otp-slot").first(),
        ringIsOnAnotherElement: true,
    },
    {
        // The input sits borderless in a bottom-ruled strip and draws the ring on its own box. It takes
        // focus when the palette opens, which the test blurs before reading the resting ring.
        name: "Command palette search input",
        url: page_("command"),
        open: async (page) => {
            await page.getByRole("button", { name: "Open Command Palette" }).first().click();
            await expect(page.getByRole("dialog")).toBeVisible();
        },
        focus: (page) => page.getByRole("combobox", { name: "Search commands" }),
    },
    {
        // The segments are the focus targets; the group is the field the ring belongs on.
        name: "Date picker (ring on the group)",
        url: page_("date_picker"),
        focus: (page) => page.locator("#component-preview-frame").getByRole("spinbutton", { name: "month" }).first(),
        ring: (page) => page.locator("#component-preview-frame .dx-date-picker-group").first(),
        ringIsOnAnotherElement: true,
    },
    {
        name: "Date picker trigger",
        url: page_("date_picker"),
        focus: (page) => page.locator("#component-preview-frame").getByRole("button", { name: "Show Calendar" }).first(),
    },
];

test("Input's focus ring is the --dx-ring-field token", async ({ page }) => {
    const reference = await inputFocusRing(page);
    expect(reference).toBe(await ringFieldToken(page));
});

for (const field of FIELDS) {
    const body = async ({ page }: { page: Page }) => {
        const reference = await inputFocusRing(page);

        await gotoHydrated(page, field.url, NAV);
        await field.open?.(page);
        const focus = field.focus(page);
        await expect(focus).toBeVisible();
        // A control that takes focus on mount (the command palette's input) is read unfocused first.
        await focus.evaluate((el) => (el as HTMLElement).blur());
        await expect(focus).not.toBeFocused();
        const resting = await settledBoxShadow((field.resting ?? field.ring ?? field.focus)(page));

        await focus.focus();
        await expect(focus).toBeFocused();
        // Keyboard focus: a text field always matches `:focus-visible`; a button focused by script does when
        // nothing was pointed at first. Either way this is the state the ring is for.
        expect(await focus.evaluate((el) => el.matches(":focus-visible"))).toBe(true);

        const focused = await settledBoxShadow((field.ring ?? field.focus)(page));
        expect(focused).not.toBe("none");
        expect(focused).not.toBe(resting);
        expect(focused).toBe(reference);

        if (field.ringIsOnAnotherElement) {
            // The stripped control itself draws nothing: the ring owner draws the one ring.
            await expect(focus).toHaveCSS("box-shadow", "none");
        }
    };

    test(`${field.name} draws Input's keyboard-focus ring`, body);
}
