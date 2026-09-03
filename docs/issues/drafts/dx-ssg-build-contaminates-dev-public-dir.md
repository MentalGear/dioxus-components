# Draft issue: `dx build --ssg` writes prerendered route pages into the same directory `dx serve`'s dev server serves, contaminating the next CSR dev session

**Status:** drafted 2026-09-03, not filed.

---

**Target repo:** `DioxusLabs/dioxus` (the `dx` CLI).

**Versions in use** (from this repo's `Cargo.lock`): `dioxus 0.7.9`, `dioxus-fullstack 0.7.9`. `dx`'s own version is not a `Cargo.lock` entry (it's a separate binary, not a workspace dependency) — confirm the exact `dx --version` in use before filing; this repo's docs record it only as "must match the lockfile" (`docs/lifting-from-forks.md` §6), not a pinned number.

## Title

`dx build --ssg` prerenders route pages directly into the CSR dev server's served directory, so a later `dx serve`/`dx run --web` mounts on top of stale prerendered markup

## Body

### Context

`dx build --ssg --features fullstack --platform web` (this repo's CI recipe, mirroring `.github/workflows/web.yml`) writes its build output to `target/dx/<app>/debug/web/public` — the **same directory** `dx serve`/`dx run --web` serves for ordinary CSR development. The SSG build's prerender step additionally writes fully-rendered HTML for every registered route (in this repo's case: `component/`, `dashboard/`, `docs/`, `demos/`, and others) as static files under that directory. Nothing in `dx build --ssg`'s output, nor in `dx serve`'s startup, cleans or distinguishes these prerendered route pages from the plain `index.html` a CSR dev session expects to find and mount its client app into.

The result: run `dx build --ssg` once (to test the production SSG artifact locally, or because CI's build script was copied for local reproduction), then switch back to `dx run --web`/`dx serve` for ordinary development — the dev server serves the same `public/` directory, which still contains the previous SSG build's prerendered route HTML. Visiting one of those routes in the CSR dev session serves the **prerendered static page first**, and the CSR client then mounts its own app on top of it without clearing the prerendered markup, producing duplicate page chrome (two navbars, two footers, two toast regions — anything the app renders unconditionally on every route).

### Minimal reproduction

```bash
cd preview
dx build --ssg --features fullstack --platform web --force-sequential true
# target/dx/preview/debug/web/public now contains prerendered
# component/, dashboard/, docs/, demos/ route directories.

dx run --web --port 8080
# Visit http://127.0.0.1:8080/component/dialog (or any prerendered route).
```

Inspect the served page: the prerendered SSG route's static HTML is present in the initial response body, and the CSR client's own render appears *alongside* it rather than replacing it — visible as duplicated layout chrome, and confirmable via axe's `landmark-unique`/similar duplicate-region rules, or a Playwright locator resolving to two matching elements where only one is expected (e.g. two `role="navigation"` landmarks).

### Expected vs actual

- **Expected:** `dx build --ssg`'s output directory and `dx serve`/`dx run --web`'s served directory are either kept separate by default, or `dx serve` refuses to start (or emits a loud warning) when it detects prerendered SSG route output sitting in the directory it's about to serve for CSR — the two build shapes produce structurally different content for the same route path, and silently layering one dev workflow on top of the other's leftover output is very unlikely to be anyone's intent.
- **Actual:** both build modes share one output directory with no cleanup step and no cross-mode detection; the only workaround is manually deleting the prerendered route directories before returning to CSR development.

### A related build-race note (same recipe, worth mentioning together)

Building `--ssg` with the client and server sub-builds running concurrently (the CLI's default) was observed to race on writing `index.html`: the client sub-build's `index.html` write can land after (and clobber) the server sub-build's prerendered output for the same file, non-deterministically. `--force-sequential true` (used in the reproduction above and in this repo's own CI, `.github/workflows/web.yml`) avoids the race by forcing the two sub-builds to run one after the other rather than concurrently — but this is a workaround discovered by observation, not a documented requirement, and it isn't obvious from `dx build --ssg --help` (or wasn't, as of the version this was found against) that the concurrent-by-default path has this hazard.

### Evidence

`docs/conformance-harness.md` (`MentalGear/dioxus-components`), "Hydration/deployment parity" section, "The SSG lane — how to build and run it locally" — documents the exact contamination hazard and the local workaround (snapshot the SSG build's output to a separate directory with `cp -r`, then delete the prerendered route directories from the original before going back to the CSR dev server), plus the `--force-sequential` race note above, both discovered by direct execution while building this repo's `oracle/hydration-parity.spec.ts` SSG-lane harness.

### Proposed fix

Two independent asks, either helps on its own:

1. `dx build --ssg` writes to a distinct output directory (or a distinct subdirectory) from the one `dx serve`/`dx run --web` reads from by default, so the two workflows cannot collide without an explicit `--output-dir`-style override.
2. If sharing one directory by design, `dx serve`/`dx run --web` detects prerendered SSG route output already present and either refuses to start with a clear message, or clears it automatically before serving — either is better than silently layering a CSR mount on top of stale prerendered markup with no signal to the developer.

Separately: default `dx build --ssg` to sequential client/server sub-builds (or otherwise make the shared-`index.html`-write race impossible) rather than requiring `--force-sequential` to be discovered by trial and error.

## Before filing

- [ ] Re-verify against the latest `dx` release — check whether `dx build --ssg`'s output-directory behavior or the concurrent-build race have already changed.
- [ ] Confirm the exact `dx` CLI version this was observed on (not a `Cargo.lock` entry; check local `dx --version` output at the time of filing) and state it explicitly in the filed issue.
- [ ] Link a permalink to `docs/conformance-harness.md`'s "Hydration/deployment parity" section (`MentalGear/dioxus-components`) at the commit this issue is filed from.
- [ ] Search existing `dx`/`dioxus` issues for prior SSG-output-directory reports before filing.
