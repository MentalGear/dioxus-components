/**
 * Shared base URL for specs, so a suite (or a single spec) can be pointed at
 * a non-default dev server port -- e.g. a per-worktree lane running its own
 * `dx serve` on 8083+ instead of the shared default :8080 -- without editing
 * every spec file. See dev-docs/dev-loop.md's "Per-lane servers" section.
 *
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 \
 *     npx playwright test --config=session.local.config.ts avatar.spec.ts
 *
 * The default is unchanged (`http://127.0.0.1:8080`), so every spec that has
 * not been migrated to import this -- most of them still hardcode the
 * literal inline -- keeps working exactly as before.
 *
 * This consolidates three ad hoc variants of the same idea that predate this
 * file (kept working, not "wrong", just three names for one thing):
 *   - `drag_and_drop_list.spec.ts`: `const BASE = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080"`
 *   - `sidebar.spec.ts`: `const BASE_URL = "http://127.0.0.1:8080"` (no env override)
 *   - every other spec: the literal inline, repeated per `page.goto(...)` call
 * A spec can migrate by importing `BASE_URL` from here (aliasing to `BASE`
 * where that's the file's existing local name) instead of declaring its own.
 */
export const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080";

/**
 * Base URL for a spec whose conventional server is NOT the :8080 dev server
 * (e.g. `oracle/hydration-parity.spec.ts`, which defaults to the :8090 static
 * SSG server). Same `PLAYWRIGHT_BASE_URL` override as `BASE_URL`, only the
 * fallback differs, so one env var points any spec at any server -- including
 * one that serves under a path prefix (`http://127.0.0.1:8090/shadcn-dioxus`,
 * a `--base-path` build). The trailing slash is stripped so specs can keep
 * writing `${base}/component/x/`.
 */
export function baseUrlOr(fallback: string): string {
  return (process.env.PLAYWRIGHT_BASE_URL ?? fallback).replace(/\/+$/, "");
}

/**
 * Per-test timeout (ms) shared by every config in this directory, so a lane
 * can shorten (or lengthen) it without editing a config:
 *
 *   PW_TEST_TIMEOUT=45000 npx playwright test --config=ssg.local.config.ts ...
 *
 * Only the config-level DEFAULT moves. A spec that calls `test.setTimeout(...)`
 * / `test.slow()` (charts_gallery 240s, carousel-virtual-wheel 120-180s,
 * computed-style-snapshot 20min, hydration-parity per-component 5min) keeps its
 * own absolute budget -- that is intended, those are the genuinely long tests.
 * A non-numeric or non-positive value is ignored (falls back to `fallbackMs`).
 */
export function perTestTimeoutMs(fallbackMs: number): number {
  const n = Number(process.env.PW_TEST_TIMEOUT);
  return Number.isFinite(n) && n > 0 ? n : fallbackMs;
}

/**
 * Which cargo profile the build under test came from: `"debug"`, `"release"`,
 * or `"unknown"`. Explicit `PW_BUILD_PROFILE` wins; otherwise it is read off
 * `SSG_SITE_DIR`'s `dx/<crate>/<profile>/web/public` path. `"unknown"` (no hint,
 * e.g. a `dx serve` dev server) is treated as "do not interfere".
 */
export function buildProfile(): "debug" | "release" | "unknown" {
  const explicit = process.env.PW_BUILD_PROFILE?.toLowerCase();
  if (explicit === "debug" || explicit === "release") return explicit;
  const dir = (process.env.SSG_SITE_DIR ?? "").replace(/\\/g, "/");
  if (/\/dx\/[^/]+\/debug\/web\/public\/?$/.test(dir)) return "debug";
  if (/\/dx\/[^/]+\/release\/web\/public\/?$/.test(dir)) return "release";
  return "unknown";
}

/**
 * `testIgnore` for configs: timing-threshold specs (long-task / INP budgets in
 * `oracle/tier2-html/main-thread.spec.ts`) measure 64-153 ms long tasks on a
 * debug build that are 0 on release (backlog row 116, 2026-10-01: 4 false reds),
 * so they are skipped -- loudly, once -- when the build is known to be debug.
 * `PW_FORCE_TIMING_SPECS=1` runs them anyway (e.g. to confirm the false red).
 */
export function timingSpecIgnore(): RegExp[] {
  if (buildProfile() !== "debug" || process.env.PW_FORCE_TIMING_SPECS === "1") return [];
  if (!process.env.PW_TIMING_SKIP_WARNED) {
    // Set in the main process before workers spawn, so workers inherit it and stay quiet.
    process.env.PW_TIMING_SKIP_WARNED = "1";
    console.warn(
      "[playwright] debug build detected: skipping oracle/tier2-html/main-thread.spec.ts " +
        "(timing thresholds give false reds on debug; use a release build, or PW_FORCE_TIMING_SPECS=1 to run anyway).",
    );
  }
  return [/main-thread\.spec\.ts$/];
}
