#!/usr/bin/env bash
#
# check-css-delivery.sh
#
# Static half of playwright/oracle/tier2-html/first-paint-styles.spec.ts: the construction that keeps a route's CSS, and
# the web fonts, from arriving after first paint cannot be un-done by an innocent edit. Invisible to cargo and clippy, and
# with CI frozen (dev-docs/backlog.md row 33) nothing else would notice.
#
# THE CLASS (measured 2026-10-05): a stylesheet discovered at RENDER time is inserted after first paint, and a late
# `<link>` is not render-blocking (`blocking=render` measured as a no-op once <body> exists). A client-side navigation to
# /component/field/ or a popover's first open therefore painted white UA inputs / UA buttons, then reflowed (5-10 late
# sheets, 150-270 ms unstyled, CLS 0.02-0.08 on 8 of 8 pages sampled). The construction:
#   1. scripts/ssg-css-bundle.mjs puts every component stylesheet in every page's <head> as ONE render-blocking bundle;
#   2. scripts/build-ssg.sh runs it by default (guard 4b) and scripts/deploy-preview.sh never turns it off;
#   3. preview/src/eager_head.rs skips a stylesheet link whose file the bundle already carries (data-covers);
#   4. GlobalHead preloads the two latin Geist woff2 files and asks Google for display=optional, so the font swap cannot
#      re-wrap text after first paint (CLS 0.117 -> 0.000 on /component/form/).
#
# Usage: scripts/check-css-delivery.sh
# Exit 0: clean. Exit 1: a violation, with detail on stderr.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

bad=0
fail() {
  echo "check-css-delivery: $1" >&2
  bad=1
}

BUNDLER="scripts/ssg-css-bundle.mjs"
BUILD="scripts/build-ssg.sh"
DEPLOY="scripts/deploy-preview.sh"
EAGER="preview/src/eager_head.rs"
MAIN="preview/src/main.rs"

# 1. the bundler exists and still refuses what a concatenated sheet cannot hold.
if [ ! -f "$BUNDLER" ]; then
  fail "$BUNDLER is missing (the per-route stylesheet set is back: client-side navigation paints before its CSS)"
else
  grep -q 'data-dx-css-bundle' "$BUNDLER" || fail "$BUNDLER no longer marks its link with data-dx-css-bundle (eager_head and the oracle look for it)"
  grep -q 'data-covers' "$BUNDLER" || fail "$BUNDLER no longer writes data-covers (eager_head cannot skip covered links)"
  grep -q '@import' "$BUNDLER" || fail "$BUNDLER no longer rejects @import/@charset/url() in a sheet it concatenates"
fi

# 2. the build runs it by default; deploy never switches it off.
if [ ! -f "$BUILD" ]; then
  fail "$BUILD is missing"
else
  grep -q 'ssg-css-bundle.mjs' "$BUILD" || fail "$BUILD no longer runs scripts/ssg-css-bundle.mjs (guard 4b)"
  grep -q '^css_bundle=1' "$BUILD" || fail "$BUILD must bundle by default (css_bundle=1); --no-css-bundle is for before/after measurements only"
fi
if [ -f "$DEPLOY" ] && grep -q 'no-css-bundle' "$DEPLOY"; then
  fail "$DEPLOY passes --no-css-bundle: a deployed site without the bundle flashes unstyled content on client-side navigation"
fi

# 3. eager_head still skips covered links (comments stripped so the docs that explain it do not satisfy the check).
if [ ! -f "$EAGER" ]; then
  fail "$EAGER is missing"
else
  code=$(sed -E 's://.*$::' "$EAGER")
  printf '%s\n' "$code" | grep -qE 'fn[[:space:]]+create_link' || fail "$EAGER no longer overrides create_link (the covered-sheet skip)"
  printf '%s\n' "$code" | grep -q 'data-dx-css-bundle' || fail "$EAGER no longer consults link[data-dx-css-bundle]"
  printf '%s\n' "$code" | grep -q 'data-covers' || fail "$EAGER no longer reads data-covers"
fi

# 4. fonts: preloaded latin files, display=optional (a shift-proof pair; either alone is not: see GlobalHead's doc).
if [ ! -f "$MAIN" ]; then
  fail "$MAIN is missing"
else
  head=$(awk '/^fn GlobalHead\(\)/{f=1} f{print} f&&/^}/{exit}' "$MAIN")
  if [ -z "$head" ]; then
    fail "could not find 'fn GlobalHead()' in $MAIN"
  else
    n=$(printf '%s\n' "$head" | grep -c 'rel: "preload"' || true)
    [ "$n" -ge 2 ] || fail "GlobalHead must preload the Geist and Geist Mono latin woff2 files (found $n preload links): without them the face arrives after first paint"
    printf '%s\n' "$head" | grep -q 'r#as: "font"' || fail "GlobalHead's font preloads must say r#as: \"font\""
    printf '%s\n' "$head" | grep -q 'crossorigin: "anonymous"' || fail "GlobalHead's font preloads need crossorigin (a font preload without it is fetched twice and never used)"
    printf '%s\n' "$head" | grep -q 'display=optional' || fail "GlobalHead's Google font stylesheet must use display=optional (display=swap re-wraps the page when Geist arrives: CLS 0.117 measured)"
    if printf '%s\n' "$head" | grep -q 'display=swap'; then
      fail "GlobalHead still asks for display=swap"
    fi
  fi
fi

if [ "$bad" -ne 0 ]; then
  exit 1
fi
echo "check-css-delivery: OK"
