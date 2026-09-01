#!/usr/bin/env bash
#
# check-preview-composition.sh
#
# Enforces the preview app's composition rule (docs/preview-composition.md):
#
#   Preview markup composes ONLY themed wrappers from `crate::components::*`.
#   A raw `dioxus_primitives::` import is legitimate in exactly two places:
#
#     1. Inside the themed wrapper itself
#        (`preview/src/components/<name>/component.rs`) -- that file *is*
#        the layer responsible for pulling in the raw primitive and
#        attaching the theme's `css_module` class. Importing the raw
#        primitive there is the wrapper doing its job, not a violation.
#
#     2. Anywhere else in `preview/src`, for a small, explicit allowlist of
#        *non-markup* items -- hooks (`use_toast`) and plain value
#        types/enums (`CheckboxState`, `ToastOptions`, `Color`, `DateRange`,
#        `ContentSide`, `ScrollDirection`) that render nothing themselves
#        and so have no theme for a wrapper to attach.
#
#   Everything else -- a component/markup type such as `Switch`,
#   `SwitchThumb`, `Select`, `SelectTrigger`, `SelectList`, `SelectValue`,
#   or a module alias like `dioxus_primitives::select as primitive_select`
#   that pulls in a whole family of such components -- must come from
#   `crate::components::*` instead. Composing the raw primitive directly
#   skips the theme wrapper's `css_module` class, which is exactly the bug
#   this script exists to catch (see docs/preview-composition.md for the
#   two real incidents that motivated it).
#
# Usage: scripts/check-preview-composition.sh
# Exit status: 0 if the tree is clean, 1 and a listing of every offending
# line otherwise.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

preview_src="preview/src"
fail=0

# ---------------------------------------------------------------------------
# 1. Which files are exempt (the themed-wrapper layer)?
# ---------------------------------------------------------------------------
#
# By default every `preview/src/components/<name>/component.rs` is exempt:
# for the ~45 components that follow the normal pattern (Switch, Checkbox,
# Select, ...), that file's entire job is importing
# `dioxus_primitives::<name>::*` and attaching the theme's `css_module`
# class -- see e.g. preview/src/components/switch/component.rs.
#
# Two named files break that pattern and are handled explicitly instead of
# by the blanket exemption above:
#
#   - preview/src/components/form/component.rs is NOT a themed wrapper --
#     there is no `dioxus_primitives::form` primitive. It is a fixture page
#     (docs/conformance-harness.md tier 2) whose job is to compose *other*
#     components' THEMED wrappers, per its own header comment. This is the
#     file the reported bug (collapsed library Switch/Select in the form
#     fixture) lives in, so it is explicitly re-included in the scan below
#     rather than exempted by the general glob.
#
#   - preview/src/components/top_layer/component.rs is also not a themed
#     wrapper (no `dioxus_primitives::top_layer` primitive exists -- the
#     module isn't even `pub` in primitives/src/lib.rs) and, like `form`,
#     composes many other primitives directly (Dialog, Popover, Menubar,
#     Select, Combobox, Toast, ContextMenu, DropdownMenu, HoverCard,
#     Tooltip, AlertDialog). Unlike `form`, those raw compositions exist to
#     exercise native top-layer/promotion behaviour against native
#     `<dialog>`/`popover` references -- see its own header comment and
#     playwright/oracle/tier2-html/{top-layer,native-dialog}.spec.ts. That
#     is a deliberate, pre-existing, already-independently-tested choice
#     about interaction/positioning behaviour, not a theming bug, and
#     nobody has asked for it to change. Flagging it here would be exactly
#     the kind of false positive that gets a lint deleted rather than
#     fixed, so it stays exempt like the ~45 ordinary wrappers. If it is
#     ever migrated to compose themed wrappers, this exemption should be
#     removed in the same change.
is_exempt_wrapper_file() {
    case "$1" in
        "$preview_src"/components/*/component.rs)
            case "$1" in
                "$preview_src"/components/form/component.rs) return 1 ;;
                *) return 0 ;;
            esac
            ;;
        *) return 1 ;;
    esac
}

# Generated/docs-code files: preview/build.rs renders component markdown
# and highlighted code snippets, but writes its output under Cargo's
# `OUT_DIR` (outside the source tree), not into `preview/src` -- confirmed
# by reading preview/build.rs. There is no generated `.rs` under
# `preview/src` today, so no such exclusion is needed; this comment
# documents that the case was checked, not skipped.

mapfile -t all_files < <(find "$preview_src" -name '*.rs' | sort)

target_files=()
for f in "${all_files[@]}"; do
    if ! is_exempt_wrapper_file "$f"; then
        target_files+=("$f")
    fi
done

# ---------------------------------------------------------------------------
# 2. The non-markup allowlist.
# ---------------------------------------------------------------------------
#
# Keys are "module::Item" (the path segment(s) between `dioxus_primitives::`
# and the final imported name), or a bare item name for the handful of
# items re-exported straight off the crate root (e.g. `ContentSide`). Every
# entry below was verified by inspection against every
# `dioxus_primitives::` reference left in a non-exempt file, and is
# commented with why it's a value/hook rather than markup, and where it's
# used.
declare -A allowed_qualified=(
    ["toast::use_toast"]=1        # hook, not markup -- toast/variants/main/mod.rs,
                                   # dashboard/views/email_client/{compose,read_pane}.rs
    ["toast::ToastOptions"]=1     # plain builder struct, not markup -- same call sites
    ["checkbox::CheckboxState"]=1 # plain enum, not markup -- main.rs,
                                   # preview/src/components/form/component.rs
    ["color_picker::Color"]=1     # plain type, not markup -- main.rs,
                                   # color_picker/variants/main/mod.rs
    ["calendar::DateRange"]=1     # plain type, not markup -- calendar & date_picker
                                   # range/multi_month/unavailable_dates variants
    ["scroll_area::ScrollDirection"]=1 # plain enum, not markup -- scroll_area/variants/main
)
declare -A allowed_root=(
    ["ContentSide"]=1 # plain enum re-exported off the crate root, not markup --
                       # hover_card & tooltip variants/main
)

trim() {
    local s="$1"
    s="${s#"${s%%[![:space:]]*}"}"
    s="${s%"${s##*[![:space:]]}"}"
    printf '%s' "$s"
}

is_allowed() {
    local module="$1" item="$2"
    if [[ -z "$module" ]]; then
        [[ -n "${allowed_root[$item]:-}" ]]
    else
        [[ -n "${allowed_qualified["$module::$item"]:-}" ]]
    fi
}

report() {
    local file="$1" line_no="$2" line="$3" reason="$4"
    echo "VIOLATION: $file:$line_no: $reason"
    echo "    $line"
    fail=1
}

# ---------------------------------------------------------------------------
# 3. Scan.
# ---------------------------------------------------------------------------
for f in "${target_files[@]}"; do
    [[ -f "$f" ]] || continue
    # Process substitution (not a pipe) so `fail=1` set inside the loop
    # body is visible after the loop -- a piped `while` runs in a subshell
    # and would silently lose that assignment.
    while IFS=: read -r line_no line; do
        handled=0

        # Ignore anything after a `//` -- this repo's fixtures document past
        # incidents in prose that itself mentions raw `dioxus_primitives::`
        # paths (see the header comment in preview/src/components/form/
        # component.rs), and those mentions are not imports. Matches this
        # codebase's style of only ever using `//` to start a comment (never
        # inside a string literal on the same line as real code), so a
        # blanket "drop from the first `//` on" is safe here.
        code="${line%%//*}"
        [[ "$code" == *dioxus_primitives::* ]] || continue

        # Shape A: `use dioxus_primitives::MODULE::{ITEM, ITEM, ...};`
        if [[ "$code" =~ use[[:space:]]+dioxus_primitives::([A-Za-z0-9_]+)::\{([^}]*)\} ]]; then
            handled=1
            module="${BASH_REMATCH[1]}"
            items_raw="${BASH_REMATCH[2]}"
            IFS=',' read -ra items <<<"$items_raw"
            for raw_item in "${items[@]}"; do
                item="$(trim "$raw_item")"
                [[ -z "$item" || "$item" == "self" ]] && continue
                # `Foo as Bar` -- the imported name that matters is `Foo`.
                item="${item%% as *}"
                item="$(trim "$item")"
                if ! is_allowed "$module" "$item"; then
                    report "$f" "$line_no" "$line" \
                        "raw dioxus_primitives::${module}::${item} used outside a themed wrapper -- compose crate::components::${module}::${item} instead (or add it to the allowlist in this script if it is genuinely non-markup)"
                fi
            done

        # Shape B: `use dioxus_primitives::MODULE as ALIAS;` -- a whole
        # module aliased in, almost always to pull in several of its
        # components at once (e.g. `select as primitive_select`).
        elif [[ "$code" =~ use[[:space:]]+dioxus_primitives::([A-Za-z0-9_]+)[[:space:]]+as[[:space:]]+[A-Za-z0-9_]+\; ]]; then
            handled=1
            module="${BASH_REMATCH[1]}"
            report "$f" "$line_no" "$line" \
                "whole dioxus_primitives::${module} module aliased in outside a themed wrapper -- compose crate::components::${module}::* instead"

        # Shape C: `use dioxus_primitives::MODULE::ITEM;` (single item, no braces).
        elif [[ "$code" =~ use[[:space:]]+dioxus_primitives::([A-Za-z0-9_]+)::([A-Za-z0-9_]+)\; ]]; then
            handled=1
            module="${BASH_REMATCH[1]}"
            item="${BASH_REMATCH[2]}"
            if ! is_allowed "$module" "$item"; then
                report "$f" "$line_no" "$line" \
                    "raw dioxus_primitives::${module}::${item} used outside a themed wrapper -- compose crate::components::${module}::${item} instead (or add it to the allowlist in this script if it is genuinely non-markup)"
            fi

        # Shape D: `use dioxus_primitives::ITEM;` (single item off the crate root).
        elif [[ "$code" =~ use[[:space:]]+dioxus_primitives::([A-Za-z0-9_]+)\; ]]; then
            handled=1
            item="${BASH_REMATCH[1]}"
            if ! is_allowed "" "$item"; then
                report "$f" "$line_no" "$line" \
                    "raw dioxus_primitives::${item} used outside a themed wrapper -- compose the matching crate::components:: wrapper instead (or add it to the allowlist in this script if it is genuinely non-markup)"
            fi
        fi

        # Fallback: any other shape (inline fully-qualified paths outside a
        # `use` line, e.g. `dioxus_primitives::checkbox::CheckboxState::Checked`
        # in main.rs). Scan every occurrence on the line individually rather
        # than skipping it -- an unrecognized shape should be checked, not
        # silently passed.
        if [[ "$handled" -eq 0 ]]; then
            rest="$code"
            while [[ "$rest" =~ dioxus_primitives::([A-Za-z0-9_]+)(::([A-Za-z0-9_]+))? ]]; do
                module="${BASH_REMATCH[1]}"
                item="${BASH_REMATCH[3]:-}"
                if [[ -n "$item" ]]; then
                    if ! is_allowed "$module" "$item"; then
                        report "$f" "$line_no" "$line" \
                            "raw dioxus_primitives::${module}::${item} used outside a themed wrapper -- compose crate::components::${module}::${item} instead (or add it to the allowlist in this script if it is genuinely non-markup)"
                    fi
                else
                    if ! is_allowed "" "$module"; then
                        report "$f" "$line_no" "$line" \
                            "raw dioxus_primitives::${module} used outside a themed wrapper -- compose the matching crate::components:: wrapper instead (or add it to the allowlist in this script if it is genuinely non-markup)"
                    fi
                fi
                # Advance past this match so a second occurrence on the same
                # line (main.rs's CheckboxState::Checked / ::Unchecked pair)
                # is also checked.
                rest="${rest#*"${BASH_REMATCH[0]}"}"
            done
        fi
    done < <(grep -n 'dioxus_primitives::' "$f")
done

if [[ "$fail" -ne 0 ]]; then
    echo
    echo "check-preview-composition: FAILED -- see docs/preview-composition.md for the rule."
    exit 1
fi

echo "check-preview-composition: OK"
