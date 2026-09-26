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
 * `EXCLUDE_VENDORED_CODE_HIGHLIGHT` — **fixed by construction, row 39
 * residue closed.** This used to scope out `.dx-preview-code-theme` --
 * every syntax-highlighted code span this app renders, both the "Manual
 * installation"/component-source code viewer (`preview/src/main.rs`'s
 * `CodeBlock`, which wraps a `PreviewCode`) and the same highlighter's
 * output embedded directly in a component's markdown-rendered "Usage
 * notes" prose -- because its comment token measured 4.39:1, from the
 * vendored, build-time-generated `github-light` syntax-highlighting
 * theme. That theme's CSS is NOT source this repo owns -- it's generated
 * fresh from the `dioxus-code`/`arborium-theme` crates (crates.io) into
 * `docs/assets/github-light*.css` on every `scripts/deploy-preview.sh`
 * run, so a fix committed to that generated file is silently reverted by
 * the next deploy. The real fix lives in `preview/assets/main.css`
 * instead: an override rule, `.dx-preview-code-theme <theme class>`
 * (two classes, beating the theme's own single-class rule on
 * specificity regardless of stylesheet load order), for the theme's
 * three comment-family custom properties
 * (`--dxc-*-a-c-color`/`--dxc-*-a-cd-color`/`--dxc-*-var-muted`) across
 * every class variant the theme crate generates for `github-light`
 * (plain, `-system-light-`, `-system-dark-`). Root cause: those
 * properties carry the upstream `arborium-theme` crate's `#6e7781`,
 * which clears 4.5:1 against `#ffffff`/`#f2f2f2` but not against this
 * app's actual code-block background, `--primary-color-1`'s light value
 * `#fbfbfb` (`.dx-preview-code-theme`'s own `background-color`,
 * `preview/assets/main.css`) -- 4.39:1. Every other token in the theme
 * already cleared 4.5:1 against `#fbfbfb` (measured: `#1f2328` 15.27:1,
 * `#8250df` 4.88:1, `#0550ae` 7.34:1, `#0a3069` 12.38:1, `#cf222e`
 * 5.18:1, `#953800`/`#116329` 7.14:1, `#0969da` 5.02:1), so the comment
 * token was the one change needed: `#6e7781` -> `#59636e` (GitHub's own
 * newer light-theme comment color), 5.91:1 against `#fbfbfb` (6.11:1
 * against the theme's own claimed `#ffffff`/`#fff` background, 5.46:1
 * against its `#f2f2f2` surface token) -- same hue family, same visual
 * register as a comment, now compliant everywhere it's actually
 * rendered. See `preview/assets/main.css`'s own comment on that override
 * rule for the full detail. The paired `github-dark` theme (used for
 * `data-theme=dark`, a different theme entirely, not this constant's
 * concern) was checked too: its comment token `#8b949e` already clears
 * 6.28:1 against this app's dark code-block background
 * (`--primary-color-1`'s dark value, `#0e0e0e`), and every other
 * `github-dark` token clears at least 6.28:1 as well, so it needed no
 * change, and is untouched.
 *
 * `EXCLUDE_VENDORED_CODE_HIGHLIGHT` itself is kept as an exported
 * `AxeRegionExclusion` -- with a selector that now excludes nothing (see
 * `expectNoAxeViolations`'s handling of an empty selector, below) -- purely
 * for call-site compatibility: `~20+` spec files (owned by other lanes at
 * the time this was fixed) still `import` and pass it to
 * `expectNoAxeViolations({ excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] })`,
 * and rewriting every one of those call sites was out of this fix's file
 * scope. It is slated for removal (along with every call site that still
 * references it) the next time those spec files are touched.
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
 * RETAINED ONLY FOR CALL-SITE COMPATIBILITY -- slated for removal.
 *
 * The underlying defect this used to work around (the vendored
 * `github-light` syntax-highlighting theme's comment token, `#6e7781`,
 * measuring 4.39:1 against this app's actual code-block background,
 * `#fbfbfb`) was fixed by construction, in source -- NOT in the
 * crate-generated `docs/assets/github-light*.css` (regenerated, and so
 * silently reverted, by every `scripts/deploy-preview.sh` run) but as a
 * higher-specificity override in `preview/assets/main.css`, which sets
 * the theme's comment-token custom properties to `#59636e` (5.91:1) on
 * `.dx-preview-code-theme <theme class>`. `.dx-preview-code-theme` has
 * no remaining `color-contrast` violation to exclude. See this file's
 * header doc ("`EXCLUDE_VENDORED_CODE_HIGHLIGHT` — fixed by
 * construction, row 39 residue closed") and `preview/assets/main.css`'s
 * own comment on that override rule for the full before/after
 * measurement.
 *
 * The constant stays exported, with a selector that excludes nothing (see
 * `expectNoAxeViolations`'s empty-selector handling below), purely so the
 * many spec files that still `import` and pass it to
 * `expectNoAxeViolations({ excludeRegions: [...] })` keep compiling and
 * running unchanged -- rewriting each of those call sites is out of this
 * fix's scope (they belong to other lanes/rounds). Once every call site is
 * updated to drop this import, delete it along with this comment.
 */
export const EXCLUDE_VENDORED_CODE_HIGHLIGHT: AxeRegionExclusion = {
  selector: "",
  reason:
    "no-op, retained only for call-site compatibility -- the github-light " +
    "theme's comment-token contrast defect this used to exclude " +
    "(#6e7781 on #fbfbfb, 4.39:1) was fixed by construction, in source " +
    "(preview/assets/main.css overrides the theme's comment tokens -> " +
    "#59636e, 5.91:1, since the theme's own generated CSS is reverted by " +
    "every deploy); see docs/backlog.md row 39 and this file's header doc",
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
    // A no-op region exclusion (empty selector, or an array of only empty/
    // blank selectors) is ignored rather than passed to axe's `.exclude()`.
    // This is what lets `EXCLUDE_VENDORED_CODE_HIGHLIGHT` stay a real,
    // importable `AxeRegionExclusion` after its underlying defect was fixed
    // by construction (see that constant's doc comment): its selector is
    // `""`, and every existing call site keeps compiling and scanning the
    // page normally instead of passing an empty string straight to axe
    // (which is not guaranteed to safely mean "exclude nothing").
    const selectors = Array.isArray(region.selector) ? region.selector : [region.selector];
    const isNoop = selectors.every((s) => s.trim().length === 0);
    if (isNoop) {
      continue;
    }
    builder = builder.exclude(region.selector);
  }

  const results = await builder.analyze();
  expect(results.violations, formatViolations(results.violations, label)).toEqual([]);
}
