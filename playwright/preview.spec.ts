import { test } from "@playwright/test";
import { expectNoAxeViolations } from "./axe";

const COLOR_CONTRAST_REASON =
  "grandfathered pre-existing exclusion (see playwright/axe.ts header doc): " +
  "not a false positive -- the theme's contrast ratios are a real, open, " +
  "tracked gap (docs/backlog.md rows 31/32, design tokens + styling engine, " +
  "not yet landed); this call's coverage is preserved unchanged per the " +
  "axe-coverage round's own instruction, not newly claimed as a false positive";

test.describe("homepage", () => {
  test("should not have any automatically detectable accessibility issues", async ({
    page,
  }) => {
    await page.goto("http://127.0.0.1:8080/", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes

    // Wait for the page to fully load
    let heroSection = page.locator("#hero");
    await heroSection.waitFor({ state: "visible" });

    await expectNoAxeViolations(page, "homepage", {
      exclude: [{ ids: "color-contrast", reason: COLOR_CONTRAST_REASON }],
    });
  });
});


test.describe("details", () => {
  test("should not have any automatically detectable accessibility issues", async ({
    page,
  }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=calendar", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes

    // Wait for the page to fully load
    let componentSection = page.getByRole("heading", { name: "calendar" });
    await componentSection.waitFor({ state: "visible" });

    await expectNoAxeViolations(page, "component/calendar", {
      exclude: [{ ids: "color-contrast", reason: COLOR_CONTRAST_REASON }],
    });
  });
});
