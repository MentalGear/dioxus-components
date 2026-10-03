#!/usr/bin/env bash
# Isolated, warm CARGO_TARGET_DIRs for lanes (backlog row 116 step 3; rows 98/100).
#
#   scripts/lane-target.sh create <name> [--template DIR] [--root DIR] [--force]
#   scripts/lane-target.sh prune  [--root DIR] [--older-than DAYS] [--apply]
#   scripts/lane-target.sh du     [--root DIR]
#
# `create` copies a warm template (cp -a --reflink=auto) to <root>/<name> and prints
# `export CARGO_TARGET_DIR=<abs path>` -- use `eval "$(scripts/lane-target.sh create x)"`
# (progress goes to stderr, only the export line to stdout). The path is always
# absolute (dx --ssg breaks on relative ones, row 98) and never the shared repo `target/`.
#
# The template must be base-path-free (row 100: a tree that ever held a
# `--base-path dioxus-components` build keeps emitting prefixed URLs even after
# `rm -rf public`). Two checks, because neither alone is enough:
#   1. marker `<template>/.base-path-build`, written by scripts/deploy-preview.sh --
#      instant, and catches a tree whose `public/` was already wiped;
#   2. grep of the template's dx `public/` HTML for quoted `/dioxus-components/` URLs --
#      authoritative on the actual output, catches templates made by hand.
# Defaults: root $LANE_TARGET_ROOT or /home/user/tgt, template $LANE_TARGET_TEMPLATE or <root>/template.
set -euo pipefail

root="${LANE_TARGET_ROOT:-/home/user/tgt}"
template="${LANE_TARGET_TEMPLATE:-}"
older_than=2 apply=0 force=0

die() { echo "lane-target: $*" >&2; exit 1; }
usage() { sed -n '2,8p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2; exit 2; }

cmd="${1:-}"; [ -n "$cmd" ] || usage; shift
name=""
if [ "$cmd" = create ]; then name="${1:-}"; [ -n "$name" ] || usage; shift; fi
while [ $# -gt 0 ]; do
  case "$1" in
    --root) root="${2:?}"; shift 2 ;;
    --template) template="${2:?}"; shift 2 ;;
    --older-than) older_than="${2:?}"; shift 2 ;;
    --apply) apply=1; shift ;;
    --force) force=1; shift ;;
    *) usage ;;
  esac
done
root="${root%/}"; template="${template:-$root/template}"; template="${template%/}"
case "$root" in /*) ;; *) die "root must be absolute: $root" ;; esac

contaminated() { # $1 = tree; prints why and returns 0 if it holds a base-path build
  [ -e "$1/.base-path-build" ] && { echo "marker $1/.base-path-build"; return 0; }
  local d
  for d in "$1"/dx/*/*/web/public; do
    [ -d "$d" ] || continue
    grep -rqE --include='*.html' "[\"'(]/dioxus-components/" "$d" && { echo "base-path URLs in $d"; return 0; }
  done
  return 1
}

case "$cmd" in
  create)
    [[ "$name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || die "bad lane name '$name'"
    [ "$name" != template ] || die "'template' is reserved"
    [ -d "$template" ] || die "template not found: $template (build one: CARGO_TARGET_DIR=$template dx build ... without --base-path)"
    why="$(contaminated "$template")" && die "refusing template $template: $why (base-path build, backlog row 100). Build a clean template."
    dest="$root/$name"
    [ ! -e "$dest" ] || die "$dest already exists (prune it or pick another name)"
    need_kb="$(du -sk "$template" | cut -f1)"; free_kb="$(df -Pk "$root" | awk 'NR==2{print $4}')"
    if [ "$force" -ne 1 ] && [ "$free_kb" -lt "$need_kb" ]; then
      die "only $((free_kb/1024)) MiB free under $root, template is $((need_kb/1024)) MiB (reflink may not apply; --force to try)"
    fi
    echo "lane-target: copying $template -> $dest ($((need_kb/1024)) MiB) ..." >&2
    mkdir -p "$root"; cp -a --reflink=auto "$template" "$dest"
    rm -rf "$dest"/dx/*/*/web/public   # never inherit another build's served output
    echo "export CARGO_TARGET_DIR=$dest"
    ;;
  du)
    [ -d "$root" ] || die "no such root: $root"
    du -sh "$root"/*/ 2>/dev/null | sort -h; du -sh "$root"; df -h "$root" | tail -1
    ;;
  prune)
    [ -d "$root" ] || die "no such root: $root"
    echo "lane dirs under $root idle > ${older_than}d ($([ "$apply" -eq 1 ] && echo REMOVING || echo "dry run, pass --apply")); the template is never pruned:"
    for d in "$root"/*/; do
      d="${d%/}"; [ -d "$d" ] || continue; [ "$d" != "$template" ] || continue
      # "Active" = any file touched within N days (depth-bounded, first hit wins).
      [ -z "$(find "$d" -maxdepth 4 -type f -mtime "-$older_than" -print -quit 2>/dev/null)" ] || continue
      printf '  %s  %s\n' "$(du -sh "$d" | cut -f1)" "$d"
      [ "$apply" -ne 1 ] || rm -rf "$d"
    done
    df -h "$root" | tail -1
    ;;
  *) usage ;;
esac
