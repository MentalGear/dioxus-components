import { test, expect } from "@playwright/test";

/**
 * Row 32 migration oracle — a themed component must render an UNHASHED
 * `dx-` class AND still actually be styled by its own stylesheet.
 *
 * Rule source: this repo's own row 32 decision (`docs/backlog.md`) — plain
 * `dx-`-prefixed CSS with no `#[css_module]` class hashing. It is not an
 * external conformance rule, so this sits with the component specs rather
 * than under `oracle/` (whose three tiers are APG / HTML / Radix-opinion).
 *
 * Why it needs to exist at all: dropping `#[css_module]` removes two things
 * at once, and only one of them is the point. The macro hashed class names
 * (what row 32 wants gone) but it ALSO bundled and injected the stylesheet
 * (what must keep working). A migration that deletes the macro and forgets
 * the delivery half leaves the component rendering with a correct-looking
 * `class` attribute and no CSS behind it — which no existing spec would
 * catch, because every other spec in this suite selects by role and name,
 * never by class or appearance. That is exactly the silent-failure shape
 * this file exists to make loud.
 *
 * Each case therefore asserts BOTH halves:
 *   1. the rendered `class` carries the plain `dx-…` name with no
 *      `-<8 hex>` suffix (the hash is gone), and
 *   2. a computed property that ONLY that component's own `style.css`
 *      supplies is present — so the sheet is provably reaching the element.
 *      A UA default (`border-radius: 0px`, `width: auto`) means the CSS
 *      never arrived.
 *
 * Extend `MIGRATED` as each component moves; a component absent from this
 * table is simply not yet migrated.
 */

interface MigratedCase {
  /** Component folder name, i.e. the `?name=` route segment. */
  name: string;
  /** The unhashed class expected on the probed element. */
  className: string;
  /** Locates the element carrying `className` on that component's page. */
  probe: (page: import("@playwright/test").Page) => import("@playwright/test").Locator;
  /**
   * A computed property whose value comes only from the component's own
   * stylesheet, plus the UA-default value that would mean "no CSS arrived".
   */
  styled: { property: string; notDefault: string };
}

const MIGRATED: MigratedCase[] = [
  {
    name: "checkbox",
    className: "dx-checkbox",
    probe: (page) => page.getByRole("checkbox").first(),
    // checkbox/style.css gives it a 4px radius; unstyled would be 0px.
    styled: { property: "border-radius", notDefault: "0px" },
  },
  {
    name: "select",
    className: "dx-select-trigger",
    probe: (page) =>
      page.getByRole("button").filter({ hasText: /Select an option|Choose/ }).first(),
    // select/style.css gives the trigger an 8px radius; unstyled would be 0px.
    styled: { property: "border-radius", notDefault: "0px" },
  },
];

for (const kase of MIGRATED) {
  test(`${kase.name}: renders an unhashed ${kase.className} and is styled by its own sheet`, async ({
    page,
  }) => {
    await page.goto(`http://127.0.0.1:8080/component/?name=${kase.name}&`);
    const el = kase.probe(page);
    await expect(el).toBeVisible();

    const cls = await el.getAttribute("class");
    expect(cls, `expected the plain class on ${kase.name}`).toContain(kase.className);
    // `#[css_module]` appended an 8-hex-digit scope suffix
    // (`.dx-checkbox` -> `.dx-checkbox-7ca1a710`) to the DOM and the
    // stylesheet alike. Its absence is the migration's own signature.
    expect(cls, `expected NO css_module hash suffix on ${kase.name}`).not.toMatch(
      new RegExp(`${kase.className}-[0-9a-f]{8}`),
    );

    const value = await el.evaluate(
      (node, property) => getComputedStyle(node).getPropertyValue(property),
      kase.styled.property,
    );
    expect(
      value,
      `${kase.name}'s own stylesheet did not reach the element: ${kase.styled.property} is the UA default, so the class is right but the CSS never arrived`,
    ).not.toBe(kase.styled.notDefault);
  });
}
