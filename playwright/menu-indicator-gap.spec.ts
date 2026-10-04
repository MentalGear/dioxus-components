import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

/**
 * Menu-item indicator slot -- the class guard.
 *
 * The owner reported the same defect twice ("Select: check icon too close",
 * "Dropdown Menu: there needs to be min margin between the check icon and the
 * label"): a menu-like item draws an indicator (check / radio dot / chevron /
 * shortcut) next to its label and reserves no room for it, so a label as wide
 * as the row meets the icon. That is a class, not two coincidences, so the fix
 * is one construction applied to every menu-like stylesheet (select, combobox,
 * native_select, dropdown_menu, context_menu, menubar, command): the item
 * reserves `inset + size + gap` of inline padding and the indicator is
 * absolutely positioned inside that reserved room -- shadcn's `pr-8` +
 * `absolute right-2 size-4` (shadcn-ui/ui@295a1f1, style-nova.css
 * `.cn-select-item`). The three tokens are
 * `--dx-menu-item-indicator-{size,inset,gap}`; `gap` is the guaranteed clear
 * space between the label's text box and the indicator's box.
 *
 * Checkable menu items (`*CheckboxItem` / `*RadioItem`, drawing a check) are
 * the same construction. Which side the slot is on follows shadcn Nova
 * (shadcn-ui/ui@295a1f1, style-nova.css): the dropdown and context menus
 * draw the check at the inline END (`pr-8`, `right-2`), exactly like the
 * sub-trigger's chevron; the menubar draws it at the inline START (`pl-7`,
 * `left-1.5`). An unchecked item keeps its slot (the check is only
 * `visibility: hidden`), so a label never shifts when an item is toggled and
 * a menu that mixes plain and checkable items keeps its labels on one edge.
 *
 * What this file asserts, per component, is the RENDERED geometry, not the
 * CSS text: the horizontal distance between the label's text box (a Range
 * over the item's own text nodes, so a wrapped label counts by its widest
 * line) and the indicator's box is >= MIN_GAP, both for the demo's natural
 * label and for a deliberately hostile one (long, and a 40-character word
 * with no break opportunity) at a 320px viewport. Direction-aware: the gap is
 * measured on whichever side the indicator actually sits, so the RTL select
 * variant is covered by the same assertion.
 *
 * MIN_GAP is 8px: shadcn's own clear space (item `pr-8` = 32px, minus the
 * indicator's `right-2` 8px inset and `size-4` 16px). Before this construction
 * the select option measured 6px, the combobox input and native select 4px.
 */
const MIN_GAP = 8;

const LONG_LABEL = "Extraordinarily long label text that has to wrap onto several lines";
const UNBROKEN_LABEL = "Supercalifragilisticexpialidocious_label_no_breaks";

type GapMeasure = {
    itemLeft: number;
    itemRight: number;
    labelLeft: number;
    labelRight: number;
    indicatorLeft: number;
    indicatorRight: number;
    gap: number;
};

/**
 * Wait for every finite animation on the page to finish. A menu popup opens
 * with a `scale(0.95)` -> `scale(1)` entry animation, and a box measured
 * mid-way is 5% smaller than it will be: a gap that is exactly 8px at rest
 * reads 7.6px, and two items measured a few frames apart disagree by the
 * animation's progress. Measure only a settled menu.
 */
async function settleAnimations(page: Page): Promise<void> {
    await page.evaluate(() =>
        Promise.all(
            document
                .getAnimations()
                .filter((a) => Number.isFinite(a.effect?.getComputedTiming().endTime as number))
                .map((a) => a.finished.catch(() => undefined)),
        ),
    );
}

/**
 * The horizontal gap between `item`'s label text and its indicator. The
 * indicator is the first element matching `indicatorSelector` inside `item`;
 * the label is every non-blank text node inside `item` that is not inside the
 * indicator (so a trailing shortcut, which carries text, counts as the
 * indicator when it is the one passed).
 */
async function labelIndicatorGap(item: Locator, indicatorSelector: string): Promise<GapMeasure> {
    await settleAnimations(item.page());
    return item.evaluate((el, selector) => {
        const indicator = el.querySelector(selector);
        if (!indicator) throw new Error(`no ${selector} inside the item`);
        let left = Infinity;
        let right = -Infinity;
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        for (let node = walker.nextNode(); node; node = walker.nextNode()) {
            if (!node.textContent || !node.textContent.trim()) continue;
            if (indicator.contains(node)) continue;
            const range = document.createRange();
            range.selectNodeContents(node);
            for (const rect of Array.from(range.getClientRects())) {
                left = Math.min(left, rect.left);
                right = Math.max(right, rect.right);
            }
        }
        if (!Number.isFinite(left)) throw new Error("the item has no label text");
        const box = indicator.getBoundingClientRect();
        const itemBox = el.getBoundingClientRect();
        return {
            itemLeft: itemBox.left,
            itemRight: itemBox.right,
            labelLeft: left,
            labelRight: right,
            indicatorLeft: box.left,
            indicatorRight: box.right,
            // Whichever side the indicator is on: LTR puts it after the label,
            // RTL before it. A negative number is an overlap.
            gap: Math.max(box.left - right, left - box.right),
        };
    }, indicatorSelector);
}

/**
 * The gap alone is not enough: a label that overflows its row can push an
 * in-flow indicator clean out of the row (gap preserved, indicator gone), or
 * overflow the far edge instead. Both indicator and label must stay inside
 * the row.
 */
function expectInsideRow(m: GapMeasure, what: string) {
    const slack = 0.5;
    const msg = `${what}: ${JSON.stringify(m)}`;
    expect(m.indicatorLeft, `indicator left of row, ${msg}`).toBeGreaterThanOrEqual(m.itemLeft - slack);
    expect(m.indicatorRight, `indicator right of row, ${msg}`).toBeLessThanOrEqual(m.itemRight + slack);
    expect(m.labelLeft, `label left of row, ${msg}`).toBeGreaterThanOrEqual(m.itemLeft - slack);
    expect(m.labelRight, `label right of row, ${msg}`).toBeLessThanOrEqual(m.itemRight + slack);
}

/** Replace the item's first non-blank text node (the label) with `text`. */
async function setLabel(item: Locator, text: string): Promise<void> {
    await item.evaluate((el, next) => {
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        for (let node = walker.nextNode(); node; node = walker.nextNode()) {
            if (node.textContent && node.textContent.trim()) {
                node.textContent = next;
                return;
            }
        }
        throw new Error("the item has no label text to replace");
    }, text);
}

/**
 * Assert the gap for the item as rendered, then for each hostile label.
 */
async function expectGapHolds(page: Page, found: Locator, indicatorSelector: string, what: string) {
    const natural = await labelIndicatorGap(found, indicatorSelector);
    expect(natural.gap, `${what}: natural label, ${JSON.stringify(natural)}`).toBeGreaterThanOrEqual(MIN_GAP);
    expectInsideRow(natural, `${what}: natural label`);

    // Swapping the label changes the item's accessible name, so the caller's
    // name-based locator would stop matching: pin the element by a probe
    // attribute instead.
    await found.evaluate((el) => el.setAttribute("data-gap-probe", ""));
    const item = page.locator("[data-gap-probe]");
    for (const label of [LONG_LABEL, UNBROKEN_LABEL]) {
        await setLabel(item, label);
        // Let the popup re-measure after the text swap.
        await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
        const hostile = await labelIndicatorGap(item, indicatorSelector);
        expect(hostile.gap, `${what}: label "${label}", ${JSON.stringify(hostile)}`).toBeGreaterThanOrEqual(MIN_GAP);
        expectInsideRow(hostile, `${what}: label "${label}"`);
    }
    // So a second item in the same menu can be probed without a strict-mode clash.
    await item.evaluate((el) => el.removeAttribute("data-gap-probe"));
}

/**
 * Which side of its label an item's indicator is on, in the item's own
 * writing direction: "start" is where text begins (left in LTR, right in
 * RTL), "end" the opposite. Measured from the rendered boxes, so it is the
 * truth on screen, not an echo of the CSS.
 */
function indicatorSide(m: GapMeasure, direction: "ltr" | "rtl"): "start" | "end" {
    const onLeft = m.indicatorRight <= m.labelLeft + 0.5;
    const onRight = m.indicatorLeft >= m.labelRight - 0.5;
    expect(onLeft || onRight, `indicator is beside the label: ${JSON.stringify(m)}`).toBe(true);
    const physical = onLeft ? "left" : "right";
    return (physical === "left") === (direction === "ltr") ? "start" : "end";
}

/**
 * One checkable item, the whole contract: its indicator sits on `side`, keeps
 * MIN_GAP clear of the label (natural, long and unbroken labels) inside the
 * row; and the slot is reserved whether or not the item is checked -- the
 * checked and the unchecked item put their labels and their indicator boxes
 * in the same place, and the check is drawn only on the checked one. Items
 * are probed in place (the label swap in `expectGapHolds` changes their
 * accessible names), so this takes locators, not names.
 */
async function expectCheckableSlot(
    page: Page,
    checked: Locator,
    unchecked: Locator,
    direction: "ltr" | "rtl",
    side: "start" | "end",
    what: string,
) {
    const a = await labelIndicatorGap(checked, "svg");
    const b = await labelIndicatorGap(unchecked, "svg");
    expect(indicatorSide(a, direction), `${what}: checked item's indicator side, ${JSON.stringify(a)}`).toBe(side);
    expect(indicatorSide(b, direction), `${what}: unchecked item's indicator side, ${JSON.stringify(b)}`).toBe(side);
    // Reserved slot: same label edge and same indicator box, checked or not.
    // The edge a label starts from is its inline START: left in LTR, right in
    // RTL (labels of different widths only share that one).
    const labelStart = (m: GapMeasure) => (direction === "ltr" ? m.labelLeft : m.labelRight);
    expect(Math.abs(labelStart(a) - labelStart(b)), `${what}: label start edge shifts between checked/unchecked: ${JSON.stringify([a, b])}`).toBeLessThanOrEqual(0.5);
    expect(Math.abs(a.indicatorLeft - b.indicatorLeft), `${what}: indicator box shifts between checked/unchecked: ${JSON.stringify([a, b])}`).toBeLessThanOrEqual(0.5);
    await expect(checked.locator("svg"), `${what}: the checked item draws its check`).toBeVisible();
    await expect(unchecked.locator("svg"), `${what}: the unchecked item draws nothing`).toBeHidden();
    await expectGapHolds(page, checked, "svg", `${what} (checked)`);
    await expectGapHolds(page, unchecked, "svg", `${what} (unchecked)`);
}

const block = (name: string, variant = "main") =>
    `${BASE_URL}/component/block/?name=${name}&variant=${variant}&`;

test.describe("Menu-item indicator slot: label never meets the indicator", () => {
    test.beforeEach(async ({ page }) => {
        // Narrow on purpose: this is where a popup is clamped to the viewport
        // and a long label has to wrap instead of growing the popup.
        await page.setViewportSize({ width: 320, height: 800 });
    });

    test("select: the selected option's check keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("select"));
        const trigger = page.getByRole("combobox").first();
        await trigger.click();
        const listbox = page.getByRole("listbox");
        await expect(listbox).toHaveAttribute("data-state", "open");
        // The owner's screenshot: the longest option, selected.
        await listbox.getByRole("option", { name: "Watermelon" }).click();
        await expect(listbox).toHaveCount(0);
        await trigger.click();
        await expect(listbox).toHaveAttribute("data-state", "open");

        const option = listbox.getByRole("option", { name: "Watermelon" });
        await expectGapHolds(page, option, "svg", "select option check");
    });

    test("select: a long selected value truncates before the trigger's chevron", async ({ page }) => {
        await gotoHydrated(page, block("select"));
        const trigger = page.getByRole("combobox").first();
        await trigger.click();
        await page.getByRole("listbox").getByRole("option", { name: "Watermelon" }).click();
        await expect(page.getByRole("listbox")).toHaveCount(0);

        for (const value of [LONG_LABEL, UNBROKEN_LABEL]) {
            const m = await trigger.evaluate((el, text) => {
                const span = el.querySelector("span[data-placeholder]") as HTMLElement;
                span.textContent = text;
                const icon = el.querySelector("svg") as Element;
                const t = el.getBoundingClientRect();
                const v = span.getBoundingClientRect();
                const i = icon.getBoundingClientRect();
                return {
                    triggerRight: t.right,
                    triggerHeight: t.height,
                    viewport: document.documentElement.clientWidth,
                    valueRight: v.right,
                    iconLeft: i.left,
                    truncated: span.scrollWidth > span.clientWidth,
                };
            }, value);
            const detail = `"${value}": ${JSON.stringify(m)}`;
            // The value's box ends before the chevron's (the trigger's own
            // `gap` -- shadcn's gap-1.5 -- is the clear space)...
            expect(m.iconLeft - m.valueRight, detail).toBeGreaterThanOrEqual(6);
            // ...on one line, in a trigger that never outgrows the screen.
            expect(m.triggerHeight, detail).toBeLessThanOrEqual(33);
            expect(m.triggerRight, detail).toBeLessThanOrEqual(m.viewport);
        }
    });

    test("select (rtl): the check sits on the inline-end side and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("select", "rtl"));
        const trigger = page.getByRole("combobox").first();
        await trigger.click();
        const listbox = page.getByRole("listbox");
        await expect(listbox).toHaveAttribute("data-state", "open");
        const option = listbox.getByRole("option", { name: "First" });
        const measure = await labelIndicatorGap(option, "svg");
        // In RTL the inline end is the LEFT edge: the check precedes the label
        // on screen, and `gap` is measured on that side.
        expect(measure.indicatorRight, JSON.stringify(measure)).toBeLessThanOrEqual(measure.labelLeft);
        await expectGapHolds(page, option, "svg", "select option check (rtl)");
    });

    test("combobox: the selected option's check keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("combobox"));
        const input = page.getByRole("combobox", { name: "Select framework" });
        await input.click();
        const listbox = page.locator("[role='listbox'][data-state='open']");
        await listbox.getByRole("option", { name: "SolidStart" }).click();
        await input.click();
        await expect(listbox).toBeVisible();

        const option = listbox.getByRole("option", { name: "SolidStart" });
        await expectGapHolds(page, option, "svg", "combobox option check");
    });

    test("combobox: the typed text keeps clear of the expand chevron", async ({ page }) => {
        await gotoHydrated(page, block("combobox"));
        const gap = await page.locator(".dx-combobox-input-wrapper").first().evaluate((wrapper) => {
            const input = wrapper.querySelector("input") as HTMLInputElement;
            const icon = wrapper.querySelector(".dx-combobox-expand-icon") as Element;
            const style = getComputedStyle(input);
            const box = input.getBoundingClientRect();
            // The text is clipped at the input's content-box inline-end edge.
            const contentEnd = style.direction === "rtl"
                ? box.left + parseFloat(style.paddingLeft)
                : box.right - parseFloat(style.paddingRight);
            const iconBox = icon.getBoundingClientRect();
            return style.direction === "rtl" ? contentEnd - iconBox.right : iconBox.left - contentEnd;
        });
        expect(gap, `input text -> chevron gap ${gap}px`).toBeGreaterThanOrEqual(MIN_GAP);
    });

    test("native select: the selected text keeps clear of the chevron", async ({ page }) => {
        await gotoHydrated(page, block("native_select"));
        const gap = await page.locator(".dx-native-select-wrapper").first().evaluate((wrapper) => {
            const select = wrapper.querySelector("select") as HTMLSelectElement;
            const icon = wrapper.querySelector(".dx-native-select-icon") as Element;
            const style = getComputedStyle(select);
            const box = select.getBoundingClientRect();
            const contentEnd = style.direction === "rtl"
                ? box.left + parseFloat(style.paddingLeft)
                : box.right - parseFloat(style.paddingRight);
            const iconBox = icon.getBoundingClientRect();
            return style.direction === "rtl" ? contentEnd - iconBox.right : iconBox.left - contentEnd;
        });
        expect(gap, `native select text -> chevron gap ${gap}px`).toBeGreaterThanOrEqual(MIN_GAP);
    });

    test("dropdown menu: the sub-trigger chevron keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu"));
        await page.getByRole("button", { name: "Open Menu" }).click();
        const subTrigger = page.getByRole("menuitem", { name: "More tools" });
        await expect(subTrigger).toBeVisible();
        await expectGapHolds(page, subTrigger, "svg", "dropdown sub-trigger chevron");
    });

    test("dropdown menu (rtl): the chevron sits on the inline-end side and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "rtl"));
        await page.getByRole("button", { name: "Launch Menu" }).click();
        const subTrigger = page.getByRole("menuitem", { name: "Extra tools" });
        await expect(subTrigger).toBeVisible();
        const measure = await labelIndicatorGap(subTrigger, "svg");
        expect(measure.indicatorRight, JSON.stringify(measure)).toBeLessThanOrEqual(measure.labelLeft);
        await expectGapHolds(page, subTrigger, "svg", "dropdown sub-trigger chevron (rtl)");
    });

    test("context menu: the sub-trigger chevron keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("context_menu"));
        await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
        const subTrigger = page.getByRole("menuitem", { name: "More tools" });
        await expect(subTrigger).toBeVisible();
        await expectGapHolds(page, subTrigger, "svg", "context-menu sub-trigger chevron");
    });

    // ---- Checkable items. Dropdown and context menu: indicator at the inline END. ----

    test("dropdown menu: a checkbox item's check keeps clear of the label, and a toggle never moves the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "checkboxes"));
        await page.getByRole("button", { name: "Checkboxes" }).click();
        const statusBar = page.getByRole("menuitemcheckbox", { name: "Status Bar" });
        const panel = page.getByRole("menuitemcheckbox", { name: "Panel" });
        await expect(statusBar).toBeVisible();
        await expectCheckableSlot(page, statusBar, panel, "ltr", "end", "dropdown checkbox item");
    });

    test("dropdown menu: toggling a checkbox item keeps its label where it was", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "checkboxes"));
        await page.getByRole("button", { name: "Checkboxes" }).click();
        const panel = page.getByRole("menuitemcheckbox", { name: "Panel" });
        const before = await labelIndicatorGap(panel, "svg");
        await panel.click();
        await expect(panel).toHaveAttribute("aria-checked", "true");
        await expect(panel.locator("svg")).toBeVisible();
        const after = await labelIndicatorGap(panel, "svg");
        expect(Math.abs(after.labelLeft - before.labelLeft), `${JSON.stringify([before, after])}`).toBeLessThanOrEqual(0.5);
        expect(after.gap, JSON.stringify(after)).toBeGreaterThanOrEqual(MIN_GAP);
    });

    test("dropdown menu: a checkbox item inside a submenu keeps its check clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "checkboxes"));
        await page.getByRole("button", { name: "Checkboxes" }).click();
        await page.getByRole("menuitem", { name: "More options" }).click();
        const wordWrap = page.getByRole("menuitemcheckbox", { name: "Word Wrap" });
        const minimap = page.getByRole("menuitemcheckbox", { name: "Minimap" });
        await expect(wordWrap).toBeVisible();
        // Neither is checked yet: check one so both states are measured.
        await wordWrap.click();
        await expect(wordWrap).toHaveAttribute("aria-checked", "true");
        await expectCheckableSlot(page, wordWrap, minimap, "ltr", "end", "dropdown submenu checkbox item");
    });

    test("dropdown menu: a radio item's check keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "radio_group"));
        await page.getByRole("button", { name: "Radio Group" }).click();
        const bottom = page.getByRole("menuitemradio", { name: "Bottom" });
        const top = page.getByRole("menuitemradio", { name: "Top" });
        await expect(bottom).toBeVisible();
        await expectCheckableSlot(page, bottom, top, "ltr", "end", "dropdown radio item");
    });

    test("dropdown menu (rtl): the checkable items' check sits on the inline-end (left) side and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("dropdown_menu", "rtl"));
        await page.getByRole("button", { name: "Launch Menu" }).click();
        const pinned = page.getByRole("menuitemcheckbox", { name: "Pinned" });
        const ascending = page.getByRole("menuitemradio", { name: "Ascending" });
        const descending = page.getByRole("menuitemradio", { name: "Descending" });
        await expect(pinned).toBeVisible();
        // Checkbox: "Pinned" is checked in the fixture; radio: "Ascending" is
        // chosen and "Descending" is not.
        await expectCheckableSlot(page, ascending, descending, "rtl", "end", "dropdown radio item (rtl)");
        const m = await labelIndicatorGap(pinned, "svg");
        expect(indicatorSide(m, "rtl"), JSON.stringify(m)).toBe("end");
        await expectGapHolds(page, pinned, "svg", "dropdown checkbox item (rtl)");
    });

    test("context menu: a checkbox item's check keeps clear of the label, and a toggle never moves the label", async ({ page }) => {
        await gotoHydrated(page, block("context_menu", "checkboxes"));
        await page.getByRole("button", { name: "Right click for checkboxes" }).click({ button: "right" });
        const bookmarks = page.getByRole("menuitemcheckbox", { name: "Show Bookmarks Bar" });
        const fullUrls = page.getByRole("menuitemcheckbox", { name: "Show Full URLs" });
        await expect(bookmarks).toBeVisible();
        await expectCheckableSlot(page, bookmarks, fullUrls, "ltr", "end", "context-menu checkbox item");
    });

    test("context menu: a radio item's check keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("context_menu", "radio_group"));
        await page.getByRole("button", { name: "Right click for radio group" }).click({ button: "right" });
        const pedro = page.getByRole("menuitemradio", { name: "Pedro Duarte" });
        const colm = page.getByRole("menuitemradio", { name: "Colm Tuite" });
        await expect(pedro).toBeVisible();
        await expectCheckableSlot(page, pedro, colm, "ltr", "end", "context-menu radio item");
    });

    test("context menu (rtl): the checkable items' check sits on the inline-end (left) side and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("context_menu", "rtl"));
        await page.getByRole("button", { name: "RTL context zone" }).click({ button: "right" });
        const pinned = page.getByRole("menuitemcheckbox", { name: "Pinned" });
        const ascending = page.getByRole("menuitemradio", { name: "Ascending" });
        const descending = page.getByRole("menuitemradio", { name: "Descending" });
        await expect(pinned).toBeVisible();
        await expectCheckableSlot(page, ascending, descending, "rtl", "end", "context-menu radio item (rtl)");
        const m = await labelIndicatorGap(pinned, "svg");
        expect(indicatorSide(m, "rtl"), JSON.stringify(m)).toBe("end");
        await expectGapHolds(page, pinned, "svg", "context-menu checkbox item (rtl)");
    });

    test("context menu: plain and checkable items share one label edge, so a mixed menu stays aligned", async ({ page }) => {
        await gotoHydrated(page, block("context_menu", "main"));
        await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
        const plain = page.getByRole("menuitem", { name: "Edit" });
        const checkable = page.getByRole("menuitemcheckbox", { name: "Show Bookmarks" });
        await expect(checkable).toBeVisible();
        const p = await labelIndicatorGap(checkable, "svg");
        // A plain item draws no indicator: measure its label alone.
        const plainLabel = await plain.evaluate((el) => {
            const range = document.createRange();
            range.selectNodeContents(el);
            const rect = range.getBoundingClientRect();
            return { left: rect.left, right: rect.right };
        });
        expect(Math.abs(plainLabel.left - p.labelLeft), `plain ${JSON.stringify(plainLabel)} vs checkable ${JSON.stringify(p)}`).toBeLessThanOrEqual(0.5);
    });

    // ---- Menubar: indicator at the inline START (shadcn Nova's `pl-7` + `left-1.5`). ----

    test("menubar: a checkbox item's check sits at the inline start and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("menubar", "checkboxes"));
        await page.getByRole("menuitem", { name: "View" }).click();
        const fullUrls = page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" });
        const bookmarks = page.getByRole("menuitemcheckbox", { name: "Always Show Bookmarks Bar" });
        await expect(fullUrls).toBeVisible();
        await expectCheckableSlot(page, fullUrls, bookmarks, "ltr", "start", "menubar checkbox item");
    });

    test("menubar: a radio item's check sits at the inline start and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("menubar", "radio_group"));
        await page.getByRole("menuitem", { name: "Accounts" }).click();
        const benoit = page.getByRole("menuitemradio", { name: "Benoit" });
        const andy = page.getByRole("menuitemradio", { name: "Andy" });
        await expect(benoit).toBeVisible();
        await expectCheckableSlot(page, benoit, andy, "ltr", "start", "menubar radio item");
    });

    test("menubar (rtl): the checkable items' check sits at the inline start (the right edge) and keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("menubar", "rtl"));
        await page.getByRole("menuitem", { name: "Options" }).click();
        const pinned = page.getByRole("menuitemcheckbox", { name: "Pinned" });
        await expect(pinned).toBeVisible();
        const m = await labelIndicatorGap(pinned, "svg");
        expect(indicatorSide(m, "rtl"), JSON.stringify(m)).toBe("start");
        // Start in RTL is the right: the check follows the label on screen.
        expect(m.indicatorLeft, JSON.stringify(m)).toBeGreaterThanOrEqual(m.labelRight - 0.5);
        await expectGapHolds(page, pinned, "svg", "menubar checkbox item (rtl)");
    });

    test("menubar: an inset plain item shares the checkable items' label edge, so a mixed menu stays aligned", async ({ page }) => {
        await gotoHydrated(page, block("menubar", "checkboxes"));
        await page.getByRole("menuitem", { name: "View" }).click();
        const checkable = page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" });
        const reload = page.getByRole("menuitem", { name: "Reload", exact: true });
        await expect(checkable).toBeVisible();
        const c = await labelIndicatorGap(checkable, "svg");
        const plainLabel = await reload.evaluate((el) => {
            const range = document.createRange();
            range.selectNodeContents(el);
            const rect = range.getBoundingClientRect();
            return { left: rect.left, right: rect.right };
        });
        expect(Math.abs(plainLabel.left - c.labelLeft), `inset plain ${JSON.stringify(plainLabel)} vs checkable ${JSON.stringify(c)}`).toBeLessThanOrEqual(0.5);
    });

    test("command: the trailing shortcut keeps clear of the label", async ({ page }) => {
        await gotoHydrated(page, block("command"));
        await page.getByRole("button", { name: "Open Command Palette" }).click();
        const item = page.getByRole("option", { name: /New File/ });
        await expect(item).toBeVisible();
        await expectGapHolds(page, item, "[data-command-item-shortcut]", "command shortcut");
    });
});

/**
 * One value everywhere. The construction is only a construction if every
 * stylesheet reads the same numbers; a stylesheet that quietly picked 4px
 * would reopen the defect for its component alone. Each stylesheet declares
 * the tokens on its own item (components are copy-out units and cannot share
 * a file), so this reads them back from the rendered item. `command` has no
 * fixed-size indicator (its shortcut is in-flow text), so it declares `gap`
 * only; every other component also declares `size`.
 */
test.describe("Menu-item indicator slot: one set of tokens", () => {
    const readTokens = (el: Element) => {
        const read = (name: string) => getComputedStyle(el).getPropertyValue(name).trim();
        return {
            size: read("--dx-menu-item-indicator-size"),
            gap: read("--dx-menu-item-indicator-gap"),
        };
    };

    test("select, combobox, native select, dropdown, context menu, menubar and command agree on size and gap", async ({ page }) => {
        const seen: Record<string, { size: string; gap: string }> = {};

        await gotoHydrated(page, block("select"));
        await page.getByRole("combobox").first().click();
        seen.select = await page.getByRole("option").first().evaluate(readTokens);

        await gotoHydrated(page, block("combobox"));
        await page.getByRole("combobox", { name: "Select framework" }).click();
        seen.combobox = await page.locator("[role='listbox'][data-state='open'] [role='option']").first().evaluate(readTokens);

        await gotoHydrated(page, block("native_select"));
        seen.native_select = await page.locator(".dx-native-select-wrapper").first().evaluate(readTokens);

        await gotoHydrated(page, block("dropdown_menu"));
        await page.getByRole("button", { name: "Open Menu" }).click();
        seen.dropdown_menu = await page.getByRole("menuitem", { name: "Edit" }).evaluate(readTokens);

        await gotoHydrated(page, block("context_menu"));
        await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
        seen.context_menu = await page.getByRole("menuitem", { name: "Edit" }).evaluate(readTokens);

        await gotoHydrated(page, block("menubar"));
        await page.getByRole("menuitem", { name: "File" }).click();
        seen.menubar = await page.getByRole("menuitem", { name: "New" }).evaluate(readTokens);

        await gotoHydrated(page, block("command"));
        await page.getByRole("button", { name: "Open Command Palette" }).click();
        seen.command = await page.getByRole("option", { name: /New File/ }).evaluate(readTokens);

        const detail = JSON.stringify(seen);
        const reference = seen.select;
        expect(reference.size, detail).not.toBe("");
        expect(reference.gap, detail).not.toBe("");
        for (const [component, tokens] of Object.entries(seen)) {
            expect(tokens.gap, `${component} gap must match select's -- ${detail}`).toBe(reference.gap);
            if (component !== "command") {
                expect(tokens.size, `${component} size must match select's -- ${detail}`).toBe(reference.size);
            }
        }
    });
});
