import re
# Mini resolver for the space-toggle + var() fallback subset used in this repo.
def parse_theme(path):
    src=open(path).read()
    src=re.sub(r'/\*.*?\*/','',src,flags=re.S)
    m=re.search(r'\n:root\s*\{(.*?)\n\}',src,flags=re.S)
    toks={}
    body=m.group(1)
    # split declarations on ';' at depth 0
    depth=0;buf=''
    for ch in body:
        if ch=='(':depth+=1
        if ch==')':depth-=1
        if ch==';' and depth==0:
            if ':' in buf:
                k,_,v=buf.partition(':'); toks[k.strip()]=' '.join(v.split())
            buf=''
        else: buf+=ch
    return toks
def split_args(s):
    # split "name, fallback" at first top-level comma
    depth=0
    for i,ch in enumerate(s):
        if ch=='(':depth+=1
        elif ch==')':depth-=1
        elif ch==',' and depth==0: return s[:i].strip(), s[i+1:].strip()
    return s.strip(), None
def resolve(val,toks,mode,extra=None,depth=0):
    """mode 'light'|'dark'. Returns normalized string. Unknown custom prop w/o fallback -> '<<INVALID>>'."""
    if depth>40: raise RuntimeError('loop')
    out='';i=0
    while i<len(val):
        if val.startswith('var(',i):
            j=i+4;d=1
            while d:
                if val[j]=='(':d+=1
                elif val[j]==')':d-=1
                j+=1
            inner=val[i+4:j-1]
            name,fb=split_args(inner)
            if name=='--light': r=resolve(fb,toks,mode,extra,depth+1) if mode=='light' else ''
            elif name=='--dark': r=resolve(fb,toks,mode,extra,depth+1) if mode=='dark' else ''
            elif extra and name in extra: r=resolve(extra[name],toks,mode,extra,depth+1)
            elif name in toks: r=resolve(toks[name],toks,mode,extra,depth+1)
            elif fb is not None: r=resolve(fb,toks,mode,extra,depth+1)
            else: r='<<INVALID:%s>>'%name
            out+=r;i=j
        else:
            out+=val[i];i+=1
    return ' '.join(out.split())
