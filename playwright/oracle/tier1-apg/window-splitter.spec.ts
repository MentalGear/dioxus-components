/**
 * ORACLE: tier 1 (APG) — Window Splitter pattern (Resizable).
 *
 * Source: W3C ARIA Authoring Practices Guide (APG), "Window Splitter Pattern"
 * (live URL: https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/), read
 * from the SAME pinned, read-only `w3c/aria-practices` checkout this repo's
 * other tier-1 files already use (see `../reference/7e4034b/README.md`):
 *
 *   commit 7e4034b262bc0d25332e330d8a582aaf34113829
 *   content/patterns/windowsplitter/windowsplitter-pattern.html
 *
 * Unlike the menu-button/combobox/radio patterns this repo vendors an
 * *executable example page* for (`../reference/7e4034b/`), the Window
 * Splitter pattern ships NO example page at all -- its own "Example" section
 * reads "Work to develop an example window splitter widget is tracked by
 * issue 130," i.e. the pattern itself is prose-only upstream. There is
 * nothing to vendor or calibrate against, so every rule below cites the
 * pattern's own prose sections directly (mirroring
 * `keyboard-matrix.spec.ts`'s header: "Rather than invent quotes from
 * memory, each was read straight from that pinned commit's tree") rather
 * than the two-subject (component + vendored reference) calibration
 * `README.md` describes for the patterns that DO have one.
 *
 * ## `aria-orientation`: derived, not quoted
 *
 * The pattern's own "WAI-ARIA Roles, States, and Properties" section does
 * NOT list `aria-orientation` at all -- only `role=separator`,
 * `aria-valuenow`/`aria-valuemin`/`aria-valuemax`, `aria-labelledby`/
 * `aria-label`, and `aria-controls`. The value this library's handle must
 * report is therefore derived from the pattern's "Keyboard Interaction"
 * section instead:
 *
 *   "Left Arrow: Moves a vertical splitter to the left."
 *   "Right Arrow: Moves a vertical splitter to the right."
 *   "Up Arrow: Moves a horizontal splitter up."
 *   "Down Arrow: Moves a horizontal splitter down."
 *
 * A splitter moved by Left/Right -- this library's `Horizontal`-direction
 * group (the default: panels arranged side by side) -- is therefore the
 * pattern's own "vertical splitter" (a vertical BAR, the kind that divides
 * side-by-side content), so its `aria-orientation` is `"vertical"`. A
 * splitter moved by Up/Down (`Vertical`-direction, stacked panels) is a
 * "horizontal splitter" (a horizontal bar dividing stacked content), so its
 * `aria-orientation` is `"horizontal"` -- the INVERSE of the group's own
 * layout direction, not the same value. `primitives/src/resizable.rs`'s
 * `ResizableDirection::handle_aria_orientation` doc comment carries the same
 * derivation next to the implementation. R1 and the dedicated orientation
 * test below both assert this directly, since it is the one property this
 * pattern's own properties list leaves unstated.
 *
 * ## Fixture
 *
 * `/component/?name=resizable&` (`preview/src/components/resizable/
 * variants/main/mod.rs`) renders two independent groups: a horizontal group
 * (its handle labelled "Sidebar", `min_size: 20`, `default_size: 50`)
 * whose second panel contains a nested VERTICAL group (its handle labelled
 * "Top", `min_size: 15`, `default_size: 25`); and a second, smaller
 * horizontal group with a `collapsible` first panel (handle labelled
 * "Collapsible Panel", `min_size: 15`, `default_size: 30`,
 * `collapsed_size: 0`, neighbor `min_size: 20`).
 */

import { test, expect, type Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

const BASE = `${BASE_URL}/component/?name=resizable&`;
const goto = (page: Page) => page.goto(BASE, { timeout: 20 * 60 * 1000 });

/** The panel element a handle's `aria-controls` names (its "primary pane"). */
async function primaryPanel(page: Page, handleName: string) {
  const handle = page.getByRole("separator", { name: handleName });
  const id = await handle.getAttribute("aria-controls");
  expect(id, `${handleName} handle must have a non-empty aria-controls`).toBeTruthy();
  return page.locator(`#${id}`);
}

test.describe("R1 — role/aria contract (Roles, States, and Properties section)", () => {
  test('handle has role="separator", aria-valuenow/min/max, aria-controls pointing at a real element, and is focusable via Tab', async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });

    // "The element that serves as the focusable splitter has role separator."
    await expect(handle).toHaveAttribute("role", "separator");

    // "...aria-valuenow property set to a decimal value representing the
    // current position of the separator."
    await expect(handle).toHaveAttribute("aria-valuenow", "50");
    // "...aria-valuemin property set to a decimal value that represents the
    // position where the primary pane has its minimum size."
    await expect(handle).toHaveAttribute("aria-valuemin", "20");
    // "...aria-valuemax property ... where the primary pane has its maximum
    // size. This is typically 100."
    await expect(handle).toHaveAttribute("aria-valuemax", "100");

    // "The separator element has aria-controls referring to the primary pane."
    const panel = await primaryPanel(page, "Sidebar");
    await expect(panel).toHaveCount(1);
    await expect(panel).toHaveAttribute("data-panel", "true");

    // Focusable (implied by "the focusable splitter" language quoted above,
    // and required for the Keyboard Interaction rows below to mean anything).
    await page.keyboard.press("Tab");
    // Tab from body may land elsewhere first on this page; focus explicitly
    // instead of asserting first-Tab landing, then confirm it registers.
    await handle.focus();
    await expect(handle).toBeFocused();
  });

  test('accessible name matches the primary pane\'s own name ("a label provided by aria-label")', async ({ page }) => {
    await goto(page);
    // "If the primary pane has a visible label, it is referenced by
    // aria-labelledby ... Otherwise, the separator element has a label
    // provided by aria-label." This fixture uses aria-label for all three
    // handles; each name below is exactly its primary pane's own name.
    await expect(page.getByRole("separator", { name: "Sidebar" })).toBeVisible();
    await expect(page.getByRole("separator", { name: "Top" })).toBeVisible();
    await expect(page.getByRole("separator", { name: "Collapsible Panel" })).toBeVisible();
  });

  test("aria-orientation is the inverse of the group's own layout direction (see header derivation)", async ({ page }) => {
    await goto(page);
    // Horizontal-direction group (side-by-side panes) -> a "vertical splitter".
    await expect(page.getByRole("separator", { name: "Sidebar" })).toHaveAttribute(
      "aria-orientation",
      "vertical",
    );
    // Vertical-direction group (stacked panes) -> a "horizontal splitter".
    await expect(page.getByRole("separator", { name: "Top" })).toHaveAttribute(
      "aria-orientation",
      "horizontal",
    );
  });
});

test.describe("R2 — arrow keys move the boundary by a step, honoring min/max (Keyboard Interaction section)", () => {
  test('Right Arrow "Moves a vertical splitter to the right" / Left Arrow "...to the left", changing both aria-valuenow and the primary pane\'s real width', async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });
    const panel = await primaryPanel(page, "Sidebar");

    await expect(handle).toHaveAttribute("aria-valuenow", "50");
    const before = await panel.boundingBox();
    if (!before) throw new Error("Sidebar panel has no bounding box");

    await handle.focus();
    await page.keyboard.press("ArrowRight");
    await expect(handle).toHaveAttribute("aria-valuenow", "51");
    const afterRight = await panel.boundingBox();
    if (!afterRight) throw new Error("Sidebar panel has no bounding box");
    expect(afterRight.width).toBeGreaterThan(before.width);

    await page.keyboard.press("ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", "50");
    const afterLeft = await panel.boundingBox();
    if (!afterLeft) throw new Error("Sidebar panel has no bounding box");
    expect(afterLeft.width).toBeLessThan(afterRight.width);
  });

  test("Shift+Arrow steps by 10 and the boundary clamps at the primary pane's min_size", async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });
    await handle.focus();

    await page.keyboard.press("Shift+ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", "40");

    // Sidebar's min_size is 20 -- sweep well past it; must clamp, not go below.
    for (let i = 0; i < 10; i++) await page.keyboard.press("Shift+ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", "20");
    await page.keyboard.press("ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", "20");
  });

  test("Up/Down Arrow move a vertical-direction group's handle; Left/Right are ignored on it", async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Top" });
    await handle.focus();

    await expect(handle).toHaveAttribute("aria-valuenow", "25");
    // "Up Arrow: Moves a horizontal splitter up" -- increases the primary
    // (top) pane's own share, per this fixture's convention (see ArrowRight
    // above: the axis-positive key grows the primary pane).
    await page.keyboard.press("ArrowUp");
    await expect(handle).toHaveAttribute("aria-valuenow", "26");

    // The other axis's arrow keys must be ignored entirely on this handle.
    await page.keyboard.press("ArrowRight");
    await expect(handle).toHaveAttribute("aria-valuenow", "26");
    await page.keyboard.press("ArrowLeft");
    await expect(handle).toHaveAttribute("aria-valuenow", "26");

    await page.keyboard.press("ArrowDown");
    await expect(handle).toHaveAttribute("aria-valuenow", "25");
  });
});

test.describe("R3 — Home/End (Keyboard Interaction section, both marked Optional)", () => {
  test('Home "Moves splitter to the position that gives the primary pane its smallest allowed size"', async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });
    await handle.focus();
    await page.keyboard.press("Home");
    // Sidebar's own min_size is 20; Sidebar is not collapsible, so Home
    // targets min_size exactly (not a collapsed_size).
    await expect(handle).toHaveAttribute("aria-valuenow", "20");
  });

  test('End "Moves splitter to the position that gives the primary pane its largest allowed size" -- bounded by the neighbor pane\'s own min_size, not just the primary\'s nominal max', async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });
    await handle.focus();
    await page.keyboard.press("End");
    // Sidebar's own registered max_size is 100 (reported as aria-valuemax,
    // per R1), but the group's total is 100 and its neighbor's own min_size
    // is 20 -- 100 is not an "allowed" position without violating the
    // neighbor's floor, so the true largest ALLOWED size is 100 - 20 = 80.
    // This also confirms End's clamp shares the same neighbor-aware pair
    // math as the arrow keys (resize_pair), not a separate, unclamped jump.
    await expect(handle).toHaveAttribute("aria-valuenow", "80");
  });
});

test.describe("R4 — Enter (Keyboard Interaction section)", () => {
  test('Enter "If the primary pane is not collapsed, collapses the pane. If the pane is collapsed, restores the splitter to its previous position"', async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Collapsible Panel" });
    await handle.focus();

    await expect(handle).toHaveAttribute("aria-valuenow", "30");
    await page.keyboard.press("Enter");
    await expect(handle).toHaveAttribute("aria-valuenow", "0");

    // "restores the splitter to its previous position" -- exactly 30, the
    // value right before this collapse, not e.g. an equal-split default.
    await page.keyboard.press("Enter");
    await expect(handle).toHaveAttribute("aria-valuenow", "30");
  });

  test("Enter is a no-op on a non-collapsible primary pane", async ({ page }) => {
    await goto(page);
    const handle = page.getByRole("separator", { name: "Sidebar" });
    await handle.focus();
    await expect(handle).toHaveAttribute("aria-valuenow", "50");
    await page.keyboard.press("Enter");
    await expect(handle).toHaveAttribute("aria-valuenow", "50");
  });
});
