import { test, expect } from "@playwright/test";

/**
 * Row 32 migration oracle — every themed component must render UNHASHED
 * `dx-` classes AND still have its own stylesheet actually delivered.
 *
 * Rule source: this repo's own row 32 decision (`docs/backlog.md`) — plain
 * `dx-`-prefixed CSS with no `#[css_module]` class hashing. Not an external
 * conformance rule, so this lives with the component specs rather than under
 * `oracle/` (whose three tiers are APG / HTML / Radix-opinion).
 *
 * Why it must exist: dropping `#[css_module]` removes two things at once and
 * only one of them is the point. The macro hashed class names (what row 32
 * wants gone) but it ALSO bundled and injected the stylesheet (what must keep
 * working). A migration that deletes the macro and forgets the delivery half
 * leaves a component rendering a correct-looking `class` attribute with no CSS
 * behind it — and NOTHING else in this suite would notice, because every other
 * spec selects by role and accessible name, never by class or appearance. That
 * silent-failure shape is what this file exists to make loud.
 *
 * Three assertions per component, all generic rather than per-component:
 *
 *   1. **Delivery** — some stylesheet reachable from the document defines at
 *      least one rule whose selector mentions `.dx-<component>`. This works
 *      generically precisely because `scripts/check-dx-class-prefix.sh`
 *      guarantees every class a component's own stylesheet defines is
 *      namespaced under that prefix, so the prefix is a sound proxy for "this
 *      component's sheet arrived". If the `document::Link` were missing, no
 *      such rule would be present anywhere.
 *   2. **Unhashed** — no element anywhere on the page carries a class of the
 *      shape `dx-…-<8 hex>`, the exact signature `#[css_module]` used to
 *      append (`.dx-checkbox` -> `.dx-checkbox-7ca1a710`).
 *   3. **Composed delivery** (docs/backlog.md row 51) — assertion 1 only ever
 *      checks the CURRENT route's own prefix, so it cannot see a themed
 *      component silently losing a DIFFERENT component's CSS by composing
 *      that other component's building blocks directly (`date_picker`
 *      rendering calendar's `CalendarRoot`/`CalendarMonthView` without ever
 *      rendering `calendar::Calendar`, the one place the `Link` used to
 *      live — the exact row-51 bug). This assertion instead looks at every
 *      `dx-<known-component>` prefix actually rendered ANYWHERE on the page,
 *      for every element, and requires each one's own stylesheet to be
 *      reachable too — so it catches that shape for any pair of components,
 *      not just calendar/date_picker, and needs no per-component list of
 *      "what does X compose" to stay accurate as composition changes.
 *
 * Deliberately generic: an earlier draft used a hand-written locator and
 * computed-property pair per component, which meant ~45 bespoke, fragile
 * selectors asserting the same two facts. The prefix invariant the lint
 * already enforces makes that unnecessary.
 */

/** Every component migrated off `#[css_module]` that ships its own stylesheet. */
const MIGRATED: string[] = [
  "accordion",
  "alert_dialog",
  "aspect_ratio",
  "avatar",
  "badge",
  "button",
  "calendar",
  "card",
  "checkbox",
  "collapsible",
  "color_picker",
  "combobox",
  "context_menu",
  "date_picker",
  "dialog",
  "drag_and_drop_list",
  "dropdown_menu",
  "form",
  "hover_card",
  "input",
  "item",
  "label",
  "menubar",
  "navbar",
  "pagination",
  "popover",
  "progress",
  "radio_group",
  "select",
  "separator",
  "sheet",
  "sidebar",
  "skeleton",
  "slider",
  "switch",
  "tabs",
  "tag_group",
  "textarea",
  "toast",
  "toggle",
  "toggle_group",
  "toolbar",
  "tooltip",
];

/** Every known component's own `dx-` prefix, longest first so a prefix
 * match against a rendered class picks the most specific owner (matters if
 * one component's name is itself a prefix of another's -- not the case in
 * `MIGRATED` today, but this keeps the mapping correct if it ever is). */
const KNOWN_PREFIXES = [...MIGRATED]
  .map((n) => `dx-${n.replace(/_/g, "-")}`)
  .sort((a, b) => b.length - a.length);

/** Which known component (if any) a rendered `dx-…` class belongs to,
 * e.g. `dx-calendar-grid-cell` -> `dx-calendar`. */
function ownerPrefix(cls: string): string | null {
  return (
    KNOWN_PREFIXES.find((p) => cls === p || cls.startsWith(`${p}-`)) ?? null
  );
}

/** Search EVERY frame, not just the top document, for a CSS rule whose
 * selector mentions `.prefix`. Some components' demos (`sidebar`) render
 * entirely inside `/component/block/` iframes, which are separate documents
 * with their own stylesheets -- the parent page then legitimately carries no
 * rule for that component, and checking only the top document would report
 * a delivery failure that isn't one. */
async function prefixReachesAnyFrame(
  page: import("@playwright/test").Page,
  prefix: string,
): Promise<boolean> {
  const findIn = async (frame: import("@playwright/test").Frame) =>
    frame.evaluate((pfx) => {
      for (const sheet of Array.from(document.styleSheets)) {
        let rules: CSSRuleList;
        try {
          // A cross-origin sheet throws on access; none of ours are, but a
          // vendored one could be, and it is never the sheet we are looking
          // for, so skipping it is correct rather than merely convenient.
          rules = (sheet as CSSStyleSheet).cssRules;
        } catch {
          continue;
        }
        for (const rule of Array.from(rules)) {
          const selector = (rule as CSSStyleRule).selectorText;
          if (selector && selector.includes(`.${pfx}`)) return true;
        }
      }
      return false;
    }, prefix);

  for (const frame of page.frames()) {
    if (await findIn(frame).catch(() => false)) return true;
  }
  return false;
}

for (const name of MIGRATED) {
  test(`${name}: stylesheet delivered, classes unhashed`, async ({ page }) => {
    await page.goto(`http://127.0.0.1:8080/component/?name=${name}&`);
    // The page's own demo must have rendered before the sheet is meaningful.
    await expect(page.locator("body")).toBeVisible();

    const prefix = `dx-${name.replace(/_/g, "-")}`;

    // Polled rather than sampled once: an iframe-embedded demo may not have
    // attached its own stylesheet at the moment the parent finishes loading,
    // and `page.frames()` itself grows as those frames appear.
    await expect
      .poll(() => prefixReachesAnyFrame(page, prefix), {
        message:
          `no CSS rule mentioning .${prefix} reached any frame of the page — ` +
          `the component's own stylesheet was not delivered (a missing ` +
          `document::Link after the #[css_module] removal, docs/backlog.md row 32)`,
        timeout: 15000,
      })
      .toBe(true);

    const hashed = await page.evaluate(() =>
      Array.from(new Set(
        Array.from(document.querySelectorAll("[class]"))
          .flatMap((el) => Array.from(el.classList))
          .filter((cls) => /^dx-.+-[0-9a-f]{8}$/.test(cls)),
      )),
    );
    expect(
      hashed,
      `found css_module-style hashed class names still in the DOM`,
    ).toEqual([]);

    // Assertion 3 (docs/backlog.md row 51): every OTHER known component's
    // prefix that actually shows up on a rendered element's class list must
    // be delivered too -- catches a themed component composing another
    // component's building blocks (rather than its top-level entry point)
    // and silently losing that component's CSS, the exact row-51 shape.
    // Gathered across every frame for the same iframe-embedded-demo reason
    // as assertion 1.
    //
    // Scoped to `article.dx-component-page` (this route's own demo content:
    // header, live preview, code block) rather than the whole document,
    // falling back to `document.body` on routes that don't have that
    // wrapper (`ComponentBlockDemo`'s bare iframe content, which has no
    // site chrome anyway). This deliberately excludes the sitewide
    // navbar/footer/language-picker chrome that wraps every route: it hand-
    // rolls a `dx-navbar-*`/`dx-select-expand-icon`-prefixed class or two of
    // its own (collisions with the `navbar`/`select` components' own
    // prefixes, confirmed by reading `preview/src/main.rs` and
    // `preview/assets/main.css`/`language-select.css`) that are unrelated
    // to this route's OWN composition and would otherwise make this
    // assertion fail on almost every component page for a reason that has
    // nothing to do with row 51.
    const composedPrefixes = new Set<string>();
    for (const frame of page.frames()) {
      const classesHere = await frame
        .evaluate(() => {
          const scope =
            document.querySelector("article.dx-component-page") ??
            document.body;
          return Array.from(new Set(
            Array.from(scope.querySelectorAll("[class]"))
              .flatMap((el) => Array.from(el.classList)),
          ));
        })
        .catch(() => [] as string[]);
      for (const cls of classesHere) {
        const owner = ownerPrefix(cls);
        if (owner && owner !== prefix) composedPrefixes.add(owner);
      }
    }

    for (const composed of composedPrefixes) {
      await expect
        .poll(() => prefixReachesAnyFrame(page, composed), {
          message:
            `elements with a .${composed} class render on the "${name}" route, ` +
            `but no CSS rule mentioning .${composed} reached any frame — this ` +
            `component composes ${composed.replace(/^dx-/, "")}'s markup without ` +
            `delivering its stylesheet (docs/backlog.md row 51)`,
          timeout: 15000,
        })
        .toBe(true);
    }
  });
}
