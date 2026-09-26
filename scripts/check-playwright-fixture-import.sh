#!/usr/bin/env bash
#
# check-playwright-fixture-import.sh
#
# Every Playwright spec must take its runtime `test`/`expect` from
# `playwright/fixtures.ts`, never straight from "@playwright/test".
#
# fixtures.ts overrides the worker-scoped `browser` fixture so every
# BrowserContext gets `html { scroll-behavior: auto !important }`: the
# site's global `scroll-behavior: smooth` otherwise animates Playwright's
# own actionability pre-scroll and drags hover/click targets out from under
# the cursor (dev-docs/backlog.md row 110). A fixture only reaches tests
# declared through the extended `test` object, so a spec that imports the
# stock one silently opts out -- which happened once already, to
# navbar.spec.ts, the day the fixture landed. Type-only imports
# (`import type { Page } ...`, `import { type Page } ...`) are fine.

set -euo pipefail
cd "$(dirname "$0")/.."

bad=0
while IFS= read -r file; do
  # Collapse each import statement onto one line so multi-line braces match.
  while IFS= read -r stmt; do
    names=$(printf '%s' "$stmt" | sed -E 's/^import[[:space:]]*\{([^}]*)\}.*/\1/')
    for name in $(printf '%s' "$names" | tr ',' ' '); do
      case "$name" in
        test|expect|devices)
          echo "$file: imports runtime '$name' from \"@playwright/test\" -- import it from fixtures.ts instead"
          bad=1 ;;
      esac
    done
  done < <(tr '\n' ' ' < "$file" | grep -oE "import[[:space:]]*\{[^}]*\}[[:space:]]*from[[:space:]]*['\"]@playwright/test['\"]" || true)
done < <(find playwright -name '*.spec.ts' -not -path '*/node_modules/*')

if [ "$bad" -ne 0 ]; then
  exit 1
fi
echo "check-playwright-fixture-import: OK"
