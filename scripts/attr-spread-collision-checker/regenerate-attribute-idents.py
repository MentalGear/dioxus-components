#!/usr/bin/env python3
# Mechanically (re)generates the two ident data files
# scripts/check-attr-spread-collision.sh's analyzer depends on:
#
#   dev-docs/issues/duplicate-attribute-style-shorthand-idents.txt
#   dev-docs/issues/duplicate-attribute-global-attribute-idents.txt
#
# from this workspace's own resolved `dioxus-html` crate source -- never hand-edit either
# .txt file directly; re-run this script instead (e.g. after a `dioxus-html` version bump)
# and commit whatever it writes.
#
# Both files come from the exact same place: dioxus-html's `attribute_groups.rs` defines
# one `mod_methods! { @base global_attributes; ... }` macro invocation whose body is a
# flat table of entries, each shaped one of:
#   ident;                          ident: "literal";
#   ident in "ns";                  ident: "literal" in "ns";
# Every entry in this table that carries `in "style"` is a CSS-shorthand style property
# (`padding`, `flex_direction`, ...) that dioxus-ssr folds into one `style="...;"` string
# at render time -- a different, non-erroring code path from the WHATWG
# duplicate-HTML-attribute class this gate looks for, so the checker excludes them.
# Every OTHER entry in the same table is a real `GlobalAttributes` trait member (`id`,
# `class`, `role`, every `aria_*`, ...) -- the only names a
# `#[props(extends = GlobalAttributes)]` struct's derived builder generates an ad-hoc
# setter for. The checker's secondary, `component-attributes-forward` check (see
# scripts/attr-spread-collision-checker/src/main.rs) uses this second list to recognize
# the `5fc1439`-shaped collision shape without needing to hardcode or guess the set of
# names that mechanism can carry.
#
# Usage: python3 scripts/attr-spread-collision-checker/regenerate-attribute-idents.py
# Exit 0 and prints what it wrote; exit 1 (via SystemExit) if dioxus-html's own macro
# shape has changed enough that this script's extraction can no longer trust its output
# (rather than silently writing a suspiciously small list).

import json
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
STYLE_OUT = REPO_ROOT / "dev-docs/issues/duplicate-attribute-style-shorthand-idents.txt"
GLOBAL_OUT = REPO_ROOT / "dev-docs/issues/duplicate-attribute-global-attribute-idents.txt"

# The macro invocation's own meta-parameters (mod name / fn names it generates), not real
# HTML attributes -- they sit right after `@base`, before the real attribute table
# begins, and happen to match the same trailing-`;` shape as a real bare global-attribute
# entry (dioxus-html's attribute_groups.rs, inside the `global_attributes!` block).
MACRO_META_NAMES = {
    "global_attributes",
    "map_global_attributes",
    "map_html_global_attributes_to_rsx",
}

ENTRY_RE = re.compile(
    r'^\s*(?:#\[[^\]]*\]\s*)*([A-Za-z_][A-Za-z0-9_]*)\s*'
    r'(?::\s*"[^"]*")?\s*(?:in\s*"([a-zA-Z-]+)")?\s*;\s*$'
)


def find_attribute_groups_rs() -> Path:
    """Resolve this workspace's own pinned `dioxus-html` source path via `cargo
    metadata`, rather than hardcoding a `~/.cargo/registry/...` hash that is specific to
    one machine's cache layout and would silently go stale across a version bump."""
    out = subprocess.check_output(
        ["cargo", "metadata", "--format-version=1", "--locked"],
        cwd=REPO_ROOT,
    )
    meta = json.loads(out)
    for pkg in meta["packages"]:
        if pkg["name"] == "dioxus-html":
            manifest_dir = Path(pkg["manifest_path"]).parent
            path = manifest_dir / "src" / "attribute_groups.rs"
            if path.is_file():
                return path
            raise SystemExit(
                f"dioxus-html resolved to {manifest_dir} but it has no src/attribute_groups.rs "
                "-- its internal layout may have changed; update this script by hand."
            )
    raise SystemExit("dioxus-html not found in `cargo metadata` output -- is Cargo.lock present/up to date?")


def extract(path: Path):
    lines = path.read_text().splitlines()

    # Find `mod_methods! {` followed (skipping blank lines) by `@base` then
    # `global_attributes;` -- the fixed opening shape of the specific invocation we want
    # (this file defines others afterward, e.g. `svg_attributes!`, with their own,
    # unrelated idents that must NOT leak into either output list).
    start = None
    for i, raw in enumerate(lines):
        if raw.strip() != "mod_methods! {":
            continue
        j, seen_base, ok = i + 1, False, False
        while j < len(lines) and j < i + 6:
            s = lines[j].strip()
            if not s:
                j += 1
                continue
            if not seen_base:
                if s == "@base":
                    seen_base = True
                    j += 1
                    continue
                break
            ok = s == "global_attributes;"
            break
        if ok:
            start = i
            break
    if start is None:
        raise SystemExit(
            "could not find `mod_methods! { @base global_attributes; ...` in "
            f"{path} -- dioxus-html's macro shape may have changed; update this script by hand."
        )

    depth = 0
    end = None
    for i in range(start, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if depth == 0 and i > start:
            end = i
            break
    if end is None:
        raise SystemExit("brace-matching for the global_attributes! block never returned to depth 0.")

    style_idents, global_idents = set(), set()
    for raw in lines[start : end + 1]:
        line = raw.strip()
        if not line or line.startswith("//") or line.startswith("#[") or line == "@base":
            continue
        m = ENTRY_RE.match(raw)
        if not m:
            continue
        ident, ns = m.group(1), m.group(2)
        if ident in MACRO_META_NAMES:
            continue
        if ns == "style":
            style_idents.add(ident)
        elif ns is None:
            global_idents.add(ident)
        # else: some other namespace -- none observed in dioxus-html-0.7.9's
        # global_attributes! block (verified: only "style" occurs), so intentionally not
        # bucketed as either; if this ever fires, it needs a human to look, not a guess.
    return style_idents, global_idents


def main():
    src = find_attribute_groups_rs()
    style_idents, global_idents = extract(src)

    if len(style_idents) < 100 or len(global_idents) < 20:
        raise SystemExit(
            f"extraction looks too small (style={len(style_idents)}, global={len(global_idents)}) "
            f"-- dioxus-html's macro shape in {src} may have changed; inspect it by hand "
            "before trusting/committing this output."
        )

    STYLE_OUT.write_text("".join(f"{i}\n" for i in sorted(style_idents)))
    GLOBAL_OUT.write_text("".join(f"{i}\n" for i in sorted(global_idents)))

    print(f"source: {src}")
    print(f"wrote {len(style_idents)} style-shorthand idents -> {STYLE_OUT.relative_to(REPO_ROOT)}")
    print(f"wrote {len(global_idents)} global-attribute idents -> {GLOBAL_OUT.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    sys.exit(main())
