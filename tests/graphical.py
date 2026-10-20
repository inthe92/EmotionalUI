#!/usr/bin/env python3
"""Native UI checks and real 640x480 renderer captures; no image synthesis."""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
ROOT=Path(__file__).resolve().parent.parent
os.chdir(ROOT)
BINARY=ROOT/'target/debug/emotional-ui'
ENV=dict(os.environ,SLINT_SCALE_FACTOR='1')
def run(*args, success=True):
    result=subprocess.run([str(BINARY),'--db',':memory:',*args],env=ENV,capture_output=True,text=True,timeout=25)
    print(result.stdout,end='',flush=True)
    if success and result.returncode!=0:
        raise RuntimeError(result.stderr)
    if not success and result.returncode==0:
        raise RuntimeError('Expected the command to fail')
    return result
for count in [6,0,1,2]:
    run('--at','2026-10-18T21:30','--demo-count',str(count),'--ui-test')
report={}
for name,hour in [('despertar','08:00'),('horizonte','15:00'),('refugio','21:30'),('silencio','02:00')]:
    path=ROOT/f'docs/{name}.png'
    run('--at',f'2026-10-18T{hour}','--reduced-motion','--snapshot',str(path))
    data=path.read_bytes()
    assert data[:8]==b'\x89PNG\r\n\x1a\n'
    width,height=struct.unpack('>II',data[16:24])
    assert (width,height)==(640,480),(name,width,height)
    assert data[25] == 2, 'Screenshot must be opaque RGB, not an invisible RGBA image'
    baseline=ROOT/f'docs/before-phase2/{name}.png'
    compare_private=os.environ.get('EMOTIONAL_PRIVATE_SCENE_TESTS')=='1'
    before=baseline.read_bytes() if compare_private else None
    report[name]={'logical_size':[width,height],'sha256':hashlib.sha256(data).hexdigest(),
                  'changed_from_phase1':data!=before if before is not None else None,
                  'historical_comparison':'executed' if compare_private else 'pending: historical private baseline excluded; requires authorized originals and EMOTIONAL_PRIVATE_SCENE_TESTS=1'}
for name,view,extra in [('library','library',[]),('menu','menu',[]),('focus','focus',[]),('empty','home',['--demo-count','0']),('unknown-time','home',['--unknown-time']),('halloween','home',['--at','2026-10-31T21:30','--weather','rain'])]:
    run('--at','2026-10-18T21:30',*extra,'--reduced-motion','--snapshot-view',view,'--snapshot',str(ROOT/f'docs/{name}.png'))
# Invalid dates/configuration and capture errors must be actual failing commands.
run('--at','2026-02-30T20:00',success=False)
run('--weather','invalid',success=False)
run('--snapshot',str(ROOT/'docs/nonexistent-directory/output.png'),success=False)
(ROOT/'docs/visual-audit.json').write_text(json.dumps(report,indent=2)+'\n')
print('Native UI checks, four palettes, six auxiliary states and error propagation passed. Historical comparisons are pending unless private-baseline opt-in is enabled.',flush=True)
