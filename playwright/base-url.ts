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
