import { test, expect, type Locator, type Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

const URL = 'http://127.0.0.1:8080/component/?name=resizable&';

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
