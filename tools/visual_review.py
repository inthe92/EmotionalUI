#!/usr/bin/env python3
"""Reproducible real-render review of existing sections and global navigation."""
import json,os,shutil,sqlite3,subprocess,sys
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
binary,out=map(lambda p:Path(p).resolve(),sys.argv[1:]);repo=Path(__file__).resolve().parents[1]
subprocess.run([sys.executable,str(repo/'tools/visual_pair.py'),str(binary),str(out)],check=True)
fixture=out.parent/'fixtures';seed=out.parent/'visual-seed.sqlite';checks=[]
def keys(names):
 steps=[]
 for i,key in enumerate(names):steps.extend([dict(at_ms=900+i*100,text=key,pressed=True),dict(at_ms=908+i*100,text=key,pressed=False)])
 return steps
def run(name,steps=(),extra=(),fixture_db=True,view='resume'):
 db=out/(name+'.sqlite')
 if fixture_db:shutil.copyfile(seed,db)
 args=[str(binary),'--db',str(db),'--view',view,'--at','2026-10-19T18:30','--audio-mode','silence','--reduced-motion','--smoke-seconds','3','--snapshot',str(out/(name+'.png'))]
 if fixture_db:args+=['--library-root',str(fixture)]
 args+=list(map(str,extra))
 if steps:
  replay=out/(name+'.toml');replay.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(k+'='+json.dumps(str(v) if isinstance(v,Path) else v)+'\n' for k,v in s.items()) for s in steps));args+=['--sim-replay',str(replay)]
 p=subprocess.run(args,capture_output=True,text=True,timeout=30,env=dict(os.environ,SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(out.parent/'cache')))
 (out/(name+'.stdout')).write_text(p.stdout);(out/(name+'.stderr')).write_text(p.stderr);assert p.returncode==0,(name,p.stderr)
 image=Image.open(out/(name+'.png'));image.load();assert image.mode=='RGB' and image.size==(640,480)
 checks.append(dict(case=name,passed=True))
run('biblioteca',[*keys(['a']),dict(at_ms=1800,library=True,active_section=2)])
run('ajustes',[*keys(['s']),dict(at_ms=1800,menu=True,active_section=3)])
run('capsula-cerrada',[*keys(['up']),dict(at_ms=1800,system_open=False,system_focused=True)])
run('capsula-abierta',[*keys(['up','z']),dict(at_ms=1800,system_open=True,system_focused=True,selected=0,library=False,menu=False)])
run('capsula-navegacion',[*keys(['up','z','right','e']),dict(at_ms=1400,system_open=True,resume=True,selected=0),dict(at_ms=1600,text='x',pressed=True,system_open=False,system_focused=True),dict(at_ms=1608,text='x',pressed=False),dict(at_ms=1750,text='down',pressed=True,system_focused=False,selected=0),dict(at_ms=1758,text='down',pressed=False)])
run('descubre-sin-arte',extra=['--demo-history','--visual-case','no-art'],fixture_db=False,view='emotional')
run('vacio',extra=['--demo-count','0'],fixture_db=False)
run('biblioteca-vacia',[*keys(['z']),dict(at_ms=1800,library=True,active_section=2)],extra=['--demo-count','0'],fixture_db=False)
run('titulo-largo',[dict(at_ms=1600,text='up',pressed=True,selected=0,dock_cursor=-1),dict(at_ms=1608,text='up',pressed=False)],extra=['--trial','--library-root',repo/'docs/editorial-artwork-2026-10-19/fixtures/long-title'],fixture_db=False,view='emotional')
# All destinations are real, the inactive destination can take focus independently.
steps=[*keys(['right','left','left','z']),dict(at_ms=1350,active_section=0,dock_cursor=0),dict(at_ms=1450,text='right',pressed=True,dock_cursor=1,active_section=0),dict(at_ms=1458,text='right',pressed=False),dict(at_ms=1550,text='z',pressed=True,active_section=1),dict(at_ms=1558,text='z',pressed=False),dict(at_ms=1650,text='right',pressed=True,dock_cursor=2),dict(at_ms=1658,text='right',pressed=False),dict(at_ms=1750,text='z',pressed=True,library=True,active_section=2),dict(at_ms=1758,text='z',pressed=False),dict(at_ms=1850,text='right',pressed=True,dock_cursor=3),dict(at_ms=1858,text='right',pressed=False),dict(at_ms=1950,text='z',pressed=True,menu=True,active_section=3),dict(at_ms=1958,text='z',pressed=False),dict(at_ms=2200,text='left',pressed=True,dock_cursor=2),dict(at_ms=2208,text='left',pressed=False),dict(at_ms=2300,text='left',pressed=True,dock_cursor=1),dict(at_ms=2308,text='left',pressed=False),dict(at_ms=2400,text='left',pressed=True,dock_cursor=0,active_section=3),dict(at_ms=2408,text='left',pressed=False)]
run('cuatro-destinos',steps)
# Guidance is an annotated diagnostic of a real render, not another app screenshot.
im=Image.open(out/'descubre.png').copy();d=ImageDraw.Draw(im);font=ImageFont.truetype(str(repo/'assets/fonts/DejaVuSans.ttf'),11)
for box,label in [((24,16,408,72),'Cabecera x24 / y16'),((24,104,616,344),'R16 / imagen interior R12 (separacion 4)'),((24,352,312,400),'Baldosa R12'),((148,416,492,468),'Dock 344 x 52 / R26'),((364,292,552,328),'Accion 188 x 36 / R18'),((440,16,616,44),'Capsula 176 x 28 / R14')]:
 d.rectangle(box,outline='#9ce2bf',width=1);d.text((box[0]+4,box[1]+2),label,font=font,fill='#9ce2bf',stroke_width=1,stroke_fill='#101713')
for x in [24,148,492,616]:d.line((x,0,x,479),fill='#9ce2bf',width=1)
im.save(out/'guias.png')
(out/'visual-review.json').write_text(json.dumps(dict(passed=True,checks=checks,scope='Real Slint software renderer; owned art/activity fixtures; system indicators unknown, no system controls changed; not hardware validation'),indent=2)+'\n')
