#!/usr/bin/env python3
"""Small real Slint matrix for Hito 2.4; own fixtures, weather DEMO, no OS changes."""
import argparse,json,os,shutil,subprocess,tempfile
from pathlib import Path
from PIL import Image,ImageDraw
from preview_environment import prepare_preview,ROOT

CASES=[
    ('Invierno-nieve-Navidad','2026-12-25T16:00','snow'),
    ('Otono-lluvia-Halloween','2026-10-31T20:00','rain'),
    ('Verano-manana','2026-06-21T10:30','clear'),
    ('Invierno-madrugada-niebla','2026-01-15T03:00','fog'),
]
def run(binary,out,work,deny_network,fixture_work=None):
    if fixture_work:
        # Reuse read-only preview inputs for exact before/after game identities.
        lib,seed=fixture_work/'library/snes',fixture_work/'catalog-seed.sqlite'
        if not lib.is_dir() or not seed.is_file():raise ValueError('Missing preview fixtures')
    else:lib,seed=prepare_preview(binary,work)
    env=dict(os.environ,SLINT_BACKEND='winit-software',SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(work/'art-cache'))
    records=[]
    def capture(name,at,weather,view,events=True):
        db=work/(name+'-'+view+'.sqlite');shutil.copy2(seed,db)
        config=work/(name+'.toml')
        config.write_text('[ambient]\nhemisphere="north"\ntimezone="Etc/UTC"\n'+('' if events else 'celebrations=[]\n'))
        target=out/(name+'-'+view+'.png')
        args=[str(binary),'--config',str(config),'--db',str(db),'--library-root',str(lib.parent),'--demo-history','--visual-case','resume-three','--view',view,'--at',at,'--weather',weather,'--audio-mode','silence','--reduced-motion','--smoke-seconds','2','--snapshot',str(target)]
        if deny_network:args=['/usr/bin/sandbox-exec','-p','(version 1)(allow default)(deny network*)',*args]
        with (work/(name+'-'+view+'.log')).open('w') as log:
            subprocess.run(args,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=25)
        with Image.open(target) as im:assert im.size==(640,480) and im.mode=='RGB'
        records.append(dict(file=target.name,at=at,weather_demo=weather,events=events,view=view))
        print(target.name,flush=True)
    for name,at,weather in CASES:
        for view in ['resume','emotional']:capture(name,at,weather,view)
    for name,at,weather in CASES[:2]:capture(name+'-sin-evento',at,weather,'emotional',False)
    sheet=Image.new('RGB',(1280,4*504),'#141718');draw=ImageDraw.Draw(sheet)
    for row,(name,_,_) in enumerate(CASES):
        for col,view in enumerate(['resume','emotional']):
            x=col*640;y=row*504
            with Image.open(out/(name+'-'+view+'.png')) as im:sheet.paste(im,(x,y+24))
            draw.text((x+8,y+6),name+' / '+('Continuar' if view=='resume' else 'Descubre')+' - DEMO',fill='white')
    sheet.save(out/'Combinaciones.jpg',quality=95)
    for name,_,_ in CASES[:2]:
        sheet=Image.new('RGB',(1280,504),'#141718');draw=ImageDraw.Draw(sheet)
        for x,suffix,label in [(0,'-sin-evento','Sin evento'),(640,'','Evento opcional')]:
            with Image.open(out/(name+suffix+'-emotional.png')) as im:sheet.paste(im,(x,24))
            draw.text((x+8,6),label+' - mismo contexto DEMO',fill='white')
        sheet.save(out/(name+'-evento.jpg'),quality=95)
    (work/'captures.json').write_text(json.dumps(dict(captures=records,physical_tested=False,network_denied=deny_network),indent=2)+'\n')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,default=ROOT/'target/debug/emotional-ui');p.add_argument('--out',type=Path,required=True);p.add_argument('--work',type=Path);p.add_argument('--deny-network',action='store_true');p.add_argument('--fixture-work',type=Path);args=p.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    work=args.work or Path(tempfile.mkdtemp(prefix='emotional-h24-'));work.mkdir(parents=True,exist_ok=True)
    run(args.binary.resolve(),args.out,work,args.deny_network,args.fixture_work)
