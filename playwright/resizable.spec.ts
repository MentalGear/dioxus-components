import { test, expect, type Locator, type Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';

const URL = `${BASE_URL}/component/?name=resizable&`;

/** The panel a handle's `aria-controls` names -- its "primary pane". */
async function primaryPanel(page: Page, handleName: string) {
  const handle = page.getByRole('separator', { name: handleName });
  const id = await handle.getAttribute('aria-controls');
  if (!id) throw new Error(`${handleName} handle has no aria-controls`);
  return page.locator(`#${id}`);
}

/** The handle's other (following) neighbor -- panels and handles are direct,
 * ordered siblings inside their `ResizablePanelGroup` (see
 * primitives/src/resizable.rs's doc comment on ResizablePanel/ResizableHandle
 * markup). */
function secondaryPanel(handle: Locator) {
  return handle.locator('xpath=following-sibling::*[1]');
}

async function dragHandleBy(page: Page, handle: Locator, dx: number, dy: number) {
  const box = await handle.boundingBox();
  if (!box) throw new Error('handle has no bounding box');
  const startX = box.x + box.width / 2;
  const startY = box.y + box.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + dx, startY + dy, { steps: 10 });
  await page.mouse.up();
}

test('dragging the Sidebar handle resizes both adjacent panels while the group\'s total width stays constant', async ({ page }) => {
  await page.goto(URL, { timeout: 20 * 60 * 1000 });
  const handle = page.getByRole('separator', { name: 'Sidebar' });
  const left = await primaryPanel(page, 'Sidebar');
  const right = secondaryPanel(handle);

  const beforeLeft = await left.boundingBox();
  const beforeRight = await right.boundingBox();
  if (!beforeLeft || !beforeRight) throw new Error('panel has no bounding box');
  const totalBefore = beforeLeft.width + beforeRight.width;

  await expect(handle).toHaveAttribute('data-state', 'idle');
  await dragHandleBy(page, handle, 80, 0);
  await expect(handle).toHaveAttribute('data-state', 'idle');

  const afterLeft = await left.boundingBox();
  const afterRight = await right.boundingBox();
  if (!afterLeft || !afterRight) throw new Error('panel has no bounding box');

  expect(afterLeft.width).toBeGreaterThan(beforeLeft.width);
  expect(afterRight.width).toBeLessThan(beforeRight.width);
  expect(afterLeft.width + afterRight.width).toBeCloseTo(totalBefore, 0);

  // Dragging left (negative dx) must move it back the other way.
  await dragHandleBy(page, handle, -40, 0);
  const afterLeft2 = await left.boundingBox();
  if (!afterLeft2) throw new Error('panel has no bounding box');
  expect(afterLeft2.width).toBeLessThan(afterLeft.width);
});

test('the nested vertical group\'s handle drags independently of the outer horizontal group', async ({ page }) => {
  await page.goto(URL, { timeout: 20 * 60 * 1000 });
  const outerHandle = page.getByRole('separator', { name: 'Sidebar' });
  const sidebar = await primaryPanel(page, 'Sidebar');
  const innerHandle = page.getByRole('separator', { name: 'Top' });
  const top = await primaryPanel(page, 'Top');
  const bottom = secondaryPanel(innerHandle);

  const sidebarBefore = await sidebar.boundingBox();
  const topBefore = await top.boundingBox();
  const bottomBefore = await bottom.boundingBox();
  if (!sidebarBefore || !topBefore || !bottomBefore) {
    throw new Error('panel has no bounding box');
  }
  const totalBefore = topBefore.height + bottomBefore.height;

  await dragHandleBy(page, innerHandle, 0, 30);

  const topAfter = await top.boundingBox();
  const bottomAfter = await bottom.boundingBox();
  const sidebarAfter = await sidebar.boundingBox();
  if (!topAfter || !bottomAfter || !sidebarAfter) {
    throw new Error('panel has no bounding box');
  }

  expect(topAfter.height).toBeGreaterThan(topBefore.height);
  expect(bottomAfter.height).toBeLessThan(bottomBefore.height);
  expect(topAfter.height + bottomAfter.height).toBeCloseTo(totalBefore, 0);

  // The nested group's own drag must not move the OUTER boundary at all.
  expect(sidebarAfter.width).toBeCloseTo(sidebarBefore.width, 0);
  await expect(outerHandle).toHaveAttribute('aria-valuenow', '50');
});

test.describe('Appearance (shadcn/ui v4 translation)', () => {
  // Translates shadcn/ui v4's Resizable (react-resizable-panels) look onto this repo's own
  // tokens (dev-docs/backlog.md's cheap-primitives-wave follow-up) -- a `w-px` handle with a
  // `h-4 w-3` grip, replacing the old 4px-wide handle and 16x24px grip. Both values below are
  // exact per-pixel translations of shadcn's own `w-px`/`h-4 w-3` (this repo's --dx-space-3/-4
  // tokens), not approximations.
  test('the handle is a 1px hairline with a ~12x16px grip', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Sidebar' });
    await expect(handle).toHaveCSS('width', '1px');

    const grip = handle.locator('.dx-resizable-handle-grip');
    await expect(grip).toHaveCSS('width', '12px');
    await expect(grip).toHaveCSS('height', '16px');
  });

  test("the demo's container is a bordered, rounded, ~450px-wide box", async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Sidebar' });
    // The handle's own direct parent is the outer ResizablePanelGroup div (panels and handles
    // are direct siblings inside it -- see primitives/src/resizable.rs's markup doc comment).
    const container = handle.locator('xpath=..');
    await expect(container).toHaveCSS('border-radius', '8px');

    // shadcn: `max-w-md ... md:min-w-[450px]` -- before this translation the group had no width
    // cap/floor at all and rendered at its own content-hugging width (~269px measured against
    // this fixture), well short of shadcn's ~450px.
    const box = await container.boundingBox();
    if (!box) throw new Error('container has no bounding box');
    expect(box.width).toBeGreaterThanOrEqual(440);
  });
});

test.describe('Axe automated scan', () => {
  // Resizable has no overlay/expand interaction to reach a second state --
  // loaded (default sizes) is the only meaningful scan, per the convention
  // axe.ts's header documents ("A component with no such state ... scans
  // once, at load"), matching slider.spec.ts's own identical single-state
  // scan.
  test('loaded has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('separator', { name: 'Sidebar' })).toBeVisible();
    await expectNoAxeViolations(page, 'resizable: loaded', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
