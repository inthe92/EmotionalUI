#!/usr/bin/env python3
"""Process launch through actual GUI; isolated text fixtures, never commercial ROMs."""
import hashlib,json,os,sqlite3,subprocess,sys,threading,time,tomllib
from pathlib import Path
SIM=Path(sys.argv[1]).resolve();OUT=Path(sys.argv[2]).resolve();OUT.mkdir(parents=True,exist_ok=False);checks=[];completed=False

def scenario(path,steps):
 text='keyboard=true\n[mapping]\n'
 for step in steps:text+='\n[[steps]]\n'+''.join(f'{k}={json.dumps(v)}\n' for k,v in step.items())
 path.write_text(text)

def case(name,behavior='normal',clock=True,cancel=False,missing=False,unconfigured=False,silence=False,concurrent=False):
 root=OUT/name;root.mkdir();roms=root/'library';roms.mkdir();rom=roms/'Faro ; $(not executed).fixture';rom.write_text('Original synthetic data, not a ROM.');save=roms/'progress.sav';save.write_bytes(b'Keep this exact synthetic save.');savehash=hashlib.sha256(save.read_bytes()).hexdigest()
 xml=roms/'gamelist.xml';xml.write_text('<gameList><game><path>'+rom.name+'</path><name>Faro de prueba</name><command>touch INJECTED</command></game></gameList>');xmlhash=hashlib.sha256(xml.read_bytes()).hexdigest();db=root/'catalog.sqlite'
 p=subprocess.run([str(SIM),'--db',str(db),'--import-gamelist',str(xml),'--rom-root',str(roms)],capture_output=True,text=True,timeout=10);assert p.returncode==0,p.stderr
 game=root/'controlled_game.py';game.write_text('''import sys,time,json,os\nfrom pathlib import Path\nrom=Path(sys.argv[1]);out=Path(sys.argv[2]);behavior=sys.argv[3]\nout.write_text(json.dumps({'argv':sys.argv[1:],'pid':os.getpid(),'parent_pid':os.getppid(),'cwd':os.getcwd(),'read_original':rom.read_text()}))\nif behavior=='empty-return':rom.unlink()\nif behavior=='root-missing-return':rom.parent.rename(rom.parent.with_name('unmounted'))\nif behavior!='early':time.sleep(2.12)\nraise SystemExit(7 if behavior=='error' else 0)\n''');log=root/'game.json'
 profile=root/'launcher.toml';authorized=roms
 if unconfigured:authorized=root/'other';authorized.mkdir()
 exe='/usr/bin/python3';args=[str(game),'{rom}',str(log),behavior]
 if behavior=='spawn-failure':
  broken=root/'broken-executable';broken.write_text('not an executable format');broken.chmod(0o700);exe=str(broken);args=['{rom}']
 profile.write_text('handoff="process-exit"\nclock_confirmed='+json.dumps(clock)+'\n[[launchers]]\nid="controlled-fixture"\nroot='+json.dumps(str(authorized))+'\nexecutable='+json.dumps(exe)+'\nworking_directory='+json.dumps(str(root))+'\narguments='+json.dumps(args)+'\nwaits_for_exit=true\n')
 initial=root/'initial.toml';steps=[dict(at_ms=1000,text='z',pressed=True,launch_pending=True),dict(at_ms=1000,text='z',pressed=False),dict(at_ms=1000,text='z',pressed=True,launch_pending=True),dict(at_ms=1000,text='z',pressed=False)]
 if cancel:steps+=[dict(at_ms=1000,text='x',pressed=True,launch_pending=False),dict(at_ms=1000,text='x',pressed=False)]
 scenario(initial,steps);back=root/'return.toml';success=behavior in ('normal','empty-return','root-missing-return') and not(missing or unconfigured or cancel)
 expected_recent=success and behavior not in ('empty-return','root-missing-return')
 scenario(back,[dict(at_ms=800,resume=True,has_recent=expected_recent,selected=0 if expected_recent else 2,audio_mode='Silencio' if silence else 'Minimalista'),dict(at_ms=850,text='e',pressed=True,resume=False),dict(at_ms=860,text='e',pressed=False),dict(at_ms=900,text='a',pressed=True,library=True),dict(at_ms=910,text='a',pressed=False),dict(at_ms=950,text='x',pressed=True,library=False),dict(at_ms=960,text='x',pressed=False)])
 command=[str(SIM),'--launcher-config',str(profile),'--db',str(db),'--library-root',str(roms),'--view','resume','--demo-history','--smoke-test','--smoke-seconds','2','--sim-replay',str(initial),'--sim-return-replay',str(back)]
 if silence:command+=['--audio-mode','silence']
 if not cancel:command+=['--snapshot',str(root/'returned.png')]
 timer=None
 if missing:timer=threading.Timer(.7,rom.unlink);timer.start()
 start=time.monotonic();process=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 if concurrent:
  deadline=time.monotonic()+6
  while not log.exists():
   assert time.monotonic()<deadline;time.sleep(.02)
  if Path('/proc').exists():
   def sample():
    stat=Path(f'/proc/{process.pid}/stat').read_text().split();status=Path(f'/proc/{process.pid}/status').read_text();rss=int(next(s for s in status.splitlines() if s.startswith('VmRSS:')).split()[1]);return int(stat[13])+int(stat[14]),rss
   ticks,rss=sample();t=time.monotonic();time.sleep(.25);end,endrss=sample();dt=time.monotonic()-t
   (root/'supervisor-performance.json').write_text(json.dumps(dict(scope='Docker ARM64 parent while synthetic child runs; frontend process exited',rss_kib=max(rss,endrss),cpu_percent_one_core=100*(end-ticks)/os.sysconf('SC_CLK_TCK')/dt,sample_seconds=dt),indent=2)+'\n')
  other=subprocess.run(command,capture_output=True,text=True,timeout=5);assert other.returncode!=0 and 'supervisor already active' in other.stderr
  (root/'concurrent.stderr').write_text(other.stderr)
 stdout,stderr=process.communicate(timeout=18)
 if timer:timer.join()
 (root/'stdout.log').write_text(stdout);(root/'stderr.log').write_text(stderr);assert process.returncode==0,(name,stderr)
 with sqlite3.connect(db) as con:
  rows=con.execute('SELECT data FROM launch_log').fetchall();history=con.execute('SELECT * FROM activity').fetchall();completed_games=con.execute('SELECT * FROM progress').fetchall()
 if cancel:assert not rows and not log.exists() and not history
 else:
  assert len(rows)==1,'Double confirmation launched more than once';record=tomllib.loads(rows[0][0]);assert record['outcome']==('exited-success' if success else 'spawn-error' if missing or unconfigured or behavior=='spawn-failure' else 'exit-error' if behavior=='error' else 'early-exit'),record
  assert len(history)==int(success and clock)
  if success:assert record['elapsed_ms']>=2000
  if log.exists():
   data=json.loads(log.read_text());assert data['argv'][0]==str(rom) and data['cwd']==str(root) and data['parent_pid']==process.pid
   assert stdout.index('FRONTEND exited')<stdout.index('LAUNCH started')
  assert 'FRONTEND exited' in stdout and 'SIMULATOR:' in stdout
 if behavior=='root-missing-return':save=root/'unmounted'/save.name;xml=root/'unmounted'/xml.name
 assert not completed_games and hashlib.sha256(save.read_bytes()).hexdigest()==savehash and hashlib.sha256(xml.read_bytes()).hexdigest()==xmlhash
 assert not (root/'INJECTED').exists() and not (roms/'INJECTED').exists()
 checks.append(dict(case=name,passed=True,elapsed_seconds=time.monotonic()-start,physical_game=False))
try:
 case('normal-silence',silence=True,concurrent=True)
 case('unknown-clock',clock=False)
 case('exit-error',behavior='error')
 case('early-exit',behavior='early')
 case('spawn-failure',behavior='spawn-failure')
 case('missing-rom',missing=True)
 case('no-authorized-launcher',unconfigured=True)
 case('cancel-before-launch',cancel=True)
 case('empty-library-return',behavior='empty-return')
 case('missing-root-return',behavior='root-missing-return')
 completed=True
finally:
 (OUT/'summary.json').write_text(json.dumps(dict(passed=completed,checks=checks,scope='Actual frontend -> isolated synthetic child -> new frontend; no emulator, DRM or physical input validation'),indent=2)+'\n')
