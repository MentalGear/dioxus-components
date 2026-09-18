import { test, expect, type Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

const URL = 'http://127.0.0.1:8080/component/?name=navigation_menu&';
const GOTO = { timeout: 20 * 60 * 1000 }; // Increase timeout to 20 minutes

/**
 * This demo page's own `NavigationMenu` -- scoped by its `aria_label`
 * (`preview/src/components/navigation_menu/variants/main/mod.rs`), not
 * `page.getByRole(...)` directly against the whole page. The site's own
 * persistent chrome (header navbar, footer -- `preview/src/main.rs`,
 * out of this lane's ownership) renders its own "Docs" links on every
 * route, and would otherwise make `page.getByRole('link', { name: 'Docs' })`
 * resolve to more than one element (confirmed by live reproduction: 3
 * matches page-wide -- header, footer, this demo -- vs. exactly 1 once
 * scoped to this locator). Every test below goes through this helper
 * rather than repeating the same page-wide-vs-scoped mistake per test.
 */
function nav(page: Page) {
  return page.getByRole('navigation', { name: 'Component navigation menu' });
}

test('hover opens the panel', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.hover();
  // primitives/src/navigation_menu.rs's HOVER_OPEN_INTENT_DELAY (150ms) --
  // expect()'s own auto-retry tolerates the delay without this test
  // asserting its exact length (a tight "still closed immediately after
  // hover" window would be flaky under load, not a real methodology
  // improvement -- see dev-docs/dx-serve-hot-reload.md's own caution
  // about test-methodology mistakes producing false alarms).
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
});

test('opening one item closes the other', async ({ page }) => {
  await page.goto(URL, GOTO);
  const gettingStarted = nav(page).getByRole('button', { name: 'Getting started' });
  const components = nav(page).getByRole('button', { name: 'Components' });

  await gettingStarted.hover();
  await expect(gettingStarted).toHaveAttribute('aria-expanded', 'true');

  await components.hover();
  await expect(components).toHaveAttribute('aria-expanded', 'true');
  await expect(gettingStarted).toHaveAttribute('aria-expanded', 'false');
});

test('pointer leaving the trigger and its content closes the panel', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });

  await trigger.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');

  // Move the pointer well outside both the trigger and its (top-layer
  // promoted) content -- the page's own h1 is a safe, always-present target.
  await page.getByRole('heading', { level: 1 }).hover();
  // primitives/src/navigation_menu.rs's HOVER_CLOSE_GRACE_DELAY (300ms).
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
});

test('pointer entering the content cancels the close', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });

  await trigger.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');

  // The disclosed content is promoted to the top layer for painting on the
  // web arm (`primitives/src/navigation_menu.rs`'s module doc, "Top
  // layer") but never reparented, so it stays inside `nav(page)`'s scope.
  const featuredLink = nav(page).getByRole('link', { name: 'Component Library' });
  await expect(featuredLink).toBeVisible();
  // Leave the trigger for the content itself (not somewhere else) --
  // NavigationMenuTrigger/NavigationMenuContent's own onmouseenter cancels
  // the pending close (see this primitive's module doc, "Hover intent").
  await featuredLink.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
});

test('the plain top-level link is a real link', async ({ page }) => {
  await page.goto(URL, GOTO);
  const docsLink = nav(page).getByRole('link', { name: 'Docs' });
  await expect(docsLink).toHaveAttribute('href', '/docs');
  await docsLink.click();
  await expect(page).toHaveURL(/\/docs/);
});

test.describe('Axe automated scan', () => {
  test('loaded (panel closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(nav(page).getByRole('button', { name: 'Getting started' })).toBeVisible();
    await expectNoAxeViolations(page, 'navigation_menu: loaded', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });

  test('panel open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO);
    const trigger = nav(page).getByRole('button', { name: 'Getting started' });
    await trigger.click();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await expectNoAxeViolations(page, 'navigation_menu: open', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
