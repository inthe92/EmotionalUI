#!/usr/bin/env python3
"""Actual UI + SQLite using synthetic evidence, never real play or a human review."""
import copy,hashlib,json,sqlite3,struct,subprocess,sys,time,tomllib
from pathlib import Path
SIM=Path(sys.argv[1]).resolve();OUT=Path(sys.argv[2]).resolve();OUT.mkdir(parents=True,exist_ok=False)
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools'));import atlas
checks=[];completed=False

def run(name,args):
 t=time.monotonic();p=subprocess.run([str(SIM),*map(str,args)],capture_output=True,text=True,timeout=20)
 (OUT/(name+'.stdout')).write_text(p.stdout);(OUT/(name+'.stderr')).write_text(p.stderr);assert p.returncode==0,(name,p.stderr)
 checks.append(dict(case=name,passed=True,elapsed_seconds=time.monotonic()-t));return p.stdout

def scenario(name,steps):
 p=OUT/(name+'.toml');p.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in s.items()) for s in steps));return p

def keys(actions,start=400,step=70):
 out=[]
 for i,(text,expected) in enumerate(actions):out.extend([dict(at_ms=start+i*step,text=text,pressed=True,**expected),dict(at_ms=start+i*step+10,text=text,pressed=False)])
 return out
try:
 roms=OUT/'library';roms.mkdir();xml=roms/'gamelist.xml';xml.write_text('<gameList>'+''.join(f'<game><path>fiction-{i}.fixture</path><name>Partida {i+1}</name></game>' for i in range(3))+'</gameList>')
 for i in range(3):(roms/f'fiction-{i}.fixture').write_text('Original synthetic fixture, not a ROM.')
 original={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in roms.iterdir()};db=OUT/'user.sqlite';run('import',['--import-gamelist',xml,'--rom-root',roms,'--db',db])
 with sqlite3.connect(db) as c:ids=[r[0] for r in c.execute('SELECT id FROM imported_games ORDER BY id')]
 with sqlite3.connect(db) as c:
  for i,game_id in enumerate(ids):
   started=1792317600+i*1000;duration=[3600,1800,300][i]
   data=f'id="fixture-{i}"\ngame_id='+json.dumps(game_id)+f'\nprofile="synthetic-evidence"\nstarted_utc={started}\nelapsed_ms={duration*1000}\noutcome="exited-success"\nexit_code=0\nsession_registered=true\n'
   c.execute('INSERT INTO launch_log VALUES(?,?,?,?)',(f'fixture-{i}',game_id,'exited-success',data));c.execute('INSERT INTO activity VALUES(?,?,?)',(game_id,started,duration))
 fixture=atlas.load(ROOT/'assets/atlas/source/snes-pilot.json');fixture['version']='0.0.1-synthetic-fixture';fixture['games']=fixture['games'][:3];fixture['relations']=[];fixture['sources']={'fixture':dict(scope='Synthetic test metadata, not a published human review')}
 for i,g in enumerate(fixture['games']):
  for a in g['objective'].values():
   if a is not None:a['source']='fixture'
  g['editorial']={'calm':dict(value=25+i*35,confidence=100,source='fixture',status='approved',origin='test_fixture',reviewer='synthetic-test-not-a-person')}
 adb=OUT/'fixture-atlas.db';atlas.build(fixture,adb);ahash=hashlib.sha256(adb.read_bytes()).hexdigest()
 for local,g in zip(ids,fixture['games']):run('link-'+g['id'][-1],['--db',db,'--atlas',adb,'--atlas-link',local+'='+g['id']])
 base=['--db',db,'--library-root',roms,'--atlas',adb,'--at','2026-10-18T20:00','--view','resume','--audio-mode','silence','--smoke-test','--smoke-seconds','3']
 p=scenario('three-initial',[dict(at_ms=400,resume_count=3,has_recent=True,selected=0)])
 first=run('three-initial',[*base,'--sim-replay',p,'--snapshot',OUT/'three-initial.png','--smoke-log',OUT/'cold.toml']);assert 'cache_hit=false' in first
 with sqlite3.connect(db) as c:cached=c.execute("SELECT value FROM settings WHERE key='discovery_reserve'").fetchone()[0]
 second=run('reboot-stability',[*base,'--snapshot',OUT/'reboot.png','--smoke-log',OUT/'warm.toml']);assert 'cache_hit=true' in second
 with sqlite3.connect(db) as c:assert cached==c.execute("SELECT value FROM settings WHERE key='discovery_reserve'").fetchone()[0]
 run('game-options-display',[*base,'--sim-replay',scenario('game-options-display',keys([('d',dict(game_options=True)),('down',{}),('down',{}),('down',{}),('down',dict(game_option_index=4)),('z',dict(game_reason_contains='abierto'))])),'--snapshot',OUT/'game-options.png'])
 acts=[('down',dict(selected=4)),('right',dict(selected=5)),('d',dict(game_options=True)),('z',dict(resume_count=3,selected=0)),('x',dict(game_options=False)),('d',dict(game_options=True)),('down',dict(game_option_index=1)),('z',dict(resume_count=2,selected=2)),('x',dict(game_options=False)),('a',dict(library=True)),* [('down',dict(library_index=i)) for i in range(1,3)],('d',dict(game_options=True)),('down',dict(game_option_index=1)),('z',dict(resume_count=3)),('x',dict(game_options=False))]
 run('pin-remove-restore',[*base,'--sim-replay',scenario('manual',keys(acts)),'--snapshot',OUT/'manual.png'])
 with sqlite3.connect(db) as c:
  prefs={id:tomllib.loads(data) for id,data in c.execute('SELECT game_id,data FROM continuity_preferences')};assert prefs[ids[2]]['pinned'] and not prefs[ids[2]]['removed']
 run('restart-keeps-pin',[*base,'--sim-replay',scenario('pinned-check',[dict(at_ms=400,resume_count=3)]),'--snapshot',OUT/'pinned.png'])
 # Physical button path: Start -> last menu row -> game options, B recovers same row.
 actions=[('s',dict(menu=True)),*[('down',dict(menu_index=i)) for i in range(1,10)],('z',dict(game_options=True)),('down',{}),('down',dict(game_option_index=2)),('z',dict(resume_count=2)),('x',dict(game_options=False,menu=True,menu_index=9)),('x',dict(menu=False))]
 run('complete-start-and-back',[*base,'--sim-replay',scenario('completed',keys(actions)),'--snapshot',OUT/'completed.png'])
 with sqlite3.connect(db) as c:assert c.execute('SELECT completed FROM progress WHERE game_id=?',(ids[2],)).fetchone()[0]==1
 actions=[('down',dict(selected=4)),('d',dict(game_options=True)),('down',{}),('down',{}),('down',dict(game_option_index=3)),('z',dict(selected=0)),('x',dict(game_options=False))]
 run('explicit-preference',[*base,'--sim-replay',scenario('affinity-positive',keys(actions))])
 with sqlite3.connect(db) as c:assert tomllib.loads(c.execute('SELECT data FROM continuity_preferences WHERE game_id=?',(ids[1],)).fetchone()[0])['affinity']==1
 actions=[('d',dict(game_options=True)),('down',{}),('down',{}),('down',dict(game_option_index=3)),('z',{}),('z',{}),('x',dict(game_options=False))]
 run('correct-preference',[*base,'--sim-replay',scenario('affinity-neutral',keys(actions))])
 with sqlite3.connect(db) as c:assert tomllib.loads(c.execute('SELECT data FROM continuity_preferences WHERE game_id=?',(ids[1],)).fetchone()[0])['affinity']==0
 for name,args in [('unknown',['--unknown-time']),('empty',['--demo-count','0']),('reduced',['--reduced-motion'])]:run(name,[*base,*args,'--snapshot',OUT/(name+'.png')])
 # The second continuity slot uses the existing supervisor; controlled process only.
 game=OUT/'controlled-game.py';launched=OUT/'launched.json';game.write_text("import json,sys,time\nfrom pathlib import Path\nPath(sys.argv[2]).write_text(json.dumps({'rom':sys.argv[1]}))\ntime.sleep(2.12)\n")
 profile=OUT/'launcher.toml';profile.write_text('handoff="process-exit"\nclock_confirmed=false\n[[launchers]]\nid="fixture"\nroot='+json.dumps(str(roms))+'\nexecutable="/usr/bin/python3"\nworking_directory='+json.dumps(str(OUT))+'\narguments='+json.dumps([str(game),'{rom}',str(launched)])+'\nwaits_for_exit=true\n')
 initial=scenario('secondary-launch',keys([('down',dict(selected=4)),('z',dict(launch_pending=True))],800,100))
 back=scenario('secondary-return',[dict(at_ms=600,resume=True,selected=4,resume_count=2,has_recent=True)])
 run('secondary-process-return',[*base,'--launcher-config',profile,'--sim-replay',initial,'--sim-return-replay',back,'--snapshot',OUT/'secondary-return.png'])
 with sqlite3.connect(db) as c:
  record=tomllib.loads(c.execute('SELECT data FROM launch_log ORDER BY rowid DESC LIMIT 1').fetchone()[0]);expected=c.execute('SELECT rom_path FROM imported_games WHERE id=?',(ids[1],)).fetchone()[0]
  assert record['game_id']==ids[1] and record['outcome']=='exited-success' and not record['session_registered'];assert json.loads(launched.read_text())['rom']==expected
 # Sources are identical; no commercial content introduced or source metadata rewritten.
 assert original=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in roms.iterdir()};assert hashlib.sha256(adb.read_bytes()).hexdigest()==ahash
 for p in OUT.glob('*.png'):
  b=p.read_bytes();assert struct.unpack('>II',b[16:24])==(640,480) and b[24:26]==bytes((8,2))
 starts={n:next(e['elapsed_ms'] for e in tomllib.loads((OUT/(n+'.toml')).read_text())['events'] if e['event']=='event_loop_alive') for n in ('cold','warm')}
 (OUT/'startup.json').write_text(json.dumps(dict(scope='Simulator journal from process_start to event_loop_alive, includes intentional 250 ms probe delay, warm OS caches, not power-on',**starts),indent=2)+'\n')
 completed=True
finally:(OUT/'summary.json').write_text(json.dumps(dict(passed=completed,checks=checks,scope='Synthetic histories and editorial metadata, actual Slint/SQLite; no physical games or human quality review'),indent=2)+'\n')
