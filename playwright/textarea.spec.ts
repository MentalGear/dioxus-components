import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const url = (variant: string) => `${BASE_URL}/component/?name=textarea&variant=${variant}&`;

// id -> variant URL, per `preview/src/components/textarea/variants/*`.
const VARIANTS = [
    { variant: "main", id: "default" },
    { variant: "fade", id: "fade" },
    { variant: "outline", id: "outline" },
    { variant: "ghost", id: "ghost" },
];

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

/** What `Input` draws on keyboard focus: the reference every text field's ring is built to match. */
async function inputFocusRing(page: Page): Promise<string> {
    await page.goto(`${BASE_URL}/component/?name=input&`, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState("networkidle");
    const input = page.getByRole("textbox", { name: "Enter your name" });
    await input.focus();
    await expect(input).toBeFocused();
    const ring = await settledBoxShadow(input);
    expect(ring).not.toBe("none");
    return ring;
}

test("typing updates the bound value", async ({ page }) => {
    await page.goto(url("main"), { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState("networkidle");
    await page.locator("#default").fill("hello");
    await expect(page.locator("#textarea-message").first()).toContainText("Description here: hello");
});

for (const { variant, id } of VARIANTS) {
    test(`the ${variant} variant shows a visible focus ring on keyboard focus, built like Input's`, async ({ page }) => {
        const reference = await inputFocusRing(page);

        await page.goto(url(variant), { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState("networkidle");
        const textarea = page.locator(`#${id}`);
        await expect(textarea).toBeVisible();
        const resting = await settledBoxShadow(textarea);

        await textarea.focus();
        await expect(textarea).toBeFocused();
        // The component resets `outline`: the ring is the only focus affordance.
        await expect(textarea).toHaveCSS("outline-style", "none");
        const focused = await settledBoxShadow(textarea);
        expect(focused).not.toBe("none");
        expect(focused).not.toBe(resting);

        if (variant === "outline") {
            // An edge drawn as a real border: the border takes the ring colour and the halo goes around it
            // (the construction `Button` uses) -- the halo is the outer layer of Input's ring.
            expect(reference).toContain(focused);
            const [border, ringEdge] = await Promise.all([
                textarea.evaluate((el) => getComputedStyle(el).borderTopColor),
                page.evaluate(() => {
                    const probe = document.createElement("i");
                    probe.style.color = "var(--dx-ring-color)";
                    document.body.append(probe);
                    const color = getComputedStyle(probe).color;
                    probe.remove();
                    return color;
                }),
            ]);
            expect(border).toBe(ringEdge);
        } else {
            expect(focused).toBe(reference);
        }

        await page.keyboard.press("Tab");
        await expect(textarea).not.toBeFocused();
        expect(await settledBoxShadow(textarea)).toBe(resting);
    });
}

test.describe("Axe automated scan", () => {
    test("loaded has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto(url("main"), { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState("networkidle");
        await expect(page.locator("#default")).toBeVisible();
        await expectNoAxeViolations(page, "textarea: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });

    test("focused has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto(url("main"), { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState("networkidle");
        const textarea = page.locator("#default");
        await textarea.focus();
        await expect(textarea).toBeFocused();
        await expectNoAxeViolations(page, "textarea: focused", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });
});
