#!/usr/bin/env python3
"""Real Slint layout renders and section/focus replays. Never a hardware/FPS test."""
import hashlib,json,struct,subprocess,sys,zlib
from pathlib import Path
SIM=Path(sys.argv[1]).resolve();OUT=Path(sys.argv[2]).resolve();OUT.mkdir(parents=True,exist_ok=False)
checks=[]
def run(name,args):
    result=subprocess.run([str(SIM),'--db',':memory:','--at','2026-10-19T18:30','--audio-mode','silence','--smoke-seconds','3',*map(str,args)],capture_output=True,text=True,timeout=30)
    (OUT/(name+'.stdout')).write_text(result.stdout);(OUT/(name+'.stderr')).write_text(result.stderr)
    checks.append(dict(case=name,passed=result.returncode==0))
    if result.returncode:raise RuntimeError(name+': '+result.stderr)
    return result.stdout
def replay(name,steps,args=()):
    path=OUT/(name+'.toml');path.write_text('keyboard=true\n[mapping]\n'+''.join('\n[[steps]]\n'+''.join(k+'='+json.dumps(str(v) if isinstance(v,Path) else v)+'\n' for k,v in step.items()) for step in steps))
    return run(name,[*args,'--smoke-test','--sim-replay',path])
def keys(actions,start=1000,gap=24):
    steps=[]
    for i,(key,expected) in enumerate(actions):
        at=start+i*gap
        steps.extend([dict(at_ms=at,text=key,pressed=True,**expected),dict(at_ms=at+8,text=key,pressed=False)])
    return steps
def png(path):
    b=path.read_bytes();assert b[:8]==b'\x89PNG\r\n\x1a\n' and struct.unpack('>II',b[16:24])==(640,480) and b[24:26]==bytes((8,2))
    chunks=[];at=8
    while at<len(b):
        n=struct.unpack('>I',b[at:at+4])[0];kind=b[at+4:at+8];data=b[at+8:at+8+n]
        assert zlib.crc32(kind+data)&0xffffffff==struct.unpack('>I',b[at+8+n:at+12+n])[0]
        if kind==b'IDAT':chunks.append(data)
        at+=n+12
    raw=zlib.decompress(b''.join(chunks));assert len(raw)==480*(640*3+1) and len(set(raw))>80
    return hashlib.sha256(b).hexdigest()
passed=False
try:
    captures=[('continuar','resume',['--demo-history','--visual-case','resume-three']),('continuar-una','resume',['--demo-history']),('descubre','emotional',['--demo-history']),('continuar-vacio','resume',['--demo-count','0']),('descubre-vacio','emotional',['--demo-count','0']),('hora-desconocida','emotional',['--unknown-time'])]
    for view in ('resume','emotional'):
        for case in ('long-title','no-art'):
            captures.append((view+'-'+case,view,['--demo-history','--visual-case',case]))
    for name,view,args in captures:
        run(name,['--view',view,*args,'--reduced-motion','--smoke-seconds','1','--snapshot',OUT/(name+'.png')])
    for view in ('resume','emotional'):
        run('ui-'+view,['--view',view,'--demo-history','--ui-test'])
    # Visible secondary and dock focus, using real button-equivalent events.
    replay('foco-partida',keys([('down',dict(selected=4)),('right',dict(selected=5))]),['--view','resume','--demo-history','--visual-case','resume-three','--snapshot',OUT/'foco-partida.png'])
    replay('foco-dock',keys([('down',dict(selected=1)),('down',dict(dock_cursor=2))]),['--view','emotional','--demo-history','--snapshot',OUT/'foco-dock.png'])
    # Dock confirmation works in either view with an empty library; no synthetic game.
    replay('dock-vacio',keys([('left',dict(dock_cursor=1)),('z',dict(resume=False,active_section=1)),('right',dict(dock_cursor=2)),('z',dict(library=True,active_section=2)),('right',dict(dock_cursor=3)),('z',dict(menu=True,active_section=3)),('left',dict(dock_cursor=2)),('left',dict(dock_cursor=1)),('left',dict(dock_cursor=0)),('z',dict(resume=True,active_section=0))]),['--view','resume','--demo-count','0'])
    replay('pulsaciones-durante-transicion',keys([('e',dict(resume=False,section_title='Descubre')),('down',dict(selected=1)),('right',dict(selected=1)),('up',dict(selected=0)),('q',dict(resume=True,section_title='Continuar')),('e',dict(resume=False)),('s',dict(menu=True)),('x',dict(menu=False))],gap=16),['--view','resume','--demo-history'])
    replay('foco-entre-secciones-desde-secundaria',keys([('down',dict(selected=4)),('right',dict(selected=5)),('down',dict(dock_cursor=2)),('left',dict(dock_cursor=1)),('z',dict(resume=False,active_section=1)),('up',dict(selected=0,dock_cursor=-1)),('right',dict(dock_cursor=2)),('left',dict(dock_cursor=1)),('left',dict(dock_cursor=0)),('z',dict(resume=True,active_section=0)),('up',dict(selected=5,dock_cursor=-1))],gap=60),['--view','resume','--demo-history','--visual-case','resume-three'])
    replay('capsula-vacia-y-retorno',keys([('up',dict(system_focused=True)),('z',dict(system_open=True)),('e',dict(system_open=True,resume=True)),('x',dict(system_open=False,dock_cursor=2)),('up',dict(system_focused=True)),('z',dict(system_open=True)),('z',dict(system_open=False,menu=True,active_section=3))],gap=80),['--view','resume','--demo-count','0'])
    replay('movimiento-reducido',keys([('e',dict(resume=False)),('m',dict(reduced=True,section_min=1,section_max=1)),('q',dict(resume=True,section_min=0,section_max=0)),('e',dict(resume=False,section_min=1,section_max=1))]),['--view','resume','--demo-history'])
    observed=replay('movimiento-observado',[*keys([('e',dict(resume=False))]),dict(at_ms=1500,section_min=1,section_max=1),*keys([('q',dict(resume=True))],start=1600),dict(at_ms=2100,section_min=0,section_max=0)],['--view','resume','--demo-history'])
    samples=[float(line.split('section=')[1].split()[0]) for line in observed.splitlines() if line.startswith('MOTION_SAMPLE')]
    assert any(0<v<1 for v in samples), 'No intermediate motion observed'
    # Captures are rendered states observed during a real transition. Their cost
    # changes sampling cadence; the sequence is evidence, never a timing benchmark.
    frames=OUT/'transition';frames.mkdir()
    steps=[dict(at_ms=300,capture=frames/'00.png',section_title='Continuar'),*keys([('e',dict(resume=False,section_title='Descubre'))])]
    steps.extend([dict(at_ms=1080,capture=frames/'01.png'),dict(at_ms=1500,capture=frames/'02.png',section_min=1,section_max=1)])
    steps+=keys([('q',dict(resume=True,section_title='Continuar'))],start=1900)
    steps.extend([dict(at_ms=1980,capture=frames/'03.png'),dict(at_ms=2450,capture=frames/'04.png',section_min=0,section_max=0)])
    replay('transicion-capturada',sorted(steps,key=lambda s:s['at_ms']),['--view','resume','--demo-history'])
    hashes={str(p.relative_to(OUT)):png(p) for p in OUT.rglob('*.png')}
    assert hashes['continuar.png']!=hashes['descubre.png']
    (OUT/'captures.json').write_text(json.dumps(hashes,indent=2)+'\n')
    passed=True
finally:
    (OUT/'report.json').write_text(json.dumps(dict(passed=passed,checks=checks,scope='Actual Slint software rendering in ARM64 Docker; fictional catalog; no physical display, audio or performance validation'),indent=2)+'\n')
