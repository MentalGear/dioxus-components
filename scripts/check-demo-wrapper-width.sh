#!/usr/bin/env bash
#
# check-demo-wrapper-width.sh
#
# Enforces dev-docs/backlog.md row 94: `.dx-component-preview-frame`
# (preview/assets/main.css) is `display: flex; flex-direction: column`
# with `align-items: center`, so its cross axis is horizontal and a direct
# child gets sized by that default -- shrink-to-fit, capped by its own
# `max-width` -- unless the child sets its OWN `width` (`100%` is the
# established fix; `fit-content` is also fine, since that is already what
# the item would shrink-wrap to). A child that sets `max-width` but never
# `width` renders narrower than its author intended: measured on row 94's
# carousel/combobox instances, a demo wrapper written as `max-width: 20rem`
# alone rendered at 200px against a 320px cap, and gained nothing until
# `width: 100%` was added next to it.
#
# `align-items: stretch` on the frame was measured NOT to fix this (an
# explicit `margin: 0 auto` on the wrapper wins over `align-self: stretch`
# every time) and to be actively harmful (it widened ~80 other direct
# children that carry no `max-width` at all, e.g. plain buttons). See row
# 94's own measurement writeup. This script therefore does not touch the
# frame's CSS at all -- it only checks each DEMO WRAPPER, the one place
# row 94 says the fix belongs.
#
# WHAT THIS CHECKS
#
# Every `preview/src/components/<name>/variants/<variant>/mod.rs`'s
# `pub fn Demo() -> Element` is (per `preview/src/main.rs`'s
# `ComponentVariantHighlight`) rendered as the DIRECT CHILD of
# `.dx-component-preview-frame` -- so this script resolves exactly that
# function's OUTERMOST rendered element (one hop through a same-folder
# `component.rs` wrapper function when the root is a bare call like
# `FormFixture {}` or `Gallery { children }`, since those are themselves
# `div { class: "..." }` one level down) and checks ONLY that element:
#
#   - an inline `style: "...")` string containing `max-width` but no bare
#     `width` (a distinct property -- `max-width`/`min-width` do not
#     count) is flagged;
#   - a `class: "..."` (or `Styles::x` / `#[css_module]`-generated ident,
#     read back from the same folder's stylesheet) whose CSS rule(s) --
#     across every block for that selector, including a `@media` override
#     -- ever declare `max-width` but never declare `width` anywhere for
#     that same selector is flagged.
#
# `width: fit-content` counts as satisfying the check: it already
# expresses "shrink-wrap on purpose", the same outcome this bug produces
# by accident, so a wrapper that asks for it explicitly is not the bug
# (e.g. `.dx-calendar`, out of scope on exactly this ground).
#
# WHAT THIS DOES NOT CATCH -- said plainly, per this repo's convention:
#
#   - A NESTED element several levels below the Demo's root (e.g.
#     `EmptyHeader`'s `.dx-empty-header`, inside `Empty`'s own internal
#     flex column) is never a direct child of the frame, so it is
#     correctly out of scope -- but this script only resolves ONE hop
#     through a wrapper component, so a root built from two or more
#     nested same-folder wrapper calls would not be walked further and
#     could go unchecked. None do today (checked by hand against every
#     hit this script currently allows through clean).
#   - A wrapper whose max-width-bearing class lives in a DIFFERENT
#     component's folder (a cross-folder import) is not resolved; none do
#     today.
#   - A `<dialog>`/`popover`-based overlay's own content wrapper (dialog,
#     alert_dialog, sheet, drawer, popover, dropdown_menu, menubar,
#     navigation_menu, select, command, toast, tooltip) is a legitimate,
#     deliberate exception this script does NOT special-case by name --
#     it simply never matches, because every one of those is the demo's
#     TRIGGER-plus-content composition, not a bare styled div/class at the
#     Demo function's own root; the max-width-bearing element inside them
#     sits below at least one more hop of composition than this script
#     resolves.
#
# Usage: scripts/check-demo-wrapper-width.sh
# Exit status: 0 if every demo wrapper with a `max-width` also sets a
# `width`, 1 otherwise.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 - <<'PYEOF'
import re
import sys
import glob
import os

def strip_comments(text):
    return re.sub(r"/\*.*?\*/", "", text, flags=re.S)

def find_matching_brace(text, open_idx):
    """`text[open_idx]` must be '{'. Returns the index of its matching '}'."""
    depth = 0
    for i in range(open_idx, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    return len(text) - 1

def extract_demo_root(src):
    """Given a `variants/*/mod.rs` file's full text, return (kind, name,
    attrs_text) for the outermost element rendered by `pub fn Demo() ->
    Element`, or None if it can't be found. `kind` is 'tag' (lowercase
    element) or 'component' (uppercase call); `attrs_text` is the text of
    that element's own attribute list (up to its first child/nested brace,
    or its self-closing `}`)."""
    text = strip_comments(src)
    m = re.search(r"pub\s+fn\s+Demo\s*\(\s*\)\s*->\s*Element\s*\{", text)
    if not m:
        return None
    fn_start = find_matching_brace(text, text.index("{", m.end() - 1))
    body = text[m.end():fn_start]
    rm = re.search(r"rsx!\s*\{", body)
    if not rm:
        return None
    rsx_end = find_matching_brace(body, body.index("{", rm.end() - 1))
    rsx_body = body[rm.end():rsx_end]

    # Walk top-level items in rsx_body, skipping non-visual leading
    # elements (`style { ... }` literal tags, comments, blank lines,
    # `let`-style statements are not expected here) to find the first
    # real element.
    i = 0
    n = len(rsx_body)
    while i < n:
        while i < n and rsx_body[i].isspace():
            i += 1
        if i >= n:
            return None
        em = re.match(r"([A-Za-z_][A-Za-z0-9_:]*)\s*\{", rsx_body[i:])
        if not em:
            # Not an element start we recognise (e.g. a bare expression
            # statement) -- bail out rather than guess.
            return None
        ident = em.group(1)
        brace_pos = i + em.end() - 1
        close = find_matching_brace(rsx_body, brace_pos)
        if ident == "style" or ident.split("::")[0] == "document":
            # A literal `style { {INLINE_STYLE} }` tag, or a
            # `document::Link { ... }` stylesheet include -- neither is
            # the visual root; skip it and keep looking.
            i = close + 1
            continue
        attrs_text = rsx_body[i + em.end():close]
        kind = "component" if ident[0].isupper() else "tag"
        return (kind, ident, attrs_text)
    return None

def style_attr_value(attrs_text):
    m = re.search(r'style\s*:\s*"((?:[^"\\]|\\.)*)"', attrs_text)
    return m.group(1) if m else None

def class_attr_value(attrs_text):
    m = re.search(r'class\s*:\s*"((?:[^"\\]|\\.)*)"', attrs_text)
    return m.group(1) if m else None

WIDTH_RE = re.compile(r"(?<![\w-])width\s*:", re.I)
MAXWIDTH_RE = re.compile(r"max-width\s*:", re.I)
FIT_CONTENT_RE = re.compile(r"(?<![\w-])width\s*:\s*fit-content\b", re.I)

def style_string_is_bug(style_str):
    """True if this inline `style="..."` string sets `max-width` without
    a bare `width` (fit-content counts as setting it)."""
    if not MAXWIDTH_RE.search(style_str):
        return False
    if FIT_CONTENT_RE.search(style_str):
        return False
    return not WIDTH_RE.search(style_str)

def selector_targets_class_itself(selector, class_name):
    """True if some comma-separated part of `selector` targets an element
    bearing `.class_name` DIRECTLY -- the class is the rightmost simple
    selector in that part, with at most further compound continuations
    (`:hover`, `[attr=...]`, `.other-class`, no separating space) after
    it. A descendant rule like `.class_name [data-slot=...]` does NOT
    qualify (that targets a DIFFERENT element, nested inside one bearing
    the class) -- exactly the case that must not be mistaken for a
    `width`/`max-width` declared on the class's own box."""
    class_pat = re.compile(r"\." + re.escape(class_name) + r"(?![\w-])")
    for part in selector.split(","):
        for m in class_pat.finditer(part):
            # `.rstrip()`: only trailing whitespace before the rule's `{`
            # is a formatting artifact to ignore -- LEADING whitespace
            # right after the class is a real descendant combinator and
            # must still fail the match below.
            after = part[m.end():].rstrip()
            if re.fullmatch(r"(?:[:.\[][^\s,]*)*", after):
                return True
    return False

def css_rule_bodies_for_class(css_text, class_name):
    """Every declaration block that targets `.class_name` ITSELF (see
    `selector_targets_class_itself`), anywhere in the file (base rule or a
    `@media` override)."""
    text = strip_comments(css_text)
    bodies = []
    pattern = re.compile(r"([^{}]+)\{")
    pos = 0
    while True:
        m = pattern.search(text, pos)
        if not m:
            break
        selector = m.group(1)
        open_idx = m.end() - 1
        close_idx = find_matching_brace(text, open_idx)
        if selector_targets_class_itself(selector, class_name):
            bodies.append(text[open_idx + 1:close_idx])
        pos = m.end()
    return bodies

def class_is_bug(css_text, class_name):
    bodies = css_rule_bodies_for_class(css_text, class_name)
    if not bodies:
        return None  # class not found in this stylesheet -- can't judge
    has_max_width = any(MAXWIDTH_RE.search(b) for b in bodies)
    if not has_max_width:
        return False
    has_fit_content = any(FIT_CONTENT_RE.search(b) for b in bodies)
    if has_fit_content:
        return False
    has_width = any(WIDTH_RE.search(b) for b in bodies)
    return not has_width

def resolve_component_root(folder, ident):
    """One hop: `ident {}` calls a same-folder `component.rs` function
    `pub fn <ident>` (or `pub fn <ident>(...)`); return that function's
    own outermost element the same shape `extract_demo_root` returns, or
    None if it can't be resolved (a different folder, a primitive import,
    a props-taking signature this script doesn't attempt to trace, ...)."""
    comp_path = os.path.join(folder, "component.rs")
    if not os.path.isfile(comp_path):
        return None
    src = strip_comments(open(comp_path, encoding="utf-8").read())
    m = re.search(r"pub\s+fn\s+" + re.escape(ident) + r"\s*\(", src)
    if not m:
        return None
    brace_m = re.search(r"\{", src[m.end():])
    if not brace_m:
        return None
    fn_open = m.end() + brace_m.start()
    fn_close = find_matching_brace(src, fn_open)
    body = src[fn_open + 1:fn_close]
    rm = re.search(r"rsx!\s*\{", body)
    if not rm:
        return None
    rsx_open = body.index("{", rm.end() - 1)
    rsx_close = find_matching_brace(body, rsx_open)
    rsx_body = body[rsx_open + 1:rsx_close]
    i = 0
    n = len(rsx_body)
    while i < n:
        while i < n and rsx_body[i].isspace():
            i += 1
        if i >= n:
            return None
        em = re.match(r"([A-Za-z_][A-Za-z0-9_:]*)\s*\{", rsx_body[i:])
        if not em:
            return None
        elem_ident = em.group(1)
        brace_pos = i + em.end() - 1
        close = find_matching_brace(rsx_body, brace_pos)
        if elem_ident in ("document",):
            i = close + 1
            continue
        if elem_ident.split("::")[0] == "document":
            # `document::Link { rel: "stylesheet", ... }` -- a stylesheet
            # include, not a visual element; keep looking.
            i = close + 1
            continue
        attrs_text = rsx_body[i + em.end():close]
        kind = "component" if elem_ident[0].isupper() else "tag"
        return (kind, elem_ident, attrs_text)
    return None

errors = []

for mod_path in sorted(glob.glob("preview/src/components/*/variants/*/mod.rs")):
    folder = os.path.dirname(os.path.dirname(os.path.dirname(mod_path)))
    name = folder.split("/")[-1]
    if name in ("carousel",):
        continue  # row 94: already fixed, own mechanism documented in-file
    src = open(mod_path, encoding="utf-8").read()
    root = extract_demo_root(src)
    if root is None:
        continue
    kind, ident, attrs_text = root

    # One hop through a same-folder `component.rs` wrapper (`FormFixture
    # {}`, `Gallery { children }`, ...) when the Demo root itself carries
    # neither a `style` nor a `class` attribute of its own.
    if kind == "component" and style_attr_value(attrs_text) is None and class_attr_value(attrs_text) is None:
        resolved = resolve_component_root(folder, ident)
        if resolved is not None:
            kind, ident, attrs_text = resolved

    style_str = style_attr_value(attrs_text)
    if style_str is not None:
        if style_string_is_bug(style_str):
            errors.append(
                f"{mod_path}: Demo root's inline `style` sets `max-width` "
                f"with no `width` (dev-docs/backlog.md row 94): {style_str!r}"
            )
        continue

    class_str = class_attr_value(attrs_text)
    if class_str is None:
        continue
    css_path = os.path.join(folder, "style.css")
    css_text = open(css_path, encoding="utf-8").read() if os.path.isfile(css_path) else ""
    # Also check the .rs file's own inline stylesheet (a `const X: &str =
    # r#"..."#` fed to a literal `style { {X} }` tag, e.g.
    # `drag_and_drop_list`'s `INLINE_STYLE`), since a demo-local CSS block
    # never lives in `style.css`.
    combined_css = css_text + "\n" + strip_comments(src)
    for cls in class_str.split():
        verdict = class_is_bug(combined_css, cls)
        if verdict:
            errors.append(
                f"{mod_path}: Demo root's `.{cls}` sets `max-width` with "
                f"no `width` anywhere for that selector "
                f"(dev-docs/backlog.md row 94)"
            )

for e in errors:
    print(e)

if errors:
    print()
    print(f"check-demo-wrapper-width: FAILED -- {len(errors)} demo wrapper(s) "
          f"set `max-width` with no `width`. See dev-docs/backlog.md row 94 "
          f"and this script's own header.")
    sys.exit(1)

print("check-demo-wrapper-width: OK")
PYEOF
