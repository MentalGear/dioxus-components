/**
 * ORACLE (tier 3 -- Radix-parity, labelled opinion; see
 * docs/conformance-harness.md "Tier 3 -- Radix-parity" and
 * playwright/oracle/tier3-radix/README.md's rule-source policy).
 *
 * Rule: RTL support, wherever the original (Radix's primitives, beneath
 * shadcn/ui) supports it -- dev-docs/backlog.md row 13, dev-docs/plan.md
 * Phase 6. Every rule below cites the specific Radix source file, and the
 * exact line range, its assertion is drawn from, pinned to
 * `radix-ui/primitives` commit `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae`
 * (the commit this lane cloned and read from -- see this lane's own
 * `$S/batch3/rtl-rust/reference.md` for the full per-component derivation
 * table). Two components here (Resizable, Calendar) have no Radix/shadcn-
 * with-Radix-primitive original to cite at all (shadcn's own versions wrap
 * `react-resizable-panels`/`react-day-picker`, neither a Radix primitive) --
 * their rules are labelled as an extrapolation from the unanimous pattern
 * every other rule below establishes, not a citation, consistent with this
 * tier's own "read as a proposal" policy.
 *
 * FIXTURE DECISION (resolves dev-docs/conformance-harness.md's queue item 8,
 * "no RTL fixture exists," and dev-docs/backlog.md row 13's "waits on an RTL
 * fixture decision"): one `rtl` variant page per RTL-aware component
 * (`/component/?name=<x>&variant=rtl&`), whose `Demo` sets `dir="rtl"` on
 * its own root element and wraps its primitives in
 * `DirectionProvider { direction: Direction::Rtl, ... }` -- see any of
 * `preview/src/components/<name>/variants/rtl/mod.rs` for the exact shape.
 * `DirectionProvider` itself renders no DOM element (a pure context
 * boundary), so the ambient direction each primitive resolves via
 * `use_direction(None)` comes entirely from that provider; each fixture's
 * own themed component instance is never given a local `dir` prop
 * override, so this also exercises the "falls back to context" half of
 * `use_direction`'s contract, not just the "local override wins" half
 * (which `primitives/src/direction.rs`'s own unit tests already cover).
 * Every RTL-aware component also has a `main` variant (already existed,
 * unmodified by this lane) that stays LTR -- used below as this file's own
 * control group, proving the LTR default is unchanged by this work.
 *
 * WHY TIER 3, NOT TIER 1/2: `@radix-ui/react-direction` and every
 * component's own `useDirection` consumption is Radix's own design, not a
 * W3C/APG requirement -- APG's own keyboard-interaction prose is written
 * assuming a single (implicitly LTR) reading direction and says nothing
 * about RTL mirroring anywhere this lane found. This tier's rule-source
 * policy fits exactly: "useful only where the standards are silent."
 *
 * SCOPING -- READ BEFORE EDITING (found live, the hard way): every variant
 * of a "Normal"-kind component renders on the SAME page at once, `main`
 * inside the page's own top section and every other variant (including
 * "rtl") inside a separate, always-mounted "Variants" section further down
 * -- see `preview/src/main.rs`'s `ComponentHighlight`
 * (`let [main, variants @ ..] = variants`, two separate render blocks). A
 * bare, unscoped `page.getByRole(...)`/`getByText(...)` therefore risks a
 * Playwright strict-mode violation (it matches more than one element) the
 * moment a component has more than one variant -- which every RTL-aware
 * component now does. `ComponentVariantHighlight` already solves exactly
 * this for its own frame element: `id="component-preview-frame"` for
 * `main`, `id="component-preview-frame-<variant name>"` for every other
 * variant (see that function's own doc comment in main.rs) -- unique
 * page-wide, and it wraps exactly one variant's own rendered demo (the
 * harness's *own* "DEMO"/"CODE" tab chrome sits *outside* this element, so
 * scoping through it also excludes that chrome -- material for any
 * RTL-aware component whose own name collides with a primitive the harness
 * itself uses, e.g. Tabs). `demoFrame()` below roots every locator in this
 * file through that id instead of the page, so each test only ever sees
 * its own variant's own markup.
 *
 * A second, independent scoping issue: a submenu's own content
 * (`DropdownMenuSubContent`/`ContextMenuSubContent`) renders NESTED inside
 * its root menu's content in the DOM (confirmed live via `scripts/
 * inspect.mjs`: `rootContent.contains(subContent) === true`, no portal
 * relocation), so once a submenu is open, `role="menu"` matches *two*
 * elements and a `.filter({ hasText: "..." })` on the sub-item's own text
 * matches *both* (the root's own `textContent` includes its submenu's,
 * since it's a descendant) -- another strict-mode violation, this one
 * present even for a single variant in isolation. `subContentFor()` below
 * resolves it the ARIA-idiomatic way: every sub-trigger carries its own
 * `aria-controls` pointing at its own submenu's `id` (confirmed live via
 * `scripts/inspect.mjs`), so the submenu is looked up by that id rather
 * than by role+text.
 *
 * A third, related issue -- not a scoping bug in this file, but in the
 * fixtures it drives -- is documented on each affected
 * `preview/src/components/<name>/variants/rtl/mod.rs`'s own doc comment:
 * several RTL fixtures were originally written reusing the `main`
 * variant's own visible labels
 * verbatim (the natural choice for an apples-to-apples LTR/RTL
 * comparison), which is exactly the condition the first scoping issue
 * above describes for every *pre-existing* spec that queries those labels
 * by name -- confirmed live, this broke 101 assertions across
 * `tabs.spec.ts`, `radio-group.spec.ts`, `toggle_group.spec.ts`,
 * `menubar.spec.ts`, `navbar.spec.ts`, `dropdown-menu.spec.ts`,
 * `context-menu.spec.ts`, `select.spec.ts`, and the shared
 * `oracle/tier1-apg/{keyboard-matrix,menu-roles,menu-submenu}.spec.ts`
 * (all strict-mode violations, `grep -c "strict mode violation"` equal to
 * the failure count exactly -- no other failure cause was present). Fixed
 * at the fixture level (giving each affected RTL fixture's labels its own,
 * non-colliding words) rather than in those spec files, which are out of
 * this lane's editable scope and need no changes of their own. This is a
 * *mitigation* within that scope, not the class fix: the root cause is
 * architectural (`ComponentHighlight` in `preview/src/main.rs`, out of
 * this lane's editable scope, renders every variant unconditionally with
 * no isolation between them, unlike the iframe-isolated `/component/
 * block/` route Block-kind components already get), so the same collision
 * can recur for any *future* variant of any component, added by any future
 * lane, that happens to reuse another variant's visible text -- this fix
 * subsumes only the 13 fixtures this lane itself authored, not that
 * general risk. See this lane's own final report for the fuller account.
 *
 * RESOURCE GATE: run against this lane's own dev server (port 8110, see
 * this lane's own final report for the exact `dx serve` invocation) --
 * `PLAYWRIGHT_BASE_URL=http://127.0.0.1:8110` retargets every test below
 * via `BASE_URL`, imported (not a local literal) since this is a new file.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import { BASE_URL } from "./../../base-url";

async function goto(page: Page, name: string, variant: string) {
  await page.goto(`${BASE_URL}/component/?name=${name}&variant=${variant}&`, {
    timeout: 20 * 60 * 1000,
  });
}

// See this file's own header ("SCOPING -- READ BEFORE EDITING") for why
// every locator below is rooted through this rather than through `page`
// directly.
function demoFrame(page: Page, variant: "main" | "rtl"): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`);
}

// See this file's own header for why a submenu's own content is looked up
// by its trigger's `aria-controls`, not by role+text.
async function subContentFor(subTrigger: Locator): Promise<Locator> {
  const id = await subTrigger.getAttribute("aria-controls");
  expect(id, "sub-trigger should own its submenu via aria-controls (see oracle/tier1-apg/menu-submenu.spec.ts)").toBeTruthy();
  return subTrigger.page().locator(`#${id}`);
}

/* -----------------------------------------------------------------------
 * Roving-focus family: Tabs, RadioGroup, ToggleGroup, Toolbar, Menubar,
 * Navbar. Radix citation: packages/react/roving-focus/src/
 * roving-focus-group.tsx, lines 348-365 (`MAP_KEY_TO_FOCUS_INTENT` +
 * `getDirectionAwareKey`) -- every one of the five Radix components below
 * delegates its *entire* direction-awareness to this one shared
 * `RovingFocusGroup` (confirmed by reading each of tabs.tsx,
 * radio-group.tsx, toggle-group.tsx, toolbar.tsx, menubar.tsx: each simply
 * forwards `dir` to it, no per-component direction logic of its own).
 * `ArrowLeft` becomes `HorizontalNav::Next` under RTL (Radix:
 * `getDirectionAwareKey('ArrowLeft', 'rtl') === 'ArrowRight'`, then
 * `MAP_KEY_TO_FOCUS_INTENT['ArrowRight'] === 'next'`); `ArrowRight`
 * becomes `Prev`, symmetrically.
 * --------------------------------------------------------------------- */

test.describe("Roving focus: Tabs", () => {
  test("RTL: ArrowLeft from the middle tab moves to the NEXT tab", async ({ page }) => {
    await goto(page, "tabs", "rtl");
    const scope = demoFrame(page, "rtl");
    const root = scope.locator('[data-direction]').first();
    await expect(root).toHaveAttribute("dir", "rtl");
    await expect(root).toHaveAttribute("data-direction", "rtl");

    const tab2 = scope.getByRole("tab", { name: "Two", exact: true });
    await tab2.click();
    await expect(tab2).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("tab", { name: "Three", exact: true })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft from the middle tab moves to the PREVIOUS tab", async ({ page }) => {
    await goto(page, "tabs", "main");
    const scope = demoFrame(page, "main");
    const tab2 = scope.getByRole("tab", { name: "Tab 2" });
    await tab2.click();
    await expect(tab2).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("tab", { name: "Tab 1" })).toBeFocused();
  });
});

test.describe("Roving focus: RadioGroup", () => {
  test("RTL: ArrowLeft from the middle item moves to the NEXT item", async ({ page }) => {
    await goto(page, "radio_group", "rtl");
    const scope = demoFrame(page, "rtl");
    const teal = scope.getByRole("radio", { name: "Teal" });
    await teal.click();
    await expect(teal).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("radio", { name: "Violet" })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft has no effect (the main fixture is vertical, not horizontal)", async ({ page }) => {
    await goto(page, "radio_group", "main");
    const scope = demoFrame(page, "main");
    const red = scope.getByRole("radio", { name: "Red" });
    await red.click();
    await expect(red).toBeFocused();
    // The `main` fixture never sets `horizontal: true` (vertical is the
    // default), so `ArrowLeft`/`ArrowRight` are not roving-focus keys for
    // it at all -- `radio_group.rs` gates that whole arm on `horizontal`
    // (confirmed live: pressing it here leaves focus exactly where it
    // was). The RTL fixture below sets `horizontal: true` specifically so
    // its own Left/Right comparison is meaningful; `main`'s existing
    // vertical Up/Down roving focus is already covered by
    // `radio-group.spec.ts`/`keyboard-matrix.spec.ts`, so this test
    // instead confirms the `horizontal` gate itself is unaffected by this
    // lane's direction work.
    await page.keyboard.press("ArrowLeft");
    await expect(red).toBeFocused();
  });
});

test.describe("Roving focus: ToggleGroup", () => {
  test("RTL: ArrowLeft from the middle item moves to the NEXT item", async ({ page }) => {
    await goto(page, "toggle_group", "rtl");
    const scope = demoFrame(page, "rtl");
    const items = scope.getByRole("button").filter({ hasText: /^[SHC]$/ });
    await items.nth(1).click();
    await expect(items.nth(1)).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(items.nth(2)).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft from the middle item moves to the PREVIOUS item", async ({ page }) => {
    await goto(page, "toggle_group", "main");
    const scope = demoFrame(page, "main");
    const items = scope.getByRole("button").filter({ hasText: /^[BIU]$/ });
    await items.nth(1).click();
    await expect(items.nth(1)).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(items.nth(0)).toBeFocused();
  });
});

test.describe("Roving focus: Toolbar", () => {
  test("RTL: ArrowLeft from the middle button moves to the NEXT button", async ({ page }) => {
    await goto(page, "toolbar", "rtl");
    const scope = demoFrame(page, "rtl");
    const two = scope.getByRole("button", { name: "Two" });
    await two.click();
    await expect(two).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("button", { name: "Three" })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft from a button moves to the PREVIOUS button", async ({ page }) => {
    await goto(page, "toolbar", "main");
    const scope = demoFrame(page, "main");
    const italic = scope.getByRole("button", { name: "Italic" });
    await italic.click();
    await expect(italic).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("button", { name: "Bold" })).toBeFocused();
  });
});

test.describe("Roving focus: Menubar (always-horizontal trigger row)", () => {
  test("RTL: ArrowLeft from the first trigger moves to the second (the next trigger)", async ({ page }) => {
    await goto(page, "menubar", "rtl");
    const scope = demoFrame(page, "rtl");
    const options = scope.getByRole("menuitem", { name: "Options" });
    await options.click();
    await page.keyboard.press("Escape");
    await options.focus();
    await expect(options).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("menuitem", { name: "Modify" })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft from Edit moves to File (the previous trigger)", async ({ page }) => {
    await goto(page, "menubar", "main");
    const scope = demoFrame(page, "main");
    const edit = scope.getByRole("menuitem", { name: "Edit" });
    await edit.click();
    await page.keyboard.press("Escape");
    await edit.focus();
    await expect(edit).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("menuitem", { name: "File" })).toBeFocused();
  });
});

test.describe("Roving focus: Navbar (always-horizontal trigger row)", () => {
  // `Navbar`'s own roving-tabindex root carries `role="menubar"`, and
  // clicking a trigger opens its dropdown rather than simply focusing it
  // (`data-state` doesn't even exist on the trigger the way it does on
  // Menubar's, confirmed live) -- so, exactly like the pre-existing
  // `navbar.spec.ts`'s own "keyboard navigation" test, focus is
  // established by focusing the `menubar` container itself (which resolves
  // to whichever item currently owns the roving `tabindex="0"`, "Widgets"/
  // "Inputs" -- the first item -- initially), not by clicking a trigger
  // then pressing Escape.
  test("RTL: ArrowLeft from the first nav moves to the second (the next trigger)", async ({ page }) => {
    await goto(page, "navbar", "rtl");
    const scope = demoFrame(page, "rtl");
    await scope.getByRole("menubar").focus();
    await expect(scope.getByRole("menuitem", { name: "Widgets" })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("menuitem", { name: "Details" })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft from the second nav moves to the first (the previous trigger)", async ({ page }) => {
    await goto(page, "navbar", "main");
    const scope = demoFrame(page, "main");
    await scope.getByRole("menubar").focus();
    await page.keyboard.press("ArrowRight");
    await expect(scope.getByRole("menuitem", { name: "Information" })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(scope.getByRole("menuitem", { name: "Inputs" })).toBeFocused();
  });
});

/* -----------------------------------------------------------------------
 * Menu family: DropdownMenu.Sub / ContextMenu.Sub open/close key + side.
 * Radix citation: packages/react/menu/src/menu.tsx, lines 37-47
 * (`SELECTION_KEYS`, `SUB_OPEN_KEYS`, `SUB_CLOSE_KEYS`) and line 1245
 * (`side={dir===RTL?'left':'right'}`). Under RTL: `ArrowLeft` opens a
 * submenu and focuses its first item (LTR: `ArrowRight`); `ArrowRight`
 * closes it and returns focus to the sub-trigger (LTR: `ArrowLeft`); the
 * submenu opens to the trigger's LEFT (LTR: right). `Escape` is not part
 * of either keyset upstream and stays direction-independent here too (this
 * crate folds it into the same close arm as a pre-existing, unrelated
 * design choice -- see primitives/src/menu_sub.rs's
 * `is_submenu_close_arrow_key` doc).
 *
 * Each test reaches the sub-trigger by roving focus (repeated `ArrowDown`
 * from the just-opened root menu), the way a keyboard user would, exactly
 * like the pre-existing `oracle/tier1-apg/menu-submenu.spec.ts` already
 * does -- not `subTrigger.focus()`. That is not a style preference: tried
 * first, `.focus()` reproducibly reverts focus to the root trigger and
 * closes the whole menu before the following key press lands, and it does
 * so identically on the untouched `main` variant with the pre-existing
 * "More tools" trigger -- confirmed live, so it is a pre-existing quirk of
 * `primitives/src/{dropdown_menu,context_menu}.rs`'s own focus handling,
 * unrelated to this lane's direction work (out of scope to fix here; see
 * this lane's own final report). Roving focus sidesteps it (and is more
 * faithful to a real keyboard user besides), matching this file's own
 * fixtures' index layout: RTL's `Modify`(0)/`Extra tools`(1)/`Erase`(2)
 * needs 2 `ArrowDown`s to reach the sub-trigger; `main`'s pre-existing
 * `Edit`(0)/`Undo`(1, disabled, skipped)/`Duplicate`(2)/`More tools`(3)
 * (DropdownMenu) or .../`Delete`(3)/`More tools`(4) (ContextMenu) needs 3
 * or 4 respectively -- see each fixture's own file for its index layout.
 * --------------------------------------------------------------------- */

test.describe("Menu family: DropdownMenu.Sub", () => {
  test("RTL: ArrowLeft opens the submenu (focus -> first item), ArrowRight closes it (focus -> sub-trigger), opens to the LEFT", async ({ page }) => {
    await goto(page, "dropdown_menu", "rtl");
    const scope = demoFrame(page, "rtl");
    await scope.getByRole("button", { name: "Launch Menu" }).click();
    const subTrigger = scope.getByRole("menuitem", { name: "Extra tools" });
    // Modify(0) -> Extra tools(1): 2 ArrowDowns (see this describe block's
    // own header comment).
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(subTrigger).toBeFocused();

    await page.keyboard.press("ArrowLeft");
    const subContent = await subContentFor(subTrigger);
    await expect(subContent).toBeVisible();
    await expect(subContent.getByRole("menuitem", { name: "Relabel" })).toBeFocused();

    const triggerBox = await subTrigger.boundingBox();
    const contentBox = await subContent.boundingBox();
    expect(triggerBox).not.toBeNull();
    expect(contentBox).not.toBeNull();
    // Opens to the left: the submenu's right edge sits at or left of the
    // trigger's own left edge (a 1px tolerance for anchor-engine rounding).
    expect(contentBox!.x + contentBox!.width).toBeLessThanOrEqual(triggerBox!.x + 1);

    await page.keyboard.press("ArrowRight");
    await expect(subContent).toBeHidden();
    await expect(subTrigger).toBeFocused();
  });

  test("control (LTR, main variant): ArrowRight opens the submenu, ArrowLeft closes it, opens to the RIGHT", async ({ page }) => {
    await goto(page, "dropdown_menu", "main");
    const scope = demoFrame(page, "main");
    await scope.getByRole("button", { name: "Open Menu" }).click();
    const subTrigger = scope.getByRole("menuitem", { name: "More tools" });
    // Edit(0) -> Undo(1, disabled, skipped) -> Duplicate(2) -> More
    // tools(3): 3 ArrowDowns.
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(subTrigger).toBeFocused();

    await page.keyboard.press("ArrowRight");
    const subContent = await subContentFor(subTrigger);
    await expect(subContent).toBeVisible();
    await expect(subContent.getByRole("menuitem", { name: "Rename" })).toBeFocused();

    const triggerBox = await subTrigger.boundingBox();
    const contentBox = await subContent.boundingBox();
    expect(triggerBox).not.toBeNull();
    expect(contentBox).not.toBeNull();
    // Opens to the right: the submenu's left edge sits at or right of the
    // trigger's own right edge.
    expect(contentBox!.x).toBeGreaterThanOrEqual(triggerBox!.x + triggerBox!.width - 1);

    await page.keyboard.press("ArrowLeft");
    await expect(subContent).toBeHidden();
    await expect(subTrigger).toBeFocused();
  });
});

test.describe("Menu family: ContextMenu.Sub", () => {
  test("RTL: ArrowLeft opens the submenu (focus -> first item), ArrowRight closes it", async ({ page }) => {
    await goto(page, "context_menu", "rtl");
    const scope = demoFrame(page, "rtl");
    const trigger = scope.getByText("RTL context zone");
    await trigger.click({ button: "right" });
    const subTrigger = scope.getByRole("menuitem", { name: "Extra tools" });
    // Modify(0) -> Extra tools(1): 2 ArrowDowns (see the DropdownMenu.Sub
    // describe block's own header comment for why roving focus, not
    // `.focus()`).
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(subTrigger).toBeFocused();

    await page.keyboard.press("ArrowLeft");
    const subContent = await subContentFor(subTrigger);
    await expect(subContent).toBeVisible();
    await expect(subContent.getByRole("menuitem", { name: "Relabel" })).toBeFocused();

    await page.keyboard.press("ArrowRight");
    await expect(subContent).toBeHidden();
    await expect(subTrigger).toBeFocused();
  });

  test("control (LTR, main variant): ArrowRight opens the submenu, ArrowLeft closes it", async ({ page }) => {
    await goto(page, "context_menu", "main");
    const scope = demoFrame(page, "main");
    const trigger = scope.getByText("right click here");
    await trigger.click({ button: "right" });
    const subTrigger = scope.getByRole("menuitem", { name: "More tools" });
    // Edit(0) -> Undo(1, disabled, skipped) -> Duplicate(2) -> Delete(3)
    // -> More tools(4): 4 ArrowDowns.
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(subTrigger).toBeFocused();

    await page.keyboard.press("ArrowRight");
    const subContent = await subContentFor(subTrigger);
    await expect(subContent).toBeVisible();
    await expect(subContent.getByRole("menuitem", { name: "Rename" })).toBeFocused();

    await page.keyboard.press("ArrowLeft");
    await expect(subContent).toBeHidden();
    await expect(subTrigger).toBeFocused();
  });
});

/* -----------------------------------------------------------------------
 * Slider: pointer value mapping + keyboard mapping + dir attribute.
 * Radix citation: packages/react/slider/src/slider.tsx, lines 341-353
 * (`isSlidingFromLeft`, `getValueFromPointer`) and lines 32-36, 385-387
 * (`BACK_KEYS`). Horizontal only -- `SliderVertical` never takes a `dir`
 * prop (same file, line 400).
 * --------------------------------------------------------------------- */

test.describe("Slider", () => {
  test("RTL: dir/data-direction present; a click near the left end yields a HIGH value; ArrowLeft increases, ArrowRight decreases", async ({ page }) => {
    await goto(page, "slider", "rtl");
    const scope = demoFrame(page, "rtl");
    const thumb = scope.getByRole("slider");
    const group = scope.locator('[data-direction]').first();
    await expect(group).toHaveAttribute("dir", "rtl");
    await expect(group).toHaveAttribute("data-direction", "rtl");

    // Click near the left end of the track (via the slider's own
    // `role="group"` container, not the thumb's -- the thumb starts at the
    // 50% default value's position).
    // `has`'s own inner locator is intentionally `page`-rooted, not
    // `scope`-rooted: confirmed live, `.filter({ has: scope.getByRole(...) })`
    // -- both locators sharing the exact same `#component-preview-frame*`
    // root -- never resolves (Playwright re-evaluates `has` relative to
    // each candidate, and a `has` locator that itself carries that same
    // absolute id root can never match as that candidate's own
    // descendant). `page`-rooted works because `has` still only checks
    // for a match *within* each candidate, which is exactly what's
    // needed here; `scope.getByRole("group")` alone still keeps the
    // group itself scoped to this variant.
    const track = scope.getByRole("group").filter({ has: page.getByRole("slider") });
    const trackBox = await track.boundingBox();
    expect(trackBox).not.toBeNull();
    // A locator-rooted `.click({ position })`, not `page.mouse.click(x, y)`
    // at the box's absolute coordinates: confirmed live, the raw
    // coordinate click landed before the track's own `onpointerdown`
    // listener had attached (no error, no effect -- the value stayed at
    // its untouched default), while the locator-rooted click's built-in
    // actionability wait (stable, receives-events) resolves it correctly
    // and was not flaky across repeated runs.
    await track.click({ position: { x: 5, y: trackBox!.height / 2 } });
    const highValue = Number(await thumb.getAttribute("aria-valuenow"));
    expect(highValue).toBeGreaterThan(90);

    // Reset to a mid value via Home then a few explicit steps isn't needed:
    // press ArrowLeft/ArrowRight from this near-max position and check the
    // relative change instead of an absolute target.
    await thumb.focus();
    const before = Number(await thumb.getAttribute("aria-valuenow"));
    await page.keyboard.press("ArrowLeft");
    const afterLeft = Number(await thumb.getAttribute("aria-valuenow"));
    expect(afterLeft).toBeGreaterThan(before);
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    const afterRight = Number(await thumb.getAttribute("aria-valuenow"));
    expect(afterRight).toBeLessThan(afterLeft);
  });

  test("control (LTR, main variant): a click near the left end yields a LOW value; ArrowLeft decreases, ArrowRight increases", async ({ page }) => {
    await goto(page, "slider", "main");
    const scope = demoFrame(page, "main");
    const thumb = scope.getByRole("slider");
    // `has`'s own inner locator is intentionally `page`-rooted, not
    // `scope`-rooted: confirmed live, `.filter({ has: scope.getByRole(...) })`
    // -- both locators sharing the exact same `#component-preview-frame*`
    // root -- never resolves (Playwright re-evaluates `has` relative to
    // each candidate, and a `has` locator that itself carries that same
    // absolute id root can never match as that candidate's own
    // descendant). `page`-rooted works because `has` still only checks
    // for a match *within* each candidate, which is exactly what's
    // needed here; `scope.getByRole("group")` alone still keeps the
    // group itself scoped to this variant.
    const track = scope.getByRole("group").filter({ has: page.getByRole("slider") });
    const trackBox = await track.boundingBox();
    expect(trackBox).not.toBeNull();
    // See the RTL test above for why this is a locator-rooted
    // `.click({ position })`, not `page.mouse.click(x, y)`.
    await track.click({ position: { x: 5, y: trackBox!.height / 2 } });
    const lowValue = Number(await thumb.getAttribute("aria-valuenow"));
    expect(lowValue).toBeLessThan(10);

    await thumb.focus();
    const before = Number(await thumb.getAttribute("aria-valuenow"));
    await page.keyboard.press("ArrowRight");
    const afterRight = Number(await thumb.getAttribute("aria-valuenow"));
    expect(afterRight).toBeGreaterThan(before);
    await page.keyboard.press("ArrowLeft");
    await page.keyboard.press("ArrowLeft");
    const afterLeft = Number(await thumb.getAttribute("aria-valuenow"));
    expect(afterLeft).toBeLessThan(afterRight);
  });
});

/* -----------------------------------------------------------------------
 * dir/data-direction attribute presence: Select, ScrollArea. Neither has
 * an ArrowLeft/ArrowRight to flip (both vertical-only/native-scrollbar --
 * see reference.md's Select/ScrollArea rows for why), so the attribute
 * contract is the whole rule.
 * --------------------------------------------------------------------- */

test.describe("Select: dir attribute", () => {
  test("RTL: dir/data-direction present on trigger and listbox", async ({ page }) => {
    await goto(page, "select", "rtl");
    const scope = demoFrame(page, "rtl");
    const trigger = scope.getByRole("combobox");
    await expect(trigger).toHaveAttribute("dir", "rtl");
    await expect(trigger).toHaveAttribute("data-direction", "rtl");

    await trigger.click();
    const listbox = scope.getByRole("listbox");
    await expect(listbox).toBeVisible();
    await expect(listbox).toHaveAttribute("dir", "rtl");
    await expect(listbox).toHaveAttribute("data-direction", "rtl");
  });

  test("control (LTR, main variant): no dir=rtl anywhere", async ({ page }) => {
    await goto(page, "select", "main");
    const trigger = demoFrame(page, "main").getByRole("combobox");
    await expect(trigger).not.toHaveAttribute("dir", "rtl");
  });
});

test.describe("ScrollArea: dir attribute", () => {
  test("RTL: dir/data-direction present on the root", async ({ page }) => {
    await goto(page, "scroll_area", "rtl");
    const root = demoFrame(page, "rtl").locator('[data-scroll-direction]').first();
    await expect(root).toHaveAttribute("dir", "rtl");
    await expect(root).toHaveAttribute("data-direction", "rtl");
  });

  test("control (LTR, main variant): no dir=rtl on the root", async ({ page }) => {
    await goto(page, "scroll_area", "main");
    const root = demoFrame(page, "main").locator('[data-scroll-direction]').first();
    await expect(root).not.toHaveAttribute("dir", "rtl");
  });
});

/* -----------------------------------------------------------------------
 * Resizable: extrapolation, not a citation (no Radix/shadcn-with-Radix
 * original -- shadcn's own Resizable wraps react-resizable-panels). Same
 * arrow-key convention as every rule above: a physical arrow key always
 * moves the divider in that same physical direction. Horizontal only.
 * --------------------------------------------------------------------- */

test.describe("Resizable (extrapolated, no Radix original -- see file header)", () => {
  test("RTL: dir/data-direction present; ArrowLeft/ArrowRight on the handle change the primary panel's size in opposite senses to LTR", async ({ page }) => {
    await goto(page, "resizable", "rtl");
    const rtlScope = demoFrame(page, "rtl");
    const handle = rtlScope.getByRole("separator", { name: "Divider" });
    const group = rtlScope.locator('[data-direction]').first();
    await expect(group).toHaveAttribute("dir", "rtl");
    await expect(group).toHaveAttribute("data-direction", "rtl");

    await handle.focus();
    const before = Number(await handle.getAttribute("aria-valuenow"));
    await page.keyboard.press("ArrowLeft");
    const afterLeft = Number(await handle.getAttribute("aria-valuenow"));
    expect(afterLeft).not.toBeCloseTo(before, 5);
    const rtlLeftDelta = afterLeft - before;

    await goto(page, "resizable", "main");
    const mainScope = demoFrame(page, "main");
    const ltrHandle = mainScope.getByRole("separator").first();
    await ltrHandle.focus();
    const ltrBefore = Number(await ltrHandle.getAttribute("aria-valuenow"));
    await page.keyboard.press("ArrowLeft");
    const ltrAfterLeft = Number(await ltrHandle.getAttribute("aria-valuenow"));
    const ltrLeftDelta = ltrAfterLeft - ltrBefore;

    // Same physical key, opposite effect on the primary pane's size.
    expect(Math.sign(rtlLeftDelta)).toBe(-Math.sign(ltrLeftDelta));
  });
});

/* -----------------------------------------------------------------------
 * Calendar: extrapolation, not a citation (no Radix Calendar -- shadcn's
 * own wraps react-day-picker). Same arrow-key convention: ArrowLeft moves
 * to the NEXT day under RTL (LTR: previous day).
 * --------------------------------------------------------------------- */

test.describe("Calendar (extrapolated, no Radix original -- see file header)", () => {
  // Day cells' accessible name is a full date ("Friday, May 15, 2026" --
  // `aria_label()`, primitives/src/calendar.rs), not the plain day number,
  // so these locate by the cell's exact visible TEXT ("15") instead of by
  // role name. Scoped to the grid so an adjacent month's filler days (a
  // handful at the visible month's edges) can't collide -- none of the
  // fixture's view month's edge-filler days land on 14/15/16/23 for
  // 2026-05 (view_date in both the `rtl` and `main` fixtures).
  test("RTL: ArrowLeft moves focus to the NEXT day; ArrowUp/ArrowDown (+/-7 days) unaffected", async ({ page }) => {
    await goto(page, "calendar", "rtl");
    const grid = demoFrame(page, "rtl").getByRole("grid");
    await expect(grid).toHaveAttribute("dir", "rtl");
    await expect(grid).toHaveAttribute("data-direction", "rtl");

    const day15 = grid.getByText("15", { exact: true });
    await day15.click();
    await expect(day15).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(grid.getByText("16", { exact: true })).toBeFocused();

    await page.keyboard.press("ArrowDown");
    await expect(grid.getByText("23", { exact: true })).toBeFocused();
  });

  test("control (LTR, main variant): ArrowLeft moves focus to the PREVIOUS day", async ({ page }) => {
    await goto(page, "calendar", "main");
    const grid = demoFrame(page, "main").getByRole("grid");
    const day15 = grid.getByText("15", { exact: true });
    await day15.click();
    await expect(day15).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(grid.getByText("14", { exact: true })).toBeFocused();
  });
});
