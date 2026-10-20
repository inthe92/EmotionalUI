#!/usr/bin/env python3
"""Actual Slint software renders and event tests. No physical display assertions."""
import hashlib,json,os,struct,subprocess,sys,time,zlib
from pathlib import Path
SIM=Path(sys.argv[1]);OUT=Path(sys.argv[2]);OUT.mkdir(parents=True,exist_ok=False)
checks=[]
def run(name,args):
    t=time.monotonic();p=subprocess.run([str(SIM),'--db',':memory:','--smoke-seconds','1',*map(str,args)],text=True,capture_output=True,timeout=40)
    (OUT/(name+'.stdout')).write_text(p.stdout);(OUT/(name+'.stderr')).write_text(p.stderr)
    checks.append(dict(case=name,passed=p.returncode==0,exit_code=p.returncode,elapsed_seconds=time.monotonic()-t));print(name,p.returncode,flush=True)
    if p.returncode:raise RuntimeError(name+': '+p.stderr)
    return p.stdout

def png(path):
    b=path.read_bytes();assert b[:8]==b'\x89PNG\r\n\x1a\n';assert struct.unpack('>II',b[16:24])==(640,480);assert b[24:26]==bytes((8,2)), 'RGB opaco requerido'
    # Validate real compressed scanlines too, not only the file extension/header.
    chunks=[];i=8
    while i<len(b):
        n=struct.unpack('>I',b[i:i+4])[0];kind=b[i+4:i+8];data=b[i+8:i+8+n]
        assert zlib.crc32(kind+data)&0xffffffff==struct.unpack('>I',b[i+8+n:i+12+n])[0]
        if kind==b'IDAT':chunks.append(data)
        i+=12+n
    raw=zlib.decompress(b''.join(chunks));assert len(raw)==480*(640*3+1)
    return hashlib.sha256(b).hexdigest()
cases=[
 ('01-resume-recent',['--view','resume','--demo-history','--at','2026-10-18T12:00']),
 ('02-resume-empty',['--view','resume','--at','2026-10-18T12:00']),
 ('03-morning',['--view','emotional','--at','2026-07-15T08:30','--weather','clear']),
 ('04-noon',['--view','emotional','--at','2026-07-15T12:00','--weather','clear']),
 ('05-afternoon',['--view','emotional','--at','2026-07-15T15:30','--weather','clear']),
 ('06-dusk',['--view','emotional','--at','2026-07-15T18:30','--weather','clear']),
 ('07-night',['--view','emotional','--at','2026-07-15T21:30','--weather','clear']),
 ('08-late',['--view','emotional','--at','2026-07-15T02:00','--weather','clear']),
 ('09-autumn-rain',['--view','emotional','--dev-moment','autumn-rain']),
 ('10-halloween',['--view','emotional','--dev-moment','halloween']),
 ('11-christmas',['--view','emotional','--dev-moment','christmas']),
 ('12-library',['--view','emotional','--snapshot-view','library','--at','2026-10-18T12:00']),
]
try:
    captures={}
    for name,args in cases:
        path=OUT/(name+'.png');run(name,[*args,'--snapshot',path]);captures[name]=png(path)
    assert len(set(captures[name] for name,_ in cases[2:8]))==6
    checks.append(dict(case='six-distinct-moments-one-master',passed=True))
    for count in (0,1,2,6):
        for view in ('resume','emotional'):
            run(f'ui-{view}-{count}',['--view',view,'--demo-count',count,'--ui-test','--at','2026-10-18T21:30'])
    run('ui-resume-recent',['--demo-history','--view','resume','--ui-test','--at','2026-10-18T21:30'])
    for case in ('long-title','no-art','empty','unknown','reduced'):
        for view in ('resume','emotional'):
            name=view+'-'+case;path=OUT/(name+'.png');run(name,['--view',view,'--demo-history','--visual-case',case,'--at','2026-10-18T12:00','--snapshot',path]);png(path)
    for concept in ('quiet','scene','editorial'):
        run('inherited-'+concept,['--concept',concept,'--ui-test','--at','2026-10-18T12:00'])
    for condition in ('fog','snow'):
        path=OUT/(condition+'.png');run(condition,['--weather',condition,'--at','2026-01-18T12:00','--snapshot',path]);png(path)
    # Replay fast direction changes and L/R within 180ms transition, no waits for animation.
    steps=[('q',True,dict(resume=True)),('q',False,{}),('e',True,dict(resume=False)),('e',False,{}),('down',True,dict(selected=2)),('down',False,{}),('z',True,dict(library=True)),('z',False,{}),('s',True,dict(menu=True)),('s',False,{}),('x',True,dict(menu=False,library=True)),('x',False,{}),('x',True,dict(library=False,selected=2)),('x',False,{})]
    replay='keyboard=true\n[mapping]\n'
    for i,(key,pressed,expected) in enumerate(steps):
        replay+='\n[[steps]]\nat_ms='+str(80+i*8)+'\ntext='+json.dumps(key)+'\npressed='+json.dumps(pressed)+'\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in expected.items())
    path=OUT/'integrated-input.toml';path.write_text(replay)
    run('interrupted-transitions',['--view','emotional','--sim-replay',path,'--smoke-test'])
    # Check privacy persistence across independent real GUI processes, without deleting user data.
    replay='keyboard=true\n[mapping]\n'
    keys=['s','down','down','down','z','x']
    for i,key in enumerate(keys):
        for j,pressed in enumerate((True,False)):
            replay+=f'\n[[steps]]\nat_ms={100+i*60+j*8}\ntext={json.dumps(key)}\npressed={json.dumps(pressed)}\n'
        if key=='z':replay+='personalized=false\n'
    path=OUT/'privacy-input.toml';path.write_text(replay)
    db=OUT/'privacy.sqlite';run('privacy-save',['--db',db,'--sim-replay',path,'--smoke-test'])
    import sqlite3
    with sqlite3.connect(db) as con:
        value=con.execute("SELECT value FROM settings WHERE key='privacy'").fetchone()[0]
        assert 'personalized = false' in value
    checks.append(dict(case='privacy-sqlite-roundtrip',passed=True))
    # Process RSS and idle CPU in this Linux VM only. No display/FPS substitution.
    if Path('/proc').exists():
        journal=OUT/'metrics.toml';start=time.monotonic();p=subprocess.Popen([str(SIM),'--db',':memory:','--at','2026-10-18T21:30','--smoke-test','--smoke-seconds','4','--smoke-log',str(journal)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            ready_deadline=time.monotonic()+3
            while not journal.exists() or 'event_loop_alive' not in journal.read_text():
                if p.poll() is not None or time.monotonic()>ready_deadline:raise RuntimeError('Metrics process not ready')
                time.sleep(.02)
            ready=time.monotonic()-start
            def sample():
                fields=Path(f'/proc/{p.pid}/stat').read_text().split();status=Path(f'/proc/{p.pid}/status').read_text();rss=int(next(s for s in status.splitlines() if s.startswith('VmRSS:')).split()[1]);return int(fields[13])+int(fields[14]),rss
            ticks,rss=sample();t=time.monotonic();time.sleep(2);end_ticks,end_rss=sample();interval=time.monotonic()-t
            stdout,stderr=p.communicate(timeout=5)
            assert p.returncode==0
            import tomllib
            events=tomllib.loads(journal.read_text())['events']
            metrics=dict(scope='Docker ARM64 software simulator, not RG353V',sample_seconds=interval,rss_kib=max(rss,end_rss),idle_cpu_percent_one_core=100*(end_ticks-ticks)/os.sysconf('SC_CLK_TCK')/interval,process_to_probe_seconds=ready,probe_delay_ms=250,window_created_ms=next(e['elapsed_ms'] for e in events if e['event']=='window_created'),rasterized_frames=int(stdout.split('rasterized_frames=')[1].split(';')[0]),presented_fps=None,input_to_photon_ms=None)
            (OUT/'performance.json').write_text(json.dumps(metrics,indent=2)+'\n');(OUT/'metrics.stdout').write_text(stdout);(OUT/'metrics.stderr').write_text(stderr)
        finally:
            if p.poll() is None:p.kill();p.communicate()
    (OUT/'captures.json').write_text(json.dumps(captures,indent=2)+'\n')
finally:
    (OUT/'summary.json').write_text(json.dumps(dict(checks=checks,passed=all(c['passed'] for c in checks),physical_hardware_tested=False),indent=2)+'\n')
