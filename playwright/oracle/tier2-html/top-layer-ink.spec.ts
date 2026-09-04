/**
 * ORACLE: tier 2 (HTML/CSS) — top-layer text color (`[popover]`/`<dialog>`
 * UA `color: CanvasText` beats DOM-tree inheritance).
 *
 * ## The bug class (already diagnosed; this file is the regression oracle)
 *
 * Both the `[popover]` and `<dialog>` user-agent stylesheets set `color:
 * CanvasText` **directly on the element**:
 *   - WHATWG HTML's rendering chapter defines the popover UA rules:
 *     <https://html.spec.whatwg.org/multipage/rendering.html#the-popover-attribute-2>
 *   - The `<dialog>` element's own UA rules (same chapter):
 *     <https://html.spec.whatwg.org/multipage/rendering.html#flow-content-3>
 *     (Chromium's concrete `html.css`/`user-agent-content.css` -- not a
 *     spec text, but the shipped instance of the same WHATWG-mandated
 *     defaults -- literally has `[popover], dialog { color: CanvasText;
 *     ... }`; `CanvasText` itself is a CSS Color Module Level 4 system
 *     color keyword: <https://www.w3.org/TR/css-color-4/#css-system-colors>.)
 *
 * A **directly-cascaded** declaration on an element always wins over an
 * **inherited** one, regardless of specificity or source order -- the
 * cascade's "winning declaration" algorithm only ever falls through to an
 * ancestor's inherited value when the element itself has *no* declared
 * value for the property at all (CSS Cascading and Inheritance Level 4,
 * "the cascade": <https://www.w3.org/TR/css-cascade-4/#cascading>, and
 * "specified value"/inheritance: <https://www.w3.org/TR/css-cascade-4/
 * #inheriting>). Author origin already beats user-agent origin outright
 * (`#cascade-origin`), so any component's *own* `color` rule for its
 * top-layer content already wins with no help needed -- but a top-layer
 * wrapper that declares no `color` of its own falls straight to the UA
 * default instead of quietly inheriting the app's ink token
 * (`--secondary-color-4`) the way every other element on the page does.
 *
 * ## The fix under test
 *
 * `primitives/src/top_layer.rs`'s `ensure_top_layer_ink_styles` installs a
 * pair of engine-injected, zero-specificity baselines:
 *
 *   :where([popover]), :where(dialog) { color: inherit; }
 *   :where([popover]) :where(button, input, select, textarea),
 *   :where(dialog) :where(button, input, select, textarea) { color: inherit; }
 *
 * The first fixes the promoted element itself; the second closes the
 * identical defect one DOM level deeper -- native form controls carry their
 * own direct UA `color` too, so the first rule's `inherit` cannot reach one
 * nested inside promoted content (see `date_picker`'s case below).
 * `:where()` contributes zero specificity in both, so neither ever does
 * more than fill a gap where nothing else is declared -- each can
 * out-cascade the UA default (author origin always beats user-agent, at any
 * specificity) but can never out-specificity-fight a component's own
 * `color` rule. `Tooltip`'s `.dx-tooltip-content` (which sets its own,
 * intentionally *inverted* `color: var(--primary-color)` against its dark
 * background) is the load-bearing negative case this file checks: it must
 * keep winning, not fall back to the shared rule's `inherit`.
 *
 * See that function's own doc (`primitives/src/top_layer.rs`) for the full
 * proof that every top-layer consumer in this crate reaches the fix,
 * through one of exactly three hooks (`use_popover_sync`,
 * `use_popover_shown_while_mounted`, `use_dialog_open_driver`) -- this file
 * is the black-box half of that proof: it opens every one of those
 * consumers as an end user would and asserts the *painted* result.
 *
 * ## Method: assert against the resolved token, not a hardcoded literal
 *
 * `resolvedColor()` below reads `--secondary-color-4` (and, for the
 * Tooltip negative case, `--primary-color`) the only way a CSS custom
 * property can be read *as a color*: by assigning it to a probe element's
 * `color` and reading that element's own `getComputedStyle().color` back
 * (`getPropertyValue('--secondary-color-4')` alone would return the raw,
 * unresolved token text -- literally `"var(--dark, #d4d4d4) var(--light,
 * #111)"` per `preview/assets/dx-components-theme.css`, not a color). Every
 * assertion below compares a surface's own computed `color` against that
 * resolved value, so this file survives a theme retune untouched.
 *
 * `--secondary-color-4` is set once, as `color`, on `body`
 * (`preview/assets/main.css`) -- and top-layer promotion (`showPopover()`/
 * `showModal()`) never moves an element in the *DOM tree*, only how it is
 * *painted*; CSS inheritance follows the DOM tree, not paint order. So
 * `color: inherit` on a promoted element resolves through its real DOM
 * ancestors back to `body`'s declaration regardless of which visual layer
 * it paints in -- which is also why this fix applies equally to a fully
 * themed component (`.dx-menubar-content`) and to a bare, unthemed
 * primitive (the `top_layer` fixture's raw `PopoverContent`, used below for
 * the non-modal `Popover` arm): neither declares `color` itself, both climb
 * the same DOM tree to the same `body` rule.
 *
 * ## Coverage
 *
 * Every top-layer surface named in the task, plus several previously-
 * *masked* siblings kept as living proof the fix doesn't regress an
 * already-correct case: `.dx-navbar-item`/`.dx-toast-title`/`.dx-dialog`/
 * `.dx-alert-dialog`/`.dx-sheet` already declare their own `color: var(
 * --secondary-color-4)`; `.dx-tooltip-content` declares its own
 * *intentionally different* `color: var(--primary-color)`; `.dx-select-
 * list`/`.dx-combobox-list` declare their own *intentionally different*
 * `color: var(--secondary-color-1)`. Every one of these is author-origin
 * and already wins over the UA default with no help from this fix -- each
 * is asserted against its OWN actual token (`expectedVar`), not assumed to
 * be the general ink token, so a real regression in any of them still
 * fails this file:
 *   menubar, navbar, dropdown_menu, context_menu, popover (non-modal arm,
 *   via the `top_layer` fixture's raw `PopoverContent` -- the themed
 *   `/component/?name=popover&` demo only exercises the *modal* arm, since
 *   `PopoverRootProps::is_modal` defaults `true`), popover (modal arm),
 *   hover_card, tooltip, date_picker, select, combobox, dialog,
 *   alert_dialog, sheet, toast. Every one of these opens cleanly in this
 *   harness -- none needed to be skipped. (`date_picker`'s text case is a
 *   real `<button>`, exercising `ensure_top_layer_ink_styles`'s second,
 *   form-control-scoped rule rather than the wrapper rule -- see that
 *   surface's own comment below for a separate, pre-existing, unrelated
 *   defect it also surfaced: `Calendar`'s own stylesheet isn't loaded on
 *   this route at all, out of this fix's scope, reported separately.)
 *
 * Each surface is checked in both `colorScheme: 'light'` and `colorScheme:
 * 'dark'` (a separate `test.describe` per scheme, sharing one `run()`) --
 * per the task brief, dark is where this bug is worst (UA `CanvasText`
 * resolves near-white-on-dark in dark mode on some engines, but Chromium's
 * concrete default is a fixed opaque black in both schemes, so the visible
 * failure mode here is "black wrapper text on a dark themed background,"
 * confirmed by execution against this repo's dev server pre-fix).
 */

import { test, expect, type Page, type Locator } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app

const url = (name: string) => `http://127.0.0.1:8080/component/?name=${name}&`;

async function goto(page: Page, name: string) {
  await page.goto(url(name), { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });
}

/**
 * Reads a CSS custom property's *resolved color* -- the only way to read
 * one as a color at all -- by assigning it to a throwaway probe element's
 * `color` and reading that element's own computed style back. See this
 * file's header doc, "Method," for why `getPropertyValue` alone cannot be
 * used here.
 */
async function resolvedColor(page: Page, cssVar: string): Promise<string> {
  return page.evaluate((v) => {
    const probe = document.createElement("span");
    probe.style.color = `var(${v})`;
    document.body.appendChild(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  }, cssVar);
}

type Surface = {
  name: string;
  /** Navigate and drive the surface open. */
  open: (page: Page) => Promise<void>;
  /** The element that actually carries `[popover]` or is the `<dialog>` itself. */
  wrapper: (page: Page) => Locator;
  /** A text-bearing descendant (or the wrapper itself, when text is a direct child). */
  text: (page: Page) => Locator;
  /**
   * The CSS custom property this surface's text should resolve to.
   * Defaults to the app's ink token; Tooltip is the sole, deliberate
   * exception (its own inverted `color` must keep winning).
   */
  expectedVar?: string;
};

const SURFACES: Surface[] = [
  {
    name: "menubar",
    open: async (page) => {
      await goto(page, "menubar");
      await page.getByRole("menuitem", { name: "File" }).click();
      await expect(page.locator('.dx-menubar-content[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-menubar-content[data-state="open"]'),
    text: (page) => page.getByRole("menuitem", { name: "New" }),
  },
  {
    name: "navbar",
    open: async (page) => {
      await goto(page, "navbar");
      await page.getByRole("menuitem", { name: "Inputs" }).hover();
      await expect(page.locator('.dx-navbar-content[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-navbar-content[data-state="open"]'),
    text: (page) => page.getByRole("menuitem", { name: "Calendar" }),
  },
  {
    name: "dropdown_menu",
    open: async (page) => {
      await goto(page, "dropdown_menu");
      await page.getByRole("button", { name: "Open Menu" }).click();
      await expect(page.locator('.dx-dropdown-menu-content[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-dropdown-menu-content[data-state="open"]'),
    text: (page) =>
      page.locator('.dx-dropdown-menu-content[data-state="open"] .dx-dropdown-menu-item').first(),
  },
  {
    name: "context_menu",
    open: async (page) => {
      await goto(page, "context_menu");
      await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
      await expect(page.locator('.dx-context-menu-content[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-context-menu-content[data-state="open"]'),
    text: (page) =>
      page.locator('.dx-context-menu-content[data-state="open"] .dx-context-menu-item').first(),
  },
  {
    // Non-modal `Popover` arm (`popover="auto"`, not a modal `<dialog>`).
    // The themed `/component/?name=popover&` demo only ever exercises the
    // *modal* arm (`is_modal` defaults `true`) -- the `top_layer` fixture's
    // `clip-popover-*` instance is a real, deliberately *unthemed*
    // `is_modal: false` `PopoverRoot`/`PopoverContent`, which doubles as
    // proof this fix works with zero component-authored CSS in play at
    // all, relying only on the DOM-tree inheritance chain up to `body`.
    name: "popover (non-modal arm, unthemed)",
    open: async (page) => {
      await goto(page, "top_layer");
      await page.locator("#clip-popover-trigger").click();
      await expect(page.locator("#clip-popover-content")).toBeVisible();
    },
    wrapper: (page) => page.locator("#clip-popover-content"),
    text: (page) => page.locator("#clip-popover-content"),
  },
  {
    // Modal `Popover` arm: a real `<dialog>` + `showModal()` on the web arm
    // (native-dialog engine migration) -- reaches the fix through
    // `use_dialog_open_driver`, not `use_popover_sync`.
    name: "popover (modal arm)",
    open: async (page) => {
      await goto(page, "popover");
      await page.getByRole("button", { name: "Show Popover" }).click();
      await expect(page.locator('.dx-popover-content[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-popover-content[data-state="open"]'),
    // The demo's "Delete Item?" heading -- this file's own second instance
    // fix gives it the `.dx-popover-content-title` class so it actually
    // picks up that class's declared color (a separate, non-inherited
    // fix); still asserted against the shared ink token here since that's
    // also `.dx-popover-content-title`'s own declared value.
    text: (page) => page.getByText("Delete Item?"),
  },
  {
    name: "hover_card",
    open: async (page) => {
      await goto(page, "hover_card");
      await page.getByRole("button", { name: "Dioxus" }).hover();
      await expect(page.locator(".dx-hover-card-content")).toBeVisible();
    },
    wrapper: (page) => page.locator(".dx-hover-card-content"),
    text: (page) => page.getByText("Dioxus is", { exact: false }),
  },
  {
    // The load-bearing negative case: `.dx-tooltip-content` declares its
    // OWN `color: var(--primary-color)` (intentionally inverted against its
    // dark background) -- the shared `:where()` rule must never override
    // it. `expectedVar` below is `--primary-color`, not the ink token, on
    // purpose.
    name: "tooltip",
    open: async (page) => {
      await goto(page, "tooltip");
      await page.locator("#component-preview-frame").first().getByText("Rich content").hover();
      await expect(page.getByRole("tooltip")).toBeVisible();
    },
    wrapper: (page) => page.locator(".dx-tooltip-content"),
    text: (page) => page.getByText("Tooltip title"),
    expectedVar: "--primary-color",
  },
  {
    // Wrapper: `.dx-date-picker-popover-content`, a real `[popover]`
    // element with no `color` of its own -- this surface's direct instance
    // of the bug. Text: a calendar day-grid cell -- a real `<button>`, so
    // it exercises the *second*, form-control-scoped `:where()` rule
    // (`ensure_top_layer_ink_styles`'s own doc, "one level deeper"), not
    // just the wrapper rule. Both matter here specifically: confirmed by
    // execution this session that `Calendar`'s own stylesheet is not even
    // loaded on this route (`document.styleSheets` has no `calendar` entry)
    // -- a separate, pre-existing composition defect, out of this fix's
    // scope (see this session's report) -- so without the form-control rule
    // these buttons would have no author `color` of any kind to fall back
    // to, only the UA default this file exists to catch.
    name: "date_picker",
    open: async (page) => {
      await goto(page, "date_picker");
      await page.getByRole("button", { name: "Show Calendar" }).first().click();
      await expect(page.locator(".dx-date-picker-popover-content").first()).toBeVisible();
    },
    wrapper: (page) => page.locator(".dx-date-picker-popover-content").first(),
    text: (page) => page.locator('.dx-calendar-grid-cell[data-month="current"]').first(),
  },
  {
    // `.dx-select-list` declares its own `color: var(--secondary-color-1)`
    // directly (author origin, already winning over the UA default with no
    // help needed from this file's fix) -- a deliberate, different token
    // from the general ink token every other surface in this file uses, not
    // an instance of the bug. `expectedVar` reflects that; this case exists
    // to prove the shared `:where()` rule leaves it alone, the same
    // load-bearing negative shape as the Tooltip case above.
    name: "select",
    open: async (page) => {
      await goto(page, "select");
      await page
        .getByRole("button")
        .filter({ hasText: /Select an option|Apple|Banana/ })
        .click();
      await expect(page.locator('.dx-select-list[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-select-list[data-state="open"]'),
    text: (page) => page.getByRole("option", { name: "Apple" }),
    expectedVar: "--secondary-color-1",
  },
  {
    // Same shape as `select` immediately above: `.dx-combobox-list` also
    // declares its own `color: var(--secondary-color-1)` directly.
    name: "combobox",
    open: async (page) => {
      await goto(page, "combobox");
      const input = page.getByRole("combobox", { name: "Select framework" });
      await input.focus();
      await page.keyboard.press("ArrowDown");
      await expect(page.locator('.dx-combobox-list[data-state="open"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-combobox-list[data-state="open"]'),
    text: (page) => page.locator('.dx-combobox-list[data-state="open"] .dx-combobox-option').first(),
    expectedVar: "--secondary-color-1",
  },
  {
    name: "dialog",
    open: async (page) => {
      await goto(page, "dialog");
      await page.getByRole("button", { name: "Show Dialog" }).click();
      await expect(page.getByRole("dialog").filter({ hasText: "Item information" })).toBeVisible();
    },
    wrapper: (page) => page.getByRole("dialog").filter({ hasText: "Item information" }),
    text: (page) => page.locator(".dx-dialog-title").filter({ hasText: "Item information" }),
  },
  {
    name: "alert_dialog",
    open: async (page) => {
      await goto(page, "alert_dialog");
      await page.getByRole("button", { name: "Show Alert Dialog" }).click();
      await expect(page.getByRole("alertdialog")).toBeVisible();
    },
    wrapper: (page) => page.getByRole("alertdialog"),
    text: (page) => page.locator(".dx-alert-dialog-title"),
  },
  {
    // `[data-slot="sheet-root"]`/`.dx-sheet-root` is NOT the promoted
    // element -- `Sheet` (`preview/src/components/sheet/component.rs`)
    // passes that class to `dialog::DialogRoot`, which renders it onto a
    // plain, always-in-flow context-provider `div` (`primitives/src/
    // dialog.rs`'s `DialogRoot`); the real `<dialog>` (rendered by
    // `DialogContent`'s web arm) carries `.dx-sheet` instead. Confirmed by
    // execution (this session's red run): asserting against `.dx-sheet-root`
    // silently passed even pre-fix, because a plain `div` was never subject
    // to the `[popover]`/`dialog` UA default in the first place -- exactly
    // the false-negative this file's own header doc warns against
    // ("assert against the resolved token," not against whichever element
    // happens to be nearby).
    name: "sheet",
    open: async (page) => {
      await goto(page, "sheet");
      await page.getByRole("button", { name: "Right" }).click();
      await expect(page.locator('.dx-sheet[data-side="right"]')).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-sheet[data-side="right"]'),
    text: (page) => page.locator(".dx-sheet-title"),
  },
  {
    // `.dx-toast-container[popover]` is a zero-height flex row by design
    // (its `.dx-toast` children stack via `position: absolute` -- see
    // `preview/src/components/toast/style.css`) -- confirmed by execution
    // this session: `getBoundingClientRect()` is genuinely `{width: 288,
    // height: 0}` even once a toast is showing, which fails Playwright's
    // `toBeVisible()` (a real, on-screen box is part of that check) despite
    // the container being exactly as intended. So this surface's "opened"
    // wait keys off the visible toast *text* (`.dx-toast-title`), not the
    // (by-design zero-height) container box -- `toHaveCSS` below still
    // reads the container's own computed `color` regardless of its size.
    name: "toast",
    open: async (page) => {
      await goto(page, "toast");
      await page.getByRole("button", { name: "Info (60s)" }).click();
      await expect(page.locator(".dx-toast-title")).toBeVisible();
    },
    wrapper: (page) => page.locator('.dx-toast-container[popover]'),
    text: (page) => page.locator(".dx-toast-title"),
  },
];

function runSurfaceChecks() {
  for (const surface of SURFACES) {
    test(surface.name, async ({ page }) => {
      await surface.open(page);
      const expected = await resolvedColor(page, surface.expectedVar ?? "--secondary-color-4");
      await expect(
        surface.wrapper(page),
        `${surface.name}: content wrapper's color must resolve to ${surface.expectedVar ?? "--secondary-color-4"}, not the UA [popover]/dialog default`,
      ).toHaveCSS("color", expected);
      await expect(
        surface.text(page),
        `${surface.name}: text must resolve to ${surface.expectedVar ?? "--secondary-color-4"}, not the UA [popover]/dialog default`,
      ).toHaveCSS("color", expected);
    });
  }
}

test.describe("light color scheme", () => {
  test.use({ colorScheme: "light" });
  runSurfaceChecks();
});

test.describe("dark color scheme (worst case for this bug)", () => {
  test.use({ colorScheme: "dark" });
  runSurfaceChecks();
});
