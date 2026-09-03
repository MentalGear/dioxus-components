import { test } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test.describe("homepage", () => {
  test("should not have any automatically detectable accessibility issues", async ({
    page,
  }) => {
    await page.goto("http://127.0.0.1:8080/", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes

    // Wait for the page to fully load
    let heroSection = page.locator("#hero");
    await heroSection.waitFor({ state: "visible" });

    // No color-contrast exclusion needed here: this round's theme-token fix
    // (docs/backlog.md row 39) resolved it, and this route has no
    // `.dx-code-block` to need the one remaining, narrower exclusion for.
    await expectNoAxeViolations(page, "homepage");
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
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
