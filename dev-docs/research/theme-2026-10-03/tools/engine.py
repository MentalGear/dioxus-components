import re, sys, json, collections
import os,sys
HERE=os.path.dirname(os.path.abspath(__file__))
ROOT=os.path.abspath(os.path.join(HERE,"../../../.."))
sys.path.insert(0,HERE)
from cssvar import *
THEME=os.path.join(ROOT,'preview/assets/dx-components-theme.css')
T=parse_theme(THEME)
REFTOK=r'--(?:(?:primary|secondary)-color(?:-\d+)?|(?:primary|secondary|contrast)-(?:success|warning|error|info)-color|focused-border-color)'
PLAIN=re.compile(r'^var\((%s)\)$'%REFTOK)
def pairhex(tok):
    e='var(%s)'%tok
    return resolve(e,T,'light'),resolve(e,T,'dark')

STATE=re.compile(r':hover|:focus|highlighted|data-state="(?:on|open|active)"|aria-current|data-selected|data-active|:checked|data-selection|data-today|data-open')
def pc_of(prop,tok):
    p=prop.lower()
    if p=='color': return 'text'
    if p.startswith('background'): return 'surface'
    if p.startswith('border') or p.startswith('outline') or p=='box-shadow': return 'line'
    if p=='fill': return 'text' if tok.startswith('--secondary') or tok=='--contrast-error-color' else 'surface'
    if p=='stroke': return 'text' if tok.startswith('--secondary') else 'line'
    if p=='filter': return 'surface'
    return 'other'
# (pc,(L,D)) -> role | callable(sel)->role
def s1text(sel):
    if 'data-style="secondary"' in sel: return 'secondary-foreground'
    if STATE.search(sel): return 'accent-foreground'
    return 'foreground-alt-2'
def mutedsurf(sel):
    return 'secondary' if 'data-style="secondary"' in sel else 'muted'
RP={
 ('text',('#707070','#a1a1a1')):'muted-foreground',
 ('text',('#111','#d4d4d4')):'foreground',
 ('text',('#2b2b2b','#dcdcdc')):'foreground-alt',
 ('text',('#000','#fafafa')):s1text,
 ('text',('#fff','#000')):'primary-foreground',
 ('text',('#dc2626','#a22e2e')):'destructive',
 ('text',('#fff','#dcdcdc')):'destructive-foreground',
 ('text',('#10b981','#b6fae3')):'success',
 ('text',('#f59e0b','#feeac7')):'warning',
 ('text',('#2b2b2b','#3e3e3e')):'info',
 ('surface',('#fff','#000')):'background',
 ('surface',('#fff','#0a0a0a')):'background-alt',
 ('surface',('#fff','#141313')):'card',
 ('surface',('#fbfbfb','#0e0e0e')):'card-alt',
 ('surface',('#fff','#262626')):'popover',
 ('surface',('#f5f5f5','#262626')):mutedsurf,
 ('surface',('#f8f8f8','#141313')):'muted-alt',
 ('surface',('#f8f8f8','#3e3e3e')):'accent',
 ('surface',('#f8f8f8','#1a1a1a')):'accent-alt',
 ('surface',('#000','#fafafa')):'primary',
 ('surface',('#0d0d0d','#e6e6e6')):'primary-alt',
 ('surface',('#dc2626','#a22e2e')):'destructive',
 ('surface',('#ef4444','#9b1c1c')):'destructive-alt',
 ('surface',('#2b7fff','#2b7fff')):'ring-color',
 ('surface',('#ecfdf5','#02271c')):'success-subtle',
 ('surface',('#fffbeb','#342203')):'warning-subtle',
 ('line',('#e5e5e5','#232323')):'border',
 ('line',('#e5e5e5','#3e3e3e')):'input',
 ('line',('#b0b0b0','#3e3e3e')):'border-alt',
 ('line',('#2b7fff','#2b7fff')):'ring-color',
 ('line',('#dc2626','#a22e2e')):'destructive',
 ('line',('#e5e5e5','#262626')):'border-alt-2',
 ('surface',('#f8f8f8','#262626')):'accent-alt-2',
 ('surface',('#111','#d4d4d4')):'foreground',
 ('line',('#111','#d4d4d4')):'foreground',
 ('surface',('#e5e5e5','#232323')):'border',
}
# role-intent exceptions: (file-substring, selector-substring) -> KEEP reason
EXC=[
 ('switch/style.css','.dx-switch','switch track is shadcn --input; ours is the border step (dark #232323 vs input #3e3e3e); snaps in phase B'),
]
def exc(f,sel,prop):
    if prop.startswith('background') and f.endswith('switch/style.css') and 'thumb' not in sel: return EXC[0][2]
    return None
def lookup(pc,pair,sel):
    if pc=='text' and pair==('#fff','#000') and 'tooltip' in sel: return 'background'
    r=RP.get((pc,pair))
    if callable(r): r=r(sel)
    return r
def find_end(v,i):
    j=i+4;d=1
    while d:
        c=v[j]
        if c=='(':d+=1
        elif c==')':d-=1
        j+=1
    return j
FORCED={'--primary-success-color':'success-subtle','--secondary-success-color':'success','--primary-warning-color':'warning-subtle','--secondary-warning-color':'warning','--primary-info-color':'info-subtle','--secondary-info-color':'info','--primary-error-color':'destructive','--secondary-error-color':'destructive-alt','--contrast-error-color':'destructive-foreground','--focused-border-color':'ring-color'}
def analyse(value,prop,sel,file=''):
    """returns (new_value, [per-ref dict]) ; refs in order of appearance"""
    # collect wrappers
    wr=[];i=0
    while i<len(value):
        m=re.match(r'var\(\s*--(light|dark)\s*,',value[i:])
        if m:
            j=find_end(value,i)
            inner_s=i+m.end(); wr.append(dict(mode=m.group(1),span=(i,j),inner=(inner_s,j-1)))
            i=j
        else: i+=1
    inwr=lambda pos:any(w['span'][0]<=pos<w['span'][1] for w in wr)
    edits=[] # (start,end,newtext)
    refs=[]  # per ref info
    # PAIRs
    used=set()
    for a,b in zip(wr,wr[1:]):
        if a['mode']!=b['mode'] and value[a['span'][1]:b['span'][0]].strip()=='' :
            used.add(id(a));used.add(id(b))
            ia=value[a['inner'][0]:a['inner'][1]].strip(); ib=value[b['inner'][0]:b['inner'][1]].strip()
            ma,mb=PLAIN.match(ia),PLAIN.match(ib)
            if ma and mb:
                L_w,D_w=(a,b) if a['mode']=='light' else (b,a)
                Lt=PLAIN.match(value[L_w['inner'][0]:L_w['inner'][1]].strip()).group(1)
                Dt=PLAIN.match(value[D_w['inner'][0]:D_w['inner'][1]].strip()).group(1)
                L=resolve('var(%s)'%Lt,T,'light');D=resolve('var(%s)'%Dt,T,'dark')
                pc=pc_of(prop,Lt if Lt.startswith('--sec') else Dt) 
                role=lookup(pc,(L,D),sel)
                if Lt in FORCED or Dt in FORCED: role=None
                if exc(file,sel,prop): role=None
                for w,t,m_ in ((a,ma.group(1),'pair'),(b,mb.group(1),'pair')):
                    refs.append(dict(tok=t,mode=w['mode'],kind='pair',pair=(L,D),pc=pc,role=role,start=w['span'][0]))
                if role: edits.append((a['span'][0],b['span'][1],'var(--dx-%s)'%role))
                continue
            else:
                used.discard(id(a));used.discard(id(b))
    # singles & refs inside wrappers not paired-plain, bare refs
    for m in re.finditer(REFTOK+r'(?![a-zA-Z0-9-])',value):
        pos=m.start(); tok=m.group(0)
        # already handled in pair?
        if any(r['start']<=pos<r['start']+1 for r in []): pass
        w=next((w for w in wr if w['span'][0]<=pos<w['span'][1]),None)
        if w is not None and id(w) in used: continue
        # need the 'var(' start of this ref
        vs=value.rfind('var(',0,pos)
        mode=w['mode'] if w else 'both'
        pair=pairhex(tok)
        pc=pc_of(prop,tok)
        role=FORCED.get(tok) or lookup(pc,pair,sel)
        if exc(file,sel,prop): role=None
        refs.append(dict(tok=tok,mode=mode,kind='bare' if w is None else 'single',pair=pair,pc=pc,role=role,start=vs,refspan=(vs,find_end(value,vs))))
        if role: edits.append((vs,find_end(value,vs),'var(--dx-%s)'%role))
    # apply edits (non-overlapping; pair spans cover their refs; singles inside used wrappers skipped)
    edits.sort(reverse=True)
    analyse.edits=list(edits)  # (start,end,newtext) in value coordinates; apply.py patches the raw source with these
    nv=value
    for s,e,t in edits: nv=nv[:s]+t+nv[e:]
    return nv,refs
