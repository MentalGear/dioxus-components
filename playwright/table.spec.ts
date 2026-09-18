import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("table renders the invoice demo: caption, header, rows, and footer total", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=table&", { timeout: 20 * 60 * 1000 });

  const table = page.locator('[data-slot="table"]');
  await expect(table).toBeVisible();
  await expect(page.getByText("A list of your recent invoices.")).toBeVisible();

  // 4 columns.
  const headerRow = table.locator("thead tr");
  await expect(headerRow.getByRole("columnheader", { name: "Invoice" })).toBeVisible();
  await expect(headerRow.getByRole("columnheader", { name: "Status" })).toBeVisible();
  await expect(headerRow.getByRole("columnheader", { name: "Method" })).toBeVisible();
  await expect(headerRow.getByRole("columnheader", { name: "Amount" })).toBeVisible();
  for (const th of await headerRow.getByRole("columnheader").all()) {
    await expect(th, "TableHead defaults to scope=col").toHaveAttribute("scope", "col");
  }

  // 6 rows (5+ per the brief).
  const bodyRows = table.locator("tbody tr");
  await expect(bodyRows).toHaveCount(6);
  await expect(bodyRows.first()).toContainText("INV001");
  await expect(bodyRows.first()).toContainText("$250.00");

  // Footer total.
  const footerRow = table.locator("tfoot tr");
  await expect(footerRow).toContainText("Total");
  await expect(footerRow).toContainText("$1,950.00");
  await expect(footerRow.locator("td").first()).toHaveAttribute("colspan", "3");
});

test.describe("Axe automated scan", () => {
  // Table has no overlay/expand/select interaction of its own -- one
  // state to scan (the two-state convention in ./axe.ts applies only to
  // components that reach an open/expanded/selected state).
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=table&", { timeout: 20 * 60 * 1000 });
    await expect(page.locator('[data-slot="table"]')).toBeVisible();
    await expectNoAxeViolations(page, "table: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
