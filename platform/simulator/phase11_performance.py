#!/usr/bin/env python3
"""Measured progressive XML import in real software GUI. Fixtures are text, not ROMs."""
import hashlib,json,os,sqlite3,subprocess,sys,time,tomllib
from pathlib import Path
SIM=Path(sys.argv[1]);OUT=Path(sys.argv[2]);OUT.mkdir(exist_ok=False)
def run(directory,n,name):
 journal=directory/(name+'-journal.toml');out=directory/(name+'.stdout');err=directory/(name+'.stderr');replay=directory/'navigation.toml'
 start=time.monotonic();peak=0;ready=None
 with out.open('w') as so,err.open('w') as se:
  p=subprocess.Popen([str(SIM),'--trial','--db',str(directory/'own.sqlite'),'--library-root',str(directory/'library'),'--audio-mode','silence','--view','emotional','--at','2026-10-18T20:00','--smoke-test','--smoke-seconds','5','--smoke-log',str(journal),'--sim-replay',str(replay)],stdout=so,stderr=se)
  try:
   while p.poll() is None:
    if time.monotonic()-start>20:raise RuntimeError('Bounded trial import deadline')
    if ready is None and f'TRIAL catalog_ready games={n}' in out.read_text():ready=(time.monotonic()-start)*1000
    try:
     text=Path(f'/proc/{p.pid}/status').read_text();line=next((s for s in text.splitlines() if s.startswith('VmRSS:')),None)
     if line:peak=max(peak,int(line.split()[1]))
    except FileNotFoundError:pass
    time.sleep(.02)
   if p.returncode:raise RuntimeError(err.read_text())
  finally:
   if p.poll() is None:p.kill();p.wait()
 if ready is None:raise RuntimeError('Catalog not completed before safe exit')
 events=tomllib.loads(journal.read_text())['events'];alive=next(e['elapsed_ms'] for e in events if e['event']=='event_loop_alive')
 with sqlite3.connect(directory/'own.sqlite')as db:assert db.execute('SELECT count(*) FROM imported_games').fetchone()[0]==n
 text=out.read_text();assert 'raw-input replay steps passed' in text
 late=[]
 for line in text.splitlines():
  if line.startswith('PAN_SAMPLE '):
   v=dict(x.split('=',1) for x in line.split()[1:]);late.append(max(0,int(v['elapsed_ms'])-int(v['scheduled_ms'])))
 return dict(event_loop_alive_ms=alive,catalog_observed_ms=round(ready,3),rss_peak_sampled_kib=peak,xml_cache_hit='xml_unchanged=true' in text,nav_timer_lateness_max_ms=max(late,default=None),nav_samples=len(late))
results=[]
try:
 for n in [100,1000,5000,10000]:
  folder=OUT/str(n);roms=folder/'library/snes';roms.mkdir(parents=True)
  fixture=roms/'game-00000.fixture';fixture.write_text('Original owned text fixture, not a ROM.')
  for i in range(1,n):os.link(fixture,roms/f'game-{i:05}.fixture')
  xml=roms/'gamelist.xml';xml.write_text('<gameList>'+''.join(f'<game><path>game-{i:05}.fixture</path><name>Experiencia propia {i:05}</name></game>' for i in range(n))+'</gameList>');before=hashlib.sha256(xml.read_bytes()).hexdigest()
  steps=[]
  for j in range(8):steps.extend([dict(at_ms=100+j*140,text='a' if j%2==0 else 'x',pressed=True,library=j%2==0,resume=False),dict(at_ms=108+j*140,text='a' if j%2==0 else 'x',pressed=False)])
  (folder/'navigation.toml').write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(f'{k}={json.dumps(v)}\n' for k,v in s.items()) for s in steps))
  cold=run(folder,n,'cold');warm=run(folder,n,'warm');assert not cold['xml_cache_hit'] and warm['xml_cache_hit'];assert hashlib.sha256(xml.read_bytes()).hexdigest()==before
  results.append(dict(games=n,cold=cold,warm=warm))
finally:
 (OUT/'report.json').write_text(json.dumps(dict(scope='Single runs in ARM64 Docker. XML actually parsed cold, content-stamp reused warm. 20 ms parent observation/polling; event-loop probe adds 250 ms. Warm OS cache uncontrolled. Navigation assertions are not input-to-photon. No hardware performance claim.',passed=len(results)==4,cases=results),indent=2)+'\n')
