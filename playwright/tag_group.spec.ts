import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations } from "./axe";

const BASE = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080";
const URL = `${BASE}/component/?name=tag_group&`;
const LOAD_TIMEOUT = 20 * 60 * 1000;

function multiVariant(page: Page) {
  return page
    .locator(".dx-component-variant")
    .filter({ has: page.getByRole("heading", { name: "multi" }) });
}

function statesVariant(page: Page) {
  return page
    .locator(".dx-component-variant")
    .filter({ has: page.getByRole("heading", { name: "states" }) });
}

function tag(page: Page, name: string) {
  return multiVariant(page).getByRole("row", { name });
}

async function loadTagGroup(page: Page) {
  await page.goto(URL, { timeout: LOAD_TIMEOUT, waitUntil: "networkidle" });
  const variant = multiVariant(page);
  await variant.scrollIntoViewIfNeeded();
  await expect(variant.getByText("Labels", { exact: true })).toBeVisible({
    timeout: 30000,
  });
  await expect(variant.getByRole("grid")).toBeVisible();
}

test.describe("Tag group", () => {
  // One page load at a time — parallel navigations contend with the preview webServer build.
  test.describe.configure({ mode: "serial", timeout: LOAD_TIMEOUT });

  test.beforeEach(async ({ page }) => {
    await loadTagGroup(page);
  });

  test.describe("Selection", () => {
    test("shows initial selection and supports multiple selection", async ({
      page,
    }) => {
      const bug = tag(page, "bug");
      const core = tag(page, "core");
      const desktop = tag(page, "desktop");

      await expect(bug).toHaveAttribute("data-selected", "true");
      await expect(core).toHaveAttribute("data-selected", "false");

      await core.click();
      await expect(bug).toHaveAttribute("data-selected", "true");
      await expect(core).toHaveAttribute("data-selected", "true");

      await desktop.click();
      await expect(desktop).toHaveAttribute("data-selected", "true");
    });

    test("does not clear the last selected tag when empty selection is disallowed", async ({
      page,
    }) => {
      const bug = tag(page, "bug");

      await expect(bug).toHaveAttribute("data-selected", "true");
      await bug.click();
      await expect(bug).toHaveAttribute("data-selected", "true");

      await tag(page, "core").click();
      await bug.click();
      await expect(bug).toHaveAttribute("data-selected", "false");
      await expect(tag(page, "core")).toHaveAttribute("data-selected", "true");
    });

    test("marks disabled tags as non-interactive", async ({ page }) => {
      const feature = tag(page, "feature");
      const example = tag(page, "example");

      await expect(feature).toHaveAttribute("data-disabled", "true");
      await expect(feature).toHaveAttribute("aria-disabled", "true");
      await expect(feature).toHaveAttribute("tabindex", "-1");
      await expect(feature).toHaveAttribute("data-selected", "false");

      await expect(example).toHaveAttribute("data-disabled", "true");
      await expect(example).toHaveAttribute("aria-disabled", "true");
      await expect(example).toHaveAttribute("tabindex", "-1");
      await expect(example).toHaveAttribute("data-selected", "false");
    });

    test("clears selection on Escape", async ({ page }) => {
      const bug = tag(page, "bug");
      await bug.click();
      await expect(bug).toBeFocused();

      await page.keyboard.press("Escape");
      await expect(bug).toHaveAttribute("data-selected", "false");
      await expect(tag(page, "core")).toHaveAttribute("data-selected", "false");
    });
  });

  test.describe("Keyboard", () => {
    test("roving focus skips disabled tags", async ({ page }) => {
      const bug = tag(page, "bug");
      const core = tag(page, "core");

      await bug.click();
      await expect(bug).toBeFocused();

      await page.keyboard.press("ArrowRight");
      await expect(core).toBeFocused();

      await page.keyboard.press("ArrowLeft");
      await expect(bug).toBeFocused();
    });

    test("Space toggles selection on the focused tag", async ({ page }) => {
      const core = tag(page, "core");

      await core.click();
      await expect(core).toBeFocused();
      await expect(core).toHaveAttribute("data-selected", "true");

      await page.keyboard.press("Space");
      await expect(core).toHaveAttribute("data-selected", "false");

      await page.keyboard.press("Space");
      await expect(core).toHaveAttribute("data-selected", "true");
    });

    test("Delete removes all selected tags", async ({ page }) => {
      const bug = tag(page, "bug");
      const core = tag(page, "core");
      const desktop = tag(page, "desktop");

      await core.click();
      await expect(bug).toHaveAttribute("data-selected", "true");
      await expect(core).toHaveAttribute("data-selected", "true");
      await expect(core).toBeFocused();

      await page.keyboard.press("Delete");

      await expect(bug).toHaveCount(0);
      await expect(core).toHaveCount(0);
      await expect(desktop).toBeFocused();
    });

    test("Delete works for non-selectable removable tags", async ({ page }) => {
      const group = statesVariant(page).getByTestId("tag-group-nonselectable");
      const alpha = group.getByRole("row", { name: "alpha" });
      const beta = group.getByRole("row", { name: "beta" });

      await alpha.click();
      await expect(alpha).toBeFocused();

      await page.keyboard.press("Delete");

      await expect(alpha).toHaveCount(0);
      await expect(beta).toBeFocused();
    });

    test("Delete keeps selected tags that do not have a remove button", async ({
      page,
    }) => {
      const group = statesVariant(page).getByTestId(
        "tag-group-mixed-removable",
      );
      const bug = group.getByRole("row", { name: "bug" });
      const core = group.getByRole("row", { name: "core" });
      const desktop = group.getByRole("row", { name: "desktop" });

      await expect(bug).toHaveAttribute("data-selected", "true");
      await expect(desktop).toHaveAttribute("data-selected", "true");

      await core.click();
      await expect(core).toBeFocused();
      await expect(core).toHaveAttribute("data-selected", "true");

      await page.keyboard.press("Delete");

      await expect(bug).toHaveCount(0);
      await expect(core).toHaveCount(0);
      await expect(desktop).toBeVisible();
      await expect(desktop).toHaveAttribute("data-selected", "true");
      await expect(desktop).toBeFocused();
    });
  });

  test.describe("Removal", () => {
    test("remove button deletes a tag", async ({ page }) => {
      const bug = tag(page, "bug");
      await expect(bug).toBeVisible();

      await multiVariant(page)
        .getByRole("button", { name: "Remove item bug" })
        .click();
      await expect(bug).toHaveCount(0);
    });

    test("disabled tags and groups disable remove buttons", async ({ page }) => {
      const states = statesVariant(page);
      const mixed = states.getByTestId("tag-group-mixed-removable");
      const groupDisabled = states.getByTestId("tag-group-disabled");

      await expect(
        mixed.getByRole("button", { name: "Remove item feature" }),
      ).toBeDisabled();
      await expect(mixed.getByRole("row", { name: "feature" })).toHaveAttribute(
        "tabindex",
        "-1",
      );

      await expect(
        groupDisabled.getByRole("button", { name: "Remove item locked" }),
      ).toBeDisabled();
      await expect(
        groupDisabled.getByRole("row", { name: "locked" }),
      ).toHaveAttribute("tabindex", "-1");
      await expect(
        groupDisabled.getByRole("row", { name: "archived" }),
      ).toHaveAttribute("tabindex", "-1");
    });
  });

  test.describe("Geometry", () => {
    // User report (live site): "Tag group looks a bit deflated." Confirmed
    // NOT a row-31b token-migration regression -- the pre-fix CSS
    // (padding-block: 0.125rem, font-size: var(--dx-text-xs)) reproduces
    // upstream PR #271 (bf007c1) byte-for-byte once its class names are
    // adapted to the current dx- naming, measured via injected-stylesheet
    // screenshots against the live :8080 demo. This asserts the modest,
    // deliberate improvement toward shadcn's Badge/Toggle sizing instead
    // (dev-docs/backlog.md): a tag is >= 28px tall (box-sizing: border-box
    // + min-height: var(--dx-space-7)) with --dx-space-1 vertical /
    // --dx-space-3 horizontal padding. Fails on the pre-fix CSS (20px tall,
    // 2px/12px padding), passes on the fixed one (28px, 4px/12px).
    test("a tag is tall enough to read comfortably (not a deflated pill)", async ({
      page,
    }) => {
      const bug = tag(page, "bug");
      await expect(bug).toBeVisible();

      const box = await bug.boundingBox();
      expect(box).not.toBeNull();
      expect(box!.height).toBeGreaterThanOrEqual(28);

      const padding = await bug.evaluate((el) => {
        const cs = getComputedStyle(el);
        return {
          top: cs.paddingTop,
          bottom: cs.paddingBottom,
          left: cs.paddingLeft,
          right: cs.paddingRight,
        };
      });
      expect(padding.top).toBe("4px"); // --dx-space-1
      expect(padding.bottom).toBe("4px"); // --dx-space-1
      expect(padding.left).toBe("12px"); // --dx-space-3
      expect(padding.right).toBe("12px"); // --dx-space-3
    });
  });

  test.describe("Accessibility", () => {
    test("has no automatically detectable a11y violations on the tag list", async ({
      page,
    }) => {
      // No color-contrast exclusion needed: this round's theme-token fix
      // (docs/backlog.md row 39) resolved the pre-existing finding here,
      // and this scan is scoped to the tag grid itself, which has no code
      // block to need the one remaining, narrower exclusion for.
      await expectNoAxeViolations(page, "tag_group: tag list", {
        include: '.dx-component-variant [role="grid"]',
      });
    });
  });
});
