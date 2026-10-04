# Unification audit: shared concerns implemented ad hoc per component (2026-10-04)

**Basis.** Static read of the working tree at `44a103e` plus the uncommitted diff (110 dirty entries, diff hash
`225d06b171b9`). Several lanes were editing while this ran (`toggle/style.css` changed between two of my reads), so
line numbers are approximate and drift. Nothing was built, run or screenshotted: every "measured" value below is a value
read from CSS/Rust, and anything that depends on rendered behaviour is labelled **H** (hypothesis, needs execution) and
collected in section 6. Nova references are `shadcn-ui/ui@295a1f1`, `apps/v4/registry/styles/style-nova.css` (`.cn-*`
blocks) and `registry/bases/base/ui/*.tsx`.

**Out of scope by instruction.** The overlay-scrim unification (Popover/Dialog/AlertDialog/Sheet/Drawer) and the
menu-item indicator tokens. Section 2 checks whether those two are *complete*; the rest of the document is new ground.

---

## 1. Ranked summary (by user-visible impact)

Effort S = under a day, M = a lane, L = multi-lane. "Gate" is whether a script or stylelint rule can stop the class
coming back.

| # | Concern | Instances (where) | Visible consequence | Construction | Gate? | Effort / risk |
|---|---------|-------------------|---------------------|--------------|-------|---------------|
| 1 | **Floating-surface recipe** (edge, shadow, padding, min-width, radius, offset, enter/exit motion) | 12 surfaces: popover, select, combobox, dropdown, context menu, menubar, navbar, navigation menu, hover card, tooltip, color picker, date picker. 6 edge/shadow idioms across them (7 with the dialog's), open durations 100/150/200 ms, 5 distinct curves, 66 `@keyframes` of which 46 sit in byte-identical clusters that collapse to 14 | Menus have a 1px ring and no drop shadow; Select/Popover have ring plus `shadow-md`; hover card and color picker carry hard-coded literal shadows (the colour picker's shows in dark mode too). Same surface family animates 100/150/200 ms on five different curves. Nova is ONE recipe | Theme-level `:where([data-dx-surface="floating"])` rule (surface + motion) + two shared keyframe pairs + `--dx-floating-offset`/`-duration`/`-ease` tokens; per-component CSS keeps only width/min-width/padding | Yes: computed-style parity spec (like `assert-backdrop-fade.ts`), `check-duplicate-keyframes.sh`, stylelint ban on literal `rgb(`/`#hex` in `box-shadow` | M / M (visible change in ~8 components; needs the row-111 computed-style snapshot review) |
| 2 | **Modal panel recipe + the scrim leftovers + close buttons** | dialog, alert_dialog, sheet, drawer, command dialog, calendar popup. Surface token differs: `--dx-popover` (dialog) vs `--dx-background` (alert/sheet/drawer) vs `--dx-card` (command). **Command dialog still paints its own 30% wrapper scrim on top of the UA `::backdrop`** (the original bug class) | In dark mode the Dialog panel is `#262626` while AlertDialog/Sheet/Drawer are `#000` and Command `#141313`. AlertDialog is not on Nova at all (radius 10px, border + 18% literal shadow, 32/24px padding, 20px/700 title, 8px/18px buttons) next to a Nova Dialog. Sheet close is 24px/0.7 opacity, Dialog close is 28px ghost, toast close has no focus style | One `--dx-panel-*` set or a `:where(dialog[class*="dx-"])` rule: bg popover, ring-subtle edge, radius-xl, 16px padding/gap, fade+zoom; Sheet/Drawer opt into slide by `data-side`; shared `::backdrop` block moved to the theme once; Close = real ghost icon `Button` composition | Yes: extend `assert-backdrop-fade.ts` to Command; surface-colour parity spec across modal panels; stylelint `font-family` rule (see 3) | M / M (AlertDialog re-skin moves layout; `alert-dialog.spec.ts` + snapshot) |
| 3 | **Typography inheritance** | 5 stylesheets hard-code a system font stack: `dialog:158`, `alert_dialog:156`, `sheet:145`, `drawer:173`, `calendar:11`. At least 11 text-bearing button stylesheets never set `font-family` (toggle, toggle-group, accordion, tag group, pagination, toolbar, dropdown trigger, navbar, menubar, navigation menu, collapsible) and there is no global `button { font-family: inherit }` (`main.css:221` only does `touch-action`) | Modal and calendar text renders in the OS font while the page is Geist (`main.css:103`); those triggers fall back to the UA button font. **H3** | Zero-specificity theme rule next to the row-111 border-box rule: `:where(button,input,select,textarea)[class*="dx-"]{font-family:inherit;letter-spacing:inherit}`; delete the 5 literal stacks and the 13 `font-family: inherit` repeats | Yes: stylelint `declaration-property-value-allowed-list` `{"font-family":["inherit","/^var\\(/"]}` on `src/components/**` | S / low (text widths shift; snapshot) |
| 4 | **Control state recipes: focus, invalid, disabled** | Focus: 7 ring families (Nova `--dx-ring-focus` x9, `--dx-ring` x14, `outline: 2px` x10, inset 2px x2, sidebar, slider 4px, a hard-coded blue in drag-and-drop). Invalid: 3 vocabularies (`aria-invalid`, `data-invalid`, none). Disabled: 10 sites use `--dx-opacity-disabled`, 18 hard-code `0.5`, 6 only recolour | Combobox input and Textarea show **no focus ring** (`outline: none`, muted background only). Field's `invalid` never reaches the control; only Button and Input style `aria-invalid`. Disabled dropdown/context items are just muted text, everything else is 50% opacity | Theme tokens `--dx-ring-invalid`, `--dx-disabled` (opacity + pointer-events) and one rule per state keyed on attributes; controls compose edge + ring through a `--dx-control-edge` custom property | Yes: `check-css-state-recipes.sh` (rule-level: `:disabled`/`[data-disabled]` rules must use the token, `:focus-visible` rules must use a focus token or be `outline:none` plus a ring rule) | M / M (small visual deltas everywhere; a11y-positive) |
| 5 | **Control chrome and sizing (Nova adoption)** | input/select trigger/button/input-group/popover-trigger are Nova (2rem, transparent / `input/30`); combobox input 2.25rem on `--dx-card`; textarea on `--dx-card` with 8/12px padding; native select and OTP slot use a literal `#FFFFFF26`; OTP 2.5x2.75rem text-lg; toggle has no height; toggle-group `35px`/`10px` | A form row mixes 32px, 36px and content-driven heights and three different fills; `native_select`/`input_otp` dark fill ignores theme presets | `data-dx-control` rule (fill, edge, hover, focus, invalid, disabled once) or `--dx-control-*` tokens; `PopoverTrigger` composes `Button` instead of "kept in step by hand" | Partial: stylelint ban on hex colour literals; Playwright height-parity spec for a Field row | M-L / M (this is row 111 phase C expressed as a class) |
| 6 | **Item rows and the highlight model** | dropdown/context/menubar/navbar items 8x12px, radius `calc(lg - sm)` = 4px; select option 4x6px, radius 8px; combobox 6x8px; command 8x12px; Nova `px-1.5 py-1 rounded-md`. Highlight by CSS `:hover`+`:focus-visible` (6 hosts) vs `data-highlighted` set on mouseenter (combobox, command) | Dropdown/context/menubar rows are 8px taller than Select rows (padding 8+8 vs 4+4, same font size), so a Dropdown and a Select in one demo disagree. Two rows can look highlighted at once. **H2** | `--dx-menu-item-padding-*`/`-radius` tokens beside the indicator tokens; every item primitive emits `data-highlighted` and moves roving focus on pointer move | Yes: Playwright "exactly one highlighted row" spec | M / M |
| 7 | **Reduced-motion coverage** | 14 stylesheets with keyframe/transition-driven motion and no override: dropdown, context menu, menubar, select, combobox, command, color picker, date picker, toast, accordion, skeleton, avatar, progress, sidebar. The theme layer only covers interactive roots (`dx-components-theme.css` ~500-520) | Menus, listboxes and toasts animate under `prefers-reduced-motion: reduce`; infinite loops would flicker if only `animation-duration` were cut (**H4**) | Theme-level `@media (prefers-reduced-motion: reduce) { :where([class*="dx-"]) { animation-duration, animation-iteration-count: 1, transition-duration } }` | Yes: Playwright `emulateMedia({reducedMotion})` sweep over `getAnimations()` | S / low |
| 8 | **Anchored-overlay marker enumeration and the triple side-placement** | Engine stylesheet (`top_layer.rs` ~200-330) lists 9 `dx-anchor-*` markers in each of 5 selector lists; `context_menu.rs:1854` borrows `dx-anchor-dropdown-menu`. Legacy `[data-side]` placement is also hand-copied into popover (18 selectors, incl. dead `::after` arrow rules), tooltip (18), hover card (16), with 8/8/10px gaps; a third copy is the JS fallback | The `menu_root.rs` doc says the three menu hosts (dropdown 2253 / context 2224 / menubar 1438 lines, 10 identically named components) stay duplicated *because of* the fixed marker list. Gap is 8px everywhere vs Nova 4px (menubar 8, combobox 6); component `margin-top`s are outranked dead CSS on the web arm | One generic marker (`dx-anchored`) + `--dx-anchor-gap`; engine lists collapse 9 to 1; web arm drops the per-component `[data-side]` copies (native arm keeps one shared sheet) | By construction (no list left to forget); Rust grep as backstop | M-L / M-H (10 files, `top-layer.spec.ts` rules 1-15) |
| 9 | **JS-owned resource lifetimes** (the timer-leak class, other spellings) | `virtual_list.rs:172` anonymous `window` resize listener never removed; `form/component.rs:249-252` document capture listeners never removed; `sidebar/component.rs:172,240` window-global handler slots overwritten on effect re-run; ~25 hand-rolled `dioxus.recv()` add/remove idioms | Listener and closure growth per mount, same shape as the `setInterval` incident. The new gate only matches `setInterval`/`spawn_forever` | `use_dom_listener(target, event, opts, handler)` owning add/remove (+ `use_raf_loop`); migrate the three offenders | Yes: `check-eval-listeners.sh` (per eval literal: named handler + matching `removeEventListener`). Would have flagged both today (add/remove balance: virtual_list 2/1, form 6/0) | S gate + 2 fixes / M hook migration, low |
| 10 | **`touch-action: manipulation` pasted into every stylesheet** | 95 declarations in 33 files, 44 copies of the same ~400-char comment | Pure maintenance cost: any change is a 33-file edit; the preview-only `main.css` backstop (`!important`) hides drift from the demo site | Move into the theme (it ships to consumers; `main.css` does not), same flat role list, zero specificity; delete the 95 | Yes: stylelint `declaration-property-value-disallowed-list` `touch-action: manipulation` in components | S / low |
| 11 | **Missing half-step spacing tokens + literal drift** | `0.375rem` x20, `0.625rem` x16, `0.125rem` x7, `0.3125rem` x1 (Nova's 1.5/2.5/0.5 steps); `10px` x14, `5px` x6, `18px` x3 | Density cannot be retuned from the theme; `check-css-literals.sh` cannot see them because no token exists | `--dx-space-0-5/1-5/2-5/3-5`; widen the gate to `animation*`, `opacity`, `inset`, px sizes | Yes (existing gate, widened) | S / low |
| 12 | **Chevrons, indicators, close glyphs** | Chevron size attr 16px (select, combobox, nav-menu, sub-triggers) vs 20px (accordion, navbar, date picker, calendar); stroke via inline token x3 foreground, x3 muted-foreground, `currentColor`, or `opacity` .5/.7; rotate 300 ms (accordion) vs 150 ms (navbar, nav-menu, date picker), select never rotates | Same affordance, three sizes, four dimming methods | Wrapper `ExpandIcon` or theme rule on `[data-dx-chevron]` (Nova: `size-4 text-muted-foreground`) | Partial (grep for inline `stroke:` in `component.rs`) | S / low |
| 13 | **Shadows and colour literals outside the token set** | toast `0 4px 12px rgb(0 0 0/15%)` (a copy of `--dx-shadow-lg` that is not dark-aware), calendar, sidebar x2, hover card, alert dialog, color picker x3, drag-and-drop, command scrim `rgb(0 0 0/30%)` | Drop shadows on dark surfaces (the theme turns them off on purpose) and values that ignore presets | Use `--dx-shadow-*`; add `--dx-shadow-inset-edge` | Yes (stylelint, see row 1) | S / low (mostly subsumed by 1-2) |
| 14 | **Primitive-level leftovers** | `AlertDialogRoot` re-implements the `use_controlled` body (`alert_dialog.rs:~110` vs `dialog.rs:170`); four independent runtime `<style>` injectors (anchor, top-layer ink, toast base, scroll-lock gutter); Escape handled three ways (global stack, element-local keydown, native `popover=auto`) | Layered-Escape behaviour is only tested one overlay at a time (**H1**) | Use `use_controlled`; one `ensure_engine_styles()`; Escape stack covers every dismissable | Partial | S each / low-M |
| 15 | **Combobox list `z-index`** | `combobox/style.css:97` uses `--dx-z-local-xl` (50); every other overlay uses `--dx-z-overlay` (1000) | Native/Blitz arm only (top layer hides it on web) | One token | Yes (stylelint `z-index` allow-list) | S / none |

**Top 3 I would do first** (full reasoning in section 7): (a) one *theme-baseline* PR that closes rows 3, 7, 10, 11 and the
two completeness gaps in section 2 with the proven row-111 mechanism (zero visual change except fonts and the Command
scrim); (b) the floating-surface recipe (row 1), the largest visible win and what Nova defines once; (c) the state
recipes (row 4), because two of its instances are real accessibility gaps (no focus ring on Combobox/Textarea) and it
unblocks row 5.

---

## 2. Completeness of the two already-unified concerns (not findings, but not done yet)

### C1. Overlay scrim

1. **Command dialog is a sixth, un-unified painter.** `primitives/src/command.rs:~899` builds `CommandDialog` from
   `DialogRoot` + `DialogContent`, so on web it is a real `<dialog>` + `showModal()`, i.e. it has a `::backdrop`. The
   stylesheet has no `::backdrop` rule for it (`grep ::backdrop` hits only dialog/alert_dialog/sheet/drawer/popover) and
   `command/style.css:33` still paints `.dx-command-dialog-backdrop` at `rgb(0 0 0 / 30%)`, no blur, 150 ms
   `--dx-motion-duration-base` keyframes. That wrapper sits over the UA's default 10% `::backdrop`: exactly the stacked
   scrim the lane removed from the other five. The panel `.dx-command-dialog` is also not `dialog.dx-dialog`, so it gets
   no fade/scale and no `overlay ... allow-discrete` exit. No spec calls `assert-backdrop-fade.ts` for command
   (`grep` finds only alert-dialog, dialog, drawer, popover, sheet).
2. **The "one token set" does not exist yet.** `--dx-overlay-duration/-ease/-scrim/-blur` are defined nowhere
   (`grep` over `preview/assets/*.css`: zero hits); every use carries its fallback, `blur(var(--dx-overlay-blur, 4px))`
   and `var(--dx-overlay-scrim, rgb(0 0 0 / 10%))`, repeated: dialog 13 uses, alert_dialog 12, sheet 9, drawer 10, popover 6.
   `.dx-dialog::backdrop` and `.dx-sheet::backdrop` blocks are byte-identical after renaming (checked with `diff`). That
   is one *shape* pasted five times and guarded at runtime, not one construction: the copy-paste that produced the
   original drift is still the mechanism. Move the block into the theme once, keyed
   `:where(dialog[class^="dx-"], dialog[class*=" dx-"])::backdrop` (the row-111 precedent; `#[css_module]` only appends a
   suffix), define the four tokens in `:root`, and the five stylesheets lose ~40 lines each.
3. **Non-modal Popover's panel now reads the scrim token** (`popover/style.css:55-59`: `animation: ... var(--dx-overlay-duration...)`):
   200 ms, while hover card (100 ms), select/menus (150/100 ms) and tooltip (200 ms, `ease-in-out`) disagree. This is row 1.
4. **Sheet panel timing disagrees with its own scrim.** Slide-in is `500ms ease-in-out` (literals, `sheet/style.css:~223`),
   slide-out 300 ms; scrim is 200 ms. Nova's `.cn-sheet-content` is `duration-200 ease-in-out`. Drawer is 200/150 ms
   `ease-out`/`ease-in`. Same component family, three timings.

### C2. Menu-item indicator tokens

1. The tokens are **declared per component, not in the theme**: `--dx-menu-item-indicator-size/-inset/-gap` appear as
   declarations in 8 rule blocks (dropdown_menu:~126, menubar:~142, context_menu:~89, select:236, combobox:28 and 178,
   native_select:8, command:173 gap-only) and in `dx-components-theme.css` zero times. `check-css-vars-defined.sh` passes
   because each is defined *somewhere*. A consumer retuning the indicator must override 8 selectors. Hoist the three
   to `:root` and keep only the `inset` override (`space-2` in select/combobox options, `space-3` elsewhere) local.
2. Not covered by the tokens and still ad hoc (row 12): the chevron/check **size attribute**, **dimming** (`opacity: .7` on
   sub-trigger icons, `.5` on the combobox expand icon, muted stroke on select/native-select) and **rotation** timing.
3. The menubar draws its check at inline-start by design (Nova `pl-7`); that is documented and fine.

---

## 3. The mechanism most of these want (and its constraints)

Row 111 already proved a construction that fits this repo: **a zero-specificity theme rule keyed on the `dx-` class
prefix** (`:where([class^="dx-"]), :where([class*=" dx-"])`, `dx-components-theme.css` 388-425). It works with
`#[css_module]` (hashing appends a suffix, so the prefix selector still matches), ships to every `dx components add`
consumer (the theme is the one file they import), and any component rule still wins because the specificity is zero.

Concerns that need *no marker at all* and can use exactly that: rows 3 (font), 7 (reduced motion), 10 (touch-action),
the scrim `::backdrop` (C1), parts of 4 (disabled/invalid attribute states).

Concerns that need to know "this element is a floating surface / a control / a menu item" (rows 1, 2, 5, 6) need one
marker. Options, cheapest first: (a) the UA's own `[popover]` / `dialog` for floating and modal surfaces (zero Rust
change, web arm only, the native arm keeps component CSS); (b) a `data-dx-surface` / `data-dx-control` attribute emitted
by the themed `component.rs` wrappers (themed layer only, so primitives stay theme-free; primitives already emit
`data-state`/`data-side` the same way). The inert `[data-dx-interactive]` layer (`dx-components-theme.css` ~428-500) is
this idea half-built: nothing in `primitives/` or `preview/src/` emits the attribute, so its three rules have never
matched an element.

Constraints from the repo's own notes, to respect: selector lists stay flat (the asset pipeline has dropped a
selector-list-inside-`:not()`); no selectors only inside `@supports` (`css_module` does not scope them); a theme rule
cannot name a component class; `box-shadow` composition needs the `--dx-shadow-*` `none`-in-dark caveat handled with a
border or a custom property, as `--dx-ring-subtle-color` already does.

---

## 4. Detail per concern

### 4.1 Floating surfaces (row 1)

Values read from CSS (working tree):

| Surface (file:line) | Edge | Drop shadow | Padding | Size | Open | Close | Gap |
|---|---|---|---|---|---|---|---|
| Popover `popover/style.css:12` | `border 1px ring-subtle` | `--dx-shadow-md` | 0.625rem | w 18rem | fade 200 ms, `--dx-overlay-ease` | fade 200 ms | `margin-top` space-2 |
| Select `select/style.css:94` | `border 1px ring-subtle` | `--dx-shadow-md` | space-1 | min = anchor width | `dx-picker-in` 150 ms ease-out | `dx-picker-out` 100 ms ease-in | space-1 |
| Combobox `combobox/style.css:95` | inset 1px `--dx-input` | none | space-1 | w 200px fixed, max-h 300px | picker-in 150 | picker-out 100 | `calc(100% + .25rem)` |
| Dropdown `dropdown_menu/style.css:39` | inset 1px `--dx-input` | none | space-1 | min 200px | 150 ms ease-out (+ dead `animation: dx-slideIn`, line 73: no such keyframe) | 150 ms ease-in | space-1 |
| Context menu `context_menu/style.css:1` | inset 1px input | none | space-1 | min 220px | 150 ms | 150 ms | n/a (click point) |
| Menubar `menubar/style.css:46` | inset 1px input | none | space-1 | min 200px | 150 ms | 150 ms | space-2 |
| Navbar `navbar/style.css:57` | inset 1px input | none | space-1 | min 200px | 200 ms `cubic-bezier(.16,1,.3,1)` slide .5rem | 150 ms | space-2 |
| Navigation menu `navigation_menu/style.css:78` | inset 1px input | none | space-1 | min 280px, `max-content` | 200 ms same curve | 150 ms | space-2 |
| Hover card `hover_card/style.css:14` | `border 1px --dx-input` | `0 2px 10px #0000001a` light only (literal) | **5px** | min 200px | fade 100 ms ease-out | 100 ms | **10px** |
| Tooltip `tooltip/style.css:14` | none (inverted fill) | none | 8x12px | max 250px | fade 200 ms **`ease-in-out`** literal | 200 ms | space-2 |
| Color picker `color_picker/style.css:190` | inset 1px input | `0 4px 16px rgb(0 0 0/15%)` **all modes** | none | n/a | fade+4px slide 150 ms | same | space-1 |
| Date picker `date_picker/style.css:71` | (calendar's own card: `border --dx-border`, `--dx-background`, literal `0 2px 10px`) | literal | n/a | n/a | fade+4px slide 150 ms | same | n/a |

Plus the dialog family's own idiom (`box-shadow: var(--dx-ring-subtle)` on `.dx-dialog`, a box-shadow ring where popover/select
use a border ring), which makes **7** edge idioms.

**Nova**: every floating surface is `bg-popover text-popover-foreground rounded-lg ring-1 ring-foreground/10 shadow-md
duration-100` with `fade-in-0 zoom-in-95` and a `slide-in-from-<opposite>-2` per `data-side`
(`.cn-dropdown-menu-content` 537, `.cn-select-content` 1019, `.cn-popover-content` 926, `.cn-hover-card-content` 668,
`.cn-menubar-content` 806, `.cn-context-menu-content` 414, `.cn-combobox-content` 296). Sizes: `min-w-32` dropdown,
`min-w-36` select/combobox/context/menubar, `w-64 p-2.5` hover card, `w-72 p-2.5` popover; tooltip `rounded-md px-3
py-1.5 text-xs`. `sideOffset` is 4 for popover/dropdown/hover-card/select/tooltip, 8 menubar, 6 combobox, 0 for submenus
(`registry/bases/base/ui/*.tsx`).

Further duplication in this family:

- **UA `[popover]` reset re-derived in 11 stylesheets** (combobox, context_menu, dropdown_menu, hover_card, menubar, navbar,
  navigation_menu, popover, select, toast, tooltip), each with a 10-20 line comment explaining the same WHATWG default
  (`border: solid; overflow: auto; inset: 0; margin: auto; padding`). `TOP_LAYER_INK_STYLES` (`top_layer.rs:624`) already does
  part of it engine-side with `:where([popover])`.
- **Slide-direction ignores `data-side`.** Every pop keyframe is `translateY(-2px)` from above and `transform-origin: top`
  (`popover:50`, `select:131`, `menubar:86`, `navbar:97`, `navigation_menu:116`, `combobox:128`), so a flipped (top-placed)
  menu still scales from its top edge. Nova slides from the opposite side and, in Radix, the origin follows the trigger.
- **Keyframe duplication**, measured by hashing normalised bodies across all `preview/src/components/*/style.css`:
  66 definitions, 46 of them members of 18 byte-identical clusters that reduce to **14 canonical** blocks once the pure
  fades and the pops are merged across clusters:
  pure `opacity` fade in/out (5+5 in drawer-root, hover-card, popover, sheet-root, tooltip, plus 3+3 in
  alert/command/dialog backdrops), menu pop `scale(.95) translateY(-2px)` (3+3 across dropdown/context/menubar, 2+2 as
  `dx-picker-*` in select/combobox), fade+`translateY(-4px)` (2+2, color/date picker), and the 8 sheet/drawer slides
  (`dx-slide-*` vs `dx-drawer-slide-*`).

**Construction.** (1) Theme tokens `--dx-floating-offset` (default 4px, consumed by the engine rule as
`var(--dx-floating-offset, 8px)` so no hard-coded `8px` stays inside a Rust string), `--dx-floating-duration-in/-out`,
`--dx-floating-ease`. (2) A theme rule `:where([data-dx-surface="floating"])` carrying the Nova recipe, and two shared
`@keyframes dx-pop-in/out` selected by `data-state`; tooltip opts into an `inverted` variant. (3) Per-component CSS
keeps width/min-width/padding only; `[popover]` UA reset becomes one `:where([popover][class*="dx-"])` rule.

**Subsumes:** the 12 rows above, 11 repeated reset comments, ~32 duplicate keyframe blocks, rows 1-2 of section 2, the
literal shadows in hover card/color picker/calendar. **Does not subsume:** modal panels (row 2 shares the tokens, not
the rule), the navigation-menu viewport content (grid widths, featured card), the arrow geometry of tooltip.
**Gate:** Playwright parity spec sampling `getComputedStyle` of every surface after open (border, radius, shadow,
padding, `animation-duration`) against the recipe, in the style of `assert-backdrop-fade.ts` (which already shows this
repo's preference for a runtime assertion over a grep for this class); `check-duplicate-keyframes.sh` (hash normalised
bodies, fail on two identical in different files) is cheap and catches the copy-paste directly.

### 4.2 Modal panels (row 2)

| Panel (file:line) | Fill | Edge | Shadow | Radius | Padding / gap | Title | Enter | Exit |
|---|---|---|---|---|---|---|---|---|
| Dialog `dialog/style.css:139` | `--dx-popover` | `--dx-ring-subtle` box-shadow | none | xl | 16 / 16 | 16px / 500 | fade+scale 200 ms (token) | same |
| AlertDialog `alert_dialog/style.css:132` | **`--dx-background`** | `1px --dx-border` | `0 2px 10px rgb(0 0 0/18%)` literal | **lg** | **32 24 24** / 16 | **20px / 700**, centred below 40rem | fade+scale 200 ms | same |
| Sheet `sheet/style.css:131` | **`--dx-background`** | 1px border on one side | `--dx-shadow-lg` | none | none | n/a | slide **500 ms** `ease-in-out` literals | **300 ms** |
| Drawer `drawer/style.css:158` | **`--dx-background`** | 1px border | `--dx-shadow-xl` + 40px `drop-shadow` overdrag trick | 2xl (inner corners) | none | n/a | slide 200 ms ease-out | 150 ms ease-in |
| Command dialog `command/style.css:76` | **`--dx-card`** | `1px --dx-border` | `--dx-shadow-2xl` | xl | 0 | n/a | none (wrapper fade 150 ms only) | wrapper fade 150 ms |
| Calendar popup `calendar/style.css:2` | `--dx-background` | `1px --dx-border` | literal `0 2px 10px` | lg | n/a | n/a | n/a | n/a |

Dark-mode values from the theme: `--dx-popover` = `--primary-color-5` `#262626`, `--dx-background` = `--primary-color`
`#000`, `--dx-card` = `--primary-color-3` `#141313` (light: all white). So under the default dark theme the modal family
paints three different greys. **Nova**: dialog, alert dialog, sheet, drawer and command dialog are all `bg-popover` with
`ring-1 ring-foreground/10`, `rounded-xl p-4 gap-4` (alert dialog 100 ms fade+zoom; sheet `shadow-lg duration-200
ease-in-out`).

Close controls (row 2 continued): Dialog `dialog/style.css:192` is Nova's ghost `icon-sm` (1.75rem, `top-2 right-2`, muted fill on hover,
`--dx-ring-focus`). Sheet `sheet/style.css:396` is 24px, `top/end: space-4`, `opacity: .7`, hover opacity 1, `--dx-ring`
(Nova's `.cn-sheet-close` is the same ghost icon button at `top-3 right-3`). Toast close `toast/style.css:217` is a bare
`font-size: lg` glyph with no hover fill and no focus rule. Drawer has a text button (Nova's drawer has no X: fine).

Fonts: `font-family: -apple-system, ...` on `.dx-dialog`, `.dx-alert-dialog`, `.dx-sheet`, `.dx-drawer`, `.dx-calendar` (row 3).

**Construction.** Define `--dx-panel-*` (or a theme rule on `dialog[class*="dx-"]` plus `[data-dx-surface="modal"]` for the
non-web `div` arm) once; Sheet/Drawer add only the `data-side` geometry and slide. Sheet slide goes to
`--dx-motion-duration-slow` + `ease-in-out` (Nova 200 ms), with the `overlay` hold derived from the same token so panel and
scrim cannot disagree. Close button becomes a composition of the real ghost `Button` (Rust, `dialog`, `sheet`, `toast`
components). Command dialog gets the shared `::backdrop` rule for free (C1). **Subsumes:** C1.1, C1.4, the dark-mode
surface split, the 4 font stacks, 3 close buttons, the AlertDialog legacy buttons (`.dx-alert-dialog-cancel/-action` are
`8px 18px`, 16px font, `--dx-ring` focus: should be `Button`). **Does not:** drawer drag mechanics.

### 4.3 State recipes (row 4)

*Focus.* Nova is one recipe: `focus-visible:border-ring ring-3 ring-ring/50` on button, input, textarea, select trigger,
checkbox, radio, switch, toggle, accordion trigger, input group, native select, OTP slot (`style-nova.css` 8, 150, 284,
973, 1250, 1342, 1338, 900, 690, 1397). In the repo:

- `--dx-ring-focus` (Nova): button:250, input:52, input_group, select:58, popover trigger:297, tabs:100, dialog close:224,
  bubble:72, marker:34; plus two **inline copies of the same formula** (`switch/style.css:26`, `color_picker/style.css:185`)
  and `item/style.css:27` (`color-mix(in srgb ...)`, a different colour space).
- `--dx-ring` (2px solid, older): alert dialog x2, breadcrumb, calendar nav, checkbox:61, dropdown trigger:35, native
  select:43, pagination:42, radio `::before`, resizable:100, sheet close, tag group tag.
- `outline: 2px solid var(--dx-ring-color); outline-offset: 2px`: carousel x4, calendar day cell, drag-and-drop remove,
  tag-group remove, toggle:41, toggle-group:~40.
- inset 2px: accordion:20, collapsible:21. Sidebar `0 0 0 2px var(--dx-sidebar-ring)` x2. Slider thumb 4px.
- A hard-coded blue: `drag_and_drop_list/style.css:62` `rgb(43 127 255 / 18%)` / `30%`, which ignores `--dx-ring-color` and so
  every theme preset.
- **No ring at all:** `combobox/style.css:59` (`outline: none`; focus = `--dx-muted` fill), `textarea/style.css` default and fade
  variants (`outline: none`, muted fill; the outline/ghost variants change only `border-color`).

*Invalid.* Nova puts `aria-invalid:border-destructive ring-3 ring-destructive/20 dark:ring-destructive/40` on 16 recipes.
Here: `button/style.css:~258` and `input/style.css:~58` carry the same recipe in two spellings (border-colour + 3px ring on
Button, inset 1px + 3px ring on Input, identical 20%/40% light/dark mixes), pasted twice; textarea changes the
border colour on the `outline` variant only; `form/style.css:75` paints `0 0 0 2px --dx-destructive` on any `[data-invalid]`
inside a `.dx-form` (Form's JS sets `data-invalid` on the control); `field/style.css:83` only recolours the label for
`.dx-field[data-invalid]`. `Field`'s `invalid` prop (`field/component.rs:46-58`) writes `data-invalid` on the wrapper and never
`aria-invalid` on the control. Select trigger, combobox, native select, checkbox, radio, switch, toggle, input group, OTP
and date picker have no invalid style at all.

*Disabled.* Nova: `disabled:opacity-50` (controls), `data-disabled:pointer-events-none data-disabled:opacity-50`
(items). Here: tokenised in button, carousel, command, input, input_otp, native_select, popover trigger, select (x2),
tabs; literal `0.5` in accordion, color picker, combobox x2, date picker, menubar x2, navbar x2, navigation menu, radio
group, resizable, sidebar x2, slider, switch, tag group; **colour only** in dropdown items (`dropdown_menu:158`), context
items (`context_menu:121`), textarea, toolbar, calendar nav, hover-card trigger. `pointer-events: none` is paired with
`cursor: not-allowed` in select option, combobox option and resizable handle, so the cursor never shows.

**Construction.** Tokens `--dx-ring-invalid` (the Nova two-layer ring, dark-aware) and `--dx-disabled` (a shorthand for
opacity + pointer-events) in the theme, plus theme rules keyed on state attributes, composing the control's own edge via a
`--dx-control-edge` custom property (so `box-shadow: var(--dx-control-edge, 0 0 #0000), var(--dx-ring-invalid)` works
for controls that draw their border as an inset shadow). Components set only `--dx-control-edge`. `Field` forwards `invalid`
to the control as `aria-invalid`. Subsumes the 7 focus families, 3 invalid vocabularies, 3 disabled treatments.
**Does not:** shape-specific rings (radio `::before`, toggle-group joined items' `z-index`), which keep their geometry rule but
read the same token. **Gate:** `check-css-state-recipes.sh` at rule level; an axe/Playwright "every focusable demo shows a
focus indicator" sweep would also have caught combobox/textarea.

### 4.4 Control chrome and sizing (row 5)

Measured: button default `height: 2rem` (`button/style.css:~95`), input 2rem, select trigger 2rem (Nova `h-8`);
popover trigger 5px padding + 20px line + 2px border = 32px by arithmetic (`popover/style.css:~270`, with the comment "kept in
step by hand"). Off Nova: combobox input `height: 2.25rem`, `padding-inline: space-3`, `--dx-card` fill, no ring
(`combobox/style.css:36`); textarea 8/12px padding, `--dx-card` fill (`textarea/style.css:2`); native select
`padding-block: space-2`, no height (`native_select/style.css:17`) with a literal `#FFFFFF26` dark fill; OTP slot 2.5x2.75rem,
`text-lg`, radius-md (`input_otp/style.css:25`, Nova `size-8 text-sm`, joined group); date-picker group padding
`space-2`, content-driven; toggle has no vertical size (`padding-inline` only; Nova `h-8 min-w-8`); toggle-group item
`min-width: 35px; padding: 10px; border-radius: 0` (`toggle_group/style.css:5`); dropdown trigger `padding: space-2 18px`,
16px font, `--dx-card` (`dropdown_menu/style.css:7`), a different button from the Nova-outline popover trigger two stylesheets away.
Fill recipes: `light transparent / dark input/30` in input, input_group, select, popover trigger, tabs, bubble, button
outline, drawer close (the `color-mix(in oklab, var(--dx-input) 30%, transparent)` line appears in 8 stylesheets);
`light background / dark #FFFFFF26 30%` in native_select and input_otp; `--dx-card` in combobox, date picker, textarea,
radio, command, tabs.

**Construction.** A `data-dx-control` rule (fill, edge, hover fill, height, padding-inline, radius) with
`--dx-control-height: 2rem`, `--dx-control-height-sm: 1.75rem` tokens; `PopoverTrigger`/`DropdownMenuTrigger` render `Button`
(`as:` composition) instead of copy-maintained CSS; `InputGroup` becomes a consumer rather than a restatement of
`.dx-input` (its header comment already says "must agree with `.dx-input`"). This is row 111 phase C (open) turned from a
per-component to-do list into one rule. **Gate:** stylelint ban on 6/8-digit hex and `rgb(` literals in component CSS (flags
`#FFFFFF26`); Playwright "a Field row's controls share one height".

### 4.5 Item rows and highlight (row 6)

| Row (file:line) | Padding | Radius | Gap | Highlight | Cursor | Group label |
|---|---|---|---|---|---|---|
| dropdown item `dropdown_menu/style.css:125` | 8x12 | `calc(lg - sm)` = 4px | space-2 | `:hover` + `:focus-visible` | pointer | 8x12, xs/500 |
| context item `context_menu/style.css:88` | 8x12 | 4px | n/a | same | pointer | same |
| menubar item `menubar/style.css:141` | 8x12 | 4px | n/a | same | pointer | same |
| navbar item `navbar/style.css:274` | 8x12 | 4px | n/a | same | n/a | n/a |
| select option `select/style.css:235` | 4 / 6px | md = 8px | 0.375rem | same | pointer | 4 / 6px, xs |
| combobox option `combobox/style.css:177` | 6 / 8px | 8px | space-2 | `data-highlighted` set on `mouseenter` | default | none |
| command item `command/style.css:172` | 8x12 | 8px | indicator-gap | `data-highlighted` set on `mouseenter` | default | 8 12 4, xs |

Nova: dropdown/select/menubar/combobox items `gap-1.5 rounded-md px-1.5 py-1` (+ `pr-8` for indicator, `focus:bg-accent`),
command item `rounded-sm px-2 py-1.5` with `data-selected:bg-muted`; groups `px-1.5 py-1 text-xs`.
`calc(var(--dx-radius-lg) - var(--dx-radius-sm))` appears in 8 places across 6 stylesheets (context_menu, dropdown_menu,
menubar, navbar, navigation_menu, toolbar) as an unnamed 4px. The primitives show why two models exist: `combobox/option.rs:101`,
`command.rs:760` and `selectable` collection set focus on `onmouseenter`; `DropdownMenuItem`, `ContextMenuItem`, `MenubarItem`
and `SelectOption` do not (no `onmouseenter`/`onpointermove` in those item components), so their highlight is whichever of
`:hover` and `:focus-visible` is true, possibly both (**H2**).

**Construction.** `--dx-menu-item-padding-block/-inline`, `--dx-menu-item-radius`, `--dx-menu-item-gap` beside the indicator
tokens, in the theme; one `data-highlighted` attribute emitted by the collection for every item-like primitive, with hover
moving roving focus (what Radix does and combobox/command already do), so a single CSS rule and a single behaviour replace
two. **Gate:** Playwright "hover B while focus is on A: exactly one highlighted row" for every host; stylelint ban on the
`calc(var(--dx-radius-lg) - var(--dx-radius-sm))` literal once the token exists.

### 4.6 Reduced motion (row 7)

Stylesheets with `animation:`/`transition` and no `prefers-reduced-motion` block (excluding interactive roots the theme
layer already covers): dropdown_menu, context_menu, menubar, select, combobox, command (the `.dx-command-dialog-backdrop`
150 ms animation), color_picker, date_picker, toast, accordion (`dx-accordion-open/close`), skeleton and avatar (`pulse`),
progress (indeterminate), sidebar. Meanwhile dialog, alert_dialog, sheet, drawer, popover, tooltip, hover_card, navbar,
navigation_menu, spinner and input_otp each carry a hand-written block. The theme layer names `[role="menuitem"]`/
`[role="option"]` but not the `menu`/`listbox` panels, which are the animated elements. The existing override only sets
`animation-duration` (`0.01ms`), which on an `infinite` animation (skeleton, avatar, progress) produces a rapid loop, not a stop
(**H4**); the standard form also sets `animation-iteration-count: 1`.

**Construction.** One theme block keyed `:where([class*="dx-"])` setting `animation-duration`, `animation-iteration-count: 1`
and `transition-duration`; delete the ~11 hand-written blocks. **Gate:** Playwright `emulateMedia({ reducedMotion: 'reduce' })`
then assert every `document.getAnimations()` is `<= 1ms` or finite, over the gallery routes (the repo already has
`scroll-main-thread.spec.ts` doing a similar page-wide sweep).

### 4.7 Anchored overlays (row 8)

Facts: the engine stylesheet in `top_layer.rs` (`@supports (anchor-name: --a)`, ~200-330) lists
`tooltip, popover, popover:modal, dropdown-menu, menubar, navbar, navigation-menu, select, combobox` in each of five selector
lists (base + top/right/bottom/left), with `margin: 8px` hard-coded inside a JS template string, and hover card in a separate
block with `10px`. `context_menu.rs:1854` gives the context-menu **sub-content** `dx-anchor-dropdown-menu` because no
`dx-anchor-context-menu` marker exists: the enumeration forces a borrowed name. `menu_root.rs`'s own header states that the
`use_anchor_position_fallback` call "still has to live in each file's own leaf render function" because of this fixed list, which is
why `dropdown_menu.rs` / `context_menu.rs` / `menubar.rs` still carry 10 identically named components. Legacy `[data-side]` /
`[data-align]` placement CSS is copied into popover (`popover/style.css:~120-210`, including `::after` arrow rules with no
`content`, i.e. dead), tooltip (`:~60-160`) and hover card (`:~45-115`), plus the third implementation in the JS fallback. The
dropdown/select `margin-top: var(--dx-space-1)` (4px, Nova's offset) is outranked by the engine's `(0,3,0)` rule, or by the
fallback's inline style, so the 4px never applies on the web arm.

**Construction.** One generic marker (`class: "dx-anchored"` or `data-dx-anchor`) and `--dx-anchor-gap` read by the engine;
selector lists shrink from 9 entries x 5 to 1 x 5; the per-component `[data-side]` rules are deleted for the web arm and the
native/Blitz arm gets one shared fallback sheet instead of three copies; the `8px` becomes the row-1 offset token. Frees
`menu_root.rs`'s stated blocker for sharing the positioning call across the three menu hosts. **Gate:** by construction (no
list to forget), with a Rust grep for any `dx-anchor-<name>` string as a backstop. **Risk:** highest of the set, touches
`top_layer.rs` (2590 lines), 10 primitives and `top-layer.spec.ts` rules 1-15.

### 4.8 JS-owned lifetimes (row 9)

The scroll-jank lane fixed the *instance* (a `setInterval` in `BlockPlayer`) and the *spelling* (`setInterval`,
`set_interval`, `Interval::new`, `spawn_forever`) with `use_interval` + `check-uncleared-intervals.sh`, and says plainly it does not
cover `setTimeout` chains, rAF loops or listeners. The same ownership defect, other spellings, in the tree today:

- `primitives/src/virtual_list.rs:172`: `window.addEventListener("resize", () => publish(false), ...)` is anonymous and never
  removed; teardown (`:176`) removes only the scroll listener. One leak per mounted `VirtualList`, holding `publish` (the
  `dioxus.send` closure) and the container.
- `preview/src/components/form/component.rs:249-252`: `document.addEventListener('click' | 'keyup', ..., true)` never
  removed; the `dxInvalidWired` flag stops a double wire on one form element but a remounted Form is a new element and adds
  another document listener.
- `preview/src/components/sidebar/component.rs:172,240`: handlers are stored in `window.__sidebarResizeHandler` /
  `__sidebarKeyHandler`; `use_drop` removes only the *last* one, and an effect re-run overwrites the slot and leaves the previous
  listener attached. The keyboard loop is `loop { if eval.recv::<bool>().await.is_ok() { ... } }`: if the channel errors,
  that never exits (**H5**).
- Shared hooks done correctly: `use_global_keydown_listener`, `use_outside_dismiss`, `use_form_reset_listener`, `use_interval`.
  About 25 other `document::eval` sites in 11 primitive files hand-roll the identical `recv-id / addEventListener /
  recv-sentinel / removeEventListener` idiom (`drawer.rs:255` names it).

**Construction.** `use_dom_listener(target, event, options, handler)` (and `use_raf_loop`) in primitives, owning add/remove, so
a leaked listener is not expressible; migrate the three offenders first. **Gate:** `check-eval-listeners.sh` (per string literal:
every `addEventListener(` needs a named handler and a matching `removeEventListener(` in the same literal, or an
`// listener-ok:` marker, the same escape hatch `check-uncleared-intervals.sh` uses). Both leaks above fail it today. Add the
script to `run-gates.sh` and `CLAUDE.md` (the runner fails any unlisted `scripts/check-*.sh`).

### 4.9 Remaining items (rows 10-15)

- **touch-action (row 10).** `grep -c "touch-action: manipulation"` gives 95 declarations in 33 files; the long explanatory comment
  appears 44 times. `main.css:221-244` has the `!important` global role list, but `main.css` is not shipped to consumers
  (which is the stated reason for the copies). The theme *is* shipped; put the same flat list there at zero specificity.
- **Spacing and literals (row 11).** The scale has `space-1,2,3,4,...` and Nova's half steps (`1.5`, `2.5`, `0.5`, `3.5`) have no token,
  so they are written as literals the gate cannot flag. `check-css-literals.sh` also skips any value starting `var(`, never scans
  `animation*`, `opacity`, `inset`, `width`/`height`, and treats composite shadow tokens as unscannable.
- **Chevrons (row 12).** `accordion` 20px / `--dx-foreground` / rotates 300 ms (`--dx-motion-duration-slower`); `navbar` 20px /
  foreground / 150 ms; `date_picker` 20px / muted / 150 ms; `navigation_menu` 16px / `currentColor` / 150 ms; `select` 16px / muted /
  no rotation; `combobox` 16px / `opacity .5`; `collapsible` `1rem` / foreground; sub-trigger chevrons 16px / `opacity .7`;
  calendar nav `20px`; pagination `1rem`. Nova: `size-4 text-muted-foreground`.
- **Shadows (row 13).** `toast/style.css:~81` `0 4px 12px rgb(0 0 0 / 15%)` equals `--dx-shadow-lg`'s light value but is not
  dark-aware; the others are listed in the table.
- **Primitives (row 14).** `AlertDialogRoot` hand-rolls `use_signal` + `use_callback` + `use_memo` (`alert_dialog.rs:~110-116`) which is
  `use_controlled` (`dialog.rs:170` uses it); `Menubar` and `Accordion` have no controlled API (an API gap versus Radix, not
  duplication). Runtime `<style>` injectors: `top_layer.rs:161` (anchor), `:624` (ink), `toast.rs:393`, `scroll_lock.rs:361`, each with a Rust
  `Cell` guard, a JS `getElementById` guard and "prepend into the dependent eval" ordering machinery (row 50). Escape: global stack
  (`use_global_escape_listener`: dialog, alert dialog, dropdown, popover), element-local `onkeydown` (select list, combobox input,
  menubar, navbar, navigation menu, hover card x3, tooltip), native `popover="auto"` light dismiss (select, dropdown). Typeahead:
  prefix matching in menus (`typeahead.rs`), Levenshtein in Select (`select/text_search.rs`), documented as deliberate.
- **Z-index (row 15).** Otherwise fully tokenised; `top_layer/style.css:95` carries a literal `9999` (a fixture, excluded from the gate).

---

## 5. Checked and found sound (so nobody re-audits them)

- **Motion duration tokens are widely adopted**: 87 `--dx-motion-duration-slow`, 66 `-base`, 23 `-fast`, 21 `-reduced`, 9 `-slower` in component
  CSS. Literal durations survive only in sheet (500 ms), tooltip (`ease-in-out`), navbar/navigation-menu (`cubic-bezier(.16,1,.3,1)`),
  progress, drag-and-drop, message-scroller and infinite loops.
- **Z-index** ladders are tokenised everywhere except row 15.
- **`use_animated_open`** is used by every animated overlay (19 primitives) and reads `getAnimations()`, so swapping keyframes for
  shared ones needs no Rust change.
- **Ids**: `use_unique_id` / `use_id_or` in 42 primitive files; the only other counters are toast item ids, the accordion registry
  and `use_portal` (toast only, client-side).
- **Controlled state**: `use_controlled` in 24 files; exceptions in row 14.
- **Roving focus**: `collection.rs` is shared by 15 primitives; `menu_root`/`menu_semantics`/`menu_item`/`menu_sub` already factor the
  three menu hosts' non-positioning plumbing.
- **Icons**: every component uses `dioxus_icons::lucide`; no duplicated inline SVG in `component.rs`.
- **Outside dismiss / scroll lock / focus restore**: shared hooks (`use_outside_dismiss` in 10 primitives, `use_scroll_lock`,
  `use_refocus_on_close_unless`); the narrower variants (`use_sub_outside_dismiss`, context-menu's wheel blocker, menubar's own refocus)
  are documented as deliberate.
- **Tall-list clamping**: the engine already caps any anchored overlay to the viewport (`top_layer.rs` ~1140-1190), so the static
  `max-height` differences (combobox 300px, command 18rem, Nova `max-h-72` = 18rem) are cosmetic.
- **Primitives contain no inline visual literals** (`box-shadow`, `rgb(`, `z-index`, `transition:` outside the engine style blocks).

---

## 6. Hypotheses that need execution (not measured here)

- **H1 Layered Escape.** A `Select` inside a `Dialog`: the select list's web arm returns early on Escape (leaving the native
  `popover="auto"` close to run, `select/components/list.rs:~147`) while `DialogRoot`'s `use_global_escape_listener` is a `document`
  keydown that calls `preventDefault`, and Select is not on its stack. Expect either both close or the select does not. Repro: open
  Dialog, open the Select inside it, press Escape; expected is only the Select closes. No current spec covers a nested pair
  (`top-layer.spec.ts` rules 2-3 test one overlay at a time).
- **H2 Double highlight.** Dropdown/Select/Context/Menubar: keyboard-focus item A, then hover item B; both match
  `:focus-visible`/`:hover` styles. Repro with Playwright `toHaveCSS` on both rows.
- **H3 Font fallback.** `getComputedStyle(button).fontFamily` for `.dx-toggle`, `.dx-accordion-trigger`, `.dx-menubar-trigger`,
  `.dx-pagination-link`, `.dx-dropdown-menu-trigger`, and for `.dx-dialog`/`.dx-calendar`, against `body`'s Geist stack.
- **H4 Reduced motion.** With `emulateMedia({ reducedMotion: 'reduce' })`, list `document.getAnimations()` after opening a dropdown,
  select, toast, and loading a page with skeleton/progress; expect menus still at 150 ms and the infinite loops still running.
- **H5 Sidebar listener loop.** Close the eval channel (navigate away within the SPA) and watch for a busy `recv` loop in
  `sidebar/component.rs:~247`.

---

## 7. Suggested sequencing and lanes (disjoint file sets, per CLAUDE.md)

1. **Lane T, theme baseline (serial, first).** Owns `preview/assets/dx-components-theme.css` (+ `main.css` removals). Adds:
   `font-family: inherit` rule (row 3), reduced-motion block (row 7), `touch-action` rule (row 10), `--dx-overlay-*` and
   `--dx-menu-item-indicator-*` tokens and the shared `::backdrop` rule (section 2), `--dx-space-*-5` tokens (row 11),
   `--dx-floating-*`, `--dx-ring-invalid`, `--dx-disabled`, `--dx-control-*` tokens. Every other lane reads this file, so land
   it first and rebase the rest. *Why first:* all mechanical, zero visual change except fonts and the Command scrim, and it
   closes four classes by construction.
2. **Lane F, floating surfaces** (popover, select, combobox, dropdown_menu, context_menu, menubar, navbar, navigation_menu,
   hover_card, tooltip, color_picker, date_picker CSS; `top_layer.rs` offsets only). **Lane M, modal panels** (dialog, alert_dialog,
   sheet, drawer, command CSS and `component.rs`; `assert-backdrop-fade.ts`). F and M share no files and can run together after T.
3. **Lane S, state/control** (input, textarea, native_select, input_group, input_otp, toggle, toggle_group, checkbox, radio,
   switch, field/form and the focus/invalid/disabled rules). It collides with lane F on `select/style.css`, `combobox/style.css`,
   `popover/style.css` and `date_picker/style.css` (trigger and content live in one file): either F owns those four files and S
   takes only the rest, or run S after F.
4. **Lane J, JS lifetimes** (`virtual_list.rs`, `form/component.rs`, `sidebar/component.rs`, new `use_dom_listener`, new
   gate): independent of all CSS lanes.
5. **Lane A, anchor marker** (`top_layer.rs`, the menu/tooltip/hover-card/popover/select/combobox/navbar primitives):
   last and alone, highest risk; do not overlap lane F's `top_layer.rs` edit (give the offset token to lane A instead).
6. **Lane G, gates** (serial, last; shared files `scripts/run-gates.sh`, `CLAUDE.md`, `.stylelintrc.json`): `check-duplicate-keyframes.sh`,
   `check-css-state-recipes.sh`, `check-eval-listeners.sh`, stylelint rules (font-family, touch-action, shadow/colour literals, `z-index`),
   widened `check-css-literals.sh`; plus the Playwright parity specs.

**Top 3 and why.**
(1) **Lane T.** Cheapest, safest, and the only change that touches all 40+ stylesheets without a per-component decision; it also
fixes the two completeness gaps (Command scrim, undefined overlay/indicator tokens) before more code copies the fallback literals.
(2) **Floating-surface recipe (row 1).** Largest visible win (every menu/popover/tooltip) and Nova defines it once; it removes
~32 keyframe blocks and 11 reset comments, and its tokens feed rows 6 and 8.
(3) **State recipes (row 4).** Two instances are real accessibility gaps (Combobox and Textarea have no focus ring, invalid never
reaches most controls), and it is the prerequisite for turning row 111 phase C into one rule (row 5).

---

## 8. Reproducing the counts

```bash
cd /home/user/dioxus-components/preview/src/components
grep -c "touch-action: manipulation" */style.css | grep -v ':0'                 # 33 files, 95 declarations
grep -n "opacity: 0.5" */style.css; grep -n "dx-opacity-disabled" */style.css    # disabled literal vs token
grep -nE "font-family" */style.css | grep -v inherit                             # 5 hard-coded stacks
grep -ln "UA popover\|\[popover\] {" */style.css                                  # 11 reset copies (+ top_layer)
grep -rn "dx-menu-item-indicator-\(size\|inset\|gap\):" . ../../assets            # tokens declared per component
grep -rn "dx-overlay-" ../../assets                                              # zero: tokens undefined in the theme
# keyframes: normalise each @keyframes body (strip comments, whitespace, 0%->from, 100%->to), hash, group.
# JS listener balance per file:
for f in $(git ls-files 'primitives/src/*.rs' 'preview/src/components/*/component.rs'); do
  echo "$f $(grep -c addEventListener $f) $(grep -c removeEventListener $f)"; done | awk '$2!=$3 && $2>0'
```

A small rule extractor (selector + flattened declarations, per stylesheet) lives in the session scratchpad as `rules.py`; it
strips comments, so its line numbers are off. Line numbers in this document come from `grep -n`.
