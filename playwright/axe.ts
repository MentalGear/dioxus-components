/**
 * ORACLE: static accessibility rules (axe-core), shared across specs.
 *
 * Source: `axe-core`'s own rule set, tagged to the WCAG success criteria it
 * implements (`wcag2a`/`wcag2aa`/`wcag21a`/`wcag21aa`) plus its curated
 * `best-practice` rules (things every mature a11y linter flags -- e.g.
 * duplicate landmarks -- that are not themselves a numbered WCAG SC).
 *
 * This is a *different* class of check than every other oracle in this
 * harness. Tiers 1-3 (`docs/conformance-harness.md`) test *behaviour*:
 * does Escape move focus where APG says, does a required control block
 * form submission, does an anchored overlay track its trigger. axe tests
 * *static* accessibility rules on whatever DOM exists at the moment it
 * runs: valid ARIA role/attribute combinations, accessible names, unique
 * landmarks, label association, contrast, list/table structure. A
 * component can pass every keyboard/behaviour oracle in this repo and
 * still fail axe (a duplicate landmark, an invalid `aria-required` on a
 * `button`, a `role="menu"` popup with no accessible name) -- exactly the
 * defect class this file exists to catch, and exactly the class that
 * caught `docs/backlog.md` row 25 (ContextMenu/Menubar menu popups with no
 * accessible name) and the toast/aria-required incidents this round's
 * brief cites.
 *
 * ## The two-state convention
 *
 * Every component spec that reaches an open/expanded/selected state scans
 * twice: once after the page loads (the component's *closed*, at-rest
 * markup), and once after its existing "open the overlay" step (its
 * *open* markup) -- reusing whatever locator/interaction that spec already
 * has for opening it, never a new fixture invented for this file. A
 * component with no such state (e.g. a static list, an already-always-
 * visible control) scans once, at load.
 *
 * ## Exclusions must be audited, never used to force green
 *
 * `disableRules` takes a `reason` per rule id and refuses (throws) an
 * empty one -- see `expectNoAxeViolations` below. This is deliberate:
 * excluding a rule silently is how a real, unfixed defect goes invisible
 * to every future run. A rule may be excluded ONLY for a documented false
 * positive at that specific scan site (axe misjudging something that is
 * not actually a defect there); a real, larger issue this round chose not
 * to fix (contrast, a structural markup change) must stay enabled and RED,
 * recorded in `docs/backlog.md` instead. The three pre-existing
 * `color-contrast` exclusions (`preview.spec.ts`,
 * `drag_and_drop_list.spec.ts`, `tag_group.spec.ts`) predate this rule and
 * are grandfathered by this round's own instruction to refactor their
 * *coverage* unchanged; they are not false positives (the theme's contrast
 * ratios are a real, open, tracked gap -- `docs/backlog.md` row 31/32) and
 * their reason strings say so rather than pretending otherwise.
 *
 * ## `CONTRAST_TRACKED_ELSEWHERE` — the one sitewide exclusion
 *
 * Running the full coverage this round adds (below) with no exclusion at
 * all surfaces `color-contrast` on very nearly every scan, at every state,
 * on every route -- not because each component under test has its own
 * contrast defect, but because the shared chrome present on every page
 * (the docs sidebar's nav links/section headings, the site footer's links
 * and "Built with Dioxus." line) already fails the same 4.5:1 threshold
 * everywhere it appears. That is a real, already-diagnosed, already-filed
 * gap (`docs/backlog.md` rows 31/32 -- the design-token and styling-engine
 * work, not yet landed, that both name contrast as in-scope), not a
 * per-scan false positive, and not something this round can fix by
 * construction (a theme-wide token/value change, explicitly out of this
 * round's remit per its own instructions on contrast failures). Leaving it
 * enabled would not surface anything new after the first finding -- every
 * one of dozens of new scan sites would independently rediscover the exact
 * same footer/sidebar nodes, drowning any real, component-specific finding
 * in repetition. `CONTRAST_TRACKED_ELSEWHERE` is exported so every call
 * site excludes it the same, explicit, auditable way (per this file's own
 * exclusion-with-reason rule) rather than 60-odd copies of the same reason
 * string -- pass it in `exclude` wherever a scan would otherwise be pure
 * chrome-contrast noise. It must never be reached for on a *component's
 * own* contrast defect discovered independently of the shared chrome.
 */

import { expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

/** The WCAG/best-practice tag set every scan in this repo runs against. */
const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"];

/** One excluded rule id, with the mandatory reason it is safe to exclude here. */
export interface AxeExclusion {
  /** The axe rule id (or ids) to disable, e.g. "color-contrast". */
  ids: string | string[];
  /**
   * Why this specific scan site is a false positive for this rule (or, for
   * the three grandfathered pre-existing exclusions, why their coverage is
   * being preserved unchanged rather than newly claimed as a false
   * positive). Must be non-empty -- see `expectNoAxeViolations`.
   */
  reason: string;
}

/**
 * Shared exclusion for the sitewide `color-contrast` chrome finding -- see
 * this file's header doc ("`CONTRAST_TRACKED_ELSEWHERE`") for why this
 * exists and when it's safe to use. Pass it in `exclude`, e.g.
 * `{ exclude: [CONTRAST_TRACKED_ELSEWHERE] }`.
 */
export const CONTRAST_TRACKED_ELSEWHERE: AxeExclusion = {
  ids: "color-contrast",
  reason:
    "sitewide chrome (docs sidebar nav links/headings, site footer) fails " +
    "the same 4.5:1 threshold on every route -- a real, already-tracked " +
    "gap (docs/backlog.md rows 31/32, design tokens + styling engine, not " +
    "yet landed), not a per-scan false positive and not fixable here (a " +
    "theme-wide value change, explicitly out of this round's remit); " +
    "excluded so it doesn't drown a component's own findings in the same " +
    "repeated footer/sidebar nodes on every one of dozens of scan sites",
};

export interface AxeScanOptions {
  /** Scope the scan to one or more CSS selectors (axe's `.include()`). */
  include?: string | string[];
  /** Rule exclusions, each requiring a written `reason` (see `AxeExclusion`). */
  exclude?: AxeExclusion[];
}

function formatViolations(
  violations: Awaited<ReturnType<AxeBuilder["analyze"]>>["violations"],
  label: string,
): string {
  const lines = [`axe: ${violations.length} violation(s) at "${label}"`];
  for (const v of violations) {
    lines.push(`  [${v.id}] impact=${v.impact ?? "unknown"} — ${v.help}`);
    lines.push(`    ${v.helpUrl}`);
    for (const node of v.nodes) {
      const html = node.html.length > 300 ? `${node.html.slice(0, 300)}…` : node.html;
      lines.push(`    - target: ${node.target.join(" ")}`);
      lines.push(`      html:   ${html}`);
    }
  }
  return lines.join("\n");
}

/**
 * Run axe-core's full WCAG 2.0/2.1 A+AA + best-practice rule set against
 * `page` (or a subset of it, via `opts.include`) and fail with a readable
 * table (rule id, impact, help URL, every offending node's target + html)
 * if anything is found. `label` identifies the scan site in that message
 * (spec name + state, e.g. "context-menu: submenu open").
 */
export async function expectNoAxeViolations(
  page: Page,
  label: string,
  opts: AxeScanOptions = {},
): Promise<void> {
  let builder = new AxeBuilder({ page }).withTags(TAGS);

  if (opts.include) {
    builder = builder.include(opts.include);
  }

  const disabledIds: string[] = [];
  for (const exclusion of opts.exclude ?? []) {
    if (!exclusion.reason || exclusion.reason.trim().length === 0) {
      throw new Error(
        `expectNoAxeViolations("${label}"): exclusion of rule(s) ` +
          `${JSON.stringify(exclusion.ids)} requires a non-empty "reason" ` +
          `(axe.ts's exclusion-with-reason rule — see this file's header doc)`,
      );
    }
    disabledIds.push(...(Array.isArray(exclusion.ids) ? exclusion.ids : [exclusion.ids]));
  }
  if (disabledIds.length > 0) {
    builder = builder.disableRules(disabledIds);
  }

  const results = await builder.analyze();
  expect(results.violations, formatViolations(results.violations, label)).toEqual([]);
}
