#!/usr/bin/env bash
#
# check-uncleared-intervals.sh
#
# Keeps a repeating timer from being written in a shape that nothing can
# ever clear. A JS `setInterval` started through `document::eval` (or from
# any script) is owned by the PAGE, not by the Dioxus scope that evaluated
# it: dropping the component drops the task that listens to it, but the
# interval itself keeps firing for the life of the page. Written from a
# `use_effect`, it is also re-created every time that effect re-runs, and
# every visit to the route adds another. Nothing in `cargo check`, clippy
# or `cargo test` can see it.
#
# The incident this guards against (dev-docs/research/scroll-jank-2026-10-04.md,
# Cause 1): `BlockPlayer` (`preview/src/main.rs`) started a 100 ms
# `setInterval` from a `use_effect` and never cleared it, and its hooks were
# being re-run (see `MasonryCard`'s doc: a block rendered through a
# `Callback` runs its hooks in the PARENT's scope), so a fresh interval
# appeared about every 1.2 s: 128 live intervals after ~100 s, 1,326 timer
# wakeups per second at 155 s, 387 layouts during one scroll instead of 0-1.
# The Progress demo had the same shape at 1 s. Fixed by construction, not
# per instance: the one sanctioned timer is `use_interval`
# (`primitives/src/interval.rs`), a task spawned in the calling component's
# scope (`dioxus_sdk_time`: `tokio::time::interval` on native, `gloo_timers`
# on wasm). It is cancelled with the scope on unmount, so there is nothing to
# forget to clear, and there is no JS timer in it at all. This script keeps
# the old shape from coming back: it fails on any
#
#   setInterval(                     (JS, including eval strings in Rust)
#   set_interval                     (web-sys / js-sys wrappers)
#   Interval::new(                   (gloo-timers callback intervals)
#   spawn_forever(                   (a task Dioxus will NOT cancel on unmount)
#
# in primitives/src or preview/src (`*.rs`, `*.js`, `*.ts`) or
# preview/index.html. A genuinely page-lifetime timer is allowed with
#
#   // interval-ok: <reason>
#
# on the same line or on the line directly above it (a Rust/JS comment; in an
# eval string, put it on the Rust line above the literal).
#
# WHAT THIS DOES NOT CATCH, said plainly: a self-re-arming `setTimeout`
# chain or a `requestAnimationFrame` loop is the same class with a different
# spelling, and a grep cannot tell a bounded one-shot from a perpetual one.
# `playwright/oracle/tier2-html/scroll-main-thread.spec.ts` is the runtime
# backstop: it counts live intervals AND timer/frame callbacks on `/` while
# the page is idle and fails when the count grows with page age.
#
# Comment handling: `//` line comments are ignored, so prose that names the
# old API (like this file's neighbours) does not trip it. A marker comment is
# looked for before stripping.
#
# Usage: scripts/check-uncleared-intervals.sh [file ...]
#   With no arguments scans the whole tree (well under a second). Explicit
#   files are scanned as given (used to prove the gate red against old code).
# Exit 0: clean. Exit 1: offending lines on stderr as file:line.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [ "$#" -gt 0 ]; then
  files=("$@")
else
  mapfile -t files < <(
    {
      find primitives/src preview/src -type f \( -name '*.rs' -o -name '*.js' -o -name '*.ts' \)
      [ -f preview/index.html ] && echo preview/index.html
    } | sort
  )
fi

# Candidate lines come from one fast grep over every file; only the (few) hits are then checked
# for being inside a `//` comment or carrying the marker, so the whole tree scans in well under a second.
pattern='setInterval[[:space:]]*\(|set_interval|Interval::new[[:space:]]*\(|spawn_forever[[:space:]]*\('

bad=0
while IFS=: read -r f lineno line; do
  [ -n "$f" ] || continue
  code="${line%%//*}"
  [[ "$code" =~ $pattern ]] || continue
  prev=""
  if [ "$lineno" -gt 1 ]; then prev="$(sed -n "$((lineno - 1))p" "$f")"; fi
  if [[ "$line" == *"interval-ok:"* || "$prev" == *"interval-ok:"* ]]; then
    continue
  fi
  echo "$f:$lineno: uncleared timer -- use primitives' \`use_interval\` (a component-owned task), or mark a genuine page-lifetime timer with \`// interval-ok: <reason>\`" >&2
  echo "    ${line#"${line%%[![:space:]]*}"}" >&2
  bad=1
done < <(grep -nHE "$pattern" "${files[@]}" 2>/dev/null || true)

if [ "$bad" -ne 0 ]; then
  echo "check-uncleared-intervals: FAIL (dev-docs/research/scroll-jank-2026-10-04.md, Cause 1)" >&2
  exit 1
fi
echo "check-uncleared-intervals: OK -- no setInterval / spawn_forever outside an explicit interval-ok marker."
