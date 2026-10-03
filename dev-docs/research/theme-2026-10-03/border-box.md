# Global border-box (row 111, phase A addendum) -- expected size changes

Static analysis only (nothing was built or rendered). The verification lane should check every row of
section 3 against the computed-style snapshot diff, and treat any `width`/`height`-family change on a
`dx-` element that is NOT in section 3 as a miss in this list.

## 1. Construction chosen

`preview/assets/dx-components-theme.css` (new "Box model" section, before the interaction-rules layer):

```css
:where([class^="dx-"]), :where([class*=" dx-"]),
:where([class^="dx-"])::before, :where([class*=" dx-"])::before,
:where([class^="dx-"])::after,  :where([class*=" dx-"])::after { box-sizing: border-box; }
```

Scoped to the `dx-` class namespace, zero specificity, not a bare `*`.

- shadcn uses `*, ::before, ::after { box-sizing: border-box }` because Tailwind's preflight is a
  prerequisite of every shadcn install, so its components can assume it and a user's own elements are
  already border-box. A Dioxus app has no such prerequisite: this theme file is copied into the user's
  tree by `dx components add` and linked at their root, so a bare `*` would resize THEIR padded/sized
  elements (and every third-party widget) the moment they add one component. Scoped, it only touches
  our own markup.
- Every shipped class carries `dx-` (`scripts/check-dx-class-prefix.sh`), and `#[css_module]` hashing
  only appends a suffix (`dx-top-layer-stack-sibling-<hash>`), so the match survives hashing.
- `:where()` gives specificity 0, so any component's own `box-sizing` (and `all: unset`, which resets it)
  still wins. One simple selector per `:where()` and a flat list, because `main.css` records the asset
  pipeline silently dropping a selector list inside a pseudo-class argument
  (`docs/issues/asset-css-not-selector-list.md`).
- Same class of fix, different effect on the demos: with a bare `*`, the unclassed carousel `main` demo div
  (`height: 12rem` + 1px border, 194px) would shrink to 192px and no longer match its sibling button's
  `calc(12rem + 2px)`; scoped, both stay 194px.

Known gaps (by design, unchanged behaviour = still content-box):

- Elements inside a component that carry no `dx-` class: a primitive's inner wrappers, inline-styled demo
  divs, `ec-*` (email client), `li`/`summary` styled via a `.dx-...` ancestor in `main.css`.
- Elements with `all: unset`: `.dx-color-picker-slider-thumb`, `.dx-color-picker-area-thumb`,
  `.dx-slider-thumb`, `.dx-switch*` stay content-box (the zero-specificity rule loses to `all`).
  `.dx-slider-thumb` keeps its own `box-sizing: border-box` for that reason.
- Pseudo-elements other than `::before`/`::after`.

## 2. What stays in `primitives/` (not touched, report only)

Primitives ship WITHOUT this theme file, so these must stay:

- `primitives/src/carousel.rs:321` inline `box-sizing:border-box` on every slide (`971e955`) -- stays.
- `primitives/src/toast.rs:424`, `primitives/src/top_layer.rs` (forced border-box on the anchored content,
  lines ~1157, ~1715, ~2082) -- stay, same reason.

Finding: `preview/src/components/navbar/component.json` has no `"globalAssets"` entry for the theme (every
other component has one), so a user who installs only `navbar` gets `.dx-navbar` without the theme. Its own
`box-sizing: border-box` (`navbar/style.css:3`) was therefore KEPT.

## 3. Expected rendered-size changes (content-box -> border-box)

"Intended?" = was the old (content-box) size load-bearing. `no` rows are the bug class (the declared size
was meant as the visible box) and are left as the fix; `yes` rows were compensated.

| file:line (current) | element | old -> new | intended? / action |
|---|---|---|---|
| `assets/main.css:532` `.dx-docs-shell-body` | docs layout div, `width: min(--dx-content-width, 100vw - 2rem)`, 1px L/R border | outer 1362 -> 1360 px (inner 1360 -> 1358) | no. Now matches `.dx-navbar-inner` (border-box, same width expression) edge to edge |
| `assets/main.css:1284` `.dx-demos-card` | home card, `width: 100%` + 1px border | 100% + 2px -> 100% | no (overflow fix) |
| `assets/main.css:1308` `.dx-demos-card-thumb` | `height: 180px` + 1px border-bottom | 181 -> 180 px | no |
| `assets/main.css:2003` `.dx-charts-code-bar` | `height: 3rem` + 1px border-bottom | 49 -> 48 px | no |
| `src/components/alert/style.css:1` `.dx-alert` | div, `width: 100%` + 16px padding both sides | 100% + 32px -> 100% | no (overflow fix) |
| `calendar/style.css:132` `.dx-calendar-grid` | div, `width: 100%` + 8px inline padding, flex child of fit-content `.dx-calendar` | 100% + 16px -> 100%; probably 0 visible change because flex-shrink absorbed it | no. Verify: calendar width and day-cell size |
| `combobox/style.css:84` `.dx-combobox-list` | `width: 200px`, `max-height: 300px`, 4px padding | 208x(<=308) -> 200x(<=300) | no (shadcn `w-[200px]` is border-box) |
| `command/style.css:144` `.dx-command-list` | `max-height: 18rem` + 8px padding | 304 -> 288 px max | no |
| `context_menu/style.css:1` `.dx-context-menu-content` | `min-width: 220px` + 4px padding | min 228 -> 220 | no |
| `dropdown_menu/style.css:39` `.dx-dropdown-menu-content` | `min-width: 200px`, `max-width: calc(100vw - 2rem)`, 4px padding | min 208 -> 200; max no longer 8px past the clamp | no |
| `menubar/style.css:46` `.dx-menubar-content` | same shape as dropdown | min 208 -> 200 | no |
| `navbar/style.css:57` `.dx-navbar-content` | same shape as dropdown | min 208 -> 200 | no |
| `date_picker/style.css:28` `.dx-date-picker-group` | `min-width: 150px` + 8px padding | min 166 -> 150 | no |
| `hover_card/style.css:14` `.dx-hover-card-content` | `min-width: 200px`, 5px padding, 1px border | min 212 -> 200 | no |
| `tooltip/style.css:14` `.dx-tooltip-content` | `max-width: 250px`, 12px inline padding | max 274 -> 250 | no |
| `kbd/style.css:1` `.dx-kbd` (a `kbd`) | `min-width: 1.25rem` + 4px inline padding | single-key min 28 -> 20 px wide (height unchanged) | no (shadcn `min-w-5 px-1`, border-box) |
| `sidebar/style.css:775` `.dx-sidebar-menu-badge` | `min-width: 1.25rem` + 4px inline padding | min 28 -> 20 px | no (same as kbd) |
| `resizable/style.css:96` `.dx-resizable-handle-grip` | `width: 12px; height: 16px` + 1px border | 14x18 -> 12x16 px | no (Nova `h-4 w-3` is border-box) |
| `empty/variants/main/mod.rs:10` on `.dx-empty` (inline `width: 100%; max-width: 28rem` + 1px border; css: 24px inline padding) | Empty demo | max 498 -> 448 px; at narrow widths the 50px overflow disappears | no |
| `form/style.css:63` `.dx-form-result` (a `pre`) | `min-height: 1.5rem` + 12px block padding + 2px border | empty box 50 -> 26 px | **yes, compensated**: `min-height: calc(1.5rem + var(--dx-space-3) * 2 + 2px)` keeps 50 px |
| `top_layer/style.css:55` `.dx-top-layer-stack-area` (fixture) | `min-height: 160px` + `padding-top: 90px` | 250 -> 160 px | **yes, compensated**: `min-height: 250px` keeps the fixture exactly as tall |
| `top_layer/style.css:93` `.dx-top-layer-stack-sibling` (fixture) | `260x80` + 1px border | 262x82 -> 260x80 | no (marker box; it still clears the 90px padding) |
| `navigation_menu/style.css:232` / `:267` `.dx-navigation-menu-link`, `.dx-navigation-menu-featured` | `height: 100%` + 8px / 16px block padding | likely no change (percentage of an `auto`-height parent computes to `auto`); if the parent is definite, overflow by 16/32 px is removed | no. Verify |
| `carousel/variants/vertical/mod.rs:75` `Card` | `height: 100%` on `.dx-card` (padding block only) | none: the demo-only `box-sizing: border-box` from `09cd59e` is removed because the theme now supplies it; box is identical | reverted workaround |

### Checked and unchanged (listed so the snapshot diff can be read against them)

- Elements whose UA default is already `border-box`: `button` (`.dx-calendar-nav-*`, `.dx-dialog-close`,
  `.dx-sidebar-trigger`, `.dx-toggle`, `.dx-toggle-group-item`, `.dx-input-group-addon button`), `select`
  (`.dx-language-select`, `.dx-calendar-month-select`, `.dx-calendar-year-select`), checkbox/radio inputs.
- Padding/border on the OTHER axis from the declared size, so the size is unaffected: `.dx-home-section`
  (width, block padding), `.dx-charts-tab`, `.dx-sidebar-group-label`, `.dx-sidebar-menu-skeleton`,
  `.dx-table-head` (height, inline padding), `.dx-command-input` (width, block padding), `.dx-slider`
  (min-width, block padding), `.dx-sidebar-sheet`, `.dx-card` (block padding only; its `width: 100%` in
  `main.css:1960` was already border-box), chart tooltip total row (`flex-basis` in a row, block border).
- `all: unset` elements (section 1) and every unclassed element (section 1): no change.
- `.dx-carousel-item` / `CarouselVirtualContent` slides: already border-box (CSS class + inline in the
  primitive), now also from the theme.

## 4. Removed redundancies (78 declarations, 1 empty rule)

Removed `box-sizing: border-box` from rules whose subject carries a `dx-` class (so the theme rule gives the
identical computed value). Count by file: accordion 2, alert_dialog, aspect_ratio, badge, button, calendar,
carousel 5, chart, checkbox, collapsible, color_picker 4, combobox, command, dialog, drag_and_drop_list,
drawer, input, input_group, input_otp, item, menubar, native_select, navigation_menu 2, pagination, popover,
progress, radio_group, select 2, sheet, sidebar 4, slider 1, tabs 3, tag_group, textarea, toast,
top_layer 2, `assets/hero.css` 3, `assets/main.css` 23 (1 each unless noted). The `.dx-carousel-item { }`
rule became empty and was deleted.

Kept on purpose:

- `carousel/style.css:21` `.dx-carousel` (carries a long comment that documents it as load-bearing).
- `slider/style.css:47` `.dx-slider-thumb` (`all: unset` resets box-sizing; the zero-specificity theme rule
  would lose).
- `navbar/style.css:3` (component.json does not ship the theme, section 2).
- Non-`dx-` subjects: `assets/main.css:336`, `:429` (`.dxc` highlighter spans), `email_client.css:149`,
  `:436` (`.ec-row`, `.ec-read-body`).
- Inline `box-sizing: border-box` in demos on unclassed divs (`resizable` main and rtl demos) and in
  primitives (section 2).

Comment fixed: `assets/main.css` `.dx-charts-card-body .dx-card` (a third instance of the class: the card's
`width: 100%` overflowed the Tooltips-tab cell) now says the border-box comes from the theme.

## 5. Gates run (no build, no Playwright)

All 14 `scripts/check-*.sh` (incl. `check-css-vars-defined`), `cargo fmt --all -- --check`,
`cd preview && npx stylelint "src/**/*.css"`, `node scripts/generate-dx-utilities.js --check`: green.
Not run (build lane owns them): `cargo clippy`, `cargo test`, Playwright.
