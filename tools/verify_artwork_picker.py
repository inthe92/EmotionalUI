#!/usr/bin/env python3
"""Real native picker/reload/restore flow with a deterministic RPC and own images."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from PIL import Image
from verify_player_flow import press,replay

def run(binary,output):
    output.mkdir(parents=True,exist_ok=False);root=Path(tempfile.mkdtemp(prefix='emotional-sgdb-picker-')).resolve()
    library=root/'roms';system=library/'snes';system.mkdir(parents=True)
    (system/'owned.fixture').write_text('Owned text fixture, not a ROM.\n')
    Image.new('RGB',(100,100),'red').save(system/'original.png')
    (system/'gamelist.xml').write_text('<gameList><game><path>owned.fixture</path><name>Juego propio</name><boxart>original.png</boxart></game></gameList>')
    source={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in system.iterdir()}
    cache=root/'art';cache.mkdir();Image.new('RGB',(600,900),'blue').save(cache/'raw.png')
    helper=root/'helper.py';helper.write_text('''import json,sys,time,os
from pathlib import Path
r=json.load(sys.stdin);root=Path(__file__).parent
with (root/'calls.jsonl').open('a') as f:f.write(json.dumps(r)+'\\n')
if (root/'slow').exists():
 (root/'slow-pid').write_text(str(os.getpid()))
 time.sleep(8)
if r['operation']=='search':print(json.dumps({'games':[{'id':101,'name':'Otra edición','release_date':None},{'id':202,'name':'Juego propio','release_date':None}]}))
elif r['operation']=='grids':print(json.dumps({'grids':[{'id':77,'url':'https://cdn2.steamgriddb.com/grid/test.png','author':{'name':'Owned fixture'},'style':'alternate'}]}))
else:print(json.dumps({'path':str(root/'art/raw.png'),'root':str(root/'art')}))
''')
    choices=root/'choices.json';env=dict(os.environ,EMOTIONAL_SGDB_HELPER=str(helper),EMOTIONAL_ARTWORK_CHOICES=str(choices))
    subprocess.run([str(binary),'--db',str(root/'library.sqlite'),'--import-gamelist',str(system/'gamelist.xml'),'--rom-root',str(system)],env=env,capture_output=True,text=True,check=True,timeout=15)
    def scenario(path,steps):
        replay(path,steps)
        text=path.read_text().replace('[[steps]]','[[mapping.buttons]]\ncode=107\naction="details"\n\n[[steps]]',1)
        path.write_text(text)
    def open_steps():
        steps=[];press(steps,600,104);steps.append(dict(at_ms=800,library_choose=2))
        press(steps,1000,107);steps.append(dict(at_ms=1200,game_options=True,game_option_choose=5,artwork_open=True))
        return steps
    def execute(name,steps,seconds=6):
        path=root/(name+'.toml');scenario(path,steps)
        command=[str(binary),'--db',str(root/'library.sqlite'),'--library-root',str(library),'--view','emotional','--audio-mode','silence','--reduced-motion','--smoke-test','--smoke-seconds',str(seconds),'--sim-replay',str(path)]
        result=subprocess.run(command,env=env,capture_output=True,text=True,timeout=15)
        (output/(name+'.log')).write_text(result.stdout+'\n'+result.stderr)
        if result.returncode:raise RuntimeError(name+' failed; inspect '+str(output/(name+'.log')))
    steps=open_steps();steps.append(dict(at_ms=2200,artwork_choose=1))
    steps.append(dict(at_ms=3600,artwork_open=True,artwork_ready=True,capture=str(output/'selector.png')))
    steps.append(dict(at_ms=4100,artwork_command=0));steps.append(dict(at_ms=4800,artwork_open=False,game_options=True))
    execute('apply',steps)
    data=json.loads(choices.read_text());assert len(data['games'])==1
    game_id=next(iter(data['games']));assert game_id.endswith('/owned.fixture')
    assert Image.open(data['games'][game_id]['path']).size==(240,360)
    calls=[json.loads(line) for line in (root/'calls.jsonl').read_text().splitlines()]
    assert [c['game_id'] for c in calls if c['operation']=='grids']==[202]
    steps=open_steps();steps.append(dict(at_ms=2400,artwork_open=True,artwork_command=2))
    steps.append(dict(at_ms=3200,artwork_open=False,game_options=True))
    execute('restore',steps);assert json.loads(choices.read_text())['games']=={}
    (root/'slow').touch();steps=open_steps();press(steps,1700,103)
    steps.append(dict(at_ms=2100,artwork_open=False,game_options=True))
    execute('cancel-network',steps,seconds=3)
    pid=int((root/'slow-pid').read_text())
    try:os.kill(pid,0)
    except ProcessLookupError:pass
    else:raise AssertionError('Cancelled provider process survived')
    assert {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in system.iterdir()}==source
    result={'kind':'real Slint picker with owned RPC/images','explicit_second_search_result_selected':True,'portrait_applied_and_persisted':True,'restored_without_deleting_originals':True,'cancel_stops_provider_process':True,'source_files_unchanged':True,'network_used':False,'physical_console':False}
    (output/'report.json').write_text(json.dumps(result,indent=2)+'\n');return result

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    print(json.dumps(run(args.binary.resolve(),args.output.resolve()),indent=2))
