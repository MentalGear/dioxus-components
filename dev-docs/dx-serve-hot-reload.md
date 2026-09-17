# Working with `dx serve`'s hot reload (agent notes)

This is a practical guide for verifying a change against `dx serve`, written from
mistakes actually made this session. `dx serve` **does** auto-rebuild on file
change — there is no need to manually kill/restart it for a normal edit. The
problems below are not about *whether* hot reload works; they are about
*knowing when it has finished*, and about test methodology mistakes that look
exactly like a stale build but aren't.

## The core problem: no clean "rebuild finished" signal

`dx serve` watches the filesystem and kicks off a rebuild when a source file
changes. A plain `curl -o /dev/null -w '%{http_code}' http://localhost:8080/`
returning `200` tells you the HTTP server is up — it does **not** tell you the
asset pipeline has finished reprocessing CSS/JS/wasm for your latest edit. A
request can land while a rebuild is still in flight, or immediately before
one has even started, and get served whatever was there before.

**What to check instead:** tail the server's own log for a completion line —
`Build completed successfully in <N>s` (dev builds) or `Client build completed
successfully! 🚀` / `Server build completed successfully! 🚀` (release/SSG
builds via `scripts/deploy-preview.sh`). Don't treat a bare HTTP 200 as proof
of anything past "the process is alive."

```bash
# after touching a source file, poll the log rather than just curl:
grep -q "Build completed successfully" dx-serve.log
```

If you changed a file and the log shows no new "Build completed" line after a
reasonable wait, the watcher may not have picked up the change at all (e.g. a
file outside its watched globs, or a change made via `git stash`/`pop` landing
in a window the watcher's debounce missed) — that's the moment to stop
guessing and force a real rebuild (see "When in doubt" below).

## Testing methodology traps that *look* like a stale build

Two mistakes this session produced symptoms indistinguishable from "the dev
server is serving old code," when the code was actually fine:

1. **Grepping built CSS for un-minified text.** Source CSS has spaces
   (`.foo { flex-direction: column; }`); the asset pipeline's *built* output
   is minified (`.foo{flex-direction:column}`). A `grep "flex-direction: column"`
   against the built file will silently find nothing even when the rule is
   correctly present — this looks exactly like "the fix never made it into the
   build." Grep the minified form (no spaces around `:`/`{`/`}`), or don't
   grep raw text at all — query `document.styleSheets`/`getComputedStyle` in
   a real browser instead, which doesn't care about formatting.

2. **`elementFromPoint` after adding `pointer-events: none`.** If a fix makes
   an overlay `pointer-events: none` (so clicks pass through to something
   underneath), `document.elementFromPoint(x, y)` will *also* skip that
   overlay — it respects `pointer-events` for hit-testing. You cannot use it
   to ask "what's visually at this point" once you've done that; you'll only
   ever see what's underneath. To find which visual element covers a point
   after such a fix, query each candidate element's own
   `getBoundingClientRect()` and do a manual point-in-rect check instead.

Both of these produced a "the fix regressed something" false alarm this
session before the real cause (test methodology, not a build problem) was
found. When a fresh regression appears immediately after a fix that should be
unrelated to the symptom, suspect the test before the build.

## When you're not sure if what you're seeing is real

Ordered from cheapest to most expensive/most certain:

1. **Re-check after confirming a "Build completed" log line**, not just an
   HTTP 200. Often this alone resolves it.
2. **`git stash` the fix, wait for a confirmed rebuild, test again.** If the
   symptom reproduces identically on the unmodified baseline, it's
   pre-existing, not caused by your change — `git stash pop` and move on. This
   is the fastest way to rule out "did I break this."
3. **Wipe the dev build's own output directory and force a clean rebuild**
   (`rm -rf target/dx/preview/debug/web/public` before restarting `dx serve`)
   if you suspect the incremental build itself is confused, not just slow.
4. **Build via the actual production recipe** (`bash scripts/deploy-preview.sh`,
   or `dx build --platform web --release --ssg --features fullstack`) and
   serve *that* output instead of the dev server. This is a fully
   recompiled, non-incremental artifact — the closest thing to ground truth
   available in this sandbox, and worth the extra build time (a few minutes)
   whenever dev-server ambiguity has already cost you one confusing result.
   `deploy-preview.sh` also runs its own hydration-bootstrap sanity check
   (see its own comments) guarding against a known, separate `dx build --ssg`
   cache-warm race — a different, already-documented issue from anything
   above, but worth knowing about if a *production* build looks broken in a
   way a dev build doesn't.

Serving a production build locally at its real base path (`/dioxus-components`)
needs a one-line symlink trick, since the built HTML references absolute
paths under that prefix:

```bash
mkdir -p /tmp/docs-root && ln -sfn "$(pwd)/target/dx/preview/release/web/public" /tmp/docs-root/dioxus-components
cd /tmp/docs-root && python3 -m http.server 8099
# then browse http://localhost:8099/dioxus-components/
```

## Summary

- Hot reload works and should be relied on for normal edits — this doc is not
  an argument against using it.
- Don't trust "the server responds" as "my change is live." Trust a logged
  build-completion line, or a `git stash`-based A/B comparison, or (when
  genuinely unsure) a from-scratch production build.
- Before concluding a fix caused a regression, rule out your own test
  methodology first — minified-CSS text grepping and `elementFromPoint` after
  a `pointer-events: none` fix are the two traps already caught here.
