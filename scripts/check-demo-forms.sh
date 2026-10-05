#!/usr/bin/env bash
#
# check-demo-forms.sh
#
# Guards the construction that makes a demo's submit unable to navigate or reload the page
# (preview/src/components/form/component.rs `DemoForm`; oracle:
# playwright/oracle/tier2-html/demo-forms-no-navigation.spec.ts). Invisible to `cargo check`, clippy and
# `cargo test` -- a bare `form { .. }` compiles and looks right -- and only a click finds it, so with CI
# frozen this is the cheap static half of that oracle.
#
# THE DEFECT (owner report 2026-10-05: "submit buttons in the demo / docs pages should not trigger / reload
# the page"): a `<form>` with no `onsubmit` is a real HTML form. Clicking its submit button -- or pressing
# Enter in one of its fields -- runs the browser's form submission algorithm, which navigates to the form's
# action URL with the entry list as the query string (`/component/card/` -> `/component/card/?`): a full
# reload. Dioxus does not prevent it on its own: dioxus-web-0.7.9/src/dom.rs:113-116 calls `preventDefault`
# only when a handler cleared `Event::default_action_enabled()`, and a form with no handler never does.
# Found on the card's login form; the same class is any new demo form that forgets `prevent_default()`,
# and -- independent of the handler -- the whole pre-hydration window of the SSG pages, where no Rust
# handler is attached yet and a submit (or any submit with scripting off) is a native navigation.
#
# THE CONSTRUCTION: every form is `DemoForm`, which renders `method="dialog"` (a dialog-method form outside
# a `<dialog>` never navigates, per the HTML Standard's form submission algorithm -- so it holds before
# hydration, with JS disabled and under any CSP) AND an `onsubmit` that calls `prevent_default()`, and whose
# props cannot carry `action`/`method`/`target`. This gate keeps that true:
#
#   1. DemoForm exists in preview/src/components/form/component.rs, its `form` element has `method: "dialog"`
#      (literal, or via `..merged` where `merged` starts as `vec![Attribute::new("method", "dialog", None, false)]`
#      and the caller's attributes are appended -- `method` stays out of the caller's reach, and no literal
#      `method` sits beside a spread for check-attr-spread-collision to flag) and an `onsubmit` that calls `prevent_default()`, no `action:`/`target:`, and its
#      props do not `extends = form` (a caller could then pass `method`/`action` and undo it).
#   2. No raw `form { .. }` element anywhere else in preview/src, except in the native-`<dialog>` reference
#      fixture (preview/src/components/top_layer/component.rs, where submit is SUPPOSED to close a real
#      `<dialog>` and which therefore cannot call prevent_default) -- and there too it must carry the literal
#      `method: "dialog"`, so even the exemption cannot navigate.
#   3. Every `form: "<id>"` attribute (a submit button outside its form) must name the id of a DemoForm (or
#      the exempt dialog form) in preview/src: a button must not be able to submit a form this gate cannot see.
#   4. No form built from a string: `<form` in a string literal, `createElement("form")`, and, inside string
#      literals (JS run through `document::eval`), `.submit()` / `.requestSubmit(` -- `form.submit()` skips
#      the submit event entirely, so `prevent_default()` could not stop it.
#
# If you are here because it failed: replace `form {` with `DemoForm {` (`use crate::components::form::DemoForm;`,
# same attributes and children; add `show_status: false` if the page already reports the result itself, and
# pass the old `onsubmit` body as `onsubmit:` -- the `prevent_default()` is now DemoForm's). Do not weaken the
# gate to make a raw form pass.
#
# Usage: scripts/check-demo-forms.sh [SRC_ROOT]      (default: preview/src; a different root is for self-tests)
# Exit 0: clean. Exit 1: a violation, listed on stderr.

set -o pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

ROOT="${1:-preview/src}"
DEMO_FORM_FILE="$ROOT/components/form/component.rs"
# Files that may hold a RAW `form {` (each such element must still say `method: "dialog"`).
NATIVE_DIALOG_FILES="$ROOT/components/top_layer/component.rs"

[ -d "$ROOT" ] || { echo "check-demo-forms: $ROOT is not a directory" >&2; exit 2; }

bad=0
fail() { echo "check-demo-forms: $1" >&2; bad=1; }

if [ ! -f "$DEMO_FORM_FILE" ]; then
  fail "$DEMO_FORM_FILE is missing (it defines DemoForm, the only way a preview page renders a <form>)"
fi

# Candidate files only: the scanner below is a character-level pass, so spare it the files that cannot matter.
mapfile -t files < <(grep -rlE '(^|[^A-Za-z0-9_#.:])form[[:space:]]*\{|<form|createElement|\.submit\(|requestSubmit|(^|[^A-Za-z0-9_#.])form[[:space:]]*:[[:space:]]*"|DemoForm|extends[[:space:]]*=[[:space:]]*form' \
  "$ROOT" --include='*.rs' 2>/dev/null | sort)

# One scan per file. Prints machine-readable lines:  ERR <line> <message> | ID <id> | REF <line> <id>
scan() {
  awk -v demo_file="$2" -v dialog_files="$3" -v this_file="$1" '
    function isid(c) { return c ~ /^[A-Za-z0-9_]$/ }
    function pad(n,   s) { if (n <= 0) return ""; s = sprintf("%" n "s", ""); return s }

    # --- pass 1: per line, CODL = strings and comments blanked; NOCL = comments blanked, strings kept ---
    # States: 0 code, 1 string, 2 raw string, 3 block comment. Positions are identical in all three arrays.
    {
      line = $0; n = length(line); oc = ""; on = ""; i = 1
      while (i <= n) {
        c = substr(line, i, 1); nx = substr(line, i + 1, 1)
        if (st == 3) {
          if (c == "*" && nx == "/") { oc = oc "  "; on = on "  "; i += 2; st = 0; continue }
          oc = oc " "; on = on " "; i++; continue
        }
        if (st == 1) {
          if (c == "\\") { oc = oc " " (nx == "" ? "" : " "); on = on c nx; i += 2; continue }
          if (c == "\"") { oc = oc c; on = on c; st = 0; i++; continue }
          oc = oc " "; on = on c; i++; continue
        }
        if (st == 2) {
          if (c == "\"" && substr(line, i + 1, rh) == hashes) { oc = oc c hashes; on = on c hashes; i += 1 + rh; st = 0; continue }
          oc = oc " "; on = on c; i++; continue
        }
        if (c == "/" && nx == "/") { p = n - i + 1; oc = oc pad(p); on = on pad(p); break }
        if (c == "/" && nx == "*") { oc = oc "  "; on = on "  "; i += 2; st = 3; continue }
        if (c == "r" && (i == 1 || !isid(substr(line, i - 1, 1)))) {
          j = i + 1; h = ""
          while (substr(line, j, 1) == "#") { h = h "#"; j++ }
          if (substr(line, j, 1) == "\"") {
            hashes = h; rh = length(h); st = 2
            seg = substr(line, i, j - i + 1); oc = oc seg; on = on seg; i = j + 1; continue
          }
        }
        if (c == "\"") { oc = oc c; on = on c; st = 1; i++; continue }
        if (c == "\047") {
          if (nx == "\\") {
            k = index(substr(line, i + 3), "\047")
            if (k > 0) { seg = substr(line, i, k + 3); oc = oc "\047" pad(length(seg) - 2) "\047"; on = on seg; i += k + 3; continue }
          } else if (substr(line, i + 2, 1) == "\047" && i + 2 <= n) {
            oc = oc "\047 \047"; on = on substr(line, i, 3); i += 3; continue
          }
        }
        oc = oc c; on = on c; i++
      }
      CODL[NR] = oc; NOCL[NR] = on
    }

    # --- helpers over the arrays ---
    # Walk from the `{` at (l, c) to its matching `}`. TOP = the element text with nested {..} blanked (so only
    # attributes written directly on the element remain); FULL = everything inside. Both come from NOCL.
    function element(l, c,    depth, ch, raw, line, col, top, full, m) {
      depth = 0; top = ""; full = ""
      for (line = l; line <= NR; line++) {
        col = (line == l) ? c : 1
        m = length(CODL[line])
        for (; col <= m; col++) {
          ch = substr(CODL[line], col, 1); raw = substr(NOCL[line], col, 1)
          if (ch == "{") { depth++; if (depth > 1) { top = top " "; full = full raw }; continue }
          if (ch == "}") { depth--; if (depth == 0) { TOP = top; FULL = full; return 1 } ; top = top " "; full = full raw; continue }
          full = full raw
          top = top (depth == 1 ? raw : " ")
        }
        top = top "\n"; full = full "\n"
      }
      TOP = top; FULL = full; return 0
    }
    function is_dialog_file(   a, k, na) {
      na = split(dialog_files, a, " ")
      for (k = 1; k <= na; k++) if (a[k] == this_file) return 1
      return 0
    }
    function err(l, msg) { print "ERR\t" l "\t" msg }

    END {
      in_demo = (this_file == demo_file)
      in_dialog = is_dialog_file()
      for (l = 1; l <= NR; l++) {
        code = CODL[l]
        if (in_demo) FULLSRC = FULLSRC NOCL[l] "\n"

        # (2) raw `form {` elements.
        rest = " " code; off = 0
        while (match(rest, /[^A-Za-z0-9_#.:]form[ \t]*\{/)) {
          ms = RSTART; ml = RLENGTH
          col = off + ms + ml - 2                  # column of the `{` in CODL[l] (rest carries one leading pad char)
          element(l, col)
          if (!in_demo && !in_dialog) {
            err(l, "raw `form {` -- use `DemoForm` (crate::components::form::DemoForm): a bare form navigates (reloads) the page on submit")
          } else {
            has_dialog = (TOP ~ /method[ \t\n]*:[ \t\n]*"dialog"/)
            if (in_demo && has_dialog) dialog_ok++
            # DemoForm\047s own rsx element takes `..merged`, built from `attributes!(form { method: "dialog" })`.
            if (in_demo && !has_dialog && TOP ~ /\.\.[ \t]*merged/) spread_forms++
            else if (!has_dialog)
              err(l, "`form {` without the literal `method: \"dialog\"` -- that attribute is what makes a submit before hydration / with JS off unable to navigate")
            if (TOP ~ /(^|[^A-Za-z0-9_])(action|target|formaction|formmethod|formtarget)[ \t\n]*:/)
              err(l, "`form {` sets action/target: a form that can navigate")
            if (in_demo) {
              demo_forms++
              if (FULL ~ /onsubmit/ && FULL ~ /prevent_default\(\)/) demo_prevented++
            }
          }
          if (TOP ~ /(^|[^A-Za-z0-9_])id[ \t\n]*:[ \t\n]*"[^"]*"/) {
            match(TOP, /(^|[^A-Za-z0-9_])id[ \t\n]*:[ \t\n]*"[^"]*"/)
            idm = substr(TOP, RSTART, RLENGTH); sub(/^[^"]*"/, "", idm); sub(/"$/, "", idm)
            print "ID\t" idm
          }
          off = col
          rest = " " substr(rest, ms + ml)
        }

        # DemoForm call sites: their ids are the legitimate targets of a `form: "<id>"` attribute.
        rest = " " code; off = 0
        while (match(rest, /[^A-Za-z0-9_]DemoForm[ \t]*\{/)) {
          ms = RSTART; ml = RLENGTH
          col = off + ms + ml - 2
          element(l, col)
          if (TOP ~ /(^|[^A-Za-z0-9_])id[ \t\n]*:[ \t\n]*"[^"]*"/) {
            match(TOP, /(^|[^A-Za-z0-9_])id[ \t\n]*:[ \t\n]*"[^"]*"/)
            idm = substr(TOP, RSTART, RLENGTH); sub(/^[^"]*"/, "", idm); sub(/"$/, "", idm)
            print "ID\t" idm
          }
          off = col
          rest = " " substr(rest, ms + ml)
        }

        # (3) `form: "<id>"` attribute (a submit button outside its form).
        rest = " " code; off = 0
        while (match(rest, /[^A-Za-z0-9_#.]form[ \t]*:[ \t]*"/)) {
          ms = RSTART; ml = RLENGTH
          col = off + ms + ml - 2                   # column of the opening quote
          val = substr(NOCL[l], col + 1); sub(/".*$/, "", val)
          if (val ~ /[{}]/) err(l, "`form: \"" val "\"` is not a literal id -- use a literal so this gate can resolve it to a DemoForm")
          else print "REF\t" l "\t" val
          off = col
          rest = " " substr(rest, ms + ml)
        }

        # (4) forms built from strings.
        if (!in_demo) {
          s = NOCL[l]
          if (s ~ /<form([ \t>\/]|$)/ ) err(l, "`<form` in a string literal -- a form that is not a DemoForm")
          if (s ~ /createElement\([ \t]*["\047`]form["\047`]/) err(l, "createElement(\"form\") -- a form that is not a DemoForm")
          # `.submit()` / `.requestSubmit(` only count INSIDE a string literal (JS), not as a Rust method call.
          t = s
          while (match(t, /\.(submit\(\)|requestSubmit\()/)) {
            p = length(s) - length(t) + RSTART
            if (substr(CODL[l], p, 1) == " ") err(l, "form.submit()/requestSubmit() in a string literal -- submit() skips the submit event, so prevent_default() cannot stop it")
            t = substr(t, RSTART + RLENGTH)
          }
        }
        # (1) the DemoForm props must not extend the `form` element (a caller could then pass method/action).
        if (in_demo && code ~ /extends[ \t]*=[ \t]*form([^A-Za-z0-9_]|$)/) err(l, "DemoForm props `extends = form`: callers could pass method/action and undo the construction (extend GlobalAttributes only)")
        if (in_demo && code ~ /pub[ \t]+fn[ \t]+DemoForm[ \t]*\(/) has_def = 1
      }
      if (in_demo) {
        if (!has_def) err(0, "`pub fn DemoForm(` not found -- the one component every preview form must use")
        if (spread_forms > 0 && !(FULLSRC ~ /merged[ \t]*=[ \t]*vec!\[[ \t]*Attribute::new\([ \t]*"method"[ \t]*,[ \t]*"dialog"/))
          err(0, "DemoForm\047s form takes `..merged` but `merged` does not start from `vec![Attribute::new(\"method\", \"dialog\", None, false)]`")
        if (spread_forms > 0 && !(FULLSRC ~ /merged\.extend\([ \t]*attributes[ \t]*\)/))
          err(0, "DemoForm must APPEND the caller\047s attributes (`merged.extend(attributes)`) so they cannot replace `method`")
        if (demo_forms < 1) err(0, "DemoForm renders no `form {` element")
        else if (demo_prevented < 1) err(0, "DemoForm form element must have an `onsubmit` that calls `prevent_default()`")
      }
    }
  ' "$1" | sed "s|^|$1\t|"
}

declare -A known_ids=()
refs=()
for f in "${files[@]}"; do
  while IFS=$'\t' read -r file kind a b; do
    case "$kind" in
      ERR) if [ "$a" = 0 ]; then fail "$file: $b"; else fail "$file:$a: $b"; fi ;;
      ID) known_ids["$a"]=1 ;;
      REF) refs+=("$file:$a|$b") ;;
    esac
  done < <(scan "$f" "$DEMO_FORM_FILE" "$NATIVE_DIALOG_FILES")
done

# DemoForm itself must exist even if no candidate file matched it (the file was emptied / renamed).
if [ -f "$DEMO_FORM_FILE" ] && ! printf '%s\n' "${files[@]}" | grep -qx "$DEMO_FORM_FILE"; then
  fail "$DEMO_FORM_FILE no longer defines DemoForm"
fi

for r in "${refs[@]}"; do
  where="${r%%|*}"; id="${r#*|}"
  [ -n "${known_ids[$id]:-}" ] || fail "$where: \`form: \"$id\"\` names no DemoForm (or exempt dialog form) with \`id: \"$id\"\` in $ROOT -- a button must not submit a form this gate cannot see"
done

if [ "$bad" -ne 0 ]; then
  echo "check-demo-forms: FAILED. Every <form> in the preview must be a DemoForm (see this script's header)." >&2
  exit 1
fi
echo "check-demo-forms: OK -- every <form> in $ROOT is a DemoForm (or the native <dialog> reference), each with method=\"dialog\"."
