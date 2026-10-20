#!/usr/bin/env python3
"""Userspace large-library probe. Owned hardlinked text fixtures, never ROMs."""
import json,os,re,sqlite3,subprocess,sys,time,tomllib
from pathlib import Path
SIM=Path(sys.argv[1]).resolve();OUT=Path(sys.argv[2]).resolve();OUT.mkdir(parents=True,exist_ok=False);ROOT=Path(__file__).resolve().parents[2];checks=[];cases=[];completed=False

def run(n,name,db,roms,replay):
 journal=OUT/f'{n}-{name}.toml';cmd=[str(SIM),'--db',str(db),'--library-root',str(roms),'--atlas',str(ROOT/'assets/atlas/emotional-atlas.db'),'--at','2026-10-18T20:00','--view','emotional','--audio-mode','silence','--smoke-test','--smoke-seconds','1','--smoke-log',str(journal),'--sim-replay',str(replay)]
 start=time.monotonic();p=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);rss=0
 while p.poll() is None:
  if time.monotonic()-start>35:p.kill();p.communicate();raise RuntimeError('Large-library deadline')
  try:
   status=Path(f'/proc/{p.pid}/status').read_text();line=next((s for s in status.splitlines() if s.startswith('VmRSS:')),None)
   if line:rss=max(rss,int(line.split()[1]))
  except (FileNotFoundError,ProcessLookupError):pass
  time.sleep(.02)
 stdout,stderr=p.communicate();(OUT/f'{n}-{name}.stdout').write_text(stdout);(OUT/f'{n}-{name}.stderr').write_text(stderr);assert p.returncode==0,stderr
 assert ('cache_hit=true' if name=='warm' else 'cache_hit=false') in stdout
 events=tomllib.loads(journal.read_text())['events'];alive=next(e['elapsed_ms'] for e in events if e['event']=='event_loop_alive')
 late=[max(0,int(b)-int(a)) for a,b in re.findall(r'PAN_SAMPLE scheduled_ms=(\d+) elapsed_ms=(\d+)',stdout)]
 return dict(event_loop_alive_ms=alive,process_wall_seconds=time.monotonic()-start,peak_sampled_rss_kib=rss or None,nav_timer_lateness_max_ms=max(late,default=0),nav_press_count=8,cache_hit=name=='warm')
try:
 for n in (100,1000,5000,10000):
  base=OUT/str(n);base.mkdir();roms=base/'library';roms.mkdir();seed=roms/'source.fixture';seed.write_text('Original benchmark text, not game content.');db=base/'user.sqlite'
  p=subprocess.run([str(SIM),'--db',str(db),'--history'],capture_output=True,text=True,timeout=10);assert p.returncode==0,p.stderr
  rows=[]
  for i in range(n):
   f=roms/f'{i:06}.fixture';os.link(seed,f);rows.append((f'bench-{i:06}',str(f),str(base/'not-an-original-gamelist.xml'),f'name="Synthetic {i:06}"\n'))
  start=time.monotonic()
  with sqlite3.connect(db) as c:
   c.executemany('INSERT INTO imported_games VALUES(?,?,?,?)',rows);c.executemany('INSERT INTO atlas_links VALUES(?,?,?)',[(r[0],f'atlas:snes:{i%52+1:06}','synthetic-performance-fixture') for i,r in enumerate(rows)])
  import_ms=(time.monotonic()-start)*1000
  replay=base/'navigation.toml';steps=[]
  for i in range(8):steps+=[dict(at_ms=400+i*40,text='down' if i%2==0 else 'up',pressed=True,selected=2 if i%2==0 else 0,resume=False),dict(at_ms=410+i*40,text='down' if i%2==0 else 'up',pressed=False)]
  replay.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in s.items()) for s in steps))
  cold=run(n,'cold',db,roms,replay);warm=run(n,'warm',db,roms,replay)
  # Availability change invalidates the pool without deleting a library entry or content.
  with sqlite3.connect(db) as c:c.execute('UPDATE imported_games SET rom_path=? WHERE id=?',(str(base/'not-present.fixture'),rows[-1][0]))
  changed=run(n,'updated',db,roms,replay)
  cases.append(dict(games=n,fixture_sqlite_insert_ms=import_ms,initial_profile_select_queries_expected=53,updated_profile_select_queries_expected=53,first_start=cold,cached_start=warm,library_changed=changed))
  checks.append(dict(case=f'library-{n}',passed=True))
 completed=True
finally:
 (OUT/'report.json').write_text(json.dumps(dict(passed=completed,checks=checks,cases=cases,scope='Docker userspace software render; cold means no app cache, OS caches uncontrolled; journal includes intentional 250 ms probe; timer lateness is not input-to-photon. Synthetic identity links do not identify real ROMs.'),indent=2)+'\n')
