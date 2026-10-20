#!/usr/bin/env python3
"""Small visual-review fixture. Real Slint renders, no hardware/gameplay claims."""
import argparse,hashlib,json,os,shutil,sqlite3,subprocess,struct
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
p=argparse.ArgumentParser();p.add_argument('binary',type=Path);p.add_argument('output',type=Path);a=p.parse_args();sim=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);repo=Path(__file__).resolve().parents[1]
fixture=out.parent/'fixtures';folder=fixture/'snes';folder.mkdir(parents=True,exist_ok=True)
if not (folder/'gamelist.xml').exists():
 entries=[]
 for i,(art,title,field) in enumerate([('boxart','La casa del faro','boxart'),('screenshot','Entre montañas','screenshot'),('hero','El viaje de vuelta','fanart')]):
  (folder/f'game-{i}.fixture').write_text('Original text fixture; not a ROM.\n')
  shutil.copyfile(repo/f'docs/editorial-artwork-2026-10-19/fixtures/{art}/snes/art.png',folder/f'art-{i}.png')
  entries.append(f'<game><path>game-{i}.fixture</path><name>{title}</name><{field}>art-{i}.png</{field}></game>')
 (folder/'gamelist.xml').write_text('<gameList>'+''.join(entries)+'</gameList>')
seed=out.parent/'visual-seed.sqlite'
if not seed.exists():
 r=subprocess.run([str(sim),'--db',str(seed),'--import-gamelist',str(folder/'gamelist.xml'),'--rom-root',str(folder)],capture_output=True,text=True,check=True)
 with sqlite3.connect(seed) as c:
  for i,(id,) in enumerate(c.execute('SELECT id FROM imported_games ORDER BY id').fetchall()):c.execute('INSERT INTO activity VALUES(?,?,?)',(id,1792426800-i*600,1200))
 c.close()
checks=[]
for view in ['resume','emotional']:
 for dock in [False,True]:
  name=('continuar' if view=='resume' else 'descubre')+('-dock' if dock else '')
  db=out/(name+'.sqlite');shutil.copyfile(seed,db)
  args=[str(sim),'--db',str(db),'--library-root',str(fixture),'--view',view,'--at','2026-10-19T18:30','--audio-mode','silence','--reduced-motion','--smoke-seconds','2','--snapshot',str(out/(name+'.png'))]
  steps=[dict(at_ms=700,resume_count=3,has_recent=True,resume=view=='resume')]
  if dock:
   keys=['down','down','left'] if view=='resume' else ['down','down','left','left']
   for i,key in enumerate(keys):steps.extend([dict(at_ms=900+i*90,text=key,pressed=True),dict(at_ms=908+i*90,text=key,pressed=False)])
   steps.append(dict(at_ms=1400,dock_cursor=1 if view=='resume' else 0))
  replay=out/(name+'.toml');replay.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in s.items()) for s in steps));args+=['--sim-replay',str(replay)]
  r=subprocess.run(args,capture_output=True,text=True,timeout=20,env=dict(os.environ,SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(out.parent/'cache')))
  (out/(name+'.stdout')).write_text(r.stdout);(out/(name+'.stderr')).write_text(r.stderr);assert r.returncode==0,(name,r.stderr)
  im=Image.open(out/(name+'.png'));im.load();assert im.mode=='RGB' and im.size==(640,480)
  checks.append(dict(case=name,passed=True,sha256=hashlib.sha256((out/(name+'.png')).read_bytes()).hexdigest()))
font=ImageFont.truetype(str(repo/'assets/fonts/DejaVuSans.ttf'),18)
for focus in [False,True]:
 sheet=Image.new('RGB',(1296,524),(24,23,19));d=ImageDraw.Draw(sheet)
 for i,label in enumerate(['Continuar','Descubre']):
  x=i*656;d.text((x+12,8),label+(' · foco en dock' if focus else ''),font=font,fill=(245,235,216));sheet.paste(Image.open(out/(label.lower()+('-dock' if focus else '')+'.png')),(x,36))
 sheet.save(out/('pareja-dock.jpg' if focus else 'pareja.jpg'),quality=95,subsampling=0)
(out/'review.json').write_text(json.dumps(dict(passed=True,scope='Slint software renderer on desktop; fixed clock and owned image/activity fixtures; no physical validation',checks=checks),indent=2)+'\n')
