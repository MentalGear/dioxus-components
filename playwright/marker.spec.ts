import { test, expect } from "./fixtures";
import type { Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const URL = `${BASE_URL}/component/?name=marker&`;
const GOTO = { timeout: 20 * 60 * 1000 };

// Scope to a demo frame. The component page also renders each demo's own source
// in a syntax-highlighted code viewer, so every visible string appears twice.
const main = (page: Page) => page.locator("#component-preview-frame").first();
// The component's stylesheet is linked after mount: wait until it has applied
// before any test reads a computed style.
const styled = (page: Page) =>
  expect
    .poll(() => main(page).locator('[data-slot="marker"]').first().evaluate((el) => getComputedStyle(el).display), {
      timeout: 60_000,
    })
    .toBe("flex");
const variant = (page: Page, name: string) => page.locator(`#component-preview-frame-${name}`).first();

test("markers render their content, icons are decorative, status markers are live regions", async ({ page }) => {
  await page.goto(URL, GOTO);
  await styled(page);
  const demo = main(page);
  await expect(demo.getByText("A default marker")).toBeVisible();
  await expect(demo.getByText("Marker with icon")).toBeVisible();

  // Every MarkerIcon is aria-hidden, so the adjacent text carries the meaning.
  const icons = demo.locator('[data-slot="marker-icon"]');
  expect(await icons.count()).toBeGreaterThan(0);
  for (const icon of await icons.all()) await expect(icon).toHaveAttribute("aria-hidden", "true");

  // The streaming markers carry role=status so they are announced.
  const status = demo.locator('[data-slot="marker"][role="status"]');
  await expect(status).toHaveCount(2);
  await expect(status.filter({ hasText: "Thinking..." })).toHaveCount(1);
  await expect(status.filter({ hasText: "Thinking..." }).locator(".dx-shimmer")).toHaveCount(1);
});

test("a marker rendered as a link or button is a real, focusable control", async ({ page }) => {
  await page.goto(URL, GOTO);
  await styled(page);
  const demo = main(page);
  const link = demo.getByRole("link", { name: "Marker as a link" });
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute("data-slot", "marker");
  await expect(link).toHaveClass(/dx-marker/);
  const button = demo.getByRole("button", { name: "Marker as a button" });
  await expect(button).toBeVisible();

  // Keyboard: both are reachable with Tab, and focus is visible (a ring).
  await link.focus();
  await expect(link).toBeFocused();
  const ring = await link.evaluate((el) => getComputedStyle(el).boxShadow);
  expect(ring).not.toBe("none");
  await page.keyboard.press("Tab");
  await expect(button).toBeFocused();
});

test("the as-button marker acts on click and Enter (link_button variant)", async ({ page }) => {
  await page.goto(URL, GOTO);
  await styled(page);
  const demo = variant(page, "link_button");
  const button = demo.getByRole("button", { name: /Show earlier activity/ });
  await expect(button).toContainText("(0)");
  await button.click();
  await expect(button).toContainText("(1)");
  await button.focus();
  await page.keyboard.press("Enter");
  await expect(button).toContainText("(2)");
});

test("variants: a labeled separator has divider lines and is not role=separator", async ({ page }) => {
  await page.goto(URL, GOTO);
  await styled(page);
  const demo = variant(page, "variants");
  const markers = demo.locator('[data-slot="marker"]');
  await expect(markers).toHaveCount(3);
  await expect(markers.nth(0)).toHaveAttribute("data-variant", "default");
  await expect(markers.nth(1)).toHaveAttribute("data-variant", "border");
  await expect(markers.nth(2)).toHaveAttribute("data-variant", "separator");

  const separator = markers.nth(2);
  // The text is announced as ordinary content: no role=separator on the marker.
  await expect(separator).not.toHaveAttribute("role", "separator");
  await expect(demo.getByRole("separator")).toHaveCount(0);
  await expect(separator.getByText("Separator: Today")).toBeVisible();
  // The divider lines are decorative ::before/::after boxes: 1px tall, flexing.
  const lines = await separator.evaluate((el) => {
    const read = (pseudo: string) => {
      const s = getComputedStyle(el, pseudo);
      return { height: s.height, flexGrow: s.flexGrow, content: s.content };
    };
    return { before: read("::before"), after: read("::after") };
  });
  for (const line of [lines.before, lines.after]) {
    expect(line.height).toBe("1px");
    expect(line.flexGrow).toBe("1");
    expect(line.content).not.toBe("none");
  }
  // The bordered marker draws a bottom border; the default one does not.
  const borders = await markers.evaluateAll((els) =>
    els.map((el) => getComputedStyle(el).borderBottomWidth),
  );
  expect(borders[1]).toBe("1px");
  expect(borders[0]).toBe("0px");
});

test("status variant: spinner markers announce, the shimmer animates", async ({ page }) => {
  await page.goto(URL, GOTO);
  await styled(page);
  const demo = variant(page, "status");
  await expect(demo.locator('[data-slot="marker"][role="status"]')).toHaveCount(3);
  const shimmer = demo.locator(".dx-shimmer").first();
  const anim = await shimmer.evaluate((el) => {
    const s = getComputedStyle(el);
    return { name: s.animationName, backgroundClip: s.backgroundClip, fill: s.webkitTextFillColor };
  });
  expect(anim.name).toBe("dx-shimmer-sweep");
  expect(anim.backgroundClip).toBe("text");
  expect(anim.fill).toBe("rgba(0, 0, 0, 0)"); // transparent: the gradient paints the glyphs
});

test("the shimmer is off under prefers-reduced-motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(URL, GOTO);
  await styled(page);
  const shimmer = variant(page, "status").locator(".dx-shimmer").first();
  await expect(shimmer).toBeVisible();
  const s = await shimmer.evaluate((el) => {
    const c = getComputedStyle(el);
    return { name: c.animationName, image: c.backgroundImage, color: c.color, fill: c.webkitTextFillColor };
  });
  expect(s.name).toBe("none");
  expect(s.image).toBe("none");
  expect(s.fill).toBe(s.color); // plain text again
});

for (const scheme of ["light", "dark"] as const) {
  test.describe(`${scheme} colour roles`, () => {
    test.beforeEach(async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.goto(URL, GOTO);
      await styled(page);
      // The emulation must reach the theme's `--light`/`--dark` toggle, or the
      // dark half of this block silently re-tests light.
      expect(await page.evaluate(() => matchMedia("(prefers-color-scheme: dark)").matches)).toBe(scheme === "dark");
    });

    test("text is --dx-muted-foreground, dividers are --dx-border, hover goes to --dx-foreground", async ({ page }) => {
      const demo = variant(page, "variants");
      const roles = await demo.evaluate((frame) => {
        const probe = (prop: "color" | "borderColor", token: string) => {
          const el = document.createElement("span");
          el.style[prop] = `var(${token})`;
          el.style.borderStyle = "solid";
          frame.appendChild(el);
          const v = getComputedStyle(el)[prop];
          el.remove();
          return v;
        };
        const markers = frame.querySelectorAll<HTMLElement>('[data-slot="marker"]');
        const sep = markers[2];
        return {
          text: getComputedStyle(markers[0]).color,
          muted: probe("color", "--dx-muted-foreground"),
          line: getComputedStyle(sep, "::before").backgroundColor,
          border: probe("borderColor", "--dx-border"),
          borderBottom: getComputedStyle(markers[1]).borderBottomColor,
        };
      });
      expect(roles.text).toBe(roles.muted);
      expect(roles.line).toBe(roles.border);
      expect(roles.borderBottom).toBe(roles.border);

      const link = main(page).getByRole("link", { name: "Marker as a link" });
      const fg = await link.evaluate((el) => {
        const p = document.createElement("span");
        p.style.color = "var(--dx-foreground)";
        el.parentElement!.appendChild(p);
        const v = getComputedStyle(p).color;
        p.remove();
        return v;
      });
      await link.hover();
      await expect.poll(() => link.evaluate((el) => getComputedStyle(el).color)).toBe(fg);
    });
  });
}

test.describe("Axe automated scan", () => {
  // A marker has no overlay/expand/select interaction -- one state to scan (the
  // page holds the main demo and every variant, so this covers all of them).
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(URL, GOTO);
  await styled(page);
    await expectNoAxeViolations(page, "marker: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
