import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=attachment&`;
const GOTO = { timeout: 20 * 60 * 1000 };

// Scope to a demo frame. The component page also renders each demo's own source
// in a syntax-highlighted code viewer, so every visible string appears twice.
const main = (page: Page) => page.locator("#component-preview-frame").first();
const variant = (page: Page, name: string) => page.locator(`#component-preview-frame-${name}`).first();

// The component's stylesheet is linked after mount: wait until it has applied
// before any test reads a computed style or a position.
async function open(page: Page) {
  await page.goto(URL, GOTO);
  await expect
    .poll(() => main(page).locator('[data-slot="attachment"]').first().evaluate((el) => getComputedStyle(el).display), {
      timeout: 60_000,
    })
    .toBe("flex");
}

test("the main demo lists files, each with a named remove action", async ({ page }) => {
  await open(page);
  const demo = main(page);
  const cards = demo.locator('[data-slot="attachment"]');
  await expect(cards).toHaveCount(3);
  await expect(demo.getByText("sales-dashboard.pdf")).toBeVisible();
  await expect(demo.getByText("PDF · 2.4 MB")).toBeVisible();
  for (const name of ["sales-dashboard.pdf", "q3-forecast.xlsx", "design-assets.zip"]) {
    await expect(demo.getByRole("button", { name: `Remove ${name}` })).toBeVisible();
  }
  // A presentational card: no role of its own, state/size/orientation exposed as data attributes.
  await expect(cards.first()).toHaveAttribute("data-state", "done");
  await expect(cards.first()).toHaveAttribute("data-size", "default");
  await expect(cards.first()).toHaveAttribute("data-orientation", "horizontal");
});

test("remove actions work by mouse and by keyboard (Enter, Space)", async ({ page }) => {
  await open(page);
  const demo = main(page);
  await demo.getByRole("button", { name: "Remove sales-dashboard.pdf" }).click();
  await expect(demo.getByText("sales-dashboard.pdf")).toHaveCount(0);
  const second = demo.getByRole("button", { name: "Remove q3-forecast.xlsx" });
  await second.focus();
  await page.keyboard.press("Enter");
  await expect(demo.getByText("q3-forecast.xlsx")).toHaveCount(0);
  const third = demo.getByRole("button", { name: "Remove design-assets.zip" });
  await third.focus();
  await page.keyboard.press("Space");
  await expect(demo.locator('[data-slot="attachment"]')).toHaveCount(0);
  await expect(demo.getByRole("status")).toHaveText("All attachments removed");
});

test("the action is the library Button (ghost, icon-xs) inside the actions slot", async ({ page }) => {
  await open(page);
  const action = main(page).getByRole("button", { name: "Remove sales-dashboard.pdf" });
  await expect(action).toHaveClass(/dx-button/);
  await expect(action).toHaveClass(/dx-attachment-action/);
  await expect(action).toHaveAttribute("data-style", "ghost");
  await expect(action).toHaveAttribute("data-size", "icon-xs");
  const box = await action.boundingBox();
  expect(Math.round(box!.width)).toBe(24);
});

test("states: each state is exposed; only uploading and processing shimmer the title", async ({ page }) => {
  await open(page);
  const demo = variant(page, "states");
  const states = await demo.locator('[data-slot="attachment"]').evaluateAll((els) =>
    els.map((el) => ({
      state: el.getAttribute("data-state"),
      shimmer: !!el.querySelector('[data-slot="attachment-title"].dx-shimmer'),
      borderStyle: getComputedStyle(el).borderTopStyle,
    })),
  );
  expect(states.slice(0, 5).map((s) => s.state)).toEqual(["idle", "uploading", "processing", "error", "done"]);
  expect(states.slice(0, 5).map((s) => s.shimmer)).toEqual([false, true, true, false, false]);
  // idle is dashed; the rest are solid.
  expect(states.slice(0, 5).map((s) => s.borderStyle)).toEqual(["dashed", "solid", "solid", "solid", "solid"]);
  // The shimmer is a real animation, and the error reason stays in text.
  const anim = await demo.locator(".dx-shimmer").first().evaluate((el) => getComputedStyle(el).animationName);
  expect(anim).toBe("dx-shimmer-sweep");
  await expect(demo.getByText("Upload failed: the file is larger than 25 MB")).toBeVisible();
});

test("states: changing the attachment's state reaches its title (idle -> uploading -> processing -> done)", async ({ page }) => {
  await open(page);
  const demo = variant(page, "states");
  const card = demo.locator('[data-slot="attachment"]').nth(5);
  const title = card.locator('[data-slot="attachment-title"]');
  const advance = demo.getByRole("button", { name: "Advance lifecycle" });
  await expect(card).toHaveAttribute("data-state", "idle");
  await expect(title).not.toHaveClass(/dx-shimmer/);
  await advance.click();
  await expect(card).toHaveAttribute("data-state", "uploading");
  await expect(title).toHaveClass(/dx-shimmer/);
  await advance.click();
  await expect(card).toHaveAttribute("data-state", "processing");
  await expect(title).toHaveClass(/dx-shimmer/);
  await advance.click();
  await expect(card).toHaveAttribute("data-state", "done");
  await expect(title).not.toHaveClass(/dx-shimmer/);
  await expect(card).toContainText("PDF · 2.4 MB");
});

test("sizes: media and gap shrink from default to sm to xs", async ({ page }) => {
  await open(page);
  const cards = variant(page, "sizes").locator('[data-slot="attachment"]');
  await expect(cards).toHaveCount(3);
  const m = await cards.evaluateAll((els) =>
    els.map((el) => ({
      size: el.getAttribute("data-size"),
      media: el.querySelector<HTMLElement>('[data-slot="attachment-media"]')!.getBoundingClientRect().width,
      font: getComputedStyle(el).fontSize,
      radius: getComputedStyle(el).borderTopLeftRadius,
    })),
  );
  expect(m.map((x) => x.size)).toEqual(["default", "sm", "xs"]);
  expect(m.map((x) => Math.round(x.media))).toEqual([40, 32, 28]);
  expect(m.map((x) => x.font)).toEqual(["14px", "12px", "12px"]);
  // xs uses a tighter corner than the other two.
  expect(parseFloat(m[2].radius)).toBeLessThan(parseFloat(m[0].radius));
});

test("image media: an <img> with alt; vertical stacks the media above the content", async ({ page }) => {
  await open(page);
  const demo = variant(page, "image");
  await expect(demo.getByRole("img", { name: "Gradient preview of workspace.png" })).toBeVisible();
  const cards = demo.locator('[data-slot="attachment"]');
  await expect(cards).toHaveCount(3);
  const m = await cards.evaluateAll((els) =>
    els.map((el) => {
      const media = el.querySelector<HTMLElement>('[data-slot="attachment-media"]')!;
      const content = el.querySelector<HTMLElement>('[data-slot="attachment-content"]')!;
      return {
        direction: getComputedStyle(el).flexDirection,
        width: el.getBoundingClientRect().width,
        mediaVariant: media.dataset.variant,
        mediaBottom: media.getBoundingClientRect().bottom,
        contentTop: content.getBoundingClientRect().top,
        mediaOpacity: getComputedStyle(media).opacity,
        imgFit: getComputedStyle(media.querySelector("img")!).objectFit,
      };
    }),
  );
  expect(m[0].direction).toBe("row");
  expect(m[1].direction).toBe("column");
  expect(m[0].mediaVariant).toBe("image");
  expect(m[1].width).toBeCloseTo(120, 0); // w-30: a vertical card with content
  expect(m[1].mediaBottom).toBeLessThanOrEqual(m[1].contentTop);
  expect(m[1].imgFit).toBe("cover");
  // A finished image is opaque, one still uploading is dimmed.
  expect(m[1].mediaOpacity).toBe("1");
  expect(m[2].mediaOpacity).toBe("0.6");
});

test("group: a labelled, focusable, snapping row that scrolls from the keyboard", async ({ page }) => {
  await open(page);
  const demo = variant(page, "group");
  const group = demo.getByRole("group", { name: "Attached files" });
  await expect(group).toHaveAttribute("tabindex", "0");
  await expect(group.locator('[data-slot="attachment"]')).toHaveCount(6);
  const s = await group.evaluate((el) => {
    const c = getComputedStyle(el);
    return { overflowX: c.overflowX, snap: c.scrollSnapType, scrollable: el.scrollWidth > el.clientWidth + 10, scrollbar: c.scrollbarWidth, mask: c.maskImage };
  });
  expect(s.overflowX).toBe("auto");
  expect(s.snap).toContain("x");
  expect(s.snap).toContain("mandatory");
  expect(s.scrollable).toBe(true);
  expect(s.scrollbar).toBe("none");
  expect(s.mask).toContain("linear-gradient");

  await group.focus();
  await expect(group).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => group.evaluate((el) => el.scrollLeft)).toBeGreaterThan(0);
});

test("group: the edge fade follows the scroll position (start crisp at rest, end crisp at the end)", async ({ page }) => {
  await open(page);
  const group = variant(page, "group").getByRole("group", { name: "Attached files" });
  // The fade depths are scroll-driven registered properties whose computed value
  // is a calc() string, so resolve each to pixels through a margin and restore it.
  const fades = () =>
    group.evaluate((el: HTMLElement) => {
      const px = (name: string) => {
        const prev = el.style.marginInlineStart;
        el.style.marginInlineStart = `var(${name})`;
        const v = parseFloat(getComputedStyle(el).marginInlineStart);
        el.style.marginInlineStart = prev;
        return v;
      };
      return { s: px("--dx-scroll-fade-s"), e: px("--dx-scroll-fade-e") };
    });
  // At rest: nothing to fade in at the start, a fade hinting at more at the end.
  await expect.poll(async () => (await fades()).e).toBeGreaterThan(0);
  expect((await fades()).s).toBe(0);
  // Mid-scroll: both edges fade.
  await group.evaluate((el) => (el.scrollLeft = 150));
  await expect.poll(async () => (await fades()).s).toBeGreaterThan(0);
  // At the end: the end edge sharpens.
  await group.evaluate((el) => (el.scrollLeft = el.scrollWidth));
  await expect.poll(async () => (await fades()).e).toBe(0);
  expect((await fades()).s).toBeGreaterThan(0);
});

test("group: the fade mirrors in RTL (the crisp edge follows the reading direction)", async ({ page }) => {
  await open(page);
  const group = variant(page, "group").getByRole("group", { name: "Attached files" });
  const gradients = await group.evaluate((el) => {
    const ltr = getComputedStyle(el).getPropertyValue("--dx-scroll-fade-inline");
    el.setAttribute("dir", "rtl");
    const rtl = getComputedStyle(el).getPropertyValue("--dx-scroll-fade-inline");
    return { ltr, rtl };
  });
  expect(gradients.ltr).toContain("to right");
  expect(gradients.rtl).toContain("to left");
});

test("trigger: a full-card overlay behind the actions; both are separately clickable", async ({ page }) => {
  await open(page);
  const demo = variant(page, "trigger");
  const card = demo.locator('[data-slot="attachment"]', { hasText: "research-summary.pdf" });
  const remove = demo.getByRole("button", { name: "Remove research-summary.pdf" });

  // The overlay covers the whole card, and the actions sit above it.
  const geo = await card.evaluate((el) => {
    const t = el.querySelector<HTMLElement>('[data-slot="attachment-trigger"]')!;
    const a = el.querySelector<HTMLElement>('[data-slot="attachment-actions"]')!;
    const c = el.getBoundingClientRect();
    const b = t.getBoundingClientRect();
    return {
      card: [c.left, c.top, c.right, c.bottom].map(Math.round),
      overlay: [b.left, b.top, b.right, b.bottom].map(Math.round),
      triggerZ: Number(getComputedStyle(t).zIndex),
      actionsZ: Number(getComputedStyle(a).zIndex),
    };
  });
  // The overlay sits inside the card's border box.
  expect(geo.overlay[0]).toBeGreaterThanOrEqual(geo.card[0]);
  expect(geo.overlay[2]).toBeLessThanOrEqual(geo.card[2]);
  expect(geo.overlay[2] - geo.overlay[0]).toBeGreaterThan((geo.card[2] - geo.card[0]) - 4);
  expect(geo.actionsZ).toBeGreaterThan(geo.triggerZ);

  // Clicking the card opens it; clicking the action removes without opening.
  await card.click({ position: { x: 12, y: 12 } });
  await expect(demo.getByRole("status")).toHaveText("Opened research-summary.pdf");
  await remove.click();
  await expect(card).toHaveCount(0);
});

test("trigger: keyboard reaches the action first, then the trigger; Enter activates it", async ({ page }) => {
  await open(page);
  const demo = variant(page, "trigger");
  const remove = demo.getByRole("button", { name: "Remove research-summary.pdf" });
  const trigger = demo.getByRole("button", { name: "Preview research-summary.pdf" });
  await remove.focus();
  await page.keyboard.press("Tab");
  await expect(trigger).toBeFocused();
  // The card shows a focus ring while the trigger holds focus.
  const ring = await demo.locator('[data-slot="attachment"]').first().evaluate((el) => getComputedStyle(el).boxShadow);
  expect(ring).not.toBe("none");
  await page.keyboard.press("Enter");
  await expect(demo.getByRole("status")).toHaveText("Opened research-summary.pdf");
});

test("trigger as a link: a real, named anchor over the card", async ({ page }) => {
  await open(page);
  const link = variant(page, "trigger").getByRole("link", { name: "Open workspace-notes.md" });
  await expect(link).toHaveAttribute("data-slot", "attachment-trigger");
  await expect(link).not.toHaveAttribute("type", /.+/);
  await link.focus();
  await expect(link).toBeFocused();
});

test("a clickable card hovers to a muted fill; a plain one does not", async ({ page }) => {
  await open(page);
  const demo = variant(page, "trigger");
  const clickable = demo.locator('[data-slot="attachment"]').first();
  const bg = () => clickable.evaluate((el) => getComputedStyle(el).backgroundColor);
  const before = await bg();
  await clickable.hover({ position: { x: 12, y: 12 } });
  await expect.poll(bg).not.toBe(before);
  const plain = variant(page, "sizes").locator('[data-slot="attachment"]').first();
  const plainBefore = await plain.evaluate((el) => getComputedStyle(el).backgroundColor);
  await plain.hover({ position: { x: 4, y: 4 } });
  await page.waitForTimeout(300);
  expect(await plain.evaluate((el) => getComputedStyle(el).backgroundColor)).toBe(plainBefore);
});

for (const scheme of ["light", "dark"] as const) {
  test.describe(`${scheme} colour roles`, () => {
    test.beforeEach(async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await open(page);
      // The emulation must reach the theme's `--light`/`--dark` toggle, or the
      // dark half of this block silently re-tests light.
      expect(await page.evaluate(() => matchMedia("(prefers-color-scheme: dark)").matches)).toBe(scheme === "dark");
      // The card eases its fill in (a 150ms transition on first style resolution);
      // read the settled colours, not a frame mid-transition.
      await page.addStyleTag({ content: '[data-slot="attachment"] { transition: none !important; }' });
    });

    test("the card is --dx-card with a --dx-border edge; media is --dx-muted; metadata is --dx-muted-foreground", async ({ page }) => {
      const c = await variant(page, "sizes").evaluate((frame) => {
        const role = (prop: "color" | "backgroundColor" | "borderColor", token: string) => {
          const el = document.createElement("span");
          el.style[prop] = `var(${token})`;
          el.style.borderStyle = "solid";
          frame.appendChild(el);
          const v = getComputedStyle(el)[prop];
          el.remove();
          return v;
        };
        const card = frame.querySelector<HTMLElement>('[data-slot="attachment"]')!;
        const q = (slot: string) => card.querySelector<HTMLElement>(`[data-slot="${slot}"]`)!;
        return {
          bg: getComputedStyle(card).backgroundColor,
          card: role("backgroundColor", "--dx-card"),
          fg: getComputedStyle(card).color,
          cardFg: role("color", "--dx-card-foreground"),
          border: getComputedStyle(card).borderTopColor,
          borderRole: role("borderColor", "--dx-border"),
          media: getComputedStyle(q("attachment-media")).backgroundColor,
          muted: role("backgroundColor", "--dx-muted"),
          desc: getComputedStyle(q("attachment-description")).color,
          mutedFg: role("color", "--dx-muted-foreground"),
        };
      });
      expect(c.bg).toBe(c.card);
      expect(c.fg).toBe(c.cardFg);
      expect(c.border).toBe(c.borderRole);
      expect(c.media).toBe(c.muted);
      expect(c.desc).toBe(c.mutedFg);
    });

    test("the error state keeps its reason legible: >= 4.5:1 on the card, and a destructive edge", async ({ page }) => {
      const e = await variant(page, "states").evaluate((frame) => {
        const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
        const rgb = (css: string, under = "#000") => {
          ctx.fillStyle = "#000";
          ctx.fillStyle = under;
          ctx.fillRect(0, 0, 1, 1);
          ctx.fillStyle = css;
          ctx.fillRect(0, 0, 1, 1);
          return Array.from(ctx.getImageData(0, 0, 1, 1).data).slice(0, 3);
        };
        const lum = ([r, g, b]: number[]) => {
          const f = (c: number) => ((c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
          return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
        };
        const ratio = (a: number[], b: number[]) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
        const card = frame.querySelector<HTMLElement>('[data-state="error"]')!;
        const cardBg = rgb(getComputedStyle(card).backgroundColor);
        const desc = rgb(getComputedStyle(card.querySelector('[data-slot="attachment-description"]')!).color);
        const media = card.querySelector<HTMLElement>('[data-slot="attachment-media"]')!;
        const mediaBg = rgb(getComputedStyle(media).backgroundColor, `rgb(${cardBg.join(",")})`);
        const mediaFg = rgb(getComputedStyle(media).color);
        const border = getComputedStyle(card).borderTopColor;
        return { descOnCard: ratio(desc, cardBg), iconOnMedia: ratio(mediaFg, mediaBg), border, red: desc[0] > desc[1] && desc[0] > desc[2] };
      });
      expect(e.descOnCard).toBeGreaterThanOrEqual(4.5);
      expect(e.iconOnMedia).toBeGreaterThanOrEqual(4.5);
      expect(e.red).toBe(true);
      expect(e.border).not.toBe("rgba(0, 0, 0, 0)");
    });
  });
}

test.describe("Axe automated scan", () => {
  // The page holds the main demo and every variant (states, sizes, images, the
  // scrolling group, the trigger), so one scan at rest covers all of them.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await open(page);
    await expectNoAxeViolations(page, "attachment: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
