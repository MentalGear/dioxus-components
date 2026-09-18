#!/usr/bin/env bash
#
# dev-wait.sh — block until a `dx serve` dev server has actually finished
# processing your most recent source edit, then exit 0 (or a distinct
# non-zero code for a known-ambiguous case). Built to replace "curl until it
# returns 200" and "eyeball the terminal", both of which are proven unsafe
# by dev-docs/dx-serve-hot-reload.md's three traps:
#
#   1. A bare HTTP 200 during a build only proves the process is alive --
#      this sandbox's dev server was empirically observed serving a real
#      HTTP 200 whose BODY reads "Err 404 - dioxus is not currently serving
#      a web app" while the first cold build was still in flight.
#   2. There are two different completion strings and they mean different
#      things: a COLD START logs "Build completed successfully in Ns"; a
#      WATCH REBUILD (the normal path after a file change) logs
#      "Build completed in Ns" -- no "successfully". Grepping the cold-start
#      string after an ordinary edit finds nothing even when the rebuild
#      genuinely finished.
#   3. A structural Rust *logic* change can make dx log only
#      "Hotreloading: <file>" -- with NO "Build completed" line at all --
#      and silently not take effect (dx misjudged the change as
#      hot-reloadable). No error is printed; the terminal just looks like
#      progress. See dev-docs/dx-serve-hot-reload.md for the full account
#      and dev-docs/dev-loop.md for how this script encodes all three.
#
# This script only reads `dx serve`'s own stdout/stderr log -- it never
# polls HTTP. Start your server with its output redirected to a file first:
#
#   dx serve --web --port 8083 > /tmp/dx-serve-8083.log 2>&1 &
#
# USAGE
#   scripts/dev-wait.sh <port> [options]
#
# OPTIONS
#   --log <path>       Path to the server's redirected log.
#                       Default: /tmp/dx-serve-<port>.log
#   --since-line <N>   Only consider log lines strictly after line N. Use
#                       this to avoid the save-then-call race on a very fast
#                       hot-reload: capture N yourself with
#                       `wc -l < "$LOG"` right BEFORE saving your edit, then
#                       pass it here after saving. Default: the log's line
#                       count at invocation time (fine for edits that take
#                       at least ~1s to compile; for a template/CSS edit
#                       that can hot-reload in tens of ms, prefer an
#                       explicit --since-line).
#   --timeout <secs>   Give up after this many seconds. Default: 60.
#   --expect <kind>    rebuild | hotreload | any (default: any).
#                       "rebuild" FAILS (exit 3) if the only completion
#                       observed is a hot-reload-only one with no
#                       "Build completed" line -- use this after a Rust
#                       LOGIC edit (a handler body, a new fn/module), where
#                       trap 3 above means that pattern can mean the change
#                       silently did NOT take effect. Do not use "rebuild"
#                       for a pure RSX text/attribute or CSS edit: those are
#                       CORRECTLY hot-reload-only and would always "fail".
#   -q, --quiet         Only print the final machine-readable result line.
#   -h, --help          Show this help.
#
# OUTPUT (stdout, on success)
#   READY <kind> <elapsed_s>s: <detail>
#     kind: cold | rebuild | hotreload
#
# EXIT CODES
#   0  ready (kind matches --expect, or --expect was "any")
#   1  timeout -- no build activity at all matched. The watcher may not
#      have picked up the change (wrong glob, or a change landed inside its
#      debounce window) -- see "When you're not sure" in
#      dev-docs/dx-serve-hot-reload.md.
#   2  usage error
#   3  ambiguous: only "Hotreloading:" was observed (trap 3) and
#      --expect rebuild was given, so this does NOT count as confirmation
#      the logic change took effect. Restart the server for an unambiguous
#      full rebuild.
set -euo pipefail

port=""
log=""
since_line=""
timeout_secs=60
expect="any"
quiet=0

while [ $# -gt 0 ]; do
  case "$1" in
    --log) log="$2"; shift 2 ;;
    --since-line) since_line="$2"; shift 2 ;;
    --timeout) timeout_secs="$2"; shift 2 ;;
    --expect) expect="$2"; shift 2 ;;
    -q|--quiet) quiet=1; shift ;;
    -h|--help) sed -n '2,60p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "dev-wait.sh: unknown option: $1" >&2; exit 2 ;;
    *)
      if [ -z "$port" ]; then port="$1"; shift; else
        echo "dev-wait.sh: unexpected extra argument: $1" >&2; exit 2
      fi
      ;;
  esac
done

if [ -z "$port" ]; then
  echo "dev-wait.sh: missing <port>. See --help." >&2
  exit 2
fi
case "$expect" in rebuild|hotreload|any) ;; *)
  echo "dev-wait.sh: --expect must be rebuild|hotreload|any, got: $expect" >&2
  exit 2 ;;
esac

log="${log:-/tmp/dx-serve-${port}.log}"
if [ ! -f "$log" ]; then
  echo "dev-wait.sh: log file not found: $log" >&2
  echo "  Start the server with its output redirected there first, e.g.:" >&2
  echo "    dx serve --web --port $port > $log 2>&1 &" >&2
  exit 2
fi

total_lines=$(wc -l < "$log" | tr -d ' ')
since_line="${since_line:-$total_lines}"

[ "$quiet" -eq 1 ] || echo "dev-wait.sh: watching $log from line $((since_line + 1)), timeout ${timeout_secs}s, expect=$expect" >&2

deadline=$(( $(date +%s) + timeout_secs ))
last_scanned="$since_line"
hotreload_seen=""
hotreload_line=""
hotreload_at=0
settle_secs=1        # quiet period after a lone "Hotreloading:" before
                      # treating it as the final word for this edit
start_ts=$(date +%s.%N 2>/dev/null || date +%s)

while :; do
  now_lines=$(wc -l < "$log" | tr -d ' ')
  if [ "$now_lines" -gt "$last_scanned" ]; then
    new_text=$(sed -n "$((last_scanned + 1)),${now_lines}p" "$log")
    last_scanned="$now_lines"

    if line=$(printf '%s\n' "$new_text" | grep -m1 -F 'Build completed successfully in'); then
      elapsed=$(awk -v s="$start_ts" 'BEGIN{cmd="date +%s.%N"; cmd|getline n; close(cmd); printf "%.1f", n-s}' 2>/dev/null || echo "?")
      echo "READY cold ${elapsed}s: $(printf '%s' "$line" | sed 's/^ *//')"
      exit 0
    fi
    if line=$(printf '%s\n' "$new_text" | grep -m1 -F 'Build completed in' | grep -v -F successfully); then
      elapsed=$(awk -v s="$start_ts" 'BEGIN{cmd="date +%s.%N"; cmd|getline n; close(cmd); printf "%.1f", n-s}' 2>/dev/null || echo "?")
      echo "READY rebuild ${elapsed}s: $(printf '%s' "$line" | sed 's/^ *//')"
      exit 0
    fi
    if line=$(printf '%s\n' "$new_text" | grep -m1 -F 'Hotreloading:'); then
      hotreload_seen=1
      hotreload_line="$line"
      hotreload_at=$(date +%s)
    fi
  fi

  if [ -n "$hotreload_seen" ] && [ $(( $(date +%s) - hotreload_at )) -ge "$settle_secs" ]; then
    elapsed=$(awk -v s="$start_ts" 'BEGIN{cmd="date +%s.%N"; cmd|getline n; close(cmd); printf "%.1f", n-s}' 2>/dev/null || echo "?")
    if [ "$expect" = "rebuild" ]; then
      echo "AMBIGUOUS hotreload-only ${elapsed}s: $(printf '%s' "$hotreload_line" | sed 's/^ *//')" >&2
      echo "  Only 'Hotreloading:' was observed, no 'Build completed' line -- this is" >&2
      echo "  trap 3 (dev-docs/dx-serve-hot-reload.md): for a Rust LOGIC edit this can" >&2
      echo "  mean the change did NOT take effect. Restart the server to force an" >&2
      echo "  unambiguous full rebuild, then re-check." >&2
      exit 3
    fi
    echo "READY hotreload ${elapsed}s: $(printf '%s' "$hotreload_line" | sed 's/^ *//')"
    exit 0
  fi

  if [ "$(date +%s)" -ge "$deadline" ]; then
    echo "TIMEOUT after ${timeout_secs}s: no build activity matched in $log since line $((since_line + 1))." >&2
    echo "  The watcher may not have picked up the change (wrong glob, or the edit" >&2
    echo "  landed inside its debounce window) -- see 'When you're not sure' in" >&2
    echo "  dev-docs/dx-serve-hot-reload.md." >&2
    exit 1
  fi
  sleep 0.2
done
