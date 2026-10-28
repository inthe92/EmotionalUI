#!/usr/bin/env python3
"""Exercise real app navigation/supervisor using own synthetic files/processes.

Requires a simulator build; does not open DRM, native controllers or emulators.
"""
import argparse
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile

BUTTONS = [(100,'menu'),(101,'down'),(102,'confirm'),(103,'back'),
           (104,'library'),(105,'previous'),(106,'up')]


def replay(path, steps):
    text = '[mapping]\n'
    for code, action in BUTTONS:
        text += '\n[[mapping.buttons]]\ncode='+str(code)+'\naction='+json.dumps(action)+'\n'
    for step in steps:
        text += '\n[[steps]]\n'+''.join(key+'='+json.dumps(value)+'\n' for key,value in step.items())
    path.write_text(text)


def press(steps, at, code):
    steps.extend([dict(at_ms=at,kind=1,code=code,value=1),dict(at_ms=at+32,kind=1,code=code,value=0)])


def run(binary, output):
    output.mkdir(exist_ok=False)
    sandbox = Path(tempfile.mkdtemp(prefix='eui-player-flow-')).resolve()
    library = sandbox/'roms'
    system = library/'snes'
    system.mkdir(parents=True)
    tools = sandbox/'options'
    (tools/'Advanced').mkdir(parents=True)
    for relative in ['Wifi.sh','Advanced/Test Settings.sh']:
        (tools/relative).write_text('exit 0\n')  # Discovery only; never executed.
    (system/'alpha.fixture').write_text('own synthetic data')
    (system/'beta.fixture').write_text('own synthetic data')
    (system/'gamma.fixture').write_text('own file without XML metadata')
    (system/'beta.srm').write_text('original-save')
    (system/'gamelist.xml').write_text('<gameList><game><path>alpha.fixture</path><name>Super Mario World</name></game><game><path>beta.fixture</path><name>Super Metroid</name></game></gameList>')
    progress = sandbox/'progress'
    for name in ['saves','states']:(progress/name).mkdir(parents=True)
    return_replay = sandbox/'return.toml'
    final_steps = [dict(at_ms=700,resume=True,has_recent=True,resume_count=1,scene_weather=0,capture=str(output/'continue.png'))]
    press(final_steps,1000,100)
    for index in range(8):press(final_steps,1200+index*100,101)
    final_steps.append(dict(at_ms=2100,menu=True,menu_index=8))
    final_steps.append(dict(at_ms=2300,kind=1,code=102,value=1))
    resume_steps = [dict(at_ms=300,library=True,library_index=1,dock_cursor=-1,library_cursor=0,library_heading_contains='METROID')]
    press(resume_steps,500,103)  # Search results -> systems.
    press(resume_steps,650,103)  # Library -> Moment.
    press(resume_steps,900,105)  # Moment -> Continue.
    resume_steps.append(dict(at_ms=1200,resume=True,has_recent=True,resume_count=1))
    press(resume_steps,1400,102)
    resume_steps.append(dict(at_ms=2000))
    replay(sandbox/'resume-next.toml',resume_steps)
    replay(sandbox/'finish-next.toml',final_steps)
    game = sandbox/'synthetic_game.py'
    game.write_text('''import json,sys,time
from pathlib import Path
root=Path(__file__).resolve().parent
resume='--resume' in sys.argv
saves=list((root/'progress/games').glob('*/saves/beta.srm'))
assert len(saves)==1
save=saves[0]
progress=save.parent.parent
assert save.read_text()==('private-progress' if resume else 'original-save')
if resume: assert (progress/'states/beta.state.auto').read_text()=='private-state'
save.write_text('private-progress')
(progress/'states/beta.state.auto').write_text('private-state')
with (root/'game-events.jsonl').open('a') as out: out.write(json.dumps({'resume':resume})+'\\n')
(root/'return.toml').write_bytes((root/('finish-next.toml' if resume else 'resume-next.toml')).read_bytes())
time.sleep(2.2)
''')
    profile = sandbox/'launchers.toml'
    profile.write_text('handoff="process-exit"\nclock_confirmed=true\nsystem_options_root='+json.dumps(str(tools))+'\n[[launchers]]\nid="synthetic-snes"\n'+
        ''.join(key+'='+json.dumps(value)+'\n' for key,value in {
            'root':str(system),'executable':sys.executable,'working_directory':str(sandbox),
            'arguments':[str(game),'{rom}'],'resume_arguments':[str(game),'--resume','{rom}'],
            'extensions':['.fixture'],'waits_for_exit':True,'progress_root':str(progress)}.items()))
    base = [str(binary),'--functional','--launcher-config',str(profile),'--library-root',str(library),'--reduced-motion','--audio-mode','silence']
    # Settings: use the actual child UI, but no supervisor can execute a tool.
    ipc = sandbox/'settings-ipc'
    ipc.mkdir()
    settings=[]
    settings.append(dict(at_ms=700,dark_mode=True,scene_weather=0,palette_index=6,capture=str(output/'dark.png')))
    press(settings,1000,100)
    for index in range(4):press(settings,1200+index*120,101)
    press(settings,1800,102)
    settings.append(dict(at_ms=2000,menu=True,menu_index=4,dark_mode=False,scene_weather=0,palette_index=3,capture=str(output/'light-settings.png')))
    for index in range(7):press(settings,2200+index*120,101)
    press(settings,3200,102)
    settings.append(dict(at_ms=3400,options_open=True,menu=True,menu_index=0,capture=str(output/'system-options.png')))
    press(settings,3600,103)  # B closes categories without opening a system tool.
    settings.append(dict(at_ms=4100,options_open=False,menu_index=11,dark_mode=False))
    for index in range(3):press(settings,4300+index*120,106)
    settings.append(dict(at_ms=4700,menu_index=8))
    settings.append(dict(at_ms=4900,kind=1,code=102,value=1))
    replay(sandbox/'settings.toml',settings)
    settings_db = sandbox/'settings.sqlite'
    commands=[]
    def execute(name, argv):
        result=subprocess.run(argv,capture_output=True,text=True,timeout=50)
        (output/(name+'.log')).write_text(result.stdout+'\n'+result.stderr)
        commands.append({'name':name,'exit_code':result.returncode})
        if result.returncode: raise RuntimeError(name+' failed; inspect '+str(output/(name+'.log')))
    execute('settings',base+['--db',str(settings_db),'--launch-child',str(ipc),'--at','2026-01-15T14:00','--weather','rain','--sim-replay',str(sandbox/'settings.toml')])
    assert not (ipc/'request.toml').exists()
    persistent=[dict(at_ms=700,dark_mode=False,scene_weather=0,palette_index=3)]
    press(persistent,1000,100)
    for index in range(8):press(persistent,1200+index*100,101)
    persistent.append(dict(at_ms=2100,menu_index=8))
    persistent.append(dict(at_ms=2300,kind=1,code=102,value=1))
    replay(sandbox/'persistent.toml',persistent)
    execute('appearance-restart',base+['--db',str(settings_db),'--launch-child',str(ipc),'--at','2026-01-15T22:00','--sim-replay',str(sandbox/'persistent.toml')])
    first=[]
    press(first,900,104)
    first.append(dict(at_ms=1100,library=True,catalog_count=3,library_first_title='Buscar juegos'))
    first.append(dict(at_ms=1200,library_choose=2,library_heading_contains='Super Nintendo'))
    first.append(dict(at_ms=1350,library_back=True,library_first_title='Buscar juegos'))
    first.append(dict(at_ms=1450,library_choose=0,library_search_open=True))
    first.append(dict(at_ms=1470,library_key_choose=39,library_search_open=False))
    first.append(dict(at_ms=1500,library_choose=0,library_search_open=True))
    for index,letter in enumerate('METROID'):
        first.append(dict(at_ms=1550+index*60,library_key_choose=ord(letter)-ord('A'),library_search_open=True))
    press(first,2050,100)  # Start performs the search.
    first.append(dict(at_ms=2200,library_search_open=False,library_index=1,library_first_title='Super Metroid  ·  Super Nintendo'))
    press(first,2350,102)
    first.append(dict(at_ms=2900))
    replay(sandbox/'first.toml',first)
    replay(return_replay,[dict(at_ms=100)])  # Real synthetic process replaces this.
    db=sandbox/'flow.sqlite'
    execute('game-cycle',base+['--db',str(db),'--view','emotional','--sim-replay',str(sandbox/'first.toml'),'--sim-return-replay',str(return_replay)])
    events=[json.loads(line) for line in (sandbox/'game-events.jsonl').read_text().splitlines()]
    assert events==[{'resume':False},{'resume':True}],events
    assert (system/'beta.srm').read_text()=='original-save'
    with sqlite3.connect(db) as connection:
        launches=connection.execute('SELECT count(*) FROM launch_log').fetchone()[0]
        sessions=connection.execute('SELECT count(*) FROM activity').fetchone()[0]
    assert launches==2,(launches,sessions)
    result={'kind':'synthetic-functional-app-flow','commands':commands,'sandbox':str(sandbox),
        'normal_then_resume':events,'launch_attempts':launches,'activity_records':sessions,
        'grouped_library_and_search_launch':True,'search_return_context_restored':True,
        'original_save_unchanged':True,'appearance_persisted_across_clock_change':True,
        'options_discovered_only':True,'physical_emulators_or_tools_executed':False}
    (output/'report.json').write_text(json.dumps(result,indent=2)+'\n')
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(run(args.binary.resolve(),args.output.resolve()),indent=2))
