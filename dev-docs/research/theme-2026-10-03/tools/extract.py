import re, sys, subprocess, json
import os,sys
HERE=os.path.dirname(os.path.abspath(__file__))
ROOT=os.path.abspath(os.path.join(HERE,"../../../.."))
sys.path.insert(0,HERE)
REF=re.compile(r'--(?:(?:primary|secondary)-color(?:-\d+)?|(?:primary|secondary|contrast)-(?:success|warning|error|info)-color|focused-border-color)(?![a-zA-Z0-9-])')
def strip_comments(s):
    return re.sub(r'/\*.*?\*/', lambda m: re.sub(r'[^\n]', ' ', m.group(0)), s, flags=re.S)
def parse(path):
    src=strip_comments(open(path).read())
    rows=[]; stack=[]; buf=[]; line=1; start_line=1; depth=0; q=None
    def flush(decl, dline):
        d=decl
        if ':' not in d: return
        prop,_,val=d.partition(':')
        prop=prop.strip()
        # per-ref lines
        off=d.index(':')+1
        for m in REF.finditer(val):
            # compute line
            pre=d[:off+m.start()]
            ln=dline+pre.count('\n')-(len(pre)-len(pre.lstrip('\n'))>0 and 0 or 0)
            rows.append((prop,val.strip(),m.group(0),ln))
    cur_start=None
    for i,ch in enumerate(src):
        if ch=='\n': line+=1
        if q:
            buf.append(ch)
            if ch==q: q=None
            continue
        if ch in '"\'': q=ch; buf.append(ch); 
        elif ch=='(': depth+=1; buf.append(ch)
        elif ch==')': depth-=1; buf.append(ch)
        elif ch=='{' and depth==0:
            sel=' '.join(''.join(buf).split()); stack.append(sel); buf=[]
        elif ch==';' and depth==0:
            d=''.join(buf); 
            lead=len(d)-len(d.lstrip())
            dline=line-d.count('\n')+d[:lead].count('\n')
            for p,v,t,ln in [(a,b,c,dline+ (d[:d.index(':')+1+0].count('\n')) ) for a,b,c,_ in []]: pass
            flushd(rows,stack,d,line)
            buf=[]
        elif ch=='}' and depth==0:
            d=''.join(buf)
            if d.strip(): flushd(rows,stack,d,line)
            buf=[]
            if stack: stack.pop()
        else: buf.append(ch)
    return rows
def flushd(rows,stack,d,endline):
    if ':' not in d: return
    prop,_,val=d.partition(':')
    prop=prop.strip()
    startline=endline-d.count('\n')
    off=len(prop_part:=d[:d.index(':')+1])
    for m in REF.finditer(d[off:]):
        pre=d[:off+m.start()]
        ln=startline+pre.count('\n')
        sel=' | '.join(s for s in stack)
        rows.append(dict(line=ln,selector=sel,prop=prop,value=' '.join(val.split()),token=m.group(0)))
parse_global=None
def parse2(path):
    rows=[]
    src=strip_comments(open(path).read())
    stack=[];buf=[];line=1;depth=0;q=None
    for ch in src:
        if ch=='\n': line+=1
        if q:
            buf.append(ch)
            if ch==q:q=None
            continue
        if ch in '"\'': q=ch; buf.append(ch)
        elif ch=='(': depth+=1; buf.append(ch)
        elif ch==')': depth-=1; buf.append(ch)
        elif ch=='{' and depth==0:
            stack.append(' '.join(''.join(buf).split())); buf=[]
        elif ch==';' and depth==0:
            flushd(rows,stack,''.join(buf),line); buf=[]
        elif ch=='}' and depth==0:
            d=''.join(buf)
            if d.strip(): flushd(rows,stack,d,line)
            buf=[]
            if stack: stack.pop()
        else: buf.append(ch)
    return rows
if __name__=='__main__':
    files=subprocess.check_output(['git','ls-files','preview/src/**.css','preview/assets/*.css'],cwd=ROOT,text=True).split()
    files=[f for f in files if 'dx-utilities' not in f and f!='preview/assets/dx-components-theme.css']
    out=[]
    for f in files:
        for r in parse2(ROOT+'/'+f):
            r['file']=f; out.append(r)
    json.dump(out,open(sys.argv[1],'w'),indent=0)
    print(len(out),len(files))
