/**
 * ORACLE: tier 2 (HTML) — top layer (`popover` attribute).
 *
 * Source: docs/plan.md Phase 4.4, drawing on the WHATWG HTML Living
 * Standard's popover chapter:
 *   - The `popover` attribute, its `auto`/`manual` states, and the implied
 *     top-layer promotion:
 *     https://html.spec.whatwg.org/multipage/popover.html#the-popover-attribute
 *   - Light dismiss (Escape, outside pointerdown, closing sibling `auto`
 *     popovers) for `popover="auto"`:
 *     https://html.spec.whatwg.org/multipage/popover.html#popover-light-dismiss
 *   - `showPopover()`/`hidePopover()` and the `toggle` event fired on every
 *     state change (browser- or script-driven alike):
 *     https://html.spec.whatwg.org/multipage/popover.html#dom-showpopover
 *     https://html.spec.whatwg.org/multipage/popover.html#dom-hidepopover
 *   - The top layer itself (a rendering surface above the document that
 *     escapes ancestor overflow/transform/stacking contexts):
 *     https://html.spec.whatwg.org/multipage/rendering.html#the-top-layer
 *
 * Fixture: preview/src/components/top_layer/component.rs (`TopLayerFixture`),
 * served at /component/?name=top_layer. Every library control there is
 * paired with a native reference control — a plain `<div popover="auto">`
 * shown by a `<button popovertarget>` — the browser's own implementation of
 * the identical WHATWG HTML feature, with zero Dioxus involvement.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): every rule below
 * runs against the native reference first (prefixed CALIBRATION: in the
 * test name) and the library component second. A CALIBRATION failure means
 * the *test* is wrong, not the component.
 *
 * Per-component dismissal semantics (docs/plan.md Phase 4.4's own
 * requirement to "document the choice per component"):
 *   - `Tooltip`/`HoverCard` render with `popover="manual"` on the web arm.
 *     Both already own their entire open/close lifecycle through
 *     hover/focus (and, for Tooltip, an explicit Escape handler on the
 *     trigger) — MDN's own naming ("hover card") does not imply the light
 *     dismiss a click-triggered popover has, and `auto`'s outside-pointerdown
 *     dismissal would race that existing lifecycle rather than usefully
 *     extend it. So rules 2 ("light dismiss") and 3 ("Escape") below apply
 *     only to the non-modal `Popover` arm (`popover="auto"`) and its native
 *     reference — see primitives/src/top_layer.rs's `PopoverKind` doc for
 *     the full reasoning.
 *   - The non-modal `Popover` arm renders as `<dialog popover="auto">`,
 *     getting light dismiss for free from the platform; this file's rules
 *     2-3 are exactly what checks that the WHATWG-mandated browser behaviour
 *     (`toggle` event) still lands back in the Rust `open` signal, per
 *     primitives/src/top_layer.rs's `use_popover_sync` (the same defect
 *     class documented for `<dialog>`'s old one-way binding in
 *     docs/recommended-implementations.md Caveat 1).
 *   - Rule 1 ("clipping escape") and rule 4 ("stacking") are top-layer
 *     properties every promoted element gets regardless of its
 *     `popover="auto"|"manual"` kind, so all three components (Tooltip,
 *     HoverCard, Popover) are checked for rule 1; rule 4 uses `Popover`
 *     (the click-triggered case is the easiest to drive deterministically).
 *   - Migration A slice 2/3 (`ContextMenu`/`Menubar`, `docs/plan.md` Phase
 *     5.2's point-anchor case is explicitly out of scope for this file):
 *     `ContextMenu` also renders `popover="manual"` on the web arm, for a
 *     reason distinct from Tooltip/HoverCard's: it opens at a raw click
 *     point with no persistent trigger to key an outside-vs-internal
 *     dismissal distinction off of the way `auto`'s native light dismiss
 *     would need, so it keeps its own pre-existing `use_outside_dismiss` +
 *     root `Escape` handler entirely unchanged (see
 *     `primitives/src/context_menu.rs`'s `ContextMenuContentRendered` doc
 *     for the full reasoning and the focus-restore risk this avoids).
 *     `Menubar` menus render `popover="auto"`, anchored to their own
 *     trigger like `DropdownMenu` -- Rule 1 (clipping) and the new Rule 9
 *     (scroll behavior, below) cover both; rules 2/3 (light dismiss/Escape)
 *     stay scoped to `Popover` as documented above, since `ContextMenu`'s
 *     `manual` mode never light-dismisses and `Menubar`'s Escape handling
 *     is APG-specific (per-trigger refocus) rather than a generic "does the
 *     Rust signal learn about a native close" check those two rules make.
 *   - Migration A slice 3/3 (final): `Select`'s listbox renders
 *     `popover="auto"` on the web arm (`select/components/list.rs`'s
 *     `SelectListRendered`) -- its own pre-existing, more precise
 *     blur-driven dismissal/focus-restore logic stays the primary
 *     mechanism, with native light dismiss layered on as a backstop; see
 *     that component's own doc for the full reasoning. `Combobox`'s listbox
 *     renders `popover="manual"` instead -- an initial `auto` attempt was
 *     reversed by execution: it broke `combobox.spec.ts`'s "dynamic option
 *     removal" case, where an external control button (with its own
 *     `prevent_default()`-guarded `pointerdown`, specifically so it does
 *     not close the combobox) got light-dismissed anyway, since WHATWG
 *     light dismiss classifies any outside pointerdown as "outside" full
 *     stop, `prevent_default()` or not -- see
 *     `combobox/components/list.rs`'s `ComboboxListRendered` doc for the
 *     complete account. Both are exercised only by Rule 1 (clipping) below,
 *     not rules 2/3, for the same reason `ContextMenu`/`Menubar` aren't:
 *     each keeps its own more precise dismissal path as the actual
 *     mechanism, with `auto` (where used) only ever a backstop. `Toast`'s
 *     region renders `popover="manual"` (`toast.rs`'s `ToastRegionRendered`)
 *     -- it has no open/close lifecycle and nothing to anchor to, so only
 *     the stacking property (a new Rule 10, below) applies to it; Rule 1
 *     does not, since a toast is not clipped by an ancestor the way a
 *     trigger-anchored popup is (it is viewport-region positioned, not
 *     positioned relative to any particular trigger element).
 *
 * Rules implemented (see this session's report for the full red/green
 * ledger per rule per component, both before and after the Phase 4.4
 * implementation landed):
 *   1. Clipping escape — component content inside an ancestor with
 *      `overflow: hidden` + `transform` + a height shorter than the
 *      content renders fully visible outside the clip.
 *   2. Light dismiss for `popover="auto"` — click outside closes it AND
 *      Rust state (`data-state` on the `PopoverRoot`) syncs.
 *   3. Escape closes `popover="auto"` AND Rust state syncs.
 *   4. Top-layer stacking — a popover opened after a high-z-index sibling
 *      renders above it.
 *   5. Near-viewport-edge flip — see "Rules 5-7" below.
 *
 * ## Rules 5-7 — CSS Anchor Positioning flip (docs/backlog.md row 10)
 *
 * Unlike every rule above, this trio's citation is the **W3C CSS Anchor
 * Positioning** spec, not WHATWG HTML — this tier's "standard" principle
 * (docs/conformance-harness.md) is satisfied by that spec instead:
 *   - `position-try-fallbacks` and the `flip-block`/`flip-inline` "existing
 *     tactics":
 *     <https://www.w3.org/TR/css-anchor-position-1/#fallback-var>
 *     <https://www.w3.org/TR/css-anchor-position-1/#accepted-existing-tactics>
 *
 * It lives in this file anyway (rather than a new spec file) because it
 * exercises the exact same fixture family (anchor-positioned `popover`
 * content) and the exact same per-component dismissal split documented
 * above — a sibling file would just re-import all of that context.
 *
 * Fixture: the "Near-viewport-edge flip" section of `TopLayerFixture`
 * (`preview/src/components/top_layer/component.rs`) — a bottom-edge row and
 * a right-edge column, each pinned by `position: fixed` a few pixels from
 * that viewport edge (deterministic regardless of page height/scroll, same
 * reasoning as the existing stacking-sibling fixture). Every trigger in a
 * row/column requests the side pointing *off* that viewport edge
 * (`side="bottom"` a few px from the bottom; `side="right"` a few px from
 * the right), so the preferred placement cannot fit and only a flip lands
 * it on-screen. Each row/column also carries a native `<div popover>` +
 * `position-try-fallbacks` reference (CALIBRATION) — the browser's own
 * implementation of the identical spec feature, no Dioxus code involved.
 *
 * Rule 5 (block-axis flip): a bottom-edge trigger with `side="bottom"`
 * renders its content fully inside the viewport, above the trigger.
 * Rule 6 (inline-axis flip): a right-edge trigger with `side="right"`
 * renders its content fully inside the viewport, to the left of the
 * trigger. All three anchor-positioned components (Tooltip, HoverCard,
 * non-modal Popover) support every `ContentSide` value, so the inline axis
 * applies to all three — no side is skipped here.
 * Rule 7 (no spurious flip): a normal, mid-viewport placement (the existing
 * "Stacking popover" from the section above, `side="bottom"` far from any
 * edge) renders unflipped, below its trigger — this is the regression guard
 * for `primitives/src/top_layer.rs`'s `use_anchor_position_fallback`: since
 * that hook's JS fallback now accepts *either* the primary or the flipped
 * placement as contract-legal (see its doc), this rule confirms it isn't
 * accepting a flip that never should have happened in the common case.
 */

import { test, expect, type Page } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const gotoFixture = (page: Page) =>
  page.goto("http://127.0.0.1:8080/component/?name=top_layer&", {
    timeout: NAV_TIMEOUT,
    waitUntil: "networkidle",
  });

/**
 * The real discriminating test for "does this content escape the clip,"
 * not just "is its geometric box taller than the clip." A normal (non-top-
 * layer) DOM child's `getBoundingClientRect()` reports its full laid-out
 * size regardless of an ancestor's `overflow: hidden` -- clipping only
 * affects *paint*, not layout geometry -- so a box-size comparison alone
 * cannot tell a genuinely-escaped element from one that is simply present
 * but invisible past the clip. `document.elementFromPoint()` at a
 * coordinate inside the content's own box but outside the clip ancestor's
 * box resolves to whatever is actually *painted* there: the content itself
 * if it escaped (top layer), or whatever sits behind/around the clip box
 * if it did not (the pre-Phase-4.4 behaviour this rule was written to
 * catch).
 *
 * Candidate probe points are the content box's four corners (inset 2px)
 * plus its center, not just "just past the ancestor's edge": a top-layer
 * element's landing spot varies by engine/positioning support (an
 * anchor-positioned popover lands right next to its trigger, which sits
 * inside the clip ancestor; the native `<div popover>` reference here has
 * no anchor-positioning applied at all, so the UA default
 * `position: fixed; inset: 0; margin: auto` centers it in the viewport --
 * nowhere near the clip ancestor). Whichever candidate point falls outside
 * the ancestor's box *and inside the actual browser viewport* is used; if
 * content and ancestor don't overlap at all, every on-screen candidate
 * qualifies.
 *
 * The viewport bound matters, confirmed by execution: `element.getBounding
 * ClientRect()` reports a box's full geometry regardless of whether any of
 * it is actually on-screen, but `document.elementFromPoint()` is a
 * viewport-relative API that returns `null` for any coordinate outside
 * `[0, innerWidth) x [0, innerHeight)` -- always, regardless of what
 * top-layer content would otherwise paint there. `use_anchor_position_
 * fallback` (primitives/src/top_layer.rs) can legitimately compute a
 * static (non-collision-aware; that's Phase 5's remaining scope) position
 * that runs partly off-screen for this fixture's unstyled, unconstrained-
 * width tooltip content -- a real, on-screen escape from the clip ancestor
 * either way, just not always at the *first* candidate in box-corner
 * order. Without this bound, a build could flip between candidate corners
 * that are on- vs off-screen for reasons unrelated to top-layer escape
 * (timing/measurement noise in exactly where the unconstrained-width
 * content lands), intermittently reporting a real escape as a false
 * negative merely because `elementFromPoint` was asked about a pixel that
 * does not exist.
 */
async function escapesClip(
  page: Page,
  contentSelector: string,
  ancestorSelector: string,
): Promise<{ escapes: boolean; reason?: string; x?: number; y?: number; hit?: string | null }> {
  return page.evaluate(
    ({ contentSelector, ancestorSelector }) => {
      const content = document.querySelector(contentSelector) as HTMLElement | null;
      const ancestor = document.querySelector(ancestorSelector) as HTMLElement | null;
      if (!content || !ancestor) {
        return { escapes: false, reason: `missing element(s): content=${!!content} ancestor=${!!ancestor}` };
      }
      const c = content.getBoundingClientRect();
      const a = ancestor.getBoundingClientRect();
      const inset = 2;
      const candidates: [number, number][] = [
        [c.left + inset, c.top + inset],
        [c.right - inset, c.top + inset],
        [c.left + inset, c.bottom - inset],
        [c.right - inset, c.bottom - inset],
        [c.left + c.width / 2, c.top + c.height / 2],
      ];
      const outsideAncestor = (x: number, y: number) =>
        x < a.left || x > a.right || y < a.top || y > a.bottom;
      const onScreen = (x: number, y: number) =>
        x >= 0 && x < window.innerWidth && y >= 0 && y < window.innerHeight;
      const probe = candidates.find(([x, y]) => outsideAncestor(x, y) && onScreen(x, y));
      if (!probe) {
        return {
          escapes: false,
          reason: `no on-screen candidate in content's box (${JSON.stringify(c)}) falls outside the ancestor's box (${JSON.stringify(a)}) within the viewport (${window.innerWidth}x${window.innerHeight}) -- cannot distinguish escape from containment this way`,
        };
      }
      const [x, y] = probe;
      const hit = document.elementFromPoint(x, y);
      const escapes = hit === content || content.contains(hit);
      return {
        escapes,
        x,
        y,
        hit: hit ? `${hit.tagName}#${hit.id || "(no id)"}` : null,
      };
    },
    { contentSelector, ancestorSelector },
  );
}

test.describe("Rule 1 — clipping escape (an ancestor with overflow:hidden + transform + a height shorter than the content must not clip it)", () => {
  test("CALIBRATION: native <div popover=auto> escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-native-trigger").click();
    await expect(page.locator("#clip-native-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-native-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("Tooltip content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-tooltip-trigger").hover();
    await expect(page.locator("#clip-tooltip-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-tooltip-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("HoverCard content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-hovercard-trigger").hover();
    await expect(page.locator("#clip-hovercard-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-hovercard-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  test("Popover (non-modal) content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-popover-trigger").click();
    await expect(page.locator("#clip-popover-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-popover-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  // docs/backlog.md item 2: DropdownMenu's web arm migrated to
  // `popover="auto"` (`DropdownMenuContentRendered`, `dropdown_menu.rs`).
  // Written RED first against the pre-migration plain-div `DropdownMenuContent`
  // (confirmed by execution: it clipped at the 60px ancestor exactly like
  // Tooltip/HoverCard/Popover did before Phase 4.4).
  test("DropdownMenu content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-dropdown-menu-trigger").click();
    await expect(page.locator("#clip-dropdown-menu-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-dropdown-menu-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  // Migration A slice 2/3: ContextMenu's web arm migrated to
  // `popover="manual"` (`ContextMenuContentRendered`, `context_menu.rs`).
  // Written RED first against the pre-migration plain `position: fixed`
  // div (confirmed by execution: it clipped at the 60px ancestor exactly
  // like the others did pre-migration -- a `transform`-ed ancestor becomes
  // the containing block for a `position: fixed` descendant, per ordinary
  // CSS, regardless of the popover migration).
  test("ContextMenu content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-context-menu-trigger").click({ button: "right" });
    await expect(page.locator("#clip-context-menu-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-context-menu-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  // Migration A slice 2/3: Menubar menus' web arm migrated to
  // `popover="auto"`, anchored to their own trigger
  // (`MenubarContentRendered`, `menubar.rs`). Written RED first against the
  // pre-migration plain `position: absolute` div (confirmed by execution:
  // it clipped at the 60px ancestor exactly like DropdownMenu's identical
  // pre-migration shape did).
  test("Menubar content escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-menubar-trigger").click();
    await expect(page.locator("#clip-menubar-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-menubar-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  // Migration A slice 3/3 (final): Select's web arm migrated to
  // `popover="auto"` (`SelectListRendered`, `select/components/list.rs`).
  // Written RED first against the pre-migration plain `div` (confirmed by
  // execution: it clipped at the 60px ancestor exactly like every other
  // pre-migration listbox/menu here did). Not portal'd -- `SelectList`
  // renders in place in the tree, so this was a real gap, not a case that
  // turned out to already escape.
  test("Select listbox escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-select-trigger").click();
    await expect(page.locator("#clip-select-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-select-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });

  // Migration A slice 3/3 (final): Combobox's web arm migrated to
  // `popover="manual"` (`ComboboxListRendered`, `combobox/components/list.rs`
  // -- see that component's doc for why `manual`, not `auto`). Written RED
  // first against the pre-migration plain `div` (confirmed by execution: it
  // clipped at the 60px ancestor exactly like the others did pre-migration).
  // Opened via keyboard (ArrowDown) rather than a click, since a click on
  // the input alone does not open this popup.
  test("Combobox listbox escapes the clip", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#clip-combobox-trigger").click();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("#clip-combobox-content")).toBeVisible();
    const result = await escapesClip(page, "#clip-combobox-content", "#clip-box");
    expect(result.escapes, JSON.stringify(result)).toBe(true);
  });
});

/**
 * Rule 10 — Toast top-layer stacking (Migration A slice 3/3, final).
 * `ToastProvider`'s region has no open/close lifecycle and never anchors to
 * a trigger (see `primitives/src/toast.rs`'s `ToastRegionRendered` doc for
 * why it renders `popover="manual"` rather than `auto`) -- the property
 * that migration buys it is exactly Rule 4's: painting above a competing
 * high-`z-index` sibling regardless of DOM/stacking-context position. The
 * fixture pins a small (40x20px) high-`z-index` sibling and the toast
 * region to the same `position: fixed` bottom-right viewport corner.
 *
 * No "before any toast exists" calibration half here, unlike every other
 * rule in this file -- confirmed by execution that the premise doesn't
 * hold for this fixture: the region itself is mounted (and, being
 * `popover="manual"`, already `:popover-open`) from page load, before any
 * toast is ever added, and even with zero toasts inside it the UA popover
 * stylesheet's own `border`/`padding` defaults (this raw, unstyled
 * `dioxus_primitives::toast::ToastProvider` carries none of the real
 * component's own border/padding reset -- see
 * `preview/src/components/toast/style.css`'s `.dx-toast-container[popover]`,
 * which only applies to the *styled* wrapper) already give the empty
 * region a real, nonzero-area box at that exact corner -- so the sibling
 * is already covered before the click, not just after. That is itself
 * further (if incidental) evidence the migration works -- the top layer
 * promotion holds for the region continuously, not only while it holds a
 * toast -- but it means the meaningful check is simply "the sibling is not
 * what paints at that point once a toast exists," verified directly below
 * instead of via a before/after diff.
 */
test.describe("Rule 10 — Toast top-layer stacking (a toast renders above a maximal-z-index sibling)", () => {
  test("Toast renders above the high-z-index sibling", async ({ page }) => {
    await gotoFixture(page);
    // The sibling (and the toast region itself) are `position: fixed` to
    // the bottom-right viewport corner -- always on-screen regardless of
    // scroll, so no `scrollIntoViewIfNeeded` is needed for either of them
    // (see `preview/src/components/top_layer/component.rs`'s comment on
    // this section for why `fixed`, not `absolute`, is load-bearing here).
    // The *trigger button* is a normal in-flow element well down this
    // fixture page, though, and does need scrolling into view to be
    // clickable below.
    await page.locator("#toast-stack-trigger").scrollIntoViewIfNeeded();

    await page.locator("#toast-stack-trigger").click();
    await expect(page.getByRole("alertdialog")).toBeVisible();

    const hit = await page.evaluate(() => {
      const sibling = document.getElementById("toast-stack-sibling")!;
      const r = sibling.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      if (!el) return { tag: null, insideRegion: false };
      return { tag: `${el.tagName}#${el.id || "(no id)"}`, insideRegion: !!el.closest('[role="region"]') };
    });
    expect(hit.tag, "expected the toast region to paint above the high-z-index sibling").not.toBe(
      "DIV#toast-stack-sibling",
    );
    expect(hit.insideRegion, `expected the hit (${hit.tag}) to land inside the toast region`).toBe(true);
  });
});

test.describe("Rule 2 — light dismiss for popover=auto (click outside closes it, and Rust state syncs)", () => {
  test("CALIBRATION: native <div popover=auto> closes on outside click", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();
    await page.locator("#outside-click-target").click();
    await expect(page.locator("#stack-native-content")).toBeHidden();
  });

  test("Popover (non-modal): click outside closes it and syncs data-state", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "open");

    await page.locator("#outside-click-target").click();

    // The browser's own light dismiss hides the element; the Rust signal
    // must independently learn about it (primitives/src/top_layer.rs
    // `use_popover_sync`'s `toggle` listener) -- assert both, since a
    // stranded signal (the exact defect class recommended-implementations.md
    // Caveat 1 documents for <dialog>) would leave data-state="open" even
    // though the content is gone.
    await expect(page.locator("#stack-popover-content")).toBeHidden();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Rule 3 — Escape closes popover=auto, and Rust state syncs", () => {
  test("CALIBRATION: native <div popover=auto> closes on Escape", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.locator("#stack-native-content")).toBeHidden();
  });

  test("Popover (non-modal): Escape closes it and syncs data-state", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "open");

    await page.keyboard.press("Escape");

    await expect(page.locator("#stack-popover-content")).toBeHidden();
    await expect(page.locator("#stack-popover-root")).toHaveAttribute("data-state", "closed");
  });
});

test.describe("Rule 4 — top-layer stacking (a popover opened after a high-z-index sibling renders above it)", () => {
  // Scroll the section into view *before* opening (item 2, 2026-09-01: the
  // stacking section went back to in-flow layout -- see
  // `preview/src/components/top_layer/style.css`'s comment on
  // `.dx-top-layer-stack-sibling` -- so it can render below the fold again),
  // not after: opening first and scrolling afterward is exactly what Rule 8
  // below exercises deliberately, and doing it here too would make this
  // rule's assertion race the fallback's rAF-throttled re-measure for no
  // reason this rule needs to take on -- confirmed by execution, this
  // ordering (scroll, then open) is deterministic where the reverse briefly
  // isn't. `#stack-sibling` is used (not the trigger) because its box is
  // taller than the trigger row's, so scrolling it fully into view also
  // covers the row.
  test("CALIBRATION: native <div popover=auto> renders above the high-z-index sibling", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    await page.locator("#stack-native-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-native-content")).toBeVisible();

    const hit = await page.evaluate(() => {
      const sibling = document.getElementById("stack-sibling")!;
      const r = sibling.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return el ? `${el.tagName}#${el.id || "(no id)"}` : null;
    });
    expect(hit).toBe("DIV#stack-native-content");
  });

  test("Popover (non-modal) renders above the high-z-index sibling", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();

    const hit = await page.evaluate(() => {
      const sibling = document.getElementById("stack-sibling")!;
      const r = sibling.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return el ? `${el.tagName}#${el.id || "(no id)"}` : null;
    });
    expect(hit).toBe("DIALOG#stack-popover-content");
  });
});

/**
 * Rects and viewport size for the flip rules below. `getBoundingClientRect`
 * is the same viewport-relative measurement `escapesClip` above already
 * relies on for the same reason: layout-box geometry, unaffected by paint
 * effects, and directly comparable to `window.inner{Width,Height}`.
 */
async function rectOf(page: Page, selector: string) {
  return page.locator(selector).evaluate((el) => {
    const r = (el as HTMLElement).getBoundingClientRect();
    return { top: r.top, left: r.left, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  });
}

async function viewportSize(page: Page) {
  return page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
}

// A 1px tolerance throughout the flip rules below for the same
// rounding/subpixel reason `use_anchor_position_fallback`'s own >2px
// tolerance exists (primitives/src/top_layer.rs) -- comparisons here are
// looser (1px) since they only need "fully inside the viewport"/"on the
// correct side of the trigger," not exact pixel equality to a formula.
const EDGE_TOLERANCE = 1;

test.describe("Rule 5 — block-axis flip: a bottom-edge trigger with side=\"bottom\" renders its content fully inside the viewport, above the trigger (W3C CSS Anchor Positioning, position-try-fallbacks: flip-block, https://www.w3.org/TR/css-anchor-position-1/#fallback-var)", () => {
  async function assertFlippedAbove(page: Page, triggerSelector: string, contentSelector: string) {
    const trigger = await rectOf(page, triggerSelector);
    const content = await rectOf(page, contentSelector);
    const viewport = await viewportSize(page);
    const debug = JSON.stringify({ trigger, content, viewport });
    // Fully inside the viewport -- not clipped or partially off the bottom
    // edge, which is what the *preferred* (unflipped) placement would do
    // here (the trigger sits only a few px from the bottom edge).
    expect(content.top, debug).toBeGreaterThanOrEqual(-EDGE_TOLERANCE);
    expect(content.bottom, debug).toBeLessThanOrEqual(viewport.height + EDGE_TOLERANCE);
    // And specifically *above* the trigger, not merely inside the
    // viewport by some other accident (e.g. shift/clamp) -- a real flip.
    expect(content.bottom, debug).toBeLessThanOrEqual(trigger.top + EDGE_TOLERANCE);
  }

  test("CALIBRATION: native <div popover> + position-try-fallbacks: flip-block flips above", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-bottom-native-trigger").click();
    await expect(page.locator("#edge-bottom-native-content")).toBeVisible();
    await assertFlippedAbove(page, "#edge-bottom-native-trigger", "#edge-bottom-native-content");
  });

  test("Tooltip content flips above its bottom-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-bottom-tooltip-trigger").hover();
    await expect(page.locator("#edge-bottom-tooltip-content")).toBeVisible();
    await assertFlippedAbove(page, "#edge-bottom-tooltip-trigger", "#edge-bottom-tooltip-content");
  });

  test("HoverCard content flips above its bottom-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-bottom-hovercard-trigger").hover();
    await expect(page.locator("#edge-bottom-hovercard-content")).toBeVisible();
    await assertFlippedAbove(page, "#edge-bottom-hovercard-trigger", "#edge-bottom-hovercard-content");
  });

  test("Popover (non-modal) content flips above its bottom-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-bottom-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#edge-bottom-popover-content")).toBeVisible();
    await assertFlippedAbove(page, "#edge-bottom-popover-trigger", "#edge-bottom-popover-content");
  });
});

test.describe("Rule 6 — inline-axis flip: a right-edge trigger with side=\"right\" renders its content fully inside the viewport, to the left of the trigger (W3C CSS Anchor Positioning, position-try-fallbacks: flip-inline, https://www.w3.org/TR/css-anchor-position-1/#fallback-var)", () => {
  async function assertFlippedLeft(page: Page, triggerSelector: string, contentSelector: string) {
    const trigger = await rectOf(page, triggerSelector);
    const content = await rectOf(page, contentSelector);
    const viewport = await viewportSize(page);
    const debug = JSON.stringify({ trigger, content, viewport });
    expect(content.left, debug).toBeGreaterThanOrEqual(-EDGE_TOLERANCE);
    expect(content.right, debug).toBeLessThanOrEqual(viewport.width + EDGE_TOLERANCE);
    expect(content.right, debug).toBeLessThanOrEqual(trigger.left + EDGE_TOLERANCE);
  }

  // All three anchor-positioned components (Tooltip, HoverCard, non-modal
  // Popover) accept every `ContentSide` value, so the inline axis applies
  // to all three -- no "block-axis only" narrowing needed here.
  test("CALIBRATION: native <div popover> + position-try-fallbacks: flip-inline flips left", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-right-native-trigger").click();
    await expect(page.locator("#edge-right-native-content")).toBeVisible();
    await assertFlippedLeft(page, "#edge-right-native-trigger", "#edge-right-native-content");
  });

  test("Tooltip content flips left of its right-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-right-tooltip-trigger").hover();
    await expect(page.locator("#edge-right-tooltip-content")).toBeVisible();
    await assertFlippedLeft(page, "#edge-right-tooltip-trigger", "#edge-right-tooltip-content");
  });

  test("HoverCard content flips left of its right-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-right-hovercard-trigger").hover();
    await expect(page.locator("#edge-right-hovercard-content")).toBeVisible();
    await assertFlippedLeft(page, "#edge-right-hovercard-trigger", "#edge-right-hovercard-content");
  });

  test("Popover (non-modal) content flips left of its right-edge trigger", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#edge-right-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#edge-right-popover-content")).toBeVisible();
    await assertFlippedLeft(page, "#edge-right-popover-trigger", "#edge-right-popover-content");
  });
});

test.describe("Rule 7 — normal mid-viewport placement is unchanged (no spurious flip)", () => {
  // Regression guard for the primitives/src/top_layer.rs
  // `use_anchor_position_fallback` interaction fix: that hook's JS fallback
  // now accepts *either* the primary or the flipped placement as
  // contract-legal (see its doc) precisely so it never fights a real CSS
  // flip -- this rule confirms the common (nothing-needs-to-flip) case
  // still renders at the primary, unflipped placement, i.e. that the
  // widened acceptance didn't turn into "accept anything." Scrolled into
  // view before opening (item 2, 2026-09-01; see Rule 4's comment above) --
  // this rule's whole premise is "plenty of room," which the fixture's
  // below-the-fold resting position (now that the stacking section is
  // in-flow again) does not give it.
  test("Popover (non-modal) with plenty of room stays on its preferred (bottom) side", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();
    const trigger = await rectOf(page, "#stack-popover-trigger");
    const content = await rectOf(page, "#stack-popover-content");
    const debug = JSON.stringify({ trigger, content });
    // Below the trigger (side="bottom", unflipped) -- a block-axis flip
    // would have put it above instead.
    expect(content.top, debug).toBeGreaterThanOrEqual(trigger.bottom - EDGE_TOLERANCE);
  });
});

/**
 * Rule 8 — scroll tracking (docs decision 2026-09-01, from a live-site
 * report: the `ColorPicker` popup floated viewport-fixed while the page
 * scrolled, detached from its trigger). Two independent positioning paths
 * exist (`primitives/src/top_layer.rs`'s `use_anchor_position_fallback`
 * doc, "Scroll/resize tracking"), so both need their own case here:
 *
 *   - The CSS-anchor path (native `anchor()` in the stylesheet): the
 *     platform re-resolves this on scroll for free, with no JS involved at
 *     all -- this fixture's own `Popover`/`Tooltip`/`HoverCard` all land on
 *     this path in this repo's Chromium once `popover/style.css`'s
 *     `@supports` block is loaded, which is the case for every page in this
 *     workspace *except* `color_picker`'s own (see the wiring-gap fix in
 *     `preview/src/components/color_picker/style.css`, added the same day
 *     as this rule) -- so the fixture page used here for the CSS-path case
 *     is `color_picker`, the exact composition the live-site report was
 *     filed against, not this file's own `TopLayerFixture`.
 *   - The JS-measured fallback path (`use_anchor_position_fallback`'s
 *     "neither matches" branch): only engages when the CSS path failed to
 *     land the content correctly, and is the path this file's own
 *     `TopLayerFixture` exercises for its stacking `Popover` -- confirmed
 *     by execution (this session's diagnosis): even with `popover/
 *     style.css`'s `@supports` block loaded on this page, the stacking
 *     popover's content still carries inline `top`/`left` after opening,
 *     the fallback's own tell. That is exactly the scroll-detach case this
 *     rule's tracking fix targets, so it is exercised here rather than
 *     assumed.
 *
 * Both cases assert the same thing: the content-to-trigger offset,
 * measured as content's top-left corner relative to the trigger's
 * bounding box, is unchanged after a 100-150px scroll. `toBeCloseTo(_, 0)`
 * (whole-pixel tolerance) accounts for subpixel rounding across the two
 * `getBoundingClientRect()` reads, not for any real drift.
 */
test.describe("Rule 8 — scroll tracking: an anchored overlay's content keeps its position relative to its trigger while the page scrolls", () => {
  test("Popover (non-modal) on the JS-measured fallback path tracks its trigger through a scroll", async ({ page }) => {
    await gotoFixture(page);
    // Scrolled comfortably into view *before* opening (same reasoning as
    // Rule 4/7 above): this rule's own 100-150px scroll happens *after*
    // opening, deliberately -- that is the thing under test -- so the
    // starting position needs enough headroom on both axes that the modest
    // follow-up scroll below cannot itself cross a flip threshold (which
    // would legitimately change the offset for a reason unrelated to
    // tracking, and this rule isn't the one guarding flip behavior --
    // Rules 5-7 already do).
    await page.locator("#stack-sibling").scrollIntoViewIfNeeded();
    await page.locator("#stack-popover-trigger").evaluate((el) => (el as HTMLElement).click());
    await expect(page.locator("#stack-popover-content")).toBeVisible();

    // Confirm this really is the fallback path (the case this rule exists
    // to cover), not a false-positive pass because CSS anchoring silently
    // took over instead: the fallback is the only thing that ever writes an
    // inline `top` on this element (see `use_anchor_position_fallback`).
    const inlineTop = await page.locator("#stack-popover-content").evaluate(
      (el) => (el as HTMLElement).style.top,
    );
    expect(inlineTop, "expected the JS fallback path to be active here").not.toBe("");

    const offsetOf = async () =>
      page.evaluate(() => {
        const c = document.getElementById("stack-popover-content")!.getBoundingClientRect();
        const t = document.getElementById("stack-popover-trigger")!.getBoundingClientRect();
        return { top: c.top - t.bottom, left: c.left - t.left };
      });

    const before = await offsetOf();
    await page.evaluate(() => window.scrollBy(0, 120));
    // The fix is rAF-throttled (`use_anchor_position_fallback`'s
    // `rafScheduled` flag) -- a couple of frames is enough for it to land.
    await page.waitForTimeout(150);
    const after = await offsetOf();

    const debug = JSON.stringify({ before, after });
    expect(after.top, debug).toBeCloseTo(before.top, 0);
    expect(after.left, debug).toBeCloseTo(before.left, 0);
  });

  test("ColorPicker popover on the CSS-anchor path tracks its trigger through a scroll", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=color_picker&", {
      timeout: NAV_TIMEOUT,
      waitUntil: "networkidle",
    });

    // Opened at scrollY=0 with a raw DOM click (bypassing Playwright's own
    // actionability auto-scroll), *not* pre-scrolled: confirmed by
    // execution (this session's diagnosis) that this repo's Chromium build
    // computes a `[popover]` + `position: fixed` element's *very first*
    // `anchor()`-resolved position incorrectly whenever the document is
    // already scrolled at the moment the popover is shown -- off by
    // exactly the scroll offset, as if measured document-relative instead
    // of viewport-relative -- and that wrong value never self-corrects for
    // the life of that popover instance, even once real further scrolling
    // happens. `use_anchor_position_fallback`'s own fallback correctly
    // detects and compensates for this (this repo's actual, existing
    // protection against exactly this failure mode -- see its doc), so
    // end-user positioning is never wrong; this test's whole point, though,
    // is to isolate the *CSS-only* path specifically, which this Chromium
    // quirk can only be kept out of by not scrolling before the open it is
    // asserting about. A stray earlier concern about needing room below the
    // trigger to avoid a spurious flip at scrollY=0 does not hold on this
    // fixture, confirmed by execution: `data-side` stays "bottom"
    // (unflipped) either way, on this viewport.
    const trigger = page.getByRole("button", { name: /Color picker/i }).first();
    await expect(trigger).toBeVisible();
    await trigger.evaluate((el) => (el as HTMLElement).click());
    const content = page.getByRole("dialog");
    await expect(content).toBeVisible();

    // Confirm this really is the CSS-anchor path (no inline `top` -- the
    // fallback never engaged), so this test cannot pass merely because the
    // JS fallback's own tracking (already covered above) papered over a
    // CSS-anchor regression here.
    const inlineTop = await content.evaluate((el) => (el as HTMLElement).style.top);
    expect(inlineTop, "expected the CSS-anchor path to be active here").toBe("");

    // Let the open-state fade-in settle before taking the "before" reading:
    // `dx-color-picker-popover-fade-in` (`.15s ease-out`,
    // `color_picker/style.css`) transiently animates this element, and
    // confirmed by execution, `getBoundingClientRect()` mid-animation can
    // read a not-yet-settled box on this path -- unrelated to tracking (it
    // resolves to the same final rect either way), but this rule needs a
    // stable "before" to compare "after" against.
    await page.waitForTimeout(200);

    const offsetOf = async () =>
      page.evaluate(() => {
        const c = document.querySelector("dialog[popover]")!.getBoundingClientRect();
        const t = document.querySelector('[style*="anchor-name"]')!.getBoundingClientRect();
        return { top: c.top - t.bottom, left: c.left - t.left };
      });

    const before = await offsetOf();
    await page.evaluate(() => window.scrollBy(0, 150));
    await page.waitForTimeout(150);
    const after = await offsetOf();

    const debug = JSON.stringify({ before, after });
    expect(after.top, debug).toBeCloseTo(before.top, 0);
    expect(after.left, debug).toBeCloseTo(before.left, 0);
  });
});

/**
 * Rule 9 — point-positioned vs. anchored scroll behavior (Migration A slice
 * 2/3). Rule 8 above already covers the *anchored* case (content tracks its
 * trigger through a scroll); this rule adds the two shapes slice 2 migrated:
 *
 *   - `ContextMenu` opens at a raw click point with **no** anchor at all
 *     (`ContextMenuContentRendered`, `context_menu.rs` -- deliberately not
 *     anchored, per this slice's own scope: "CSS anchors need an element,
 *     do NOT anchor"). Its pre-migration behavior -- measured against an
 *     unmodified `position: fixed` div, *before* writing this rule, per
 *     this slice's own instruction to preserve whatever that behavior
 *     already was -- is that `position: fixed`'s containing block is the
 *     viewport (absent a transformed ancestor), so the content stays at
 *     the click's *viewport-relative* position and does not move on screen
 *     as the page scrolls underneath it -- the same thing a native OS
 *     context menu does, and explicitly what Radix's own `ContextMenu`
 *     does too (cited in this slice's own task description). Promoting it
 *     to the top layer via `popover="manual"` does not change this: a
 *     top-layer element's containing block is *also* the viewport, so the
 *     inline `top`/`left` pixel values set at open time keep meaning
 *     exactly what they meant before migrating.
 *   - `Menubar` menus, by contrast, anchor each menu's content to its own
 *     trigger (`MenubarContentRendered`, `menubar.rs`) -- the same shape
 *     `DropdownMenu`'s Rule 8 case exercises -- and so must keep tracking
 *     that trigger through a scroll exactly as Rule 8 already checks for
 *     `Popover`.
 *
 * Fixture: the "Point-positioned vs. anchored scroll behavior" section of
 * `TopLayerFixture` -- both controls sit normally in-flow, away from any
 * clip ancestor or viewport edge, so a 120px scroll cannot itself cross a
 * flip threshold (Rules 5-7 already guard flipping; this rule is only about
 * what happens to already-open content during a scroll).
 */
test.describe("Rule 9 — point-positioned vs. anchored scroll behavior", () => {
  test("ContextMenu content stays at its click's viewport position while the page scrolls (does not track the page)", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#scroll-context-menu-trigger").scrollIntoViewIfNeeded();
    await page.locator("#scroll-context-menu-trigger").click({ button: "right" });
    await expect(page.locator("#scroll-context-menu-content")).toBeVisible();

    const rectOfContent = () => rectOf(page, "#scroll-context-menu-content");
    const before = await rectOfContent();
    await page.evaluate(() => window.scrollBy(0, 120));
    // No positioning logic runs on scroll for this component at all (no
    // fallback, no listener) -- a short settle is still given so this
    // assertion cannot pass merely because it ran before a paint.
    await page.waitForTimeout(150);
    const after = await rectOfContent();

    const debug = JSON.stringify({ before, after });
    // Viewport-relative position unchanged -- the content did not move on
    // screen, i.e. it did *not* track the page's scroll.
    expect(after.top, debug).toBeCloseTo(before.top, 0);
    expect(after.left, debug).toBeCloseTo(before.left, 0);
  });

  test("Menubar content tracks its trigger through a scroll", async ({ page }) => {
    await gotoFixture(page);
    await page.locator("#scroll-menubar-trigger").scrollIntoViewIfNeeded();
    await page.locator("#scroll-menubar-trigger").click();
    await expect(page.locator("#scroll-menubar-content")).toBeVisible();

    const offsetOf = async () =>
      page.evaluate(() => {
        const c = document.getElementById("scroll-menubar-content")!.getBoundingClientRect();
        const t = document.getElementById("scroll-menubar-trigger")!.getBoundingClientRect();
        return { top: c.top - t.bottom, left: c.left - t.left };
      });

    const before = await offsetOf();
    await page.evaluate(() => window.scrollBy(0, 120));
    // rAF-throttled, same as Rule 8's identical wait.
    await page.waitForTimeout(150);
    const after = await offsetOf();

    const debug = JSON.stringify({ before, after });
    expect(after.top, debug).toBeCloseTo(before.top, 0);
    expect(after.left, debug).toBeCloseTo(before.left, 0);
  });
});



/**
 * Rule 11 — anchored-overlay self-overlap contract, iOS on-screen keyboard
 * (2026-09-02, user report: the Combobox options list rendering on top of
 * its own search input after focusing/typing on iOS Safari).
 *
 * Citation: no single WHATWG/W3C section states "an anchored popup must
 * never cover its own anchor" as a normative MUST -- this rule cites this
 * repo's own anchored-overlay contract (the `[data-side]`/`[data-align]`
 * placement formula `use_anchor_position_fallback` implements and
 * self-checks, `primitives/src/top_layer.rs`) plus the platform surface the
 * investigation and fix are built on:
 *   - CSSOM View's `VisualViewport` -- the on-screen-keyboard-aware
 *     viewport rectangle. `window.innerWidth`/`innerHeight` do NOT shrink
 *     when iOS Safari's keyboard appears; `window.visualViewport` does:
 *     <https://www.w3.org/TR/cssom-view-1/#dom-window-visualviewport>
 *     <https://www.w3.org/TR/cssom-view-1/#the-visualviewport-interface>
 *   - The WHATWG HTML popover UA stylesheet default this file's Rule 1-10
 *     already cite -- what an anchored overlay falls back to once CSS
 *     Anchor Positioning can no longer place it:
 *     <https://html.spec.whatwg.org/multipage/rendering.html#the-popover-attribute>
 *
 * Investigation findings (this session, `<scratchpad>/ios-combobox/repro.js`)
 * identified two gaps in `use_anchor_position_fallback` pre-fix:
 *   1. No visual-viewport awareness -- the fallback reads only
 *      `window.innerWidth/innerHeight` + `getBoundingClientRect()`, both
 *      blind to a keyboard-driven `visualViewport` change.
 *   2. Scroll/resize tracking listeners were installed ONLY when the
 *      *first* measurement already concluded `usingFallback` (see that
 *      hook's own "Scroll/resize tracking" doc) -- an overlay that looked
 *      correctly anchored at open time never got a listener at all, so a
 *      later divergence (the keyboard opening after the overlay is already
 *      open -- exactly Combobox's shape: focusing its input both opens the
 *      listbox and summons the keyboard) was never re-measured for the
 *      rest of that open.
 *
 * Scope: every overlay that actually calls `use_anchor_position_fallback`
 * (grepped `primitives/src/*.rs`, `combobox/components/list.rs`,
 * `select/components/list.rs`) -- Tooltip, HoverCard, Popover,
 * DropdownMenu, Menubar, Select, Combobox. `ContextMenu` is deliberately
 * excluded: per Rule 9's own doc above and `context_menu.rs` (grepped: zero
 * references to `use_anchor_position_fallback` or `anchor_id`), it opens at
 * a raw click point with no anchor at all, so it has no "own anchor rect"
 * to overlap and neither of this fix's two gaps can apply to it.
 *
 * Parts (a)-(c) below use the "Clipping escape" section's
 * `clip-*-trigger`/`clip-*-content` pairs (one per component, already used
 * by Rule 1 above) -- reused rather than duplicated, since they already
 * give every one of these seven components its own persistent,
 * id-addressable trigger and content.
 *
 * Simulating an iOS Safari engine in this sandbox's Chromium:
 *   - No Chromium launch flag disables `anchor-name`/`anchor()` support any
 *     more (confirmed empirically this session and in the prior
 *     investigation: the feature has fully shipped and is no longer
 *     flag-gated) -- so "this engine doesn't do CSS Anchor Positioning" is
 *     simulated at the CSS-cascade level instead, in
 *     `stripAnchorSupportsBlock` below: brace-counted removal (a greedy
 *     regex would over/under-match the many nested `{}` inside it) of the
 *     `@supports (anchor-name: --a) { ... }` block's body -- the only
 *     top-level `@supports` in the engine-injected
 *     `#dx-anchor-positioning-styles` stylesheet
 *     (`primitives/src/top_layer.rs`'s `anchor_positioning_inject_js`).
 *     This drops exactly the `anchor()`-based placement rules an engine
 *     without the feature would never have applied, regardless of whether
 *     *this* browser actually supports it -- ported from this session's own
 *     `repro.js` diagnosis harness rather than reinvented, since it was
 *     already verified working there.
 *   - A real iOS keyboard shrinks `window.visualViewport` while leaving
 *     `window.innerWidth/innerHeight` untouched; `page.setViewportSize()`
 *     cannot reproduce that split -- it resizes the real layout viewport
 *     (`window.inner*` included). What it, plus a `mouse.wheel` scroll,
 *     *can* exercise is gap 2 above: does anything re-measure at all once
 *     an overlay is already open? That is what parts (b) and (c) below
 *     drive at. (Real `visualViewport`-only divergence needs a genuine iOS
 *     Safari; see `docs/backlog.md`'s new row for that follow-up.)
 *
 * Part (c)'s second sub-case, "conforming at open, diverging later," is the
 * one gap 2 exists for and needs a component that genuinely lands on the
 * CSS-anchor path at open, not the JS fallback -- confirmed by execution
 * (this session): every one of the seven `clip-*` pairs above already runs
 * the JS fallback at open in this fixture context (the `clip-box`
 * ancestor's `transform` appears to invalidate `anchor()`'s reference the
 * same way it does for `use_anchor_position_fallback`'s own pre-existing
 * cases), and DropdownMenu/Select/Combobox land on the fallback path even
 * on their own, unclipped gallery pages -- the same broad CSS-Anchor-
 * Positioning unevenness `docs/backlog.md` item 21 already documents for
 * ColorPicker/DatePicker in this sandbox. `CONFORMING_CASES` below is the
 * (smaller, execution-verified) set of contexts on this same fixture, or a
 * component's own gallery page, that DO land on the CSS-anchor path at
 * open in this sandbox; the rest are explicitly marked unsupported here
 * with the reason, rather than asserting something that was never true to
 * begin with.
 */
function rectsIntersect(
  a: { top: number; left: number; right: number; bottom: number },
  b: { top: number; left: number; right: number; bottom: number },
): boolean {
  return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
}

async function rectOfLocator(locator: import("@playwright/test").Locator) {
  return locator.evaluate((el) => {
    const r = (el as HTMLElement).getBoundingClientRect();
    return { top: r.top, left: r.left, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  });
}

/** Roughly an iPhone viewport -- see this section's header doc. */
const MOBILE_VIEWPORT = { width: 390, height: 844 };
/**
 * Roughly the visible height left on that viewport once an iOS Safari
 * on-screen keyboard (QWERTY + accessory bar, ~45% of viewport height) is
 * showing. Only the height changes -- a keyboard narrows nothing.
 */
const KEYBOARD_VIEWPORT = { width: 390, height: 460 };

/**
 * Pins `locator` a fixed 24px from the top of the viewport, rather than
 * `scrollIntoViewIfNeeded`'s browser-chosen alignment: parts (b)/(c) below
 * shrink the viewport's *height* after opening (simulating the keyboard),
 * and a trigger left near the bottom of a tall starting viewport could
 * scroll out of the shrunk one for a reason that has nothing to do with
 * this rule (not enough headroom left in the fixture's own layout) --
 * pinning it near the top guarantees the trigger (and, by the placement
 * contract, its content) has room to stay on-screen in
 * `KEYBOARD_VIEWPORT` regardless of where this section falls in page flow.
 */
async function pinNearTop(page: Page, locator: import("@playwright/test").Locator): Promise<void> {
  await locator.scrollIntoViewIfNeeded();
  const rect = await rectOfLocator(locator);
  await page.evaluate((top) => window.scrollTo(0, window.scrollY + top - 24), rect.top);
}

/**
 * See this section's header doc, "Simulating an iOS Safari engine" -- the
 * `page.setViewportSize` + `mouse.wheel` half of the simulation (gap 2:
 * does anything re-measure once already open).
 */
async function simulateOnScreenKeyboard(page: Page): Promise<void> {
  await page.setViewportSize(KEYBOARD_VIEWPORT);
  await page.mouse.wheel(0, 40);
  // rAF-throttled (`use_anchor_position_fallback`'s `rafScheduled` flag,
  // plus its settle loop) -- give tracking a few frames to land.
  await page.waitForTimeout(250);
}

/**
 * See this section's header doc, "Simulating an iOS Safari engine" -- the
 * CSS-cascade half. Ported from `<scratchpad>/ios-combobox/repro.js`'s
 * `stripAnchorSupportsBlock`.
 */
async function stripAnchorSupportsBlock(page: Page): Promise<void> {
  await page.waitForFunction(() => !!document.getElementById("dx-anchor-positioning-styles"), {
    timeout: 15000,
  });
  const result = await page.evaluate(() => {
    const tag = document.getElementById("dx-anchor-positioning-styles") as HTMLStyleElement;
    const src = tag.textContent ?? "";
    const startMarker = "@supports (anchor-name: --a)";
    const idx = src.indexOf(startMarker);
    if (idx === -1) return { ok: false, reason: "marker not found" };
    let i = src.indexOf("{", idx);
    let depth = 0;
    let end = -1;
    for (; i < src.length; i++) {
      if (src[i] === "{") depth++;
      else if (src[i] === "}") {
        depth--;
        if (depth === 0) {
          end = i + 1;
          break;
        }
      }
    }
    if (end === -1) return { ok: false, reason: "unbalanced braces" };
    tag.textContent = src.slice(0, idx) + src.slice(end);
    return { ok: true };
  });
  if (!result.ok) {
    throw new Error(`stripAnchorSupportsBlock failed: ${JSON.stringify(result)}`);
  }
}

interface AnchoredOverlay {
  name: string;
  triggerId: string;
  contentId: string;
  open: (page: Page) => Promise<void>;
  /**
   * Hover-driven overlays (Tooltip/HoverCard) can lose `:hover` when the
   * viewport shrinks and a `mouse.wheel` scroll moves the trigger out from
   * under the (stationary) synthetic pointer -- irrelevant to what this
   * rule checks (placement, not open/close persistence), so those two
   * re-assert the hover after the keyboard-simulation step. A no-op for
   * every click/keyboard-driven overlay.
   */
  reengage?: (page: Page) => Promise<void>;
}

const ANCHORED_OVERLAYS: AnchoredOverlay[] = [
  {
    name: "Tooltip",
    triggerId: "clip-tooltip-trigger",
    contentId: "clip-tooltip-content",
    open: async (page) => {
      await page.locator("#clip-tooltip-trigger").hover();
    },
    reengage: async (page) => {
      await page.locator("#clip-tooltip-trigger").hover();
    },
  },
  {
    name: "HoverCard",
    triggerId: "clip-hovercard-trigger",
    contentId: "clip-hovercard-content",
    open: async (page) => {
      await page.locator("#clip-hovercard-trigger").hover();
    },
    reengage: async (page) => {
      await page.locator("#clip-hovercard-trigger").hover();
    },
  },
  {
    name: "Popover",
    triggerId: "clip-popover-trigger",
    contentId: "clip-popover-content",
    open: async (page) => {
      await page.locator("#clip-popover-trigger").click();
    },
  },
  {
    name: "DropdownMenu",
    triggerId: "clip-dropdown-menu-trigger",
    contentId: "clip-dropdown-menu-content",
    open: async (page) => {
      await page.locator("#clip-dropdown-menu-trigger").click();
    },
  },
  {
    name: "Menubar",
    triggerId: "clip-menubar-trigger",
    contentId: "clip-menubar-content",
    open: async (page) => {
      await page.locator("#clip-menubar-trigger").click();
    },
  },
  {
    name: "Select",
    triggerId: "clip-select-trigger",
    contentId: "clip-select-content",
    open: async (page) => {
      await page.locator("#clip-select-trigger").click();
    },
  },
  {
    name: "Combobox",
    triggerId: "clip-combobox-trigger",
    contentId: "clip-combobox-content",
    // A click alone does not open this popup (Rule 1's identical case
    // above) -- ArrowDown does.
    open: async (page) => {
      await page.locator("#clip-combobox-trigger").click();
      await page.keyboard.press("ArrowDown");
    },
  },
];

async function assertNoSelfOverlapWithinViewport(page: Page, overlay: AnchoredOverlay) {
  const trigger = await rectOf(page, `#${overlay.triggerId}`);
  const content = await rectOf(page, `#${overlay.contentId}`);
  const viewport = await viewportSize(page);
  const debug = JSON.stringify({ overlay: overlay.name, trigger, content, viewport });
  expect(rectsIntersect(content, trigger), debug).toBe(false);
  expect(content.top, debug).toBeGreaterThanOrEqual(-EDGE_TOLERANCE);
  expect(content.bottom, debug).toBeLessThanOrEqual(viewport.height + EDGE_TOLERANCE);
}

test.describe("Rule 11 — anchored-overlay self-overlap contract (2026-09-02 iOS keyboard bug)", () => {
  test.describe("(a) no self-overlap once open", () => {
    for (const overlay of ANCHORED_OVERLAYS) {
      test(`${overlay.name}: content never overlaps its own anchor`, async ({ page }) => {
        await gotoFixture(page);
        await pinNearTop(page, page.locator(`#${overlay.triggerId}`));
        await overlay.open(page);
        await expect(page.locator(`#${overlay.contentId}`)).toBeVisible();
        await assertNoSelfOverlapWithinViewport(page, overlay);
      });
    }
  });

  test.describe("(b) simulated iOS on-screen keyboard (viewport shrink + scroll, after open)", () => {
    for (const overlay of ANCHORED_OVERLAYS) {
      test(`${overlay.name}: re-measures and stays clear of its anchor once the keyboard opens`, async ({
        page,
      }) => {
        await page.setViewportSize(MOBILE_VIEWPORT);
        await gotoFixture(page);
        await pinNearTop(page, page.locator(`#${overlay.triggerId}`));
        await overlay.open(page);
        await expect(page.locator(`#${overlay.contentId}`)).toBeVisible();

        await simulateOnScreenKeyboard(page);
        await overlay.reengage?.(page);

        await assertNoSelfOverlapWithinViewport(page, overlay);
      });
    }
  });

  test.describe("(c) simulated no-anchor engine (CSS Anchor Positioning support stripped)", () => {
    for (const overlay of ANCHORED_OVERLAYS) {
      test(`${overlay.name}: no anchor support from the start, then the keyboard opens`, async ({ page }) => {
        await page.setViewportSize(MOBILE_VIEWPORT);
        await gotoFixture(page);
        await stripAnchorSupportsBlock(page);
        await pinNearTop(page, page.locator(`#${overlay.triggerId}`));
        await overlay.open(page);
        await expect(page.locator(`#${overlay.contentId}`)).toBeVisible();

        await simulateOnScreenKeyboard(page);
        await overlay.reengage?.(page);

        await assertNoSelfOverlapWithinViewport(page, overlay);
      });
    }

    /**
     * The gap this whole rule exists for (gap 2 in this section's header
     * doc): CSS Anchor Positioning is fully intact and correct at *open*
     * time -- `usingFallback` stays false, so pre-fix no tracking listener
     * is ever attached -- and only THEN does anchor-positioning support
     * disappear, mid-open, the same moment an iOS keyboard would appear
     * over a focused Combobox input. Must be RED before the fix.
     *
     * Needs a context that genuinely lands on the CSS-anchor path at open
     * (see this section's header doc, last paragraph, for why the `clip-*`
     * pairs above cannot be reused for this one case) -- `CONFORMING_CASES`
     * below, execution-verified per entry, most deliberately marked
     * unsupported with the reason rather than faked:
     *   - DropdownMenu/Select/Combobox never land on the CSS-anchor path at
     *     all in this sandbox, on any page checked (their own gallery page
     *     or this fixture) -- same broad unevenness `docs/backlog.md` item
     *     21 already documents.
     *   - Tooltip/HoverCard DO conform at open here (`edge-bottom-*`), but
     *     confirmed by execution: shrinking the viewport for the keyboard
     *     simulation moves the trigger out from under Chromium's actual
     *     (unmoved) cursor, and this browser re-hit-tests `:hover` on
     *     scroll/resize for real -- so the *real* browser closes the
     *     overlay via a genuine `pointerleave` before any of this fix's
     *     logic even runs, and re-hovering to keep it open starts a whole
     *     new open cycle (a fresh, cold measurement -- not the same-open
     *     "diverges mid-open" case this test exists for). Not a gap this
     *     fix closes or should try to: a hover-only surface summoning an
     *     on-screen keyboard was never the reported shape to begin with
     *     (Combobox's is a focused, keyboard-persistent text input).
     *   - Menubar conforms at open on its own gallery page at desktop
     *     width, but confirmed by execution: not at `MOBILE_VIEWPORT`
     *     width (390px) -- the one precondition this case needs.
     *   - Popover is the sole survivor, and is enough: confirmed by
     *     execution (before this section's assertions were tightened) that
     *     its content really does stop tracking after the strip (stays at
     *     `style.top === ""`, i.e. no re-measurement ever happens) --
     *     `expect(...).not.toBe("")` below is the direct, mechanism-level
     *     assertion for that (rather than an overlap/viewport check alone,
     *     which this exact fixture's geometry can pass by coincidence even
     *     while genuinely stale -- confirmed by execution: the stale,
     *     UA-default position it fell back to for this specific trigger
     *     happened to land at the viewport's top-left corner, nowhere near
     *     -- but also nowhere *correct* relative to -- the trigger).
     */
    type ConformingCase =
      | {
          name: string;
          supported: true;
          goto: (page: Page) => Promise<void>;
          trigger: (page: Page) => import("@playwright/test").Locator;
          content: (page: Page) => import("@playwright/test").Locator;
          open: (page: Page) => Promise<void>;
        }
      | { name: string; supported: false; reason: string };

    const CONFORMING_CASES: ConformingCase[] = [
      {
        name: "Popover",
        supported: true,
        goto: gotoFixture,
        trigger: (page) => page.locator("#edge-bottom-popover-trigger"),
        content: (page) => page.locator("#edge-bottom-popover-content"),
        open: async (page) => {
          await page.locator("#edge-bottom-popover-trigger").click();
        },
      },
      {
        name: "Tooltip",
        supported: false,
        reason:
          "conforms at open (edge-bottom-tooltip-*) but is hover-driven: the keyboard-simulation resize " +
          "moves the trigger out from under the real (unmoved) cursor, and this Chromium genuinely closes " +
          "the tooltip via a real pointerleave before this fix's logic runs -- not the same-open " +
          "'diverges mid-open' case this test targets, and not the reported bug's shape (a focused text " +
          "input, not a hover surface, is what keeps an overlay open while a keyboard appears).",
      },
      {
        name: "HoverCard",
        supported: false,
        reason: "same as Tooltip above -- hover-driven, genuinely closed by the keyboard-simulation resize.",
      },
      {
        name: "DropdownMenu",
        supported: false,
        reason:
          "no naturally CSS-anchor-conforming context found for DropdownMenu in this Chromium sandbox " +
          "(checked its own gallery page's demo instance and every clip-* pair on the top_layer fixture -- " +
          "all land on the JS fallback path at open already; docs/backlog.md item 21 records this sandbox's " +
          "broader CSS-Anchor-Positioning unevenness). Verify on a real device instead.",
      },
      {
        name: "Menubar",
        supported: false,
        reason:
          "conforms at open on its own gallery page at desktop width, but confirmed by execution: not at " +
          "MOBILE_VIEWPORT width (390px) -- the fallback is already active before the strip step even runs, " +
          "so this specific case ('conforming at open') does not arise for it in this sandbox.",
      },
      {
        name: "Select",
        supported: false,
        reason:
          "no naturally CSS-anchor-conforming context found for Select in this Chromium sandbox (checked " +
          "its own gallery page's demo instance and every clip-* pair on the top_layer fixture -- all land " +
          "on the JS fallback path at open already). Verify on a real device instead.",
      },
      {
        name: "Combobox",
        supported: false,
        reason:
          "no naturally CSS-anchor-conforming context found for Combobox in this Chromium sandbox (checked " +
          "its own gallery page's demo instances and every clip-* pair on the top_layer fixture -- all land " +
          "on the JS fallback path at open already, which is itself the correctly-handled case, not this " +
          "gap). This is the reported bug's own component -- see this session's report for why real-device " +
          "iOS Safari verification is still needed for the exact 'conforming at open' shape.",
      },
    ];

    for (const kase of CONFORMING_CASES) {
      test(`${kase.name}: conforming at open, diverging later (anchor support removed after opening, then the keyboard opens)`, async ({
        page,
      }) => {
        test.skip(!kase.supported, !kase.supported ? kase.reason : "");
        if (!kase.supported) return;

        await page.setViewportSize(MOBILE_VIEWPORT);
        await kase.goto(page);
        await pinNearTop(page, kase.trigger(page));
        await kase.open(page);
        await expect(kase.content(page)).toBeVisible();

        // Confirm CSS Anchor Positioning was genuinely doing the work at
        // open (no inline `top` -- the fallback never engaged; same check
        // Rule 8 makes for its own CSS-anchor-path case above).
        const inlineTopAtOpen = await kase.content(page).evaluate((el) => (el as HTMLElement).style.top);
        expect(inlineTopAtOpen, `expected the CSS-anchor path to be active at open for ${kase.name}`).toBe("");

        await stripAnchorSupportsBlock(page);
        await simulateOnScreenKeyboard(page);

        // The direct, mechanism-level assertion: did anything re-measure at
        // all? Pre-fix, gap 2 means the answer is no -- no listener was
        // ever attached, since this content conformed (via CSS) at open --
        // so `style.top` stays exactly `""` forever, regardless of whether
        // this fixture's particular geometry also happens to overlap or
        // leave the viewport (see this section's header doc for why an
        // overlap/viewport check alone isn't sufficient here).
        const inlineTopAfter = await kase.content(page).evaluate((el) => (el as HTMLElement).style.top);
        expect(
          inlineTopAfter,
          `expected the fix's tracking to have re-measured and taken over once CSS Anchor Positioning ` +
            `stopped resolving mid-open for ${kase.name}, but style.top is still ${JSON.stringify(inlineTopAfter)}`,
        ).not.toBe("");

        const trigger = await rectOfLocator(kase.trigger(page));
        const content = await rectOfLocator(kase.content(page));
        const viewport = await viewportSize(page);
        const debug = JSON.stringify({ overlay: kase.name, trigger, content, viewport });
        expect(rectsIntersect(content, trigger), debug).toBe(false);
        expect(content.top, debug).toBeGreaterThanOrEqual(-EDGE_TOLERANCE);
        expect(content.bottom, debug).toBeLessThanOrEqual(viewport.height + EDGE_TOLERANCE);
      });
    }
  });
});
