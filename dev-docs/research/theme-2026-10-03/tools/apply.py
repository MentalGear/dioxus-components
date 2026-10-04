"""Row 111 phase A, REPOINT lane: apply every REPOINT site in place, preserving formatting.

    python3 dev-docs/research/theme-2026-10-03/tools/apply.py            # apply (idempotent: a second run finds nothing)
    python3 dev-docs/research/theme-2026-10-03/tools/apply.py --check    # verify the working tree against git HEAD

Method: the same analysis gen.py uses (engine.analyse), but instead of looking at a whitespace-normalised
value it runs on the raw declaration value and patches only the `var(...)` spans it decided to replace,
so multi-line declarations, comments and everything outside a REPOINT span stay byte-identical.
Comments are blanked to spaces (same length) before parsing, so offsets map straight onto the original.

Also deletes sidebar's six local `--dx-sidebar-*` definitions (the DELETE-local-def rows).
`--check` re-parses every touched file at HEAD and in the working tree, requires the same declaration
skeleton (selector, property, order) and that each declaration resolves to the identical value in light and
dark under the current theme, and cross-checks the per-file REPOINT count against sites.tsv.
"""
import os, re, subprocess, sys, collections, csv
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../../../.."))
sys.path.insert(0, HERE)
import engine
from engine import analyse, resolve, T, REFTOK
from extract import REF, strip_comments, parse2

TSV = os.path.join(HERE, '..', 'sites.tsv')
norm = lambda x: re.sub(r'\s*([,()])\s*', r'\1', x)


def tracked():
    css = subprocess.check_output(['git', 'ls-files', 'preview/src/**.css', 'preview/assets/*.css'], cwd=ROOT, text=True).split()
    css = [f for f in css if 'dx-utilities' not in f and f != 'preview/assets/dx-components-theme.css']
    rs = subprocess.check_output("git ls-files '*.rs' | xargs grep -lE -e '--(primary|secondary|contrast)-[a-z0-9-]*color|--focused-border-color'",
                                 shell=True, cwd=ROOT, text=True).split()
    rs = [f for f in rs if not f.startswith('primitives/')]
    return css, rs


# ---------------------------------------------------------------- CSS
def css_decls(src):
    """yield (selector, prop, val_start, val_end) for every declaration; src is comment-stripped."""
    stack = []; buf_start = 0; depth = 0; q = None
    for i, ch in enumerate(src):
        if q:
            if ch == q: q = None
            continue
        if ch in '"\'': q = ch
        elif ch == '(': depth += 1
        elif ch == ')': depth -= 1
        elif ch == '{' and depth == 0:
            stack.append(' '.join(src[buf_start:i].split())); buf_start = i + 1
        elif ch in ';}' and depth == 0:
            d = src[buf_start:i]
            if ':' in d:
                c = d.index(':')
                yield ' | '.join(stack), d[:c].strip(), buf_start + c + 1, i
            buf_start = i + 1
            if ch == '}' and stack: stack.pop()


def apply_css(f):
    path = ROOT + '/' + f
    orig = open(path).read()
    src = strip_comments(orig)
    assert len(src) == len(orig)
    edits = []; nrefs = 0; deletes = []
    for sel, prop, vs, ve in css_decls(src):
        val = src[vs:ve]
        if not REF.search(val): continue
        if prop.startswith('--'):
            deletes.append((sel, prop, vs, ve)); continue
        nv, refs = analyse(val, prop, sel, f)
        nrefs += sum(1 for r in refs if r['role'])
        for s, e, t in analyse.edits:
            edits.append((vs + s, vs + e, t))
    out = orig
    for s, e, t in sorted(edits, reverse=True):
        out = out[:s] + t + out[e:]
    # sidebar: delete the local --dx-sidebar-* declarations (whole line when the declaration owns it)
    ndel = 0
    for sel, prop, vs, ve in sorted(deletes, key=lambda x: -x[2]):
        assert prop.startswith('--dx-sidebar-') and f.endswith('sidebar/style.css'), (f, prop)
        # offsets are pre-edit; the deleted decls contain no repoint edits, and edits all sit at other offsets
        # but earlier edits shift later ones: recompute by locating in `out` via the declaration text
        decl = orig[src.rfind('\n', 0, vs) + 1: ve + 1]  # "  --dx-x: var(...);" up to and including ';'
        m = re.compile(r'^[ \t]*' + re.escape(decl.strip()) + r'[ \t]*\n', re.M).search(out)
        assert m and decl.strip() in out, (f, decl)
        out = out[:m.start()] + out[m.end():]; ndel += 1
    if ndel:  # the six defs were followed by a separator blank line; do not leave `{` + blank behind
        out = re.sub(r'(\.dx-sidebar-wrapper \{\n)[ \t]*\n', r'\1', out, count=1)
    if out != orig:
        open(path, 'w').write(out)
    return nrefs, ndel


# ---------------------------------------------------------------- Rust inline style strings
def rust_line_edits(line, f, i):
    """Return [(start,end,text)] on this line; mirrors gen.py's line scanner (same props, same value spans)."""
    edits = []; refs_n = 0; found = False; taken = []
    for m in re.finditer(r'(?<![\w-])([a-z][a-z-]*)\s*:\s*', line):
        prop = m.group(1)
        if prop == 'style': continue
        j = m.end(); d = 0; k = j
        while k < len(line):
            c = line[k]
            if c == '(': d += 1
            elif c == ')': d -= 1
            elif d == 0 and c in ';"': break
            k += 1
        val = line[j:k]
        if not re.search(REFTOK, val): continue
        sel = '(inline style / Rust string) ' + f.split('/')[-2]
        nv, refs = analyse(val, prop, sel, f)
        if refs: found = True
        refs_n += sum(1 for r in refs if r['role'])
        edits += [(j + s, j + e, t) for s, e, t in analyse.edits]
    if not found:
        pm = re.search(r'(?<![\w-])([a-z][a-z-]*)\s*:', line)
        for qm in re.finditer(r'"([^"]*)"', line):
            q = qm.group(1)
            if not re.search(REFTOK, q): continue
            nv, refs = analyse(q, pm.group(1), '(inline style / Rust string)', f)
            if refs: found = True
            refs_n += sum(1 for r in refs if r['role'])
            edits += [(qm.start(1) + s, qm.start(1) + e, t) for s, e, t in analyse.edits]
    return edits, refs_n, found


def apply_rs(f):
    path = ROOT + '/' + f
    lines = open(path).read().split('\n')
    nrefs = 0; changed = False
    for idx, line in enumerate(lines):
        st = line.strip()
        if st.startswith('//') or st.startswith('*') or st.startswith('/*'): continue
        if not re.search(REFTOK, line): continue
        edits, n, found = rust_line_edits(line, f, idx + 1)
        assert found, (f, idx + 1, line)
        nrefs += n
        for s, e, t in sorted(set(edits), reverse=True):  # set(): the two scans may both see a span
            line = line[:s] + t + line[e:]
        if line != lines[idx]:
            lines[idx] = line; changed = True
    if changed:
        open(path, 'w').write('\n'.join(lines))
    return nrefs


# ---------------------------------------------------------------- --check
def head(f):
    return subprocess.check_output(['git', 'show', 'HEAD:' + f], cwd=ROOT, text=True)


def check(css, rs):
    bad = []
    for f in css:
        a = head(f); b = open(ROOT + '/' + f).read()
        if a == b: continue
        sa = list(css_decls(strip_comments(a))); sb = list(css_decls(strip_comments(b)))
        A = strip_comments(a); B = strip_comments(b)
        da = [(s, p, A[vs:ve]) for s, p, vs, ve in sa]; db = [(s, p, B[vs:ve]) for s, p, vs, ve in sb]
        da = [x for x in da if not x[1].startswith('--dx-sidebar-')]
        db = [x for x in db if not x[1].startswith('--dx-sidebar-')]
        if len(da) != len(db): bad.append((f, 'decl count', len(da), len(db))); continue
        for x, y in zip(da, db):
            if x[:2] != y[:2]: bad.append((f, 'skeleton', x[:2], y[:2])); continue
            for m in ('light', 'dark'):
                if norm(resolve(x[2], T, m)) != norm(resolve(y[2], T, m)): bad.append((f, m, x, y))
        # everything outside value spans must be identical: blank the value spans of changed decls and compare
        def skel(s, src, spans):
            out = s
            for _, _, vs, ve in sorted(spans, key=lambda t: -t[2]):
                if REF.search(src[vs:ve]) or '--dx-' in src[vs:ve]: out = out[:vs] + out[ve:]
            return out
        ka = skel(a, A, [t for t in sa if not t[1].startswith('--dx-sidebar-')])
        kb = skel(b, B, [t for t in sb if not t[1].startswith('--dx-sidebar-')])
        # sidebar deletions remove whole lines; drop them from the skeleton before comparing
        ka = re.sub(r'^[ \t]*--dx-sidebar-[a-z-]+:[^\n]*\n', '', ka, flags=re.M)
        kb = re.sub(r'^[ \t]*--dx-sidebar-[a-z-]+:[^\n]*\n', '', kb, flags=re.M)
        ka = re.sub(r'(\.dx-sidebar-wrapper \{\n)[ \t]*\n', r'\1', ka, count=1)
        if ka != kb: bad.append((f, 'text outside repointed values changed'))
    for f in rs:
        a = head(f).split('\n'); b = open(ROOT + '/' + f).read().split('\n')
        if len(a) != len(b): bad.append((f, 'line count', len(a), len(b))); continue
        for n, (x, y) in enumerate(zip(a, b), 1):
            if x == y: continue
            # same line with every var()-span that mentions a role token blanked must match
            strip = lambda s: re.sub(r'V(?: V)+', 'V', re.sub(r'var\((?:[^()]|\([^()]*\))*\)', 'V', s))  # a collapsed light/dark pair is 2 spans -> 1
            if strip(x) != strip(y): bad.append((f, n, 'non-var text changed', x.strip(), y.strip())); continue
            for m in ('light', 'dark'):
                vx = re.findall(r'var\((?:[^()]|\([^()]*\))*\)', x); vy = re.findall(r'var\((?:[^()]|\([^()]*\))*\)', y)
                if norm(resolve(' '.join(vx), T, m)) != norm(resolve(' '.join(vy), T, m)):
                    bad.append((f, n, m, x.strip(), y.strip()))
    # TSV cross-check
    want = collections.Counter(); wantdel = 0
    for r in csv.DictReader(open(TSV), delimiter='\t', quoting=csv.QUOTE_NONE):
        if r['action'] == 'REPOINT': want[r['file']] += 1
        if r['action'] == 'DELETE-local-def': wantdel += 1
    return bad, want, wantdel


def main():
    css, rs = tracked()
    if '--check' in sys.argv:
        bad, want, wantdel = check(css, rs)
        for b in bad[:40]: print('BAD', b)
        print('check problems:', len(bad))
        sys.exit(1 if bad else 0)
    got = collections.Counter(); dels = 0
    for f in css:
        n, d = apply_css(f)
        if n: got[f] += n
        dels += d
    for f in rs:
        n = apply_rs(f)
        if n: got[f] += n
    want = collections.Counter(); wantdel = 0
    for r in csv.DictReader(open(TSV), delimiter='\t', quoting=csv.QUOTE_NONE):
        if r['action'] == 'REPOINT': want[r['file']] += 1
        if r['action'] == 'DELETE-local-def': wantdel += 1
    print('applied refs', sum(got.values()), 'TSV REPOINT rows', sum(want.values()), '| deleted defs', dels, 'of', wantdel)
    # a re-run applies nothing, so the per-file comparison is only meaningful on the first run
    if sum(got.values()):
        diff = {f: (got[f], want[f]) for f in set(got) | set(want) if got[f] != want[f]}
        print('per-file mismatches vs TSV:', diff or 'none')
    sys.exit(0)


main()
