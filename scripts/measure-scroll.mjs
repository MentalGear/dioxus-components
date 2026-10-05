#!/usr/bin/env node
// measure-scroll.mjs -- reproduce and quantify "scrolling the overview page
// feels janky" in a real (headless) Chromium, with Chrome DevTools Protocol
// tracing, and bisect suspects by switching them off.
//
// What it does, for one URL (default: the home route `/`):
//   1. loads the page, waits for hydration (`.dx-component-card`), settles;
//   2. installs (via addInitScript, BEFORE any page script) an inventory of
//      every scroll/resize/wheel/touch listener (with `passive`), every
//      ResizeObserver / IntersectionObserver / MutationObserver, every
//      setInterval, every requestAnimationFrame caller (with stack), every
//      getBoundingClientRect/getComputedStyle/offset* layout read;
//   3. measures an IDLE window (no input) -- main-thread cost of the page
//      doing nothing (timers, rAF loops, signal-driven re-renders);
//   4. scrolls top -> bottom with real wheel input (CDP Input.dispatchMouseEvent
//      mouseWheel) and records, over that window only:
//        - CDP Tracing (devtools.timeline, cc, input, v8): per-name counts +
//          durations on the renderer main thread, raster/compositor threads,
//          Layout/UpdateLayoutTree (and how many were JS-forced), long tasks;
//        - Performance.getMetrics deltas (LayoutCount, RecalcStyleCount, ...);
//        - in-page rAF frame deltas (p50/p95/max, #>20/33/50 ms);
//        - PerformanceObserver: long-animation-frame, longtask, layout-shift;
//        - scrollY gained per wheel tick (detects scroll latching in a nested
//          scroller: the page "sticks" while the pointer is over it).
//
// USAGE
//   node scripts/measure-scroll.mjs [url] [options]
//     --viewport WxH        default 1280x800
//     --step N              wheel deltaY per tick, px      (default 100)
//     --interval N          ms between ticks               (default 16)
//     --x N                 pointer x while wheeling       (default 40 = left gutter)
//     --runs N              repeat the scroll N times, report the median run (default 3)
//     --idle N              idle window seconds            (default 3)
//     --disable a,b,c       switch suspects off (see SUSPECTS below)
//     --css '<css>'         extra CSS injected after load
//     --init-js '<js>'      extra JS run in the page after load (before the idle window)
//     --trace <file>        keep the raw trace of the first run (Perfetto/DevTools loadable)
//     --json <file>         write the full result JSON
//     --label <text>        free text echoed in the report
//     --no-listeners        skip the (long) inventory block of the report
//     --brief               print ONE summary line per scenario (for tables / bisects)
//     --profile <name>      PW_BUILD_PROFILE-style tag echoed in the report (informational)
//     --warmup N            seconds to wait after hydration before measuring (default 2.5).
//                           Use 60 to expose anything that GROWS with page age (e.g. leaked timers).
//     --raf                 also run an in-page rAF frame sampler (OFF by default: a rAF loop forces a
//                           main-frame every vsync and so perturbs the thing being measured; the default
//                           frame stats come from the compositor's PipelineReporter instead)
//     --driver wheel|gesture  wheel = discrete CDP mouse-wheel ticks (default);
//                           gesture = Input.synthesizeScrollGesture (compositor-driven fling, 6000 px/s)
//
// SUSPECTS (--disable): each is a CSS override or JS stub, so a bisect is
//   `--disable backdrop` vs the plain baseline, same box, same minute.
//     smooth      html { scroll-behavior: auto }
//     backdrop    * { backdrop-filter: none }
//     filters     * { filter: none }
//     shadows     * { box-shadow: none; text-shadow: none }
//     willchange  * { will-change: auto }
//     sticky      sticky/fixed -> static (changes layout; for blame only)
//     anim        pause every CSS animation/transition
//     svg         hide every <svg> that is not an icon (charts)
//     cv          content-visibility:auto + contain-intrinsic-size on gallery cards / masonry
//     contain     contain: layout paint style on gallery cards / masonry cards
//     scrolllisten  drop every 'scroll' + 'resize' listener the page registers (init-script stub)
//     nowheel     drop every wheel/touchstart/touchmove listener (upper bound of what removing
//                 the Input onwheel + slider/resizable/color_picker ontouchstart could buy)
//     passive     force passive:true on wheel/touchstart/touchmove listeners (Dioxus' #main root
//                 listeners are non-passive: they make the compositor wait for the main thread)
//     intervals   neutralise ALL setInterval (progress / player tickers)
//     tick1hz     any setInterval faster than 1 s runs at 1 s (the player's slider has step 1.0)
//     leak        allow only the FIRST setInterval per period -- simulates "the player/progress
//                 effect creates its timer once and never again" (the leaked-interval fix)
//     ro          ResizeObserver/IntersectionObserver never fire
//     virtual     hide the dx-virtual-list-container demo
//     nested      overscroll-behavior: auto on nested scrollers
//
// EXAMPLES
//   python3 -m http.server 8090 -d <release public dir> &
//   node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --runs 3
//   node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --disable backdrop,shadows
//   node scripts/measure-scroll.mjs http://127.0.0.1:8090/ --x 640   # pointer over content
//
// SETUP: like scripts/inspect.mjs this resolves `playwright` through a gitignored symlink at the repo root:
//   ln -s playwright/node_modules node_modules      (see dev-docs/dev-loop.md)
// Findings and the measured bisect this harness produced: dev-docs/research/scroll-jank-2026-10-04.md
//
// NOTE: use a RELEASE build (CLAUDE.md: debug timings are meaningless) and
// prefer COUNTS (layouts, style recalcs, listeners) over absolute ms when
// other lanes are compiling on the same box; the report prints both.

import { chromium } from "playwright";
import fs from "node:fs";

const CHROMIUM_PATH = process.env.PW_CHROMIUM || "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";

function parseArgs(argv) {
  const o = {
    url: "http://127.0.0.1:8090/", viewport: "1280x800", step: 100, interval: 16, x: 40,
    runs: 3, idle: 3, disable: [], css: "", initJs: "", trace: null, json: null, label: "",
    listeners: true, brief: false, profile: "", warmup: 2.5, raf: false, driver: "wheel",
  };
  const a = argv.slice(2);
  for (let i = 0; i < a.length; i++) {
    const k = a[i];
    switch (k) {
      case "--viewport": o.viewport = a[++i]; break;
      case "--step": o.step = Number(a[++i]); break;
      case "--interval": o.interval = Number(a[++i]); break;
      case "--x": o.x = Number(a[++i]); break;
      case "--runs": o.runs = Number(a[++i]); break;
      case "--idle": o.idle = Number(a[++i]); break;
      case "--disable": o.disable = a[++i].split(",").filter(Boolean); break;
      case "--css": o.css = a[++i]; break;
      case "--init-js": o.initJs = a[++i]; break;
      case "--trace": o.trace = a[++i]; break;
      case "--json": o.json = a[++i]; break;
      case "--label": o.label = a[++i]; break;
      case "--profile": o.profile = a[++i]; break;
      case "--no-listeners": o.listeners = false; break;
      case "--brief": o.brief = true; break;
      case "--warmup": o.warmup = Number(a[++i]); break;
      case "--raf": o.raf = true; break;
      case "--driver": o.driver = a[++i]; break;
      case "-h": case "--help":
        { const lines = fs.readFileSync(new URL(import.meta.url), "utf8").split("\n").slice(1); const head = []; for (const l of lines) { if (!l.startsWith("//")) break; head.push(l.replace(/^\/\/ ?/, "")); } console.log(head.join("\n")); }
        process.exit(0);
      default:
        if (!k.startsWith("--")) { o.url = k; break; }
        console.error("unknown arg " + k); process.exit(2);
    }
  }
  return o;
}

const SUSPECT_CSS = {
  smooth: "html{scroll-behavior:auto !important}",
  backdrop: "*,*::before,*::after{backdrop-filter:none !important;-webkit-backdrop-filter:none !important}",
  filters: "*,*::before,*::after{filter:none !important}",
  shadows: "*,*::before,*::after{box-shadow:none !important;text-shadow:none !important}",
  willchange: "*,*::before,*::after{will-change:auto !important}",
  sticky: "*{position:static !important}",
  anim: "*,*::before,*::after{animation-play-state:paused !important;transition:none !important}",
  svg: "svg:not([width='16']):not([width='18']):not([width='24']):not([width='20']){display:none !important}",
  cv: ".dx-component-card,.dx-widget-card{content-visibility:auto;contain-intrinsic-size:auto 420px}",
  contain: ".dx-component-card,.dx-widget-card{contain:layout paint style}",
  virtual: ".dx-virtual-list-container{display:none !important}",
  nested: ".dx-virtual-list-container,.dx-scroll-area-auto-hide,.dx-sidebar-content{overscroll-behavior:auto !important}",
};

// Runs BEFORE any page script. Keeps everything in window.__probe.
function initScript(disable) {
  const dis = new Set(disable);
  const P = (window.__probe = {
    listeners: [], ro: [], io: [], mo: [], intervals: [], timeouts: 0, rafCalls: 0, rafStacks: {},
    reads: { getBoundingClientRect: 0, getClientRects: 0, getComputedStyle: 0, offset: 0, scrollTopSets: 0 },
    readStacks: {}, scrollEvents: 0, active: false,
  });
  const short = (n = 6) => (new Error().stack || "").split("\n").slice(2, 2 + n).map(s => s.trim().replace(/https?:\/\/[^/]+/, "")).join(" <- ");
  const desc = (t) => {
    if (t === window) return "window";
    if (t === document) return "document";
    if (!t || !t.tagName) return String(t);
    return t.tagName.toLowerCase() + (t.id ? "#" + t.id : "") + (t.classList && t.classList.length ? "." + [...t.classList].slice(0, 2).join(".") : "");
  };
  const WATCH = new Set(["scroll", "scrollend", "wheel", "mousewheel", "touchstart", "touchmove", "touchend", "resize", "pointermove", "mousemove", "orientationchange", "visualviewportresize"]);
  const origAdd = EventTarget.prototype.addEventListener;
  EventTarget.prototype.addEventListener = function (type, fn, opts) {
    if (dis.has("nowheel") && (type === "wheel" || type === "mousewheel" || type === "touchstart" || type === "touchmove") && !(this && this.classList && this.classList.contains && this.classList.contains("dx-carousel-content"))) return; // drop page wheel/touch listeners entirely
    if (dis.has("passive") && (type === "wheel" || type === "mousewheel" || type === "touchstart" || type === "touchmove")) {
      opts = Object.assign({}, typeof opts === "object" && opts ? opts : { capture: !!opts }, { passive: true });
    }
    if (WATCH.has(type)) {
      const o = typeof opts === "object" && opts ? opts : { capture: !!opts };
      const rootish = this === window || this === document || this === document.body || this === document.documentElement;
      const defaultPassive = rootish && (type === "wheel" || type === "mousewheel" || type === "touchstart" || type === "touchmove");
      P.listeners.push({ type, target: desc(this), passive: o.passive === undefined ? defaultPassive : o.passive, capture: !!o.capture, stack: short(5), t: performance.now() });
      if (dis.has("scrolllisten") && (type === "scroll" || type === "resize") ) return; // drop it
    }
    return origAdd.call(this, type, fn, opts);
  };
  const wrapCtor = (name, bucket, neutralise) => {
    const Orig = window[name];
    if (!Orig) return;
    window[name] = class extends Orig {
      constructor(cb, ...rest) {
        super(neutralise ? () => {} : (...args) => { P[bucket + "Fires"] = (P[bucket + "Fires"] || 0) + 1; return cb(...args); }, ...rest);
        this.__stack = short(5);
        P[bucket].push({ stack: this.__stack, observed: [] });
        this.__idx = P[bucket].length - 1;
      }
      observe(el, ...r) { const e = P[bucket][this.__idx]; if (e && e.observed.length < 3) e.observed.push(desc(el)); e.n = (e.n || 0) + 1; return super.observe(el, ...r); }
    };
  };
  wrapCtor("ResizeObserver", "ro", dis.has("ro"));
  wrapCtor("IntersectionObserver", "io", dis.has("ro"));
  wrapCtor("MutationObserver", "mo", false);
  const origSI = window.setInterval;
  window.setInterval = function (fn, ms, ...r) {
    P.intervals.push({ ms, stack: short(5), t: Math.round(performance.now()) });
    if (dis.has("intervals")) return 0;
    if (dis.has("tick1hz") && ms < 1000) ms = 1000;
    if (dis.has("leak")) { P.seenMs = P.seenMs || {}; if (P.seenMs[ms]) return 0; P.seenMs[ms] = true; }
    return origSI.call(window, fn, ms, ...r);
  };
  const origST = window.setTimeout;
  window.setTimeout = function (...a) { P.timeouts++; return origST.apply(window, a); };
  const origRAF = window.requestAnimationFrame.bind(window);
  P.rawRaf = origRAF;
  window.requestAnimationFrame = function (cb) {
    if (P.active) {
      P.rafCalls++;
      const s = short(4);
      P.rafStacks[s] = (P.rafStacks[s] || 0) + 1;
    }
    return origRAF(cb);
  };
  const count = (proto, name, key, stackKey) => {
    const orig = proto[name];
    proto[name] = function (...a) {
      if (P.active) {
        P.reads[key]++;
        const s = short(4);
        P.readStacks[key + " @ " + s] = (P.readStacks[key + " @ " + s] || 0) + 1;
      }
      return orig.apply(this, a);
    };
  };
  count(Element.prototype, "getBoundingClientRect", "getBoundingClientRect");
  count(Element.prototype, "getClientRects", "getClientRects");
  const origGCS = window.getComputedStyle;
  window.getComputedStyle = function (...a) {
    if (P.active) { P.reads.getComputedStyle++; const s = short(4); P.readStacks["getComputedStyle @ " + s] = (P.readStacks["getComputedStyle @ " + s] || 0) + 1; }
    return origGCS.apply(window, a);
  };
  for (const p of ["offsetWidth", "offsetHeight", "offsetTop", "offsetLeft", "clientWidth", "clientHeight", "scrollHeight", "scrollWidth"]) {
    const d = Object.getOwnPropertyDescriptor(HTMLElement.prototype, p) || Object.getOwnPropertyDescriptor(Element.prototype, p);
    if (!d || !d.get) continue;
    Object.defineProperty(HTMLElement.prototype, p, { configurable: true, get() { if (P.active) { P.reads.offset++; const s = short(4); P.readStacks[p + " @ " + s] = (P.readStacks[p + " @ " + s] || 0) + 1; } return d.get.call(this); } });
  }
  // PerformanceObservers (registered early, buffered).
  P.loaf = []; P.longtasks = []; P.cls = [];
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) P.loaf.push({ start: e.startTime, dur: e.duration, block: e.blockingDuration, render: e.renderStart ? e.startTime + e.duration - e.renderStart : 0, style: e.styleAndLayoutStart ? e.startTime + e.duration - e.styleAndLayoutStart : 0, scripts: (e.scripts || []).map(s => ({ d: s.duration, inv: s.invoker, type: s.invokerType, src: (s.sourceURL || "").split("/").pop(), fn: s.sourceFunctionName, forced: s.forcedStyleAndLayoutDuration })) }); }).observe({ type: "long-animation-frame", buffered: true }); } catch {}
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) P.longtasks.push({ start: e.startTime, dur: e.duration }); }).observe({ type: "longtask", buffered: true }); } catch {}
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) P.cls.push({ start: e.startTime, v: e.value, hadInput: e.hadRecentInput }); }).observe({ type: "layout-shift", buffered: true }); } catch {}
}

const median = (xs) => { const s = [...xs].sort((a, b) => a - b); return s.length ? s[Math.floor(s.length / 2)] : 0; };
const pct = (xs, p) => { const s = [...xs].sort((a, b) => a - b); return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * p))] : 0; };
const r1 = (n) => Math.round(n * 10) / 10;

const TRACE_CATEGORIES = [
  "devtools.timeline", "disabled-by-default-devtools.timeline", "disabled-by-default-devtools.timeline.frame",
  "disabled-by-default-devtools.timeline.stack", "cc", "input", "input.scrolling", "latencyInfo", "benchmark", "rail", "v8.execute", "blink",
].join(",");

function analyzeTrace(events) {
  // thread names
  const tname = {};
  for (const e of events) if (e.ph === "M" && e.name === "thread_name") tname[e.pid + ":" + e.tid] = e.args.name;
  const rendererMain = Object.entries(tname).filter(([, n]) => n === "CrRendererMain").map(([k]) => k);
  const byThread = {};
  for (const e of events) {
    if (e.ph !== "X" && e.ph !== "B") continue;
    const k = e.pid + ":" + e.tid;
    (byThread[k] = byThread[k] || []).push(e);
  }
  const agg = (evs) => {
    const m = {};
    for (const e of evs) { const x = (m[e.name] = m[e.name] || { n: 0, ms: 0 }); x.n++; x.ms += (e.dur || 0) / 1000; }
    for (const k of Object.keys(m)) m[k].ms = r1(m[k].ms);
    return m;
  };
  // pick the main thread with the most events (the page's renderer)
  let mainKey = rendererMain.sort((a, b) => (byThread[b] || []).length - (byThread[a] || []).length)[0];
  const main = (byThread[mainKey] || []).filter(e => e.ph === "X").sort((a, b) => a.ts - b.ts);
  const JS = new Set(["EventDispatch", "FireAnimationFrame", "TimerFire", "FunctionCall", "v8.callFunction", "RunMicrotasks", "FireIdleCallback", "ResizeObserver", "MutationObserver"]);
  const jsEvents = main.filter(e => JS.has(e.name) && e.dur);
  const inJs = (e) => jsEvents.some(j => j !== e && j.ts <= e.ts && j.ts + j.dur >= e.ts + (e.dur || 0));
  const layouts = main.filter(e => e.name === "Layout");
  const recalcs = main.filter(e => e.name === "UpdateLayoutTree");
  const tasks = main.filter(e => e.name === "RunTask" && e.dur);
  const longTasks = tasks.filter(t => t.dur >= 50000);
  const over16 = tasks.filter(t => t.dur >= 16700);
  // children of the longest tasks
  const worst = [...tasks].sort((a, b) => b.dur - a.dur).slice(0, 5).map(t => {
    const kids = main.filter(e => e !== t && e.ts >= t.ts && e.ts + (e.dur || 0) <= t.ts + t.dur && e.dur).sort((a, b) => b.dur - a.dur).slice(0, 4).map(e => `${e.name}:${r1(e.dur / 1000)}`);
    return { ms: r1(t.dur / 1000), top: kids };
  });
  // compositor-side frame outcomes (PipelineReporter) -- the real "was a frame dropped" signal
  const pr = {}; let prSmoothDropped = 0, prSmoothFrames = 0, prMainAnim = 0, prPresented = 0;
  for (const e of events) if (e.name === "PipelineReporter" && e.ph === "b") {
    const f = e.args && e.args.frame_reporter; if (!f) continue;
    pr[f.state] = (pr[f.state] || 0) + 1;
    if (f.affects_smoothness) { prSmoothFrames++; if (f.state === "STATE_DROPPED") prSmoothDropped++; }
    if (f.state === "STATE_PRESENTED_ALL" || f.state === "STATE_PRESENTED_PARTIAL") prPresented++;
    if (f.has_main_animation) prMainAnim++;
  }
  // input latency per event type: EventLatency b/e pairs
  const elOpen = {}, elDur = {};
  for (const e of events) if (e.name === "EventLatency") {
    const id = e.pid + ":" + (e.id2 && e.id2.local || e.id);
    if (e.ph === "b") elOpen[id] = { ts: e.ts, type: e.args && e.args.event_latency && e.args.event_latency.event_type };
    else if (e.ph === "e" && elOpen[id]) { (elDur[elOpen[id].type] = elDur[elOpen[id].type] || []).push((e.ts - elOpen[id].ts) / 1000); delete elOpen[id]; }
  }
  const eventLatency = {};
  for (const [t, ds] of Object.entries(elDur)) eventLatency[t] = { n: ds.length, p50: r1(median(ds)), p95: r1(pct(ds, 0.95)), max: r1(Math.max(...ds)), over50: ds.filter(d => d > 50).length };
  const blocking = events.filter(e => e.name === "InputEventSentBlocking").length;
  const nonBlocking = events.filter(e => e.name === "InputEventSentNonBlocking").length;
  const sentToMain = events.filter(e => e.name === "WidgetInputHandlerManager::DidHandleInputEventSentToMain").length;
  const sentToCompositor = events.filter(e => e.name === "WidgetInputHandlerManager::DidHandleInputEventSentToCompositor").length;
  const interesting = ["Layout", "UpdateLayoutTree", "PrePaint", "Paint", "Layerize", "Commit", "UpdateLayer", "EventDispatch", "FireAnimationFrame", "TimerFire", "FunctionCall", "v8.callFunction", "HitTest", "ScheduleStyleRecalculation", "InvalidateLayout", "StyleRecalcInvalidationTracking", "ResizeObserver", "IntersectionObserverController::computeIntersections", "ParseHTML", "BeginMainThreadFrame", "RunTask", "ThreadControllerImpl::RunTask"];
  const mainAgg = agg(main);
  const mainPicked = {};
  for (const n of interesting) if (mainAgg[n]) mainPicked[n] = mainAgg[n];
  // layout-affecting invalidations (what caused style/layout)
  const inval = {};
  for (const e of events) if (e.name === "StyleRecalcInvalidationTracking" || e.name === "StyleInvalidatorInvalidationTracking" || e.name === "LayoutInvalidationTracking") {
    const k = e.name + ":" + (e.args?.data?.reason || e.args?.data?.invalidatedSelector || "?");
    inval[k] = (inval[k] || 0) + 1;
  }
  const raster = {}, other = {};
  for (const [k, evs] of Object.entries(byThread)) {
    const n = tname[k] || "?";
    if (k === mainKey) continue;
    const tgt = /Raster|TileWorker/.test(n) ? raster : /Compositor|VizCompositor|GPU|Chrome_InProcGpu/.test(n) ? other : null;
    if (!tgt) continue;
    const a = agg(evs.filter(e => e.ph === "X"));
    const key = n.replace(/\d+$/, "");
    for (const [nm, v] of Object.entries(a)) { const x = (tgt[key + "/" + nm] = tgt[key + "/" + nm] || { n: 0, ms: 0 }); x.n += v.n; x.ms = r1(x.ms + v.ms); }
  }
  const topOf = (o, n = 8) => Object.fromEntries(Object.entries(o).sort((a, b) => b[1].ms - a[1].ms).slice(0, n));
  const mainBusy = tasks.reduce((s, t) => s + t.dur / 1000, 0);
  return {
    mainThread: mainKey ? tname[mainKey] : null,
    mainBusyMs: r1(mainBusy), tasks: tasks.length, tasksOver16ms: over16.length, longTasksOver50ms: longTasks.length,
    layoutCount: layouts.length, layoutMs: r1(layouts.reduce((s, e) => s + e.dur / 1000, 0)),
    layoutJsForced: layouts.filter(inJs).length,
    recalcCount: recalcs.length, recalcMs: r1(recalcs.reduce((s, e) => s + e.dur / 1000, 0)),
    recalcJsForced: recalcs.filter(inJs).length,
    pipeline: { states: pr, smoothFrames: prSmoothFrames, smoothDropped: prSmoothDropped, presented: prPresented, mainAnimFrames: prMainAnim },
    eventLatency, input: { blocking, nonBlocking, sentToMain, sentToCompositor },
    worstTasks: worst, mainPicked, invalidations: topOf(Object.fromEntries(Object.entries(inval).map(([k, v]) => [k, { n: v, ms: v }])), 6),
    rasterTop: topOf(raster, 6), compositorTop: topOf(other, 6),
  };
}

async function runOnce(page, cdp, o, runIdx) {
  // reset to top
  await page.evaluate(() => { document.documentElement.style.scrollBehavior = "auto"; window.scrollTo(0, 0); });
  await page.waitForTimeout(300);
  await page.evaluate(() => { document.documentElement.style.scrollBehavior = ""; });
  const [vw, vh] = o.viewport.split("x").map(Number);
  await page.mouse.move(o.x, vh / 2);

  const total = await page.evaluate(() => document.scrollingElement.scrollHeight - innerHeight);
  const ticks = Math.ceil(total / o.step);

  // in-page frame sampler
  await page.evaluate((rafOn) => {
    const P = window.__probe;
    P.frames = []; P.ticks = []; P.active = true; P.rafCalls = 0; P.rafStacks = {}; P.readStacks = {};
    for (const k of Object.keys(P.reads)) P.reads[k] = 0;
    P.loafMark = P.loaf.length; P.ltMark = P.longtasks.length; P.clsMark = P.cls.length;
    P.roFires0 = P.roFires || 0; P.ioFires0 = P.ioFires || 0; P.moFires0 = P.moFires || 0;
    let last = performance.now();
    P.sampling = !!rafOn;
    // P.rawRaf bypasses the counting wrapper so the sampler never counts itself
    const loop = (t) => { if (!P.sampling) return; P.frames.push(t - last); last = t; window.__rafSample = P.rawRaf(loop); };
    if (rafOn) window.__rafSample = P.rawRaf(loop);
    P.rafCalls = 0; P.rafStacks = {};
    const onScroll = () => { P.scrollEvents++; };
    window.addEventListener("scroll", onScroll, { passive: true });
    window.__scrollSampler = onScroll;
  }, o.raf);
  const liveIntervals0 = await page.evaluate(() => window.__probe.intervals.length);
  const m0 = await cdp.send("Performance.getMetrics");
  await cdp.send("Tracing.start", { categories: TRACE_CATEGORIES, transferMode: "ReturnAsStream", streamFormat: "json" });
  const traceDone = new Promise((res) => cdp.once("Tracing.tracingComplete", res));

  const tStart = Date.now();
  const gains = [];
  let lastY = await page.evaluate(() => scrollY);
  if (o.driver === "gesture") {
    // compositor-driven fling: one synthesized gesture, 6000 px/s, whole page
    await cdp.send("Input.synthesizeScrollGesture", { x: o.x, y: vh / 2, yDistance: -total, speed: 6000, gestureSourceType: "mouse", preventFling: true });
  } else {
    for (let i = 0; i < ticks + 3; i++) {
      await page.mouse.wheel(0, o.step);
      await page.waitForTimeout(o.interval);
      if (i % 4 === 0) {
        const y = await page.evaluate(() => scrollY);
        gains.push(y - lastY); lastY = y;
      }
    }
  }
  await page.waitForTimeout(600); // let momentum settle
  const tEnd = Date.now();
  if (o.raf) await page.evaluate(() => { window.__probe.sampling = false; });
  await cdp.send("Tracing.end");
  const { stream } = await traceDone;
  const m1 = await cdp.send("Performance.getMetrics");
  let text = "";
  for (;;) {
    const chunk = await cdp.send("IO.read", { handle: stream });
    text += chunk.data;
    if (chunk.eof) break;
  }
  await cdp.send("IO.close", { handle: stream });
  const trace = JSON.parse(text);
  const events = Array.isArray(trace) ? trace : trace.traceEvents;
  if (o.trace && runIdx === 0) fs.writeFileSync(o.trace, JSON.stringify(trace));

  const pageData = await page.evaluate(() => {
    const P = window.__probe;
    P.sampling = false; P.active = false;
    window.removeEventListener("scroll", window.__scrollSampler);
    return {
      frames: P.frames, finalY: scrollY, total: document.scrollingElement.scrollHeight - innerHeight, scrollEvents: P.scrollEvents,
      loaf: P.loaf.slice(P.loafMark), longtasks: P.longtasks.slice(P.ltMark), cls: P.cls.slice(P.clsMark),
      reads: { ...P.reads }, readStacks: P.readStacks, rafCalls: P.rafCalls, rafStacks: P.rafStacks,
      roFires: (P.roFires || 0) - P.roFires0, ioFires: (P.ioFires || 0) - P.ioFires0, moFires: (P.moFires || 0) - P.moFires0,
      intervalsNow: P.intervals.length,
    };
  });
  const mm = (name) => (m1.metrics.find(x => x.name === name)?.value ?? 0) - (m0.metrics.find(x => x.name === name)?.value ?? 0);
  const frames = pageData.frames.slice(1);
  const secs = (tEnd - tStart) / 1000;
  const liveIntervals1 = pageData.intervalsNow;
  const loafTotal = pageData.loaf.reduce((s, e) => s + e.dur, 0);
  const loafBlocking = pageData.loaf.reduce((s, e) => s + e.block, 0);
  const scriptsByInvoker = {};
  for (const e of pageData.loaf) for (const s of e.scripts) { const k = `${s.type}:${(s.inv || "").slice(0, 60)} ${s.src}:${s.fn || ""}`; const x = (scriptsByInvoker[k] = scriptsByInvoker[k] || { n: 0, ms: 0, forced: 0 }); x.n++; x.ms += s.d; x.forced += s.forced || 0; }
  for (const k of Object.keys(scriptsByInvoker)) { scriptsByInvoker[k].ms = r1(scriptsByInvoker[k].ms); scriptsByInvoker[k].forced = r1(scriptsByInvoker[k].forced); }
  const tr = analyzeTrace(events);
  return {
    intervals: { before: liveIntervals0, after: liveIntervals1 },
    secs: r1(secs), ticks: ticks + 3, scrolledPx: pageData.finalY, totalPx: pageData.total,
    stalledTicks: gains.filter(g => g < o.step * 0.5 * 4).length, sampledTicks: gains.length,
    frames: { n: frames.length, p50: r1(median(frames)), p95: r1(pct(frames, 0.95)), p99: r1(pct(frames, 0.99)), max: r1(Math.max(0, ...frames)), over20: frames.filter(f => f > 20).length, over33: frames.filter(f => f > 33.4).length, over50: frames.filter(f => f > 50).length, fps: r1(frames.length / secs) },
    cdpMetrics: { layoutCount: mm("LayoutCount"), recalcStyleCount: mm("RecalcStyleCount"), layoutMs: r1(mm("LayoutDuration") * 1000), recalcStyleMs: r1(mm("RecalcStyleDuration") * 1000), scriptMs: r1(mm("ScriptDuration") * 1000), taskMs: r1(mm("TaskDuration") * 1000), jsListeners: m1.metrics.find(x => x.name === "JSEventListeners")?.value, nodes: m1.metrics.find(x => x.name === "Nodes")?.value },
    loaf: { n: pageData.loaf.length, totalMs: r1(loafTotal), blockingMs: r1(loafBlocking), maxMs: r1(Math.max(0, ...pageData.loaf.map(e => e.dur))), scripts: Object.fromEntries(Object.entries(scriptsByInvoker).sort((a, b) => b[1].ms - a[1].ms).slice(0, 6)) },
    longtasks: { n: pageData.longtasks.length, totalMs: r1(pageData.longtasks.reduce((s, e) => s + e.dur, 0)) },
    cls: { n: pageData.cls.length, value: Math.round(pageData.cls.reduce((s, e) => s + (e.hadInput ? 0 : e.v), 0) * 1000) / 1000 },
    scrollEvents: pageData.scrollEvents,
    reads: pageData.reads, readStacks: Object.fromEntries(Object.entries(pageData.readStacks).sort((a, b) => b[1] - a[1]).slice(0, 8)),
    rafCalls: pageData.rafCalls, rafStacks: Object.fromEntries(Object.entries(pageData.rafStacks).sort((a, b) => b[1] - a[1]).slice(0, 5)),
    observerFires: { ro: pageData.roFires, io: pageData.ioFires, mo: pageData.moFires },
    trace: tr,
  };
}

async function idleWindow(page, cdp, secs) {
  await page.evaluate(() => { const P = window.__probe; P.active = true; P.rafCalls = 0; P.rafStacks = {}; P.readStacks = {}; for (const k of Object.keys(P.reads)) P.reads[k] = 0; P.loafMark = P.loaf.length; P.roFires0 = P.roFires || 0; P.moFires0 = P.moFires || 0; });
  const m0 = await cdp.send("Performance.getMetrics");
  await cdp.send("Tracing.start", { categories: TRACE_CATEGORIES, transferMode: "ReturnAsStream", streamFormat: "json" });
  const done = new Promise((res) => cdp.once("Tracing.tracingComplete", res));
  await page.waitForTimeout(secs * 1000);
  await cdp.send("Tracing.end");
  const { stream } = await done;
  const m1 = await cdp.send("Performance.getMetrics");
  let text = "";
  for (;;) { const c = await cdp.send("IO.read", { handle: stream }); text += c.data; if (c.eof) break; }
  await cdp.send("IO.close", { handle: stream });
  const trace = JSON.parse(text);
  const events = Array.isArray(trace) ? trace : trace.traceEvents;
  const pd = await page.evaluate(() => { const P = window.__probe; P.active = false; return { rafCalls: P.rafCalls, rafStacks: P.rafStacks, reads: { ...P.reads }, readStacks: P.readStacks, loaf: P.loaf.slice(P.loafMark).length, ro: (P.roFires || 0) - P.roFires0, mo: (P.moFires || 0) - P.moFires0 }; });
  const mm = (n) => (m1.metrics.find(x => x.name === n)?.value ?? 0) - (m0.metrics.find(x => x.name === n)?.value ?? 0);
  const tr = analyzeTrace(events);
  return { secs, rafCallsPerSec: r1(pd.rafCalls / secs), rafStacks: Object.fromEntries(Object.entries(pd.rafStacks).sort((a, b) => b[1] - a[1]).slice(0, 4)), reads: pd.reads, loaf: pd.loaf, observerFires: { ro: pd.ro, mo: pd.mo }, layouts: mm("LayoutCount"), recalcs: mm("RecalcStyleCount"), taskMsPerSec: r1(mm("TaskDuration") * 1000 / secs), scriptMsPerSec: r1(mm("ScriptDuration") * 1000 / secs), mainBusyMsPerSec: r1(tr.mainBusyMs / secs), timers: tr.mainPicked.TimerFire, animationFrames: tr.mainPicked.FireAnimationFrame, paint: tr.mainPicked.Paint };
}

async function main() {
  const o = parseArgs(process.argv);
  const [vw, vh] = o.viewport.split("x").map(Number);
  const browser = await chromium.launch({ executablePath: CHROMIUM_PATH, args: ["--no-sandbox", "--enable-gpu-rasterization=false", "--disable-smooth-scrolling=false"] });
  const ctx = await browser.newContext({ viewport: { width: vw, height: vh } });
  const page = await ctx.newPage();
  await page.addInitScript(initScript, o.disable);
  const cdp = await ctx.newCDPSession(page);
  await cdp.send("Performance.enable");
  const t0 = Date.now();
  await page.goto(o.url, { waitUntil: "load" });
  await page.waitForSelector(".dx-component-card, #hero, main", { timeout: 60000 });
  await page.waitForTimeout(o.warmup * 1000);
  const loadMs = Date.now() - t0;
  const css = o.disable.map(d => SUSPECT_CSS[d]).filter(Boolean).join("\n") + "\n" + o.css;
  if (css.trim()) await page.addStyleTag({ content: css });
  if (o.initJs) await page.evaluate(o.initJs);
  await page.waitForTimeout(800);

  const stat = await page.evaluate(() => {
    const all = document.querySelectorAll("*");
    const cs = (e) => getComputedStyle(e);
    let bf = 0, fl = 0, sh = 0, wc = 0, st = 0, fx = 0, anim = 0, svg = 0, anchor = 0, cv = 0, contain = 0;
    const bfx = []; const animNames = {};
    all.forEach(e => {
      if (e instanceof SVGElement) svg++;
      const c = cs(e);
      if (c.backdropFilter && c.backdropFilter !== "none") { bf++; bfx.push(e.className?.baseVal ?? e.className); }
      if (c.filter && c.filter !== "none") fl++;
      if (c.boxShadow && c.boxShadow !== "none") sh++;
      if (c.willChange && c.willChange !== "auto") wc++;
      if (c.position === "sticky") st++;
      if (c.position === "fixed") fx++;
      if (c.animationName && c.animationName !== "none") { anim++; animNames[c.animationName] = (animNames[c.animationName] || 0) + 1; }
      if ((c.anchorName && c.anchorName !== "none") || (c.positionAnchor && c.positionAnchor !== "auto" && c.positionAnchor !== "none")) anchor++;
      if (c.contentVisibility === "auto") cv++;
      if (c.contain && c.contain !== "none") contain++;
    });
    return { nodes: all.length, svgNodes: svg, docH: document.scrollingElement.scrollHeight, backdropFilter: bf, filter: fl, boxShadow: sh, willChange: wc, sticky: st, fixed: fx, animating: anim, positionAnchor: anchor, contentVisibilityAuto: cv, contain, backdropSamples: bfx.slice(0, 5), animNames };
  });

  const inv = await page.evaluate(() => {
    const P = window.__probe;
    const by = (arr, f) => { const m = {}; arr.forEach(x => { const k = f(x); m[k] = (m[k] || 0) + 1; }); return m; };
    return {
      listeners: by(P.listeners, l => `${l.type} on ${l.target} passive=${l.passive}${l.capture ? " capture" : ""}`),
      listenerStacks: Object.fromEntries(Object.entries(by(P.listeners.filter(l => /scroll|resize|wheel|touch/.test(l.type)), l => `${l.type}@${l.target}: ${l.stack}`)).slice(0, 12)),
      ro: P.ro.length, roObserved: P.ro.reduce((s, r) => s + (r.n || 0), 0), roStacks: by(P.ro, r => r.stack.split(" <- ").slice(0, 3).join(" <- ")),
      io: P.io.length, ioObserved: P.io.reduce((s, r) => s + (r.n || 0), 0), ioStacks: by(P.io, r => r.stack.split(" <- ").slice(0, 3).join(" <- ")),
      mo: P.mo.length, moStacks: by(P.mo, r => r.stack.split(" <- ").slice(0, 3).join(" <- ")),
      intervals: P.intervals.map(i => `${i.ms}ms: ${i.stack}`), timeouts: P.timeouts,
    };
  });
  // CDP view of the same thing for window/document/body
  const cdpListeners = {};
  for (const [name, expr] of [["window", "window"], ["document", "document"], ["body", "document.body"], ["html", "document.documentElement"], ["#main", "document.querySelector('#main')"]]) {
    try {
      const { result } = await cdp.send("Runtime.evaluate", { expression: expr });
      if (!result.objectId) continue;
      const { listeners } = await cdp.send("DOMDebugger.getEventListeners", { objectId: result.objectId });
      const m = {};
      for (const l of listeners) { const k = `${l.type}${l.passive ? "(passive)" : ""}${l.useCapture ? "(capture)" : ""}`; m[k] = (m[k] || 0) + 1; }
      cdpListeners[name] = m;
    } catch {}
  }

  const idle = await idleWindow(page, cdp, o.idle);

  const runs = [];
  for (let i = 0; i < o.runs; i++) runs.push(await runOnce(page, cdp, o, i));
  const key = (r) => r.trace.mainBusyMs;
  const sorted = [...runs].sort((a, b) => key(a) - key(b));
  const med = sorted[Math.floor(sorted.length / 2)];

  const result = { label: o.label, disable: o.disable, url: o.url, viewport: o.viewport, step: o.step, interval: o.interval, x: o.x, profile: o.profile, loadMs, stat, inv, cdpListeners, idle, runs, median: med };
  if (o.json) fs.writeFileSync(o.json, JSON.stringify(result, null, 1));
  if (o.brief) brief(result); else report(result, o);
  await browser.close();
}

function brief(R) {
  const rs = R.runs;
  const med = (f) => r1(median(rs.map(f)));
  const wl = (r) => r.trace.eventLatency.MOUSE_WHEEL || r.trace.eventLatency.GESTURE_SCROLL_UPDATE || {};
  console.log([
    `${(R.label || R.disable.join("+") || "baseline").padEnd(22)}`,
    `idle ${R.idle.mainBusyMsPerSec}ms/s lay ${R.idle.layouts}/${R.idle.secs}s`,
    `| scroll(med of ${rs.length}) busy ${med(r => r.trace.mainBusyMs)}ms`,
    `lay ${med(r => r.trace.layoutCount)} rec ${med(r => r.trace.recalcCount)}`,
    `tasks>16ms ${med(r => r.trace.tasksOver16ms)} >50ms ${med(r => r.trace.longTasksOver50ms)}`,
    `LoAF ${med(r => r.loaf.n)}/${med(r => r.loaf.totalMs)}ms`,
    `droppedFrames ${med(r => r.trace.pipeline.smoothDropped)}/${med(r => r.trace.pipeline.smoothFrames)}`,
    `wheelLat p50/p95/max ${med(r => wl(r).p50 || 0)}/${med(r => wl(r).p95 || 0)}/${med(r => wl(r).max || 0)}ms`,
    `blocking ${med(r => r.trace.input.blocking)} nonblk ${med(r => r.trace.input.nonBlocking)}`,
    `intervals@end ${rs[rs.length - 1].intervals.after}`,
  ].join(" "));
}

function report(R, o) {
  const m = R.median;
  const L = [];
  L.push(`== measure-scroll ${R.label ? `[${R.label}] ` : ""}${R.url}  disable=[${R.disable.join(",")}]  viewport=${R.viewport} step=${R.step}px/${R.interval}ms pointer.x=${R.x}${R.profile ? " profile=" + R.profile : ""}`);
  L.push(`page: ${R.stat.docH}px tall, ${R.stat.nodes} nodes (${R.stat.svgNodes} svg), backdrop-filter:${R.stat.backdropFilter} filter:${R.stat.filter} box-shadow:${R.stat.boxShadow} will-change:${R.stat.willChange} sticky:${R.stat.sticky} fixed:${R.stat.fixed} animating:${R.stat.animating} ${JSON.stringify(R.stat.animNames)} anchor-name/position-anchor set on:${R.stat.positionAnchor} content-visibility:auto:${R.stat.contentVisibilityAuto} contain:${R.stat.contain}`);
  if (o.listeners) {
    L.push(`listeners (registered by page, watched types): ${JSON.stringify(R.inv.listeners)}`);
    L.push(`cdp getEventListeners: ${JSON.stringify(R.cdpListeners)}`);
    L.push(`ResizeObserver x${R.inv.ro} (observe() calls ${R.inv.roObserved}), IntersectionObserver x${R.inv.io} (observe() calls ${R.inv.ioObserved}), MutationObserver x${R.inv.mo}; setInterval x${R.inv.intervals.length}: ${JSON.stringify(R.inv.intervals)}`);
  }
  const i = R.idle;
  L.push(`IDLE ${i.secs}s: main busy ${i.mainBusyMsPerSec} ms/s, script ${i.scriptMsPerSec} ms/s, rAF calls/s ${i.rafCallsPerSec}, layouts ${i.layouts}, style recalcs ${i.recalcs}, LoAF ${i.loaf}, timers ${JSON.stringify(i.timers || {})}, animation frames ${JSON.stringify(i.animationFrames || {})}`);
  if (Object.keys(i.rafStacks).length) L.push(`  idle rAF callers: ${JSON.stringify(i.rafStacks)}`);
  L.push(`SCROLL (median of ${R.runs.length} runs by main-thread busy): ${m.secs}s wall, ${m.ticks} wheel ticks, scrolled ${m.scrolledPx}/${m.totalPx}px, stalled-sample ticks ${m.stalledTicks}/${m.sampledTicks}`);
  L.push(`  compositor frames: ${JSON.stringify(m.trace.pipeline.states)}; smooth-affecting frames ${m.trace.pipeline.smoothFrames}, DROPPED ${m.trace.pipeline.smoothDropped}, main-animation frames ${m.trace.pipeline.mainAnimFrames}`);
  L.push(`  input: sent BLOCKING ${m.trace.input.blocking} / non-blocking ${m.trace.input.nonBlocking}; handled-by-main ${m.trace.input.sentToMain} / by-compositor ${m.trace.input.sentToCompositor}; EventLatency(ms) ${JSON.stringify(m.trace.eventLatency)}`);
  L.push(`  live setIntervals: ${m.intervals.before} -> ${m.intervals.after} during the scroll`);
  if (m.frames.n) L.push(`  frames(rAF sampler): n=${m.frames.n} fps=${m.frames.fps} p50=${m.frames.p50} p95=${m.frames.p95} p99=${m.frames.p99} max=${m.frames.max} ms; >20ms:${m.frames.over20} >33ms:${m.frames.over33} >50ms:${m.frames.over50}`);
  L.push(`  main thread: busy ${m.trace.mainBusyMs} ms over ${m.trace.tasks} tasks (>16ms: ${m.trace.tasksOver16ms}, >50ms: ${m.trace.longTasksOver50ms}); Layout x${m.trace.layoutCount} (${m.trace.layoutMs}ms, JS-forced ${m.trace.layoutJsForced}); UpdateLayoutTree x${m.trace.recalcCount} (${m.trace.recalcMs}ms, JS-forced ${m.trace.recalcJsForced})`);
  L.push(`  CDP metrics: layouts ${m.cdpMetrics.layoutCount}, style recalcs ${m.cdpMetrics.recalcStyleCount}, layout ${m.cdpMetrics.layoutMs}ms, recalc ${m.cdpMetrics.recalcStyleMs}ms, script ${m.cdpMetrics.scriptMs}ms, task ${m.cdpMetrics.taskMs}ms`);
  L.push(`  LoAF n=${m.loaf.n} total=${m.loaf.totalMs}ms blocking=${m.loaf.blockingMs}ms max=${m.loaf.maxMs}ms; longtask n=${m.longtasks.n} total=${m.longtasks.totalMs}ms; layout-shift n=${m.cls.n} cls=${m.cls.value}`);
  L.push(`  scroll events ${m.scrollEvents}; layout reads during scroll: ${JSON.stringify(m.reads)}; observers fired ${JSON.stringify(m.observerFires)}; rAF calls ${m.rafCalls}`);
  if (Object.keys(m.readStacks).length) L.push(`  layout-read callers: ${JSON.stringify(m.readStacks)}`);
  if (Object.keys(m.rafStacks).length) L.push(`  rAF callers: ${JSON.stringify(m.rafStacks)}`);
  L.push(`  main-thread events: ${JSON.stringify(m.trace.mainPicked)}`);
  L.push(`  raster/compositor/gpu: ${JSON.stringify(m.trace.rasterTop)} | ${JSON.stringify(m.trace.compositorTop)}`);
  L.push(`  worst tasks: ${JSON.stringify(m.trace.worstTasks)}`);
  if (Object.keys(m.loaf.scripts).length) L.push(`  LoAF scripts: ${JSON.stringify(m.loaf.scripts)}`);
  console.log(L.join("\n"));
  const all = R.runs;
  console.log(`  per-run main busy ms: ${all.map(r => r.trace.mainBusyMs).join(", ")}; layouts: ${all.map(r => r.trace.layoutCount).join(", ")}; recalcs: ${all.map(r => r.trace.recalcCount).join(", ")}; dropped frames: ${all.map(r => r.trace.pipeline.smoothDropped + '/' + r.trace.pipeline.smoothFrames).join(", ")}; wheel p95 latency ms: ${all.map(r => (r.trace.eventLatency.MOUSE_WHEEL || {}).p95).join(", ")}`);
}

main().catch((e) => { console.error(e); process.exit(1); });
