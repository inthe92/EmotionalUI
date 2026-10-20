#!/usr/bin/env python3
"""Audit actual prerendered material pixels; no claim about a physical LCD."""
import json,re,sys
from pathlib import Path
from PIL import Image
root=Path(__file__).resolve().parents[1]
source=(root/'app/ui/integrated.slint').read_text()
colors=dict(re.findall(r'property <color> (ink|muted|mark):#([0-9a-f]{6})',source))
def rgb(value):return [int(value[i:i+2],16)/255 for i in (0,2,4)]
def luminance(c):return sum((x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4)*w for x,w in zip(c,(.2126,.7152,.0722)))
def contrast(a,b):
 lo,hi=sorted((luminance(a),luminance(b)));return (hi+.05)/(lo+.05)
report={}
for path in sorted((root/'assets/illustrations').glob('room-*.png')):
 image=Image.open(path);image.load();assert image.mode=='RGBA'
 assert image.getpixel((0,0))[3]==0,('Square material corner',path)
 samples=[image.getpixel((x,y)) for y in range(image.height) for x in range(image.width) if image.getpixel((x,y))[3]>=225];assert samples
 minimum=100
 for r,g,b,a in samples:
  for scene in (0.,1.):
   bg=[x/255*a/255+scene*(1-a/255) for x in (r,g,b)]
   # Text on the normal panel and on its stronger active selection tint.
   for strength in (0.,.14):
    lit=[x*(1-strength)+y*strength for x,y in zip(bg,rgb(colors['mark']))]
    minimum=min(minimum,*(contrast(rgb(value),lit) for value in colors.values()))
 assert minimum>=4.5,(path,minimum)
 report[path.name]=dict(size=list(image.size),minimum_text_contrast=round(minimum,3),transparent_corner=True)
Path(sys.argv[1]).write_text(json.dumps(dict(passed=True,scope='Interior material pixels over white/black, including active tint; corners excluded from text zones; no LCD certification',materials=report),indent=2)+'\n')
