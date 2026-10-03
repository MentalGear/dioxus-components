#!/usr/bin/env bash
#
# check-css-vars-defined.sh
#
# Enforces dev-docs/backlog.md row 111 (phase A): every CSS custom property a
# component reads with `var(--name)` must be defined somewhere, or carry a
# fallback in the `var()` itself, or be on the explicit allowlist below.
#
#   color: var(--primary-color-9);            -> FAIL: nothing declares it
#   color: var(--dx-muted-foreground);        -> FAIL if the theme lacks it
#   color: var(--dx-muted-foreground, #666);  -> ok (fallback in the var())
#
# Why: an undefined custom property is not an error in CSS. `color:
# var(--nope)` is "invalid at computed-value time" and silently computes to
# `unset` -- the text renders in the inherited colour, a `background` vanishes,
# a border disappears -- with no console message and no failing test. It bites
# hardest on the role-token migration: `dx components add` copies a component
# stylesheet into a user's tree next to THEIR copy of dx-components-theme.css,
# and an older theme copy lacks any `--dx-<role>` token added since. The
# component then renders wrongly in exactly the consumer's tree and nowhere in
# ours. The token-design lane also found `--primary-color-8/9/11/12` read
# (virtual_list demo, navigation_menu dark-mode gradient) but never declared.
#
# WHAT COUNTS AS A USE
#   * `var(--name` in preview/src/**/*.css, preview/assets/*.css,
#     primitives/src/**/*.css (comments stripped);
#   * `var(--name` in preview/src/**/*.rs and primitives/src/**/*.rs --
#     inline `style:` strings, `format!` colours, embedded CSS. Comment lines
#     and everything after a file's `#[cfg(test)]` marker are ignored, as are
#     files that a parent declares `#[cfg(test)] mod name;` (test modules
#     assert on rendered strings and shadcn fixtures, not on shipped styles).
#
# WHAT COUNTS AS A DEFINITION (any file in the same scope)
#   * a `--name:` declaration (CSS, or inside a Rust string/inline style);
#   * `@property --name`;
#   * `setProperty('--name', ...)` in embedded JS;
#   * an interpolated Rust declaration `--prefix-{..}:` defines the PREFIX
#     `--prefix-`; a `var(--prefix-{..})` use is satisfied by any definition
#     or defined prefix that starts with `--prefix-`.
#
# WHAT COUNTS AS HANDLED WITHOUT A DEFINITION
#   * a fallback: `var(--name, <anything>)` -- including a nested `var()`.
#     The theme's `--dx-x: var(--x, <ramp>)` bridge relies on this for the
#     optional shadcn host variables, so those are legitimately undeclared;
#   * an ALLOWLIST entry below, each with a reason. Keep this list short: an
#     entry is a promise that the variable is supplied from outside the
#     scanned source, and a wrong entry re-opens the exact hole this gate
#     closes.
#
# Usage: scripts/check-css-vars-defined.sh
# Exit status: 0 when every use is defined, has a fallback, or is allowlisted.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'PYEOF'
import glob
import os
import re
import sys

# --------------------------------------------------------------------------
# Allowlist: name (or `prefix-*`) -> why it may be used without a definition
# or fallback. Add an entry only with a reason a reviewer can check.
# --------------------------------------------------------------------------
ALLOWLIST = {
    # (currently empty -- every legitimately-external variable in this tree is
    # either declared in a scanned file or is always read with a fallback.)
    #
    # Examples of the shape, for the day one is needed:
    # "--some-host-var": "set by the embedding app's stylesheet, not by us",
    # "--runtime-*":     "set by JS at runtime via style.setProperty",
}

CSS_GLOBS = [
    "preview/src/**/*.css",
    "preview/assets/*.css",
    "primitives/src/**/*.css",
]
RS_GLOBS = [
    "preview/src/**/*.rs",
    "primitives/src/**/*.rs",
]

def files(globs):
    out = set()
    for g in globs:
        out.update(glob.glob(g, recursive=True))
    return sorted(out)

def strip_css_comments(text):
    return re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)

def strip_rs(text):
    """Drop `#[cfg(test)]` tails, full-line `//` comments and trailing `//`
    comments (only when the code before them has balanced quotes, so a
    `https://` inside a string literal survives). Line count is preserved."""
    m = re.search(r"^[ \t]*#\[cfg\(test\)\]", text, flags=re.M)
    if m:
        text = text[: m.start()]
    out = []
    for line in text.split("\n"):
        s = line.lstrip()
        if s.startswith("//"):
            out.append("")
            continue
        i = line.find(" //")
        if i != -1 and line[:i].count('"') % 2 == 0:
            line = line[:i]
        out.append(line)
    return "\n".join(out)

sources = []  # (path, text)
for p in files(CSS_GLOBS):
    sources.append((p, strip_css_comments(open(p, encoding="utf-8").read())))

# A `#[cfg(test)] mod name;` declaration gates the whole file `name.rs` (or
# `name/mod.rs`) behind tests -- e.g. preview/src/chart_parity.rs, which
# rewrites shadcn's `var(--chart-N)` fixtures. Such files never ship.
test_only = set()
rs_files = files(RS_GLOBS)
for p in rs_files:
    d = os.path.dirname(p)
    for m in re.finditer(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;",
                         open(p, encoding="utf-8").read()):
        test_only.add(os.path.join(d, m.group(1) + ".rs"))
        test_only.add(os.path.join(d, m.group(1), "mod.rs"))
for p in rs_files:
    if p in test_only:
        continue
    sources.append((p, strip_rs(open(p, encoding="utf-8").read())))

NAME = r"--[A-Za-z_][A-Za-z0-9_-]*"

defined = set()      # exact names
prefixes = set()     # `--prefix-` from Rust-interpolated declarations
DECL = re.compile(r"(?<![\w-])(" + NAME + r")\s*:")
ATPROP = re.compile(r"@property\s+(" + NAME + r")")
SETPROP = re.compile(r"setProperty\(\s*['\"](" + NAME + r")['\"]")
DYN_DECL = re.compile(r"(?<![\w-])(--[A-Za-z0-9_-]*-)\{[^}]*\}\s*:")

for path, text in sources:
    for rx in (DECL, ATPROP, SETPROP):
        for m in rx.finditer(text):
            defined.add(m.group(1))
    for m in DYN_DECL.finditer(text):
        prefixes.add(m.group(1))

def allowed(name):
    for k in ALLOWLIST:
        if k.endswith("*"):
            if name.startswith(k[:-1]):
                return True
        elif k == name:
            return True
    return False

def satisfied(name):
    if name in defined or allowed(name):
        return True
    if name.endswith("-"):  # interpolated use: `var(--prefix-{..})`
        return (any(d.startswith(name) for d in defined)
                or any(p.startswith(name) for p in prefixes))
    # a use like `--color-desktop` is also satisfied by a defined prefix
    return any(name.startswith(p) for p in prefixes)

USE = re.compile(r"var\(\s*(" + NAME + r")\s*([,)]?)")

errors = []
total = 0
for path, text in sources:
    for m in USE.finditer(text):
        total += 1
        name, term = m.group(1), m.group(2)
        # An interpolated name (`var(--color-{slot})`) stops the regex at the
        # `{`, leaving a name that ends in `-`; `satisfied` treats that as a
        # prefix.
        if term == ",":
            continue  # fallback present
        if satisfied(name):
            continue
        lineno = text.count("\n", 0, m.start()) + 1
        errors.append((path, lineno, name))

for path, lineno, name in errors:
    print(f"{path}:{lineno}: `var({name})` is never declared and has no fallback")

if errors:
    names = sorted({n for _, _, n in errors})
    print()
    print(f"check-css-vars-defined: FAILED -- {len(errors)} use(s) of "
          f"{len(names)} undefined custom propert{'y' if len(names) == 1 else 'ies'}: "
          + ", ".join(names))
    print("Fix: declare it (theme/stylesheet/inline style), give the var() a "
          "fallback, or point it at a defined token. If it is supplied from "
          "outside the scanned source, add it to ALLOWLIST in this script "
          "WITH a reason. See dev-docs/backlog.md row 111.")
    sys.exit(1)

print(f"check-css-vars-defined: OK -- {total} var() uses checked, "
      f"{len(defined)} custom properties defined.")
PYEOF
