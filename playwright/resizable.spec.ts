import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
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

/** True when the topmost element at a viewport point is the handle itself, its `::after` hit area
 * (a pseudo-element hit-tests as its originating element) or one of its descendants (the grip) --
 * i.e. a pointerdown there bubbles to the handle's own drag listener. */
async function hitsHandle(handle: Locator, x: number, y: number) {
  return handle.evaluate((el, [px, py]) => el.contains(document.elementFromPoint(px, py)), [x, y] as const);
}

async function dragFromPoint(page: Page, x: number, y: number, dx: number, dy: number) {
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 10 });
  await page.mouse.up();
}

test.describe('Hit area: the whole handle drags, not just the 1px line', () => {
  // Owner report: "the handle must be draggable, not just the line". The 1px `.dx-resizable-handle`
  // div owns the drag listener; its grip (12x16) used to be `pointer-events: none` with only a 4px
  // `::after` strip behind it, so ~4px either side of the grip -- and the same few px beside the
  // bare line -- fell through to the panels and started nothing. shadcn's grip is a plain child
  // (its pointerdown bubbles to the handle) inside a handle with a wider invisible hit area.
  const GRIP_INSET = 1; // px inside the grip's own edge: visibly "on the grip", far from the 1px line
  const BESIDE_LINE = 4; // px from the line's centre: inside the hit area, outside the old 4px strip

  test('grabbing the grip anywhere on its footprint drags the handle, not just its centre line', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Sidebar' });
    const grip = handle.locator('.dx-resizable-handle-grip');
    const left = await primaryPanel(page, 'Sidebar');

    for (const side of ['left', 'right'] as const) {
      const gripBox = await grip.boundingBox();
      if (!gripBox) throw new Error('grip has no bounding box');
      const x = side === 'left' ? gripBox.x + GRIP_INSET : gripBox.x + gripBox.width - GRIP_INSET;
      const y = gripBox.y + gripBox.height / 2;
      expect(await hitsHandle(handle, x, y), `the grip's ${side} edge hit-tests to the handle`).toBe(true);

      const before = (await left.boundingBox())!.width;
      await dragFromPoint(page, x, y, 40, 0);
      // Polled: pointermoves reach the Rust drag loop through an async eval round trip, which lags
      // when the machine is busy.
      await expect
        .poll(async () => (await left.boundingBox())!.width - before, { message: `dragging from the grip's ${side} edge moves the boundary with the pointer` })
        .toBeGreaterThan(30);
    }
  });

  test('grabbing the vertical (nested) grip by its top or bottom edge drags the row handle', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Top' });
    const grip = handle.locator('.dx-resizable-handle-grip');
    const top = await primaryPanel(page, 'Top');

    for (const side of ['top', 'bottom'] as const) {
      const gripBox = await grip.boundingBox();
      if (!gripBox) throw new Error('grip has no bounding box');
      const x = gripBox.x + gripBox.width / 2;
      const y = side === 'top' ? gripBox.y + GRIP_INSET : gripBox.y + gripBox.height - GRIP_INSET;
      expect(await hitsHandle(handle, x, y), `the grip's ${side} edge hit-tests to the handle`).toBe(true);

      const before = (await top.boundingBox())!.height;
      await dragFromPoint(page, x, y, 0, 20);
      await expect
        .poll(async () => (await top.boundingBox())!.height - before, { message: `dragging from the grip's ${side} edge moves the boundary with the pointer` })
        .toBeGreaterThan(14);
    }
  });

  test('a few px beside the 1px line, clear of the grip, still drags -- and well outside the hit area does not', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Sidebar' });
    const left = await primaryPanel(page, 'Sidebar');
    for (const side of [-1, 1]) {
      // Re-measured per pass: the first drag moves the handle.
      const box = await handle.boundingBox();
      if (!box) throw new Error('handle has no bounding box');
      const x = box.x + box.width / 2 + side * BESIDE_LINE;
      const y = box.y + 24; // near the top of the handle: well clear of the vertically-centred grip
      const label = side < 0 ? 'left of' : 'right of';
      expect(await hitsHandle(handle, x, y), `${BESIDE_LINE}px ${label} the line hit-tests to the handle`).toBe(true);
      const before = (await left.boundingBox())!.width;
      await dragFromPoint(page, x, y, 30, 0);
      await expect
        .poll(async () => (await left.boundingBox())!.width - before, { message: `dragging from ${BESIDE_LINE}px ${label} the line` })
        .toBeGreaterThan(22);
    }

    // The hit area is comfortable, not a giant: 12px out the pointer is back on the panel.
    const fresh = (await handle.boundingBox())!;
    const outY = fresh.y + 24;
    expect(await hitsHandle(handle, fresh.x + fresh.width / 2 - 12, outY)).toBe(false);
    expect(await hitsHandle(handle, fresh.x + fresh.width / 2 + 12, outY)).toBe(false);
  });

  test('the grip shows the resize cursor, like the line does', async ({ page }) => {
    await page.goto(URL, { timeout: 20 * 60 * 1000 });
    const sidebar = page.getByRole('separator', { name: 'Sidebar' });
    await expect(sidebar.locator('.dx-resizable-handle-grip')).toHaveCSS('cursor', 'col-resize');
    const top = page.getByRole('separator', { name: 'Top' });
    await expect(top.locator('.dx-resizable-handle-grip')).toHaveCSS('cursor', 'row-resize');
    // ...and the cursor the pointer actually gets at the grip's edge is the handle's.
    const grip = (await sidebar.locator('.dx-resizable-handle-grip').boundingBox())!;
    const cursorAt = (x: number, y: number) =>
      page.evaluate(([px, py]) => getComputedStyle(document.elementFromPoint(px, py)!).cursor, [x, y] as const);
    expect(await cursorAt(grip.x + GRIP_INSET, grip.y + grip.height / 2)).toBe('col-resize');
  });

  test.describe('Touch', () => {
    // `hasTouch` also flips `(pointer: coarse)`, which doubles the hit area (a finger is blunter
    // than a mouse). Chromium additionally fuzzes a touch target by a few px, which is why the
    // grip's edge alone never reproduced the bug with a finger -- the 10px case does (it is
    // outside both the old 4px strip and that fuzz, inside the new 24px coarse hit area).
    test.use({ hasTouch: true });

    for (const [name, offsetFromLine, onGrip] of [
      ['the grip\'s edge', 5, true],
      ['10px beside the line, clear of the grip', 10, false],
    ] as const) {
      test(`a finger on ${name} drags the handle (real touch events)`, async ({ page }) => {
        await page.goto(URL, { timeout: 20 * 60 * 1000 });
        const handle = page.getByRole('separator', { name: 'Sidebar' });
        await handle.scrollIntoViewIfNeeded();
        await expect.poll(() => page.evaluate(() => matchMedia('(pointer: coarse)').matches)).toBe(true);
        const left = await primaryPanel(page, 'Sidebar');
        const box = (await handle.boundingBox())!;
        const gripBox = (await handle.locator('.dx-resizable-handle-grip').boundingBox())!;
        const x = box.x + box.width / 2 - offsetFromLine;
        const y = onGrip ? gripBox.y + gripBox.height / 2 : box.y + 24;
        const before = (await left.boundingBox())!.width;

        const cdp = await page.context().newCDPSession(page);
        const touch = (type: string, tx: number) =>
          cdp.send('Input.dispatchTouchEvent', { type, touchPoints: type === 'touchEnd' ? [] : [{ x: tx, y, id: 1 }] });
        await touch('touchStart', x);
        for (let s = 1; s <= 8; s++) await touch('touchMove', x + (40 * s) / 8);
        await touch('touchEnd', 0);

        await expect.poll(async () => (await left.boundingBox())!.width - before).toBeGreaterThan(30);
      });
    }
  });
});

test.describe('RTL', () => {
  // Under `dir="rtl"` a horizontal group's panels mirror, so the primary pane sits on the RIGHT of
  // its handle. The arrow keys already flipped for that; the pointer drag did not, so the handle
  // slid the OPPOSITE way to the pointer. Both now share `primary_delta` (primitives/src/resizable.rs).
  test('dragging the handle moves it with the pointer, from the line or a few px beside it', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/block?name=resizable&variant=rtl&`, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Divider' });
    await expect(handle).toBeVisible();

    for (const offset of [0, 4, -4]) {
      for (const dx of [30, -30]) {
        const box = (await handle.boundingBox())!;
        const x = box.x + box.width / 2 + offset;
        const y = box.y + 20; // clear of the (absent here) grip; inside the hit area at +/-4px
        expect(await hitsHandle(handle, x, y), `${offset}px from the line hit-tests to the handle`).toBe(true);
        await dragFromPoint(page, x, y, dx, 0);
        await expect
          .poll(async () => Math.abs((await handle.boundingBox())!.x - box.x - dx), { message: `a ${dx}px pointer drag (from ${offset}px) moves the handle the same way, by the same amount` })
          .toBeLessThan(2);
      }
    }
  });

  test('keyboard still moves the divider in the physical direction of the arrow key', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/block?name=resizable&variant=rtl&`, { timeout: 20 * 60 * 1000 });
    const handle = page.getByRole('separator', { name: 'Divider' });
    await handle.focus();
    const before = (await handle.boundingBox())!.x;
    await page.keyboard.press('ArrowRight');
    await expect.poll(async () => (await handle.boundingBox())!.x).toBeGreaterThan(before);
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');
    await expect.poll(async () => (await handle.boundingBox())!.x).toBeLessThan(before);
  });
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
    // `--dx-radius-lg` (the demo's radius) is 10px since row 111's shadcn radius scale (was 8px).
    await expect(container).toHaveCSS('border-radius', '10px');

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
