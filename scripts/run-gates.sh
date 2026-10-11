#!/usr/bin/env bash
# The ONE way to run every pre-commit gate from CLAUDE.md, in order, with timings and a summary.
# Every workflow is `workflow_dispatch`-only (CI freeze, dev-docs/backlog.md row 33), so nothing
# else runs these -- and a green that was taken on a different tree, or with a weaker command
# than the one documented, is how a clippy error reached `main` (see "Clippy arms" below).
#
#   CARGO_TARGET_DIR=/abs/isolated/dir scripts/run-gates.sh [options]
#
#   --only REGEX      run only gates whose name matches REGEX (egrep)
#   --skip REGEX      skip gates whose name matches REGEX
#   --stop-on-fail    stop at the first red gate (default: run everything, report all reds)
#   --log-dir DIR     per-gate logs + summary.tsv (default: $CARGO_TARGET_DIR/run-gates)
#   --list            print the gate names and exit
#
# Exit codes: 0 = every gate ran and passed; 1 = at least one gate red; 2 = bad usage /
# environment; 3 = green but PARTIAL (--only/--skip/--stop-on-fail hid some gates) -- a partial
# run can therefore never be mistaken for the full gate.
#
# Guards -- each one closes a failure class by construction instead of relying on the caller:
#  1. CARGO_TARGET_DIR must be set and ABSOLUTE, and must not be the shared repo-root `target/`
#     (dev-docs/backlog.md rows 98/100; check-attr-spread-collision.sh also builds into it).
#  2. Clippy arms. `cargo clippy --workspace` with no `--features` lints ONLY the
#     `#[cfg(not(feature = "web"))]` arm of primitives (104 `feature = "web"` cfg sites) and none of
#     preview's `feature = "server"` code -- yet both are what ships: `dx build --platform web
#     --ssg --features fullstack` compiles the client with `web` and the prerender server with
#     `server` (-> primitives/web). So the gate lints each arm explicitly:
#        default  -- the not(web)/native arm, every workspace member
#        web      -- primitives + preview with `web` (the browser client's arm)
#        server   -- preview with `server` (SSG prerender server; implies fullstack + primitives/web)
#     `--all-features` is NOT usable instead: preview's `desktop` needs webkit2gtk/gtk (absent in
#     the remote container -- no pkg-config entry) and `web`/`desktop`/`native` are alternative
#     renderers that are not meant to be enabled together; it would also switch `web` on
#     everywhere and so stop linting the not(web) arm. `--all-targets` (plan.md "Definition of
#     done" item 4) is used on every arm, which is a superset of CLAUDE.md's old
#     `--tests --examples` (adds the plain bin and benches), so the two definitions agree.
#  3. A clippy gate is red if its log contains a "generated N warning(s)" summary even when cargo
#     exited 0. `-D warnings` makes this impossible for a fresh lint, but a command run without it
#     (or a future cargo that replays cached diagnostics as warnings) exits 0 with the lint printed.
#     Proven NOT to be the cause of the navbar miss (cargo re-lints after a no-`-D` run and after a
#     `cargo check`), kept because the check is free and makes exit-code-only greens impossible.
#  4. The summary names the tree the gates ran on (HEAD + a hash of the uncommitted diff), so a
#     green cannot be quoted for a commit it was not taken on.
#  5. Any scripts/check-*.sh that is not in the list below fails the run, so a newly added check
#     cannot silently stay out of the gate set. Add it here AND to CLAUDE.md.
#  6. stylelint runs with `npx --no-install`: a worktree without node_modules fails loudly
#     (`npm ci`, never `npm install`, or symlink the main checkout's node_modules -- backlog row 72)
#     instead of letting npx download an unvetted version.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

die() { echo "run-gates: $*" >&2; exit 2; }

only="" skip="" stop_on_fail=0 list=0 log_dir=""
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="${2:?--only needs a REGEX}"; shift 2 ;;
    --skip) skip="${2:?--skip needs a REGEX}"; shift 2 ;;
    --stop-on-fail) stop_on_fail=1; shift ;;
    --log-dir) log_dir="${2:?--log-dir needs a DIR}"; shift 2 ;;
    --list) list=1; shift ;;
    -h|--help) sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

# --- the gate list: CLAUDE.md order. name | command (run with bash -c from the repo root) -------
CLIPPY_FLAGS='--all-targets -- -D warnings'
gates=(
  "check-preview-composition|scripts/check-preview-composition.sh"
  "check-cfg-axis|scripts/check-cfg-axis.sh"
  "check-dx-class-prefix|scripts/check-dx-class-prefix.sh"
  "check-cross-component-overrides|scripts/check-cross-component-overrides.sh"
  "check-css-literals|scripts/check-css-literals.sh"
  "check-hooks-in-closures|scripts/check-hooks-in-closures.sh"
  "check-self-subscribing-effects|scripts/check-self-subscribing-effects.sh"
  "check-css-logical-properties|scripts/check-css-logical-properties.sh"
  "check-attr-spread-collision|scripts/check-attr-spread-collision.sh"
  "check-demo-wrapper-width|scripts/check-demo-wrapper-width.sh"
  "check-playwright-fixture-import|scripts/check-playwright-fixture-import.sh"
  "check-raw-text-interpolation|scripts/check-raw-text-interpolation.sh"
  "check-internal-hrefs|scripts/check-internal-hrefs.sh"
  "check-installed-paths|scripts/check-installed-paths.sh"
  "check-css-vars-defined|scripts/check-css-vars-defined.sh"
  "check-uncleared-intervals|scripts/check-uncleared-intervals.sh"
  "check-blocking-scroll-listeners|scripts/check-blocking-scroll-listeners.sh"
  "check-js-listeners|scripts/check-js-listeners.sh"
  "check-eager-head-document|scripts/check-eager-head-document.sh"
  "check-css-delivery|scripts/check-css-delivery.sh"
  "check-anchored-keyframes|scripts/check-anchored-keyframes.sh"
  "check-infinite-animations|scripts/check-infinite-animations.sh"
  "check-component-catalog|scripts/check-component-catalog.sh"
  "check-demo-forms|scripts/check-demo-forms.sh"
  "check-demo-remote-assets|scripts/check-demo-remote-assets.sh"
  "check-component-fonts|scripts/check-component-fonts.sh"
  "check-registry-url|scripts/check-registry-url.sh"
  "check-motion-gating|scripts/check-motion-gating.sh"
  "fmt|cargo fmt --all -- --check"
  "clippy-default|cargo clippy --workspace $CLIPPY_FLAGS"
  "clippy-web|cargo clippy -p dioxus-primitives -p preview --features dioxus-primitives/web,preview/web $CLIPPY_FLAGS"
  "clippy-server|cargo clippy -p preview --features server $CLIPPY_FLAGS"
  "test|cargo test --workspace"
  "stylelint|cd preview && npx --no-install stylelint \"src/**/*.css\""
)

if [ "$list" = 1 ]; then
  for g in "${gates[@]}"; do printf '%s\t%s\n' "${g%%|*}" "${g#*|}"; done
  exit 0
fi

# --- environment ---------------------------------------------------------------------------
case "${CARGO_TARGET_DIR:-}" in
  "") die "CARGO_TARGET_DIR is not set. Use an ABSOLUTE, isolated dir (scripts/lane-target.sh create <name>); never the shared repo-root target/." ;;
  /*) ;;
  *) die "CARGO_TARGET_DIR must be absolute (dx build --ssg breaks on a relative one, backlog row 98): $CARGO_TARGET_DIR" ;;
esac
[ "${CARGO_TARGET_DIR%/}" != "$repo_root/target" ] || die "CARGO_TARGET_DIR must not be the shared repo-root target/ (backlog row 100): $CARGO_TARGET_DIR"
export CARGO_TARGET_DIR
# Gates are one-shot checks, so the incremental cache is mostly dead weight: on 2026-10-05
# it regrew to 11-13 GB in about an hour of shared gate runs and filled the session's disk
# allowance three times. Default it off; a caller can still opt back in with CARGO_INCREMENTAL=1.
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
log_dir="${log_dir:-$CARGO_TARGET_DIR/run-gates}"
mkdir -p "$log_dir" || die "cannot create log dir $log_dir"
summary_tsv="$log_dir/summary.tsv"
: > "$summary_tsv"

# --- guard 5: every scripts/check-*.sh must be in the list -----------------------------------
unlisted=()
for f in scripts/check-*.sh; do
  [ -e "$f" ] || continue
  listed=0
  for g in "${gates[@]}"; do
    [ "${g#*|}" = "$f" ] && { listed=1; break; }
  done
  [ "$listed" = 1 ] || unlisted+=("$f")
done
if [ "${#unlisted[@]}" -gt 0 ]; then
  echo "run-gates: scripts not in the gate list (add to run-gates.sh AND CLAUDE.md): ${unlisted[*]}" >&2
  exit 2
fi

# --- guard 4: which tree is this green about -----------------------------------------------
head_sha="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
dirty_files="$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
diff_hash="$( { git diff HEAD 2>/dev/null; git ls-files -o --exclude-standard 2>/dev/null | sort | xargs -r sha1sum 2>/dev/null; } | sha1sum | cut -c1-12 )"
echo "run-gates: tree $head_sha, $dirty_files uncommitted path(s), diff-hash $diff_hash"
echo "run-gates: CARGO_TARGET_DIR=$CARGO_TARGET_DIR  logs=$log_dir"
echo "run-gates: $(cargo --version), $(cargo clippy --version)"

now() { date +%s.%N; }

red=() ran=0 hidden=0
for g in "${gates[@]}"; do
  name="${g%%|*}"; cmd="${g#*|}"
  if { [ -n "$only" ] && ! [[ "$name" =~ $only ]]; } || { [ -n "$skip" ] && [[ "$name" =~ $skip ]]; }; then
    hidden=$((hidden + 1)); continue
  fi
  log="$log_dir/$name.log"
  printf '>> %-34s ' "$name"
  t0="$(now)"
  # `nice` keeps the box usable when other lanes compile; the cargo gates are the slow ones.
  nice -n 10 bash -c "$cmd" > "$log" 2>&1
  rc=$?
  t1="$(now)"
  secs="$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.1f", b - a }')"
  status=PASS why=""
  if [ "$rc" -ne 0 ]; then
    status=FAIL why="exit $rc"
  elif [[ "$name" == clippy-* ]] && grep -Eq 'generated [0-9]+ warnings?' "$log"; then
    status=FAIL why="exit 0 but clippy printed warnings (guard 3)"
  fi
  ran=$((ran + 1))
  printf '%s  %7ss  %s\n' "$status" "$secs" "$why"
  printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$status" "$rc" "$secs" "$log" >> "$summary_tsv"
  if [ "$status" = FAIL ]; then
    red+=("$name")
    echo "   --- last 25 lines of $log"; tail -n 25 "$log" | sed 's/^/   | /'
    if [ "$stop_on_fail" = 1 ]; then hidden=$((hidden + 1)); echo "run-gates: --stop-on-fail"; break; fi
  fi
done

echo
echo "================ run-gates summary (tree $head_sha, diff-hash $diff_hash) ================"
awk -F'\t' '{ printf "%-34s %-5s rc=%-3s %8ss  %s\n", $1, $2, $3, $4, $5 }' "$summary_tsv"
total="$(awk -F'\t' '{ s += $4 } END { printf "%.1f", s }' "$summary_tsv")"
echo "gates run: $ran   red: ${#red[@]}   total: ${total}s"
if [ "${#red[@]}" -gt 0 ]; then
  echo "RED: ${red[*]}"
  exit 1
fi
if [ "$hidden" -gt 0 ] || [ "$ran" -ne "${#gates[@]}" ]; then
  echo "PARTIAL: $ran of ${#gates[@]} gates ran, all green -- this is NOT the full gate"
  exit 3
fi
echo "ALL GREEN: all ${#gates[@]} gates passed on $head_sha (diff-hash $diff_hash)"
