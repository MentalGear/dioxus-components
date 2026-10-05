# shadcn/ui catalog audit — what we are missing (base flavour)

**Date:** 2026-10-04
**Status:** research only, no repo edits besides this file. Awaiting the owner's go/no-go on the recommendations in section 1.
**shadcn/ui snapshot:** `shadcn-ui/ui` `main` @ **`295a1f114a138f23b5dfee0e0c6812394dfeb90c`** (2026-10-02, "feat(registry): add ten community registries (#12105)"). Confirmed identical to `git ls-remote origin refs/heads/main` at the time of writing.
**Method:** the GitHub API was not reachable (`GitHub access to this repository is not enabled for this session`), so the catalog was read from a sparse, depth-1 anonymous clone at `/home/user/shadcn-ui/ui` (outside this repo; scratch, not part of the workspace). Everything below is cited to a path in that tree at the SHA above. `https://ui.shadcn.com/docs/components/base/message-scroller` and `raw.githubusercontent.com` were reachable too but not needed.

Sources read, all at the SHA above (paths relative to the shadcn repo root):

- Docs index (the sidebar is `meta.json` = `["..."]`, i.e. every file): `apps/v4/content/docs/components/base/*.mdx`
- Registry: `apps/v4/registry/bases/base/ui/*.tsx` (62 component files + `_registry.ts`)
- First-party headless package the new chat components sit on: `packages/react/src/{message-scroller,questionnaire}/`, docs at `apps/v4/content/docs/react/`
- Style layer (the `cn-*` classes): `apps/v4/registry/styles/style-*.css`; CSS utilities: `packages/shadcn/src/tailwind.css`
- Release notes: `apps/v4/content/docs/changelog/2026-06-chat-components.mdx` (2026-06-26), `2026-07-toast.mdx` (2026-07-23), `2026-08-questionnaire.mdx` (2026-08-05)

---

## 1. Bottom line

**Counts (base flavour):** shadcn **64** doc pages / we have **56** of them by name / **8 missing**.

The 8 missing are not 8 equal gaps:

| Class | Items | Count |
|---|---|---|
| Genuinely new, buildable components | message-scroller, message, bubble, marker, attachment, questionnaire | 6 |
| Partial: the primitive exists, only the docs page/`component.json` is absent | direction | 1 |
| Not a component (a prose-styles docs page) | typography | 1 |

All six genuinely new components shipped in **June-August 2026** (chat kit 2026-06-26; questionnaire 2026-08-05), i.e. **after** `component-backlog.md`'s 2026-09-03 scope rule enumerated shadcn's catalog from the older list, so that enumeration is stale rather than the gap being a deliberate skip (see section 6). By the repo's own rule ("this repo tracks shadcn/ui parity") they are in scope.

### Recommendation table (the missing items)

| Item | What it is | Needs new primitive? | Effort | Recommendation |
|---|---|---|---|---|
| **message-scroller** | Chat transcript scroll container: follow-the-stream, anchor new turns near the top, jump-to-latest button, prepend without jumping, jump-to-message | **Yes** (`primitives/src/message_scroller.rs` + one JS controller bridge) | **L** | **ADD NOW** (own lane; stage it, section 4) |
| **message** | Row layout: avatar, header, content, footer, alignment, group | No, styled-only | **S** | **ADD NOW** |
| **bubble** | Message surface: 7 variants, alignment, reactions (absolute, 4 placements), group | No, styled-only | **S–M** | **ADD NOW** |
| **marker** | Inline status / system note / bordered row / labeled separator | No, styled-only | **S** | **ADD NOW** |
| **attachment** | File or image card: media, title/description, 5 states, 3 sizes, 2 orientations, actions, full-card trigger, scroll-snap group | No, styled-only (composes our `Button`) | **M** | **ADD NOW** |
| **scroll-fade + shimmer utilities** | Two CSS utilities the chat kit uses (scroll-aware edge mask; text shimmer) | No, theme CSS | **S** | **ADD NOW** (with the kit, they are prerequisites for visual parity) |
| **questionnaire** | Multi-step question flow (single/multiple/freeform/skippable), native form, progress, shortcuts | Yes (a state machine: ~2.7k LOC headless TS) | **L** | **BACKLOG**: AI-agent-specific, no APG pattern, big; revisit after the chat kit proves out |
| **direction** | `DirectionProvider` docs page | No, `primitives/src/direction.rs` + 11 RTL variants already exist | **S** | **BACKLOG** (docs page only, low value) |
| **typography** | Prose styles (a docs page, no registry file) | No | n/a | **SKIP** (already `N/A` in `component-backlog.md`; not a component) |

**Does it make sense?** Yes for the chat kit, with one caveat. Four of the five new chat components plus the utilities are exactly the cheap styled-only class this repo already landed nine of in one round (`9c81d16`, `dev-docs/backlog.md` row 27), so they are low risk and should go together. `message-scroller` is the only one with real engineering; it is still L, not XL (Carousel's primitive is 7,053 lines; this would be roughly 1,000-1,400 including the embedded JS), because shadcn's own design is **DOM-driven** and keeps the scroll hot path out of the UI framework, which is the shape our `document::eval` bridges already use. It has no APG pattern or Radix reference to calibrate against, so the oracle is tier-3 (port shadcn's own 17 browser tests). The owner should confirm that trade (first JS-heavy controller with no standards oracle) before the lane starts.

Not built, not worth building: virtualization inside the scroller (shadcn deliberately leaves it out too), `Questionnaire` now, blocks (section 7).

---

## 2. Full inventory: shadcn base catalog vs ours

`ours` is the folder under `preview/src/components/` (`-` becomes `_`). "Registry file (lines)" is `apps/v4/registry/bases/base/ui/<name>.tsx`; `doc-only` pages have no registry file (patterns). "Built on" lists the Base UI part or the third-party library (the libraries are all React-ecosystem and irrelevant to a port; shown because the task asked).

| # | shadcn base page | registry file (lines) | built on | ours | status |
|---|---|---|---|---|---|
| 1 | accordion | 88 | Base UI accordion | accordion | present |
| 2 | alert | 71 | styled markup only | alert | present |
| 3 | alert-dialog | 175 | Base UI alert-dialog | alert_dialog | present |
| 4 | aspect-ratio | 22 | styled markup only | aspect_ratio | present |
| 5 | attachment | 209 | composes button | - | **MISSING** |
| 6 | avatar | 109 | Base UI avatar | avatar | present |
| 7 | badge | 47 | styled markup only | badge | present |
| 8 | breadcrumb | 137 | styled markup only | breadcrumb | present |
| 9 | bubble | 120 | styled markup only | - | **MISSING** |
| 10 | button | 50 | Base UI button | button | present |
| 11 | button-group | 87 | composes separator | button_group | present |
| 12 | calendar | 245 | react-day-picker | calendar | present |
| 13 | card | 93 | styled markup only | card | present |
| 14 | carousel | 256 | embla-carousel-react | carousel | present |
| 15 | chart | 369 | recharts | chart | present |
| 16 | checkbox | 34 | Base UI checkbox | checkbox | present |
| 17 | collapsible | 21 | Base UI collapsible | collapsible | present |
| 18 | combobox | 323 | Base UI combobox (+ Button, Input Group) | combobox | present |
| 19 | command | 204 | cmdk | command | present |
| 20 | context-menu | 285 | Base UI context-menu | context_menu | present |
| 21 | data-table | doc-only | TanStack Table (pattern over table) | data_table | present |
| 22 | date-picker | doc-only | pattern: popover + calendar | date_picker | present |
| 23 | dialog | 156 | Base UI dialog | dialog | present |
| 24 | direction | 6 | Base UI direction-provider | - | **partial** (primitive only) |
| 25 | drawer | 227 | Base UI drawer | drawer | present |
| 26 | dropdown-menu | 285 | Base UI menu | dropdown_menu | present |
| 27 | empty | 103 | styled markup only | empty | present |
| 28 | field | 224 | composes label, separator | field | present |
| 29 | hover-card | 50 | Base UI preview-card | hover_card | present |
| 30 | input | 19 | Base UI input | input | present |
| 31 | input-group | 149 | composes button, input, textarea | input_group | present |
| 32 | input-otp | 92 | input-otp | input_otp | present |
| 33 | item | 200 | composes separator | item | present |
| 34 | kbd | 26 | styled markup only | kbd | present |
| 35 | label | 19 | styled markup only | label | present |
| 36 | marker | 63 | styled markup only | - | **MISSING** |
| 37 | menubar | 281 | Base UI menu, menubar | menubar | present |
| 38 | message | 91 | styled markup only | - | **MISSING** |
| 39 | message-scroller | 139 | @shadcn/react (first-party headless) | - | **MISSING** |
| 40 | native-select | 70 | styled markup only | native_select | present |
| 41 | navigation-menu | 173 | Base UI navigation-menu | navigation_menu | present |
| 42 | pagination | 156 | composes button | pagination | present |
| 43 | popover | 89 | Base UI popover | popover | present |
| 44 | progress | 79 | Base UI progress | progress | present |
| 45 | questionnaire | 335 | @shadcn/react (first-party headless) | - | **MISSING** |
| 46 | radio-group | 37 | Base UI radio, radio-group | radio_group | present |
| 47 | resizable | 49 | react-resizable-panels | resizable | present |
| 48 | scroll-area | 54 | Base UI scroll-area | scroll_area | present |
| 49 | select | 220 | Base UI select | select | present |
| 50 | separator | 24 | Base UI separator | separator | present |
| 51 | sheet | 140 | Base UI dialog | sheet | present |
| 52 | sidebar | 729 | composes button, input, separator, sheet, skeleton, tooltip | sidebar | present |
| 53 | skeleton | 13 | styled markup only | skeleton | present |
| 54 | slider | 51 | Base UI slider | slider | present |
| 55 | spinner | 22 | styled markup only | spinner | present |
| 56 | switch | 31 | Base UI switch | switch | present |
| 57 | table | 100 | styled markup only | table | present |
| 58 | tabs | 81 | Base UI tabs | tabs | present |
| 59 | textarea | 17 | styled markup only | textarea | present |
| 60 | toast | 278 | Base UI toast | toast | present |
| 61 | toggle | 43 | Base UI toggle | toggle | present |
| 62 | toggle-group | 89 | Base UI toggle, toggle-group | toggle_group | present |
| 63 | tooltip | 65 | Base UI tooltip | tooltip | present |
| 64 | typography | doc-only | prose styles (docs page) | - | N/A (docs page) |

Notes on the inventory:

- Flavours differ only slightly. `radix/` has 65 pages (adds `sonner`); `aria/` has 63 (drops `menubar` and `navigation-menu`, adds `sonner`). **Base has no `sonner` page**: it ships its own `toast` (Base UI Toast, 278 lines, release note 2026-07-23: types, action, promise, stacking, swipe dismissal). A `sonner.tsx` (85 lines, `sonner` + `next-themes`) still exists in `registry/bases/base/ui/` with no base docs page.
- **`form` is no longer in shadcn's catalog at all** (no `form.mdx` in any flavour; Field replaced it). Our `form` folder is a conformance fixture page (see `scripts/check-preview-composition.sh`), not a catalog component.
- Registry `ui/` has 62 files: the 61 with a base docs page + `sonner`. Docs-only pages: `data-table`, `date-picker`, `typography`.

### Our extras (in our tree, not in the shadcn base catalog)

15 folders. None need action; per the 2026-09-03 scope rule they are "not planned" but already exist (most inherited from upstream `DioxusLabs/components`):

| Ours | Note |
|---|---|
| `area_chart`, `bar_chart`, `line_chart`, `pie_chart`, `radar_chart`, `radial_chart`, `chart_tooltip` | Gallery sub-pages. shadcn covers these as sections inside the single `chart` page, so there is no 1:1 catalog item; counted as extras by name. |
| `color_picker`, `drag_and_drop_list`, `navbar`, `tag_group`, `toolbar`, `virtual_list` | Upstream components with no shadcn counterpart. `navbar` is the menubar-pattern nav; shadcn's `navigation-menu` is the disclosure pattern and is the separate `navigation_menu` we already have. |
| `top_layer` | Conformance/behaviour fixture, not a component (exempt in the composition gate). |
| `form` | Fixture page, see above. |

### Naming differences worth knowing

- shadcn `toast` (base) = our `toast`. shadcn `sonner` (radix/aria) is covered by the same component; `component-backlog.md` already marks Sonner `N/A`.
- shadcn `data-table` / `date-picker` are doc-only patterns; we ship them as folders (`data_table`, `date_picker`), more than shadcn does.
- `direction`: shadcn's registry file is a 6-line re-export of Base UI's `DirectionProvider`; ours is `primitives/src/direction.rs` (`Direction`, `DirectionProvider`, `use_direction`), used by 11 `variants/rtl` demos and allowlisted in `check-preview-composition.sh`. The only missing piece is a user-facing docs page.

---

## 3. Per-item detail (missing components)

Effort key follows `component-backlog.md`: S = one styled-only component (precedent: `empty` 115 rs + 66 css lines; `item` 275 + 177; nine landed in one 58-file commit); M = a Drawer/Resizable-sized primitive (≈ 950-980 primitive lines + 150-560 css + 160-390 line spec); L = Command/Chart-sized; XL = Carousel.

### 3.1 message-scroller — assessed in depth in section 4

Registry 139 lines (styled wrapper) over `@shadcn/react/message-scroller` (2,514 non-test lines + ~4,000 test lines). Docs: `apps/v4/content/docs/components/base/message-scroller.mdx`, API at `docs/react/message-scroller.mdx`.

### 3.2 message

- **What:** `MessageGroup`, `Message` (`align: start|end`), `MessageAvatar`, `MessageContent`, `MessageHeader`, `MessageFooter`. Pure flex layout.
- **Source:** 91 lines. **Deps:** none (only `cn`).
- **Primitive?** No, pure styled composition. Compose with our `Avatar`, `Button`.
- **A11y:** presentational wrapper; the labelling burden is on content (icon-only footer actions need `aria-label`; in-progress state via a `Marker role="status"`). No APG pattern.
- **Effort:** S. **Rec:** ADD NOW.

### 3.3 bubble

- **What:** `BubbleGroup`, `Bubble` (`variant`: default, secondary, muted, tinted, outline, ghost, destructive; `align`), `BubbleContent` (polymorphic, so it can be a real `<button>`/`<a>` via `render`), `BubbleReactions` (absolutely positioned; `side: top|bottom` x `align: start|end`).
- **Source:** 120 lines + the `cn-bubble-*` rules in the style CSS (about 58 matching lines across all chat components in `style-nova.css`, which is 1,748 lines). **Deps:** none (`merge-props`/`use-render` are Base UI helpers, replaced here by our `r#as: Option<Callback<Vec<Attribute>, Element>>` polymorphism idiom, as `collapsible`/`tooltip`/`drawer` do).
- **Primitive?** No. **A11y:** presentational; `BubbleReactions` as a static row needs `role="img"` + `aria-label`; interactive bubbles must be real `button`/`a`; do not convey tone by colour alone. No APG pattern.
- **Effort:** S–M (7 variants and four reaction placements make the CSS the bulk). **Rec:** ADD NOW.

### 3.4 marker

- **What:** `Marker` (`variant`: default, separator, border), `MarkerIcon` (`aria-hidden`), `MarkerContent`. Streaming status ("Thinking..."), tool activity, date separators.
- **Source:** 63 lines. **Deps:** none.
- **Primitive?** No. **A11y (from the docs):** presentational by default; set `role="status"` for streaming/progress markers; a labeled separator must **not** get `role="separator"` (its text would not be announced); interactive markers must be real `a`/`button`. Our attribute-spread already forwards `role`.
- **Effort:** S. **Rec:** ADD NOW (it is the Marker that makes streaming status announce correctly, so the scroller demo wants it).

### 3.5 attachment

- **What:** `Attachment` (`state`: idle, uploading, processing, error, done; `size`: default, sm, xs; `orientation`), `AttachmentMedia` (icon or image), `AttachmentContent/Title/Description`, `AttachmentActions/AttachmentAction`, `AttachmentTrigger` (full-card overlay button behind the actions so both stay separately clickable), `AttachmentGroup` (horizontal scroll-snap row with a scroll-fade mask).
- **Source:** 209 lines. **Deps:** composes our existing `Button` (`ButtonSize::IconXs` and `Ghost` already exist). Uses the `shimmer` and `scroll-fade-x` utilities.
- **Primitive?** No. **A11y (from the docs):** icon-only actions need `aria-label` naming action and target; the trigger needs an `aria-label`; a group of presentational attachments that scrolls needs `tabindex=0` + `role="group"` + `aria-label` so keyboard users can scroll it; error state must keep the reason in text, not colour alone. No APG pattern.
- **Effort:** M (states x sizes x orientation matrix, the stacking-order trigger/actions interplay, scroll-snap group). **Rec:** ADD NOW.

### 3.6 questionnaire

- **What:** a multi-step question flow for agent clarification prompts, onboarding, surveys: single and multiple choice, freeform "other" answer, explicit skip, previous/next/submit, required + custom validation, controlled navigation, resume with defaults, conditional items, optional letter/number answer shortcuts, native `<form>` serialization, server-rendered collection state.
- **Source:** registry 335 lines over `@shadcn/react/questionnaire` (about 2,700 non-test lines: `use-questionnaire-item` 571, `components` 551, `use-questionnaire-root` 525, `types` 305, `collection` 234, `choice` 170, `input` 149, `utils` 141, `context` 59; plus ~4,300 test lines). **Deps:** the headless package only; no third-party library.
- **Primitive?** Yes: a new primitive (navigation state machine over an item collection, validation, shortcut keys, focus management). The fixed choices are **native** `input[type=radio|checkbox]` inside `fieldset`/`legend`; our `RadioGroup`/`Checkbox` primitives render custom widgets with hidden mirrors, so they cannot be reused as-is.
- **A11y:** `fieldset` + `legend` per item, `aria-describedby` for description and active error, `aria-invalid`, a named `progressbar` (current/min/max/text), `aria-keyshortcuts`, inactive items `hidden` + `inert`, focus moves to the new active fieldset on navigation and to the first answer control on a failed validation. There is **no APG pattern** for a multi-step wizard; native form semantics plus WCAG are the reference, so the oracle would be tier-3 (shadcn's own 2,183-line jsdom suite and 1,421-line browser suite are the porting source).
- **Effort:** L. **Rec:** **BACKLOG.** It is AI-agent-flow specific, the largest of the six after the scroller, and needs the chat kit to prove demand first. Cheap to reverse; no dependency from the kit onto it.

### 3.7 direction

- **What:** `DirectionProvider` + `useDirection` (a re-export of Base UI's, 6 lines).
- **Ours:** `primitives/src/direction.rs` already provides it (`Direction`, `DirectionProvider`, `use_direction`, `Direction::resolve_horizontal`), every RTL-aware root takes a `dir` prop, 11 components ship an `rtl` demo variant.
- **Primitive?** No. **A11y:** n/a.
- **Effort:** S (docs page + `component.json`, no markup). **Rec:** BACKLOG. Optional; the user already reaches it as `dioxus_primitives::direction`.

### 3.8 typography

- A docs page of prose classes (`h1`...`blockquote`, lists, inline code), `component: true` in front matter but no registry file. Already `N/A` in `component-backlog.md`. **Rec:** SKIP.

---

## 4. message-scroller in depth

### 4.1 What it actually does (read from `packages/react/src/message-scroller/`)

The docs frame it as 15 rules ("never move the reader against their intent"). The behaviours, with where each lives:

| Behaviour | Mechanism | Source |
|---|---|---|
| **Follow the stream, only while following** | `autoScroll` (default `false`). Mode machine: `following-bottom`, `free-scrolling`, `anchored-to-message`, `settling-jump`. Armed when the viewport is within `scrollEdgeThreshold` (8 px) of the end; **released** by wheel, touchmove, scroll keys (ArrowUp/Down, Home/End, PageUp/PageDown, Space), scrollbar drag (detected as `scrollTop` decreasing while armed), or an explicit jump. Content growth alone never releases it. Re-armed by `scrollToEnd` / the button. | `use-message-scroller-controller.ts` (`reconcileFollowMode`), `components.tsx` (Viewport `onWheel`/`onTouchMove`/`onKeyDown`), `types.ts` (`USER_SCROLL_KEYS`) |
| **"Stick to bottom unless the user scrolled up"** | The above, plus: while following, the published `end` flag is forced `false` so the jump button does not strobe per streamed chunk | `commitScrollState` |
| **Anchor a new turn near the top** | A row marked `scrollAnchor` (user message by default) that is newly appended scrolls to the top, leaving `scrollPreviousItemPeek` (64 px) of the previous row visible. A hidden **tail spacer** (`data-message-scroller-spacer`, `aria-hidden`) inside `Content` is sized so the anchor can reach the top even when the content is short; the streaming reply consumes it, and when it reaches 0 the scroller hands off from the anchor hold to follow-bottom. | `handleContentChange`, `handleResize`, `use-message-scroller-commands.ts` (`setTailSpacerHeight`) |
| **Scroll-to-latest button** | `MessageScrollerButton` is a real `<button>` (`direction: end|start`, `behavior` smooth). When there is nothing to scroll toward it is `inert` + `tabindex=-1` + `data-active="false"` (the styled version slides/fades it out). Click calls `scrollToEnd` and re-engages follow-bottom. | `components.tsx` |
| **Open a saved thread** | `defaultScrollPosition`: `start`, `end` (default), `last-anchor` (last user turn with the reply below it; falls back to `end` if that turn fits). `data-pending-scroll` keeps the viewport `invisible` until applied, to avoid a flash on reload. | `applyDefaultScrollPosition` |
| **Load history without jumping** | `preserveScrollOnPrepend` (default on): records the first visible `messageId` row and its viewport-relative top; after a prepend (MutationObserver `childList`, first item index > 0) corrects `scrollTop` by the delta. A no-op where native CSS scroll anchoring already did it; corrects Safari. | `capturePrependAnchor`, `restorePrependedAnchor` |
| **Jump anywhere** | `scrollToMessage(id, {align: start|center|end|nearest, behavior, scrollMargin})` returns `bool`; queues the target if rows are not yet mounted (client-resolved permalinks), returns `false` for a missing id once mounted. Plus `scrollToStart/End`. | `use-message-scroller-commands.ts` |
| **Where am I** | `useMessageScrollerScrollable()` -> `{start, end}` (also mirrored as `data-scrollable="start end"` and `data-autoscrolling` on root and viewport); `useMessageScrollerVisibility()` -> `{currentAnchorId, visibleMessageIds}` via IntersectionObserver, ref-counted so it costs nothing until a subscriber exists | `stores.ts`, `observeVisibility` |
| **Accessibility** | Viewport `role="region"` `aria-label="Messages"` `tabindex=0` (so keyboard users can scroll it; also satisfies axe `scrollable-region-focusable`); Content `role="log"` `aria-relevant="additions"` (new rows announced, token-level text mutations not), `aria-busy` while streaming defers announcement | `components.tsx`, docs "Accessibility" |
| **Performance** | **Not virtualized.** Real DOM rows with CSS `content-visibility: auto` + `contain-intrinsic-size`; scroll hot path outside React state (rAF-coalesced commits; state mirrored to `data-*` attributes; ResizeObserver on viewport and content; MutationObserver on content `childList`; one `getBoundingClientRect` per row on a commit). Their benchmark (`PERFORMANCE.md`): under 1 ms per scroll commit at 1,000 rows. | `PERFORMANCE.md`, `components.tsx` |
| **Virtualization** | Explicitly **out of scope of the primitive**: the docs show using `MessageScrollerViewport` as the scroll element for `@tanstack/react-virtual`. | docs "Virtualization" |

Public surface: `Provider` (`autoScroll`, `defaultScrollPosition`, `scrollPreviousItemPeek`, `scrollMargin`, `scrollEdgeThreshold`), `Root`, `Viewport` (`preserveScrollOnPrepend`), `Content` (`spacerClassName`), `Item` (`messageId`, `scrollAnchor`), `Button`, and three hooks. Styled registry wrapper is 139 lines (the viewport adds `scroll-fade-b`, `overscroll-contain`, `contain-content`, `data-pending-scroll:invisible`; the button is `absolute`, centred with `inset-s-1/2`, bottom-4).

### 4.2 Do our `virtual_list` / `scroll_area` cover parts of it?

**No, and they should not be stretched to.**

- **`scroll_area`** (`primitives/src/scroll_area.rs`, 161 lines) is a native `overflow` wrapper: axis, scrollbar visibility, `dir`. It exposes no element handle, no scroll state, no commands. It cannot be the Viewport (the controller needs a stable id/ref and the raw scroll events). What transfers is the RTL idea: `use_direction` is already how the button placement should mirror.
- **`virtual_list`** (`primitives/src/virtual_list.rs`, 291 lines + `virtual/` engine) owns its **own** scroll container (`role="list"`, `tabindex=0`), measures rows through Dioxus `onresize`, and corrects `scrollTop` when a measurement changes an estimate (`scroll_adjustments`/`deferred_adjustments`), plus `aria-setsize/posinset`. That is a relative of "keep the reader's place when layout changes", but only for estimated-height virtualization. It lacks follow-bottom, anchoring, prepend preservation by id, jump-to-id, edge state, and the button. Worse, its bridge `dioxus.send`s the scroll offset into Rust on **every scroll event** (`publish(true)`), which is the opposite of the hot-path discipline shadcn's design depends on (their per-scroll commit never touches the framework).
- Because `VirtualList` renders its own container `div`, it cannot be placed inside a `MessageScrollerViewport` today. Making it accept an external scroll element is a separate M-sized follow-up (and its row cache assumes a top-anchored list). shadcn leaves virtualization outside the primitive and says transcripts of hundreds to low thousands of turns do not need it, so **our v1 should be non-virtualized**, matching.
- **What we do reuse:** our `Button` (`ButtonSize::IconSm`, secondary variant exists), `merge_attributes`/`attributes!`, `use_unique_id`, `use_direction`, the `document::eval` bridge idiom (`carousel.rs` `CAROUSEL_*_JS`, `virtual_list.rs`, `drawer.rs`, `input_otp.rs`), the `r#as` polymorphism idiom.

### 4.3 How it ports to Dioxus (design sketch, not a plan)

The decisive observation: shadcn's controller is **DOM-driven**. Rows are discovered through `data-message-id` / `data-scroll-anchor` attributes on the DOM, content change through a `MutationObserver`, size change through `ResizeObserver`, and state is written back as `data-*` attributes. React is only used for rendering the shell, three tiny external stores, and context. That maps one-to-one onto our house construction:

1. **One JS controller module per Provider** (a `const MESSAGE_SCROLLER_JS: &str`, the carousel precedent; a TS file under `primitives/src/ts/` is possible but today only `focus-trap.ts` goes through `lazy_js_bundle` and that one is gated native-only). It owns geometry (`geometry.ts`, 387 lines), mode machine, tail spacer, prepend restore, observers. Roughly 600-800 lines after dropping React plumbing, ported with a provenance header (shadcn/ui is MIT).
2. **Rust renders the shell** (`MessageScrollerProvider`, `MessageScroller`, `...Viewport`, `...Content`, `...Item`, `...Button`) with the same ARIA defaults and data attributes, and exposes `use_message_scroller()` (commands) and `use_message_scroller_scrollable()` / `use_message_scroller_visibility()` (signals fed by eval messages sent **only when the value changes**, never per token or per scroll event).
3. **Streaming needs no per-token bridge traffic:** a signal update re-renders the row's text node, the ResizeObserver sees growth, the controller scrolls. This is exactly why the design survives the Rust to JS boundary.

Risks and constraints specific to this repo:

- **Hydration/SSG** (hydration-parity rule: no effect may change first-render output). `data-pending-scroll` must be present in the SSR HTML when `defaultScrollPosition != start` and removed imperatively by the controller; confirm Dioxus never re-asserts it, and cover with `hydration-parity.spec.ts`. The controller-written `data-scrollable`/`data-autoscrolling` must **not** be in the vdom attribute set (JS-owned, like Drawer's `translate`), or a later render would clobber them; watch `check-attr-spread-collision.sh`.
- **Native/Blitz arm:** `document::eval` is unavailable there, as for Carousel; provide an inert `#[cfg(not(feature = "web"))]` arm (plain overflow, no auto-follow). `check-cfg-axis.sh` enforces the cfg axis.
- **Keyed row churn** (regenerate/branch) produces `childList` records the controller must treat as appends vs prepends vs reorders; shadcn's tests (`prepend`, `bulk appending anchored turns`, `StrictMode remount`) are the cases to port.
- **Oracle is tier-3.** No APG pattern covers a chat transcript; the references are the ARIA `log` role, WCAG keyboard-scroll, and shadcn's own suites: 17 real-Chromium behaviour tests (`message-scroller.browser.test.tsx`), 49 jsdom tests, 17 geometry tests, 6 perf cases. Port the 17 browser cases to Playwright (one real browser, as for every other spec), add axe at rest and with the button visible, RTL, and an SSG hydration check. **Timing/perf assertions only on release builds** (CLAUDE.md: debug SSG gives false timing reds).
- **Gate friction to plan for:** `use_message_scroller*` hooks are non-markup items and need entries in the `allowed_qualified` table of `scripts/check-preview-composition.sh`; `check-css-logical-properties.sh` (the button's `inset-s-1/2` and `rtl:` flip must be written with logical properties); `check-self-subscribing-effects.sh` and `check-hooks-in-closures.sh` for the controller wiring; `check-demo-wrapper-width.sh` (a scroller demo needs a height-constrained wrapper with an explicit `width`).

### 4.4 Suggested staging (if approved)

- **Stage 1 (L):** Provider/Root/Viewport/Content/Item/Button; `autoScroll`; anchoring + tail spacer + peek; `defaultScrollPosition` + `data-pending-scroll`; prepend preservation; `scrollToMessage/End/Start`; `data-scrollable`; accessibility defaults; styled wrapper + demos (basic, anchoring, streaming, load-history, opening position) built on the new Message/Bubble/Marker; ported behaviour spec + axe + hydration.
- **Stage 2 (M):** `use_message_scroller_visibility` (IntersectionObserver, ref-counted), jump menu/table-of-contents demo, `scroll_margin`/`align` polish, RTL pass, release-build timing spec; optionally `VirtualList` hand-off (separate decision).

---

## 5. Things the owner asked about that shadcn does NOT have

Verified by listing `registry/bases/base/ui/` and the docs tree for `prompt-input`, `conversation`, `code-block`, `reasoning`, `suggestion`, `chat-input`: **none exist in shadcn/ui.** Those names are Vercel's *AI Elements*, a separate community registry (`@ai-elements` appears in `apps/v4/registry/directory.json`, which this snapshot's HEAD commit just extended with ten community registries). shadcn's own AI-era set is exactly: **message-scroller, message, bubble, marker, attachment, questionnaire**, plus the `@shadcn/react` headless package, the `scroll-fade`/`shimmer` utilities, and `@shadcn/helpers` (a JS-only scripted mock of the AI SDK for demos, release note 2026-08-12; **SKIP**, nothing to port).

---

## 6. Corrections this audit implies for `component-backlog.md`

(Listed for the owner; not applied, this lane owns only this file.)

1. The 2026-09-03 scope-rule enumeration (59 items, corrected from 58 on 2026-09-26) is stale: shadcn base is now **64**. Delta: **minus** Form (removed from the catalog), **minus** Sonner (no base page; base ships `toast`), **plus** Attachment, Bubble, Direction, Marker, Message, Message Scroller, Questionnaire (59 - 2 + 7 = 64).
2. The tally "56 buildable, 56 present, nothing in the catalog remains unbuilt" is no longer true: against the current catalog, **6 genuinely new components** and 1 partial remain.
3. Typography stays `N/A`; Sonner stays `N/A`; Form is now simply not in the catalog.
4. Depth, not breadth (outside this audit's scope, flagged only): shadcn's base `toast` documents **types, action button, promise, stacking, swipe-dismiss**. Our `primitives/src/toast.rs` has the four types and `ToastOptions { description, duration, permanent }` but no action, promise helper, swipe dismissal or stacking (grep of `toast.rs`/`toast/component.rs` finds none). A parity-depth candidate (S-M), not a catalog gap.

---

## 7. Blocks and non-component items (listed separately, as asked)

**Blocks** (`apps/v4/registry/bases/base/blocks/`, 30 directories; `new-york-v4/blocks/` has the 27 public ones without the three internal previews): `dashboard-01` (1), `login-01..05` (5), `signup-01..05` (5), `sidebar-01..16` (16), plus internal `preview`, `preview-02`, `preview-03`. These are page templates composed from catalog components, not catalog items. We already ship `sidebar` block variants (`floating`, `inset`) and a dashboard email-client app. **Rec: SKIP as catalog work.** If wanted later, `login-*`/`signup-*` are S each as `Field`-based demos and `sidebar-03..16` are variants of the Sidebar we already have; neither adds primitives.

**Utilities** (`apps/v4/content/docs/utils/`, `packages/shadcn/src/tailwind.css`): `scroll-fade` (scroll-aware edge mask via `mask-image` + `animation-timeline: scroll(self y|inline)`, `@supports`-guarded, with a static fallback in `@supports not`) and `shimmer` (text shimmer, `--shimmer-*` custom properties). Neither exists in our CSS today (grep of `preview/assets` and component stylesheets). **Rec: ADD NOW with the kit** (S, as `dx-scroll-fade` / `dx-shimmer` in the theme CSS, MIT with attribution). They are progressive enhancement; the guard means unsupported browsers degrade to no fade, which is safe.

---

## 8. Registration surface: what wiring a new component needs here

Derived from the commits that added the closest precedents, not from memory:

- **Styled-only (the class for message, bubble, marker, attachment):** `9c81d16` "feat(components): the nine styled-only shadcn components (row 27)", 58 files for nine components. One of them, `empty`, touched exactly:
  `preview/src/components/empty/{component.json, component.rs, docs.md, style.css, variants/main/mod.rs}`, `playwright/empty.spec.ts`, plus the shared files `component.json` (root) and `preview/src/components/mod.rs`.
- **Primitive-backed (the class for message_scroller):** `cdb073b` "feat(drawer): add shadcn/ui Drawer primitive and themed component" (2026-09-18), 10 files: `primitives/src/drawer.rs`, `primitives/src/lib.rs` (`pub mod drawer;`), `preview/src/components/drawer/{component.json, component.rs, docs.md, style.css, variants/main/mod.rs}`, `playwright/drawer.spec.ts`, `component.json` (root), `preview/src/components/mod.rs`. The Input OTP commit `61b313c` is the same shape plus `dev-docs/backlog.md` and `dev-docs/component-backlog.md` edits (convention: the lane records itself in both docs); Resizable `0a849f2` additionally added `preview/src/components/resizable/mod.rs` and a tier-1 oracle `playwright/oracle/tier1-apg/window-splitter.spec.ts`.

### The checklist (files per new component `<name>`; `_` in file names, `-` in CSS classes)

Per-component, new files (lane-owned):

1. `preview/src/components/<name>/component.rs`: the themed wrapper. Imports `dioxus_primitives::<name>::*` and attaches the `dx-<name>` class; merges caller attributes with `merge_attributes(vec![base, attributes])`; renders `document::Link { rel: "stylesheet", href: asset!("/src/components/<name>/style.css") }` (see `empty/component.rs`).
2. `preview/src/components/<name>/style.css`: plain `dx-<name>` / `dx-<name>-...` classes on the `--dx-*` tokens (row 32); logical properties; stylelint property order.
3. `preview/src/components/<name>/docs.md`: the docs body, rendered to `docs.html` by `preview/build.rs` (the gallery description comes from `component.json`'s `description`, written to `description.txt` by the same script).
4. `preview/src/components/<name>/variants/main/mod.rs`: `pub fn Demo() -> Element`, rendered as the direct child of `.dx-component-preview-frame` (so `check-demo-wrapper-width.sh` applies). Extra variants go in `variants/<variant>/mod.rs`.
5. `preview/src/components/<name>/component.json`: `name`, `description`, `authors`, `exclude: ["variants","docs.md","component.json"]`, `cargoDependencies` (`dioxus-primitives`, git `https://github.com/DioxusLabs/components`) when it uses a primitive, `globalAssets: ["../../../assets/dx-components-theme.css"]`.
6. `playwright/<name>.spec.ts`: imports `test`/`expect` from `./fixtures` (not `@playwright/test`; gate `check-playwright-fixture-import.sh`), navigates `${BASE_URL}/component/?name=<name>&`, scopes assertions to `#component-preview-frame` (the page also shows highlighted source, so text appears twice), plus `expectNoAxeViolations(page, "<name>: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] })` (see `empty.spec.ts`). Optional tier specs under `playwright/oracle/...` (e.g. a `main-thread.spec.ts` entry, release builds only).
7. If a primitive is needed: `primitives/src/<name>.rs` (module doc with the rule source, unit tests).

Shared files edited (one owner per batch, see "lane rules" below):

8. `primitives/src/lib.rs`: add `pub mod <name>;` (alphabetical).
9. `preview/src/components/mod.rs`: add `<name>` (plus `[variant, ...]` and `(block)` if needed) to the `examples!(...)` list. This single list generates the module tree, `DEMOS`, the gallery, the sidebar, **and the SSG route list** (`server_static_routes()` in `preview/src/main.rs` enumerates `components::DEMOS`; since `ddcc8b8` routes are `/component/:name/` path segments), so **`preview/src/main.rs` needs no edit**, which the Drawer, Input OTP and Resizable commits confirm.
10. `preview/src/components/mod.rs` `category_of()`: a hand-maintained `match`; an unlisted name **silently falls through to `DataDisplay`** (`carousel` is not in any arm today, so it already lands there by accident). Add each new name to the right arm (or add a `Chat` category: that is three more edits in the same file, `enum` + `ALL` + `label`).
11. Root `component.json`: add `"preview/src/components/<name>"` to `members` (verified: 70 members vs 71 folders, the only exception is `top_layer`).
12. `scripts/check-preview-composition.sh`: only if the new primitive exports non-markup items (hooks, plain enums) that the preview consumes directly (e.g. `use_message_scroller*`); add to `allowed_qualified`. Markup types must be reached through `crate::components::*`.
13. `dev-docs/component-backlog.md` and `dev-docs/backlog.md`: the lane's record (convention from `61b313c`).

Not part of the surface (confirmed by grep): `docs/` at the repo root is generated Pages output; `playwright/dx-class-migration.spec.ts`'s `MIGRATED` list does not include recent components (drawer, command, carousel are absent), so it is not required; `preview/src/main.rs` is not edited.

### Gotchas that have already bitten

- **`preview/build.rs` does not rerun when only `components/mod.rs` changes** (`dev-docs/backlog.md` row 92): after adding a component to a *warm* target dir, `cargo clippy/test` and `dx` fail with `couldn't read .../out/<name>/description.txt` until `touch preview/build.rs`. A lane building in a fresh target dir never sees it.
- Build only through `scripts/build-ssg.sh [debug|release]` with an **absolute, isolated** `CARGO_TARGET_DIR` (`scripts/lane-target.sh create <name>`), never the shared `target/` (CLAUDE.md, rows 98/100/116). Playwright `node_modules`: `npm ci` or symlink, never `npm install`.
- Classes must be `dx-<folder-with-dashes>` or `check-dx-class-prefix.sh` fails; component paths must not go through `components::<name>::component::` (`check-installed-paths.sh`; a sibling is `crate::components::<name>::*`, a variant reaches its own component as `super::super::component::*`).

### Gates to run before the commit (CLAUDE.md list; all workflows are `workflow_dispatch`-only)

`scripts/check-preview-composition.sh`, `check-cfg-axis.sh`, `check-dx-class-prefix.sh`, `check-css-literals.sh`, `check-hooks-in-closures.sh`, `check-self-subscribing-effects.sh`, `check-css-logical-properties.sh`, `check-attr-spread-collision.sh`, `check-demo-wrapper-width.sh`, `check-playwright-fixture-import.sh`, `check-raw-text-interpolation.sh`, `check-internal-hrefs.sh`, `check-installed-paths.sh`, `check-css-vars-defined.sh`, then `cargo fmt --all -- --check`, `cargo clippy --workspace --tests --examples -- -D warnings`, `cargo test --workspace`, `cd preview && npx stylelint "src/**/*.css"`.

### Proposed lanes (disjoint file sets, per CLAUDE.md)

Batch 1, dispatched together:

| Lane | Owns | Must not touch |
|---|---|---|
| A: styled chat kit | `preview/src/components/{message,bubble,marker,attachment}/**`, their `playwright/<name>.spec.ts`, the theme CSS block for `dx-scroll-fade` / `dx-shimmer` in `preview/assets/dx-components-theme.css` | shared registration files (below), `message_scroller/**`, `primitives/**` |
| B: message-scroller stage 1 | `primitives/src/message_scroller.rs`, `preview/src/components/message_scroller/**`, `playwright/message_scroller.spec.ts` | shared registration files, lane A's folders |

Lane B's demo wants Message/Bubble/Marker; either it builds the first demo from plain elements and swaps in lane A's components after batch 1 merges, or B's demo variants land in a second batch (a real dependency on lane A's output, named so rather than hidden).

Shared registration files, edited once by the main loop after the lanes return (a real single-holder constraint: two lanes would edit the same lines): `primitives/src/lib.rs`, `preview/src/components/mod.rs` (`examples!` + `category_of`), root `component.json`, `scripts/check-preview-composition.sh` (allowlist), `dev-docs/component-backlog.md`, `dev-docs/backlog.md`. The cargo lock and the dev-server port are the other single-holder resources: parallelize editing, serialize build/test at the end.

Baseline to capture before the batch (it cannot be recovered afterwards): the current Playwright pass/fail counts and the red/green state of the gate list on `main` (44a103e at the time of writing; the working tree already carries uncommitted edits from other lanes in `playwright/*.spec.ts` and `preview/assets/dx-components-theme.css`, which lane A's theme-CSS edit would collide with, so sequence that one edit after those land).

---

## 9. Open questions for the owner

1. Approve the chat kit (message, bubble, marker, attachment + the two utilities) as one styled-only round? (Low risk, S–M, precedent row 27.)
2. Approve `message_scroller` as its own L lane with a tier-3 oracle (no APG), staged as in 4.4? Alternatively hold it until the kit lands so its demos use the real components.
3. Questionnaire stays BACKLOG unless you want AI-agent-flow parity; say so and it becomes the next L lane after the scroller.
4. Do you want `component-backlog.md`'s catalog count and "nothing unbuilt" claim corrected as in section 6 (a docs-only edit)?
5. Toast depth parity (action/promise/stack/swipe) is not a catalog gap; do you want it queued separately?
