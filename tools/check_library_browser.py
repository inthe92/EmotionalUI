#!/usr/bin/env python3
"""Bounded real-Slint library replays with owned text fixtures, never real ROMs.

The output contains private temporary databases/logs. Only the labelled PNG
screenshots are suitable for an explicitly reviewed public example.
"""
import argparse
import ast
import re
import json
import os
from pathlib import Path
import shutil
import subprocess
import xml.etree.ElementTree as ET

BUTTONS = {"up":0,"down":1,"left":2,"right":3,"confirm":4,"back":5,"previous":6,"next":7,"details":8,"menu":9}
class Replay:
    def __init__(self): self.steps=[]
    def check(self, at, **fields): self.steps.append(dict(at_ms=at, **fields))
    def press(self,at,action,**expected):
        self.steps.extend([dict(at_ms=at,kind=1,code=BUTTONS[action],value=1,**expected),dict(at_ms=at+8,kind=1,code=BUTTONS[action],value=0)])
    def text(self,at,text,**expected):
        self.steps.extend([dict(at_ms=at,text=text,pressed=True,**expected),dict(at_ms=at+8,text=text,pressed=False)])
    def tap(self,at,x,y,**expected):
        self.steps.extend([dict(at_ms=at,pointer=dict(x=x,y=y,down=True)),dict(at_ms=at+40,pointer=dict(x=x,y=y,down=False),**expected)])
    def capture(self,at,path,**expected): self.check(at,capture=str(path),**expected)
    def write(self,path):
        lines=['keyboard=true','[mapping]']
        for action,code in BUTTONS.items():lines+=['[[mapping.buttons]]',f'code={code}','action='+json.dumps(action)]
        for step in sorted(self.steps,key=lambda s:s['at_ms']):
            lines+=['[[steps]]']
            for key,value in step.items():
                if key=='pointer':lines+=['pointer={'+','.join(k+'='+json.dumps(v) for k,v in value.items())+'}']
                else:lines+=[key+'='+json.dumps(value)]
        path.write_text('\n'.join(lines)+'\n')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--motion',action='store_true',help='Keep normal finite animations enabled')
    parser.add_argument('--cases',nargs='+',help='Run only these focused cases')
    args=parser.parse_args();binary=args.binary.resolve();out=args.out.resolve();out.mkdir(exist_ok=False,parents=True)
    fixture=out/'fixtures';seed=out/'seed.sqlite'
    for system,count in [('gba',10),('psx',4),('snes',30)]:
        folder=fixture/system;folder.mkdir(parents=True)
        games=ET.Element('gameList')
        for i in range(count):
            rom=folder/f'game-{i:03}.fixture';rom.write_text('Owned non-executable library test fixture.\n')
            name=f'Aventura {system.upper()} {i:02}'
            if system=='gba' and i==0:name='VIAJE portátil'
            if system=='snes' and i==0:name='Viaje del faro'
            if system=='snes' and i==29:name='Un título extraordinariamente largo con acentos: el pequeño árbol & su última travesía hasta el horizonte'
            game=ET.SubElement(games,'game');ET.SubElement(game,'path').text=rom.name;ET.SubElement(game,'name').text=name
        ET.ElementTree(games).write(folder/'gamelist.xml',encoding='utf-8')
        subprocess.run([str(binary),'--db',str(seed),'--import-gamelist',str(folder/'gamelist.xml'),'--rom-root',str(folder)],check=True,capture_output=True,text=True,timeout=20)
    cases={}
    r=Replay();r.press(500,'next',active_section=1);r.press(1000,'next',active_section=2);r.capture(1600,out/'01-systems.png',library_rows=3,library_results=44)
    r.press(1800,'confirm',library_rows=13);r.press(2000,'down',library_cursor=1,library_game_title='Aventura GBA 01')
    for n in range(9):r.press(2200+n*120,'down')
    r.capture(3600,out/'02-expanded-scrolled.png',library_game_title='VIAJE portátil')
    r.press(3800,'back',library_rows=3,library_cursor=0);r.press(4000,'confirm');r.press(4200,'down',library_game_title='VIAJE portátil')
    r.press(4400,'back');r.press(4600,'down');r.press(4800,'confirm',library_rows=7)
    r.press(5000,'down',library_game_title='Aventura PSX 00');r.press(5200,'back',library_rows=3)
    r.press(5400,'up');r.press(5600,'up');r.press(5800,'up',dock_cursor=2)
    r.press(6000,'previous',active_section=1);r.press(6400,'next',active_section=2);r.press(6800,'down',dock_cursor=-1)
    cases['controller']=(r,8)
    r=Replay();r.press(500,'next');r.press(1000,'next');r.press(1600,'up');r.press(1800,'confirm',library_editing=True)
    r.capture(2100,out/'03-keyboard.png');r.text(2300,'ViAjE',library_query='ViAjE',library_results=2,library_rows=2)
    r.text(2600,'z',library_query='ViAjEz',library_results=0,library_editing=True);r.capture(2900,out/'04-empty-search.png')
    r.text(3100,'backspace',library_query='ViAjE',library_results=2);r.text(3400,'enter',library_editing=False)
    r.press(3600,'confirm',library_rows=3);r.press(3900,'down',library_game_title='VIAJE portátil');r.capture(4200,out/'05-search-results.png')
    r.press(4400,'details',game_options=True);r.press(4700,'back',game_options=False,library_game_title='VIAJE portátil',library_query='ViAjE')
    r.press(4900,'confirm',launch_pending=False,library_game_title='VIAJE portátil')
    r.press(5200,'back',library_rows=2);r.press(5500,'back',library_results=44,library_query='')
    r.press(5800,'up');r.press(6100,'confirm',library_editing=True);r.press(6400,'confirm',library_query='a')
    r.press(6700,'back',library_editing=False);r.press(7000,'back',library_query='');r.press(7300,'back',active_section=1)
    cases['search']=(r,9)
    r=Replay();r.press(500,'next');r.press(1000,'next');r.tap(1700,130,176,library_editing=True)
    for at,x,y,text in [(2000,124,334,'v'),(2250,510,272,'vi'),(2500,68,272,'via'),(2750,564,272,'viaj'),(3000,289,272,'viaje')]:r.tap(at,x,y,library_query=text)
    r.tap(3300,564,364,library_editing=False,library_results=2);r.tap(3650,180,220,library_rows=3)
    r.tap(4000,180,262,library_game_title='VIAJE portátil',launch_pending=False)
    r.press(4300,'back');r.press(4600,'back',library_query='')
    r.tap(5000,180,306,library_rows=33)
    r.check(5400,pointer=dict(x=520,y=370,down=True))
    for at,y in [(5480,350),(5560,320),(5640,285),(5720,250),(5800,220)]:r.check(at,pointer=dict(x=520,y=y))
    r.check(5850,pointer=dict(x=520,y=220,down=False));r.capture(6500,out/'06-touch-scroll.png',launch_pending=False,library_cursor=2,library_scroll_max=-60.0)
    cases['touch']=(r,8)
    r=Replay();r.press(500,'next');r.press(1000,'next');r.press(1600,'up');r.press(1800,'confirm')
    r.text(2100,'extraordinariamente',library_results=1);r.text(2400,'enter');r.press(2700,'confirm',library_rows=2);r.press(3000,'down')
    r.capture(3400,out/'07-long-title.png');r.press(3700,'details',game_options=True);r.press(4000,'back',game_options=False)
    cases['long-title']=(r,5)
    r=Replay();r.press(500,'next');r.press(1000,'next');r.press(1800,'confirm');r.press(2100,'down')
    r.press(2400,'confirm',launch_pending=False,message_contains='Puedes explorar')
    cases['trial']=(r,4)
    r=Replay();r.press(500,'next');r.press(1000,'next');r.press(1800,'confirm');r.press(2100,'down');r.check(7800,active_section=2,library_rows=13)
    cases['idle']=(r,8)
    if args.cases:cases={k:v for k,v in cases.items() if k in args.cases}
    if not cases:raise ValueError('No matching replay cases')
    report=[]
    for name,(replay,seconds) in cases.items():
        db=out/f'{name}.sqlite';shutil.copyfile(seed,db);path=out/f'{name}.toml';replay.write(path)
        result=subprocess.run([str(binary),'--db',str(db),'--library-root',str(fixture),'--view','resume','--at','2026-10-24T18:00','--audio-mode','silence','--smoke-test','--smoke-seconds',str(seconds),'--sim-replay',str(path)]+(['--trial'] if name=='trial' else [])+([] if args.motion else ['--reduced-motion']),capture_output=True,text=True,timeout=seconds+15,env=dict(os.environ,SLINT_SCALE_FACTOR='1',EMOTIONAL_ARTWORK_CACHE=str(out/'cache'),**({'EMOTIONAL_PERF':'full'} if name=='idle' else {})))
        (out/f'{name}.log').write_text(result.stdout+'\n'+result.stderr)
        report.append(dict(case=name,steps=len(replay.steps),exit_code=result.returncode))
        if result.returncode:raise RuntimeError(name+': '+result.stderr)
        if name=='idle':
            buckets=ast.literal_eval(re.search(r'PERF raster_frames_by_second=(\{.*\});',result.stdout).group(1))
            idle_frames=sum(buckets.get(second,0) for second in range(3,8))
            report[-1]['idle_raster_frames_seconds_3_to_7']=idle_frames
            assert idle_frames==0,buckets
    (out/'report.json').write_text(json.dumps(dict(hardware=False,synthetic_fixtures=True,cases=report),indent=2)+'\n')
    print(json.dumps(report))
if __name__=='__main__':main()
