#!/usr/bin/env python3
"""Development-only Slint matrix, synthetic own catalog, never changes OS time.

Requires a DEBUG binary and Pillow. No API calls; --deny-network on macOS uses
sandbox-exec. Output and databases stay outside the source tree.
"""
import argparse
import sys
import math
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as XML
from PIL import Image, ImageDraw, ImageChops

ROOT=Path(__file__).resolve().parents[1]
IDS=['madrugada','manana_temprana','media_manana','primera_tarde','media_tarde','anochecer','noche']
HOURS=['04:00','07:30','10:00','14:00','17:00','19:45','22:30']
SEASONS={'spring':'Primavera','summer':'Verano','autumn':'Otoño','winter':'Invierno'}


def toml_value(value):
    """Small fixture serializer; never writes a personal context into the repo."""
    if isinstance(value, bool): return str(value).lower()
    if isinstance(value, str): return json.dumps(value, ensure_ascii=False)
    if isinstance(value, (float, int)) and math.isfinite(value): return str(value)
    if isinstance(value, list): return '['+', '.join(toml_value(v) for v in value)+']'
    if isinstance(value, dict): return '{'+', '.join(toml_value(k)+' = '+toml_value(v) for k,v in value.items())+'}'
    raise ValueError('Unsupported fixture value')


def dev_run(args, work):
    """Manual CLI or a compact matrix. Existing Slint binary, always network denied."""
    if sys.platform != 'darwin' or not Path('/usr/bin/sandbox-exec').exists():
        raise ValueError('This DEV driver requires macOS sandbox-exec')
    import tomllib
    ambient={'timezone':'Etc/UTC','hemisphere':'north'}
    if args.context_config:
        ambient.update(tomllib.loads(args.context_config.read_text()).get('ambient',{}))
    if args.latitude is not None or args.longitude is not None:
        if args.latitude is None or args.longitude is None: raise ValueError('Supply both coordinates')
        ambient['location']={'latitude':args.latitude,'longitude':args.longitude}
    if args.timezone: ambient['timezone']=args.timezone
    if args.hemisphere: ambient['hemisphere']=args.hemisphere
    if not ambient.get('location'): raise ValueError('Choose --context-config or explicit test coordinates')
    if args.open and args.compact: raise ValueError('Choose manual window OR compact captures')
    lib,seed=prepare_preview(args.binary.resolve(),work)
    env=dict(os.environ,SLINT_BACKEND='winit-software',SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(work/'art-cache'))
    cases=[('manual',args.dev_at or '2026-06-21T12:00',args.weather_state or 'unknown',args.condition or 'unknown')]
    if args.compact:
        cases=[('LIVE-primavera','2026-03-20T10:30','live','clear'),
               ('CACHED-otono','2026-10-20T18:00','cached','rain'),
               ('STALE-invierno','2026-12-21T16:00','stale','snow'),
               ('UNKNOWN-verano','2026-06-21T10:30','unknown','unknown'),
               ('Halloween','2026-10-31T20:00','cached','rain'),
               ('Navidad','2026-12-25T16:00','cached','snow')]
    views=['resume','emotional'] if args.view=='both' else [args.view]
    records=[]
    for name,at,state,condition in cases:
        case={'at':at,'ambient':ambient,'weather_state':state,'condition':condition,'unknown_clock':args.unknown_clock}
        if args.cache_file: case['cache_file']=str(args.cache_file.resolve())
        if args.moment: case['moment']=args.moment
        fixture=work/(name+'.dev.toml')
        fixture.write_text('\n'.join(k+' = '+toml_value(v) for k,v in case.items())+'\n');fixture.chmod(0o600)
        for view in views:
            db=work/(name+'-'+view+'.sqlite');shutil.copy2(seed,db)
            target=args.out/(name+'-'+view+'.png')
            command=[str(args.binary.resolve()),'--dev-environment-file',str(fixture),'--db',str(db),'--library-root',str(lib.parent),'--demo-history','--visual-case','resume-three','--view',view,'--concept','integrated','--audio-mode','silence','--reduced-motion']
            if not args.open: command+=['--snapshot',str(target),'--smoke-seconds','2']
            command=['/usr/bin/sandbox-exec','-p','(version 1)(allow default)(deny network*)',*command]
            log=work/(name+'-'+view+'.log')
            with log.open('w') as stream:
                done=subprocess.run(command,cwd=ROOT,env=env,stdout=stream,stderr=subprocess.STDOUT,timeout=None if args.open else 25)
            if done.returncode: raise RuntimeError(f'DEV failed; inspect {log}')
            trace=next((line for line in log.read_text().splitlines() if line.startswith('DEMO_CONTEXT ')),None)
            if trace is None or 'ui_verified=true' not in trace: raise RuntimeError('Missing engine/Slint verification')
            if not args.open:
                with Image.open(target) as image: assert image.size==(640,480) and image.mode=='RGB'
                target.chmod(0o600)
            records.append({'case':name,'view':view,'trace':trace,'file':None if args.open else target.name})
            print(name,view,trace,flush=True)
    if not args.open:
        columns=2 if len(cases)<3 else 4
        rows=(len(records)+columns-1)//columns
        sheet=Image.new('RGB',(320*columns,264*rows),'#141718');draw=ImageDraw.Draw(sheet)
        for index,record in enumerate(records):
            x=(index%columns)*320;y=(index//columns)*264
            with Image.open(args.out/record['file']) as image:sheet.paste(image.resize((320,240)),(x,y+24))
            draw.text((x+6,y+6),record['case']+' / '+record['view']+' - DEMO',fill='white')
        sheet.save(args.out/'Comparacion-DEMO.jpg',quality=94)
    (work/'dev-results.json').write_text(json.dumps({'network':'OS denied','api_calls':0,'hardware_tested':False,'captures':records},indent=2))
    return records


def prepare_preview(binary,work):
    lib=work/'library/snes';lib.mkdir(parents=True,exist_ok=True)
    xml=XML.Element('gameList')
    for i,(name,kind) in enumerate([('La casa del faro','boxart'),('Entre montañas','screenshot'),('El viaje de vuelta','hero')]):
        (lib/f'{i}.fixture').write_bytes(b'Own preview fixture, not a ROM')
        shutil.copy2(ROOT/f'docs/editorial-artwork-2026-10-19/fixtures/{kind}/snes/art.png',lib/f'{i}.png')
        game=XML.SubElement(xml,'game')
        for key,value in dict(path=f'./{i}.fixture',name=name,image=f'./{i}.png').items():XML.SubElement(game,key).text=value
    XML.ElementTree(xml).write(lib/'gamelist.xml',encoding='unicode')
    seed=work/'catalog-seed.sqlite'
    if not seed.exists():
        subprocess.run([str(binary),'--import-gamelist',str(lib/'gamelist.xml'),'--rom-root',str(lib),'--db',str(seed)],cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
    return lib,seed


def run(binary,output,work,deny_network):
    lib,seed=prepare_preview(binary,work)
    config=work/'context.toml';config.write_text('[ambient]\nhemisphere="north"\ntimezone="Etc/UTC"\n')
    env=dict(os.environ,SLINT_BACKEND='winit-software',SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(work/'art-cache'))
    captures=[]
    def capture(name,hour,season,view='resume',weather='unknown',extra=()):
        target=output/(name+'.png')
        db=work/(name+'.sqlite')
        for suffix in ('-wal','-shm'):
            Path(str(db)+suffix).unlink(missing_ok=True)
        shutil.copy2(seed,db)
        args=[str(binary),'--config',str(config),'--db',str(work/(name+'.sqlite')),'--library-root',str(lib.parent),
              '--demo-history','--visual-case','resume-three','--view',view,'--at','2026-10-20T'+hour,
              '--dev-season',season,'--weather',weather,'--audio-mode','silence','--reduced-motion',
              '--smoke-seconds','2','--snapshot',str(target),*extra]
        if deny_network:args=['/usr/bin/sandbox-exec','-p','(version 1)(allow default)(deny network*)',*args]
        with (work/(name+'.log')).open('w') as log:
            done=subprocess.run(args,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=20)
        assert done.returncode==0,(name,done.returncode)
        with Image.open(target) as image:
            image.load();assert image.mode=='RGB' and image.size==(640,480)
        captures.append(dict(name=name,season=season,view=view,demo_weather=weather))
    for season in SEASONS:
        for id,hour in zip(IDS,HOURS):capture(season+'-'+id,hour,season)
        capture(season+'-Descubre','17:00',season,'emotional')
        print(season,flush=True)
    for weather in ['clear','cloud','rain','fog','snow']:
        capture('DEMO-'+weather,'17:00','autumn','emotional',weather)
    capture('reloj-no-fiable','17:00','autumn',extra=['--unknown-time','--snapshot-view','menu'])
    # Same input date, library and hour: only season and its DEMO label may differ.
    layout=[]
    for id in IDS:
        with Image.open(output/('spring-'+id+'.png')) as source:
            for season in ['summer','autumn','winter']:
                with Image.open(output/(season+'-'+id+'.png')) as target:
                    diff=ImageChops.difference(source,target)
                    draw=ImageDraw.Draw(diff)
                    draw.rectangle((0,72,639,101),fill=0)  # Explicit DEMO subtitle
                    # Seasonal light legitimately changes scene pixels behind the
                    # translucent UI. Compare the unchanged opaque cover interior.
                    bounds=diff.crop((52,136,120,270)).getbbox()
                    layout.append(dict(moment=id,season=season,opaque_artwork_geometry_unchanged=bounds is None,bounds=bounds))
    for view in ['Continuar','Descubre']:
        size=(7*240,4*204) if view=='Continuar' else (4*320,264)
        sheet=Image.new('RGB',size,(20,23,25));draw=ImageDraw.Draw(sheet)
        for row,(season,label) in enumerate(SEASONS.items()):
            for col,id in enumerate(IDS if view=='Continuar' else ['Descubre']):
                file=season+'-'+id+'.png'
                with Image.open(output/file) as image:
                    if view=='Continuar':x,y=col*240,row*204;thumb=(240,180)
                    else:x,y=row*320,0;thumb=(320,240)
                    sheet.paste(image.resize(thumb),(x,y+24))
                draw.text((x+6,y+6),label+' · '+id,fill='white')
        sheet.save(output/('Matriz-'+view+'.jpg'),quality=94)
    report=dict(captures=captures,layout=layout,hardware_tested=False,network_denied=deny_network)
    (work/'matrix.json').write_text(json.dumps(report,indent=2)+'\n')
    return report


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,default=ROOT/'target/debug/emotional-ui')
    p.add_argument('--out',type=Path,required=True);p.add_argument('--work',type=Path)
    p.add_argument('--deny-network',action='store_true')
    p.add_argument('--dev-at');p.add_argument('--context-config',type=Path)
    p.add_argument('--cache-file',type=Path)
    p.add_argument('--latitude',type=float);p.add_argument('--longitude',type=float)
    p.add_argument('--timezone');p.add_argument('--hemisphere',choices=['north','south'])
    p.add_argument('--moment',choices=IDS);p.add_argument('--weather-state',choices=['live','cached','stale','unknown'])
    p.add_argument('--condition',choices=['clear','cloud','rain','snow','fog','unknown'])
    p.add_argument('--unknown-clock',action='store_true');p.add_argument('--compact',action='store_true')
    p.add_argument('--open',action='store_true');p.add_argument('--view',choices=['both','resume','emotional'],default='both')
    args=p.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    work=args.work or Path(tempfile.mkdtemp(prefix='emotional-environment-'))
    work.mkdir(parents=True,exist_ok=True)
    if args.compact or args.dev_at or args.weather_state or args.moment or args.open or args.unknown_clock or args.context_config or args.condition or args.latitude is not None or args.longitude is not None or args.timezone or args.cache_file:
        dev_run(args,work)
    else:
        result=run(args.binary.resolve(),args.out,work,args.deny_network)
        print(f'{len(result["captures"])} Slint captures; opaque artwork unchanged: {all(r["opaque_artwork_geometry_unchanged"] for r in result["layout"])}')
