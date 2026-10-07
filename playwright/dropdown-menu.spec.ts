import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { gotoHydrated } from './hydration';
import { expectFadeKeepsTransform } from './assert-anchor-transform';

test('test', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
  let menuElement = page.getByRole('button', { name: 'Open Menu' });
  // The menu should not be open initially
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
  // Clicking the menu should open it
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing down should focus the first item
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'Edit' })).toBeFocused();
  await expect(page.getByRole('menuitem', { name: 'Undo' })).toHaveAttribute('data-disabled', 'true');
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'Duplicate' })).toBeFocused();
  // The menu should close after selecting an item
  await page.keyboard.press('Enter');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
  // The selected item should be displayed
  await expect(page.getByText('Selected: Duplicate')).toBeVisible();

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing Escape should close the menu
  await page.keyboard.press('Escape');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing Tab should close the menu
  await page.keyboard.press('Tab');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Clicking outside the menu should close it
  await page.locator('body').click({ position: { x: 0, y: 0 } });
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Clicking an item should close the menu.
  await page.getByRole('menuitem', { name: 'Edit' }).click();
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
});

// Regression guard for the 2026-09-04 user report ("dropdown menu has full
// page width"): `DropdownMenuContent` is a fit-content popover positioned
// under its trigger, not a full-viewport-width one. See
// `oracle/tier2-html/top-layer.spec.ts`'s "Rule 13" for the full mechanism
// this guards against (a duplicated/un-folded `style` attribute letting the
// popover UA stylesheet's centering-trap default, or a stray caller
// `width`, stretch the content) and why every anchored content's own
// stylesheet authors `min-width` only, never `width`.
test('open content is fit-content width, not full page width, and sits near its trigger', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
  const trigger = page.getByRole('button', { name: 'Open Menu' });
  const triggerBox = await trigger.boundingBox();
  await trigger.click();
  const content = page.getByRole('menu');
  await expect(content).toBeVisible();
  const box = await content.boundingBox();
  if (!triggerBox || !box) {
    throw new Error(`expected both trigger and content boxes, got trigger=${triggerBox} content=${box}`);
  }
  const viewport = page.viewportSize();
  const debug = JSON.stringify({ triggerBox, box, viewport });
  expect(box.width, debug).toBeLessThan((viewport?.width ?? 1280) * 0.6);
  // Left-aligned under its trigger (DropdownMenu's `side="bottom"
  // align="start"` contract) -- generous tolerance for viewport-edge
  // clamping, just enough to catch full detachment to the viewport's own
  // left edge.
  expect(Math.abs(box.x - triggerBox.x), debug).toBeLessThan(200);
});

// REGRESSION (docs/backlog.md row 85): a raw DOM `.focus()` call landing
// directly on a menu item -- the exact thing a Playwright test driving
// focus programmatically does, and an assistive-technology focus-
// restoration path plausibly could -- used to close the whole menu and
// revert focus to the trigger, instead of simply focusing the item in
// place. Root cause: `DropdownMenuTrigger`'s own `onblur` decided "did
// focus leave the whole menu" by synchronously reading `ctx.focus.
// any_focused()`, a Rust-tracked roving-focus signal this crate's own
// click/keyboard code always updates *before* moving DOM focus -- but a
// raw external `.focus()` call updates nothing, so the guard read stale
// "nothing focused" state and closed. Confirmed RED on the unmodified
// tree: `trigger` ended up `data-state="closed"` after this exact
// sequence. Fixed by construction: `DropdownMenuContentRendered` now
// calls `use_outside_dismiss` (the same DOM-truth-based "did focus/a
// click land outside" check `ContextMenu`'s own root already used, never
// vulnerable to this because it asks `element.contains(event.target)`
// rather than trusting a signal), and the fragile `onblur` branch is
// removed. See `DropdownMenuSubTrigger`'s identical fix
// (`oracle/tier1-apg/menu-submenu.spec.ts`) for the sub-trigger instance
// of the same class.
test('a raw .focus() call on a plain item does not close the menu (row 85)', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
  const trigger = page.getByRole('button', { name: 'Open Menu' });
  // Mouse-click-opened, deliberately with NO keyboard navigation
  // afterward -- this leaves `ctx.focus` (the root's own roving-focus
  // collection) never populated, exactly the precondition the bug needs.
  await trigger.click();
  await expect(trigger).toHaveAttribute('data-state', 'open');
  const editItem = page.getByRole('menuitem', { name: 'Edit' });
  await expect(editItem).toBeVisible();

  await editItem.focus();
  // Give any (correctly, now, no-op) reactive close a moment to have
  // fired if it were going to -- avoids a false green from asserting
  // before a real regression's own close would have completed.
  await page.waitForTimeout(150);

  await expect(trigger, 'the whole menu must not close').toHaveAttribute('data-state', 'open');
  await expect(editItem, 'the item must simply be focused in place').toBeFocused();
});

// --- Checkable items: DropdownMenuCheckboxItem / DropdownMenuRadioGroup / DropdownMenuRadioItem ---
//
// shadcn's DropdownMenu has CheckboxItem and RadioGroup + RadioItem; this one
// had neither. They are one construction shared with ContextMenu and Menubar
// (`primitives/src/menu_item.rs`), graded here for DropdownMenu's own wiring:
// role/aria contract, click + keyboard behaviour, close semantics, roving
// focus and typeahead. The shared role contract is also graded across all
// three hosts by `oracle/tier1-apg/menu-roles.spec.ts`; the label/indicator
// geometry by `menu-indicator-gap.spec.ts`.
//
// The `checkboxes` and `radio_group` variants keep the menu open after a
// toggle (`close_on_select: false`) so several toggles fit in one visit -- the
// APG-optional behaviour for Space ("changes the state without closing the
// menu", menu-and-menubar-pattern.html, "Keyboard Interaction"). The `rtl`
// variant's "Pinned"/"Ascending" items use the primitive's default, Radix's
// `onSelect` default: selecting closes the menu.
const block = (variant: string) => `${BASE_URL}/component/block/?name=dropdown_menu&variant=${variant}&`;

test.describe('Checkable items', () => {
  test('checkbox items: menuitemcheckbox with an always-present aria-checked; a click toggles, a disabled item does not, the menu stays open', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    const trigger = page.getByRole('button', { name: 'Checkboxes' });
    await trigger.click();
    await expect(trigger).toHaveAttribute('data-state', 'open');

    const items = page.getByRole('menuitemcheckbox');
    await expect(items, 'three checkbox items, none of them a plain menuitem').toHaveCount(3);
    await expect(page.getByRole('menuitem'), 'only the "More options" sub-trigger is a plain menuitem').toHaveCount(1);
    const statusBar = page.getByRole('menuitemcheckbox', { name: 'Status Bar' });
    const activityBar = page.getByRole('menuitemcheckbox', { name: 'Activity Bar' });
    const panel = page.getByRole('menuitemcheckbox', { name: 'Panel' });

    // aria-checked is present on every item, "true"/"false", never absent.
    await expect(statusBar).toHaveAttribute('aria-checked', 'true');
    await expect(statusBar).toHaveAttribute('data-state', 'checked');
    await expect(activityBar).toHaveAttribute('aria-checked', 'false');
    await expect(activityBar).toHaveAttribute('data-state', 'unchecked');
    await expect(panel).toHaveAttribute('aria-checked', 'false');
    await expect(activityBar).toHaveAttribute('aria-disabled', 'true');
    await expect(statusBar).toHaveAttribute('aria-disabled', 'false');

    await panel.click();
    await expect(panel).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByText('Status bar true, panel true')).toBeVisible();
    await expect(trigger, 'close_on_select: false keeps the menu open').toHaveAttribute('data-state', 'open');

    await statusBar.click();
    await expect(statusBar).toHaveAttribute('aria-checked', 'false');
    await expect(page.getByText('Status bar false, panel true')).toBeVisible();

    // A disabled item cannot be toggled (`force`: Playwright otherwise waits
    // forever for an `aria-disabled` element to become actionable).
    await activityBar.click({ force: true });
    await expect(activityBar).toHaveAttribute('aria-checked', 'false');
    await expect(trigger).toHaveAttribute('data-state', 'open');
  });

  test('checkbox items: arrow keys skip the disabled item, Space and Enter toggle in place, typeahead reaches them', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    const trigger = page.getByRole('button', { name: 'Checkboxes' });
    await trigger.click();
    const statusBar = page.getByRole('menuitemcheckbox', { name: 'Status Bar' });
    const panel = page.getByRole('menuitemcheckbox', { name: 'Panel' });

    await page.keyboard.press('ArrowDown');
    await expect(statusBar).toBeFocused();
    // Status Bar(0) -> Activity Bar(1, disabled, skipped) -> Panel(2).
    await page.keyboard.press('ArrowDown');
    await expect(panel).toBeFocused();

    // APG (Optional): Space on a menuitemcheckbox "changes the state without
    // closing the menu" -- what close_on_select: false gives.
    await page.keyboard.press('Space');
    await expect(panel).toHaveAttribute('aria-checked', 'true');
    await expect(trigger).toHaveAttribute('data-state', 'open');
    await expect(panel, 'focus stays on the toggled item').toBeFocused();
    await page.keyboard.press('Enter');
    await expect(panel).toHaveAttribute('aria-checked', 'false');
    await expect(trigger).toHaveAttribute('data-state', 'open');

    // Typeahead: a checkbox item is a target when it has a text_value...
    await page.keyboard.press('s');
    await expect(statusBar, "typeahead 's' wraps past Panel to Status Bar").toBeFocused();
    await page.waitForTimeout(1100);
    await page.keyboard.press('p');
    await expect(panel, "typeahead 'p' reaches Panel").toBeFocused();
    // ...and a disabled one is never a candidate.
    await page.waitForTimeout(1100);
    await page.keyboard.press('a');
    await expect(panel, "typeahead 'a' must not land on the disabled Activity Bar").toBeFocused();
  });

  test('a checkbox item inside a submenu registers in the submenu, not the root: keyboard opens it, Space toggles it in place, the root keeps its own roving focus', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    const trigger = page.getByRole('button', { name: 'Checkboxes' });
    await trigger.click();
    const statusBar = page.getByRole('menuitemcheckbox', { name: 'Status Bar' });
    const subTrigger = page.getByRole('menuitem', { name: 'More options' });
    const wordWrap = page.getByRole('menuitemcheckbox', { name: 'Word Wrap' });

    // The 1st ArrowDown lands on Status Bar(0); then Activity Bar(1, disabled,
    // skipped) -> Panel(2) -> More options(3): 3 presses in all.
    for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowDown');
    await expect(subTrigger).toBeFocused();
    await page.keyboard.press('ArrowRight');
    // The submenu's first item is `Word Wrap` (its own index 0). Had it
    // registered in the root collection it would have collided with
    // `Status Bar` (also index 0) and focus would not have landed on it.
    await expect(wordWrap).toBeVisible();
    await expect(wordWrap).toBeFocused();
    await expect(wordWrap).toHaveAttribute('aria-checked', 'false');

    await page.keyboard.press('Space');
    await expect(wordWrap).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByText('word wrap true')).toBeVisible();
    await expect(trigger, 'close_on_select: false keeps the whole menu open').toHaveAttribute('data-state', 'open');
    await expect(wordWrap, 'focus stays on the toggled item').toBeFocused();

    // Escape/ArrowLeft hands focus back to the sub-trigger; the root's own
    // roving focus is intact (ArrowDown from the last item wraps to Status Bar).
    await page.keyboard.press('ArrowLeft');
    await expect(subTrigger).toBeFocused();
    await expect(wordWrap).toBeHidden();
    await page.keyboard.press('ArrowDown');
    await expect(statusBar).toBeFocused();
  });

  test('a checkbox item inside a submenu: a click toggles it, and by default it closes the whole menu tree', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    const trigger = page.getByRole('button', { name: 'Checkboxes' });
    await trigger.click();
    const subTrigger = page.getByRole('menuitem', { name: 'More options' });
    await subTrigger.click();
    const wordWrap = page.getByRole('menuitemcheckbox', { name: 'Word Wrap' });
    const minimap = page.getByRole('menuitemcheckbox', { name: 'Minimap' });
    await expect(wordWrap).toBeVisible();

    await wordWrap.click();
    await expect(wordWrap).toHaveAttribute('aria-checked', 'true');
    await expect(trigger).toHaveAttribute('data-state', 'open');

    // `Minimap` uses the primitive's default: choosing it closes the menu --
    // the entire tree, submenu included, like `DropdownMenuSubItem`.
    await minimap.click();
    await expect(trigger, 'the default close takes the whole tree with it').toHaveAttribute('data-state', 'closed');
    await expect(page.getByRole('menu')).toHaveCount(0);
    await expect(page.getByText('minimap true')).toBeVisible();
  });

  test('radio group: role=group named by its label, exactly one item checked, a choice moves the check, the disabled item cannot be chosen', async ({ page }) => {
    await gotoHydrated(page, block('radio_group'));
    const trigger = page.getByRole('button', { name: 'Radio Group' });
    await trigger.click();
    await expect(trigger).toHaveAttribute('data-state', 'open');

    const group = page.getByRole('group', { name: 'Panel Position' });
    await expect(group, 'the group is named by its heading via aria-labelledby').toBeVisible();
    const radios = group.getByRole('menuitemradio');
    await expect(radios).toHaveCount(3);
    const top = group.getByRole('menuitemradio', { name: 'Top' });
    const bottom = group.getByRole('menuitemradio', { name: 'Bottom' });
    const right = group.getByRole('menuitemradio', { name: 'Right' });
    const checkedCount = () => group.locator('[role="menuitemradio"][aria-checked="true"]').count();

    await expect(top).toHaveAttribute('aria-checked', 'false');
    await expect(bottom).toHaveAttribute('aria-checked', 'true');
    await expect(right).toHaveAttribute('aria-checked', 'false');
    await expect(right).toHaveAttribute('aria-disabled', 'true');
    expect(await checkedCount(), 'exactly one radio is checked').toBe(1);

    await top.click();
    await expect(top).toHaveAttribute('aria-checked', 'true');
    await expect(bottom).toHaveAttribute('aria-checked', 'false');
    await expect(page.getByText('Panel position top')).toBeVisible();
    expect(await checkedCount(), 'still exactly one radio checked after a choice').toBe(1);
    await expect(trigger, 'close_on_select: false keeps the menu open').toHaveAttribute('data-state', 'open');

    await right.click({ force: true });
    await expect(right, 'a disabled radio cannot be chosen').toHaveAttribute('aria-checked', 'false');
    await expect(top).toHaveAttribute('aria-checked', 'true');
  });

  test('radio group: arrow keys skip the disabled radio, Space and Enter choose it, typeahead falls back to the value', async ({ page }) => {
    await gotoHydrated(page, block('radio_group'));
    const trigger = page.getByRole('button', { name: 'Radio Group' });
    await trigger.click();
    const top = page.getByRole('menuitemradio', { name: 'Top' });
    const bottom = page.getByRole('menuitemradio', { name: 'Bottom' });

    await page.keyboard.press('ArrowDown');
    await expect(top).toBeFocused();
    await page.keyboard.press('Space');
    await expect(top).toHaveAttribute('aria-checked', 'true');
    await expect(bottom).toHaveAttribute('aria-checked', 'false');
    await expect(trigger).toHaveAttribute('data-state', 'open');

    await page.keyboard.press('ArrowDown');
    await expect(bottom).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(bottom).toHaveAttribute('aria-checked', 'true');
    await expect(top).toHaveAttribute('aria-checked', 'false');

    // Bottom(1) -> Right(2, disabled, skipped) -> wraps to Top(0).
    await page.keyboard.press('ArrowDown');
    await expect(top).toBeFocused();

    // A radio item has no text_value here: typeahead falls back to its value.
    await page.keyboard.press('b');
    await expect(bottom, "typeahead 'b' reaches Bottom via its value").toBeFocused();
    await page.waitForTimeout(1100);
    await page.keyboard.press('r');
    await expect(bottom, "typeahead 'r' must not land on the disabled Right").toBeFocused();
  });

  test('by default (Radix onSelect default) choosing a checkable item closes the menu and the new state is kept', async ({ page }) => {
    await gotoHydrated(page, block('rtl'));
    const trigger = page.getByRole('button', { name: 'Launch Menu' });
    await trigger.click();
    await expect(trigger).toHaveAttribute('data-state', 'open');

    const pinned = page.getByRole('menuitemcheckbox', { name: 'Pinned' });
    await expect(pinned).toHaveAttribute('aria-checked', 'true');
    await pinned.click();
    await expect(trigger, 'a checkbox item closes the menu by default').toHaveAttribute('data-state', 'closed');
    await expect(trigger, 'focus returns to the trigger like any other close').toBeFocused();

    await trigger.click();
    await expect(pinned, 'the (controlled) state survived the close').toHaveAttribute('aria-checked', 'false');

    // Keyboard: Enter on a radio item chooses it and closes the menu. End
    // lands on the last item, "Descending".
    await page.keyboard.press('End');
    const descending = page.getByRole('menuitemradio', { name: 'Descending' });
    await expect(descending).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(trigger).toHaveAttribute('data-state', 'closed');

    await trigger.click();
    await expect(descending).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByRole('menuitemradio', { name: 'Ascending' })).toHaveAttribute('aria-checked', 'false');
  });
});

test.describe('Axe automated scan', () => {
  test('loaded (menu closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Open Menu' })).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('menu open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    await page.getByRole('button', { name: 'Open Menu' }).click();
    await expect(page.getByRole('menu')).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: menu open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  // Checkable items: role="menuitemcheckbox"/"menuitemradio" need aria-checked
  // and a menu/group context; a radio group needs to be a real `group`.
  test('menu with checkbox items open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    await page.getByRole('button', { name: 'Checkboxes' }).click();
    await expect(page.getByRole('menuitemcheckbox', { name: 'Status Bar' })).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: checkboxes open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('menu with a radio group open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    await page.getByRole('button', { name: 'Radio Group' }).click();
    await expect(page.getByRole('menuitemradio', { name: 'Top' })).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: radio group open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// ---------------------------------------------------------------------------
// `DropdownMenuSub { open_on_hover: false }` -- click-activated submenu
// (preview/src/components/dropdown_menu/variants/click_only_submenu/mod.rs;
// renders alongside `main` on this page under its own labels). The default
// hover-open / hover-leave-close contract is pinned, unchanged, by
// `oracle/tier1-apg/menu-submenu.spec.ts`'s "hover:" tests on `main`.
// ---------------------------------------------------------------------------
test.describe('DropdownMenuSub open_on_hover: false (click activation)', () => {
  const HOVER_SETTLE_MS = 1000; // >> SUBMENU_OPEN_INTENT_DELAY / SUBMENU_CLOSE_GRACE_DELAY (200ms each)

  async function openMenu(page: import('@playwright/test').Page) {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`, { timeout: 20 * 60 * 1000 });
    const trigger = page.getByRole('button', { name: 'Actions', exact: true });
    await trigger.click();
    await expect(trigger).toHaveAttribute('data-state', 'open');
    return {
      trigger,
      sub: page.getByRole('menuitem', { name: 'Export as' }),
      submenu: page.getByRole('menu', { name: 'Export as' }),
    };
  }

  test('hovering the sub-trigger for a second does not open it; a click does', async ({ page }) => {
    const { sub, submenu } = await openMenu(page);
    await expect(sub).toHaveAttribute('aria-expanded', 'false');

    await sub.hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(submenu).toHaveCount(0);

    await sub.click();
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(submenu).toBeVisible();
    // A click (unlike a hover) lands focus in the submenu.
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();
  });

  test('the pointer leaving a click-opened submenu does not close it', async ({ page }) => {
    const { sub, submenu } = await openMenu(page);
    await sub.click();
    await expect(submenu).toBeVisible();

    // Onto a sibling item of the parent menu -- with hover on, this closes
    // the submenu after SUBMENU_CLOSE_GRACE_DELAY.
    await page.getByRole('menuitem', { name: 'Share' }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(submenu).toBeVisible();

    // And the pointer resting inside then leaving the submenu content does not close it either.
    await page.getByRole('menuitem', { name: 'Plain text' }).hover();
    await page.getByRole('menuitem', { name: 'Print' }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
  });

  test('keyboard is unchanged: ArrowRight / Enter / Space open, ArrowLeft / Escape close', async ({ page }) => {
    const { trigger, sub } = await openMenu(page);
    await page.keyboard.press('ArrowDown'); // Share
    await page.keyboard.press('ArrowDown'); // Export as
    await expect(sub).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();

    await page.keyboard.press('ArrowLeft');
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(sub).toBeFocused();
    await expect(trigger).toHaveAttribute('data-state', 'open');

    await page.keyboard.press('Enter');
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(sub).toBeFocused();

    await page.keyboard.press('Space');
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();
  });

  test('selecting a sub-item still closes the whole menu', async ({ page }) => {
    const { trigger, sub } = await openMenu(page);
    await sub.click();
    await page.getByRole('menuitem', { name: 'Plain text' }).click();
    await expect(trigger).toHaveAttribute('data-state', 'closed');
    await expect(page.getByText('Chosen: text')).toBeVisible();
  });

  test('default (open_on_hover: true) is unchanged: hovering More tools still opens it', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Open Menu' }).click();
    const sub = page.getByRole('menuitem', { name: 'More tools' });
    await sub.hover();
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
  });
});

// ---------------------------------------------------------------------------
// Open/close fade vs. the anchor's centring transform (2026-10-05). An anchored overlay centres itself
// with `transform: translateX(-50%)` (top_layer.rs's engine stylesheet); a `@keyframes` that sets
// `transform` REPLACES it for as long as it runs (the date picker played its fade 144px off-centre, the
// colour picker 133px). `DropdownMenuContent` sets no `data-side`, so it has no centring to lose today -- the same
// shape, latent -- so this probes the rendered keyframes instead of sampling a position that could not
// move anyway: `assert-anchor-transform.ts` supplies an inline `transform`, seeks the element's own
// CSS animation to its start/midpoint/end, and asserts the transform survives while `scale`/`translate`
// do the moving. Source-level guard: scripts/check-anchored-keyframes.sh.
// ---------------------------------------------------------------------------
test.describe('Open/close animation', () => {
  test('the fade animates translate/scale and leaves transform alone (root content and submenu, open and close)', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Open Menu' }).click();
    const content = page.locator('.dx-dropdown-menu-content[data-state="open"]:not(.dx-dropdown-menu-sub-content)').first();
    await expect(content).toBeVisible();
    await expectFadeKeepsTransform(content, 'dropdown menu content, open');
    await expectFadeKeepsTransform(content, 'dropdown menu content, close', { 'data-state': 'closed' });

    // The submenu shares the keyframes but is its own element.
    const sub = page.getByRole('menuitem', { name: 'More tools' });
    await sub.hover();
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    const subContent = page.locator('.dx-dropdown-menu-sub-content[data-state="open"]').first();
    await expect(subContent).toBeVisible();
    await expectFadeKeepsTransform(subContent, 'dropdown submenu content, open');
    await expectFadeKeepsTransform(subContent, 'dropdown submenu content, close', { 'data-state': 'closed' });
  });
});
