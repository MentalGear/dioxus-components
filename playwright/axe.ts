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
 * to fix (a structural markup change, a vendored third-party asset) must
 * stay enabled and RED, recorded in `docs/backlog.md` instead. The three
 * pre-existing `color-contrast` exclusions (`preview.spec.ts`,
 * `drag_and_drop_list.spec.ts`, `tag_group.spec.ts`) predate this rule and
 * are grandfathered by this round's own instruction to refactor their
 * *coverage* unchanged.
 *
 * ## `color-contrast`: fixed by construction, not excluded
 *
 * Running the full coverage this round adds with no exclusion at all
 * initially surfaced `color-contrast` on very nearly every scan, at every
 * state, on every route. Measured across 49 routes (every component page,
 * the homepage, `/docs`, `/demos`, the dashboard), the overwhelming
 * majority of that noise -- every distinct color/background/ratio
 * combination but one -- traced back to a single CSS custom property,
 * `--secondary-color-5` (`preview/assets/dx-components-theme.css`), this
 * theme's "muted secondary text" token, used across ~28 component
 * stylesheets plus the docs sidebar and site footer: its light value,
 * `#848484`, measured 3.74:1 against white (WCAG requires 4.5:1 for
 * normal text). One token-value change (`#848484` -> `#707070`, same
 * hue, clears 4.5:1 against every background it's actually paired with in
 * this app) fixed effectively the entire surface at once -- not a
 * per-scan exclusion, a real construction fix, landed in this round (see
 * `docs/backlog.md` row 39 for the full remediation list, including a
 * same-class site-CSS accent color, a component's own literal inline
 * color, and two components misusing a token meant for dark surfaces as
 * light-mode text).
 *
 * `EXCLUDE_VENDORED_CODE_HIGHLIGHT` — the one remaining exclusion, and it
 * is a genuine false positive at every site it's used: after the fix
 * above, re-measuring the same 49 routes found exactly one remaining
 * combination, inside `.dx-preview-code-theme` -- every syntax-highlighted
 * code span this app renders, both the "Manual installation"/component-
 * source code viewer (`preview/src/main.rs`'s `CodeBlock`, which wraps a
 * `PreviewCode`) and the same highlighter's output embedded directly in a
 * component's markdown-rendered "Usage notes" prose (no `CodeBlock`
 * wrapper there, so `.dx-preview-code-theme` itself, not `.dx-code-block`,
 * is the one selector both sites actually share) -- a comment token at
 * 4.39:1, from the vendored, build-time-generated `github-light`
 * syntax-highlighting theme (`preview/assets/github-light*.css`, not a
 * file this repo authors or owns the palette of; regenerating it from a
 * different highlighter theme is a real fix, but out of this round's
 * scope, filed as part of row 39). Scoped with axe's own `.exclude()` --
 * a *region* exclusion, skipping that one already-known, already-narrow
 * subtree entirely, not a page-wide `disableRules` -- so a component's own
 * contrast defect anywhere else on the same page still fails the scan.
 *
 * ## Readiness: every scan waits for the app's first paint
 *
 * `expectNoAxeViolations` waits (`waitForAppReady`, below) before it scans.
 * `page.goto()` resolving only means the CSR shell loaded, not that
 * Dioxus's wasm client has painted anything into it yet; scanning in that
 * gap intermittently produced two false violations
 * (`landmark-one-main`/`page-has-heading-one`, both against the bare
 * `<html>` element) on whichever demo page happened to be fastest. See
 * `waitForAppReady`'s own doc comment for the full mechanism and why two
 * waits, not one. This lives in the shared helper, not each of the ~72
 * call sites, per this repo's "same problem more than once -> fix the
 * class" rule.
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
 * One excluded *region*: a CSS selector axe should skip scanning entirely
 * (axe's `.exclude()`, a scan-context change -- every rule is skipped for
 * that subtree, not just one), with the mandatory reason it is safe to
 * exclude here. Scoped, unlike `AxeExclusion`/`disableRules`, which turns
 * a rule off for the *whole* page: prefer a region exclusion whenever the
 * false positive is isolated to a specific, identifiable subtree, so the
 * rest of the page (a component's own markup included) stays checked by
 * every rule.
 */
export interface AxeRegionExclusion {
  /** CSS selector (or a frame-traversal chain) to exclude from the scan. */
  selector: string | string[];
  /** Why this specific region is a false positive. Must be non-empty. */
  reason: string;
}

/**
 * The one remaining `color-contrast` exclusion after this round's
 * construction fix -- see this file's header doc ("`color-contrast`:
 * fixed by construction, not excluded") for the measurement and the fix.
 * Scoped to `.dx-preview-code-theme` (every syntax-highlighted code span
 * this app renders -- see this file's header doc for the two distinct
 * sites that share this one class), where the vendored, build-time-
 * generated `github-light` theme's comment-token color still measures
 * 4.39:1. Harmless to pass on a scan whose page has no highlighted code at
 * all -- an `.exclude()` selector matching nothing excludes nothing.
 */
export const EXCLUDE_VENDORED_CODE_HIGHLIGHT: AxeRegionExclusion = {
  selector: ".dx-preview-code-theme",
  reason:
    "vendored, build-time-generated github-light syntax-highlighting theme " +
    "(preview/assets/github-light*.css) measures 4.39:1 for comment tokens " +
    "(#6e7781 on #fbfbfb) -- a third-party theme this repo does not author " +
    "or own the palette of; filed docs/backlog.md row 39",
};

export interface AxeScanOptions {
  /** Scope the scan to one or more CSS selectors (axe's `.include()`). */
  include?: string | string[];
  /** Rule exclusions, each requiring a written `reason` (see `AxeExclusion`). */
  exclude?: AxeExclusion[];
  /** Region exclusions, each requiring a written `reason` (see `AxeRegionExclusion`). */
  excludeRegions?: AxeRegionExclusion[];
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
 * How long `waitForAppReady` will wait for either readiness signal before
 * giving up. Bounded well above every *passing* scan's observed duration
 * in this suite (1.8-8.5s, see this file's header doc's Group-B analysis)
 * but far below Playwright's own per-test ceiling (5min in this repo's
 * config), so a page that genuinely never renders (a real defect, not a
 * race) still fails fast with a clear "waiting for locator(...) to be
 * attached" timeout instead of silently eating the whole test budget.
 */
const APP_READY_TIMEOUT_MS = 15_000;

/**
 * Wait for Dioxus's first client-side paint before axe scans `page`.
 *
 * `page.goto()` resolving (the `load` event) only proves the CSR shell
 * (`preview/index.html`, an empty `<div id="main">` -- note: a `div`
 * *carrying* the id `main`, not a `<main>` element, so it can never
 * satisfy either wait below on its own) was fetched. It says nothing
 * about whether the wasm client has run its first render pass yet, and
 * for a CSR app the `load` event routinely fires before that paint.
 * Scanning in that gap is exactly what produced the two false violations
 * this wait fixes: `landmark-one-main`/`page-has-heading-one`, both
 * targeting the bare `<html>` element, on whichever demo page's scan
 * happened to run fastest (finished in ~1.3-2.0s -- well under every
 * *passing* scan's 1.8-8.5s -- because there was almost nothing in the
 * DOM yet to scan).
 *
 * Two waits, not one, because the race produces *two* violations and
 * every route this suite scans (`preview/src/main.rs`'s `Home`, `Docs`,
 * `Demos`, `ComponentHighlight`/`ComponentDemo`, the dashboard) renders
 * both together as part of the same first paint -- confirmed by reading
 * every route function in `preview/src/main.rs`, each either rendering
 * its own top-level `main { .. }` + `h1` (`Home`, `Docs`, `Demos`) or
 * composing `DocsLayout`, whose `SidebarInset` always renders the page's
 * one `<main>` (`preview/src/components/sidebar/component.rs`), around
 * content that always carries an `h1`:
 *   1. the page's one `<main>` landmark, and
 *   2. its level-one heading (an `h1`, or an ARIA `role="heading"` +
 *      `aria-level="1"` equivalent -- `getByRole` matches either).
 * Both checks use `state: "attached"` only (no visibility/animation
 * wait, no `networkidle`), so this stays cheap and matches the exact two
 * rules the race violates -- a direct fix for the observed failure mode,
 * not a general "wait a bit longer."
 */
async function waitForAppReady(page: Page): Promise<void> {
  await page
    .locator("main")
    .first()
    .waitFor({ state: "attached", timeout: APP_READY_TIMEOUT_MS });
  await page
    .getByRole("heading", { level: 1 })
    .first()
    .waitFor({ state: "attached", timeout: APP_READY_TIMEOUT_MS });
}

/**
 * Run axe-core's full WCAG 2.0/2.1 A+AA + best-practice rule set against
 * `page` (or a subset of it, via `opts.include`) and fail with a readable
 * table (rule id, impact, help URL, every offending node's target + html)
 * if anything is found. `label` identifies the scan site in that message
 * (spec name + state, e.g. "context-menu: submenu open").
 *
 * Waits for the app's first paint first (`waitForAppReady`, above) -- see
 * that function's doc comment and this file's header doc ("Readiness")
 * for why.
 */
export async function expectNoAxeViolations(
  page: Page,
  label: string,
  opts: AxeScanOptions = {},
): Promise<void> {
  await waitForAppReady(page);

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

  for (const region of opts.excludeRegions ?? []) {
    if (!region.reason || region.reason.trim().length === 0) {
      throw new Error(
        `expectNoAxeViolations("${label}"): region exclusion of ` +
          `${JSON.stringify(region.selector)} requires a non-empty "reason" ` +
          `(axe.ts's exclusion-with-reason rule — see this file's header doc)`,
      );
    }
    builder = builder.exclude(region.selector);
  }

  const results = await builder.analyze();
  expect(results.violations, formatViolations(results.violations, label)).toEqual([]);
}
