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

To reproduce: `git clone`, then `git checkout 7e4034b262bc0d25332e330d8a582aaf34113829`. No files were edited after copying — only files outside the three examples' dependency graph were left out.

## Pages vendored, and why

Per conformance-harness.md's reachability note (`patterns/menu-button/examples/menu-button-actions/`, `patterns/combobox/examples/combobox-select-only/`, `patterns/radio/`):

| Pattern | Page | Path (under `7e4034b/content/patterns/`) |
|---|---|---|
| Menu Button | Actions Menu Button Example Using `element.focus()` | `menu-button/examples/menu-button-actions.html` |
| Combobox | Select-Only Combobox Example | `combobox/examples/combobox-select-only.html` |
| Radio Group | Radio Group Example Using Roving `tabindex` | `radio/examples/radio.html` |
| Radio Group | Rating Radio Group Example (also roving-`tabindex`) | `radio/examples/radio-rating.html` |

The radio pattern's examples directory has two techniques and, within the roving-`tabindex` technique, two variants: `radio.html` (two independent groups: pizza crust / delivery method) and `radio-rating.html` (a star-rating widget), both stated in-page to use "a roving tabindex for managing focus" and cross-linked to each other as "Similar examples". The third file in that directory, `radio-activedescendant.html`, uses the *other* technique (`aria-activedescendant`, single tabindex) and was **not** vendored — conformance-harness.md and plan.md ask for the roving-tabindex example(s), and this pattern's own text is what distinguishes "roving tabindex" (`radio.html`, `radio-rating.html`) from "activedescendant" (`radio-activedescendant.html`).

## Dependencies vendored alongside each page

Each example page loads shared chrome plus its own CSS/JS. Everything each of the four pages needs to load and run was traced (via `href=`, `src=`, `url()`, and JS `fetch`/dynamic `import`) and copied, preserving the relative path each reference already uses:

- `content/shared/css/core.css` (and its own `url("github.css")` dependency, `content/shared/css/github.css`)
- `content/shared/js/{app.js, examples.js, highlight.pack.js, skipto.js, specLinks.mjs}` — `app.js` dynamically `import()`s `specLinks.mjs` and `fetch()`es a usage-warning template at runtime (see "Known non-essential gaps" below); `examples.js` drives the in-page "view source" / syntax highlighting and an XHR-based "Open in CodePen" button
- `content/shared/templates/{example-usage-warning.html, experimental-example-usage-warning.html}` — fetched by `app.js` at `DOMContentLoaded` to insert the usage-warning banner
- `content/images/{pattern-menu-button.svg, pattern-combobox.svg, pattern-radio.svg}` — each page's pattern icon, three levels up from its own `examples/` directory
- Each pattern's own `examples/css/*.css` and `examples/js/*.js` (`menu-button-actions.{css,js}`, `select-only.{css,js}`, `radio.{css,js}`, `radio-rating.{css,js}`)
- `LICENSE.md` (see Licence, below)

Sibling example pages in the same directories (e.g. `menu-button-links.html`, `combobox-datepicker.html`, `radio-activedescendant.html`) are linked *from* the vendored pages via plain `<a href>` cross-links but are not required for the vendored pages to load or function, and were not vendored — only assets reachable via `src=`, `<link>`/`@import`/`url()`, or script-driven `fetch`/`import` were treated as load-bearing.

## Licence

Per the source repository's own `LICENSE.md` (vendored at `7e4034b/LICENSE.md`):

> All documents in this Repository are licensed by contributors under the [W3C Software and Document License](https://www.w3.org/Consortium/Legal/copyright-software).

The pages are used here unmodified, for internal, non-distributed calibration/testing purposes, with provenance recorded on this page as the license requires attribution of the source. See the linked W3C Software and Document License for the full permissive terms (short form: redistribution and use in source or binary form, with or without modification, are permitted, provided the copyright notice and this permission notice are retained).

## Known non-essential gaps (verified not to affect the roles/behaviour under test)

- `menu-button/examples/css/menu-button-actions.css` references `../images/separator.svg` (a decorative background-image on `[role="separator"]`). This file **does not exist anywhere in the `w3c/aria-practices` repository at the pinned commit** — it is a pre-existing dead reference upstream, not something dropped in vendoring. It is purely cosmetic (a background image on a menu separator); its absence does not change the DOM, roles, or keyboard behaviour of the example.
- All four pages load `https://www.w3.org/StyleSheets/TR/2016/base.css` (base W3C document chrome styling) and `app.js`/`examples.js` reference `https://aria-at.w3.org/...` (an embedded AT-report iframe on the menu-button and radio pages) and W3C spec-link rewriting. These are external, non-essential to the pattern's own semantics/behaviour, and were deliberately left as external references rather than vendored, per the task's guidance to leave non-essential external references in place. They fail closed (no network in CI) without breaking the example markup itself.
- `app.js`'s `fetch()` of the usage-warning template and `examples.js`'s XHR-driven "Open in CodePen" button both use relative same-origin requests; over `file://` these can be blocked by the browser's file-access-from-file restrictions. Verified (see below) that this does not affect the presence of the pattern's own roles/elements in the DOM.

## Offline verification

Each vendored page was loaded via `file://` with the repo's own headless Chromium (`/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell --headless --disable-gpu --dump-dom`) and checked for (a) the expected ARIA roles in the dumped DOM and (b) every relative `src=`/`href=` reference used to load a resource (not a same-page anchor or cross-link to a sibling doc) resolving to a vendored file. Results are recorded in the harness execution notes (see the PR/commit that added this directory); summary: all four pages render their pattern's roles correctly offline, and the only unresolved reference across all four pages is the pre-existing upstream `separator.svg` gap noted above.
