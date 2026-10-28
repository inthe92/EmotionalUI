#!/usr/bin/env python3
"""Same-game invalidation must not blank a displayed cover during async decode.
Own fixture only; simulator raster evidence, not GPU/LCD certification.
"""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from PIL import Image

def run(binary,output):
    output.mkdir(parents=True,exist_ok=False)
    root=Path(tempfile.mkdtemp(prefix='emotional-art-stability-')).resolve()
    system=root/'roms/snes';system.mkdir(parents=True)
    (system/'own.fixture').write_text('Own text fixture, not a ROM.\n')
    Image.new('RGB',(240,360),(48,104,180)).save(system/'cover.png')
    (system/'gamelist.xml').write_text('<gameList><game><path>own.fixture</path><name>Super Metroid</name><boxart>cover.png</boxart></game></gameList>')
    db=root/'library.sqlite'
    subprocess.run([str(binary),'--db',str(db),'--import-gamelist',str(system/'gamelist.xml'),'--rom-root',str(system)],check=True,capture_output=True,timeout=15)
    steps=[{'at_ms':6000,'local_art':True}]
    for start in [6400,8000]:
        steps.append({'at_ms':start,'artwork_refresh':True})
        for delta in [60,160,300,800]:
            steps.append({'at_ms':start+delta,'local_art':True,'capture':str(output/f'cover-{start+delta}.png')})
    scenario=root/'stability.toml'
    scenario.write_text('[mapping]\n[[mapping.buttons]]\ncode=100\naction="confirm"\n'+''.join('\n[[steps]]\n'+''.join(f'{k} = {json.dumps(v)}\n' for k,v in step.items()) for step in steps))
    env=dict(os.environ,EMOTIONAL_ARTWORK_DELAY_MS='650',EMOTIONAL_ARTWORK_CACHE=str(root/'cache'),EMOTIONAL_ARTWORK_CHOICES=str(root/'choices.json'))
    result=subprocess.run([str(binary),'--db',str(db),'--library-root',str(root/'roms'),'--trial','--view','emotional','--audio-mode','silence','--reduced-motion','--sim-replay',str(scenario),'--smoke-log',str(output/'events.jsonl'),'--smoke-test','--smoke-seconds','10'],env=env,capture_output=True,text=True,timeout=20)
    (output/'run.log').write_text(result.stdout+'\n'+result.stderr)
    if result.returncode:raise RuntimeError('Cover stability failed; see '+str(output/'run.log'))
    # Compare actual cover-region pixels while the worker is delayed and after
    # replacement. A fallback flash must not be masked by final-state assertions.
    crops=[Image.open(p).convert('RGB').crop((50,185,145,350)).tobytes() for p in sorted(output.glob('cover-*.png'))]
    assert len(crops)==8 and len(set(crops))==1,'The visible cover changed during same-game refresh'
    report={'same_game_refreshes':2,'worker_delay_ms':650,'intermediate_cover_checks':8,'cover_pixels_stable':True,'source':'native simulator software raster','physical_console':False}
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n');return report
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();print(json.dumps(run(a.binary.resolve(),a.output.resolve()),indent=2))
