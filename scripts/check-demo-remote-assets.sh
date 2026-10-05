#!/usr/bin/env bash
#
# check-demo-remote-assets.sh
#
# No demo may hot-link a remote image. Every image the preview shows is either bundled
# (`asset!(..)`, e.g. preview/assets/avatars) or self-contained (a `data:` URI, a CSS gradient), so each
# demo and docs page renders the same offline, in a sandbox with an egress proxy, and on GitHub Pages --
# deterministically, and without sending a visitor's request to a third party.
#
# THE DEFECT: the avatar, item, drag-and-drop and home-page demos used GitHub profile photos of real people
# (`github.com/<user>.png`, `avatars.githubusercontent.com/u/<id>`) and `avatar.vercel.sh/<name>` gradients.
# That is a privacy and licence problem (real people's photos), and a reliability one: with the network
# blocked or slow every one of them fell back to initials (or sat on a broken-image box), and a spec that
# reads `img[src]` or waits for `load` depends on a third party. Found three times over (avatar, item,
# drag_and_drop_list + the home blocks), so this is a class and not one bad URL: the gate makes the class
# unable to come back.
#
# WHAT IT FLAGS (a remote URL is `http://`, `https://` or protocol-relative `//host`), in .rs/.css/.html under
# the scanned roots:
#   1. an `src` / `srcset` / `poster` attribute whose value is a remote URL literal (`src: "https://..."`,
#      `src: format!("https://...")`, `src="https://..."`);
#   2. a CSS `url(...)` with a remote URL (stylesheets and inline `style:` strings alike);
#   3. a `const` / `static` string with a remote URL whose name says it is an image (`..._SRC`, `AVATAR`,
#      `IMAGE`, `PHOTO`, `COVER`, `LOGO`, ...), because `src: SOME_CONST` hides the literal from rule 1;
#   4. any string literal that is a remote URL ending in an image extension, or on a well-known image/avatar
#      host (githubusercontent, avatar.vercel.sh, gravatar, unsplash, picsum, placehold, dicebear, ...).
# Not flagged: `data:` URIs (they only *contain* `http://www.w3.org/2000/svg`, never start with it), links
# (`href:` to docs or GitHub), fonts, and comments. `.md` is not scanned: a docs page may show
# `src: "https://example.com/avatar.png"` as illustrative code, which renders nothing.
#
# THE ALLOWLIST below is only for demos whose whole point is a request that cannot succeed or must stay
# pending. Each entry is `<file suffix>|<exact URL>` plus a comment saying why, so the exemption cannot be
# reused by another file, and an entry that no longer matches anything FAILS the run (the list stays honest).
#
# If you are here because it failed: bundle the image under preview/assets and use `asset!("/assets/...")`,
# or inline it (a `data:` SVG, a `linear-gradient`). Do not add an allowlist entry for a demo that merely
# wants a nicer picture.
#
# Usage: scripts/check-demo-remote-assets.sh [ROOT ...]   (default: preview/src preview/assets; other roots are for
# self-tests and skip the stale-allowlist check)
# Exit 0: clean. Exit 1: a violation, listed on stderr. Exit 2: bad usage.

set -o pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

ALLOW=(
  # avatar "Error State": the image must fail to load so the initials fallback shows. `.example` is
  # reserved (RFC 2606) and never resolves, so it fails the same way online and offline.
  "components/avatar/variants/main/mod.rs|https://invalid-url.example/image.jpg"
  # avatar "Loading": the primitive's `loading` state is only reached while an `<img>` request is in
  # flight, and any local src settles at once (loaded or error), so this demo needs a request that stays
  # pending. Offline or behind a blocking proxy it settles as `error` and shows the fallback instead.
  "components/avatar/variants/main/mod.rs|https://httpbin.org/delay/3600"
)

if [ "$#" -gt 0 ]; then ROOTS=("$@"); CHECK_STALE=0; else ROOTS=(preview/src preview/assets); CHECK_STALE=1; fi
for r in "${ROOTS[@]}"; do
  [ -d "$r" ] || { echo "check-demo-remote-assets: $r is not a directory" >&2; exit 2; }
done

python3 - "$CHECK_STALE" "${ALLOW[@]}" -- "${ROOTS[@]}" <<'PY'
import os, re, sys

check_stale = sys.argv[1] == "1"  # only for the default roots: a self-test root has no avatar demo to match
args = sys.argv[2:]
sep = args.index("--")
allow = [tuple(a.split("|", 1)) for a in args[:sep]]
roots = args[sep + 1:]

REMOTE = r'(?:https?:)?//[^\s"\'`)<>\\]+'
IMG_EXT = r'(?:png|jpe?g|gif|webp|avif|svg|ico|bmp|apng)'
IMG_HOSTS = (r'(?:avatars\.githubusercontent\.com|raw\.githubusercontent\.com|user-images\.githubusercontent\.com|'
             r'avatar\.vercel\.sh|(?:www\.)?gravatar\.com|(?:i\.)?pravatar\.cc|picsum\.photos|(?:images|source)\.unsplash\.com|'
             r'placehold\.(?:co|it)|via\.placeholder\.com|randomuser\.me|ui-avatars\.com|api\.dicebear\.com|loremflickr\.com)')
IMAGE_NAME = re.compile(r'SRC|IMG|IMAGE|AVATAR|PHOTO|PICTURE|COVER|POSTER|LOGO|ICON|THUMB|BANNER|BACKGROUND')

rules = [
    ("src/srcset/poster attribute", re.compile(r'\b(?:src|srcset|poster)\b\s*[:=]\s*(?:format!\(\s*)?r?#*"(' + REMOTE + ')')),
    ("CSS url()", re.compile(r'url\(\s*[\'"]?(' + REMOTE + ')')),
    ("image-named const/static", re.compile(r'\b(?:const|static)\s+([A-Z0-9_]+)\s*:[^=;]*=\s*r?#*"(https?://[^"]+)"')),
    ("remote image file", re.compile(r'"(https?://[^"\s]*\.' + IMG_EXT + r'(?:[?#][^"\s]*)?)"', re.I)),
    ("known image/avatar host", re.compile(r'"(https?://' + IMG_HOSTS + r'[^"\s]*)"', re.I)),
]

def strip_css_comments(text):
    return re.sub(r'/\*.*?\*/', lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)

found = {}
for root in roots:
    for dirpath, _, files in os.walk(root):
        for name in files:
            if not name.endswith((".rs", ".css", ".html")):
                continue
            path = os.path.join(dirpath, name)
            text = open(path, encoding="utf-8", errors="replace").read()
            if name.endswith((".css", ".html")):
                text = strip_css_comments(text)
            for n, line in enumerate(text.splitlines(), 1):
                if line.lstrip().startswith(("//", "*", "/*")):
                    continue  # a comment may quote a URL; that is not a request
                for label, rx in rules:
                    for m in rx.finditer(line):
                        if label == "image-named const/static":
                            if not IMAGE_NAME.search(m.group(1)):
                                continue
                            url = m.group(2)
                        else:
                            url = m.group(1)
                        if "w3.org" in url:
                            continue
                        found.setdefault((path, n, url), label)

used = set()
bad = []
for (path, n, url), label in sorted(found.items()):
    hit = next((a for a in allow if path.endswith(a[0]) and url == a[1]), None)
    if hit:
        used.add(hit)
    else:
        bad.append(f"{path}:{n}: remote image {url} ({label})")

stale = [a for a in allow if a not in used] if check_stale else []
for line in bad:
    print("check-demo-remote-assets: " + line, file=sys.stderr)
for path, url in stale:
    print(f"check-demo-remote-assets: stale allowlist entry (no longer matches anything): {path}|{url}", file=sys.stderr)
if bad or stale:
    print(f"check-demo-remote-assets: FAILED -- {len(bad)} remote image reference(s), {len(stale)} stale allowlist entr{'y' if len(stale) == 1 else 'ies'}. "
          "Bundle the image (asset!) or inline it (data: URI / CSS gradient); see this script's header.", file=sys.stderr)
    sys.exit(1)
print(f"check-demo-remote-assets: OK -- no remote image in {', '.join(roots)} ({len(allow)} allowlisted deliberate demo URL(s) still in use).")
PY
