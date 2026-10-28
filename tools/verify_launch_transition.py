#!/usr/bin/env python3
"""Render/cancel/commit the real launch UI using owned files; no physical game.

The child-only cases cannot spawn anything. The failure case uses the actual
supervisor with an intentionally absent executable and verifies its return UI.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import subprocess
import struct
import tempfile
import time
import zlib
from verify_player_flow import press, replay


def run(binary, output):
    output.mkdir(parents=True, exist_ok=False)
    sandbox=Path(tempfile.mkdtemp(prefix='emotional-launch-transition-')).resolve()
    library=sandbox/'roms';system=library/'snes';system.mkdir(parents=True)
    fixture=system/'own.fixture';fixture.write_text('Owned text fixture, not a ROM.\n')
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    raster=(b'\0'+bytes((44,97,131))*240)*360
    (system/'cover.png').write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',240,360,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raster))+chunk(b'IEND',b''))
    (system/'gamelist.xml').write_text('<gameList><game><path>own.fixture</path><name>Una partida propia</name><boxart>cover.png</boxart></game></gameList>')
    before=hashlib.sha256(fixture.read_bytes()).hexdigest()
    profile=sandbox/'launchers.toml'
    profile.write_text('handoff="process-exit"\n[[launchers]]\nid="missing-on-purpose"\nroot='+json.dumps(str(system))+
        '\nexecutable='+json.dumps(str(sandbox/'absent-executable'))+'\narguments=["{rom}"]\nworking_directory='+json.dumps(str(sandbox))+'\nwaits_for_exit=true\n')
    commands=[]
    def execute(name,extra):
        command=[str(binary),'--functional','--launcher-config',str(profile),'--library-root',str(library),
            '--db',str(sandbox/(name+'.sqlite')),'--audio-mode','silence','--view','emotional',*extra]
        started=time.monotonic();result=subprocess.run(command,capture_output=True,text=True,timeout=20)
        (output/(name+'.log')).write_text(result.stdout+'\n'+result.stderr)
        commands.append({'name':name,'exit_code':result.returncode,'wall_seconds':time.monotonic()-started})
        if result.returncode:raise RuntimeError(name+' failed; inspect '+str(output/(name+'.log')))

    def opening():
        steps=[];press(steps,700,104)
        steps.append(dict(at_ms=850,library=True,library_first_title='Buscar juegos'))
        steps.append(dict(at_ms=900,library_choose=2))
        steps.append(dict(at_ms=1050,library_choose=0))
        return steps

    # B and touch both dispatch this Back action. Assert cancellation leaves no
    # request that a supervisor could consume, and the unchanged library returns.
    ipc=sandbox/'cancel-ipc';ipc.mkdir()
    steps=opening()
    # PNG encoding must not occupy the short cancellation window on the
    # software backend. Visual poses are captured separately with the GPU example.
    steps.append(dict(at_ms=1120,launch_open=True,launch_pending=True,launch_title='Una partida propia'))
    press(steps,1150,103)
    steps.append(dict(at_ms=1400,launch_open=False,launch_pending=False,library=True,capture=str(output/'cancelado.png')))
    replay(sandbox/'cancel.toml',steps)
    execute('cancel',['--launch-child',str(ipc),'--smoke-test','--smoke-seconds','2','--sim-replay',str(sandbox/'cancel.toml')])
    assert not (ipc/'request.toml').exists()

    ipc=sandbox/'commit-ipc';ipc.mkdir()
    steps=opening()
    steps.append(dict(at_ms=1220,launch_open=True,launch_pending=True))
    replay(sandbox/'commit.toml',steps)
    execute('commit',['--launch-child',str(ipc),'--smoke-test','--smoke-seconds','3','--sim-replay',str(sandbox/'commit.toml')])
    assert (ipc/'request.toml').exists()
    wait=float(re.search(r'handoff_wait_ms=([0-9.]+)',(output/'commit.log').read_text()).group(1))
    assert 440 <= wait < 2000,wait

    # A failed executable resolves through the real supervisor; no emulator or
    # system tool is present, and returning must show the original UI again.
    returned=[]
    returned.append(dict(at_ms=500,launch_open=False,launch_pending=False,message_contains='No pudimos iniciar',capture=str(output/'error-recuperado.png')))
    press(returned,700,100)
    for i in range(8):press(returned,850+i*80,101)
    returned.append(dict(at_ms=1550,menu=True,menu_index=8))
    # Quit terminates this event loop, so it must be the final replay event.
    returned.append(dict(at_ms=1600,kind=1,code=102,value=1))
    replay(sandbox/'returned.toml',returned)
    replay(sandbox/'failure.toml',opening())
    execute('spawn-failure',['--sim-replay',str(sandbox/'failure.toml'),'--sim-return-replay',str(sandbox/'returned.toml')])
    assert hashlib.sha256(fixture.read_bytes()).hexdigest()==before
    for path in output.glob('*.png'):
        header=path.read_bytes()[:24]
        assert header[:8]==b'\x89PNG\r\n\x1a\n' and struct.unpack('>II',header[16:24])==(640,480)
    result={'kind':'real-app-simulator-owned-fixtures','commands':commands,'cancel_removes_request':True,
        'commit_writes_request_after_transition':True,'spawn_failure_returns_to_ui':True,
        'commit_wait_ms':wait,
        'source_fixture_unchanged':True,'hardware_framebuffer_written':False,'physical_game_launched':False}
    (output/'report.json').write_text(json.dumps(result,indent=2)+'\n')
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(run(args.binary.resolve(),args.output.resolve()),indent=2))
