import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=message&`;
const GOTO = { timeout: 20 * 60 * 1000 };

// Scope to a demo frame. The component page also renders each demo's own source
// in a syntax-highlighted code viewer, so every visible string appears twice.
const main = (page: Page) => page.locator("#component-preview-frame").first();
const variant = (page: Page, name: string) => page.locator(`#component-preview-frame-${name}`).first();
// The component's stylesheet is linked after mount: wait until it has applied
// before any test reads a computed style or a position.
const styled = (page: Page) =>
  expect
    .poll(() => main(page).locator('[data-slot="message"]').first().evaluate((el) => getComputedStyle(el).display), {
      timeout: 60_000,
    })
    .toBe("flex");

async function open(page: Page) {
  await page.goto(URL, GOTO);
  await styled(page);
}

test("the main demo is a conversation of rows alternating between the two sides", async ({ page }) => {
  await open(page);
  const rows = main(page).locator('[data-slot="message"]');
  await expect(rows).toHaveCount(4);
  await expect(rows.nth(0)).toHaveAttribute("data-align", "end");
  await expect(rows.nth(1)).toHaveAttribute("data-align", "start");
  await expect(rows.nth(2)).toHaveAttribute("data-align", "end");
  await expect(rows.nth(3)).toHaveAttribute("data-align", "start");
  await expect(rows.nth(0)).toContainText("Deploying to prod real quick.");
  await expect(rows.nth(1)).toContainText("It's 4:55 PM. On a Friday.");
  // A message is presentational layout: no role of its own.
  expect(await rows.evaluateAll((els) => els.filter((e) => e.hasAttribute("role")).length)).toBe(0);
});

test("align=end mirrors the row and the bubble hugs the end; start hugs the start", async ({ page }) => {
  await open(page);
  const rows = main(page).locator('[data-slot="message"]');
  expect(await rows.nth(0).evaluate((el) => getComputedStyle(el).flexDirection)).toBe("row-reverse");
  expect(await rows.nth(1).evaluate((el) => getComputedStyle(el).flexDirection)).toBe("row");
  const edge = await main(page).evaluate((frame) => {
    const rows = Array.from(frame.querySelectorAll<HTMLElement>('[data-slot="message"]'));
    const gap = (row: HTMLElement) => {
      const r = row.getBoundingClientRect();
      const b = row.querySelector<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
      return { fromStart: b.left - r.left, fromEnd: r.right - b.right };
    };
    return { end: gap(rows[0]), start: gap(rows[1]) };
  });
  expect(edge.end.fromEnd).toBeLessThan(1);
  expect(edge.end.fromStart).toBeGreaterThan(20);
  expect(edge.start.fromStart).toBeLessThan(1);
  expect(edge.start.fromEnd).toBeGreaterThan(20);
});

test("the row mirrors in RTL: a start message hugs the right edge", async ({ page }) => {
  await open(page);
  const edge = await main(page).evaluate((frame) => {
    const row = frame.querySelectorAll<HTMLElement>('[data-slot="message"]')[1];
    row.setAttribute("dir", "rtl");
    const r = row.getBoundingClientRect();
    const b = row.querySelector<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
    return { fromRight: r.right - b.right, fromLeft: b.left - r.left };
  });
  expect(edge.fromRight).toBeLessThan(1);
  expect(edge.fromLeft).toBeGreaterThan(20);
});

test("avatar: named images anchored to the bottom of the row", async ({ page }) => {
  await open(page);
  const demo = variant(page, "avatar");
  await expect(demo.getByRole("img", { name: "Evil Rabbit" })).toBeVisible();
  await expect(demo.getByRole("img", { name: "Me" })).toBeVisible();
  const geo = await demo.evaluate((frame) => {
    // The second row has a tall (wrapping) bubble: the avatar sits at its bottom.
    const row = frame.querySelectorAll<HTMLElement>('[data-slot="message"]')[1];
    const avatar = row.querySelector<HTMLElement>('[data-slot="message-avatar"]')!.getBoundingClientRect();
    const bubble = row.querySelector<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
    return { avatarBottom: avatar.bottom, bubbleBottom: bubble.bottom, avatarWidth: avatar.width, bubbleHeight: bubble.height };
  });
  expect(geo.bubbleHeight).toBeGreaterThan(40);
  expect(Math.abs(geo.avatarBottom - geo.bubbleBottom)).toBeLessThan(2);
  expect(geo.avatarWidth).toBeGreaterThanOrEqual(32);
});

test("group: an empty avatar reserves the avatar's width so bubbles line up", async ({ page }) => {
  await open(page);
  const demo = variant(page, "group");
  const lefts = await demo.evaluate((frame) => {
    const groups = frame.querySelectorAll<HTMLElement>('[data-slot="message-group"]');
    const withAvatars = groups[2];
    const stack = Array.from(groups).map((g) => getComputedStyle(g).flexDirection);
    const bubbles = Array.from(withAvatars.querySelectorAll<HTMLElement>('[data-slot="bubble"]')).map((b) => b.getBoundingClientRect().left);
    return { stack, bubbles };
  });
  expect(lefts.stack).toEqual(["column", "column", "column"]);
  expect(lefts.bubbles).toHaveLength(2);
  expect(Math.abs(lefts.bubbles[0] - lefts.bubbles[1])).toBeLessThan(1);
});

test("header and footer: muted small text; a footer lifts the avatar to stay level with the bubble", async ({ page }) => {
  await open(page);
  const demo = variant(page, "header_footer");
  await expect(demo.getByText("Evil Rabbit", { exact: true }).first()).toBeVisible();
  await expect(demo.getByText("10:42 AM")).toBeVisible();
  const m = await demo.evaluate((frame) => {
    const row = frame.querySelectorAll<HTMLElement>('[data-slot="message"]')[0];
    const avatar = row.querySelector<HTMLElement>('[data-slot="message-avatar"]')!;
    const bubble = row.querySelector<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
    const a = avatar.getBoundingClientRect();
    const footer = row.querySelector<HTMLElement>('[data-slot="message-footer"]')!;
    const header = row.querySelector<HTMLElement>('[data-slot="message-header"]')!;
    return {
      lift: new DOMMatrix(getComputedStyle(avatar).transform).m42,
      avatarBottom: a.bottom,
      bubbleBottom: bubble.bottom,
      footerBottom: footer.getBoundingClientRect().bottom,
      footerSize: getComputedStyle(footer).fontSize,
      headerSize: getComputedStyle(header).fontSize,
      headerWeight: getComputedStyle(header).fontWeight,
    };
  });
  expect(m.lift).toBe(-32); // -translate-y-8: the avatar clears the footer
  // Level with the bubble (to within the footer's own line box), not with the footer.
  expect(Math.abs(m.avatarBottom - m.bubbleBottom)).toBeLessThan(10);
  expect(m.avatarBottom).toBeLessThan(m.footerBottom - 20);
  expect(m.footerSize).toBe("12px");
  expect(m.headerSize).toBe("12px");
  expect(m.headerWeight).toBe("500");
});

test("an end-aligned footer's content stays at the end", async ({ page }) => {
  await open(page);
  const demo = variant(page, "header_footer");
  const edge = await demo.evaluate((frame) => {
    const row = frame.querySelectorAll<HTMLElement>('[data-slot="message"]')[1];
    const footer = row.querySelector<HTMLElement>('[data-slot="message-footer"]')!;
    return { justify: getComputedStyle(footer).justifyContent, selfAlign: getComputedStyle(footer).alignSelf };
  });
  expect(edge.justify).toBe("flex-end");
  expect(edge.selfAlign).toBe("flex-end");
});

test("actions: icon-only footer buttons are named and operable by keyboard", async ({ page }) => {
  await open(page);
  const demo = variant(page, "actions");
  const status = demo.getByRole("status");
  for (const name of ["Copy", "Good response", "Bad response", "Retry"]) {
    await expect(demo.getByRole("button", { name })).toBeVisible();
  }
  await demo.getByRole("button", { name: "Copy" }).click();
  await expect(status).toHaveText("Copied");
  await demo.getByRole("button", { name: "Good response" }).focus();
  await page.keyboard.press("Enter");
  await expect(status).toHaveText("Good response");
  await page.keyboard.press("Tab");
  await expect(demo.getByRole("button", { name: "Bad response" })).toBeFocused();
  await page.keyboard.press("Space");
  await expect(status).toHaveText("Bad response");
  // A ghost bubble has no inset, so the footer loses its own.
  const pad = await demo.locator('[data-slot="message-footer"]').evaluate((el) => getComputedStyle(el).paddingInlineStart);
  expect(pad).toBe("0px");
});

test("attachment variant: files sit above the bubble on the sender's side", async ({ page }) => {
  await open(page);
  const demo = variant(page, "attachment");
  const group = demo.getByRole("group", { name: "Attached files" });
  await expect(group).toBeVisible();
  await expect(group.locator('[data-slot="attachment"]')).toHaveCount(2);
  const geo = await demo.evaluate((frame) => {
    const row = frame.querySelector<HTMLElement>('[data-slot="message"]')!;
    const g = row.querySelector<HTMLElement>('[data-slot="attachment-group"]')!.getBoundingClientRect();
    const b = row.querySelector<HTMLElement>('[data-slot="bubble"]')!.getBoundingClientRect();
    const r = row.getBoundingClientRect();
    return { groupBottom: g.bottom, bubbleTop: b.top, bubbleFromEnd: r.right - b.right };
  });
  expect(geo.groupBottom).toBeLessThanOrEqual(geo.bubbleTop);
  expect(geo.bubbleFromEnd).toBeLessThan(1);
});

for (const scheme of ["light", "dark"] as const) {
  test.describe(`${scheme} colour roles`, () => {
    test.beforeEach(async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await open(page);
      // The emulation must reach the theme's `--light`/`--dark` toggle, or the
      // dark half of this block silently re-tests light.
      expect(await page.evaluate(() => matchMedia("(prefers-color-scheme: dark)").matches)).toBe(scheme === "dark");
    });

    test("header and footer are --dx-muted-foreground; the avatar slot is --dx-muted", async ({ page }) => {
      const demo = variant(page, "header_footer");
      const c = await demo.evaluate((frame) => {
        const role = (prop: "color" | "backgroundColor", token: string) => {
          const el = document.createElement("span");
          el.style[prop] = `var(${token})`;
          frame.appendChild(el);
          const v = getComputedStyle(el)[prop];
          el.remove();
          return v;
        };
        const row = frame.querySelector<HTMLElement>('[data-slot="message"]')!;
        const q = (slot: string) => row.querySelector<HTMLElement>(`[data-slot="${slot}"]`)!;
        return {
          header: getComputedStyle(q("message-header")).color,
          footer: getComputedStyle(q("message-footer")).color,
          muted: role("color", "--dx-muted-foreground"),
          slot: getComputedStyle(q("message-avatar")).backgroundColor,
          mutedBg: role("backgroundColor", "--dx-muted"),
        };
      });
      expect(c.header).toBe(c.muted);
      expect(c.footer).toBe(c.muted);
      expect(c.slot).toBe(c.mutedBg);
    });
  });
}

test.describe("Axe automated scan", () => {
  // A message has no overlay/expand/select interaction -- one state to scan (the
  // page holds the main demo and every variant, so this covers all of them).
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await open(page);
    await expectNoAxeViolations(page, "message: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
