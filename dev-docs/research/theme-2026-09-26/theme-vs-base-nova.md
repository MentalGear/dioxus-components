<!-- Research notes written 2026-09-26 by a read-only research lane. Point-in-time; shadcn-ui/ui@98a1fe6. -->

# Theme vs shadcn base+style matrix — research notes

## 1. What shadcn ships now (apps/v4, @98a1fe6)

- **Bases** (`registry/bases/`): `radix`, `aria`, `base` — three implementations of the same
  component set on different primitive libraries (Radix UI, React Aria, shadcn's own headless
  "base" primitives). Docs URLs are namespaced by base: `content/docs/components/{base,radix,aria}/carousel.mdx`.
- **Styles** (`registry/styles/style-*.css`): 8 styles — `vega`, `nova`, `maia`, `lyra`, `mira`,
  `rhea`, `sera`, `luma` (`registry/styles.tsx` STYLES array, `registry/styles/*.css` filenames).
  Each is a `.style-{name} { .cn-xxx { @apply ...tailwind utilities... } }` block — Tailwind v4
  utility bundles keyed by `cn-*` slot classes, not a CSS-variable palette.
- **Foundational tokens are style-agnostic**: `app/globals.css` `:root`/`.dark` define
  `--radius: 0.625rem` (10px) with `--radius-sm/md/lg/xl/...` as `calc()` off that one value, plus
  OKLCH colors (`--background: oklch(1 0 0)`, `--primary: oklch(0% 0 0)`, `--ring: oklch(0.708 0 0)`,
  etc.). None of `style-nova.css`/`style-vega.css` override these — confirmed via `grep ":root\|--radius:"`
  on both files (only `.style-nova {` / `.style-vega {` at top). So **color/radius/ring are not what
  a "style" changes** — every style shares one palette and one radius scale; style changes are
  per-component density/rounding/shadow *recipes* layered on top.
- **Docs default**: `registry/_legacy-styles.ts` (`getActiveStyle()`, hardcoded to
  `legacyStyles[0] = "new-york-v4"`) is a *compat shim* used only by the old full-page "blocks"/"charts"
  registry (`app/(app)/blocks/*`, `app/(app)/charts/*`) that hasn't been ported to the base×style
  matrix yet. `lib/registry-health/monitor.ts` separately hardcodes `DEFAULT_STYLE = "radix-vega"` for
  its automated dependency-health checks — also not what visitors see on a component doc page.
  **What visitors actually see** is set per-mdx-file via `<ComponentPreview styleName="...">`. Counting
  across `content/docs/components/{base,radix,aria}/*.mdx`:
  - base: 471 pages `base-nova`, 52 `base-rhea`, 2 `radix-nova` (typos/overrides)
  - radix: 467 `radix-nova`, 44 `radix-rhea`, 12 stray `base-nova`
  - aria: 468 `aria-nova`, 44 `aria-rhea`
  → **Nova is the de-facto default style across ~95% of component docs pages, for every base.**
  `rhea` ("like Luma but compact") appears only on a handful of newer components (e.g. `attachment`).
  Vega ("clean, neutral, and familiar" — the old default) is not used as a demo style on any
  individual component page in this snapshot; it survives only as the health-check/legacy default.
  `/docs/components/base/carousel.mdx` explicitly hardcodes `styleName="base-nova"`.

## 2. Style axis is per-component, not token-level

Diffed `style-nova.css` vs `style-vega.css` (Button MARK block, line-for-line):
- Nova: `rounded-lg` default button radius; sizes `h-6/7/8/9`; icon buttons `size-6/7/8/9`.
- Vega: `rounded-md`; sizes `h-6/8/9/10`; icon buttons `size-6/8/9/10`.
- Vega's `outline`/`secondary`/checkbox/card variants add `shadow-xs`; Nova's do not (Nova is
  deliberately flatter/borderless — ring-based, not shadow-based).
- Card: Nova `--card-spacing: --spacing(4)` (16px) header/content/footer padding, no shadow.
  Vega `--card-spacing: --spacing(6)` (24px), `shadow-xs` present.
- Combobox item: Nova `rounded-md py-1 pl-1.5`; Vega `rounded-sm py-1.5 pl-2` (denser vs looser).
- Carousel prev/next buttons: identical in both (`rounded-full`) — carousel nav chrome doesn't
  vary by style; the carousel's `box-shadow: none` already matches both.

This confirms the styles.tsx descriptions: Nova = "reduced padding and margins" (a *density* preset),
Vega = "clean, neutral, and familiar" (the original, more spacious new-york-adjacent look). The
difference is real per-component CSS (heights, padding, radius-per-slot, presence/absence of
`shadow-xs`), not a swappable set of root variables.

## 3. Our theme (`preview/assets/dx-components-theme.css`, row 31a) vs both

- Radius: our `--dx-radius-lg: 0.5rem` (8px, "dominant literal, 52/137 uses") vs shadcn's shared
  `--radius-lg: var(--radius)` = 10px (Nova and Vega both use `rounded-lg`/`var(--radius-lg)` for
  the same slots) — we're already ~2px tighter than *either* shadcn style at the token level.
- Color system: ours is hex/sRGB, hand-built primary/secondary 7-step grayscale + semantic
  success/warning/error triples (`preview/assets/dx-components-theme.css:46-88`); shadcn (both
  styles) is OKLCH, pure-black/white primary, single accent-free neutral scale
  (`globals.css:99-162`). This is a base-theme difference independent of nova vs vega.
- Shadow: we have a real 5-step elevation scale used broadly (`--dx-shadow-sm..2xl`,
  `dx-components-theme.css:157-162`; e.g. `card/style.css:8` `0 2px 10px rgb(0 0 0/10%)`,
  `tabs/style.css:90`). This matches **Vega's** convention (shadow-xs on cards/surfaces), not
  Nova's (flat, no card shadow). So on the shadow axis specifically, our theme is closer to the
  OLD measured target (Vega/new-york-era) than to Nova.
- Ring: ours `--dx-ring-width: 2px`, full-opacity color, only box-shadow (dx-components-theme.css:159-161).
  shadcn (both styles): `ring-3` (3px) at `ring-ring/50` (50% opacity) *plus* a `border-ring` color
  change on the element itself — a two-part focus treatment neither of our tokens reproduce.
- Carousel nav buttons: our `carousel/style.css:79-93` (`height: var(--dx-space-7)`, `border-radius:
  var(--dx-radius-full)`, `box-shadow: none`, with an in-file comment "shadcn's measured box-shadow
  is none in both themes") already matches shadcn on this one component — evidence a prior session
  already did a targeted computed-style comparison here and it happened to agree, which is likely
  why the carousel page reads as "close" while the rest of the app doesn't.

## 4. Migration estimate

- **Global/token-level (cheap, one file — `dx-components-theme.css`), S:**
  `--dx-radius-*` scale (retarget base unit 8px→10px, keep the `calc()`-derived steps' proportions),
  ring width/opacity (2px solid → 3px @ 50%, would also need a `border-color` companion which we
  don't have a slot for today). Verify with `computed-style-snapshot.spec.ts` before/after — its
  stated job is exactly proving a token change doesn't silently touch more than intended, run on
  full component set. Effort: S (config + 1 snapshot run), risk: low, but this alone will NOT make
  us look like Nova — Nova's actual character (flatness, size compression) isn't in the token layer.
- **Per-component CSS edits (M-L):** removing `shadow-xs`/our elevation scale from card, checkbox,
  select/combobox/popover surfaces, secondary+outline buttons (~15-20 of the ~56 component
  `style.css` files use `--dx-shadow-*` per the row 31a survey); compressing height/padding scales
  toward Nova's tighter steps (button h-6..9 vs our current heights, card/tabs padding). This is a
  per-file, per-slot job — the exact "class not coincidence" case CLAUDE.md flags, but the
  construction here has to be "flatten + compress," applied file by file, because there's no single
  shared "elevation" or "density" switch today (the elevation scale is used, not abolished, by
  design in row 31a). Effort: M for foundations-only (card/button/input/badge/tabs/dialog/select
  ~10 components), L for all ~56.
  scripts/check-css-literals.sh already blocks new hard-coded values, so edits are forced through
  tokens — that discipline lowers the regression risk of this phase but doesn't reduce its size.
- **Structural/markup changes:** none identified for the components sampled — Nova's differences
  from Vega and from our theme are all CSS-recipe-level (utility classes / custom-property values),
  no new DOM slots required.
- **Risk:** ~56 components × visual regression surface; axe color-contrast (row 39) constraints
  already shape our color tokens and would need re-validation if colors move toward shadcn's OKLCH
  scale (out of scope for a pure Nova-density adoption, in scope if the color-system gap is also
  addressed).

## 6. Semantic-role-token mechanism: shadcn vs ours (scope extension)

### shadcn's mechanism (apps/v4/app/globals.css)
- Full role-token set (`:root`/`.dark`, lines ~99-170): `--background/--foreground`, `--card/--card-foreground`,
  `--popover/--popover-foreground`, `--primary/--primary-foreground`, `--secondary/--secondary-foreground`,
  `--muted/--muted-foreground`, `--accent/--accent-foreground`, `--destructive/--destructive-foreground`,
  `--border`, `--input`, `--ring`, `--chart-1..5`, `--sidebar` + `--sidebar-{foreground,primary,
  primary-foreground,accent,accent-foreground,border,ring}`, plus `--surface/--surface-foreground`,
  `--code/--code-foreground/--code-highlight/--code-number`, `--selection/--selection-foreground`, and
  `--radius` (with `--radius-sm/md/lg/xl/2xl/3xl/4xl` as `calc(var(--radius) * N)`). All values OKLCH.
  `.dark` re-declares the same names with different OKLCH values — no separate dark variable names.
- `@theme inline { --color-background: var(--background); ... }` (lines 43-96) is the bridge: it maps every
  bare role name onto a Tailwind v4 utility (`--color-primary` → class `bg-primary`/`text-primary`/etc.),
  so components are written once as `bg-primary text-primary-foreground` and both themes/every style reuse
  the same utility classes.
- Styles (Nova et al.) layer **only** `@apply` bundles referencing these same role names/utilities
  (`bg-secondary`, `text-muted-foreground`, `border-ring`, confirmed: `grep oklch\|--background: style-nova.css`
  → 0 hits, only `.style-nova {` at line 1) — a style never redefines a role, it only changes which utility
  classes (padding/radius/shadow/etc.) get attached to a `cn-*` slot. This is why the two axes (role tokens
  vs. style) are independently swappable in shadcn's system.

### Our mechanism (`preview/assets/dx-components-theme.css`)
- No semantic-role layer at all. Colors are a **numbered ramp**: `--primary-color`/`-1..7` (near-black→near-white
  in dark, mirrored in light) and `--secondary-color`/`-1..6`, plus fixed status pairs (`--primary/secondary-
  {success,warning,error}-color`, lines 46-88). Every value is `var(--dark, X) var(--light, Y)` — a *space*-separated
  two-value fallback pair, toggled by `--dark`/`--light` being `initial` or empty per `data-theme` attribute
  (or `prefers-color-scheme` when unset, lines 13-40) — a different dark-mode mechanism than shadcn's separate
  `.dark {}` block, but not itself the semantic gap.
- Components reference ramp steps **directly** (`var(--primary-color-4)`, `var(--secondary-color-5)`, …) — there
  is no `--dx-border`/`--dx-muted-foreground`/`--dx-card` name a component writes; the ramp step *is* the API.

### Mapping table (grep across `preview/src/components/*/style.css`, 64 of 72 components use the ramp directly)
| shadcn role | Our closest equivalent usage | Consistency |
|---|---|---|
| border | `--primary-color-6` (18x, plurality) but also `-7` (4x), `--secondary-color-4` (8x), `-6` (1x), `-5` (1x) | **Inconsistent** — 5 different steps used as "the border" |
| muted-foreground / foreground (secondary text) | `--secondary-color-4` (72x) and `-5` (66x) roughly split, plus `-1` (49x, `-2` 10x, `-3` 6x, `-6` 4x) | **Inconsistent** — no single step owns "muted text"; `-1` is also used near primary-text contexts elsewhere |
| card/popover/surface background | `--primary-color` bare (41x), `-4` (35x), `-5` (29x), `-3` (19x), `-7` (17x), `-6` (10x), `-2` (10x), `-1` (5x) | **Highly inconsistent** — every one of the 8 ramp steps is used as *some* component's "surface" |
| primary text/foreground | `-5` (15x), `-4` (13x), bare (8x), `-7` (6x), `-6`/`-2` (3x each), `-1` (3x), `-3` (1x) | Inconsistent, same spread |
| popover surface specifically | `popover/style.css:262,320`, `dropdown_menu/style.css:24` all use `-4` | Consistent *within* this one cluster |
| ring/focus | `--focused-border-color` (a separate, dedicated token, 12 usage sites) — this is the one role that *is* already semantic | Consistent |
| destructive | `--primary-error-color`/`--secondary-error-color` (25 sites) — also already semantic | Consistent |

Net: **status colors and the focus ring already have real semantic tokens; everything else (surface, border,
muted text, primary text) is ad hoc per-component ramp-step selection**, tuned by eye per component rather than
by a shared rule. This is the row 39 axe-contrast fix's likely origin too — one step (`--secondary-color-5`) was
adjusted for contrast and 28 components consuming it were all affected together, evidence the ramp is already
being used *as if* semantic (one token, many consumers) without being named that way.

### Migration estimate
- **New role names**: prefer **`--dx-` prefixed with a shadcn-name fallback**, e.g.
  `--dx-border: var(--shadcn-border, var(--primary-color-6))` — keeps this app's existing `dx-` naming
  convention (avoids collisding with a host app's own bare `--border`/`--primary` custom properties, a real risk
  since this is a component library meant to drop into arbitrary host pages) while still accepting a shadcn
  theme's variable names as an optional override for drop-in compatibility. Bare shadcn names alone would be
  the more "drop-in compatible" choice but risk exactly the collision this hedges against.
- **Size**: ~15-20 new role tokens; ~424 call sites across 64 component stylesheets to repoint from a ramp
  step to a role name (mechanical `var(--primary-color-N)` → `var(--dx-border)` etc., but each site needs a
  human/LLM judgment call on *which* role it actually means — this is the "which step means what" work the
  table above starts).
- **Value-preserving proof**: yes — `computed-style-snapshot.spec.ts` is exactly suited to this: run before,
  do the repoint so each new role token is defined equal to today's already-chosen step for that component,
  run after, diff must be empty. This proves the *rename* introduced no visual change, isolating that from
  any later, deliberate step-value correction (e.g. unifying "border" onto one step) as a separate, visible
  change.
- **Upstream-divergence cost**: low for this step — the ramp/theme file lives in `preview/assets/`, not in the
  `DioxusLabs/components` primitives fork this repo tracks (row 66/CLAUDE.md); primitives supply unstyled
  markup/behavior, not color values, so renaming CSS custom properties here doesn't touch anything upstream
  reviews or rebases against. Effort: **M** (mechanical repoint + per-site judgment, no markup changes,
  provable value-preserving via the existing snapshot tool).

## 7. Recommendation (see report)
