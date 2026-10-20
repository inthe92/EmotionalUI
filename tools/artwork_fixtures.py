#!/usr/bin/env python3
"""Owned procedural test graphics, not editorial/final art or commercial games."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import argparse,json,hashlib
P=argparse.ArgumentParser();P.add_argument('output',type=Path);a=P.parse_args();a.output.mkdir(parents=True,exist_ok=False)
font=ImageFont.truetype(str(Path(__file__).resolve().parents[1]/'assets/fonts/DejaVuSerif.ttf'),20)
def landscape(size,bright=False,dark=False):
 w,h=size;im=Image.new('RGB',size);d=ImageDraw.Draw(im)
 for y in range(h):
  t=y/h;d.line((0,y,w,y),fill=(int(195-95*t),int(145-48*t),int(100-17*t)))
 d.ellipse((w*.7,h*.14,w*.7+h*.24,h*.38),fill=(239,211,151))
 d.polygon([(0,h*.65),(w*.2,h*.3),(w*.4,h*.57),(w*.68,h*.33),(w,h*.7),(w,h),(0,h)],fill=(62,83,79))
 d.polygon([(0,h*.8),(w*.3,h*.58),(w*.5,h*.8),(w*.8,h*.57),(w,h*.78),(w,h),(0,h)],fill=(34,57,51))
 d.rectangle((w*.16,h*.67,w*.28,h*.88),fill=(112,78,51));d.polygon([(w*.12,h*.68),(w*.22,h*.55),(w*.32,h*.68)],fill=(49,42,38));d.rectangle((w*.2,h*.73,w*.24,h*.8),fill=(241,195,109))
 if bright:im=im.point(lambda v:min(255,int(v*.6+140)))
 if dark:im=im.point(lambda v:int(v*.15))
 return im
cases=[('hero','Un lugar al que volver','fanart'),('boxart','Un lugar al que volver','boxart'),('screenshot','Un lugar al que volver','screenshot'),('pixel-art','Un lugar al que volver','screenshot'),('logo','Un lugar al que volver','marquee'),('missing','Un lugar al que volver',None),('corrupt','Un lugar al que volver','image'),('long-title','La casa junto al último faro y el pequeño viaje de regreso entre montañas','boxart'),('bright','Un lugar al que volver','fanart'),('dark','Un lugar al que volver','fanart')]
manifest=[]
for case,title,field in cases:
 root=a.output/case;(root/'snes').mkdir(parents=True);rom=root/'snes/owned.fixture';rom.write_text('Original text fixture; NOT a ROM.\n');art=root/'snes/art.png'
 if case in ('boxart','long-title'):
  im=landscape((170,280));d=ImageDraw.Draw(im);d.rectangle((0,0,169,58),fill=(28,39,34));d.text((12,6),'LA CASA\nDEL FARO',font=font,fill=(242,222,184));im.save(art)
 elif case=='pixel-art':landscape((64,48)).save(art)
 elif case=='screenshot':landscape((320,240)).save(art)
 elif case=='logo':
  im=Image.new('RGBA',(250,90),(0,0,0,0));ImageDraw.Draw(im).text((12,26),'LA CASA DEL FARO',font=font,fill=(235,207,159,255));im.save(art)
 elif case=='corrupt':art.write_bytes(b'not a valid PNG')
 elif case!='missing':landscape((640,360),case=='bright',case=='dark').save(art)
 # Same identity and text across art modes. Deliberately unrelated to real ROM names.
 xml=root/'snes/gamelist.xml';xml.write_text(f'<gameList><game><path>owned.fixture</path><name>{title}</name><genre>Aventura</genre>'+ (f'<{field}>art.png</{field}>' if field else '') +'</game></gameList>')
 manifest.append(dict(case=case,title=title,field=field,files={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'snes').iterdir()}))
(a.output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
