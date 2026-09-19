import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

// Fixture mirrors preview/src/components/data_table/variants/main/mod.rs's
// `PAYMENTS` (12 rows, id/status/email/amount) exactly -- single source of
// truth for the sort/filter/page expectations below is that file's own
// header comment, not re-derived here.
const goto = (page: Page) =>
  page.goto(`${BASE_URL}/component/?name=data_table&`, { timeout: 20 * 60 * 1000 });

const table = (page: Page) => page.locator('[data-slot="table"]');
const filterInput = (page: Page) => page.getByRole("textbox", { name: "Filter by email" });
const selectAllCheckbox = (page: Page) => page.getByRole("checkbox", { name: "Select all" });
const rowCheckboxes = (page: Page) => page.getByRole("checkbox", { name: /^Select row / });
const pagination = (page: Page) => page.locator('[data-slot="data-table-pagination"]');
const selectedSummary = (page: Page) => pagination(page).locator(".dx-data-table-pagination-selected");
const pageReadout = (page: Page) => pagination(page).locator(".dx-data-table-pagination-page");
const previousButton = (page: Page) => pagination(page).getByRole("button", { name: "Previous" });
const nextButton = (page: Page) => pagination(page).getByRole("button", { name: "Next" });
// The Select trigger's role is changing elsewhere in this repo (button ->
// combobox); located by its own data-slot + accessible name, not role, so
// this spec doesn't depend on which role wins.
const pageSizeControl = (page: Page) =>
  page.locator('[data-slot="data-table-page-size"]').locator('[aria-label="Rows per page"]');

test("renders the payments demo with pagination", async ({ page }) => {
  await goto(page);

  await expect(table(page)).toBeVisible();
  await expect(table(page).locator("tbody tr")).toHaveCount(5); // page 1 of 3, 5 rows/page
  await expect(selectedSummary(page)).toHaveText("0 of 12 row(s) selected.");
  await expect(pageReadout(page)).toHaveText("Page 1 of 3");
});

test("filter narrows rows by email", async ({ page }) => {
  await goto(page);

  await filterInput(page).fill("ken");

  const rows = table(page).locator("tbody tr");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("ken99@example.com");
  await expect(pageReadout(page)).toHaveText("Page 1 of 1");
  await expect(selectedSummary(page)).toHaveText("0 of 1 row(s) selected.");

  // Clearing the filter restores every row.
  await filterInput(page).fill("");
  await expect(table(page).locator("tbody tr")).toHaveCount(5);
  await expect(selectedSummary(page)).toHaveText("0 of 12 row(s) selected.");
});

test("filter matches a case-insensitive fragment anywhere in the email, not just a prefix", async ({ page }) => {
  await goto(page);

  // shadcn/ui's own Data Table filters the email column through TanStack
  // Table's default `includesString` filter fn -- a case-insensitive
  // substring match anywhere in the cell, not restricted to the start of
  // the string. "REK" is an uppercase, mid-to-end-of-string fragment of
  // derek@example.com (the tail of "derek"), not a prefix of any PAYMENTS
  // email -- a filter that only matched from the start would narrow to 0
  // rows here instead of 1.
  await filterInput(page).fill("REK");

  const rows = table(page).locator("tbody tr");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("derek@example.com");
  await expect(selectedSummary(page)).toHaveText("0 of 1 row(s) selected.");
});

test("select-all checks every visible row and the selected count updates", async ({ page }) => {
  await goto(page);

  const checkboxesBefore = rowCheckboxes(page);
  await expect(checkboxesBefore).toHaveCount(5);
  for (const checkbox of await checkboxesBefore.all()) {
    await expect(checkbox).toHaveAttribute("aria-checked", "false");
  }

  await selectAllCheckbox(page).click();

  for (const checkbox of await rowCheckboxes(page).all()) {
    await expect(checkbox).toHaveAttribute("aria-checked", "true");
  }
  await expect(selectAllCheckbox(page)).toHaveAttribute("aria-checked", "true");
  await expect(selectedSummary(page)).toHaveText("5 of 12 row(s) selected.");

  // Unchecking Select All clears exactly those 5 again.
  await selectAllCheckbox(page).click();
  await expect(selectedSummary(page)).toHaveText("0 of 12 row(s) selected.");
  for (const checkbox of await rowCheckboxes(page).all()) {
    await expect(checkbox).toHaveAttribute("aria-checked", "false");
  }
});

test("Next/Previous move pages and disable at the ends", async ({ page }) => {
  await goto(page);

  await expect(previousButton(page), "disabled on the first page").toBeDisabled();
  await expect(nextButton(page)).toBeEnabled();

  await nextButton(page).click();
  await expect(pageReadout(page)).toHaveText("Page 2 of 3");
  await expect(previousButton(page)).toBeEnabled();
  await expect(nextButton(page)).toBeEnabled();
  await expect(table(page).locator("tbody tr")).toHaveCount(5);

  await nextButton(page).click();
  await expect(pageReadout(page)).toHaveText("Page 3 of 3");
  await expect(table(page).locator("tbody tr"), "last page is a short page (12 rows, 5/page)").toHaveCount(2);
  await expect(nextButton(page), "disabled on the last page").toBeDisabled();
  await expect(previousButton(page)).toBeEnabled();

  await previousButton(page).click();
  await expect(pageReadout(page)).toHaveText("Page 2 of 3");
});

test.describe("Axe automated scan", () => {
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page);
    await expect(table(page)).toBeVisible();
    await expectNoAxeViolations(page, "data_table: loaded", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });

  test("rows-per-page select open has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page);
    await pageSizeControl(page).click();
    await expect(page.getByRole("listbox")).toBeVisible();
    await expectNoAxeViolations(page, "data_table: rows-per-page select open", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
