#!/usr/bin/env python3
"""Audit declared sRGB colors and conservative static glass layers; not LCD certification."""
import json
from pathlib import Path
import re
ROOT=Path(__file__).resolve().parent.parent
source=(ROOT/'app/ui/design.slint').read_text()
rows=re.findall(r'base:\s*#([0-9a-f]{6}),ink:\s*#([0-9a-f]{6}),muted:\s*#([0-9a-f]{6}),accent:\s*#([0-9a-f]{6})',source)
assert len(rows)==4

def rgb(h):return tuple(int(h[i:i+2],16)/255 for i in (0,2,4))
def blend(f,b,a):return tuple(x*a+y*(1-a) for x,y in zip(f,b))
def lum(c):return sum((v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4)*w for v,w in zip(c,(.2126,.7152,.0722)))
def contrast(a,b):
    a,b=sorted((lum(a),lum(b)))
    return (b+.05)/(a+.05)

def audit(palette):
    base,ink,muted,accent=palette
    backgrounds=[base]
    for ambient in (0,.14):
        bg=blend(rgb('9aada8'),base,ambient)
        bg=blend(ink,bg,.055)
        backgrounds.extend((bg,blend((1,1,1),bg,.07),blend((0,0,0),bg,.035)))
    return {'main':min(contrast(ink,b) for b in backgrounds),'secondary':min(contrast(muted,b) for b in backgrounds),'tag':min(contrast(accent,b) for b in backgrounds),'button':contrast(base,accent)}

names=['despertar','horizonte','refugio','silencio']
palettes=[list(map(rgb,row)) for row in rows]
report={name:{key:round(value,2) for key,value in audit(p).items()} for name,p in zip(names,palettes)}
# Only equal-polarity pairs interpolate; cross-polarity text/background switch atomically.
minimum=100
for a,b in [(0,1),(2,3)]:
    for step in range(101):
        p=[blend(y,x,step/100) for x,y in zip(palettes[a],palettes[b])]
        minimum=min(minimum,*audit(p).values())
report['same_polarity_transition_minimum']=round(minimum,2)
assert minimum>=4.5,report
import argparse
parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,default=ROOT/'docs/contrast-audit.json');args=parser.parse_args()
args.output.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
