#!/usr/bin/env python3
"""Actual Slint trial renders; owned text fixtures, no ROM/emulator/hardware claims."""
import hashlib,json,os,shutil,sqlite3,struct,subprocess,sys,time,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];SIM=Path(sys.argv[1]);OUT=Path(sys.argv[2]);OUT.mkdir(exist_ok=False)
checks=[]
def replay(name,steps):
 p=OUT/(name+'.toml');p.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(f'{k}={json.dumps(v)}\n' for k,v in s.items()) for s in steps));return p
def run(name,args,expect=0):
 p=subprocess.run([str(SIM),*map(str,args)],capture_output=True,text=True,timeout=20)
 (OUT/(name+'.stdout')).write_text(p.stdout);(OUT/(name+'.stderr')).write_text(p.stderr)
 if p.returncode!=expect:raise AssertionError(name+': '+p.stderr)
 checks.append(dict(case=name,passed=True,exit_code=p.returncode));return p.stdout
passed=False
try:
 library=OUT/'library';(library/'snes').mkdir(parents=True);rom=library/'snes/owned=game.fixture';rom.write_text('Original text fixture, not a ROM.')
 art=library/'snes/cover.png';shutil.copyfile(ROOT/'assets/scenes/lofi-room-v2/master.png',art)
 xml=library/'snes/gamelist.xml';xml.write_text('<gameList><game><path>owned=game.fixture</path><name>Una isla propia</name><image>cover.png</image></game></gameList>')
 original={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in xml.parent.iterdir()}
 db=OUT/'trial.sqlite';base=['--trial','--db',db,'--library-root',library,'--view','emotional','--at','2026-10-18T20:00','--audio-mode','silence','--smoke-test','--smoke-seconds','3']
 # Keep the 900ms catalog/control check. Artwork is intentionally asynchronous:
 # assert its completion independently within this process's unchanged 3s budget.
 steps=[dict(at_ms=900,catalog_count=1),dict(at_ms=950,text='a',pressed=True,library=True),dict(at_ms=958,text='a',pressed=False),dict(at_ms=1050,text='x',pressed=True,library=False),dict(at_ms=1058,text='x',pressed=False),dict(at_ms=1150,text='up',pressed=True,selected=0),dict(at_ms=1158,text='up',pressed=False),dict(at_ms=1250,text='z',pressed=True,message_contains='Modo prueba'),dict(at_ms=1258,text='z',pressed=False)]
 run('cold',[*base,'--sim-replay',replay('cold-replay',steps),'--snapshot',OUT/'trial-real-library.png','--smoke-log',OUT/'cold-journal.toml'])
 events=tomllib.loads((OUT/'cold-journal.toml').read_text())['events']
 ready=next(e for e in events if e['event']=='local_art_ready')
 # The shared design requests exactly the visible Discover artwork bounds.
 # The source/decoded-memory limits and 3-second readiness deadline are unchanged.
 assert 0<ready['width']<=320 and 0<ready['height']<=232 and ready['elapsed_ms']<3000
 checks.append(dict(case='asynchronous-cover-ready',passed=True,elapsed_ms=ready['elapsed_ms'],deadline_ms=3000))
 with sqlite3.connect(db) as c:
  assert c.execute('SELECT count(*) FROM imported_games').fetchone()[0]==1
  assert c.execute('SELECT count(*) FROM launch_log').fetchone()[0]==0
  local=c.execute('SELECT id FROM imported_games').fetchone()[0]
 stdout=run('warm',[*base,'--snapshot',OUT/'trial-warm.png']);assert 'xml_unchanged=true' in stdout
 adb=ROOT/'assets/atlas/emotional-atlas-0.2.0-experimental.1.db'
 run('link-equals-filename',['--db',db,'--atlas',adb,'--atlas-link',local+'=atlas:snes:000001'])
 steps=[dict(at_ms=1000,catalog_count=1),dict(at_ms=1100,text='d',pressed=True,game_options=True),dict(at_ms=1108,text='d',pressed=False)]
 for i in range(4):steps.extend([dict(at_ms=1200+i*90,text='down',pressed=True),dict(at_ms=1208+i*90,text='down',pressed=False)])
 steps += [dict(at_ms=1600,text='z',pressed=True,game_reason_contains='Atlas experimental'),dict(at_ms=1608,text='z',pressed=False)]
 run('experimental-explanation',[*base,'--atlas',adb,'--atlas-experimental','--sim-replay',replay('why',steps),'--snapshot',OUT/'atlas-why.png'])
 empty=OUT/'empty';empty.mkdir()
 for name,root in [('empty',empty),('disconnected',OUT/'absent')]:
  run(name,['--trial','--db',OUT/(name+'.sqlite'),'--library-root',root,'--at','2026-10-18T20:00','--audio-mode','silence','--smoke-test','--sim-replay',replay(name+'-replay',[dict(at_ms=700,catalog_count=0,has_recent=False)]),'--snapshot',OUT/(name+'.png')])
 run('trial-rejects-demo',['--trial','--demo-count','3'],1)
 run('trial-rejects-launcher',['--trial','--launcher-config','not-a-profile'],1)
 # Metadata changes invalidate the content stamp; source files remain untouched by app.
 xml.write_text(xml.read_text().replace('Una isla propia','Isla al atardecer'));updated=hashlib.sha256(xml.read_bytes()).hexdigest()
 assert 'xml_imported=1' in run('incremental',[*base])
 assert hashlib.sha256(xml.read_bytes()).hexdigest()==updated
 assert all(hashlib.sha256(p.read_bytes()).hexdigest()==h for p,h in [(rom,original[rom.name]),(art,original[art.name])])
 art.unlink()
 run('missing-cover',[*base,'--sim-replay',replay('no-cover',[dict(at_ms=900,catalog_count=1,local_art=False)]),'--snapshot',OUT/'missing-cover.png'])
 # Preserve Resume focus by ID when a newly imported filename sorts before it.
 before_file=library/'snes/ahead.fixture';before_file.write_text('Original additional text fixture, not a ROM.')
 xml.write_text('<gameList><game><path>ahead.fixture</path><name>Otro camino</name></game><game><path>owned=game.fixture</path><name>Isla al atardecer</name></game></gameList>')
 with sqlite3.connect(db) as c:c.execute('INSERT INTO activity VALUES(?,?,?)',(local,1792346400,1200))
 resume_base=list(base);resume_base[resume_base.index('--view')+1]='resume'
 run('resume-focus-survives-insertion',[*resume_base,'--sim-replay',replay('resume-insert',[dict(at_ms=1000,catalog_count=2,resume=True,has_recent=True,resume_count=1,selected=0)]),'--snapshot',OUT/'trial-resume.png'])
 for p in OUT.glob('*.png'):
  b=p.read_bytes();assert struct.unpack('>II',b[16:24])==(640,480) and b[24:26]==bytes((8,2))
 passed=True
finally:
 (OUT/'summary.json').write_text(json.dumps(dict(passed=passed,checks=checks,scope='Actual software Slint, SQLite, original text fixtures; no ES return or emulator/hardware validation'),indent=2)+'\n')
