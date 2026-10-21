#!/usr/bin/env python3
"""Bounded Slint motion evidence. Offscreen buffers are not LCD presentation."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
from PIL import Image, ImageDraw


def replay(path, steps):
    path.write_text('keyboard=true\n[mapping]\n'+''.join(
        '\n[[steps]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in step.items()) for step in steps))


def key(when, value):
    return [dict(at_ms=when,text=value,pressed=True),dict(at_ms=when+8,text=value,pressed=False)]


def run(binary, out, seed, fixtures, name, view, steps, seconds=2, perf=False, reduced=False, pixel_target='rgb', context=None):
    target = out/name
    target.mkdir()
    shutil.copyfile(seed,target/'fixture.sqlite')
    config=target/'context.toml'
    config.write_text(context or 'at="2026-12-25T16:00"\nweather="snow"\n[ambient]\ntimezone="Etc/UTC"\nhemisphere="north"\n')
    script=target/'replay.toml'
    replay(script,steps)
    command=[str(binary),'--db',str(target/'fixture.sqlite'),'--library-root',str(fixtures),
             '--config',str(config),'--view',view,'--audio-mode','silence','--smoke-test',
             '--smoke-seconds',str(seconds),'--sim-replay',str(script)]
    if reduced:command+=['--reduced-motion']
    env=dict(os.environ,SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(out/'art-cache'))
    for name in ['SLINT_DEBUG_PERFORMANCE','EMOTIONAL_PERF','EMOTIONAL_PERF_TARGET','EMOTIONAL_PERF_VERIFY_SHADOW']:
        env.pop(name,None)
    if perf:
        env['EMOTIONAL_PERF']='full'
        env['EMOTIONAL_PERF_TARGET']=pixel_target
    if perf:
        with (target/'stdout.log').open('x') as log, (target/'stderr.log').open('x') as errors:
            started=time.monotonic()
            child=subprocess.Popen(command,env=env,stdout=log,stderr=errors)
            _,status,usage=os.wait4(child.pid,0)
            child.returncode=os.waitstatus_to_exitcode(status)
        elapsed=time.monotonic()-started
        result=subprocess.CompletedProcess(command,child.returncode,(target/'stdout.log').read_text(),(target/'stderr.log').read_text())
        measured=dict(wall_seconds=elapsed,cpu_seconds=usage.ru_utime+usage.ru_stime,
                      peak_rss_mib=usage.ru_maxrss/(1024*1024 if sys.platform=='darwin' else 1024))
        (target/'process.json').write_text(json.dumps(measured,indent=2))
    else:
        result=subprocess.run(command,env=env,capture_output=True,text=True,timeout=25)
    (target/'stdout.log').write_text(result.stdout)
    (target/'stderr.log').write_text(result.stderr)
    if result.returncode:raise RuntimeError(target.name+': '+result.stdout[-1200:]+result.stderr[-1200:])
    return target,result.stdout


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--baseline',type=Path)
    parser.add_argument('--seed',type=Path,required=True)
    parser.add_argument('--fixtures',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--mode',choices=['capture','profile','regression'],default='capture')
    parser.add_argument('--pixel-target',choices=['rgb','xrgb'],default='rgb')
    args=parser.parse_args()
    out=args.out.resolve();out.mkdir(parents=True,exist_ok=False)
    records=[]
    if args.mode=='regression':
        steps=[dict(at_ms=700,distinct_protagonists=True,page_at_rest=0,motion_full_viewport=True)]
        for at,value in [(800,'e'),(820,'e'),(840,'e')]:steps+=key(at,value)
        steps.append(dict(at_ms=1200,active_section=3,page_at_rest=3))
        steps+=key(1300,'e')
        steps.append(dict(at_ms=1650,active_section=0,page_at_rest=0))
        for at,value in [(1700,'e'),(1770,'q'),(1840,'e')]:steps+=key(at,value)
        steps.append(dict(at_ms=2260,active_section=1,page_at_rest=1))
        for at,value in [(2300,'d'),(2340,'x'),(2380,'d'),(2420,'x')]:steps+=key(at,value)
        steps.append(dict(at_ms=2700,game_options=False,modal_kind=0,modal_progress_max=0.0))
        steps+=key(2750,'m')+key(2800,'q')
        steps.append(dict(at_ms=2820,active_section=0,page_at_rest=0,reduced=True,pan_min=0.0,pan_max=0.0))
        run(args.binary.resolve(),out,args.seed,args.fixtures,'rapid-modal-reduced','resume',steps,seconds=4)
        records.append(dict(name='rapid-modal-reduced',steps=len(steps),passed=True,hardware=False))
    elif args.mode=='profile':
        # Same controller route, same scene/catalog, no screenshot encoding.
        steps=[]
        for i,at in enumerate([1500,2000,2500,3000,3500,4000]):steps+=key(at,'e' if i%2==0 else 'q')
        steps.append(dict(at_ms=6000,active_section=0))
        for name,binary in [('before',args.baseline),('after',args.binary)]:
            if not binary:continue
            _,stdout=run(binary.resolve(),out,args.seed,args.fixtures,name,'resume',steps,seconds=8,perf=True,pixel_target=args.pixel_target)
            metrics={}
            for line in stdout.splitlines():
                match=re.match(r'PERF metric=(\S+) n=(\d+) median=([\d.]+) p95=([\d.]+) max=([\d.]+) total=([\d.]+)',line)
                if match:metrics[match[1]]=dict(zip(['n','median_ms','p95_ms','max_ms','total_ms'],map(float,match.groups()[1:])))
            records.append(dict(name=name,sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),metrics=metrics,process=json.loads((out/name/'process.json').read_text())))
    else:
        cases=[('continuar-momento','resume',[],'e',360),('momento-continuar','emotional',[],'q',360),
               ('momento-biblioteca','emotional',[],'e',240),
               ('biblioteca-ajustes','resume',key(450,'e')+key(650,'e'),'e',240),
               ('menu-abre','resume',[],'d',180),('menu-cierra','resume',key(500,'d'),'x',130)]
        for name,view,setup,action,duration in cases:
            target=out/name
            steps=setup+[dict(at_ms=950,capture=str(target/'00.png'))]+key(1000,action)
            for i,fraction in enumerate([.25,.5,.75,1.25],1):
                steps.append(dict(at_ms=1000+round(duration*fraction),capture=str(target/f'{i:02}.png')))
            folder,stdout=run(args.binary.resolve(),out,args.seed,args.fixtures,name,view,steps)
            sheet=Image.new('RGB',(5*320,268),'#141718');draw=ImageDraw.Draw(sheet)
            for i in range(5):
                with Image.open(folder/f'{i:02}.png') as im:
                    assert im.size==(640,480) and im.mode=='RGB'
                    sheet.paste(im.resize((320,240)),(i*320,28))
                draw.text((i*320+8,8),['Inicio','1/4','1/2','3/4','Final'][i],fill='#eee8d8')
            sheet.save(out/(name+'.png'))
            records.append(dict(name=name,frames=5,physical=False,buffer='actual Slint offscreen raster' if (folder/'00.frame.txt').exists() else 'legacy Slint snapshot'))
        if args.baseline:
            folder=out/'before-continuar-momento'
            steps=[dict(at_ms=950,capture=str(folder/'00.png'))]+key(1000,'e')
            steps += [dict(at_ms=1000+at,capture=str(folder/f'{i:02}.png')) for i,at in enumerate([60,120,180,320],1)]
            run(args.baseline.resolve(),out,args.seed,args.fixtures,'before-continuar-momento','resume',steps)
    (out/'report.json').write_text(json.dumps(dict(records=records,hardware=False,pixel_target=args.pixel_target if args.mode=='profile' else 'rgb',encoding_affects_capture_timing=args.mode=='capture'),indent=2)+'\n')
    print(out,flush=True)


if __name__=='__main__':main()
