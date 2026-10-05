#!/usr/bin/env bash
#
# check-eager-head-document.sh
#
# Guards the construction that makes `document::Link`/`Stylesheet`/`Meta`/
# `Script`/`Style` insertion independent of Dioxus's effect queue
# (preview/src/eager_head.rs). Invisible to `cargo check`/clippy/`cargo test`
# and only caught at runtime by playwright/oracle/tier2-html/
# stylesheets-present.spec.ts, so with CI frozen this is the cheap static
# half of that oracle.
#
# THE DEFECT (dev-docs/backlog.md, stylesheet-dedupe finding, 2026-10-04):
# dioxus-document 0.7.9's `Link` records its href in a de-duplication set
# while the component renders (`elements/link.rs:123-137`) but the web
# document appends the `<link>` from `queue_effect` on that component's own
# scope (`dioxus-web/src/document.rs:160-164`), and `Runtime::remove_scope`
# drops a removed scope's queued effects (`dioxus-core/src/runtime.rs:190-
# 193`). A subtree torn down in the turn it first rendered -- the legacy
# `/component/?name=X` shell renders `Navbar`, then redirects -- therefore
# never inserts its links, yet every later component on the page is skipped
# as "already present". It lost `language-select-*.css`, the theme picker's
# `Popover`/`Button` CSS, and (row 46) `main.css` after a redirect.
#
# What this enforces, so the fix cannot be un-done by an innocent edit:
#   1. preview/src/eager_head.rs exists, implements `Document` for a wrapper,
#      delegates `create_head_component` (the hydration handshake) and does
#      NOT reintroduce `queue_effect` (the thing that loses the link).
#   2. `App` in preview/src/main.rs installs it via `use_eager_head_document()`
#      on every client build (`#[cfg(not(feature = "server"))]`), before the
#      `Router` renders -- never on the server, where `ServerDocument` must
#      keep collecting the SSR head.
#
# Usage: scripts/check-eager-head-document.sh
# Exit 0: clean. Exit 1: a violation, with detail on stderr.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

MODULE="preview/src/eager_head.rs"
MAIN="preview/src/main.rs"
bad=0

fail() {
  echo "check-eager-head-document: $1" >&2
  bad=1
}

if [ ! -f "$MODULE" ]; then
  fail "$MODULE is missing (the construction that keeps document::Link insertion out of the effect queue)"
else
  # Strip comments so the doc text that EXPLAINS queue_effect does not trip the check.
  code=$(sed -E 's://.*$::' "$MODULE")
  printf '%s\n' "$code" | grep -qE 'impl[[:space:]]+Document[[:space:]]+for' \
    || fail "$MODULE no longer implements Document for its wrapper"
  printf '%s\n' "$code" | grep -qE 'fn[[:space:]]+create_head_component' \
    || fail "$MODULE must delegate create_head_component (the fullstack hydration handshake)"
  if printf '%s\n' "$code" | grep -qE 'queue_effect'; then
    fail "$MODULE uses queue_effect: a queued effect is dropped when its scope is removed, losing the <link> while the de-dup set still counts it"
  fi
  printf '%s\n' "$code" | grep -qE 'pub[[:space:]]+fn[[:space:]]+use_eager_head_document' \
    || fail "$MODULE must export use_eager_head_document"
fi

if [ ! -f "$MAIN" ]; then
  fail "$MAIN is missing"
else
  # The body of `pub fn App()` up to its closing brace at column 0.
  app=$(awk '/^pub fn App\(\)/{f=1} f{print} f&&/^}/{exit}' "$MAIN")
  if [ -z "$app" ]; then
    fail "could not find 'pub fn App()' in $MAIN"
  else
    printf '%s\n' "$app" | grep -q 'use_eager_head_document()' \
      || fail "App must call use_eager_head_document() so no component's head element depends on the effect queue"
    printf '%s\n' "$app" | grep -B1 'use_eager_head_document()' | grep -q 'cfg(not(feature = "server"))' \
      || fail "App's use_eager_head_document() must sit under #[cfg(not(feature = \"server\"))] (the server keeps ServerDocument)"
    # It must come before the Router renders its first component.
    install_line=$(printf '%s\n' "$app" | grep -n 'use_eager_head_document()' | head -1 | cut -d: -f1)
    router_line=$(printf '%s\n' "$app" | grep -n 'Router::<Route>' | head -1 | cut -d: -f1)
    if [ -n "${install_line:-}" ] && [ -n "${router_line:-}" ] && [ "$install_line" -gt "$router_line" ]; then
      fail "App installs use_eager_head_document() after the Router: install it first, or the first routes' links use the effect queue"
    fi
  fi
fi

if [ "$bad" -ne 0 ]; then
  exit 1
fi
echo "check-eager-head-document: OK"
