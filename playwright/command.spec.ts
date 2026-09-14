/**
 * `Command` (command palette) conformance spec.
 *
 * Oracle choice, per docs/plan.md's definition of done: primarily a
 * smoke + axe suite, the same shape row-27's nine styled-only components
 * use, rather than a bespoke tier-1 APG-cited file. Two reasons, both
 * checked before writing this file rather than assumed:
 *
 * - `Command`'s underlying keyboard/filtering machinery is this repo's own
 *   `Combobox` infrastructure (`primitives/src/command.rs`'s module doc),
 *   and `combobox.spec.ts` -- the oracle for that infrastructure's actual
 *   consumer -- is itself a smoke + axe suite, not a tier-1
 *   `keyboard-matrix.spec.ts`-cited one (grep confirms zero "Combobox" rows
 *   there). There is no existing APG-cited row for this repo's own
 *   `Combobox` to "re-run against Command's inline-list subject" the way
 *   the plan doc's oracle section anticipated -- so this spec follows the
 *   precedent that actually exists (`combobox.spec.ts`'s own structure)
 *   rather than inventing a first tier-1 citation for either component.
 * - `Command`'s list is never a popup (it has no open/closed state of its
 *   own -- see `primitives/src/command.rs`'s module doc), so the APG
 *   "Combobox (Editable, List Autocomplete)" pattern's own keyboard rows
 *   (which are largely about opening/closing the popup) don't map cleanly
 *   onto it either; the closer normative shape is a plain Listbox with a
 *   filter, for which this repo vendors no APG reference page at all
 *   (`playwright/oracle/reference/README.md` -- only menu-button,
 *   combobox-select-only, and radio are vendored).
 *
 * The dialog shell's own behaviour (Escape closes, focus trap, native
 * `<dialog>` semantics) is already covered by
 * `oracle/tier2-html/native-dialog.spec.ts` and `dialog.spec.ts` against
 * `DialogRoot`/`DialogContent`, which `CommandDialog` reuses unmodified
 * (`primitives/src/command.rs`) -- this file re-confirms Escape closes
 * `CommandDialog` specifically (smoke-level, not a re-derivation of that
 * oracle) rather than re-testing the shell's full contract.
 */
import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

const URL = "http://127.0.0.1:8080/component/?name=command&";

const openButton = (page: Page) => page.getByRole("button", { name: "Open Command Palette" });
const dialog = (page: Page) => page.getByRole("dialog");
const input = (page: Page) => page.getByRole("combobox", { name: "Search commands" });
const list = (page: Page) => page.getByRole("listbox", { name: "Commands" });

async function open(page: Page) {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    await page.waitForLoadState("networkidle");
    await openButton(page).click();
    await expect(dialog(page)).toBeVisible();
}

test("opens via CommandDialog with the input focused", async ({ page }) => {
    await open(page);
    // WHATWG's `showModal()` dialog-focusing steps focus the first
    // `autofocus` descendant -- `CommandInput` always carries `autofocus`
    // (primitives/src/command.rs) specifically so this lands here rather
    // than on the dialog element itself.
    await expect(input(page)).toBeFocused();
    await expect(list(page).getByRole("option")).toHaveCount(6);
});

test("types to filter, arrow-navigates, and selects with the keyboard", async ({ page }) => {
    await open(page);

    await page.keyboard.type("new");
    await expect(list(page).getByRole("option", { name: "New File" })).toBeVisible();
    await expect(list(page).getByRole("option", { name: "New Window" })).toBeVisible();
    await expect(list(page).getByRole("option", { name: "Open File..." })).toHaveCount(0);
    await expect(list(page).getByRole("option")).toHaveCount(2);

    await page.keyboard.press("ArrowDown");
    const newFile = list(page).getByRole("option", { name: "New File" });
    await expect(newFile).toHaveAttribute("data-highlighted", "true");
    await expect(input(page)).toHaveAttribute("aria-activedescendant", await newFile.getAttribute("id"));

    await page.keyboard.press("ArrowDown");
    const newWindow = list(page).getByRole("option", { name: "New Window" });
    await expect(newWindow).toHaveAttribute("data-highlighted", "true");

    await page.keyboard.press("Enter");
    // Selecting closes the dialog (the demo's `on_value_change` sets
    // `open` to `false`) and records which command ran.
    await expect(dialog(page)).toHaveCount(0);
    await expect(page.getByTestId("command-last-run")).toHaveText("Ran: new-window");
});

test("shows an empty state when no command matches", async ({ page }) => {
    await open(page);
    await page.keyboard.type("zzz");
    await expect(list(page).getByText("No results found.")).toBeVisible();
    await expect(list(page).getByRole("option")).toHaveCount(0);
});

test("clicking a command commits and closes", async ({ page }) => {
    await open(page);
    await list(page).getByRole("option", { name: "Toggle Sidebar" }).click();
    await expect(dialog(page)).toHaveCount(0);
    await expect(page.getByTestId("command-last-run")).toHaveText("Ran: toggle-sidebar");
});

test("groups are labelled and structured", async ({ page }) => {
    await open(page);
    const fileGroup = page.getByRole("group", { name: "File" });
    await expect(fileGroup).toBeVisible();
    await expect(fileGroup.getByRole("option", { name: "New File" })).toBeVisible();

    const viewGroup = page.getByRole("group", { name: "View" });
    await expect(viewGroup).toBeVisible();
    await expect(viewGroup.getByRole("option", { name: "Toggle Terminal" })).toBeVisible();
});

test("shortcut hints are presentation-only", async ({ page }) => {
    await open(page);
    const newFile = list(page).getByRole("option", { name: "New File" });
    await expect(newFile.getByText("N")).toBeVisible();

    // Pressing the displayed shortcut key alone does nothing -- `shortcut`
    // carries no keyboard binding of its own (primitives/src/command.rs's
    // module doc, "Scope"). It types into the filter input like any other
    // key, it does not fire "New File".
    await page.keyboard.press("n");
    await expect(dialog(page)).toBeVisible();
    await expect(input(page)).toHaveValue("n");
});

test("Escape closes CommandDialog", async ({ page }) => {
    await open(page);
    await page.keyboard.press("Escape");
    await expect(dialog(page)).toHaveCount(0);
});

test.describe("Axe automated scan", () => {
    test("loaded (dialog closed) has no automatically detectable a11y issues", async ({ page }) => {
        await page.goto(URL, { timeout: 20 * 60 * 1000 });
        await page.waitForLoadState("networkidle");
        await expectNoAxeViolations(page, "command: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });

    test("dialog open has no automatically detectable a11y issues", async ({ page }) => {
        await open(page);
        await expectNoAxeViolations(page, "command: dialog open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
    });
});
