#!/usr/bin/env bash
#
# check-css-logical-properties.sh
#
# Enforces dev-docs/backlog.md row 13 (Phase 6 RTL, styling half): a themed
# component stylesheet must not hard-code an inline-axis PHYSICAL property
# (one that means "left" or "right" no matter what direction the page
# reads) where a logical property (`margin-inline-start`, `inset-inline-end`,
# `border-start-start-radius`, `text-align: start`, ...) would express the
# same rule and mirror correctly under `dir="rtl"`.
#
# Scanned properties (dev-docs "RTL-CSS" lane brief, Step 2):
#   margin-left/right, padding-left/right, left/right (incl. asymmetric
#   4-value `inset`), border-left/right(-color/-width/-style),
#   border-{top,bottom}-{left,right}-radius (and asymmetric border-radius
#   shorthands), text-align: left/right, float: left/right, a non-zero
#   translateX()/translate3d()/translate() (transforms are never
#   direction-aware, so a real sign always encodes a physical side),
#   transform-origin: left/right, background-position: left/right.
#
# A hit is not automatically an error. Two escape hatches, matching the
# lane brief's own convention:
#
#  1. STRUCTURAL: any declaration inside a rule whose selector mentions
#     `data-side=` is exempt, with no per-line comment needed. `data-side`
#     in this codebase is always Radix-Popper-style resolved physical
#     placement (`popover`/`tooltip`/`hover_card`'s anchored-overlay side,
#     `sidebar`'s explicit `side` prop) -- a screen-geometry fact, not a
#     reading-direction one, matching upstream (Radix's own `side` prop is
#     physical; shadcn's `Sidebar` `side` prop carries no RTL handling
#     either -- see reference.md). This is a deliberate, named trust
#     boundary: it assumes `data-side` is never repurposed for something
#     that SHOULD mirror. See reference.md's shadcn-parity table for the
#     evidence this rests on, and this script's own header for the
#     limitation.
#
#  2. PER-LINE: a `/* rtl-physical: <reason> */` comment (or a shorter
#     `/* rtl-physical */`, when a fuller reason already sits in a nearby
#     comment) either (a) trailing on the same line, or (b) anywhere inside
#     the contiguous run of comment-only lines immediately above it (no
#     blank line, no other declaration, in between).
#
# WHAT THIS DOES NOT CATCH (say plainly, per this repo's own convention):
#   - A future misuse of `data-side` for something that genuinely should
#     mirror would be silently exempted (the structural trust boundary
#     above). Nothing currently in this codebase does that (verified by
#     reading every `data-side` rule this script exempts -- see
#     reference.md); a reviewer adding a new one should check by hand.
#   - `[dir="rtl"]` OVERRIDE rules this lane added (the correct fix for a
#     transform/translate that must differ under RTL) still contain a
#     literal physical value by construction and are allowlisted the same
#     way as their LTR sibling -- this script does not verify the override
#     is actually a correct sign-flip, only that it is intentional.
#   - Symmetric shorthands (`padding: 0 1rem`, `border-radius: 8px`, a
#     4-value form whose left/right halves already match) are not hits at
#     all: they are already direction-safe (swapping left and right is a
#     no-op), so flagging them would be noise, not signal.
#   - Batch-2-owned stylesheets (drawer, tag_group, sheet, resizable,
#     navigation_menu) and the date-picker/data_table/top_layer lanes'
#     stylesheets were out of scope for the rtl-css lane itself, but were
#     finished at batch-3 integration (drawer/sheet's per-side slide
#     keyframes and resizable's transform-paired centering allowlisted;
#     navigation_menu's anchor reset and tag_group's tag/remove-button gap
#     converted to logical properties; date_picker/data_table/top_layer had
#     no hits) -- no folder is excluded from the scan any more.
#
# Usage: scripts/check-css-logical-properties.sh
# Exit status: 0 if clean, 1 if any unallowlisted physical property remains.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'PYEOF'
import re
import sys
import glob
import json

THEME = "preview/assets/dx-components-theme.css"

# Component folders excluded from the scan. Empty: the batch-2-owned
# stylesheets and the date-picker/data_table/top_layer lanes' stylesheets
# (originally excluded here, out of the rtl-css lane's own editable scope)
# were audited and finished at batch-3 integration -- see this script's
# header. Kept as a set, not removed outright, so a future lane can name a
# folder here again if it ever needs a deliberate, reviewed exemption from
# the whole scan (rather than allowlisting each line) -- see this script's
# header for why that must stay a rare, named exception, not a default.
EXCLUDE = set()

members = set()
for m in json.load(open("component.json"))["members"]:
    members.add(m.rstrip("/").split("/")[-1])

FILES = []
for p in sorted(glob.glob("preview/src/components/*/style.css")):
    name = p.split("/")[-2]
    if name not in members or name in EXCLUDE:
        continue
    FILES.append(p)
FILES.append(THEME)

# ---- property-level patterns (matched against one declaration's `prop` and
# `value`, already extracted from a `prop: value;` line) --------------------

def is_zero_len(tok):
    return re.fullmatch(r"0(\.0+)?(%|[a-z]+)?", tok.strip()) is not None

def split_top_level(value):
    """Split on whitespace, but not inside ()/[]/quotes -- so a `calc(a + b)`
    or `var(--x, 1 2)` argument is never mistaken for multiple tokens."""
    toks, cur, depth = [], "", 0
    for ch in value:
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        if ch.isspace() and depth == 0:
            if cur:
                toks.append(cur)
                cur = ""
        else:
            cur += ch
    if cur:
        toks.append(cur)
    return toks

def shorthand_lr_asymmetric(value):
    """True if a 2/3/4-value TRBL-family shorthand's left != right half --
    the only case that is actually a mirrored-layout bug. 1/2/3-value forms
    always mirror the horizontal component onto both sides by the CSS
    shorthand's own expansion rule, so they can never be asymmetric."""
    toks = split_top_level(value)
    if len(toks) != 4:
        return False
    return toks[1].lower() != toks[3].lower()

def border_radius_asymmetric(value):
    """True if a border-radius shorthand's two left corners don't match its
    two right corners (the general "swap left<->right, compare" test --
    correct for 1-4 values, ignores the elliptical `/ ry` part since only
    the horizontal radius can ever encode a side)."""
    horiz = value.split("/")[0]
    toks = split_top_level(horiz)
    n = len(toks)
    if n == 1:
        return False
    if n == 2:
        tl, tr, br, bl = toks[0], toks[1], toks[0], toks[1]
    elif n == 3:
        tl, tr, br, bl = toks[0], toks[1], toks[2], toks[1]
    elif n == 4:
        tl, tr, br, bl = toks
    else:
        return False
    return not (tl.lower() == tr.lower() and bl.lower() == br.lower())

def nonzero_translate(value):
    """True if `value` (a `transform` property's RHS) contains a
    translateX()/translate3d()/translate() whose horizontal argument isn't
    zero -- transforms are never direction-aware, so any real offset here
    encodes a physical side."""
    for m in re.finditer(r"(translateX|translate3d|translate)\(([^)]*)\)", value, re.I):
        fn = m.group(1).lower()
        args = [a.strip() for a in m.group(2).split(",")]
        x = args[0] if args else "0"
        if fn == "translate" and len(args) == 1:
            # `translate: <x>` single-axis form -- x is the whole thing.
            pass
        if not is_zero_len(x):
            return True
    return False

def classify(prop, value):
    p = prop.lower()
    if p in ("margin-left", "margin-right", "padding-left", "padding-right",
             "left", "right"):
        return "physical inline-axis property"
    if p in ("border-left", "border-right", "border-left-color",
              "border-right-color", "border-left-width", "border-right-width",
              "border-left-style", "border-right-style"):
        return "physical border side"
    if p in ("border-top-left-radius", "border-top-right-radius",
              "border-bottom-left-radius", "border-bottom-right-radius"):
        return "physical border-radius corner"
    if p == "text-align" and value.strip().lower() in ("left", "right"):
        return "physical text-align"
    if p == "float" and value.strip().lower() in ("left", "right"):
        return "physical float"
    if p == "transform-origin" and re.search(r"\b(left|right)\b", value, re.I):
        return "physical transform-origin"
    if p == "background-position" and re.search(r"\b(left|right)\b", value, re.I):
        return "physical background-position"
    if p == "transform" and nonzero_translate(value):
        return "non-zero translateX/translate3d/translate (never direction-aware)"
    if p == "margin" and shorthand_lr_asymmetric(value):
        return "asymmetric margin shorthand"
    if p == "padding" and shorthand_lr_asymmetric(value):
        return "asymmetric padding shorthand"
    if p == "inset" and shorthand_lr_asymmetric(value):
        return "asymmetric inset shorthand"
    if p == "border-radius" and border_radius_asymmetric(value):
        return "asymmetric border-radius shorthand"
    return None

# ---- allowlist detection ---------------------------------------------------

MARKER = "rtl-physical"

def comment_block_above_has_marker(lines, idx):
    """Walk upward from lines[idx-1] (0-based) through a contiguous run of
    comment-only lines (no blank line, no code line) and look for MARKER."""
    j = idx - 1
    collected = []
    # A single-line trailing/standalone comment, or the closing of a
    # multi-line one, must be the line directly above.
    if j < 0:
        return False
    if "*/" not in lines[j]:
        return False
    while j >= 0:
        collected.append(lines[j])
        if "/*" in lines[j]:
            break
        j -= 1
    else:
        return False
    text = "\n".join(reversed(collected))
    return MARKER in text

def same_line_has_marker(line):
    return MARKER in line

# ---- selector tracking (for the structural data-side= exemption) ----------

def scan_file(path):
    text = open(path, encoding="utf-8").read()
    lines = text.splitlines()
    errors = []

    depth = 0
    selector_stack = [""]  # selector text active at each brace depth
    pending_selector_lines = []
    in_comment = False

    for i, raw in enumerate(lines):
        line = raw

        # Track (and strip, for selector/brace purposes only) block comments
        # that span multiple lines -- declarations never live inside one.
        if in_comment:
            if "*/" in line:
                in_comment = False
                line = line.split("*/", 1)[1]
            else:
                continue

        # Remove same-line comments for brace/selector bookkeeping (but the
        # ORIGINAL raw line is what the marker-detection functions read).
        code = line
        while "/*" in code:
            start = code.index("/*")
            if "*/" in code[start:]:
                end = code.index("*/", start) + 2
                code = code[:start] + code[end:]
            else:
                code = code[:start]
                in_comment = True
                break

        stripped = code.strip()
        if not stripped:
            continue

        # Declaration line (only meaningful at depth >= 1, inside a rule),
        # matched against `code` (comment-stripped).
        dm = re.match(r"^\s*([a-zA-Z-]+)\s*:\s*([^;{}]+);", code)
        if dm and depth >= 1 and "{" not in code:
            prop, value = dm.group(1), dm.group(2)
            reason = classify(prop, value)
            if reason:
                selector = selector_stack[-1]
                if "data-side=" in selector:
                    pass  # structurally exempt
                elif same_line_has_marker(raw) or comment_block_above_has_marker(lines, i):
                    pass  # per-line allowlisted
                else:
                    errors.append(
                        f"{path}:{i+1}: `{prop}: {value.strip()}` -- {reason} "
                        f"(selector: {selector[:80]!r})"
                    )

        # Brace bookkeeping, after declaration extraction (a line can both
        # declare a property via a preceding `;` and open/close a brace, but
        # this codebase's style never does -- one concern per line).
        opens = code.count("{")
        closes = code.count("}")
        if opens:
            sel_text = " ".join(pending_selector_lines + [code.split("{")[0]]).strip()
            for _ in range(opens):
                selector_stack.append(sel_text)
                depth += 1
            pending_selector_lines = []
        elif not dm and stripped and "}" not in stripped:
            # Accumulating a multi-line selector (comma-separated groups).
            pending_selector_lines.append(stripped)
        for _ in range(closes):
            if depth > 0:
                selector_stack.pop()
                depth -= 1
            pending_selector_lines = []

    return errors

all_errors = []
for f in FILES:
    all_errors.extend(scan_file(f))

for e in all_errors:
    print(e)

if all_errors:
    print()
    print(f"check-css-logical-properties: FAILED -- {len(all_errors)} physical "
          f"inline-axis value(s) outside the allowlist. See dev-docs/backlog.md "
          f"row 13 and this script's own header.")
    sys.exit(1)

print("check-css-logical-properties: OK")
PYEOF
