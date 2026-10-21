#!/usr/bin/env python3
"""Controlled cache A/B: same Slint executable, XRGB buffers, no LCD claims."""
import argparse, hashlib, json, os, re
from pathlib import Path
from PIL import Image, ImageChops
from check_phase14_motion import run, key


def metrics(stdout):
    result = {}
    for line in stdout.splitlines():
        m = re.match(r'PERF metric=(\S+) n=(\d+) median=([\d.]+) p95=([\d.]+) max=([\d.]+) total=([\d.]+)', line)
        if m:
            result[m[1]] = dict(zip(['n','median_ms','p95_ms','max_ms','total_ms'], map(float,m.groups()[1:])))
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for arg in ['binary','seed','fixtures','out']: p.add_argument('--'+arg,type=Path,required=True)
    p.add_argument('--mode',choices=['visual','profile'],required=True)
    a=p.parse_args();a.out.mkdir(parents=True,exist_ok=False)
    report=dict(binary_sha256=hashlib.sha256(a.binary.read_bytes()).hexdigest(),physical=False,records=[])
    if a.mode=='profile':
        steps=[]
        for i,t in enumerate([1500,2000,2500,3000,3500,4000]): steps+=key(t,'e' if i%2==0 else 'q')
        steps.append(dict(at_ms=6000,active_section=0))
        for repeat in range(2):
            for mode in (['off','layers'] if repeat==0 else ['layers','off']):
                os.environ['EMOTIONAL_SCENE_CACHE']=mode
                folder,stdout=run(a.binary.resolve(),a.out,a.seed,a.fixtures,f'{repeat}-{mode}','resume',steps,
                                  seconds=8,perf=True,pixel_target='xrgb')
                record=dict(mode=mode,repeat=repeat,metrics=metrics(stdout),process=json.loads((folder/'process.json').read_text()))
                if mode=='layers':
                    assert record['metrics']['scene_cache_prepare_ms']['n']==1
                    assert record['metrics']['near_cache_prepare_ms']['n']==1
                report['records'].append(record);print(json.dumps(record),flush=True)
    else:
        contexts=[('winter','2026-12-25T16:00','snow'),('autumn','2026-10-31T20:00','rain'),
                  ('summer','2026-06-21T10:30','clear'),('fog','2026-01-15T03:00','fog')]
        for label,at,weather in contexts:
            context=f'at="{at}"\nweather="{weather}"\n[ambient]\ntimezone="Etc/UTC"\nhemisphere="north"\n'
            for view in ['resume','emotional']:
                # Sample the actual production plan at exact elapsed times.
                # Freezing is simulator-only, never part of profiling or KMS.
                for mode in ['off','layers']:
                    os.environ['EMOTIONAL_SCENE_CACHE']=mode
                    name=f'{label}-{view}-{mode}';folder=a.out/name
                    steps=[dict(at_ms=900,capture=str(folder/'rest.png'))]
                    if label=='winter':
                        steps+=key(1000,'e' if view=='resume' else 'q')
                        for i,t in enumerate([0,90,180,270,450]):
                            steps.append(dict(at_ms=1100+i*130,capture=str(folder/f'motion-{i}.png'),motion_sample_ms=t))
                    run(a.binary.resolve(),a.out,a.seed,a.fixtures,name,view,steps,seconds=2,
                        perf=True,pixel_target='xrgb',context=context)
                left=a.out/f'{label}-{view}-off';right=a.out/f'{label}-{view}-layers'
                for before in sorted(left.glob('*.png')):
                    with Image.open(before) as x, Image.open(right/before.name) as y:
                        assert x.size==y.size==(640,480)
                        diff=ImageChops.difference(x,y)
                        changed=sum(px!=(0,0,0) for px in diff.get_flattened_data())
                        record=dict(context=label,view=view,frame=before.name,changed_pixels=changed,
                                    max_channel_difference=max(e[1] for e in diff.getextrema()))
                        report['records'].append(record)
                        if before.name=='rest.png':
                            sheet=Image.new('RGB',(1280,480));sheet.paste(x,(0,0));sheet.paste(y,(640,0))
                            sheet.save(a.out/f'{label}-{view}-comparison.png')
                print(label,view,report['records'][-1],flush=True)
    (a.out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    if a.mode=='visual' and any(r['changed_pixels'] for r in report['records']):
        raise SystemExit('Visual difference detected: inspect before retaining cache')


if __name__=='__main__': main()
