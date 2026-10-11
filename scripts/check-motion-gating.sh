#!/usr/bin/env bash
#
# check-motion-gating.sh
#
# Motion nobody can see must not cost anything. cargo, clippy and `cargo test` cannot tell a ticking
# component from a gated one, and with CI frozen (dev-docs/backlog.md row 33) nothing else would.
#
# THE CLASS (owner, 2026-10-10: "animations running in the background are not about cards, but the actual
# components and their animations"): a component that keeps a loop or a timer alive while it is scrolled off
# screen, inside a skipped `content-visibility` card, or in a hidden tab spends main-thread work on
# nothing. The scroll re-profile measured the infinite animations alone at -15% scroll busy time, and the
# home page's 1 Hz tickers wake the page every second for cards nobody is looking at. Fixed by construction,
# not per instance: `primitives/src/activity.rs` answers "is this visible" once for the whole page (one shared
# IntersectionObserver + `visibilitychange` + `contentvisibilityautostatechange`); a CSS loop is paused by ONE
# theme rule on `data-dx-motion="paused"`, a timer is `use_interval_while` / `use_timeout_while`. This script
# keeps the ungated shapes from coming back. Two checks:
#
#  A. TIMERS. In every `*.rs` under primitives/src and preview/src (interval.rs itself excepted):
#       - a bare `use_interval(` call, and
#       - a `loop {` / `while ... {` whose next few lines `sleep(...)` (a hand-rolled tick loop),
#     must either (the hand-rolled loop only) sit within 40 lines after a use of the gate (`use_motion`, `use_motion_when`,
#     `use_motion_active_for`, `use_entered_view_when`, `use_document_visible`) or (both) carry
#     `// motion-ok: <reason>` on the same line or the line above (a timer that carries meaning, or one
#     that is genuinely page-lifetime). `use_interval_while` is the gated form and always passes.
#
#  B. CSS LOOPS. Every component `style.css` (preview/src/components/<name>/) with an `infinite` animation
#     must have a component that applies the gate: `preview/src/components/<name>/component.rs` or the
#     primitive it wraps (`primitives/src/<name>.rs` or `primitives/src/<name>/**`) mentions `use_motion`,
#     or the stylesheet is in ALLOW below with a reason. Non-component sheets under preview/assets with an
#     infinite animation are UTILITY classes (`.dx-shimmer` in dx-effects.css) with no root of their own; they
#     are gated at their HOST instead (check C). A new utility loop must be added to UTILITY below. An ALLOW entry whose
#     sheet no longer has an infinite animation, or whose component has since been wired, fails the run, so
#     the list cannot rot into blanket permission.
#
#  C. UTILITY CLASSES. A utility class that loops (`dx-shimmer`) is applied by a host: a component that
#     spreads `use_motion*().attributes()` onto the element (`MarkerContent`, `AttachmentTitle`), or a raw
#     element. Every `*.rs` under preview/src and primitives/src that names the class in code (comments
#     skipped) must either mention the gate in the same file, or the line must be a call of a listed gated
#     host component. The host's own file is checked to mention the gate, so the chain cannot be cut quietly.
#
# NOT covered, said plainly: a self-re-arming `setTimeout` chain or a `requestAnimationFrame` loop written in
# JS (a grep cannot tell a bounded one from a perpetual one; `scroll-main-thread.spec.ts` and
# `playwright/oracle/tier2-html/motion-gating.spec.ts` count timer/frame callbacks at runtime), and the
# prefers-reduced-motion preference (the stylesheets answer that, not the gate).
#
# Usage: scripts/check-motion-gating.sh
# Exit 0: clean. Exit 1: a violation, with file:line detail on stderr.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

python3 - <<'PYEOF'
import re
import sys
from pathlib import Path

GATE = re.compile(r"use_motion\w*|use_entered_view_when|use_document_visible")

# (stylesheet path, reason). The reason says WHY this loop is not paused by the gate.
ALLOW = [
    ("preview/src/components/input_otp/style.css",
     "the caret blink exists only on the ONE focused slot (`[data-active=\"true\"]::after`), i.e. while the "
     "user is typing into that field: it is visible by definition, and a hidden tab renders no frames"),
]

# utility class -> (stylesheet, {host component: file that must mention the gate})
UTILITY = {
    "dx-shimmer": ("preview/assets/dx-effects.css", {
        "MarkerContent": "preview/src/components/marker/component.rs",
        "AttachmentTitle": "preview/src/components/attachment/component.rs",
    }),
}

errors = []

def strip_line_comment(line: str) -> str:
    # good enough for Rust: a `//` outside a string. Strings containing `//` (URLs) only cause a missed check.
    i = line.find("//")
    return line if i < 0 else line[:i]

# ---- A. timers ------------------------------------------------------------------------------------------
rs_files = sorted(
    p for root in ("primitives/src", "preview/src") for p in Path(root).rglob("*.rs")
    if p.as_posix() != "primitives/src/interval.rs"
)
for p in rs_files:
    lines = p.read_text().splitlines()
    for i, raw in enumerate(lines):
        code = strip_line_comment(raw)
        hit = None
        if re.search(r"\buse_interval\s*\(", code):
            hit = "bare `use_interval(`"
        elif re.search(r"\b(loop|while\b[^{]*)\s*\{\s*$", code):
            window = "\n".join(strip_line_comment(l) for l in lines[i + 1 : i + 5])
            if re.search(r"\bsleep\s*\(", window):
                hit = "hand-rolled `loop` + `sleep(` tick"
        if not hit:
            continue
        marked = "motion-ok:" in raw or (i > 0 and "motion-ok:" in lines[i - 1])
        before = "\n".join(strip_line_comment(l) for l in lines[max(0, i - 40) : i + 1])
        gated = bool(GATE.search(before))
        if hit.startswith("bare") and not marked:
            gated = False  # a bare `use_interval` is never the gated form, whatever else is nearby
        if not (marked or gated):
            errors.append(
                f"{p}:{i + 1}: {hit} with no visibility gate -- use `use_interval_while(use_motion().active(), ..)` "
                "(primitives/src/activity.rs), or mark a timer that carries meaning / lives as long as the page "
                "with `// motion-ok: <reason>`"
            )

# ---- B. CSS loops ---------------------------------------------------------------------------------------
INFINITE = re.compile(r"animation(?:-iteration-count)?\s*:[^;{}]*\binfinite\b", re.S)

def has_infinite(path: Path) -> bool:
    src = re.sub(r"/\*.*?\*/", "", path.read_text(), flags=re.S)
    return bool(INFINITE.search(src))

def component_wired(name: str) -> bool:
    candidates = [Path(f"preview/src/components/{name}/component.rs"), Path(f"primitives/src/{name}.rs")]
    prim_dir = Path(f"primitives/src/{name}")
    if prim_dir.is_dir():
        candidates += sorted(prim_dir.rglob("*.rs"))
    return any(c.is_file() and GATE.search(c.read_text()) for c in candidates)

allow = dict(ALLOW)
seen_allow = set()
sheets = sorted(Path("preview/src/components").glob("*/style.css")) + sorted(Path("preview/assets").glob("*.css"))
for sheet in sheets:
    if not has_infinite(sheet):
        continue
    key = sheet.as_posix()
    is_component = sheet.parent.parent.as_posix() == "preview/src/components"
    if key in allow:
        seen_allow.add(key)
        if is_component and component_wired(sheet.parent.name):
            errors.append(f"{key}: allowlisted, but its component now applies the motion gate -- remove the ALLOW entry")
        continue
    if not is_component:
        if not any(sheet_ == key for sheet_, _ in UTILITY.values()):
            errors.append(f"{key}: infinite animation in a non-component sheet -- add its class to UTILITY (check C) so every host is gated, or move it into a gated component")
    elif not component_wired(sheet.parent.name):
        errors.append(
            f"{key}: infinite animation, but neither preview/src/components/{sheet.parent.name}/component.rs nor "
            f"primitives/src/{sheet.parent.name}* uses `use_motion` (primitives/src/activity.rs) -- wire it, or add the "
            "sheet to ALLOW in scripts/check-motion-gating.sh with a reason"
        )
for key in allow:
    if key not in seen_allow:
        errors.append(f"{key}: ALLOW entry, but the sheet has no infinite animation any more -- remove it")

# ---- C. utility classes ---------------------------------------------------------------------------------
for cls, (sheet_path, hosts) in UTILITY.items():
    if not has_infinite(Path(sheet_path)):
        errors.append(f"{sheet_path}: UTILITY lists `{cls}`, but the sheet has no infinite animation any more -- remove it")
    for host, hfile in hosts.items():
        hp = Path(hfile)
        if not (hp.is_file() and GATE.search(hp.read_text())):
            errors.append(f"{hfile}: `{host}` hosts `{cls}` but does not apply the motion gate (use_motion_when + motion.attributes())")
    for p in rs_files:
        text = p.read_text()
        file_gated = bool(GATE.search(text))
        for i, raw in enumerate(text.splitlines()):
            code = strip_line_comment(raw)
            if cls not in code:
                continue
            if file_gated or any(re.search(rf"\b{h}\b", code) for h in hosts):
                continue
            errors.append(
                f"{p}:{i + 1}: `{cls}` on an element nothing pauses -- put it on a gated host ({', '.join(hosts)}), "
                "or spread `use_motion().attributes()` on that element (primitives/src/activity.rs)"
            )

if errors:
    for e in errors:
        print(e, file=sys.stderr)
    print("check-motion-gating: FAIL", file=sys.stderr)
    sys.exit(1)
print("check-motion-gating: OK -- every timer and infinite CSS animation is visibility-gated or allowlisted with a reason.")
PYEOF
