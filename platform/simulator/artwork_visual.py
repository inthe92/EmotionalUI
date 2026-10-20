#!/usr/bin/env python3
"""Actual Slint captures using owned fixtures prepared by tools/artwork_fixtures.py."""
import hashlib,json,os,shutil,sqlite3,struct,subprocess,sys,tomllib,zlib
from pathlib import Path
sim,fixtures,out=map(Path,sys.argv[1:]);sim=sim.resolve();fixtures=fixtures.resolve();out=out.resolve();out.mkdir(parents=True,exist_ok=False)
def verify_png(path):
 data=path.read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n' and struct.unpack('>II',data[16:24])==(640,480) and data[24:26]==bytes((8,2))
 at=8;chunks=[]
 while at<len(data):
  n=struct.unpack('>I',data[at:at+4])[0];kind=data[at+4:at+8];payload=data[at+8:at+8+n];assert zlib.crc32(kind+payload)&0xffffffff==struct.unpack('>I',data[at+8+n:at+12+n])[0]
  if kind==b'IDAT':chunks.append(payload)
  at+=12+n
 raw=zlib.decompress(b''.join(chunks));assert len(raw)==480*(640*3+1) and len(set(raw))>80
 return hashlib.sha256(data).hexdigest()
cases=json.loads((fixtures/'manifest.json').read_text());report=[]
for case in cases:
 name=case['case'];root=fixtures/name
 log=out/(name+'.toml');png=out/(name+'.png')
 replay=out/(name+'-focus.toml');replay.write_text('keyboard=true\n[mapping]\n[[steps]]\nat_ms=1300\ntext="up"\npressed=true\nselected=0\n[[steps]]\nat_ms=1308\ntext="up"\npressed=false\n')
 command=[str(sim),'--trial','--library-root',str(root),'--db',str(out/(name+'.sqlite')),'--view','emotional','--at','2026-10-19T18:30','--audio-mode','silence','--reduced-motion','--smoke-test','--sim-replay',str(replay),'--smoke-seconds','3','--snapshot',str(png),'--smoke-log',str(log)]
 env=dict(os.environ,EMOTIONAL_ARTWORK_CACHE=str(out/'cache'),SLINT_SCALE_FACTOR='1')
 p=subprocess.run(command,capture_output=True,text=True,timeout=20,env=env);(out/(name+'.stdout')).write_text(p.stdout);(out/(name+'.stderr')).write_text(p.stderr);assert p.returncode==0,(name,p.stderr)
 data=png.read_bytes();assert struct.unpack('>II',data[16:24])==(640,480) and data[24:26]==bytes((8,2))
 events=tomllib.loads(log.read_text())['events'];ready=[e for e in events if e['event']=='local_art_ready'];expected={'hero':'Landscape','boxart':'Boxart','screenshot':'Screenshot','pixel-art':'Screenshot','long-title':'Boxart','bright':'Landscape','dark':'Landscape'}.get(name)
 if expected:assert ready and expected in ready[0].get('detail',''),(name,ready)
 else:assert not ready,(name,ready)
 assert all(hashlib.sha256((root/path).read_bytes()).hexdigest()==digest for path,digest in case['files'].items())
 report.append(dict(case=name,passed=True,mode=expected or 'Typography',events=ready,sha256=hashlib.sha256(data).hexdigest()))
 # A new process must reuse the persistent variant, not merely the memory LRU.
 if name=='boxart':
  command[command.index('--snapshot')+1]=str(out/'boxart-warm.png');command[command.index('--smoke-log')+1]=str(out/'boxart-warm.toml');p=subprocess.run(command,capture_output=True,text=True,timeout=20,env=env);assert p.returncode==0
  warm=tomllib.loads((out/'boxart-warm.toml').read_text())['events'];assert any('cache_hit=true' in e.get('detail','') for e in warm)
# Three installed text fixtures exercise real imported art in both layouts.
collection=out/'collection/snes';collection.mkdir(parents=True)
entries=[]
for i,(name,title) in enumerate([('boxart','La casa del faro'),('screenshot','Entre montañas'),('hero','El viaje de vuelta')]):
 (collection/f'game-{i}.fixture').write_text('Original text fixture, not a ROM.\n')
 shutil.copyfile(fixtures/name/'snes/art.png',collection/f'art-{i}.png')
 field={'boxart':'boxart','screenshot':'screenshot','hero':'fanart'}[name]
 entries.append(f'<game><path>game-{i}.fixture</path><name>{title}</name><{field}>art-{i}.png</{field}></game>')
(collection/'gamelist.xml').write_text('<gameList>'+''.join(entries)+'</gameList>')
db=out/'collection.sqlite'
base=[str(sim),'--trial','--library-root',str(collection.parent),'--db',str(db),'--at','2026-10-19T18:30','--audio-mode','silence','--reduced-motion','--smoke-seconds','3']
env=dict(os.environ,EMOTIONAL_ARTWORK_CACHE=str(out/'cache'),SLINT_SCALE_FACTOR='1')
p=subprocess.run([*base,'--view','emotional','--snapshot',str(out/'descubre-coleccion.png'),'--smoke-log',str(out/'collection-discover.toml')],capture_output=True,text=True,timeout=20,env=env)
(out/'collection-discover.stdout').write_text(p.stdout);(out/'collection-discover.stderr').write_text(p.stderr);assert p.returncode==0
with sqlite3.connect(db) as c:
 ids=[r[0] for r in c.execute('SELECT id FROM imported_games ORDER BY id')]
 for i,id in enumerate(ids):c.execute('INSERT INTO activity VALUES(?,?,?)',(id,1792426800-i*600,1200))
replay=out/'collection-focus.toml';replay.write_text('keyboard=true\n[mapping]\n[[steps]]\nat_ms=1500\nresume=true\nresume_count=3\ntext="down"\npressed=true\nselected=4\n[[steps]]\nat_ms=1508\ntext="down"\npressed=false\n[[steps]]\nat_ms=1600\ntext="right"\npressed=true\nselected=5\n[[steps]]\nat_ms=1608\ntext="right"\npressed=false\n')
p=subprocess.run([*base,'--view','resume','--snapshot',str(out/'continuar-coleccion.png'),'--sim-replay',str(replay),'--smoke-log',str(out/'collection-resume.toml')],capture_output=True,text=True,timeout=20,env=env)
(out/'collection-resume.stdout').write_text(p.stdout);(out/'collection-resume.stderr').write_text(p.stderr);assert p.returncode==0,p.stderr
ready=[e for e in tomllib.loads((out/'collection-resume.toml').read_text())['events'] if e['event']=='local_art_ready']
assert all(any(f'slot={slot};' in e.get('detail','') for e in ready) for slot in [0,2,3]),ready
report.append(dict(case='three-imported-games-both-views',passed=True,events=ready,activity='synthetic SQLite fixture; not a real play session'))
# Simulator-only delay: controls remain usable while the image worker is slow.
slow=out/'slow-worker.toml';slow.write_text('keyboard=true\n[mapping]\n[[steps]]\nat_ms=500\ntext="s"\npressed=true\nmenu=true\n[[steps]]\nat_ms=508\ntext="s"\npressed=false\n[[steps]]\nat_ms=800\ntext="x"\npressed=true\nmenu=false\n[[steps]]\nat_ms=808\ntext="x"\npressed=false\n')
p=subprocess.run([str(sim),'--trial','--library-root',str(fixtures/'hero'),'--db',str(out/'slow.sqlite'),'--view','emotional','--audio-mode','silence','--smoke-test','--smoke-seconds','3','--sim-replay',str(slow),'--smoke-log',str(out/'slow-worker-journal.toml')],capture_output=True,text=True,timeout=20,env=dict(env,EMOTIONAL_ARTWORK_DELAY_MS='1200'))
(out/'slow-worker.stdout').write_text(p.stdout);(out/'slow-worker.stderr').write_text(p.stderr);assert p.returncode==0,p.stderr
assert 'raw-input replay steps passed' in p.stdout
report.append(dict(case='controls-during-1200ms-worker-delay',passed=True,scope='Injected preparation delay; not a measured microSD'))
for png in out.glob('*.png'):verify_png(png)
(out/'report.json').write_text(json.dumps(dict(passed=True,scope='Actual 640x480 opaque RGB Slint software render; procedural owned fixtures; no hardware measurement',cases=report),indent=2)+'\n')
