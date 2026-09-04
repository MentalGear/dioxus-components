#!/usr/bin/env bash
#
# check-css-literals.sh
#
# Enforces docs/backlog.md row 31b: a themed component stylesheet must not
# hard-code a value that the design-token set (docs/backlog.md row 31a,
# `preview/assets/dx-components-theme.css`) already names.
#
#   padding: 0.5rem;              -> padding: var(--dx-space-2);
#   border-radius: 0.5rem;        -> border-radius: var(--dx-radius-lg);
#   transition: opacity 150ms;    -> transition: opacity var(--dx-motion-duration-base);
#
# Why: a token set only pays for itself if the components actually read from
# it. A literal that duplicates a token's value is a value that will silently
# fail to follow when the token is retuned -- exactly the drift row 31a's
# survey found across the 46 stylesheets, and the reason row 31b exists.
#
# WHAT IS AND IS NOT AN ERROR
#
# Only an EXACT match, in the token's own unit, is an error. `0.5rem` where
# `--dx-space-2` is `0.5rem` fails. `8px` does NOT fail, even though it
# computes to the same 16 CSS pixels at the default root font-size: swapping
# it for a rem token is a genuine behavior change under a consumer that
# overrides the root font-size, so it is a design decision rather than a
# mechanical one. Those cross-unit candidates are printed as informational
# notes and never affect the exit status.
#
# Near-miss values (`0.85rem`, `13px`, `26px`, ...) match no token at all and
# are not reported: normalising them onto the nearest step changes rendered
# output, which is deliberately not this lint's business.
#
# Only the properties the token set actually covers are scanned. Colors,
# `line-height` and `font-weight` have no tokens (row 31a surveyed them and
# found no clean clusters) and are ignored.
#
# Usage: scripts/check-css-literals.sh
# Exit status: 0 if no component stylesheet hard-codes a tokenised value.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'PYEOF'
import re
import sys
import glob

THEME = "preview/assets/dx-components-theme.css"

# Properties the token set covers. Anything else is out of scope by design.
SCANNED = re.compile(
    r"^\s*(font-size|padding|padding-top|padding-right|padding-bottom|padding-left"
    r"|margin|margin-top|margin-right|margin-bottom|margin-left|gap|row-gap|column-gap"
    r"|border-radius|box-shadow|transition|transition-duration|z-index)\s*:\s*([^;]+);"
)

def norm(v):
    """Normalise a value for comparison: lowercase, `.5rem` -> `0.5rem`."""
    v = v.strip().lower()
    return re.sub(r"(^|[\s(,])\.(\d)", r"\g<1>0.\2", v)

# Token name -> value, taken from the theme file's own definitions.
tokens = {}
for m in re.finditer(r"(--dx-[a-z0-9-]+)\s*:\s*([^;]+);", open(THEME).read()):
    tokens[m.group(1)] = norm(m.group(2))

# Which token FAMILY each property draws from. Without this the suggestion
# would be wrong wherever two scales share a value -- `1rem` is both
# `--dx-text-base` and `--dx-space-4`, so a bare value->token map would tell
# you to write `gap: var(--dx-text-base)`, which is the wrong token even
# though it renders the same today. A lint that suggests the wrong fix is
# worse than no lint: it teaches the mistake it exists to prevent.
FAMILY = {
    "font-size": "--dx-text-",
    "padding": "--dx-space-", "padding-top": "--dx-space-",
    "padding-right": "--dx-space-", "padding-bottom": "--dx-space-",
    "padding-left": "--dx-space-",
    "margin": "--dx-space-", "margin-top": "--dx-space-",
    "margin-right": "--dx-space-", "margin-bottom": "--dx-space-",
    "margin-left": "--dx-space-",
    "gap": "--dx-space-", "row-gap": "--dx-space-", "column-gap": "--dx-space-",
    "border-radius": "--dx-radius-",
    "transition": "--dx-motion-", "transition-duration": "--dx-motion-",
    "z-index": "--dx-z-",
    "box-shadow": "--dx-shadow-",
}

# value -> token, bucketed per family so each property gets the right one.
by_family = {}
for name, val in tokens.items():
    if "var(" in val:            # composite tokens (shadows, ring) handled separately
        continue
    for prefix in set(FAMILY.values()):
        if name.startswith(prefix):
            by_family.setdefault(prefix, {}).setdefault(val, name)

def token_for(prop, frag):
    prefix = FAMILY.get(prop)
    if not prefix:
        return None
    return by_family.get(prefix, {}).get(frag)

errors, notes = [], []

# Scope: only components the root `component.json` lists as `members` -- the
# set `dx components add` can actually install. `top_layer` is deliberately
# outside it (its own component.json calls itself "Oracle fixture (not an
# installable component)"): it is a probe surface for
# playwright/oracle/tier2-html/, with hand-tuned geometry calibrated against
# specific viewport math, and it never reaches a consumer. Holding a fixture
# to a consumer-facing style rule would be enforcing tidiness on a place
# where the literal values ARE the point.
import json
members = set()
for m in json.load(open("component.json"))["members"]:
    members.add(m.rstrip("/").split("/")[-1])

for path in sorted(glob.glob("preview/src/components/*/style.css")):
    if path.split("/")[-2] not in members:
        continue
    text = open(path).read()
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    for lineno, line in enumerate(text.splitlines(), 1):
        m = SCANNED.match(line)
        if not m:
            continue
        prop, value = m.group(1), m.group(2)
        for frag in re.split(r"[\s,]+", norm(value)):
            if not frag or frag.startswith("var("):
                continue
            hit = token_for(prop, frag)
            if hit:
                errors.append(
                    f"{path}:{lineno}: `{prop}: {value.strip()}` hard-codes "
                    f"{frag}, which is {hit} "
                    f"(docs/backlog.md row 31b)"
                )
            else:
                px = re.fullmatch(r"(\d+(?:\.\d+)?)px", frag)
                if px:
                    rem = f"{float(px.group(1)) / 16:g}rem"
                    rem_hit = token_for(prop, rem)
                    if rem_hit:
                        notes.append(
                            f"{path}:{lineno}: `{prop}: {value.strip()}` uses {frag}, "
                            f"equal to {rem_hit} ({rem}) at a 16px root "
                            f"-- a unit change, not a mechanical swap"
                        )

for n in notes:
    print(f"note: {n}")
if notes:
    print(f"note: {len(notes)} cross-unit candidate(s); informational only.\n")

for e in errors:
    print(e)

if errors:
    print()
    print(f"check-css-literals: FAILED -- {len(errors)} hard-coded value(s) "
          f"duplicate a design token. See docs/backlog.md row 31b.")
    sys.exit(1)

print("check-css-literals: OK")
PYEOF
