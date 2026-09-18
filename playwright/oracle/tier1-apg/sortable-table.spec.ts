/**
 * ORACLE: tier 1 (APG) — Table pattern, sortable columns (`aria-sort`).
 *
 * Rule source, quoted verbatim:
 *
 *   "If the table contains sortable columns or rows, aria-sort is set to
 *   an appropriate value on the header cell element for the sorted column
 *   or row as described in the Grid and Table Properties Practice."
 *   — APG Table pattern, "WAI-ARIA Roles, States, and Properties"
 *     (pinned checkout's `content/patterns/table/table-pattern.html`,
 *     section `#roles_states_properties`)
 *     https://www.w3.org/WAI/ARIA/apg/patterns/table/
 *
 *   "aria-sort="value" ... Set on the currently sorted column. When the
 *   sorted column is changed, the aria-sort attribute is removed and set
 *   on the newly sorted column."
 *   "A value of "ascending" indicates the data cells in the column are
 *   sorted in ascending order."
 *   "A value of "descending" indicates the data cells in the column are
 *   sorted in descending order."
 *   — APG Table pattern, Sortable Table Example, "Role, Property, State,
 *     and Tabindex Attributes" (`../reference/7e4034b/content/patterns/
 *     table/examples/sortable-table.html`, section `#rps_label`)
 *
 *   "Not applicable: The only interactive elements are HTML button
 *   elements, and all their keyboard functionality is provided by
 *   browsers."
 *   — same page, "Keyboard Support" (section `#kbd_label`)
 *
 *   "The aria-sort attribute is set on the column header of the currently
 *   sorted column, and the header text of sortable columns is wrapped in
 *   a button element."
 *   — same page, "About This Example"
 *
 * From these, three rules (R1-R3 below):
 *
 *   R1. Every sortable column header exposes an `aria-sort` state (either
 *       the literal attribute, or its documented default, `"none"`, when
 *       absent — see the R1 assertion helper's own comment for why an
 *       absent attribute still counts as "exposing" the state), and
 *       exactly one sortable header carries `ascending`/`descending` at a
 *       time (never zero, per "aria-sort is removed and set on the newly
 *       sorted column" — the column changes, one is always active; never
 *       two, since the *singular* "the currently sorted column" rules out
 *       more than one).
 *   R2. Activating the active (ascending) column's sort button toggles it
 *       to `descending` and the table's first data row changes
 *       accordingly (the reference's own `sortColumn`/`setColumnHeaderSort`
 *       behaviour this prose describes).
 *   R3. The sort control is a real `button` element, a child of the `th`
 *       it sorts, operable by keyboard (focusable, activated by Enter) --
 *       "all their keyboard functionality is provided by browsers" is only
 *       true if the control really is a native `button`, not a `div`/`span`
 *       with a click handler.
 *
 * Calibration (`tier1-apg/README.md`): every rule below runs against BOTH
 * subjects --
 *
 *   - the pattern's own vendored, executable example page
 *     (`../reference/7e4034b/content/patterns/table/examples/
 *     sortable-table.html`, loaded over `file://`, the same technique
 *     `focus-restore-reference.spec.ts`/`menu-roles.spec.ts` already use);
 *   - this crate's `DataTable` composition
 *     (`/component/?name=data_table&`, `DataTableColumnHeader`).
 *
 * If either subject fails a rule below, treat the *rule* as suspect first
 * (conformance-harness.md's calibration principle) -- though for the
 * reference page specifically, a red result more likely means the pinned
 * commit's example changed shape than that the rule itself is wrong.
 *
 * Locating "the sortable column headers" is necessarily subject-specific
 * (the two pages' markup differs beyond what the rule constrains), so each
 * `describe` block below supplies its own locator for that; the rule
 * assertions themselves (R1/R2/R3) are one shared function each, run
 * against both.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { BASE_URL } from "../../base-url";

const sortableTableUrl = pathToFileURL(
  path.resolve(
    __dirname,
    "../reference/7e4034b/content/patterns/table/examples/sortable-table.html",
  ),
).href;

const gotoDataTable = (page: Page) =>
  page.goto(`${BASE_URL}/component/?name=data_table&`, { timeout: 20 * 60 * 1000 });

/**
 * R1: every locator in `sortableHeaders` exposes a valid `aria-sort` state,
 * and exactly one carries `ascending`/`descending`.
 *
 * An absent `aria-sort` attribute is treated as the value `"none"` --
 * `aria-sort`'s own documented default -- so a header that omits the
 * attribute entirely (as every *inactive* sortable header on the vendored
 * reference page does; only the currently-sorted one carries the literal
 * attribute there) still counts as "exposing" the property: assistive
 * tech resolves the same default either way. `DataTableColumnHeader`
 * instead sets the attribute explicitly on every sortable header,
 * `"none"` included (see its own doc comment) -- both are valid per this
 * rule, which is exactly why the rule is phrased in terms of the
 * *effective*, not literal, attribute.
 */
async function assertExactlyOneActiveSort(sortableHeaders: Locator, label: string): Promise<void> {
  const headers = await sortableHeaders.all();
  expect(headers.length, `${label}: expected at least one sortable column header`).toBeGreaterThan(0);

  let activeCount = 0;
  for (const th of headers) {
    const raw = await th.getAttribute("aria-sort");
    const effective = raw ?? "none";
    expect(
      ["ascending", "descending", "none"],
      `${label}: aria-sort must be a valid APG token (got "${raw}")`,
    ).toContain(effective);
    if (effective === "ascending" || effective === "descending") {
      activeCount += 1;
    }
  }
  expect(
    activeCount,
    `${label}: exactly one sortable column header must carry ascending/descending at a time (found ${activeCount})`,
  ).toBe(1);
}

/**
 * R3: `sortButton` is a real `<button>`, a child of a `<th>`, and
 * operable by keyboard (focus, then Enter toggles the sort -- checked by
 * the caller comparing `header`'s `aria-sort` before/after).
 */
async function assertKeyboardOperableButtonInHeader(
  page: Page,
  sortButton: Locator,
  header: Locator,
  label: string,
): Promise<void> {
  expect(await sortButton.evaluate((el) => el.tagName.toLowerCase()), `${label}: sort control must be a <button>`).toBe(
    "button",
  );
  expect(
    await sortButton.evaluate((el) => el.closest("th") !== null),
    `${label}: sort <button> must be a descendant of the <th> it sorts`,
  ).toBe(true);

  const before = await header.getAttribute("aria-sort");
  await sortButton.focus();
  await expect(sortButton, `${label}: sort button must be keyboard-focusable`).toBeFocused();
  await page.keyboard.press("Enter");
  const after = await header.getAttribute("aria-sort");
  expect(after, `${label}: Enter on the focused sort button must toggle aria-sort`).not.toBe(before);
}

test.describe("CALIBRATION — APG Table pattern's own vendored sortable-table example", () => {
  // Scoped to #ex1 -- the page also renders a second, unrelated
  // "Role, Property, State, and Tabindex Attributes" reference table
  // further down (`table.data.attributes`, no sort behaviour at all).
  const table = (page: Page) => page.locator("#ex1 table.sortable");
  const sortableHeaders = (page: Page) => table(page).locator("th").filter({ has: page.locator("button") });
  // Column order in the static markup: First Name(0), Last Name(1),
  // Company(2), Address(3, no-sort/no button), Favorite Number(4).
  // Last Name starts `aria-sort="ascending"` in the page's own HTML.
  const lastNameHeader = (page: Page) => table(page).locator("thead th").nth(1);
  const lastNameButton = (page: Page) => lastNameHeader(page).locator("button");
  const firstRowLastName = (page: Page) => table(page).locator("tbody tr").first().locator("td").nth(1);

  test("R1: every sortable header exposes aria-sort; exactly one is active", async ({ page }) => {
    await page.goto(sortableTableUrl);
    await assertExactlyOneActiveSort(sortableHeaders(page), "reference");
  });

  test("R2: activating the sorted column's button toggles ascending -> descending and reorders the first row", async ({
    page,
  }) => {
    await page.goto(sortableTableUrl);

    await expect(lastNameHeader(page), "starts ascending (the page's own static markup)").toHaveAttribute(
      "aria-sort",
      "ascending",
    );
    const before = (await firstRowLastName(page).textContent())?.trim();
    expect(before, "first row's Last Name, ascending").toBe("Jackson");

    await lastNameButton(page).click();

    await expect(lastNameHeader(page), "toggles to descending").toHaveAttribute("aria-sort", "descending");
    const after = (await firstRowLastName(page).textContent())?.trim();
    expect(after, "first row's Last Name must change").not.toBe(before);
    expect(after, "first row's Last Name, descending").toBe("Jensen");
  });

  test("R3: the sort control is a real <button> inside the <th>, operable by keyboard", async ({ page }) => {
    await page.goto(sortableTableUrl);
    await assertKeyboardOperableButtonInHeader(page, lastNameButton(page), lastNameHeader(page), "reference");
    // Enter from the ascending default must land on descending, the same
    // direction R2's click produces.
    await expect(lastNameHeader(page)).toHaveAttribute("aria-sort", "descending");
  });
});

test.describe("APG Table pattern (sortable columns) — DataTable", () => {
  // `Table`'s own `data-slot="table"` (preview/src/components/table/
  // component.rs) scopes past anything else `/component/?name=data_table&`
  // renders on the same page (docs prose, source-code viewer, ...).
  const table = (page: Page) => page.locator('[data-slot="table"]');
  // Only `DataTableColumnHeader` sets `aria-sort` (see its own doc); the
  // Select-All checkbox header, ID, and Status headers don't, so
  // `th[aria-sort]` -- unlike a `has: button` filter -- correctly excludes
  // the checkbox column's own `<button>` (Checkbox's underlying element).
  const sortableHeaders = (page: Page) => table(page).locator("th[aria-sort]");
  // Default sort (variants/main/mod.rs's `Demo`): Email, ascending.
  const emailHeader = (page: Page) => table(page).getByRole("columnheader", { name: "Email" });
  const emailButton = (page: Page) => emailHeader(page).getByRole("button");
  // Column order in the demo: select-checkbox(0), ID(1), Status(2),
  // Email(3), Amount(4).
  const firstRowEmail = (page: Page) => table(page).locator("tbody tr").first().locator("td").nth(3);

  test("R1: every sortable header exposes aria-sort; exactly one is active", async ({ page }) => {
    await gotoDataTable(page);
    await assertExactlyOneActiveSort(sortableHeaders(page), "data_table");
  });

  test("R2: activating the sorted column's button toggles ascending -> descending and reorders the first row", async ({
    page,
  }) => {
    await gotoDataTable(page);

    await expect(emailHeader(page), "starts ascending (the demo's default sort)").toHaveAttribute(
      "aria-sort",
      "ascending",
    );
    const before = (await firstRowEmail(page).textContent())?.trim();
    expect(before, "first row's Email, ascending").toBe("alexandra@example.com");

    await emailButton(page).click();

    await expect(emailHeader(page), "toggles to descending").toHaveAttribute("aria-sort", "descending");
    const after = (await firstRowEmail(page).textContent())?.trim();
    expect(after, "first row's Email must change").not.toBe(before);
    expect(after, "first row's Email, descending").toBe("zara@example.com");
  });

  test("R3: the sort control is a real <button> inside the <th>, operable by keyboard", async ({ page }) => {
    await gotoDataTable(page);
    await assertKeyboardOperableButtonInHeader(page, emailButton(page), emailHeader(page), "data_table");
    await expect(emailHeader(page)).toHaveAttribute("aria-sort", "descending");
  });
});
