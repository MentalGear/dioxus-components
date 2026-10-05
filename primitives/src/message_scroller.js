// Message scroller controller -- one instance per `MessageScrollerProvider`.
//
// This file is the body of a `document::eval` async function (see
// `MESSAGE_SCROLLER_JS` in `message_scroller.rs`): `dioxus.recv()` /
// `dioxus.send()` are its only bridge to Rust, and a top-level `return` /
// `await` is legal.
//
// Provenance: a port of the controller in `@shadcn/react`'s
// `message-scroller` (`use-message-scroller-controller.ts`,
// `use-message-scroller-commands.ts`, `geometry.ts`), shadcn/ui, MIT
// licence. The mode machine, the tail spacer, the anchoring maths, prepend
// preservation and the opening-position rules are theirs; what changed is
// the host: React refs and `useSyncExternalStore` are replaced by DOM
// discovery (`data-message-*` attributes), and state is written back as
// `data-*` attributes. Stage 1 omits the visibility store
// (`useMessageScrollerVisibility`, an IntersectionObserver).
//
// Deliberate deviations from the reference, each a small refinement:
//   * a wheel / touch / key gesture that cannot move the reader away from the
//     live edge (wheel down while already at the end) does not release
//     follow-bottom, because nothing moved;
//   * keys typed into an editable control inside a row are not scroll intent;
//   * the "jump to latest" button moves focus to the viewport instead of
//     dropping it on <body> when it turns inert;
//   * `smooth` honours `prefers-reduced-motion`;
//   * the opening position waits until the viewport really is a scroll
//     container (its stylesheet may arrive after this script);
//   * following is (re-)armed only by the reader scrolling DOWN to the end, not
//     by merely being at the end (a wheel release is not undone by a state
//     commit that lands before the scroll animation has moved the viewport);
//   * a row counts as a new anchor only when it was not one already (see
//     `takeFreshAnchor`), so swapping a typing marker for its reply never
//     yanks the reader back to a user message that was there all along;
//   * a far jump re-aims for a moment while the rows around the target render
//     at their real size (see `reaimJump`).
//
// Virtualization (`cfg.virtualize`): "none" renders every row always;
// "content-visibility" (the default) lets the stylesheet skip off-screen rows
// with `content-visibility: auto` and keeps three groups of rows rendered
// through a controller-owned `data-keep-rendered` attribute -- the rows at the
// live edge (so streaming growth and the end geometry are always exact), the
// row holding focus and the rows holding the selection's ends.
//
// A row is either a direct child of the content, or one of the rows of a
// "chunk": a `[data-message-scroller-chunk]` wrapper (`MessageScrollerRows`)
// that the stylesheet skips as a unit, because one skippable element per row
// costs the main thread time proportional to the row count on every scroll
// frame (see `primitives/src/virtual/cv_chunks.rs`). The unit that is kept
// rendered, and the unit whose size the browser remembers, is therefore the
// chunk. A row inside a skipped chunk has no box of its own, and reading its
// geometry makes the browser lay the whole chunk out, so geometry is only read
// from rows that must be read (the last row, the rows around the viewport, a
// jump target), never swept over every row.
//
// Hot-path discipline: everything that runs at scroll or token frequency
// lives here. Rust receives one message when the published
// `{ start, end }` scrollability CHANGES, never per scroll event or token.
//
// Messages in  (Rust -> JS): { t: "init" | "config" | "end" | "start" |
//                              "message" | "sync" | "teardown", ... }
// Messages out (JS -> Rust): ["scrollable", start, end]

const EPS = 0.5;
const AUTOSCROLL_CLEAR_MS = 180;
const OPEN_POLL_FRAMES = 120;
const JUMP_SETTLE_MS = 1000;
const USER_KEYS = new Set(["ArrowDown", "ArrowUp", "End", "Home", "PageDown", "PageUp", " "]);
const KEYS_TOWARD_END = new Set(["ArrowDown", "End", "PageDown", " "]);
const LIVE_EDGE_ROWS = 8;
const CHUNK = "data-message-scroller-chunk";
const EDITABLE = "input, textarea, select, [contenteditable]:not([contenteditable='false'])";

// Provider props. `init` is always the first message Rust sends.
const cfg = await dioxus.recv();

cfg.virtualize = cfg.virtualize || "content-visibility";

const nextFrame = () => new Promise((resolve) => requestAnimationFrame(() => resolve()));
const rootSelector = '[data-message-scroller-root="' + CSS.escape(cfg.id) + '"]';
let root = document.querySelector(rootSelector);
for (let i = 0; !root && i < 60; i++) {
  await nextFrame();
  root = document.querySelector(rootSelector);
}

// Elements (bound lazily: the content may mount after the root).
let viewport = null;
let content = null;
let spacer = null;
let bound = false;

// Mode machine: "following-bottom" | "free-scrolling" | "anchored-to-message"
// | "settling-jump".
let mode = cfg.autoScroll ? "following-bottom" : "free-scrolling";
let autoscrolling = false;
let autoscrollingTimer = 0;
let itemCount = 0;
let firstItem = null;
let lastScrollTop = 0;
let streamingTurn = null;
let spacerHeight = 0;
let spacerGap = 0;
let prependRestore = null;
let defaultApplied = false;
let pendingJump = null;
let stateFrame = 0;
let resizeFrame = 0;
let openFrame = 0;
let openFrames = 0;
// row -> whether it was a scroll anchor the last time the content changed. A row
// only counts as a *new* anchor when it was not already an anchor: an anchor
// that was there all along (the open transcript's user messages) must never
// pull the reader back when an unrelated row is swapped (a typing marker
// replaced by its reply keeps the row count unchanged).
const anchorSeen = new WeakMap();
let published = null;
let writtenScrollable = null;
let writtenAutoscrolling = null;
let touchY = null;
// The row a message jump is settling on, re-aimed while its neighbourhood
// renders (see `reaimJump`).
let jumpTarget = null;
let kept = new Set();
let keepFrame = 0;

const reducedMotion = () =>
  !!(window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches);
const px = (value) => {
  const n = parseFloat(value);
  return Number.isFinite(n) ? n : 0;
};

// --------------------------------------------------------- virtualization

// The direct child of the content that holds `node`, if any: the row, or the
// chunk holding the row. This is the element `content-visibility` skips.
function rowOf(node) {
  let el = node && node.nodeType === 1 ? node : node && node.parentElement;
  while (el && el.parentElement !== content) el = el.parentElement;
  return el && el !== spacer ? el : null;
}

// The skip units (rows, or chunks) holding the last `rows` rows.
function tailUnits(rows) {
  const units = [];
  let n = 0;
  for (let i = content.children.length - 1; i >= 0 && n < rows; i--) {
    const child = content.children[i];
    if (child === spacer || !(child instanceof HTMLElement)) continue;
    units.push(child);
    n += child.hasAttribute(CHUNK) ? child.childElementCount : 1;
  }
  return units;
}

// Rows that must never be skipped by `content-visibility`: the live edge, the
// row with focus, and the rows holding the ends of the selection (each as the
// skip unit that holds it).
function syncKeepRendered() {
  if (!bound) return;
  const want = new Set();
  if (cfg.virtualize === "content-visibility") {
    for (const unit of tailUnits(LIVE_EDGE_ROWS)) want.add(unit);
    const active = document.activeElement;
    if (active && content.contains(active)) {
      const row = rowOf(active);
      if (row) want.add(row);
    }
    const selection = document.getSelection();
    if (selection && selection.rangeCount && !selection.isCollapsed) {
      for (const node of [selection.anchorNode, selection.focusNode]) {
        if (node && content.contains(node)) {
          const row = rowOf(node);
          if (row) want.add(row);
        }
      }
    }
  }
  for (const el of kept) if (!want.has(el)) el.removeAttribute("data-keep-rendered");
  for (const el of want) if (!kept.has(el)) el.setAttribute("data-keep-rendered", "");
  kept = want;
}

function scheduleKeepSync() {
  if (keepFrame || !bound) return;
  keepFrame = requestAnimationFrame(() => {
    keepFrame = 0;
    syncKeepRendered();
  });
}

// ---------------------------------------------------------------- geometry

// Every row, in order, whether it is a direct child of the content or one of
// the rows of a chunk. Reads no geometry.
function items() {
  const out = [];
  for (const child of content.children) {
    if (child === spacer || !(child instanceof HTMLElement)) continue;
    if (child.hasAttribute(CHUNK)) {
      for (const row of child.children) if (row instanceof HTMLElement) out.push(row);
    } else {
      out.push(child);
    }
  }
  return out;
}

// The last row, without walking the others.
function lastItem() {
  for (let i = content.children.length - 1; i >= 0; i--) {
    const child = content.children[i];
    if (child === spacer || !(child instanceof HTMLElement)) continue;
    if (!child.hasAttribute(CHUNK)) return child;
    if (child.lastElementChild instanceof HTMLElement) return child.lastElementChild;
  }
  return null;
}

function blockPadding(el) {
  const s = getComputedStyle(el);
  return {
    end: px(s.paddingBlockEnd || s.paddingBottom),
    start: px(s.paddingBlockStart || s.paddingTop),
  };
}

function flexGap(el) {
  const s = getComputedStyle(el);
  return px(s.rowGap === "normal" ? s.gap : s.rowGap);
}

// Bottom edge of the real rows (the tail spacer excluded), in scroll
// coordinates, plus the content's end padding.
function contentBottom() {
  const pad = blockPadding(content);
  const top = viewport.getBoundingClientRect().top;
  const scrollTop = viewport.scrollTop;
  let bottom = pad.start + pad.end;
  // Rows flow in order, so the last one is the lowest; asking every row for its
  // box would lay out every skipped chunk.
  const last = lastItem();
  if (last) bottom = Math.max(bottom, last.getBoundingClientRect().bottom - top + scrollTop + pad.end);
  return bottom;
}

function maxScrollTop() {
  return Math.max(0, viewport.scrollHeight - viewport.clientHeight);
}

function scrollableState() {
  return {
    start: viewport.scrollTop > cfg.edge,
    end: contentBottom() - viewport.scrollTop - viewport.clientHeight > cfg.edge,
  };
}

function elementTop(el) {
  return el.getBoundingClientRect().top - viewport.getBoundingClientRect().top + viewport.scrollTop;
}

function elementViewportTop(el) {
  return el.getBoundingClientRect().top - viewport.getBoundingClientRect().top;
}

function elementScrollTop(el, align, margin) {
  const top = elementTop(el);
  const height = el.getBoundingClientRect().height;
  const pad = blockPadding(content);
  if (align === "center") {
    const inset = Math.max(0, viewport.clientHeight - pad.start - pad.end);
    return top - pad.start - (inset - height) / 2 - margin;
  }
  if (align === "end") {
    return top - viewport.clientHeight + height + pad.end + margin;
  }
  if (align === "nearest") {
    const bottom = top + height;
    const viewTop = viewport.scrollTop + pad.start;
    const viewBottom = viewport.scrollTop + viewport.clientHeight - pad.end;
    if (top >= viewTop && bottom <= viewBottom) return viewport.scrollTop;
    if (top < viewTop) return top - pad.start - margin;
    return bottom - viewport.clientHeight + pad.end + margin;
  }
  return top - pad.start - margin;
}

function lastAnchor(list) {
  for (let i = list.length - 1; i >= 0; i--) {
    if (list[i].dataset.scrollAnchor === "true") return list[i];
  }
  return null;
}

function newAnchor(list, previousCount) {
  for (let i = previousCount; i < list.length; i++) {
    if (list[i].dataset.scrollAnchor === "true") return list[i];
  }
  return null;
}

function manyNewAnchors(list, previousCount) {
  let count = 0;
  for (let i = previousCount; i < list.length; i++) {
    if (list[i].dataset.scrollAnchor === "true" && ++count > 1) return true;
  }
  return false;
}

// The first row that is an anchor now and was not one before (a new row, or a
// row whose `data-scroll-anchor` flipped). Records every row's state.
function takeFreshAnchor(list) {
  let fresh = null;
  for (const item of list) {
    const isAnchor = item.dataset.scrollAnchor === "true";
    if (isAnchor && !fresh && anchorSeen.get(item) !== true) fresh = item;
    anchorSeen.set(item, isAnchor);
  }
  return fresh;
}

function firstVisibleItem() {
  const box = viewport.getBoundingClientRect();
  const inView = (el) => {
    const r = el.getBoundingClientRect();
    return r.bottom > box.top && r.top < box.bottom;
  };
  for (const child of content.children) {
    if (child === spacer || !(child instanceof HTMLElement)) continue;
    // A chunk's own box is always laid out; only look inside one that is in view
    // (and so rendered).
    if (!inView(child)) continue;
    if (child.hasAttribute(CHUNK)) {
      for (const row of child.children) {
        if (row instanceof HTMLElement && row.dataset.messageId && inView(row)) return row;
      }
    } else if (child.dataset.messageId) {
      return child;
    }
  }
  return null;
}

function atEnd() {
  return viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= cfg.edge;
}

// The viewport is only a scroll container once its stylesheet applies.
function ready() {
  if (!viewport || !viewport.isConnected) return false;
  const overflow = getComputedStyle(viewport).overflowY;
  return (overflow === "auto" || overflow === "scroll") && viewport.clientHeight > 0;
}

// ------------------------------------------------------------ state output

function updateButtons(state) {
  for (const button of root.querySelectorAll("[data-message-scroller-button]")) {
    const active = button.getAttribute("data-direction") === "start" ? state.start : state.end;
    button.setAttribute("data-active", active ? "true" : "false");
    button.toggleAttribute("inert", !active);
    if (active) button.removeAttribute("tabindex");
    else button.setAttribute("tabindex", "-1");
  }
}

function writeAttributes(state) {
  const scrollable = [state.start && "start", state.end && "end"].filter(Boolean).join(" ");
  if (scrollable !== writtenScrollable || autoscrolling !== writtenAutoscrolling) {
    writtenScrollable = scrollable;
    writtenAutoscrolling = autoscrolling;
    for (const el of [root, viewport]) {
      if (!el) continue;
      if (scrollable) el.setAttribute("data-scrollable", scrollable);
      else el.removeAttribute("data-scrollable");
      el.toggleAttribute("data-autoscrolling", autoscrolling);
    }
  }
}

// `force` re-writes the buttons even when the state did not change (a button
// that mounted after the last change).
function publish(state, force) {
  writeAttributes(state);
  const changed = !published || published.start !== state.start || published.end !== state.end;
  if (changed || force) updateButtons(state);
  if (changed) {
    published = { start: state.start, end: state.end };
    dioxus.send(["scrollable", state.start, state.end]);
  }
}

function reconcileFollowMode(state) {
  const scrollTop = viewport.scrollTop;
  // Content growing past the live edge also reads as "not at the end", but
  // only a scrollbar drag moves scrollTop up: growth must not release follow.
  const scrolledUp = scrollTop < lastScrollTop - EPS;
  // Following is only (re-)armed by the reader moving DOWN to the end. Arming
  // on "is at the end" alone races a wheel gesture: the wheel handler releases,
  // then a state commit lands before the scroll animation has moved the
  // viewport, sees it still at the end, and would re-arm.
  const scrolledDown = scrollTop > lastScrollTop + EPS;
  lastScrollTop = scrollTop;
  if (
    cfg.autoScroll &&
    !state.end &&
    scrolledDown &&
    mode !== "settling-jump" &&
    mode !== "anchored-to-message"
  ) {
    mode = "following-bottom";
  } else if (mode === "following-bottom" && state.end && scrolledUp && !autoscrolling) {
    mode = "free-scrolling";
  }
}

function commitScrollState(force) {
  if (!bound) return;
  const next = scrollableState();
  reconcileFollowMode(next);
  // While following, the scroller is already closing every gap a streamed
  // chunk opens; publishing it would strobe the jump button once per chunk.
  publish(
    mode === "following-bottom" && cfg.autoScroll ? { start: next.start, end: false } : next,
    force,
  );
}

function scheduleStateCommit() {
  if (stateFrame) return;
  stateFrame = requestAnimationFrame(() => {
    stateFrame = 0;
    commitScrollState();
  });
}

// ---------------------------------------------------------------- commands

function setAutoScrolling(value) {
  if (autoscrollingTimer) {
    clearTimeout(autoscrollingTimer);
    autoscrollingTimer = 0;
  }
  if (autoscrolling !== value) {
    autoscrolling = value;
    commitScrollState();
  }
  if (value) {
    autoscrollingTimer = setTimeout(() => {
      autoscrollingTimer = 0;
      autoscrolling = false;
      commitScrollState();
    }, AUTOSCROLL_CLEAR_MS);
  }
}

function setTailSpacerHeight(height) {
  if (!spacer) return;
  const next = Math.max(0, Math.ceil(height));
  if (spacerHeight === next) return;
  spacerHeight = next;
  spacer.hidden = next === 0;
  spacer.style.height = next + "px";
  spacer.style.marginTop = next > 0 ? -spacerGap + "px" : "";
}

function scrollToPosition(top, behavior, autoscroll) {
  const next = Math.max(0, top);
  if (Math.abs(viewport.scrollTop - next) <= EPS) {
    viewport.scrollTop = next;
    commitScrollState();
    return;
  }
  if (autoscroll) setAutoScrolling(true);
  viewport.scrollTo({ top: next, behavior: behavior === "smooth" && reducedMotion() ? "auto" : behavior });
  scheduleStateCommit();
}

function scrollToStart(behavior) {
  if (!bound) return false;
  setTailSpacerHeight(0);
  streamingTurn = null;
  jumpTarget = null;
  mode = "free-scrolling";
  scrollToPosition(0, behavior || "auto", false);
  return true;
}

function scrollToEnd(behavior) {
  if (!bound) return false;
  setTailSpacerHeight(0);
  streamingTurn = null;
  jumpTarget = null;
  mode = cfg.autoScroll ? "following-bottom" : "free-scrolling";
  scrollToPosition(maxScrollTop(), behavior || "auto", true);
  return true;
}

function scrollToElement(el, align, behavior, margin, keepPeek) {
  if (!bound || !content.contains(el)) return false;
  const scrollMargin = (margin == null ? cfg.margin : margin) + (keepPeek ? cfg.peek : 0);
  const top = elementScrollTop(el, align || "start", scrollMargin);
  setTailSpacerHeight(top + viewport.clientHeight - contentBottom());
  // Seed the prepend anchor with the jump target so a prepend landing before
  // this scroll settles still preserves the row we jumped to.
  prependRestore = { element: el, viewportTop: elementViewportTop(el) };
  mode = keepPeek ? "anchored-to-message" : "settling-jump";
  streamingTurn = keepPeek ? el : null;
  // A far jump under `content-visibility` aims at rows whose heights are only
  // estimates; they render at their real size as they arrive, which moves the
  // target. Keep re-aiming for a short window (smooth jumps animate instead).
  jumpTarget =
    !keepPeek && behavior !== "smooth"
      ? { el, align: align || "start", margin, until: performance.now() + JUMP_SETTLE_MS }
      : null;
  scrollToPosition(top, behavior || "auto", false);
  return true;
}

// Re-aim a settling jump when the layout under it changed. Returns true when it
// moved the viewport.
function reaimJump() {
  const target = jumpTarget;
  if (!target) return false;
  if (mode !== "settling-jump" || !target.el.isConnected || performance.now() > target.until) {
    jumpTarget = null;
    return false;
  }
  const margin = target.margin == null ? cfg.margin : target.margin;
  const top = Math.max(0, elementScrollTop(target.el, target.align, margin));
  if (Math.abs(viewport.scrollTop - top) <= EPS) return false;
  const until = target.until;
  scrollToElement(target.el, target.align, "auto", target.margin, false);
  if (jumpTarget) jumpTarget.until = until;
  return true;
}

function reanchor() {
  if (!streamingTurn || !streamingTurn.isConnected || mode !== "anchored-to-message") return false;
  return scrollToElement(streamingTurn, "start", "auto", null, true);
}

function findMessage(id) {
  for (const item of items()) {
    if (item.dataset.messageId === id) return item;
  }
  return null;
}

function markApplied() {
  defaultApplied = true;
  clearPending();
}

function clearPending() {
  root.removeAttribute("data-pending-scroll");
  if (viewport) viewport.removeAttribute("data-pending-scroll");
}

function jumpToMessage(id, align, behavior) {
  const el = bound ? findMessage(id) : null;
  if (!el) {
    if (itemCount === 0) {
      pendingJump = { id, align, behavior };
      markApplied();
    }
    return;
  }
  markApplied();
  if (scrollToElement(el, align, behavior, null, false)) pendingJump = null;
  else pendingJump = { id, align, behavior };
}

function flushPendingJump() {
  if (!pendingJump) return false;
  const el = findMessage(pendingJump.id);
  if (!el) return false;
  if (!scrollToElement(el, pendingJump.align, pendingJump.behavior, null, false)) return false;
  pendingJump = null;
  markApplied();
  return true;
}

// ------------------------------------------------------- opening position

function applyDefaultScrollPosition() {
  if (defaultApplied || itemCount === 0 || !ready()) return false;
  let handled;
  if (cfg.defaultScrollPosition === "last-anchor") {
    const anchor = lastAnchor(items());
    if (!anchor) {
      handled = scrollToEnd("auto");
    } else {
      // A short last turn already fits below its anchor, so opening at the
      // end shows it whole without a blank gap underneath.
      const fits = contentBottom() - elementTop(anchor) <= viewport.clientHeight;
      handled = fits ? scrollToEnd("auto") : scrollToElement(anchor, "start", "auto", null, true);
    }
  } else if (cfg.defaultScrollPosition === "start") {
    handled = scrollToStart("auto");
  } else {
    handled = scrollToEnd("auto");
  }
  if (!handled) return false;
  markApplied();
  return true;
}

// Poll for readiness: an opening position computed before the viewport's own
// stylesheet applies would land at 0 and then be frozen there.
function pollOpen() {
  if (openFrame || defaultApplied) return;
  openFrame = requestAnimationFrame(() => {
    openFrame = 0;
    if (defaultApplied) return;
    if (applyDefaultScrollPosition()) {
      capturePrependAnchor();
      return;
    }
    if (++openFrames >= OPEN_POLL_FRAMES) {
      // Give up hiding the transcript; a later resize can still apply it.
      clearPending();
      return;
    }
    pollOpen();
  });
}

// ----------------------------------------------------------------- prepend

function capturePrependAnchor() {
  if (!bound) return;
  const anchor = firstVisibleItem();
  prependRestore = anchor ? { element: anchor, viewportTop: elementViewportTop(anchor) } : null;
}

function restorePrependedAnchor() {
  const anchor = prependRestore;
  if (!anchor || !anchor.element.isConnected) return false;
  // Compare viewport-relative positions: native scroll anchoring leaves the
  // viewport-relative position unchanged, so this is a no-op where the
  // browser already handled the prepend and only corrects where it did not.
  const delta = elementViewportTop(anchor.element) - anchor.viewportTop;
  if (Math.abs(delta) <= EPS) return false;
  viewport.scrollTop += delta;
  anchor.viewportTop = elementViewportTop(anchor.element);
  scheduleStateCommit();
  return true;
}

// ----------------------------------------------------------------- changes

function handleContentChange() {
  if (!bound) return;
  observeChunks();
  // Before any geometry is read: the live-edge rows must be rendered at their
  // real size, not at the skipped estimate.
  syncKeepRendered();
  const list = items();
  const previousCount = itemCount;
  const previousFirst = firstItem;
  itemCount = list.length;
  firstItem = list[0] || null;
  const freshAnchor = takeFreshAnchor(list);

  const reconcile = () => {
    if (flushPendingJump()) return;
    if (previousCount === 0) {
      if (list.length > 0 && !defaultApplied && !ready()) {
        pollOpen();
        return;
      }
      if (applyDefaultScrollPosition()) return;
      if (list.length > 0 && cfg.autoScroll && scrollToEnd("auto")) return;
      commitScrollState();
      return;
    }
    const preserve = viewport.getAttribute("data-preserve-scroll-on-prepend") !== "false";
    const firstIndex = previousFirst ? list.indexOf(previousFirst) : -1;
    if (preserve && firstIndex > 0) {
      // Prepended rows are not new appends.
      restorePrependedAnchor();
      return;
    }
    if (list.length > previousCount) {
      const anchor = newAnchor(list, previousCount);
      if (anchor) {
        // A burst of several anchored turns while following keeps following
        // the end instead of yanking back to the first of the batch.
        if (cfg.autoScroll && mode === "following-bottom" && manyNewAnchors(list, previousCount)) {
          scrollToEnd("auto");
          return;
        }
        scrollToElement(anchor, "start", "auto", null, true);
        return;
      }
    }
    if (list.length === previousCount && freshAnchor) {
      // A row became an anchor in place (or replaced another row).
      scrollToElement(freshAnchor, "start", "auto", null, true);
      return;
    }
    if (mode === "following-bottom" && cfg.autoScroll) scrollToEnd("auto");
    else commitScrollState();
  };

  reconcile();
  capturePrependAnchor();
}

function handleResize() {
  if (!bound) return;
  if (!defaultApplied && itemCount > 0 && applyDefaultScrollPosition()) return;
  if (mode === "following-bottom" && cfg.autoScroll) {
    scrollToEnd("auto");
    return;
  }
  if (reaimJump()) return;
  // Hold the anchored turn in place while content below it resizes (a reply
  // streaming in, a transient marker collapsing); otherwise the browser
  // clamps scrollTop and the turn drops.
  const previousSpacer = spacerHeight;
  if (reanchor()) {
    // The reply consumes the tail spacer as it grows. Once it is gone the
    // reply fills the viewport, the reader is at the live edge, and follow
    // takes over from the anchor hold (a turn taller than the viewport,
    // placed with no spacer, stays held).
    if (cfg.autoScroll && previousSpacer > 0 && spacerHeight === 0) scrollToEnd("auto");
    return;
  }
  scheduleStateCommit();
}

function userScrollIntent() {
  if (mode === "following-bottom" || mode === "anchored-to-message" || mode === "settling-jump") {
    // A deliberate gesture releases follow, turn anchoring and an in-flight
    // jump, so re-pinning never fights the reader.
    streamingTurn = null;
    jumpTarget = null;
    mode = "free-scrolling";
  }
}

// ------------------------------------------------------------------ events

function onScroll() {
  commitScrollState();
  capturePrependAnchor();
}

function onWheel(event) {
  // Wheeling down at the live edge cannot move the reader: not an escape.
  if (event.deltaY > 0 && atEnd()) return;
  userScrollIntent();
}

function onTouchStart(event) {
  touchY = event.touches.length ? event.touches[0].clientY : null;
}

function onTouchMove(event) {
  const y = event.touches.length ? event.touches[0].clientY : null;
  // A finger moving up scrolls toward the end.
  if (touchY !== null && y !== null && y < touchY && atEnd()) return;
  if (y !== null) touchY = y;
  userScrollIntent();
}

function onKeyDown(event) {
  if (!USER_KEYS.has(event.key)) return;
  if (event.target instanceof Element && event.target.closest(EDITABLE)) return;
  if (KEYS_TOWARD_END.has(event.key) && !event.shiftKey && atEnd()) return;
  userScrollIntent();
}

const resizeObserver = new ResizeObserver(() => {
  cancelAnimationFrame(resizeFrame);
  // Coalesce into a frame: handleResize mutates the spacer inside the
  // observed content, and resizing an observed element during delivery
  // raises "ResizeObserver loop completed with undelivered notifications".
  resizeFrame = requestAnimationFrame(handleResize);
});
const mutationObserver = new MutationObserver(() => handleContentChange());

// Rows are added to and removed from the chunks, not only from the content.
const observedChunks = new WeakSet();
function observeChunks() {
  for (const child of content.children) {
    if (child instanceof HTMLElement && child.hasAttribute(CHUNK) && !observedChunks.has(child)) {
      observedChunks.add(child);
      mutationObserver.observe(child, { childList: true });
    }
  }
}

function bind() {
  if (bound || !root) return bound;
  const vp = root.querySelector("[data-message-scroller-viewport]");
  const ct = vp && vp.querySelector("[data-message-scroller-content]");
  if (!vp || !ct) return false;
  viewport = vp;
  content = ct;
  spacer = ct.querySelector(":scope > [data-message-scroller-spacer]");
  spacerGap = flexGap(ct);
  viewport.addEventListener("scroll", onScroll, { passive: true });
  viewport.addEventListener("wheel", onWheel, { passive: true });
  viewport.addEventListener("touchstart", onTouchStart, { passive: true });
  viewport.addEventListener("touchmove", onTouchMove, { passive: true });
  viewport.addEventListener("keydown", onKeyDown);
  viewport.addEventListener("focusin", scheduleKeepSync);
  viewport.addEventListener("focusout", scheduleKeepSync);
  document.addEventListener("selectionchange", scheduleKeepSync);
  resizeObserver.observe(viewport);
  resizeObserver.observe(content);
  mutationObserver.observe(content, { childList: true });
  observeChunks();
  bound = true;
  return true;
}

function teardown() {
  cancelAnimationFrame(stateFrame);
  cancelAnimationFrame(resizeFrame);
  cancelAnimationFrame(openFrame);
  cancelAnimationFrame(keepFrame);
  clearTimeout(autoscrollingTimer);
  document.removeEventListener("selectionchange", scheduleKeepSync);
  resizeObserver.disconnect();
  mutationObserver.disconnect();
  if (viewport) {
    viewport.removeEventListener("scroll", onScroll);
    viewport.removeEventListener("wheel", onWheel);
    viewport.removeEventListener("touchstart", onTouchStart);
    viewport.removeEventListener("touchmove", onTouchMove);
    viewport.removeEventListener("keydown", onKeyDown);
    viewport.removeEventListener("focusin", scheduleKeepSync);
    viewport.removeEventListener("focusout", scheduleKeepSync);
  }
  for (const el of kept) el.removeAttribute("data-keep-rendered");
  kept = new Set();
  bound = false;
}

// A command that came from a button: hand focus to the viewport before the
// button turns inert (an inert element silently drops focus to <body>).
function releaseButtonFocus() {
  const active = document.activeElement;
  if (active && root.contains(active) && active.matches("[data-message-scroller-button]")) {
    viewport.focus({ preventScroll: true });
  }
}

function handle(msg) {
  switch (msg.t) {
    case "config": {
      const wasFollowing = cfg.autoScroll;
      cfg.autoScroll = msg.autoScroll;
      cfg.edge = msg.edge;
      cfg.peek = msg.peek;
      cfg.margin = msg.margin;
      const virtualizeChanged = cfg.virtualize !== msg.virtualize;
      cfg.virtualize = msg.virtualize;
      if (bound && virtualizeChanged) syncKeepRendered();
      if (bound) {
        // Turning auto-scroll on while the reader is at the end starts following.
        if (cfg.autoScroll && !wasFollowing && atEnd() && mode !== "settling-jump" && mode !== "anchored-to-message") {
          mode = "following-bottom";
        }
        if (cfg.autoScroll && !wasFollowing && itemCount > 0 && mode === "following-bottom") scrollToEnd("auto");
        else commitScrollState();
      }
      break;
    }
    case "end":
      bind();
      if (msg.focus && bound) releaseButtonFocus();
      scrollToEnd(msg.behavior);
      break;
    case "start":
      bind();
      if (msg.focus && bound) releaseButtonFocus();
      scrollToStart(msg.behavior);
      break;
    case "message":
      bind();
      jumpToMessage(msg.id, msg.align, msg.behavior);
      break;
    case "sync":
      if (bound) commitScrollState(true);
      break;
    default:
      break;
  }
}

// ------------------------------------------------------------------- main

if (root) {
  for (let i = 0; !bind() && i < 60; i++) await nextFrame();
  if (bound) {
    handleContentChange();
    if (!defaultApplied) {
      if (itemCount === 0) clearPending();
      else pollOpen();
    }
    commitScrollState(true);
  } else {
    clearPending();
  }
  for (;;) {
    let msg;
    try {
      msg = await dioxus.recv();
    } catch (_) {
      break;
    }
    if (!msg || msg.t === "teardown") break;
    handle(msg);
  }
  teardown();
}
