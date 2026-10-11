import { test, expect } from "./fixtures";
import { expectNoAxeViolations } from "./axe";
import { BASE_URL as BASE } from "./base-url";

const URL = `${BASE}/component/?name=drag_and_drop_list&`;
const REMOVABLE_URL = `${BASE}/component/block/?name=drag_and_drop_list&variant=removable&`;
const TUNING_URL = `${BASE}/component/block/?name=drag_and_drop_list&variant=tuning&`;
const LOAD_TIMEOUT = 20 * 60 * 1000;

/** Navigate to the DnD page and return the first (main) variant list. */
async function loadMainList(page: import("@playwright/test").Page) {
  await page.goto(URL, { timeout: LOAD_TIMEOUT });
  const list = page.getByRole("list", { name: "Sortable list" }).first();
  await expect(list).toBeVisible({ timeout: 30000 });
  return list;
}

/** Navigate to the DnD page and return the second (removable) variant list. */
async function loadRemovableList(page: import("@playwright/test").Page) {
  await page.goto(REMOVABLE_URL, { timeout: LOAD_TIMEOUT });
  const list = page.getByRole("list", { name: "Sortable list" }).first();
  await expect(list).toBeVisible({ timeout: 30000 });
  return list;
}

/** Navigate to the slider-driven (tuning) variant and return its list. */
async function loadTuningList(page: import("@playwright/test").Page) {
  await page.goto(TUNING_URL, { timeout: LOAD_TIMEOUT });
  const list = page.getByRole("list", { name: "Sortable list" }).first();
  await expect(list).toBeVisible({ timeout: 30000 });
  return list;
}

/** Helper to get list items from a dnd-list container. */
function getItems(list: import("@playwright/test").Locator) {
  return list.locator('[aria-roledescription="sortable item"]');
}

function getLiveRegion(list: import("@playwright/test").Locator) {
  return list.locator("xpath=..").locator('[role="status"][aria-live="assertive"]');
}

/**
 * Identity of a list item for before/after comparisons: its text, minus the
 * avatar. The demo's `ImageAvatar` renders its fallback initials ("RH") only
 * once the image has failed or timed out, so reading the whole `textContent`
 * twice -- once before and once after that happens -- compared two different
 * strings for the same item (`...Apr29` vs `...Apr29RH`) and flaked under
 * parallel load (backlog row 114: 7-8 failures in ~100 runs at
 * `--workers=4`). The avatar is decoration, not part of an item's identity;
 * what remains (title, task code, status, due date) is stable from the first
 * render, so nothing needs to wait for the avatars to settle.
 */
async function itemText(locator: import("@playwright/test").Locator) {
  return locator.evaluate((el) => {
    const clone = el.cloneNode(true) as HTMLElement;
    clone.querySelectorAll(".dx-avatar").forEach((node) => node.remove());
    return (clone.textContent ?? "").replace(/\s+/g, "");
  });
}

async function dispatchDragLifecycle(
  page: import("@playwright/test").Page,
  options: {
    sourceIndex: number;
    targetIndex: number;
    drop?: "list" | "document";
    end?: boolean;
    /** Where in the target's height the pointer is: < 0.5 = before it, >= 0.5 = after it. */
    yFraction?: number;
  },
) {
  await page.evaluate(async ({ sourceIndex, targetIndex, drop, end = true, yFraction = 0.8 }) => {
    const list = document.querySelector('ul[aria-roledescription="sortable list"]');
    const items = list?.querySelectorAll('li[aria-roledescription="sortable item"]');
    const source = items?.[sourceIndex];
    const target = items?.[targetIndex];
    if (!list || !source || !target) {
      throw new Error("Drag-and-drop test elements were not found");
    }

    const dataTransfer = new DataTransfer();
    const dispatch = (node: EventTarget, type: string, init: DragEventInit = {}) => {
      const event = new DragEvent(type, {
        bubbles: true,
        cancelable: true,
        dataTransfer,
        ...init,
      });
      node.dispatchEvent(event);
    };

    dispatch(source, "dragstart");
    await new Promise(requestAnimationFrame);

    const targetRect = target.getBoundingClientRect();
    dispatch(target, "dragover", {
      clientX: targetRect.left + targetRect.width / 2,
      clientY: targetRect.top + targetRect.height * yFraction,
    });
    await new Promise(requestAnimationFrame);

    if (drop === "list") {
      dispatch(list, "drop");
      await new Promise(requestAnimationFrame);
    } else if (drop === "document") {
      dispatch(document, "drop");
      await new Promise(requestAnimationFrame);
    }

    if (end) {
      dispatch(source, "dragend");
      await new Promise(requestAnimationFrame);
    }
  }, options);
}

/** Opacity of the visible drop-indicator line (0 when none is shown). */
async function dropIndicatorOpacity(
  page: import("@playwright/test").Page,
): Promise<number> {
  return page.evaluate(() => {
    const beforeTarget = document.querySelector(
      '[data-position="before"] + li[aria-roledescription="sortable item"]',
    );
    if (beforeTarget) {
      return Number.parseFloat(
        getComputedStyle(beforeTarget, "::before").opacity,
      );
    }
    const afterTarget = document.querySelector(
      'li[aria-roledescription="sortable item"]:has(+ [data-position="after"])',
    );
    if (afterTarget) {
      return Number.parseFloat(
        getComputedStyle(afterTarget, "::after").opacity,
      );
    }
    return 0;
  });
}

test.describe("Keyboard focus management", () => {
  test("first item is tab-reachable", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await expect(items.first()).toHaveAttribute("tabindex", "0");
    await expect(items.nth(1)).toHaveAttribute("tabindex", "-1");
  });

  test("arrow up from first wraps to last", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const lastIndex = (await items.count()) - 1;
    await items.first().click();
    await page.keyboard.press("ArrowUp");
    await expect(items.nth(lastIndex)).toBeFocused();
  });

  test("arrow down from last wraps to first", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const lastIndex = (await items.count()) - 1;
    await items.nth(lastIndex).click();
    await page.keyboard.press("ArrowDown");
    await expect(items.first()).toBeFocused();
  });

  test("roving tabindex updates on arrow navigation", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.first().click();
    await page.keyboard.press("ArrowDown");
    await expect(items.first()).toHaveAttribute("tabindex", "-1");
    await expect(items.nth(1)).toHaveAttribute("tabindex", "0");
  });
});

test.describe("Drag and drop lifecycle", () => {
  test("each arrow press moves one position with announcement", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const liveRegion = getLiveRegion(list);
    const itemCount = await items.count();

    // Grab item 3 (index 2)
    await items.nth(2).click();
    await page.keyboard.press("Enter");
    await expect(liveRegion).toContainText(
      `You have lifted an item in position 3 of ${itemCount}`,
    );

    // First ArrowUp immediately moves to position 2
    await page.keyboard.press("ArrowUp");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 2 of ${itemCount}`,
    );

    // Second ArrowUp moves to position 1
    await page.keyboard.press("ArrowUp");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 1 of ${itemCount}`,
    );

    // ArrowDown back to position 2
    await page.keyboard.press("ArrowDown");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 2 of ${itemCount}`,
    );

    // ArrowDown past original position to position 3
    await page.keyboard.press("ArrowDown");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 3 of ${itemCount}`,
    );

    // ArrowDown to position 4
    await page.keyboard.press("ArrowDown");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 4 of ${itemCount}`,
    );

    // Drop
    await page.keyboard.press("Enter");
    await expect(liveRegion).toContainText(
      "You have dropped the item. It has moved from position 3 to position 4",
    );
  });

  test("cancelling announces and returns focus to source", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.first().click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Escape");
    const liveRegion = getLiveRegion(list);
    await expect(liveRegion).toContainText(
      "Movement cancelled. The item has returned to its starting position of 1",
    );
    await expect(items.first()).toBeFocused();
  });

  test("grabbed item has aria-grabbed true", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.first().click();
    await page.keyboard.press("Enter");
    await expect(items.first()).toHaveAttribute("aria-grabbed", "true");
  });

  test("focus after successful drop lands on moved item", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.first().click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(items.nth(1)).toBeFocused();
  });

  test("space key grabs and drops items", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const itemCount = await items.count();
    await items.first().click();
    await page.keyboard.press("Space");
    const liveRegion = getLiveRegion(list);
    await expect(liveRegion).toContainText(
      `You have lifted an item in position 1 of ${itemCount}`,
    );
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Space");
    await expect(liveRegion).toContainText(
      "You have dropped the item. It has moved from position 1 to position 2",
    );
  });

  test("mouse drag shows the drop-indicator line", async ({ page }) => {
    const list = await loadMainList(page);

    await dispatchDragLifecycle(page, {
      sourceIndex: 2,
      targetIndex: 3,
      end: false,
    });

    await expect(page.locator("[data-position]")).toHaveCount(1);
    await expect.poll(() => dropIndicatorOpacity(page)).toBeGreaterThan(0.5);
  });

  test("keyboard drag shows the drop-indicator line", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.nth(2).click();
    await expect(items.nth(2)).toBeFocused();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");

    await expect.poll(() => dropIndicatorOpacity(page)).toBeGreaterThan(0.5);

    await page.keyboard.press("Escape");
  });

  test("mouse drop to the side commits without cancelling the native drop", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const sourceTitle = await itemText(items.nth(2));

    await dispatchDragLifecycle(page, {
      sourceIndex: 2,
      targetIndex: 3,
      drop: "document",
    });

    await expect.poll(() => itemText(items.nth(3))).toBe(sourceTitle);
  });

  test("cancelled mouse drag does not reorder the list", async ({ page }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const sourceText = await itemText(items.nth(2));
    const targetText = await itemText(items.nth(3));

    await dispatchDragLifecycle(page, {
      sourceIndex: 2,
      targetIndex: 3,
    });

    await expect.poll(() => itemText(items.nth(2))).toBe(sourceText);
    await expect.poll(() => itemText(items.nth(3))).toBe(targetText);
  });
});

/** Computed opacity of the list's ghost: the item being dragged (`data-is-grabbing`). */
function ghostOpacity(list: import("@playwright/test").Locator) {
  return list.evaluate((ul) => {
    const ghost = ul.querySelector('li[data-is-grabbing="true"]');
    return ghost ? Number.parseFloat(getComputedStyle(ghost).opacity) : Number.NaN;
  });
}

/** A CSS custom property as the list (hence its items) resolves it. */
function cssVar(list: import("@playwright/test").Locator, name: string) {
  return list.evaluate(
    (ul, prop) => getComputedStyle(ul).getPropertyValue(prop).trim(),
    name,
  );
}

/** Vertical space (px) between item `above`'s bottom edge and item `below`'s top edge. */
function spaceBetween(
  list: import("@playwright/test").Locator,
  above: number,
  below: number,
) {
  return list.evaluate(
    (ul, [a, b]) => {
      const items = ul.querySelectorAll('li[aria-roledescription="sortable item"]');
      return items[b].getBoundingClientRect().top - items[a].getBoundingClientRect().bottom;
    },
    [above, below] as const,
  );
}

/** Duration (ms) of the `transform` transition on item `index`. */
function transformTransitionMs(list: import("@playwright/test").Locator, index: number) {
  return list.evaluate((ul, i) => {
    const item = ul.querySelectorAll('li[aria-roledescription="sortable item"]')[i];
    const style = getComputedStyle(item);
    const props = style.transitionProperty.split(",").map((p) => p.trim());
    const durations = style.transitionDuration.split(",").map((d) => d.trim());
    const at = props.indexOf("transform");
    const raw = durations[at % durations.length];
    return raw.endsWith("ms") ? Number.parseFloat(raw) : Number.parseFloat(raw) * 1000;
  }, index);
}

/** The slider thumb labelled `name` in the tuning variant. */
function tuningSlider(page: import("@playwright/test").Page, name: string) {
  return page.getByRole("slider", { name });
}

const DEFAULT_GHOST_OPACITY = 0.9;
const DEFAULT_DROP_GAP = 25;

test.describe("Ghost opacity", () => {
  test("the dragged item's ghost is 0.9 opaque by default, at its origin and once moved", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    // Compared as a number: the asset pipeline minifies the stylesheet's `0.9` to `.9`.
    expect(Number(await cssVar(list, "--dx-dnd-ghost-opacity"))).toBeCloseTo(
      DEFAULT_GHOST_OPACITY,
      5,
    );

    await items.nth(2).click();
    await page.keyboard.press("Enter");
    // Lifted: the drop target is still the item's own slot.
    await expect(items.nth(2)).toHaveAttribute("data-drop-at-origin", "true");
    await expect
      .poll(() => ghostOpacity(list))
      .toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);

    // Moved away from the origin: a different CSS state, the same ghost.
    await page.keyboard.press("ArrowDown");
    await expect(items.nth(2)).not.toHaveAttribute("data-drop-at-origin", "true");
    await expect(items.nth(2)).toHaveAttribute("data-is-grabbing", "true");
    await expect
      .poll(() => ghostOpacity(list))
      .toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);
    await page.keyboard.press("Escape");
  });

  test("a mouse drag leaves the same ghost opacity", async ({ page }) => {
    const list = await loadMainList(page);
    await dispatchDragLifecycle(page, { sourceIndex: 1, targetIndex: 4, end: false });
    await expect
      .poll(() => ghostOpacity(list))
      .toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);
  });

  test("the ghost's title text passes axe colour contrast at the default opacity", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    await items.nth(2).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    // axe reads the live computed opacity: let the opacity transition settle first.
    await expect
      .poll(() => ghostOpacity(list))
      .toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);
    // Scoped to the title, the item's primary text (14.6:1 at 0.9; 3.5:1 at the
    // old 0.5, which this scan would flag). The muted meta line is not in
    // scope: it is 4.0:1 at 0.9 -- translucency costs it more than any other
    // text, and only 0.96+ reaches 4.5:1 (see style.css).
    await expectNoAxeViolations(page, "drag_and_drop_list: ghost title at default opacity", {
      include: 'li[data-is-grabbing="true"] .dx-task-title',
    });
    await page.keyboard.press("Escape");
  });

  test("the ghost_opacity prop sets the ghost and --dx-dnd-ghost-opacity", async ({
    page,
  }) => {
    const list = await loadTuningList(page);
    const items = getItems(list);
    const slider = tuningSlider(page, "Ghost opacity");
    await slider.focus();
    await page.keyboard.press("Home");
    const min = Number(await slider.getAttribute("aria-valuemin"));
    expect(min).toBeGreaterThan(0);
    expect(min).toBeLessThan(DEFAULT_GHOST_OPACITY);
    await expect(slider).toHaveAttribute("aria-valuenow", String(min));
    await expect.poll(() => cssVar(list, "--dx-dnd-ghost-opacity")).toBe(String(min));

    await items.nth(1).click();
    await page.keyboard.press("Enter");
    await expect.poll(() => ghostOpacity(list)).toBeCloseTo(min, 2);
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => ghostOpacity(list)).toBeCloseTo(min, 2);
    await page.keyboard.press("Escape");
  });
});

test.describe("Drop gap", () => {
  test("the other items make 25px of room at the drop slot by default (keyboard)", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    expect(await cssVar(list, "--dx-dnd-drop-gap")).toBe(`${DEFAULT_DROP_GAP}px`);
    const rest = await spaceBetween(list, 1, 2);
    expect(rest).toBeCloseTo(6, 0);

    await items.nth(0).click();
    await page.keyboard.press("Enter");
    // Lifted but still at its origin: there is no slot to open.
    await expect.poll(() => spaceBetween(list, 0, 1)).toBeCloseTo(rest, 0);
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest, 0);

    // One step down: the slot is between items 1 and 2.
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    // Only that one slot opens; every other seam keeps its resting space.
    for (const [above, below] of [[0, 1], [2, 3], [3, 4], [4, 5]]) {
      expect(await spaceBetween(list, above, below)).toBeCloseTo(rest, 0);
    }

    // The room follows the slot.
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => spaceBetween(list, 2, 3)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest, 0);
    await page.keyboard.press("Escape");
    // Cancelled: everything closes again.
    await expect.poll(() => spaceBetween(list, 2, 3)).toBeCloseTo(rest, 0);
  });

  test("the same room opens for a mouse drag, above and below the source", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const rest = await spaceBetween(list, 3, 4);

    // Downwards: pointer in the lower half of item 3 -> slot between 3 and 4.
    await dispatchDragLifecycle(page, { sourceIndex: 0, targetIndex: 3, end: false });
    await expect.poll(() => spaceBetween(list, 3, 4)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    expect(await spaceBetween(list, 4, 5)).toBeCloseTo(rest, 0);
  });

  test("a mouse drag upwards opens the room above the hovered item", async ({ page }) => {
    const list = await loadMainList(page);
    const rest = await spaceBetween(list, 0, 1);
    // Upwards: pointer in the upper half of item 1 -> slot between 0 and 1.
    await dispatchDragLifecycle(page, {
      sourceIndex: 4,
      targetIndex: 1,
      yFraction: 0.2,
      end: false,
    });
    await expect.poll(() => spaceBetween(list, 0, 1)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    expect(await spaceBetween(list, 1, 2)).toBeCloseTo(rest, 0);
  });

  test("a drop slot after the last item opens no room but still shows the line", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    const items = getItems(list);
    const last = (await items.count()) - 1;
    const rest = await spaceBetween(list, last - 1, last);
    await dispatchDragLifecycle(page, { sourceIndex: 0, targetIndex: last, end: false });
    await expect(page.locator("[data-position]")).toHaveCount(1);
    await expect.poll(() => dropIndicatorOpacity(page)).toBeGreaterThan(0.5);
    expect(await spaceBetween(list, last - 1, last)).toBeCloseTo(rest, 0);
  });

  test("the drop_gap prop sets the room and --dx-dnd-drop-gap", async ({ page }) => {
    const list = await loadTuningList(page);
    const items = getItems(list);
    const slider = tuningSlider(page, "Drop gap");
    const rest = await spaceBetween(list, 1, 2);
    await slider.focus();
    await page.keyboard.press("End");
    const max = Number(await slider.getAttribute("aria-valuemax"));
    expect(max).toBeGreaterThan(DEFAULT_DROP_GAP);
    await expect(slider).toHaveAttribute("aria-valuenow", String(max));
    await expect.poll(() => cssVar(list, "--dx-dnd-drop-gap")).toBe(`${max}px`);

    await items.nth(0).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest + max, 0);

    // And back to nothing.
    await page.keyboard.press("Escape");
    await slider.focus();
    await page.keyboard.press("Home");
    await expect.poll(() => cssVar(list, "--dx-dnd-drop-gap")).toBe("0px");
    await items.nth(0).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest, 0);
    await page.keyboard.press("Escape");
  });

  test("the room animates, and honours prefers-reduced-motion", async ({ page }) => {
    const list = await loadMainList(page);
    expect(await transformTransitionMs(list, 3)).toBeGreaterThan(100);

    await page.emulateMedia({ reducedMotion: "reduce" });
    // Every item, in every state, must follow: the item that is lifted too.
    for (const index of [0, 3]) {
      expect(await transformTransitionMs(list, index)).toBeLessThan(1);
    }
    const items = getItems(list);
    await items.nth(0).click();
    await page.keyboard.press("Enter");
    await expect(items.nth(0)).toHaveAttribute("data-is-grabbing", "true");
    for (const index of [0, 3]) {
      expect(await transformTransitionMs(list, index)).toBeLessThan(1);
    }
    // The room itself still opens (instantly): reduced motion removes the
    // animation, not the affordance.
    const rest = await spaceBetween(list, 3, 4);
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => spaceBetween(list, 3, 4)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    await page.keyboard.press("Escape");
  });

  test("RTL: keyboard reordering, announcements, gap and ghost are unchanged", async ({
    page,
  }) => {
    const list = await loadMainList(page);
    await page.evaluate(() => document.documentElement.setAttribute("dir", "rtl"));
    const items = getItems(list);
    const liveRegion = getLiveRegion(list);
    const itemCount = await items.count();
    const rest = await spaceBetween(list, 1, 2);

    await items.nth(0).click();
    await page.keyboard.press("Enter");
    await expect(liveRegion).toContainText(
      `You have lifted an item in position 1 of ${itemCount}`,
    );
    await page.keyboard.press("ArrowDown");
    await expect(liveRegion).toContainText(
      `You have moved the item to position 2 of ${itemCount}`,
    );
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    await expect.poll(() => ghostOpacity(list)).toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);
    await expect.poll(() => dropIndicatorOpacity(page)).toBeGreaterThan(0.5);
    await page.keyboard.press("Enter");
    await expect(liveRegion).toContainText(
      "You have dropped the item. It has moved from position 1 to position 2",
    );
    await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest, 0);
  });
});

/*
 * The home page renders the list twice -- the "Launch priorities" card in the
 * masonry gallery (its own `BlockTasks` composition, no ghost/gap props) and the
 * component's own demo in the catalog -- and both sit in a card with
 * `content-visibility: auto` (paint containment). The ghost and the drop gap
 * must be the component's defaults there too, and the room the gap opens must
 * not be clipped by the card. Owner report, round 4: "the dnd list on the main
 * overview demo doesn't have the new ghost / gap".
 */
test.describe("Home page lists", () => {
  const HOME_LISTS = [
    { name: "masonry card (Launch priorities)", index: 0 },
    { name: "catalog card (component demo)", index: 1 },
  ];

  for (const { name, index } of HOME_LISTS) {
    test(`${name}: default ghost opacity and drop gap, room not clipped`, async ({ page }) => {
      await page.goto(`${BASE}/`, { timeout: LOAD_TIMEOUT });
      const list = page.locator('ul[aria-roledescription="sortable list"]').nth(index);
      await expect(list).toBeAttached({ timeout: 30000 });
      // The cards are `content-visibility: auto`: render this one for real.
      await list.evaluate((el) => el.scrollIntoView({ block: "center" }));
      await expect(list).toBeVisible();
      const items = getItems(list);

      // Same custom properties as the component page resolves.
      expect(Number(await cssVar(list, "--dx-dnd-ghost-opacity"))).toBeCloseTo(
        DEFAULT_GHOST_OPACITY,
        5,
      );
      expect(await cssVar(list, "--dx-dnd-drop-gap")).toBe(`${DEFAULT_DROP_GAP}px`);

      const rest = await spaceBetween(list, 1, 2);
      await items.nth(0).click();
      await page.keyboard.press("Enter");
      await expect(items.nth(0)).toHaveAttribute("data-is-grabbing", "true");
      await page.keyboard.press("ArrowDown");
      await expect.poll(() => spaceBetween(list, 1, 2)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
      await expect.poll(() => ghostOpacity(list)).toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);

      // Slot before the last item: the last item moves down by the gap and must
      // still be inside the card (paint containment clips at its padding box).
      const last = (await items.count()) - 1;
      for (let i = 1; i < last - 1; i++) await page.keyboard.press("ArrowDown");
      await expect.poll(() => spaceBetween(list, last - 1, last)).toBeCloseTo(
        rest + DEFAULT_DROP_GAP,
        0,
      );
      const clipped = await list.evaluate((ul) => {
        const card = ul.closest(".dx-widget-card, .dx-component-card");
        const lastItem = [...ul.querySelectorAll('li[aria-roledescription="sortable item"]')].pop();
        if (!card || !lastItem) return "no card or item";
        const slack = card.getBoundingClientRect().bottom - lastItem.getBoundingClientRect().bottom;
        return slack >= 0 ? "" : `last item overflows its card by ${-slack}px`;
      });
      expect(clipped).toBe("");
      await page.keyboard.press("Escape");
      await expect.poll(() => spaceBetween(list, last - 1, last)).toBeCloseTo(rest, 0);
    });
  }

  test("a mouse drag on the masonry card dims the ghost and opens the gap", async ({ page }) => {
    await page.goto(`${BASE}/`, { timeout: LOAD_TIMEOUT });
    const list = page.locator('ul[aria-roledescription="sortable list"]').first();
    await expect(list).toBeAttached({ timeout: 30000 });
    await list.evaluate((el) => el.scrollIntoView({ block: "center" }));
    await expect(list).toBeVisible();
    const items = getItems(list);
    const rest = await spaceBetween(list, 1, 2);
    const source = (await items.nth(0).boundingBox())!;
    const target = (await items.nth(2).boundingBox())!;
    await page.mouse.move(source.x + 40, source.y + source.height / 2);
    await page.mouse.down();
    await page.mouse.move(source.x + 45, source.y + source.height / 2 + 10, { steps: 5 });
    await page.mouse.move(target.x + 60, target.y + target.height * 0.8, { steps: 15 });
    // Slot between items 2 and 3.
    await expect.poll(() => ghostOpacity(list)).toBeCloseTo(DEFAULT_GHOST_OPACITY, 2);
    await expect.poll(() => spaceBetween(list, 2, 3)).toBeCloseTo(rest + DEFAULT_DROP_GAP, 0);
    await page.mouse.up();
  });
});

test.describe("Instructions id (backlog row 120)", () => {
  test("every list's aria-describedby resolves to its own instructions element", async ({
    page,
  }) => {
    // The component page renders every variant, so several lists share it.
    await loadMainList(page);
    const lists = await page.evaluate(() =>
      [...document.querySelectorAll('ul[aria-roledescription="sortable list"]')].map((ul) => {
        const id = ul.getAttribute("aria-describedby") ?? "";
        const matches = id ? [...document.querySelectorAll(`[id="${id}"]`)] : [];
        const root = ul.parentElement;
        return {
          id,
          matches: matches.length,
          text: matches[0]?.textContent ?? "",
          // The instructions sit in the same list root as the list they describe.
          sameRoot: !!matches[0] && !!root && root.contains(matches[0]),
        };
      }),
    );
    expect(lists.length).toBeGreaterThanOrEqual(2);
    for (const entry of lists) {
      expect(entry.id).not.toBe("");
      expect(entry.matches).toBe(1);
      expect(entry.sameRoot).toBe(true);
      expect(entry.text).toContain("Press Enter to start reordering");
    }
    expect(new Set(lists.map((entry) => entry.id)).size).toBe(lists.length);
  });

  test("the page has no duplicate element ids", async ({ page }) => {
    await loadMainList(page);
    const duplicates = await page.evaluate(() => {
      const seen = new Map<string, number>();
      for (const el of document.querySelectorAll("[id]")) {
        seen.set(el.id, (seen.get(el.id) ?? 0) + 1);
      }
      return [...seen].filter(([, n]) => n > 1).map(([id, n]) => `${id} x${n}`);
    });
    expect(duplicates).toEqual([]);
  });
});

test.describe("Remove behavior", () => {
  test("focus moves to item at same index after removal", async ({
    page,
  }) => {
    const list = await loadRemovableList(page);
    const items = getItems(list);
    const initialCount = await items.count();
    const removeButtons = list.getByRole("button", { name: /Remove item/ });
    await removeButtons.nth(2).click();
    await expect(items).toHaveCount(initialCount - 1);
    await expect(items.nth(2)).toBeFocused();
  });

  test("focus moves to new last item when removing last item", async ({
    page,
  }) => {
    const list = await loadRemovableList(page);
    const items = getItems(list);
    const initialCount = await items.count();
    const removeButtons = list.getByRole("button", { name: /Remove item/ });
    await removeButtons.nth(initialCount - 1).click();
    await expect(items).toHaveCount(initialCount - 1);
    await expect(items.nth(initialCount - 2)).toBeFocused();
  });

  test("remove button tracks a keyed item after reordering", async ({
    page,
  }) => {
    const list = await loadRemovableList(page);
    const items = getItems(list);
    const initialCount = await items.count();
    const movedText = await itemText(items.nth(1));

    await items.nth(1).click();
    await page.keyboard.press("Enter");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");

    await expect.poll(() => itemText(items.nth(2))).toBe(movedText);
    await items.nth(2).getByRole("button", { name: /Remove item/ }).click();

    await expect(items).toHaveCount(initialCount - 1);
    await expect.poll(async () => {
      const count = await items.count();
      const texts = await Promise.all(
        Array.from({ length: count }, (_, index) => itemText(items.nth(index))),
      );
      return texts;
    }).not.toContain(movedText);
  });
});

test.describe("Axe automated scan", () => {
  test("no automatically detectable a11y issues", async ({ page }) => {
    await loadMainList(page);

    // No color-contrast exclusion needed: this round's theme-token fix
    // (docs/backlog.md row 39) resolved the pre-existing finding here, and
    // this scan is scoped to the list itself, which has no code block to
    // need the one remaining, narrower exclusion for.
    await expectNoAxeViolations(page, "drag_and_drop_list: main list", {
      include: 'ul[aria-roledescription="sortable list"]',
    });
  });
});
