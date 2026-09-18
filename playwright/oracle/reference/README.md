# Reference — vendored APG example pages

Source: [`../../../docs/conformance-harness.md`](../../../docs/conformance-harness.md), "Tier 1 — APG" and the "Layout" section ("`reference/` — vendored APG example pages, pinned by commit").

## Purpose and rule-source policy

Tier 1's calibration subject. Per conformance-harness.md's "Calibration" table, the tier‑1 reference is "the pattern's own APG example page" — vendored rather than fetched live, so "CI must not depend on `w3.org` being up." A tier‑1 rule is expected to pass against the page vendored here; if it does not, the rule is wrong, not the component under test.

Nothing under this directory is itself a conformance rule. It is fixture data: static, self-contained copies of pages published by the W3C ARIA Authoring Practices Guide (APG), kept byte-for-byte as retrieved except for the trim described below.

## Provenance

| | |
|---|---|
| Source repository | [`w3c/aria-practices`](https://github.com/w3c/aria-practices) |
| Pinned commit | `7e4034b262bc0d25332e330d8a582aaf34113829` (short: `7e4034b`) — resolved as `origin/main` HEAD at retrieval time |
| Retrieval date | 2026-08-29 |
| Retrieval method | `git clone --filter=blob:none --no-checkout --depth 1 https://github.com/w3c/aria-practices.git`, then `git sparse-checkout set content/patterns content/shared content/images` (cone mode), then `git checkout` |
| Vendored under | `playwright/oracle/reference/7e4034b/`, preserving the source repo's directory shape from `content/` down so the pages' existing relative `href`/`src` paths resolve unmodified |

To reproduce: `git clone`, then `git checkout 7e4034b262bc0d25332e330d8a582aaf34113829`. No files were edited after copying — only files outside the vendored examples' dependency graph were left out.

The table pattern's `sortable-table.html` (and its own CSS/JS/image dependencies, see below) was added later, 2026-09-18, from a separately-retrieved checkout of the *same* pinned commit (`7e4034b262bc0d25332e330d8a582aaf34113829` -- a commit's content is immutable, so a later `git checkout` of it is byte-identical to the original 2026-08-29 retrieval; verified with `diff` against the checkout at retrieval time). Retrieval method: same sparse-checkout as above, already covering `content/patterns` (which includes `content/patterns/table`).

## Pages vendored, and why

Per conformance-harness.md's reachability note (`patterns/menu-button/examples/menu-button-actions/`, `patterns/combobox/examples/combobox-select-only/`, `patterns/radio/`) plus the Table pattern added for `playwright/oracle/tier1-apg/sortable-table.spec.ts` (dev-docs/component-backlog.md row 68/69, Table/Data Table):

| Pattern | Page | Path (under `7e4034b/content/patterns/`) |
|---|---|---|
| Menu Button | Actions Menu Button Example Using `element.focus()` | `menu-button/examples/menu-button-actions.html` |
| Combobox | Select-Only Combobox Example | `combobox/examples/combobox-select-only.html` |
| Radio Group | Radio Group Example Using Roving `tabindex` | `radio/examples/radio.html` |
| Radio Group | Rating Radio Group Example (also roving-`tabindex`) | `radio/examples/radio-rating.html` |
| Table | Sortable Table Example | `table/examples/sortable-table.html` |
| Disclosure | Disclosure (Show/Hide) Navigation Menu | `disclosure/examples/disclosure-navigation.html` |

Added for the `navigation_menu` primitive's oracle
(`playwright/oracle/tier1-apg/disclosure-navigation.spec.ts`): the
Disclosure pattern's own site-navigation example, cited by that spec and by
`primitives/src/navigation_menu.rs`'s module doc for why Radix/shadcn's
`NavigationMenu` is a distinct ARIA contract from this crate's `Navbar`
(Menu-and-Menubar pattern) rather than a reskin of it. Vendored the same
way as the other three patterns above: `disclosure/examples/
disclosure-navigation.html` plus its own `examples/css/
disclosure-navigation.css`, `examples/js/disclosureMenu.js`, and
`content/images/pattern-disclosure.svg` (its pattern icon, referenced via
`../../../images/pattern-disclosure.svg`, three levels up from `examples/`
like the other patterns' own icons). The sibling `disclosure-navigation-
hybrid.html` example (top-level links alongside the disclosure buttons,
cited in the same spec/module doc for its own optional-arrow-key keyboard
table) is linked *from* the vendored page but was not itself vendored --
same "reachable via `src=`/`href=`/`url()`/script-driven `fetch`/`import`,
not merely cross-linked" scope rule this file's own "Dependencies vendored
alongside each page" section documents for the other three patterns'
sibling examples.

The radio pattern's examples directory has two techniques and, within the roving-`tabindex` technique, two variants: `radio.html` (two independent groups: pizza crust / delivery method) and `radio-rating.html` (a star-rating widget), both stated in-page to use "a roving tabindex for managing focus" and cross-linked to each other as "Similar examples". The third file in that directory, `radio-activedescendant.html`, uses the *other* technique (`aria-activedescendant`, single tabindex) and was **not** vendored — conformance-harness.md and plan.md ask for the roving-tabindex example(s), and this pattern's own text is what distinguishes "roving tabindex" (`radio.html`, `radio-rating.html`) from "activedescendant" (`radio-activedescendant.html`).

The table pattern's examples directory has two pages: `sortable-table.html` (the one vendored here -- `aria-sort` on sortable column headers, a `button` wrapping each sortable header's text) and `table.html` (a `div`/`span`-built ARIA table with no sort behaviour). `playwright/oracle/tier1-apg/sortable-table.spec.ts` calibrates the `aria-sort` rule, so only `sortable-table.html` -- and only its own dependency graph -- was vendored; `table.html` and its own `css/table.css` are a different example (no sorting) and were left out, the same "only load-bearing for the vendored page(s)" policy already applied above.

## Dependencies vendored alongside each page

Each example page loads shared chrome plus its own CSS/JS. Everything each page needs to load and run was traced (via `href=`, `src=`, `url()`, and JS `fetch`/dynamic `import`) and copied, preserving the relative path each reference already uses:

- `content/shared/css/core.css` (and its own `url("github.css")` dependency, `content/shared/css/github.css`)
- `content/shared/js/{app.js, examples.js, highlight.pack.js, skipto.js, specLinks.mjs}` — `app.js` dynamically `import()`s `specLinks.mjs` and `fetch()`es a usage-warning template at runtime (see "Known non-essential gaps" below); `examples.js` drives the in-page "view source" / syntax highlighting and an XHR-based "Open in CodePen" button
- `content/shared/templates/{example-usage-warning.html, experimental-example-usage-warning.html}` — fetched by `app.js` at `DOMContentLoaded` to insert the usage-warning banner
- `content/images/{pattern-menu-button.svg, pattern-combobox.svg, pattern-radio.svg, pattern-table.svg}` — each page's pattern icon, three levels up from its own `examples/` directory
- Each pattern's own `examples/css/*.css` and `examples/js/*.js` (`menu-button-actions.{css,js}`, `select-only.{css,js}`, `radio.{css,js}`, `radio-rating.{css,js}`, `sortable-table.{css,js}`)
- `LICENSE.md` (see Licence, below)

Sibling example pages in the same directories (e.g. `menu-button-links.html`, `combobox-datepicker.html`, `radio-activedescendant.html`) are linked *from* the vendored pages via plain `<a href>` cross-links but are not required for the vendored pages to load or function, and were not vendored — only assets reachable via `src=`, `<link>`/`@import`/`url()`, or script-driven `fetch`/`import` were treated as load-bearing.

## Licence

Per the source repository's own `LICENSE.md` (vendored at `7e4034b/LICENSE.md`):

> All documents in this Repository are licensed by contributors under the [W3C Software and Document License](https://www.w3.org/Consortium/Legal/copyright-software).

The pages are used here unmodified, for internal, non-distributed calibration/testing purposes, with provenance recorded on this page as the license requires attribution of the source. See the linked W3C Software and Document License for the full permissive terms (short form: redistribution and use in source or binary form, with or without modification, are permitted, provided the copyright notice and this permission notice are retained).

## Known non-essential gaps (verified not to affect the roles/behaviour under test)

- `menu-button/examples/css/menu-button-actions.css` references `../images/separator.svg` (a decorative background-image on `[role="separator"]`). This file **does not exist anywhere in the `w3c/aria-practices` repository at the pinned commit** — it is a pre-existing dead reference upstream, not something dropped in vendoring. It is purely cosmetic (a background image on a menu separator); its absence does not change the DOM, roles, or keyboard behaviour of the example. The table page has no equivalent gap -- `sortable-table.css` references no external image, and `pattern-table.svg` resolves cleanly (verified below).
- All five pages load `https://www.w3.org/StyleSheets/TR/2016/base.css` (base W3C document chrome styling) and `app.js`/`examples.js` reference `https://aria-at.w3.org/...` (an embedded AT-report iframe on the menu-button and radio pages) and W3C spec-link rewriting. These are external, non-essential to the pattern's own semantics/behaviour, and were deliberately left as external references rather than vendored, per the task's guidance to leave non-essential external references in place. They fail closed (no network in CI) without breaking the example markup itself.
- `app.js`'s `fetch()` of the usage-warning template and `examples.js`'s XHR-driven "Open in CodePen" button both use relative same-origin requests; over `file://` these can be blocked by the browser's file-access-from-file restrictions. Verified (see below) that this does not affect the presence of the pattern's own roles/elements in the DOM.

## Offline verification

Each vendored page was loaded via `file://` with the repo's own headless Chromium (`/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell --headless --disable-gpu --no-sandbox --dump-dom`) and checked for (a) the expected ARIA roles in the dumped DOM and (b) every relative `src=`/`href=` reference used to load a resource (not a same-page anchor or cross-link to a sibling doc) resolving to a vendored file. Results are recorded in the harness execution notes (see the PR/commit that added this directory); summary: all five pages render their pattern's roles correctly offline, and the only unresolved reference across all five pages is the pre-existing upstream `separator.svg` gap noted above.

`sortable-table.html` specifically (re-verified when the table pattern was added): the dumped DOM shows `aria-sort="ascending"` on the Last Name column header and a `button[data-column-index]` for each of the four sortable columns (First Name/Last Name/Company/Favorite Number, indices 0/1/2/4 -- index 3, Address, is `class="no-sort"` and correctly has no button), `pattern-table.svg` loads with no console error, and the only console errors are the same file://-CORS-blocked XHR/`fetch`/dynamic-`import` calls (examples.js's "view source" re-fetch of `sortable-table.{css,js}`, app.js's usage-warning template fetch, and the `specLinks.mjs` dynamic import) already documented above for the other four pages -- not a new gap, the same one.
