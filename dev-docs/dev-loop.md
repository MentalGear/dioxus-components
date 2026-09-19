# The fast dev loop: edit → wait → inspect → one spec → suite

This is the measured answer to "what's the fastest reliable way to see a
change working, without a full Playwright pass on a shared dev server."
Everything below was run and timed in this session (2026-09-18), on a 4-core
/ 15GB sandbox, `dioxus` 0.7.9 / `dx` 0.7.9, in a worktree-isolated
`CARGO_TARGET_DIR`. Some runs overlapped with other lanes' own `dx serve`/
Playwright activity on the same box (noted inline as "under load") — treat
those as upper bounds, not clean single-tenant numbers.

**Read [`dx-serve-hot-reload.md`](./dx-serve-hot-reload.md) first.** It
covers three traps found earlier this session (no clean "rebuild finished"
signal; two different completion strings; `Hotreloading:` with no rebuild).
This page adds two more, found while producing the table below, and turns
all five into one script.

## TL;DR recommended loop

1. **Start the server once**, log redirected to a file you know the path of:
   ```bash
   export CARGO_TARGET_DIR=/path/to/your/isolated/target-dir   # never the shared one — see below
   cd preview
   dx serve --web --port 8083 > /tmp/dx-serve-8083.log 2>&1 &
   ```
2. **Edit.**
3. **Wait for the real signal, not a guess:**
   ```bash
   scripts/dev-wait.sh 8083 --log /tmp/dx-serve-8083.log --timeout 60
   ```
   It prints `READY cold|rebuild|hotreload <Ns>: <log line>` and exits 0, or
   a timeout/ambiguous exit — see its `--help`.
4. **If it reports `hotreload`, know what that does and doesn't prove**
   (see Trap 4 below) before reaching for a fresh-navigation check.
5. **Inspect directly, in under 2 seconds against a warm server:**
   ```bash
   node scripts/inspect.mjs "http://127.0.0.1:8083/component/?name=kbd" --axe
   ```
6. **Run the one relevant spec** (not the whole suite) once you want
   Playwright-grade confidence — see "Running exactly one spec" below for
   the substring-match trap.
7. **Run the full suite only at the end**, before a commit/PR.

Measured: a warm-server `inspect.mjs` call is ~1–2s wall time (including
Chromium launch); a plain `curl` proves nothing more than "the process is
alive" (see Trap 0 below, worse than documented).

## Environment for every number below

4 CPU cores, 15GB RAM, `/` had 15GB free at the start of this session (11GB
by the end — see "Disk" under Q5). `~/.cargo/config.toml` sets
`build.target-dir` to the repo's **shared** `target/` across every worktree
— row 68 in `dev-docs/backlog.md` documents this as an active, confirmed
hazard (cross-worktree build contamination), not a hypothetical. The env var
overrides it:

```bash
export CARGO_TARGET_DIR=/home/user/dioxus-components/target-lane-<your-lane>
```

This session used `target-lane-devloop`, deleted at the end (see Cleanup).

## Q1 — Rebuild latency by edit class (plain `dx serve --web`, debug)

Each row: a from-scratch, single-purpose edit against a **warm** server
(cold build already done), measured wall-clock from just-before-`dev-wait.sh`
to its `READY` line, cross-checked with a real Chromium load
(`inspect.mjs`), not a log grep.

| # | Edit class | Where | Log signal | Latency | Served on a **fresh** page load? |
|---|---|---|---|---|---|
| a | RSX text edit in a demo | `preview/src/components/kbd/variants/main/mod.rs` (paragraph text) | `Hotreloading: <file>` only, no `Build completed` | **~0.5–0.8s** | **No** — see Trap 4. Visible instantly on an already-open, already-connected tab; never on a new navigation until a real rebuild happens for some other reason. |
| b | CSS edit, component stylesheet | `preview/src/components/kbd/style.css` (a `preview/src/components/**/style.css`, referenced via `asset!()`) | Full rebuild: `Build completed in Ns` | **~28.3s** (dx-reported 31.27s) | Yes |
| c | Rust logic edit, primitive handler body | `primitives/src/checkbox.rs`, `onclick` body (new signal + attribute) | Full rebuild | **~38.3s** (dx-reported 41.45s) | Yes |
| d | New function/module | new `preview/src/devloop_probe.rs` + `mod` decl + one call site, saved as 3 quick edits | Full rebuild — **but see Trap 5**: the first `Build completed` line (54.23s) did **not** yet contain the change; a second, trailing rebuild (34.26s later) did | dev-wait's first match: 45.7s (misleading). True end-to-end (file-mtime to wasm-mtime): **~84.6s** | Yes, but only after the *second* build |
| e | New asset file | new `preview/assets/devloop-probe.css` + a new `asset!()` `document::Link` in `preview/src/components/kbd/component.rs` (a `.rs` change) | Full rebuild | **~31.5s** | Yes |
| e′ | *Follow-up*: content-only edit to that **already-wired** asset file | same `devloop-probe.css`, value changed, no `.rs` touched | `Hotreloading: /assets/devloop-probe.css` only | **~0.86s** | Eventually yes (see Trap 4's asset-file footnote) |

**The headline finding, as first written up here: CSS is not cheap in this
repo.** Every themed component's stylesheet lives at
`preview/src/components/<name>/style.css` — under `src/`. Row (b) above
shows that costs a full ~30s rebuild, identical in kind to a Rust logic
change. Row (e′) proves CSS *can* hot-reload in well under a second — content
edits to an asset under `preview/assets/` (outside `src/`, `asset!()`
reference unchanged) do exactly that. This section originally concluded the
determining factor was the file's location relative to `src/`. **That
diagnosis was wrong — see "CSS root cause, corrected" below, which replaces
it** — moving stylesheets was never actually necessary, and rows (b)/(e′)
above are left as-is as the original, honest measurements that motivated the
follow-up, not because the location theory turned out to be right.

## CSS root cause, corrected (backlog rows 72/76 follow-up)

Row (b)'s own explanation — "it's the directory" — does not survive a
direct counter-example already sitting in this same codebase:
`preview/src/main.rs`'s shared `THEME_CSS` constant highlights
`/assets/dx-components-theme.css` for its own "Style" tab, and that file
lives under `preview/assets/`, the exact directory row (e′) called cheap —
yet, before this fix, editing it hit the identical full-rebuild path
`style.css` did, for the same reason described below. Location was never the
actual variable; it happened to correlate with the real one in every case
this session had tried so far.

**The real mechanism, found by reading dioxus-cli 0.7.9's own source
(`~/.cargo/registry/.../dioxus-cli-0.7.9/src/serve/runner.rs`,
`handle_file_change`) and `dioxus-code-macro`'s source
(`~/.cargo/registry/.../dioxus-code-macro-0.1.1/src/lib.rs`), then confirmed
live by isolating each variable in turn:**

1. `preview/src/components/mod.rs`'s `examples!` macro renders each
   component's "Style" tab by calling
   `dioxus_code::code!(concat!("/src/components/", name, "/style.css"))`.
   Reading that macro's own source shows it expands to (among other things)
   `const SOURCE: &str = include_str!(#path);` — a literal `include_str!`
   naming the `.css` file, emitted into `preview`'s own compiled source.
   `rustc` treats `include_str!` exactly like a source file: it goes into
   the compiled artifact's own dependency list (the `.d` file next to the
   binary, `RustcDepInfo` in dioxus-cli's terms).
2. `dx serve`'s file-change handler (`handle_file_change`, `runner.rs`)
   checks a changed non-`.rs` file against exactly that list
   (`artifacts.depinfo.files.contains(path)`) and, if present, sets
   `needs_full_rebuild = true` — **unconditionally overriding** the asset
   hot-reload path (`hotreload_bundled_assets`, checked first in the same
   loop) that the file's *separate* `asset!()` reference (the one that
   actually links the live stylesheet into the rendered page,
   `component.rs`) would otherwise have taken on its own. Both mechanisms
   fire for the same file; the depinfo check wins.
3. `preview/build.rs`'s blanket `cargo:rerun-if-changed=src/components`
   (a recursive directory watch) was a **second, compounding** contributor,
   not the root cause by itself: `walk_markdown_dir` unconditionally
   rewrites every component's `description.txt`/`docs.html` in `OUT_DIR` on
   every build-script run, regardless of which file triggered it, and
   `components/mod.rs` `include_str!`s those two per-component — so once
   build.rs reruns for *any* reason, cargo's mtime-based freshness check
   sees those files change too, on every component, independent of (1).

**Isolated empirically, one variable at a time, on this lane's own server
(port 8100, isolated `CARGO_TARGET_DIR`, `scripts/dev-wait.sh`; this
session's box was running 10+ concurrent lanes at a load average that
peaked past 70 on 4 cores, so treat every absolute second below as a noisy
upper bound — the *classification* (rebuild vs. hot-reload), not the
latency, is the reproducible result):**

| Step | Change | Edit | Result |
|---|---|---|---|
| Baseline | unmodified `main` | `kbd/style.css` | `READY rebuild 45.9s: Build completed in 49.72s` — full rebuild, matching row (b) |
| Isolate (3) alone | `build.rs`'s `rerun-if-changed` narrowed to the `*.md`/`component.json` paths it actually reads (the blanket directory watch removed); `code!()` embed left untouched | `kbd/style.css` (fresh edit) | `READY rebuild 37.9s: Build completed in 41.28s` — **still a full rebuild**. Narrowing `build.rs` alone does not fix it: `dx`'s classifier never consults `build.rs`'s `rerun-if-changed` at all, only the compiled artifact's own rustc dep-info, so (3) alone was never going to be sufficient — it just stops compounding (2) |
| Apply (2)+(3) together | the construction below (compile-time `code!()` embed replaced with a runtime fetch in debug builds) plus the `build.rs` narrowing | `kbd/style.css` (fresh edit) | `READY hotreload 0.8s: Hotreloading: /src/components/kbd/style.css` — **hot-reload**, confirmed live against a fresh, never-before-opened page load (not just an already-open tab — see "trap 4" below) |

**The construction (keeps `preview/src/components/<name>/` exactly as
upstream's layout and `dx components add` packaging unit — no files
moved):** a component's CSS Style tab source is compile-time-embedded (via
`dioxus_code::code!()`, unchanged) **only in release/SSG builds**
(`#[cfg(not(debug_assertions))]`); in debug builds
(`#[cfg(debug_assertions)]` — i.e. `dx serve`'s own dev loop, always CSR-only
in this repo per this doc's own recipe, never SSR/hydrated) it instead
carries just the file's own `asset!()` URL (cheap: `asset!()` does not embed
the file's text at compile time, so it registers no rustc dependency and
costs nothing extra to always compute), and a new `LazyCssCodeBlock`
component (`preview/src/main.rs`) fetches that URL's text at runtime
(`document::eval` running a plain JS `fetch`, mirroring this codebase's
other one-shot-eval hooks such as `input_otp.rs`'s `snap_caret_to_slot`) and
highlights it client-side with `dioxus_code::SourceCode`/`Language::Css`
(the crate's own runtime-highlighting API — already compiled in today,
since `preview/Cargo.toml`'s existing `dioxus-code` feature list includes
`lang-css`, and `lang-css` itself unconditionally implies `runtime` in
`dioxus-code`'s own `Cargo.toml`; no `Cargo.toml` edit was needed or made).
**Why gate on `debug_assertions` rather than `feature = "web"`/target:**
hydration parity (`dev-docs/conformance-harness.md`'s "Hydration/deployment
parity" rules) is exercised only by the SSG/fullstack lane, which is always
a release build — the debug/dev-loop path this fix changes has no server
prerender to hydrate against at all, so there is no parity surface for a
debug-only rendering difference to violate. Release output is byte-for-byte
the same code path as before this fix (nothing in `#[cfg(not(debug_
assertions))]` changed), so this is a zero-risk change to the deployed site.
Applies to all four `code!()` CSS call sites in `examples!`'s two `@demo`
arms (`style.css` ×2, `variants/demo.css` ×2) via one new
`css_highlight!` macro, plus the same class's other live instance found
above, `main.rs`'s `THEME_CSS`.

**`build.rs`'s narrowing, kept even though (3) alone doesn't fix the bug:**
it removes real, unnecessary work this build script was doing on every
single edit anywhere under any component folder (a full markdown-highlight
re-walk of all ~60 components), and it removes the *compounding* mechanism
in (3) above, in case a future change re-introduces a compile-time CSS/text
embed elsewhere. The one accepted, documented trade-off: a **brand new**
`.md`/`component.json` file isn't watched until something else causes
`build.rs` to rerun (cargo has no "watch this directory for new entries
only" primitive) — self-healing in practice (adding a new component always
also means editing `components/mod.rs`, and the very next build fails
loudly with a clear "No such file" against the missing `OUT_DIR` output
until `build.rs` is touched or `cargo clean -p preview` run), not a silent
correctness gap. See `preview/build.rs`'s own comment for the full account.

**Trap 4, re-verified against this specific fix:** a naive "did the fix
apply" check that only holds an already-open tab open across the edit would
not have caught the difference between this fix's `hotreload` classification
and the old `rebuild` one, since template hot-reload was never the mechanism
in question for a `.css` file either way. The check that actually matters —
and the one done here — is a **fresh navigation after the edit**
(`inspect.mjs` launching a brand-new browser context, never having loaded
the page before): it correctly showed the newly-edited CSS text, because
this fix's fetch runs at *request* time against whatever `dx serve`'s asset
pipeline is currently serving at that URL, not against anything baked into
the wasm binary — there is no stale-until-rebuild window here the way row
(a)'s RSX case has one.

**Verified, this lane's own server:** `cargo check -p preview` and
`cargo clippy -p preview --tests -- -D warnings` both clean in *both*
debug and `--release` profiles (the two `#[cfg(debug_assertions)]` arms
compile and lint cleanly on their own — release was not exercised by `dx
serve` at all in this session, so checking it directly mattered); `cargo
test -p preview` 23/23 passed; all four `scripts/check-*.sh` guards and
`stylelint` clean (no CSS file's content changed by this fix); a live
`inspect.mjs` check confirmed the Style tab renders the real, current CSS
(not a blank/loading placeholder) for a Normal component (`kbd`), a Block
component's `demo.css` (`sidebar`), and `THEME_CSS`, each via a fresh page
load. **Not verified:** the release/SSG code path was checked for
compilation and lint only, not rendered in a real browser (this lane's
server never runs `--release`; the code path is provably unchanged from
before this fix, so this is a compile-time-only guarantee, not a
behavioral one); Firefox/WebKit (this sandbox has Chromium only, consistent
with every other note in this file).

## Traps 4 and 5 (new this session — 1–3 are in `dx-serve-hot-reload.md`)

**Trap 0, reinforcing the existing doc's core point with a concrete
example:** during this session's own cold start, `curl` against the dev
server returned a literal HTTP **200** whose body read `Err 404 - dioxus is
not currently serving a web app`. A 200 is not even weak evidence here.

**Trap 4 — a hot-reload-only patch is invisible to any fresh navigation,
indefinitely.** RSX/template hot-reload (edit class a, and e′) is applied by
`dx` to whatever browser tab is **already open and connected** at the
moment of the edit, over a websocket, in-memory — it does **not** update the
wasm/js bytes the server has on disk. Proven directly: a page opened
*before* an RSX text edit showed the new text after the edit (confirmed via
a held-open Playwright page); two independent **fresh** page loads
afterwards — one immediately, one a full minute later — both still showed
the pre-edit text. This matters for exactly the verification method this
task asked for: a naive "reload and check with Playwright" *is* a real
browser page load, but if the edit only hot-reloaded, that check will read
as a false "the fix didn't apply" even though it applied correctly to a live
session. **Practical rule:** after a `hotreload`-classified change, either
trust it (dx's own template hot-reload is well-exercised for markup/CSS) or
verify against a page that was open *before* the edit — not a fresh
`inspect.mjs`/`curl`/new Playwright test — unless you're willing to wait for
or force a real rebuild first. The asset-file case (e′) differs slightly:
the server *did* eventually update what it serves (confirmed on a later
fresh check), just not provably within the same instant as the log line —
budget a little slack, or re-check, before concluding it failed.

**Trap 5 — rapid successive saves can produce a "successful" but
incomplete rebuild.** An agent naturally saves several files within
milliseconds of each other (new file + a `mod` line + a call site, in this
session's edit class d). `dx`'s debounce can split that into two
overlapping build cycles; the *first* `Build completed` line does not
guarantee every saved file's content is in that specific binary — a second,
"trailing" rebuild can follow. `scripts/dev-wait.sh` reports the first
match it sees, by design (a bounded, single-shot wait), so after a
**multi-file** edit, either wait for the log to go quiet for a few seconds
past the first `READY`, or (more simply) just re-run `dev-wait.sh` once more
with `--since-line` set past the first match — if it times out, you're done;
if it reports another `READY`, that was the real one.

**A sixth trap, found later (2026-09-18/19, batch-2 integration) and recorded
in `dx-serve-hot-reload.md` rather than here, since it belongs with traps
1-3's "no clean rebuild signal" family:** near-simultaneous saves to two
*different* files can make the watcher drop the rebuild entirely (no
`Hotreloading:`, no `Build completed`, ~2% CPU, indefinitely) rather than
merely mis-time it the way trap 5 above does — see that file for the full
account and how to tell the two apart.

**A methodology footnote from `getComputedStyle`:** checking whether a CSS
change landed by reading `outline-color` alone is a false-negative trap in
its own right — the browser's default computed `outline-color` is
`rgb(0, 0, 0)` whether or not any outline is actually drawn. Check
`outline-style` (or another property whose *default* isn't a plausible
"changed" value) alongside it. Same family of mistake as the existing doc's
minified-CSS-grep trap: the check, not the build, was wrong.

## Q2 — `dx serve --hot-patch`

**RSX/template edits (class a): unaffected, identical to plain mode
(~0.3s)** — template hot-reload is a separate mechanism from hot-patch and
runs regardless of the flag.

**Rust logic edits: fails, loudly, and the failure is persistent for the
rest of that server's life.** A logic edit inside `primitives/src/checkbox.rs`
(a workspace-local dependency crate, not the top-level `preview` binary)
produced:

```
ERROR Build failed: Missing rustc args for replay: 'dioxus_primitives'
```

within ~14s, not a silent hang. `dev-wait.sh` correctly timed out (no
`Hotreloading:`/`Build completed` line ever appeared) rather than
false-reporting success — **this specific failure mode is not the silent
trap 3; it is a loud, log-visible error**, which is the better of the two
outcomes for anyone relying on the log. A **second**, unrelated logic edit
in `preview/src/theme.rs` (the top-level crate only, no `primitives` touch)
was tried next, on the *same* still-running hot-patch server: it failed with
the **identical** error, twice. This means the first failure poisons the
session — hot-patching does not recover on its own for any subsequent
edit, even one that never touches the crate that triggered the original
failure. A full server restart was not tried (time-boxed out of this
session) but is the only plausible recovery given the symptom.

**Practical conclusion:** `--hot-patch` is not usable as this workspace's
default mode today. This is a multi-crate workspace
(`primitives`/`preview`/others) and editing `primitives` — a very common
edit here, it's a component-primitives library — appears to break
hot-patching for the rest of that server's run. Stick to plain `dx serve
--web` and rely on template hot-reload (fast, works) plus full rebuilds
(slow but correct) rather than hot-patch, unless/until this is re-verified
against a newer `dx` or a single-crate edit pattern.

**`--web --release`: not run.** Cargo.toml's `[profile.release]` sets
`lto = true, codegen-units = 1, opt-level = "z"` — all three multiply
compile time over debug — and this session's isolated `CARGO_TARGET_DIR` had
zero release-profile cache. The plain debug cold build alone (below) took
233s; a release cold build was judged not "cheap" per this task's own
instruction to skip it if not, and was not run to avoid burning the
remaining time/disk budget on a number this session couldn't act on anyway.
`dev-docs/dx-serve-hot-reload.md` and `scripts/deploy-preview.sh` already
describe release/SSG builds as "a few minutes" from a warm cache; a release
*cold* build (this session's disk/time situation) should be assumed
noticeably worse than that, not measured precisely here.

**Cold build costs actually measured, isolated `CARGO_TARGET_DIR`:**
- Plain mode, completely empty target dir (every dependency from scratch):
  **233.29s** (~3.9 min), 2.1GB on disk (shared by both servers below).
- Hot-patch mode, *same* already-warmed target dir (dependency crates
  already compiled by the run above): **124.47s**. This is **not** a clean
  "hot-patch is 2× faster to cold-start" result — it's mostly dependency-
  compilation cache reuse from the first server. Don't quote it as a
  hot-patch-vs-plain comparison without re-isolating both.

## Q3 — A trustworthy "my change is live" signal

Implemented as **[`scripts/dev-wait.sh`](../scripts/dev-wait.sh)** — see its
own `--help` for full usage. Summary of the design decision:

**(a) Log parsing — what was implemented, and why.** The log already
distinguishes a cold start (`Build completed successfully in Ns`) from a
watch rebuild (`Build completed in Ns`, no "successfully") and a
hot-reload-only patch (`Hotreloading: <file>`, no completion line). This is
fully proven, well-understood (thanks to the existing doc plus this
session's Traps 4/5), and needs zero source changes. `dev-wait.sh` takes a
port + log path, snapshots a baseline line, polls for the next matching
line with a bounded timeout, and exposes `--expect rebuild` to *fail* (exit
3) rather than false-succeed when only a lone `Hotreloading:` shows up for
an edit you know is Rust logic (trap 3's exact shape).

**(b) A WebSocket dx already runs — confirmed, evaluated, not made the
primary mechanism.** Reading `dioxus-cli` 0.7.9's own source
(`~/.cargo/registry/.../dioxus-cli-0.7.9/src/serve/server.rs`) and then
confirming live: `dx serve` exposes `/_dioxus/build_status`, an
adjacently-tagged JSON websocket:

```json
{"type":"ClientInit","data":{"application_name":"preview","bundle":"web"}}
{"type":"Building","data":{"progress":0.34,"build_message":"dioxus_core compiling"}}
{"type":"Ready"}
```

captured live from this session's own cold start. This is a genuinely nice
signal (fine-grained `Building` progress, an explicit `Ready`), **but it has
a gap that rules it out as the sole mechanism**: `set_ready()` in the CLI
source is a no-op once status is already `Ready` — so a *second*
hot-reload-only edit in a row produces **no new message at all** on this
socket (confirmed by source reading; the actual template patch travels over
a separate hot-reload socket this session did not fully characterize — a
real avenue for follow-up work, not pursued further here given the log
already covers every case needed). Log parsing has no equivalent blind spot.
The websocket finding is recorded here because it's useful — e.g. for a
richer progress UI — but `dev-wait.sh` uses the log as its one source of
truth.

**(c) A build-stamp `<meta>`/`data-build` attribute — evaluated, not
implemented.** Considered seriously (per this task's instructions) and
rejected for a concrete, evidence-based reason: `preview/build.rs` already
emits `cargo:rerun-if-changed=src/components` (for its markdown pipeline).
Once a build script emits *any* `rerun-if-changed`, cargo's default
"rerun on any crate-source change" behavior is replaced entirely by *only*
the paths explicitly listed — so a stamp generated in `build.rs` would go
stale on exactly the edit classes that most need proving (a plain
`main.rs`/primitive edit doesn't touch `src/components`, so the build
script wouldn't rerun and the embedded stamp wouldn't change). Fixing that
would mean either broadening `build.rs`'s rerun scope (changing existing,
working behavior for its actual job, for other contributors, out of this
task's narrow permitted-edits list) or a proc-macro/new build dependency
(disallowed — no new Cargo dependencies). The (b) websocket already answers
"is dx building right now / did it just finish" with zero source changes
and zero hydration-parity risk, which is what a stamp would have bought at
strictly higher cost and risk. Net: not built, in favor of (a) as
implemented and (b) as documented.

## Q4 — Direct inspection: `scripts/inspect.mjs`

```bash
node scripts/inspect.mjs "http://127.0.0.1:8083/component/?name=kbd"
node scripts/inspect.mjs "http://127.0.0.1:8083/component/?name=kbd" --dark --screenshot /tmp/kbd-dark.png
node scripts/inspect.mjs "http://127.0.0.1:8083/component/?name=dialog" \
  --click 'button:has-text("Open")' --axe
node scripts/inspect.mjs "http://127.0.0.1:8083/" \
  --eval 'return document.querySelectorAll(".dx-component-card").length'
```

Uses `playwright` from `playwright/node_modules` directly (no test runner,
no spec file). **One-time per worktree:** Node's ESM resolver walks up from
the *script's own* path looking for `node_modules`, and `scripts/` is a
sibling of `playwright/`, not a descendant — a fresh worktree needs one
symlink so `node scripts/inspect.mjs` can find the `playwright`/
`@axe-core/playwright` packages already installed under `playwright/`:
```bash
ln -s playwright/node_modules node_modules   # from the worktree root; gitignored, not committed
```
This session did exactly that rather than `npm install` a second copy — see
the task's own Node-package constraint. See the script's own header comment
for the full flag list
(`--variant`, `--dark`, `--viewport`, `--click`/`--press` in given order,
`--eval`, `--axe`, `--screenshot`, `--root`, `--timeout`). Two
implementation notes worth recording:

- **Playwright 1.60 removed `page.accessibility.snapshot()`.** This repo's
  installed version (`playwright/node_modules/playwright`, 1.60.0) has no
  `page.accessibility` at all — confirmed by direct probe. `inspect.mjs`
  uses `locator(root).ariaSnapshot()` (a readable YAML ARIA tree) instead,
  with a fallback to the legacy API if it's ever present. The task
  description's "`page.accessibility.snapshot()` *or* ARIA snapshot"
  phrasing anticipated exactly this; worth knowing if you're writing new
  Playwright code against this repo's pinned version generally.
- **`@axe-core/playwright` requires a page from an explicit
  `browser.newContext()`** — `browser.newPage()` alone (which normally just
  wraps context creation) makes `AxeBuilder.analyze()` throw "Please use
  browser.newContext()". `inspect.mjs` always creates an explicit context.

**Readiness signal:** `playwright/axe.ts` itself defines no shared "app is
ready" helper — every spec in this repo waits on its own page-specific
selector (`preview.spec.ts` waits on `#hero`, component pages on a
heading). Since `inspect.mjs` has to work against an arbitrary URL, it
generalizes that pattern instead of hardcoding one page's selector: it
waits for `#main` (the Dioxus mount point, from `preview/index.html`) to
have at least one child element. This does **not** wait for every linked
stylesheet to finish loading — see the `getComputedStyle` footnote under
Trap 5 if you're checking CSS immediately after navigation.

**Measured run time against a warm server:** ~1.0–1.5s wall time for a
DOM-outline-only run (browser launch dominates); ~4.7s with `--click` +
`--press` + `--axe` + `--screenshot` all together (axe's own scan is the
biggest single cost). All measured numbers above, not estimates.

## Q5 — Per-lane servers

**Recipe for N concurrent worktrees**, each fully isolated:

```bash
# In each worktree:
export CARGO_TARGET_DIR=/home/user/dioxus-components/target-lane-<name>
cd preview && dx serve --web --port 808<3+N> > /tmp/dx-serve-808<3+N>.log 2>&1 &
```

- **Port:** 8083+, one each (this session used 8083 and 8084; the
  coordinator noted other lanes on 8084–8086 concurrently — pick one no one
  else has claimed).
- **First-build cost per lane, measured:** 233.29s wall, 2.1GB on disk, from
  a completely empty `CARGO_TARGET_DIR` (this session's own cold start — see
  Q2). Budget both per lane; they do not share (row 68's whole point).
- **Disk:** started this session at 15GB free, ended at 11GB (one isolated
  debug target dir plus Playwright artifacts). N lanes cost roughly
  `N × 2.1GB` at minimum for debug-only builds — check free space before
  spinning up many at once.
- **Stopping your own server:** kill it **by PID**, captured at start
  (`... & echo $!`). Never `pkill -x dx` — row 72 in `backlog.md` already
  documents this killing an unrelated lane's server mid-session; this task's
  own instructions independently prohibit it. The sandbox's own auto-mode
  classifier additionally blocks killing the *main* `dx serve` outright
  (PID-targeted or not) — killing your own separately-started test server is
  fine and was done routinely in this session.

**Pointing Playwright at a non-default port.** `playwright/playwright.config.ts`
sets no `baseURL`, so all ~60 spec files navigate via a hardcoded
`http://127.0.0.1:8080` literal (176 occurrences counted across 69 files
this session, including config/comment mentions) — three different local
patterns already existed for this before today (`drag_and_drop_list.spec.ts`'s
own `const BASE = process.env.PLAYWRIGHT_BASE_URL ?? "..."`,
`sidebar.spec.ts`'s local unexported `const BASE_URL = "..."`, and every
other file's inline literal repeated per call). **Implemented**: a single
shared helper, [`playwright/base-url.ts`](../playwright/base-url.ts):

```ts
export const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:8080";
```

— consolidating those three prior patterns into one (same env var name as
the pre-existing `drag_and_drop_list.spec.ts`, same exported name as the
pre-existing `sidebar.spec.ts`, so both migrations below were a one-line
import swap with **zero call-site changes**). Migrated as a proof of the
pattern across every quote/usage style actually found in the repo:
`avatar.spec.ts` and `toolbar.spec.ts` (inline double-quoted literal, the
majority style), `sidebar.spec.ts` and `drag_and_drop_list.spec.ts` (the two
pre-existing local variants). **The other ~65 files were deliberately left
alone** — the literal-replacement is mechanical in principle but the actual
occurrences mix double/single/template-literal quoting and some already
interpolate other variables (e.g. `computed-style-snapshot.spec.ts`'s
`` `http://127.0.0.1:8080/component/?name=${name}&` ``); a blind sed across
69 files risked silent breakage this task's time budget couldn't fully
review. The default is byte-identical to before (same literal), so every
unmigrated spec keeps working exactly as-is. A safe mechanical follow-up for
the rest: replace `"http://127.0.0.1:8080` → `` `${BASE_URL}` `` (and the
single-quote equivalent) file by file, add
`import { BASE_URL } from "./base-url";`, and re-run each touched file once
before moving to the next.

**Verified, live, against a non-default port** (`PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083`,
`playwright/session.local.config.ts`, this session's own port-8083 server):
`toolbar.spec.ts` 2/2 passed; `avatar.spec.ts` 1/2 passed, its second
(axe scan) test failing on `page-has-heading-one` — a **pre-existing**
content finding (the `/component/?name=avatar` page has no `<h1>`),
unrelated to the base-URL mechanism itself (the URL construction is
byte-identical modulo host/port, and the *other* three tests, including
avatar's own functional assertions on that exact page, all passed against
the same port). Not fixed here — out of this task's scope — but worth
filing separately. The whole 4-test run took ~4 minutes, considerably
slower than expected for 4 simple tests; this session's own hot-patch
server was mid-build on port 8084 at the same time, consistent with the
coordinator's concurrent-load warning — treat this run's wall time as
noisy, not as a spec-runtime baseline.

**Running exactly one spec — the substring trap, confirmed live:**

```bash
npx playwright test --config=session.local.config.ts select.spec.ts --list
#   -> select.spec.ts AND native_select.spec.ts (confirmed this session)
npx playwright test --config=session.local.config.ts ./select.spec.ts --list
#   -> select.spec.ts only
```

Playwright's positional file arguments are **substring matches** against
every discovered spec path, not exact filenames — `select.spec.ts` matches
anything containing that text, including `native_select.spec.ts`. Anchor it
(a leading `./`, or a full relative path with a `/`) to get exactly one
file, or use `--grep`/`-g <regex>` to filter by test *title* instead of file
path. Other useful exact flags (from this session's own
`npx playwright test --help`): `--repeat-each <N>`, `--reporter=line` (both
used in this session's own verification run above), `--grep-invert <regex>`.

## What's still unknown / not verified this session

- Hot-patch recovery: whether a server **restart** clears the "Missing
  rustc args for replay" poisoned state (very likely yes; not tried).
- The exact websocket path/wire-format `dx` uses for the actual hot-reload
  *patch* payload (distinct from `/_dioxus/build_status`, which only
  reports build progress/readiness) — source-read far enough to know it
  exists (`hot_reload_sockets` in `server.rs`) but not far enough to give
  its URL or message shape.
- `--web --release` timing (judged not cheap; see Q2).
- Whether the remaining ~65 unmigrated spec files' literal-to-`BASE_URL`
  conversion is 100% mechanical once actually attempted (the sampled four
  covered every *pattern* found, not every file).
- Firefox/webkit behavior for anything above — this entire session ran
  Chromium only (`/opt/pw-browsers` has no other engine installed in this
  sandbox, consistent with `backlog.md` row 4's 2026-09-14 addendum).

## Cleanup performed at the end of this session

Both test servers (ports 8083, 8084) stopped by PID; `target-lane-devloop`
(`CARGO_TARGET_DIR`) removed; scratch files (`preview/src/devloop_probe.rs`,
`preview/assets/devloop-probe.css`, `tmp-devloop-scratch/`) removed; every
experimental source edit (`primitives/src/checkbox.rs`, `preview/src/main.rs`,
`preview/src/theme.rs`, `preview/src/components/kbd/**`) reverted via
`git checkout` before the final commit — see this session's own final report
for the exact commands and confirmation.
