<!-- Written 2026-10-03 by the token-design lane of backlog row 111, phase A. Point-in-time; measured against `main` + the theme edit in this change. -->

# Role-token map (row 111, phase A)

Companion files: [`sites.tsv`](./sites.tsv) (one row per ramp reference, machine-readable) and
[`tools/`](./tools) (the extractor, the role table and a value-preservation checker that produced it:
`python3 dev-docs/research/theme-2026-10-03/tools/gen.py [--write]`).

## 1. What changed in `preview/assets/dx-components-theme.css`

Only additions, plus three in-place rewrites that resolve to the identical value
(`--dx-ring-color`, `--dx-chart-1..5`, `--dx-chart-grid/-axis-text`). Checked by resolving every
pre-existing token (100 of them) in both modes before vs after: 0 changed. 44 tokens are new.
No `box-sizing` rule and no radius-scale rewrite were added (sections 7 and 8).

## 2. The problem, measured

The backlog said "~424 sites". Measured today: **842 ramp/status references in CSS** (633 in the
62 component stylesheets, 135 in `preview/assets/main.css`, 57 in `email_client.css`, the rest in
`hero.css`/`language-select.css`) **plus 126 in inline `style:` strings in `.rs` files** (`main.rs`,
variants, four components' `component.rs`). 968 rows in total, 99 files.

The inconsistency is deeper than "which step". Components already write the space toggle by hand,
so a "role" is really a *(light step, dark step) pair*:

| property family | distinct (light, dark) pairs | refs |
|---|---|---|
| text (`color`, `fill`/`stroke` on ink) | 13 | 313 |
| surface (`background*`, `fill`, `filter`) | 29 | 276 |
| line (`border*`, `outline`, `box-shadow` rings, `stroke`) | 16 | 219 |

So `--dx-border` as one step cannot be value-preserving for the line family: there are four common
border pairs (`#e5e5e5|#232323`, `#e5e5e5|#3e3e3e`, `#e5e5e5|#262626`, `#b0b0b0|#3e3e3e`), and the
dark half of each is a different ramp step.

## 3. The construction

1. **A role token is a (light, dark) pair of ramp steps**, written with the existing space toggle, e.g.
   `--dx-popover: var(--popover, var(--light, var(--primary-color)) var(--dark, var(--primary-color-5)))`.
   Resolved at `:root`, same as every ramp token, so the subtree behaviour is unchanged.
2. **Base roles carry shadcn's name** and sit on the dominant pair for that intent
   (`--dx-border` = border step `#e5e5e5|#232323`; `--dx-card` = what `.dx-card` already used; ...).
3. **A site is repointed only if its resolved (light, dark) values equal the role's.** Matching is by
   resolved hex, not by step name (light `--primary-color` and `--primary-color-2` are both `#fff`).
   For a ref wrapped in `var(--light, ...)` or `var(--dark, ...)` only that one mode has to match; a
   matched light+dark wrapper pair collapses to a single `var(--dx-role)`.
4. **A pair that has no base role gets a `-alt` variant** when 6 or more references share it (2 or
   more for status/destructive colours, where the intent is unambiguous). `-alt`, `-alt-2`
   (numbered by descending count) are *not* shadcn roles and carry no shadcn fallback: they exist so
   a site can leave the ramp without a visible change, and they are the phase-B review list
   (`grep -- '-alt'`). Phase B collapses each into its base role with a one-line edit in the theme,
   or promotes it. The set is 11 alts, deliberately uniform in naming.
5. **Everything else stays on the ramp** as `KEEP-phaseB` with the reason in the TSV: 78 refs,
   all genuine role conflicts (a step used for a purpose its role-mate does not share, e.g. the
   switch track, `--primary-color-7` used as a fill) or oddities (below).

Why not the alternatives: pointing every site at the single base token would be the cheapest and is
a visible change on ~200 refs (that is phase B's job, and it needs review per component); leaving every
non-dominant site on the ramp would keep ~200 refs on it and make phase B a 400-site edit instead of a
11-line one. The `-alt` layer turns phase B's collapse into a token-level edit, which is the point of
the exercise.

**Proof of value preservation.** `tools/gen.py` re-resolves, for every distinct declaration
(726 in CSS, plus the Rust inline strings), the old value and the proposed `new_value` under the space toggle in both light and
dark, against the new theme, and asserts the strings are equal. 0 mismatches (it exits non-zero
otherwise). This covers state selectors (hover, focus, open, `::before`) that a static snapshot cannot
reach: 207 of the 772 CSS repoints are in such selectors, and the snapshot spec captures one element state.
It is a token-layer proof, not a browser run; the computed-style snapshot remains the end-to-end check
(section 8, gaps).

## 4. Token set (light / dark resolve to)

Shadcn-named, fallback `var(--<shadcn>, <ramp>)`:

| token | light | dark | ramp step(s) (light / dark) | sites |
|---|---|---|---|---|
| `--dx-background` | `#fff` | `#000` | p0 | 23 |
| `--dx-foreground` | `#111` | `#d4d4d4` | s4 | 114 |
| `--dx-card` | `#fff` | `#141313` | p2 / p3 | 48 |
| `--dx-card-foreground` | `#111` | `#d4d4d4` | s4 | 0 |
| `--dx-popover` | `#fff` | `#262626` | p0 / p5 | 20 |
| `--dx-popover-foreground` | `#111` | `#d4d4d4` | s4 | 0 |
| `--dx-primary` | `#000` | `#fafafa` | s1 | 6 |
| `--dx-primary-foreground` | `#fff` | `#000` | p0 | 10 |
| `--dx-secondary` | `#f5f5f5` | `#262626` | p5 | 4 |
| `--dx-secondary-foreground` | `#000` | `#fafafa` | s1 | 2 |
| `--dx-muted` | `#f5f5f5` | `#262626` | p5 | 24 |
| `--dx-muted-foreground` | `#707070` | `#a1a1a1` | s5 | 135 |
| `--dx-accent` | `#f8f8f8` | `#3e3e3e` | p4 / p7 | 36 |
| `--dx-accent-foreground` | `#000` | `#fafafa` | s1 | 37 |
| `--dx-destructive` | `#dc2626` | `#a22e2e` | primary-error | 25 |
| `--dx-destructive-foreground` | `#fff` | `#dcdcdc` | contrast-error | 7 |
| `--dx-border` | `#e5e5e5` | `#232323` | p6 | 100 |
| `--dx-input` | `#e5e5e5` | `#3e3e3e` | p6 / p7 | 60 |
| `--dx-ring-color` (shadcn `--ring`) | `#2b7fff` | `#2b7fff` | focused-border | 29 |
| `--dx-chart-1..5` (`--chart-1..5`) | unchanged | unchanged | n/a | 0 (fallback only) |
| `--dx-sidebar-{background,foreground,primary,primary-foreground,accent,accent-foreground,border,ring}` | see theme | | p2, s4, s1, p0, p4, s4, p6, p7 | 6 local defs to delete |
| `--dx-radius` (`--radius`) | `0.5rem` | `0.5rem` | n/a | 0 |

Notes. `secondary`/`muted` and `secondary-foreground`/`accent-foreground`/`primary` share a value on purpose
(shadcn's own light values coincide): separate tokens so a host can theme them apart. shadcn's `--ring` is a colour;
`--dx-ring` already means the focus-ring `box-shadow`, so the role is the existing `--dx-ring-color` (rewritten
to `var(--ring, var(--focused-border-color))`). `--dx-chart-6..8` have no shadcn counterpart and stay as they were;
`--dx-chart-1..5` now read `--chart-1..5` first, which is the token-level hook the chart-parity tie-in wanted.

Ramp-only variants (no shadcn fallback; phase-B review list):

| token | light | dark | sites | notes |
|---|---|---|---|---|
| `--dx-background-alt` | `#fff` | `#0a0a0a` | 24 | dialog/sheet/drawer/calendar/menubar surface; shadcn would use `background` (dark `#0a0a0a` is shadcn's own dark background) |
| `--dx-card-alt` | `#fbfbfb` | `#0e0e0e` | 23 | recessed panels, code frames, docs cards |
| `--dx-muted-alt` | `#f8f8f8` | `#141313` | 12 | |
| `--dx-accent-alt` | `#f8f8f8` | `#1a1a1a` | 17 | row/cell/ghost hover |
| `--dx-accent-alt-2` | `#f8f8f8` | `#262626` | 10 | input/select/textarea hover and focus |
| `--dx-primary-alt` | `#0d0d0d` | `#e6e6e6` | 8 | primary-button hover tier |
| `--dx-foreground-alt` | `#2b2b2b` | `#dcdcdc` | 53 | headings, mostly docs chrome |
| `--dx-foreground-alt-2` | `#000` | `#fafafa` | 24 | non-state titles/labels on the `s1` step |
| `--dx-border-alt` | `#b0b0b0` | `#3e3e3e` | 11 | strong border |
| `--dx-border-alt-2` | `#e5e5e5` | `#262626` | 12 | card/chart hairline (dark equals the popover surface) |
| `--dx-destructive-alt` | `#ef4444` | `#9b1c1c` | 2 | destructive hover tier |

Status (shadcn ships none; plain aliases): `--dx-success`, `--dx-success-subtle`, `--dx-warning`,
`--dx-warning-subtle`, `--dx-info`, `--dx-info-subtle` (8 sites together). Their definitions still read the
ramp-era names (`--secondary-success-color`, ...), which stay as aliases.

## 5. The shadcn-fallback convention and its risk

`--dx-border: var(--border, <ramp>)`: the bare shadcn name wins when defined. Pasting shadcn's `globals.css`
theme into a host re-themes the components with no mapping layer, and our `--dx-` names cannot collide with a
host's own. **Risk (intended, per the row 111 decision):** a host app that defines a bare `--border`,
`--background`, `--primary`, `--accent`, `--muted`, `--ring`, `--radius`, ... for some other purpose now changes
our look, in both modes (a shadcn theme's `.dark { --border: ... }` is the host's job; our own dark switch is
`data-theme`/`prefers-color-scheme`, so a host using `.dark` classes needs both). Two second-order effects to
review in phase B: (a) with a host theme present the `-alt` family stays on the ramp, so a family can look
mixed (cards recolour, `card-alt` frames do not); (b) `--dx-sidebar-*` is currently masked inside
`.dx-sidebar-wrapper` by the component's own six local declarations until the lane deletes them. In-repo
check: nothing in the repo defines any of the shadcn bare names today (grep: only a bench fixture HTML),
so the repo itself is unaffected.

## 6. How installers get the tokens

`dx components add <name>` copies the component and its `globalAssets` entry,
`../../../assets/dx-components-theme.css` (declared in each `component.json`). The theme file ships
whole, so the role tokens arrive with it and the stylesheets reference `--dx-*` names defined in that same
file. The ramp names are untouched, so an already-installed theme keeps every existing component working;
repointed components need the new theme file (same precedent as the row 31b `--dx-space-*` tokens).
Two hazards to decide on (open questions in the report): a user with an older copy of the theme who adds a
newly-repointed component gets undefined `--dx-*` values; a lint "every `--dx-*` used in a `style.css` is
defined in the theme" would catch it for the repo's own copies. `test-harness/assets/dx-components-theme.css`
is an unrelated, older copy (no `--dx-` tokens at all) and is not touched. Nothing in
`scripts/check-installed-paths.sh` is affected (it only checks Rust paths).

## 7. Counts

| | refs |
|---|---|
| total ramp/status references (CSS 842 + Rust inline 126) | 968 |
| repointed now (value-preserving) | 884 (91%), of which 685 on shadcn-named base roles and 199 on `-alt`/`-subtle` |
| `KEEP-phaseB` (stays on the ramp) | 78 |
| local definitions to delete (sidebar) | 6 |
| files fully cleared of ramp references | 69 of 99 |

`KEEP-phaseB` breakdown (78): 24 ink/ring uses of `--secondary-color-2`/`-3`/`-6` that no role carries (slider and
colour-picker thumbs, drag handles, link/hover ink, hero CTA); 24 mixed-step light/dark pairs with no role
(`#f5f5f5|#232323`, `#e5e5e5|#3e3e3e` on separator/resizable, `#fff|#141313` strokes, `#000|#d4d4d4` in
context-menu hover, which disagrees with the same hover in dropdown-menu, ...); 10 `--primary-color-7` used as a
fill (drawer handle, toggle on, avatar empty, outline-button dark mix); 8 refs on undefined tokens (below); 2 switch
track (shadcn `--input`, ours is the border step, dark `#232323` vs `#3e3e3e`); 10 singletons (`scrollbar-color`,
`--secondary-color-5` as a fill/ring, `--secondary-color` as a fill, `p0`/`p2`/`p5`/`s1` as a border colour).

Pre-existing defects found (left untouched, listed in the TSV):
- `--primary-color-9`, `-11`, `-12` are **undefined** (virtual_list demos' text colours, 6 refs: the declaration
  is invalid, so the text inherits).
- `--primary-color-8` is **undefined**: `navigation_menu`'s featured gradient is invalid in dark mode
  (`#3e3e3e, <missing>`).
- `scripts/generate-dx-utilities.js --check` reports `STALE` on HEAD already (confirmed against the
  unmodified theme); unrelated to this change but it is a red check.

## 8. Not done in phase A, and gaps in the proof

- **Global `box-sizing: border-box` (row 111 addendum, 2026-09-30): intentionally NOT added.** It is not
  value-preserving (it shrinks the rendered size of every element that sets `width`/`height` together with
  padding/border), so it must be a separate, reviewed step *after* the value-preserving repoint lands, with
  its own before/after snapshot diff reviewed by hand (the diff will not be empty, by design). Do it before
  phase B's density work so B is measured on the final box model.
- **Radius derivation is staged, not applied.** `--dx-radius` exists (fallback `0.5rem`); the existing scale
  is exactly shadcn's additive derivation at `0.5rem` (sm = base - 0.25rem, md = base - 0.125rem, lg = base,
  xl = base + 0.25rem, 2xl = base + 0.5rem; 3xl differs: ours is 1.5rem, shadcn's would be 1.25rem).
  Rewiring `--dx-radius-*` onto `calc(var(--dx-radius) +/- ...)` is computed-style-identical but
  `scripts/check-css-literals.sh` skips any token containing `var(`, so it would silently stop catching
  literal radii; teach it to read calc tokens first (phase B's "radius token swap" step).
- **Snapshot gaps** (`playwright/computed-style-snapshot.spec.ts`, not edited here): `PROPERTIES` lacks
  `outline-color`, `fill`, `stroke`, `filter`, `scrollbar-color` (about 45 sites use them); it captures one
  colour scheme (needs a second run with dark emulated, since each role is a light/dark pair) and one element
  state (hover/focus/open rules are exercised only by the token-layer proof above). It also keys on
  `[class*="dx-"]` on component pages, so `hero.css`, `language-select.css`, the dashboard and the docs chrome in
  `main.css` are only partly covered.
- Text mentions of ramp names outside stylesheets (`docs.md`, `playwright/*.spec.ts` such as
  `top-layer-ink.spec.ts`, comments in `primitives/src/top_layer.rs`) are not repointed; the specs read the
  token from the page, so they keep passing as long as the ramp names live.

## 9. Four worker lanes (disjoint file sets, 242 sites each)

Each lane edits every file of its units (the component's `style.css` plus its `component.rs` / `variants/**/mod.rs`
inline `style:` strings) by applying the TSV rows of those files: replace the declaration's `current_value` with
`new_value` (the TSV already collapsed light/dark wrappers), leave `KEEP-phaseB` rows alone, and for `sidebar`
delete the six `--dx-sidebar-*` declarations on `.dx-sidebar-wrapper`. Done criterion per lane:
`python3 dev-docs/research/theme-2026-10-03/tools/gen.py` reports 0 `REPOINT` rows for its files.

- **Lane A (242):** `preview/assets/main.css`, `preview/assets/language-select.css`; components `alert`,
  `button_group`, `combobox`, `command`, `date_picker`, `form`, `hover_card`, `input_otp`, `label`,
  `scroll_area`, `sheet`, `switch`, `toolbar`.
- **Lane B (242):** `preview/src/main.rs`; components `aspect_ratio`, `avatar`, `carousel`, `checkbox`,
  `context_menu`, `drag_and_drop_list`, `input`, `line_chart`, `menubar`, `separator`, `slider`, `table`, `tabs`,
  `tag_group`, `textarea`, `toggle`, `toggle_group`.
- **Lane C (242):** `preview/assets/hero.css`, `preview/src/dashboard/views/email_client/email_client.css`;
  components `breadcrumb`, `button`, `card`, `chart_tooltip`, `color_picker`, `data_table`, `dialog`,
  `dropdown_menu`, `empty`, `native_select`, `navigation_menu`, `progress`, `radio_group`, `select`,
  `skeleton`, `toast`, `tooltip`.
- **Lane D (242):** components `accordion`, `alert_dialog`, `badge`, `calendar`, `chart`, `collapsible`,
  `drawer`, `field`, `input_group`, `item`, `kbd`, `navbar`, `pagination`, `popover`, `resizable`, `sidebar`,
  `spinner`, `top_layer`, `virtual_list`.

(`gen.py` prints the exact unit list and re-derives it deterministically; the `lane` column of the TSV is the
authority. A unit is a whole component directory, so no two lanes touch the same file.)

## 10. Phase B reconciliation (colour lane, 2026-10-03)

Scope: every component directory except `card`, `button`, `input`, `badge`, `tabs`, `dialog`, `select` (another
lane), plus `preview/assets/{main,hero,language-select}.css`, `preview/src/main.rs` and the dashboard CSS. Token
values in the theme file were not touched. Hex pairs are `light / dark`. Where a result is a `color-mix` over a
page colour the "after" is the approximate flattened value. No browser run (the build belongs to another lane); the
deltas below are computed from the theme's resolved values.

Result: 170 `-alt` refs in scope, 164 reconciled onto base roles and 6 kept (below); 72 `KEEP-phaseB` refs in scope,
70 reconciled and 2 kept (below). `scripts/check-css-vars-defined.sh` passes on the whole tree. Remaining
`-alt` use in scope: `--dx-border-alt` x6. Remaining ramp refs in scope: `--primary-color-7` x2.

### 10.1 The five undefined custom properties

| site | before | after | why |
|---|---|---|---|
| `navigation_menu` featured gradient, dark end `--primary-color-8` (ramp stops at `-7`) | dark: the whole `background` was invalid, so the card was transparent. light: `#f8f8f8` | gradient `accent` to `muted`: light `#f8f8f8 > #f5f5f5`, dark `#3e3e3e > #262626` | the light end was `p3`, i.e. a surface one tier under the hover colour; the intent is a card that fades from `accent` to the quieter surface. `muted` is that role. Hex delta in light is 3 units |
| `toast` `.dx-toast-container:hover .dx-toast { margin-top: var(--toast-padding) }` | invalid at computed time, so it resolved to the initial `0` | literal `0` (identical computed value) | `--toast-padding` has never been declared: `git log --all -S'--toast-padding:'` finds no commit that defines it, only the `var()` use (first seen in ddcc8b8, a path move). Collapsed stack uses `margin-top: -4rem` to overlap; on hover/focus the stack expands and the gap comes from the `ol` `gap: var(--dx-space-3)` (12px, sonner's expanded `--gap` is 14px). Adding padding on top would double the gap, so `0` is the intended value and is now explicit |
| `virtual_list` demos, subtitle `--primary-color-9` (2 sites) | invalid, so text inherited `#111 / #d4d4d4` | `--dx-muted-foreground` `#707070 / #a1a1a1` | Radix-scale naming (`-9` solid, `-11` low-contrast text, `-12` high-contrast text) copied from a different system; subtitle is secondary text |
| `virtual_list` demos, body `--primary-color-11` (2 sites) | inherited `#111 / #d4d4d4` | `--dx-muted-foreground` | `-11` is "low-contrast text" on that scale = muted text; matches shadcn card descriptions |
| `virtual_list` demos, title `--primary-color-12` (2 sites) | inherited `#111 / #d4d4d4` | `--dx-foreground` `#111 / #d4d4d4` (no visual change) | `-12` is "high-contrast text" = foreground |

### 10.2 `-alt` tokens (default mapping and where it was overridden)

| token | refs | before | after | rationale |
|---|---|---|---|---|
| `foreground-alt` | 49 | `#2b2b2b / #dcdcdc` | `foreground` `#111 / #d4d4d4` | headings and docs chrome are the foreground role; delta 24 units light, 8 dark |
| `foreground-alt-2` | 23 | `#000 / #fafafa` | `foreground` `#111 / #d4d4d4` | titles/labels; light 17 units, dark 22 units softer |
| `card-alt` | 15 | `#fbfbfb / #0e0e0e` | `card` `#fff / #141313` | recessed docs frames/forms: same role, lighter by 4 (light) and 6 (dark) units |
| `card-alt` (slider thumb, chart inside-label fill, sidebar inset + outline menu button + demo headers; 6) | | same | `background` `#fff / #000` | shadcn: thumb/inset/outline buttons are `bg-background`; the chart fill is text on a coloured mark, wants the extremes |
| `background-alt` | 11 | `#fff / #0a0a0a` | `background` `#fff / #000` | dialog/sheet/drawer/menubar/calendar/chart tooltip are `bg-background` in shadcn; dark `#0a0a0a` to `#000` |
| `background-alt` (docs inline `code`, `th`, 4 sites) | | `#fff / #0a0a0a` | `muted` `#f5f5f5 / #262626` | shadcn inline code is `bg-muted`; this makes chips visible in light for the first time (before: white on white). Biggest light-mode delta here |
| `background-alt` (manual-install `summary:hover`) | | same | `accent` `#f8f8f8 / #3e3e3e` | hover surface |
| `background-alt` (demos card thumb gradient; virtual_list card; sidebar demo settings panel) | | `#fff / #0a0a0a` | `card` `#fff / #141313` | panel surface; dark `#0a0a0a` to `#141313` |
| `muted-alt` | 5 | `#f8f8f8 / #141313` | `muted` `#f5f5f5 / #262626` | checkbox fill, calendar nav, avatar error fallback, code-bar header, aspect-ratio demo, email rows (hover, selected): the muted role. Dark `#141313` to `#262626` is the visible step |
| `muted-alt` (email read pane / main / list pane / list scroll / compose textarea; 5) | | `#f8f8f8 / #141313` | `card` `#fff / #141313` | big panes: dark is identical, light +7 units; `muted` would have lightened a whole pane in dark |
| `accent-alt` | 13 | `#f8f8f8 / #1a1a1a` | `accent` `#f8f8f8 / #3e3e3e` | hover/active surfaces. Dark `#1a1a1a` to `#3e3e3e` is the largest dark delta of this pass and is consistent with `dropdown_menu`/`context_menu`/`select` items, which already used `accent`. See the recommendation in 10.5 |
| `accent-alt` (table row hover, table footer) | | `#f8f8f8 / #1a1a1a` | `color-mix(muted 50%)` ~`#fafafa / #131313` | shadcn's table is `hover:bg-muted/50` and `bg-muted/50`; stays within 2 to 7 units of before and keeps hover distinct from `data-state=selected` (`muted`) |
| `accent-alt-2` | 3 | `#f8f8f8 / #262626` | `muted` `#f5f5f5 / #262626` | combobox/textarea hover/focus fill: dark identical, light 3 units. Picked `muted` over `accent` so a full-width text field does not jump to `#3e3e3e`; flag if the input lane chose otherwise |
| `border-alt-2` | 5 | `#e5e5e5 / #262626` | `border` `#e5e5e5 / #232323` | chart hairlines; dark 3 units |
| `primary-alt` (checkbox checked, slider range, calendar selected day) | 3 | `#0d0d0d / #e6e6e6` | `primary` `#000 / #fafafa` | selected/filled state is the primary role, not its hover tier; 13 / 20 units |
| `primary-alt` (hero CTA hover, email compose hover) | 2 | `#0d0d0d / #e6e6e6` | `color-mix(primary 90%)` ~`#1a1a1a / #e1e1e1` | shadcn's `hover:bg-primary/90`; the hover tier now derives from the host's `--primary` |
| `destructive-alt` (alert_dialog action, popover action hover) | 2 | `#ef4444 / #9b1c1c` | `color-mix(destructive 90%)` ~`#df3c3c / #942a2a` over the surface | `hover:bg-destructive/90`; note the old hover tier was LIGHTER than the resting `#dc2626` in light, now it is slightly lighter by transparency, which reads as the same cue |
| `border-alt` (date_picker chevron stroke) | 1 | `#b0b0b0 / #3e3e3e` | `muted-foreground` `#707070 / #a1a1a1` | it is an icon, not a line; was low-contrast in both modes |
| `border-alt` (slider thumb hover halo) | 1 | `color-mix(#b0b0b0 / #3e3e3e 50%)` | `color-mix(ring 50%)` blue `#2b7fff` | shadcn `ring-ring/50`; the focus ring already uses the same colour |

Kept (`--dx-border-alt`, 6 refs): `.dx-demos-card:hover`, `.dx-calendar-nav-*:hover`, `.dx-drag-and-drop-list-item:hover` and
`[data-is-grabbing]`, `.dx-textarea[data-style=outline]:hover`, and the `checkbox` unchecked ring. These are a
distinct role shadcn has no token for: a line one step stronger than `border`, used as hover emphasis and as a
control boundary. Collapsing to `border` or `input` would make the hover invisible in light (`#e5e5e5` resting
vs `#e5e5e5` hover) and drop the checkbox ring to 1.26:1 on white. Recommendation: promote it (see 10.5).

### 10.3 `KEEP-phaseB` ramp refs

| site(s) | before | after | rationale |
|---|---|---|---|
| hero CTA bg+border (2 refs on the resting rule, 1 hover border) | `s3 #2b2b2b / #dcdcdc`, hover border `s2 #0d0d0d / #e6e6e6` | `primary` `#000 / #fafafa` | the CTA is the primary button (text is already `primary-foreground`) |
| `.dx-charts-hero-button[primary]` bg | `s0 #000 / #fff` | `primary` `#000 / #fafafa` | same; dark 5 units |
| scrollbar hover thumb (main.css) | `s2 #0d0d0d / #e6e6e6` | `foreground` `#111 / #d4d4d4` | ink colour; 4 / 22 units |
| manual-install chevron borders (2) | `s5 #707070 / #a1a1a1` | `muted-foreground` (identical) | value-preserving, it was only unmatched because of the property |
| carousel indicators + picker dot bg (2) | `s5` | `muted-foreground` (identical) | same |
| `chart-active-dot` stroke | `p2/p3 #fff / #141313` | `card` (identical) | the comment already says "same pair as card" |
| `radio_group` checked gap ring | `#fff / #141313` | `card` (identical) | same pair as card |
| `avatar[data-state=empty]` | `p7 #b0b0b0 / #3e3e3e` | `muted` `#f5f5f5 / #262626` | shadcn `AvatarFallback` is `bg-muted`; matches the neighbouring `loading` state |
| calendar disabled nav border / text | `p5 #f5f5f5 / #262626`, text `#2b2b2b / #dcdcdc` | `border` `#e5e5e5 / #232323`, `muted-foreground` | disabled text should be muted |
| calendar other-month selected day | `s6 #d0d0d0 / #5d5d5d` | `accent` `#f8f8f8 / #3e3e3e` | the quiet selected surface; text is already `muted-foreground` |
| color_picker swatch border | `mix(p0 10%)` = invisible | `border` `#e5e5e5 / #232323` | the old value was white-on-white in light and black-on-black in dark; a swatch outline is a border. New visible edge |
| color_picker slider/area thumb ring (2) | `s2 #0d0d0d / #e6e6e6` | `primary` `#000 / #fafafa` | shadcn `border-primary` thumbs |
| context_menu hover/open text (2) | `s1 / s4`: `#000 / #d4d4d4` | `accent-foreground` `#000 / #fafafa` | agrees with `dropdown_menu`, which the map flagged as disagreeing |
| date_picker segment focus fill | `s3 #2b2b2b / #dcdcdc` | `primary` `#000 / #fafafa` | text on it is `primary-foreground` |
| drag_and_drop_list: handle and remove-button text | `s6 #d0d0d0 / #5d5d5d` | `muted-foreground` `#707070 / #a1a1a1` | was below 2:1; now a legible muted ink |
| drag_and_drop_list: hover ink / title / due text (5) | `s2` | `foreground` | ink |
| drag_and_drop_list: remove-button hover fill | `p5/p6 #f5f5f5 / #232323` | `muted` `#f5f5f5 / #262626` | |
| drag_and_drop_list: item resting fill | `#fff / #1a1a1a` (a literal plus `accent-alt`) | `card` `#fff / #141313` | the literal `#fff` is gone |
| drag_and_drop_list demo: separator dot | `p7 #b0b0b0 / #3e3e3e` | `muted-foreground` | a dot inside metadata text; same for the dot in `main.rs` |
| drawer handle | `p7 #b0b0b0 / #3e3e3e` | `input` `#e5e5e5 / #3e3e3e` | dark identical; light lighter, a handle is a quiet control line |
| drawer close button (dark mix) | `mix(p7 30% / 50%)` | `mix(input 30% / 50%)` | identical value (`input` dark is `p7`), now themeable; this is shadcn's `dark:bg-input/30` |
| resizable handle + grip, separator | `p6/p7 #e5e5e5 / #3e3e3e` | `border` `#e5e5e5 / #232323` | shadcn separator and handle are `bg-border`. Dark hairlines are dimmer (`#3e3e3e` to `#232323`), same as every other border in dark. The resizable rtl demo border `s3` is now `foreground` |
| slider thumb ring (2) | `s2 #0d0d0d / #e6e6e6` | `primary` | `border-primary` |
| switch track | `p6 #e5e5e5 / #232323` | `input` `#e5e5e5 / #3e3e3e` | shadcn track is `bg-input`; dark track is lighter |
| switch checked track | `s2` | `primary` | |
| switch thumb | `p0 / s2: #fff / #e6e6e6` | `background / foreground`: `#fff / #d4d4d4` | |
| tag_group remove button | text `s6`, hover `p5/p6` + `s2` | `muted-foreground`; `muted` + `foreground` | as drag_and_drop_list |
| toggle_group edge borders (4) | `s6 #d0d0d0 / #5d5d5d` | `input` `#e5e5e5 / #3e3e3e` | edge of a pressed item |
| toolbar demo toggle fill | `p5/p6 #f5f5f5 / #232323` | `muted` `#f5f5f5 / #262626` | |
| email_client tag trigger hover border | `s1 #000 / #fafafa` | `primary` (identical) | |
| `main.rs` separator dot | `p7` | `muted-foreground` | |

Kept on the ramp (2 refs): `toggle` and `toggle_group` `[data-state="on"] { background-color: var(--primary-color-7) }`
(`#b0b0b0 / #3e3e3e`). The pressed fill has to stay distinguishable from the hover fill (`accent`, `#f8f8f8` light);
`accent` in light is three units from `muted`/hover, so the pressed state would disappear. `toggle_group/style.css`
carries a comment pinning the old computed values for exactly this reason. Needs a role (a stronger `accent` in light,
or `--dx-accent-strong`), then a one-line edit in each file.

### 10.4 Biggest visible deltas (reviewers: check these first)

1. Dark hover surfaces on 13 `accent-alt` sites (calendar cells/nav, dropdown/popover triggers, pagination, toggle, toggle_group, alert_dialog cancel, chart toggle, native_select, input_group, ghost action): `#1a1a1a` to `#3e3e3e`.
2. Separators, resizable handles and grips in dark: `#3e3e3e` to `#232323` (dimmer).
3. Docs inline `code` chips and table headers in light: white-on-white to `#f5f5f5`.
4. Switch track in dark `#232323` to `#3e3e3e`; thumb `#e6e6e6` to `#d4d4d4`.
5. Drag handles / remove buttons / carousel text in dark and light: `#d0d0d0 / #5d5d5d` to `#707070 / #a1a1a1` (now legible).
6. Hero CTA and primary-filled controls: `#2b2b2b / #dcdcdc` to `#000 / #fafafa`.

### 10.5 Recommendations for the theme owner (nothing changed here)

- `--dx-accent` dark `#3e3e3e` is `p7`; shadcn's dark accent, secondary and muted all coincide at `#262626` (`p5`). Setting accent dark to `p5` would shrink delta 1 above from 36 to 12 units on 13 sites without changing any light value. `muted` and `accent` would then coincide in dark as they already do in shadcn.
- Promote `--dx-border-alt` to a documented non-shadcn role (suggest `--dx-border-strong`) and keep it, or fold it into `--dx-input` by giving `input` light `#b0b0b0`. Six sites depend on it staying distinct from `border` in light.
- Light `--dx-accent` (`#f8f8f8`) is too close to `--dx-muted` (`#f5f5f5`) and to `--dx-background` to carry a pressed state; toggle and toggle_group `on` need a stronger role in light.
- All `-alt` tokens except `border-alt` now have 0 uses in scope. They still have uses in `card`, `button`, `input`, `badge`, `tabs`, `dialog`, `select` (other lane); delete them from the theme only after that lane lands.

## 11. Phase B cleanup (2026-10-03)

- **Renamed** `--dx-border-alt` -> `--dx-border-strong` at its 6 remaining sites (`main.css` `.dx-demos-card:hover`,
  calendar nav hover, drag_and_drop_list x2, textarea hover, checkbox ring). Same value, so no visual change.
- **Deleted** from `dx-components-theme.css`, each verified unreferenced (CSS, inline Rust, docs): `--dx-background-alt`,
  `-card-alt`, `-muted-alt`, `-accent-alt`, `-accent-alt-2`, `-primary-alt`, `-foreground-alt`, `-foreground-alt-2`,
  `-border-alt`, `-border-alt-2`, `-destructive-alt`. The only non-shadcn colour role left is `--dx-border-strong`
  (plus the status roles). Sections 4 and 10 above describe the pre-cleanup state. `tools/gen.py` still runs and
  reports 0 problems (it only uses the `-alt` names as labels in its role table; nothing resolves them any more).
- **Gate**: `scripts/check-css-literals.sh` skipped every token whose value contains `var(`, so once the radius scale
  became `calc(var(--dx-radius) * k)` a literal `border-radius: 0.5rem` passed. It now resolves each
  `--dx-radius-*` step at the default base (`--dx-radius: var(--radius, X)`) and fails loudly if that shape changes.
  Proven on a scratch copy: `0.5rem` -> `--dx-radius-md`, `0.375rem` -> `--dx-radius-sm` are reported. It immediately
  found `pagination` `0.625rem`, now `var(--dx-radius-lg)` (same 10px, no visual change).
- `InputGroup` wrapper now matches `Input` focus (muted fill, ring-coloured edge + `--dx-ring-focus`, input/30% dark
  rest fill); `dx-utilities.css` regenerated (adds `.dx-radius-4xl`; the rest of the staleness was a comment path).

