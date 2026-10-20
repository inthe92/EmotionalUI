#!/usr/bin/env python3
"""Conservative text contrast over the brightest possible scene behind the panel."""
import json,sys
from pathlib import Path
def rgb(s):return [int(s[i:i+2],16)/255 for i in (0,2,4)]
def lum(c):
 x=[v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4 for v in c]
 return sum(a*b for a,b in zip(x,[.2126,.7152,.0722]))
cases=[]
for stop in ['17201cf5','24241ddd']:
 a=int(stop[6:],16)/255;background=[v*a+1-a for v in rgb(stop)]
 for name,color in [('title','f1e5d2'),('body','c9bfaf'),('label','ddbc95')]:
  l1,l2=sorted([lum(rgb(color)),lum(background)]);ratio=(l2+.05)/(l1+.05);assert ratio>=4.5
  cases.append(dict(panel_stop=stop,text=name,contrast=round(ratio,3),minimum=4.5,passed=True))
Path(sys.argv[1]).write_text(json.dumps(dict(scope='Color calculation against white behind the translucent panel; no physical LCD certification',cases=cases),indent=2)+'\n')
