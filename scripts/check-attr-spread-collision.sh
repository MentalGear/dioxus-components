#!/usr/bin/env bash
#
# check-attr-spread-collision.sh
#
# Flags a literal attribute rendered beside a raw `..spread` on the same element, where
# the enclosing component's props have no typed field claiming that name:
#
#   div {
#       class: "dx-alert",          // literal
#       role: "alert",
#       ..attributes,                // #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>
#   }
#
# If a caller passes the same attribute name, SSR emits both and the HTML parser keeps
# the FIRST (the component's own literal); CSR applies both via `setAttribute` and keeps
# the LAST (the caller's). Hydration does not reconcile the two: `set_attribute` is gated
# behind `skip_mutations()` throughout the hydration pass. The result is that a caller's
# override works in `dx serve --web` (pure CSR, where the DOM's own `setAttribute`
# coalesces the two calls and the second -- the caller's -- wins) and is silently
# discarded in this repo's actual deployment mode, SSR+hydrate (`dx build --ssg`), where
# the browser's HTML parser resolves the duplicate to the first, wrong value and nothing
# downstream ever corrects it. Full trace, every claim cited against this lockfile's own
# `dioxus-core`/`dioxus-ssr`/`dioxus-web` source:
# dev-docs/issues/duplicate-attribute-root-cause.md.
#
# The discriminator is the typed field: a props struct with a typed field of that name
# (e.g. `PopoverRootProps::id`, reconciled via `use_id_or`, `primitives/src/lib.rs`)
# resolves the caller's value into ONE literal before it ever reaches the catch-all
# `attributes: Vec<Attribute>` -- there is only ever one `Attribute` for that name, so
# there is nothing to collide, and that site is correctly not reported.
#
# WHAT THIS DOES NOT DETECT -- the `5fc1439`-shaped variant is reported separately, see
# "The second, separate check" below; beyond that:
#   - An `rsx! { .. }` invocation written lexically INSIDE another `rsx! { .. }`'s own
#     token stream (as opposed to a plain nested element/component, which IS handled --
#     see the analyzer's own header). Believed rare-to-nonexistent in idiomatic Dioxus
#     code, where nesting is expressed via child elements, not a second macro call.
#   - Anything that isn't literally "one element, one spread, one same-named literal" --
#     e.g. two DIFFERENT elements each independently receiving half of a caller's intent.
#
# History: three reactive, one-incident-at-a-time sweeps already hit this exact class
# (`b35d671`, `5fc1439`, `f2be1d7` -- dev-docs/backlog.md rows 85/93) before this gate
# existed, each catching only the sites some demo page happened to exercise. This is the
# fix-by-construction CLAUDE.md's "stop patching instances" principle calls for.
#
# THE RATCHET, NOT A WALL (read this before touching the baseline file):
# `scripts/check-attr-spread-collision.baseline.tsv` (this check) and
# `scripts/check-attr-spread-collision.component-forward.baseline.tsv` (the second check,
# below) each list the sites already known to exist when this gate was introduced --
# hundreds of them, across most of this codebase's themed wrapper components. This gate
# does NOT fail on any of those: it fails only when a site appears that ISN'T already in
# the relevant baseline file. Each baseline entry is `file<TAB>enclosing-fn<TAB>name`, one
# line per finding (duplicates are meaningful -- see below), NOT `file:line`, because a
# line-pinned baseline would spuriously "lose" every pre-existing entry (and so falsely
# report it as newly INTRODUCED, since the ratchet only ever compares the two sets) the
# moment an unrelated edit earlier in the same file shifts line numbers -- a `cargo fmt`
# pass, an added doc comment, anything. `file+fn+name` is coarser (two distinct elements
# in the same component sharing an attribute name collapse to the same key) but is stable
# under exactly that kind of unrelated churn; where a key's multiplicity in the baseline
# genuinely is >1 (two real sites), the baseline lists that key twice and the diff below
# is a proper multiset comparison, not a plain set comparison, so it stays exact.
#
# BASELINE ENTRIES ARE A DEBT REGISTER, NOT AN APPROVAL. Every line in either baseline
# file is a real, live instance of the production defect described above, left unfixed
# only because fixing ~700 call sites was explicitly out of scope for the lane that wrote
# this gate (it added detection only) -- see dev-docs/issues/duplicate-attribute-findings.md
# for the full, actionable breakdown. Do not add a NEW site to the baseline to make this
# gate pass; the baseline exists only so this gate can ship without also requiring every
# pre-existing site to be fixed in the same change.
#
# Shrinking the baseline (a site got fixed) is supported and detected: when the current
# run finds FEWER matches for a baseline entry than the baseline lists, this script prints
# an informational note naming the surplus (still exit 0) rather than silently accepting
# it, so progress is visible and `--update-baseline` can lock it in. Note what this does
# NOT catch: if a key's count in the baseline stays the same while the ACTUAL fixed site
# is replaced by a coincidentally-same-shaped NEW site elsewhere (same file, same
# enclosing fn, same attribute name), the multiset comparison cannot tell those apart --
# a known, accepted coarseness of a file+fn+name key, not a line-exact one. Regenerating
# the baseline promptly after a real fix (rather than letting it drift) is the mitigation;
# `--update-baseline` makes that a one-line operation.
#
# Usage:
#   scripts/check-attr-spread-collision.sh                 # gate mode (CI/pre-commit)
#   scripts/check-attr-spread-collision.sh --update-baseline
#       Regenerates both baseline files from the current findings and exits 0. Run this
#       deliberately, after confirming (by reading the diff) that every change is either a
#       genuine fix being locked in or a genuine new instance being knowingly accepted into
#       the debt register -- never as a reflex to make a red gate green.
#
# Exit 0: no new violation in either check (pre-existing baseline entries do not fail the
# gate; see above). Exit 1: a site was found that is not in the relevant baseline file,
# with file:line detail on stderr.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

SPREAD_BASELINE="scripts/check-attr-spread-collision.baseline.tsv"
COMPONENT_FORWARD_BASELINE="scripts/check-attr-spread-collision.component-forward.baseline.tsv"

CHECKER_DIR="scripts/attr-spread-collision-checker"
# Must be EXPORTED, not just assigned: `cargo build` runs as a subprocess, and an
# unexported shell variable never reaches it. Without the export cargo falls back
# to whatever `target-dir` the ambient cargo config names (this image's
# /root/.cargo/config.toml points at the main checkout's own target/), while the
# $BIN check below still looks at the relative default -- so the two agree in the
# main checkout by coincidence and diverge in every git worktree, failing with a
# spurious "built but not there". Found 2026-09-22 by an agent running this gate
# from a worktree.
: "${CARGO_TARGET_DIR:=target}"
export CARGO_TARGET_DIR

echo "check-attr-spread-collision: building analyzer..." >&2
if ! cargo build --release --offline -p attr-spread-collision-checker >&2; then
  echo "check-attr-spread-collision: FAILED -- could not build $CHECKER_DIR" >&2
  exit 1
fi

BIN="$CARGO_TARGET_DIR/release/attr-spread-collision-checker"
if [[ ! -x "$BIN" ]]; then
  echo "check-attr-spread-collision: FAILED -- built but $BIN is not there (unexpected CARGO_TARGET_DIR layout?)" >&2
  exit 1
fi

raw_tsv="$(mktemp -t attr-spread-collision.XXXXXX.tsv)"
trap 'rm -f "$raw_tsv"' EXIT

"$BIN" >"$raw_tsv" 2>&2

update_mode=0
if [[ "${1:-}" == "--update-baseline" ]]; then
  update_mode=1
fi

# `file<TAB>fn<TAB>name`, sorted, WITH duplicates -- a multiset, matching the baseline
# files' own format (see the header comment above for why duplicates are meaningful).
extract_keys() {
  local kind="$1"
  awk -F'\t' -v k="$kind" 'BEGIN{OFS="\t"} $1==k{print $2,$4,$5}' "$raw_tsv" | LC_ALL=C sort
}

# Human-readable detail lines (file:line: ...) for every raw finding whose
# file+fn+name key is in the given key list (one per line, stdin).
detail_for_keys() {
  local kind="$1"
  while IFS=$'\t' read -r file fn name; do
    [[ -z "$file" ]] && continue
    awk -F'\t' -v k="$kind" -v f="$file" -v fn="$fn" -v n="$name" \
      '$1==k && $2==f && $4==fn && $5==n {printf "%s:%s: literal `%s` beside a spread in `%s` -- %s\n", $2, $3, $5, $4, $6}' \
      "$raw_tsv"
  done
}

overall_status=0
total_new=0
total_baseline=0

# Regenerated verbatim into each baseline file's own top comment block every time
# --update-baseline writes one, so the disclaimer travels with the data even for
# someone who opens only the .tsv and never reads this script.
baseline_header() {
  local label="$1"
  cat <<EOF
# DEBT REGISTER, NOT AN APPROVAL -- generated by
# scripts/check-attr-spread-collision.sh --update-baseline. Every line below is a real,
# live "$label" instance of the duplicate-attribute defect this gate looks for (see
# check-attr-spread-collision.sh's own header, and
# dev-docs/issues/duplicate-attribute-findings.md for the full breakdown), listed here
# only because fixing every pre-existing site was out of scope for the lane that added
# this gate. Format: file<TAB>enclosing-fn<TAB>attribute-name, one line per finding
# (duplicates meaningful -- a multiset, not a set; see the .sh header for why). Do not
# hand-edit: adding a line here to silence a real new finding defeats the gate: fix the
# site instead, or re-run with --update-baseline and explain why in the commit.
EOF
}

check_one() {
  local label="$1" kind="$2" baseline_file="$3"

  local current
  current="$(extract_keys "$kind")"

  if [[ "$update_mode" -eq 1 ]]; then
    { baseline_header "$label"; printf '%s\n' "$current"; } >"$baseline_file"
    local n
    n="$(grep -vc '^#' "$baseline_file" || true)"
    echo "check-attr-spread-collision: wrote $n $label entries to $baseline_file" >&2
    return 0
  fi

  local baseline=""
  if [[ -f "$baseline_file" ]]; then
    baseline="$(grep -v '^#' "$baseline_file" | grep -v '^[[:space:]]*$' || true)"
  fi

  local new_keys removed_keys
  new_keys="$(comm -23 <(printf '%s\n' "$current") <(printf '%s\n' "$baseline") 2>/dev/null || true)"
  removed_keys="$(comm -13 <(printf '%s\n' "$current") <(printf '%s\n' "$baseline") 2>/dev/null || true)"

  local current_count baseline_count new_count removed_count
  current_count="$(printf '%s\n' "$current" | grep -c . || true)"
  baseline_count="$(printf '%s\n' "$baseline" | grep -c . || true)"
  new_count="$(printf '%s\n' "$new_keys" | grep -c . || true)"
  removed_count="$(printf '%s\n' "$removed_keys" | grep -c . || true)"

  total_baseline=$((total_baseline + baseline_count))

  if [[ "$new_count" -gt 0 ]]; then
    total_new=$((total_new + new_count))
    overall_status=1
    echo "check-attr-spread-collision: $new_count new $label site(s) not in $baseline_file:" >&2
    printf '%s\n' "$new_keys" | detail_for_keys "$kind" >&2
    echo >&2
  fi

  if [[ "$removed_count" -gt 0 ]]; then
    echo "check-attr-spread-collision: NOTE -- $removed_count $label baseline entr$([ "$removed_count" -eq 1 ] && echo y || echo ies) no longer detected (progress). Run '$0 --update-baseline' to lock it in." >&2
  fi

  echo "check-attr-spread-collision: $label: $current_count found, $baseline_count in baseline, $new_count new." >&2
}

check_one "spread" "spread" "$SPREAD_BASELINE"
check_one "component-attributes-forward" "component-attributes-forward" "$COMPONENT_FORWARD_BASELINE"

if [[ "$update_mode" -eq 1 ]]; then
  echo "check-attr-spread-collision: baseline(s) updated." >&2
  exit 0
fi

if [[ "$overall_status" -ne 0 ]]; then
  echo "check-attr-spread-collision: FAILED -- $total_new new site(s) beyond the checked-in baseline ($total_baseline pre-existing, tracked as debt -- dev-docs/issues/duplicate-attribute-findings.md). Either fix the new site(s) (typed field, or merge_attributes -- see that doc) or, if genuinely intentional, add them via --update-baseline and say why in the commit." >&2
  exit 1
fi

echo "check-attr-spread-collision: OK -- 0 new site(s) beyond the $total_baseline-entry baseline (debt register, not an approval -- dev-docs/issues/duplicate-attribute-findings.md)."
