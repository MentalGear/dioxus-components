import json,collections,re,subprocess,csv
import os,sys
HERE=os.path.dirname(os.path.abspath(__file__))
ROOT=os.path.abspath(os.path.join(HERE,"../../../.."))
sys.path.insert(0,HERE)
from engine import *
import engine
from extract import parse2, strip_comments
files=subprocess.check_output(['git','ls-files','preview/src/**.css','preview/assets/*.css'],cwd=ROOT,text=True).split()
files=[f for f in files if 'dx-utilities' not in f and f!='preview/assets/dx-components-theme.css']
out=[];problems=[]
def short(p): return p
def reason(ref,file,sel,prop):
    e=exc(file,sel,prop)
    if e: return e
    tok=ref['tok'];pc=ref['pc'];L,D=ref['pair']
    if 'INVALID' in D or 'INVALID' in L: return f'{tok} is not defined in the theme (pre-existing bug: the declaration is invalid in that mode); do not repoint, fix in phase B'
    if pc=='other': return f'property `{prop}` outside role scope; step ({L}|{D})'
    if ref['kind']=='pair': return f'mixed-step light/dark pair ({L}|{D}) matches no role; snaps to a role in phase B (visual change in one mode)'
    return f'{tok} as {pc} = ({L}|{D}); no role carries this pair; phase B snaps it to a role (visual change)'
def verify(old,new,tag):
    for m in('light','dark'):
        norm=lambda x:re.sub(r'\s*([,()])\s*',r'\1',x)
        a=norm(resolve(old,T,m));b=norm(resolve(new,T,m))
        if a!=b: problems.append((tag,m,old,new,a,b))
# --- CSS
for f in files:
    rows=parse2(ROOT+'/'+f)
    seen=collections.OrderedDict()
    for r in rows: seen.setdefault((r['selector'],r['prop'],r['value']),[]).append(r)
    for (sel,prop,val),rs in seen.items():
        if prop.startswith('--'):
            for r in rs:
                name=prop
                out.append(dict(file=f,line=r['line'],selector=sel,property=prop,token=r['token'],mode='both',kind='custom-prop-def',role=name.replace('--dx-',''),action='DELETE-local-def',new='-',cur=val,note='root theme now defines the same name with the identical ramp value; delete the local declaration so a host --sidebar-* can reach it'))
            continue
        nv,refs=analyse(val,prop,sel,f)
        verify(val,nv,(f,sel,prop))
        # match refs to rows by order -> rows sorted by line; use per-ref order of appearance in value
        refs_sorted=sorted(refs,key=lambda x:x['start'])
        rs_sorted=sorted(rs,key=lambda x:x['line'])
        # row order in parse2 == order of appearance in value
        rs_in_order=rs
        assert len(refs_sorted)==len(rs_in_order),(f,sel,prop,val,len(refs_sorted),len(rs_in_order))
        for ref,r in zip(refs_sorted,rs_in_order):
            assert ref['tok']==r['token'],(f,sel,prop,val,ref['tok'],r['token'])
            role=ref['role'] if not exc(f,sel,prop) else None
            out.append(dict(file=f,line=r['line'],selector=sel,property=prop,token=ref['tok'],mode=ref['mode'],kind=ref['kind'],
                role=role or '-',action='REPOINT' if role else 'KEEP-phaseB',new=nv if role else '-',cur=val,
                note='' if role else reason(ref,f,sel,prop)))
# --- rust inline
rsfiles=subprocess.check_output("git ls-files '*.rs' | xargs grep -lE -e '--(primary|secondary|contrast)-[a-z0-9-]*color|--focused-border-color'",shell=True,cwd=ROOT,text=True).split()
PROP=re.compile(r'(?<![\w-])([a-z][a-z-]*)\s*:\s*"?([^;":]*)')
for f in rsfiles:
    if f.startswith('primitives/'): continue
    for i,line in enumerate(open(ROOT+'/'+f).read().split('\n'),1):
        st=line.strip()
        if st.startswith('//') or st.startswith('*') or st.startswith('/*'): continue
        if not REF.search(line) if False else not re.search(REFTOK,line): continue
        found=False
        for m in re.finditer(r'(?<![\w-])([a-z][a-z-]*)\s*:\s*',line):
            prop=m.group(1)
            if prop=='style': continue
            j=m.end();d=0;k=j
            while k<len(line):
                c=line[k]
                if c=='(':d+=1
                elif c==')':d-=1
                elif d==0 and c in ';"': break
                k+=1
            val=line[j:k]
            if not re.search(REFTOK,val): continue
            sel='(inline style / Rust string) '+f.split('/')[-2]
            nv,refs=analyse(val,prop,sel,f)
            verify(val,nv,(f,i,prop))
            for ref in sorted(refs,key=lambda x:x['start']):
                found=True
                role=ref['role']
                out.append(dict(file=f,line=i,selector='(inline style)',property=prop,token=ref['tok'],mode=ref['mode'],kind=ref['kind'],role=role or '-',action='REPOINT' if role else 'KEEP-phaseB',new=nv if role else '-',cur=val.strip(),note='' if role else reason(ref,f,sel,prop)))
        if not found:
            pm=re.search(r'(?<![\w-])([a-z][a-z-]*)\s*:',line)
            for q in re.findall(r'"([^"]*)"',line):
                if not re.search(REFTOK,q): continue
                prop=pm.group(1);sel='(inline style / Rust string)'
                nv,refs=analyse(q,prop,sel,f)
                verify(q,nv,(f,i,prop))
                for ref in sorted(refs,key=lambda x:x['start']):
                    found=True
                    role=ref['role']
                    out.append(dict(file=f,line=i,selector='(inline style)',property=prop,token=ref['tok'],mode=ref['mode'],kind=ref['kind'],role=role or '-',action='REPOINT' if role else 'KEEP-phaseB',new=nv if role else '-',cur=q.strip(),note='' if role else reason(ref,f,sel,prop)))
        if not found:
            problems.append(('rs-unparsed',f,i,line.strip()[:120]))
print(len(out),collections.Counter(r['action'] for r in out))
print('problems',len(problems))
for p in problems[:20]: print(p)

# ---- lane assignment (greedy, whole component dirs / asset files) + TSV
def unit(f):
    m=re.match(r'preview/src/components/([^/]+)/',f)
    if m: return 'comp:'+m.group(1)
    if f.startswith('preview/src/dashboard'): return 'dashboard'
    if f=='preview/src/main.rs': return 'preview-main.rs'
    return f.replace('preview/assets/','assets:')
W=collections.Counter(unit(r['file']) for r in out)
lanes=[[] for _ in range(4)];tot=[0]*4
for u,n in sorted(W.items(),key=lambda x:(-x[1],x[0])):
    i=tot.index(min(tot)); lanes[i].append(u); tot[i]+=n
lane_of={u:'ABCD'[i] for i,l in enumerate(lanes) for u in l}
for i,l in enumerate(lanes): print('lane','ABCD'[i],tot[i],', '.join(sorted(l)))
if '--write' in sys.argv:
    cols=['file','line','selector','property','current_token','mode','kind','inferred_role','action','replacement_token','new_value','current_value','reason','lane']
    clean=lambda x: re.sub(r'[\t\r\n]+',' ',str(x)).strip()
    out.sort(key=lambda r:(r['file'],r['line']))
    with open(os.path.join(HERE,'..','sites.tsv'),'w') as fh:
        fh.write('\t'.join(cols)+'\n')
        for r in out:
            rep=('--dx-'+r['role']) if r['action']=='REPOINT' else ('(delete)' if r['action']=='DELETE-local-def' else '-')
            row=[r['file'],r['line'],r['selector'],r['property'],r['token'],r['mode'],r['kind'],r['role'],r['action'],rep,r['new'],r['cur'],r['note'],lane_of[unit(r['file'])]]
            fh.write('\t'.join(clean(x) for x in row)+'\n')
    print('wrote sites.tsv',len(out))
sys.exit(1 if problems else 0)
