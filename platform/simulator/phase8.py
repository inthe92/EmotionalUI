#!/usr/bin/env python3
"""Actual software-rendered Slint interaction checks; never hardware audio/FPS."""
import json, os, re, sqlite3, struct, subprocess, sys, time, tomllib
from pathlib import Path
SIM=Path(sys.argv[1]).resolve(); OUT=Path(sys.argv[2]).resolve(); OUT.mkdir(parents=True,exist_ok=False)
checks=[]
completed=False
def run(name,args):
    t=time.monotonic()
    p=subprocess.run([str(SIM),'--db',':memory:','--at','2026-10-18T18:30','--smoke-seconds','2',*map(str,args)],text=True,capture_output=True,timeout=12)
    (OUT/(name+'.stdout')).write_text(p.stdout);(OUT/(name+'.stderr')).write_text(p.stderr)
    checks.append(dict(case=name,passed=p.returncode==0,elapsed_seconds=time.monotonic()-t))
    if p.returncode: raise RuntimeError(name+': '+p.stderr)
    return p.stdout

def replay(name,steps,args=(),capture=False):
    text='keyboard=true\n[mapping]\n'
    for step in steps:
        text+='\n[[steps]]\n'+''.join(f'{k}={json.dumps(v)}\n' for k,v in step.items())
    path=OUT/(name+'.toml');path.write_text(text)
    extra=['--snapshot',OUT/(name+'.png')] if capture else ['--smoke-test']
    out=run(name,[*args,'--sim-replay',path,*extra])
    if capture:
        data=(OUT/(name+'.png')).read_bytes()
        assert data[:8]==b'\x89PNG\r\n\x1a\n' and struct.unpack('>II',data[16:24])==(640,480) and data[24:26]==bytes((8,2))
    return out

def keys(sequence,start=100,interval=60):
    result=[]
    for i,(key,expected) in enumerate(sequence):
        result.extend([dict(at_ms=start+i*interval,text=key,pressed=True,**expected),dict(at_ms=start+i*interval+8,text=key,pressed=False)])
    return result
try:
    for view in ('resume','emotional'):
        replay(view,[dict(at_ms=400,resume=view=='resume')],['--view',view,'--demo-history'],True)
    steps=[dict(at_ms=400,pan_min=-3.01,pan_max=-2.99),*keys([('e',dict(resume=False))],500)]
    for t in range(520,841,20):steps.append(dict(at_ms=t,pan_min=-3.01,pan_max=3.01))
    steps.extend(keys([('down',dict(selected=2)),('z',dict(library=True)),('x',dict(library=False,selected=2))],520,40))
    steps.extend(keys([('q',dict(resume=True)),('e',dict(resume=False,selected=2))],900,30))
    # Warm up first paint and the 250ms readiness probe before measuring motion.
    # Startup input is intentionally direct; it must never wait for an intro.
    for step in steps: step['at_ms']+=500
    steps.sort(key=lambda step:step['at_ms'])
    out=replay('pan-and-interrupted-input',steps,['--view','resume','--demo-history'])
    samples=[dict(scheduled_ms=int(a),elapsed_ms=int(b),position=float(c)) for a,b,c in re.findall(r'PAN_SAMPLE scheduled_ms=(\d+) elapsed_ms=(\d+) position=([-\d.]+)',out)]
    movement=[s for s in samples if s['scheduled_ms']>=1020]
    assert any(-2.9<s['position']<2.9 for s in movement),'No interpolated pan observed'
    settled=next(s for s in movement if s['position']>=2.99)
    changed=next(s for s in samples if s['scheduled_ms']==1000)
    assert 180<=settled['elapsed_ms']-changed['elapsed_ms']<=600, 'Actual callback time, not backlog of scheduled samples' 
    (OUT/'pan-timing.json').write_text(json.dumps(dict(scope='Slint property sampling in software event loop; not LCD presentation',declared_ms=240,samples=samples,first_observed_settled_ms=settled['elapsed_ms'],target_change_observed_ms=changed['elapsed_ms'],observed_settling_upper_ms=settled['elapsed_ms']-changed['elapsed_ms']),indent=2)+'\n')
    replay('reduced-pan',keys([('m',dict(reduced=True)),('e',dict(resume=False,pan_min=0,pan_max=0)),('q',dict(resume=True,pan_min=0,pan_max=0)),('e',dict(resume=False,pan_min=0,pan_max=0))],400,40),['--view','resume','--demo-history'],True)
    replay('ambient-motion-off',keys([('s',dict(menu=True)),* [('down',{})]*5,('z',{}),('x',dict(menu=False)),('e',dict(resume=False,pan_min=0,pan_max=0))]),['--view','resume'])
    for view in ('resume','emotional'):
        replay('empty-'+view,keys([('e',dict(resume=view=='emotional')),('q',dict(resume=view=='resume')),('a',dict(library=True)),('x',dict(library=False))]),['--view',view,'--demo-count','0'])
    db=OUT/'audio.sqlite'
    replay('audio-settings',keys([('s',dict(menu=True)),('down',{}),('z',dict(audio_open=True,audio_mode='Minimalista')),('right',dict(audio_mode='Ambiental')),('down',{}),('right',dict(audio_master=45))]),['--db',db],True)
    with sqlite3.connect(db) as con: prefs=tomllib.loads(con.execute("SELECT value FROM settings WHERE key='ambient_audio'").fetchone()[0])
    assert prefs['mode']=='ambient' and prefs['master']==45
    replay('audio-reload-return-focus',keys([('s',dict(menu=True)),('down',dict(menu_index=1)),('z',dict(audio_open=True,audio_mode='Ambiental',audio_master=45)),('right',dict(audio_mode='Silencio')),('x',dict(audio_open=False,menu=True,menu_index=1)),('x',dict(menu=False))]),['--db',db])
    replay('silence-persisted',[dict(at_ms=400,audio_mode='Silencio',audio_master=45)],['--db',db])
    replay('audio-fast-exit-and-reduced',keys([('s',dict(menu=True)),('down',{}),('z',dict(audio_open=True)),('m',dict(reduced=True,pan_min=0,pan_max=0)),('e',dict(resume=False,audio_open=False,menu=False,pan_min=0,pan_max=0)),('q',dict(resume=True,pan_min=0,pan_max=0)),('s',dict(menu=True,menu_index=1)),('z',dict(audio_open=True)),('x',dict(audio_open=False,menu=True,menu_index=1)),('x',dict(menu=False))],350,35),['--view','resume','--demo-history'])
    replay('missing-assets',keys([('e',dict(resume=False)),('down',dict(selected=2)),('z',dict(library=True))]),['--view','resume','--audio-assets',OUT/'absent','--audio-mode','ambient','--audio-volume','0'],True)
    # Native API fallback in the unprivileged runtime, without /dev/snd exposure.
    probe=SIM.parent/'audio-probe'
    if probe.exists():
        for flag,name in [([], 'audio-lifecycle'),(['--native-silent'],'native-zero-device')]:
            p=subprocess.run([str(probe),*flag],text=True,capture_output=True,timeout=10)
            (OUT/(name+'.json')).write_text(p.stdout);(OUT/(name+'.stderr')).write_text(p.stderr)
            assert p.returncode==0
            checks.append(dict(case=name,passed=True))
    if Path('/proc').exists():
        metrics=[]
        for mode in ('silence','minimal'):
            p=subprocess.Popen([str(SIM),'--db',':memory:','--audio-mode',mode,'--smoke-test','--smoke-seconds','4'],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            def sample():
                fields=Path(f'/proc/{p.pid}/stat').read_text().split();status=Path(f'/proc/{p.pid}/status').read_text();rss=int(next(s for s in status.splitlines() if s.startswith('VmRSS:')).split()[1]);return int(fields[13])+int(fields[14]),rss
            try:
                time.sleep(1);tick,rss=sample();t=time.monotonic();time.sleep(2);last,endrss=sample();dt=time.monotonic()-t
                stdout,stderr=p.communicate(timeout=5); assert p.returncode==0
                metrics.append(dict(mode=mode,sample_seconds=dt,rss_kib=max(rss,endrss),cpu_percent_one_core=100*(last-tick)/os.sysconf('SC_CLK_TCK')/dt,rasterized_frames=int(stdout.split('rasterized_frames=')[1].split(';')[0])))
            finally:
                if p.poll() is None:p.kill();p.communicate()
        (OUT/'idle-performance.json').write_text(json.dumps(dict(scope='Docker ARM64 software simulator; Minimal before first phrase; no physical audio device',metrics=metrics,presented_fps=None,input_to_photon_ms=None),indent=2)+'\n')
    completed=True
finally:
    (OUT/'summary.json').write_text(json.dumps(dict(checks=checks,passed=completed and bool(checks) and all(c['passed'] for c in checks),physical_hardware_tested=False),indent=2)+'\n')
