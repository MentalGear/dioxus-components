import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("table renders the invoice demo: caption, header, rows, and footer total", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=table&", { timeout: 20 * 60 * 1000 });

  const table = page.locator('[data-slot="table"]');
  await expect(table).toBeVisible();
  // Scoped to the real <caption>, not an unscoped page.getByText() -- the
  // demo's own source text (including this exact caption string) is also
  // always in the DOM in the page's syntax-highlighted "Code" tab
  // (ComponentVariantHighlight's TabContent for "Code" renders next to the
  // "Demo" one, both mounted regardless of which tab is active) and in the
  // "Manual installation" <details> section's component.rs source, so an
  // unscoped text/name lookup for a string that also appears in either
  // resolves 2+ elements and fails Playwright's strict mode. `table` is
  // scoped by data-slot="table", an attribute the highlighted/copied
  // source text never actually carries (it's highlighted as text, not
  // reparsed into real DOM attributes), so every lookup below stays
  // scoped under it rather than querying `page` directly.
  const caption = table.locator("caption");
  await expect(caption).toBeVisible();
  await expect(caption).toHaveText("A list of your recent invoices.");

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
