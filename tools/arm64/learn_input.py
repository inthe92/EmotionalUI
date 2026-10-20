#!/usr/bin/env python3
"""Create a profile only from the explicitly confirmed guided physical sequence."""
import argparse,json
from pathlib import Path
LABELS=['up','down','left','right','A','B','X','Y','Start','Select','L1','R1','L2','R2']
ACTIONS=['up','down','left','right','confirm','back','refresh','library','menu','reduced_motion','previous','next','previous','next']
def button_prefix(data,count):
    if data.get('device_name')!='retrogame_joypad' or not data.get('owned_frontend_observed'):raise ValueError('Capture must belong to the supervised frontend')
    events=data['events']
    if any(e['kind']==0 and e['code']==3 for e in events):raise ValueError('Dropped input stream; repeat calibration')
    pressed=[e for e in events if e['kind']==1 and e['value']==1]
    codes=[e['code'] for e in pressed]
    if len(codes)!=count or len(set(codes))!=count:raise ValueError(f'Expected exactly {count} different button presses in the announced order')
    held=set()
    for e in events:
        if e['kind']==1:
            if e['value']==1:
                if held:raise ValueError('Release each button before pressing the next')
                held.add(e['code'])
            elif e['value']==0:held.discard(e['code'])
    if held:raise ValueError('A button was left held')
    if not set(codes)<=set(data['key_capabilities']):raise ValueError('Button absent from device capabilities')
    return [{'code':code,'action':action,'physical_label':label} for code,action,label in zip(codes,ACTIONS,LABELS)]

def learn_partial(data):
    count=sum(e['kind']==1 and e['value']==1 for e in data['events'])
    if not 9<=count<14:raise ValueError('A partial profile needs the confirmed prefix through Start, at minimum')
    return button_prefix(data,count),[]

def learn(data):
    buttons=button_prefix(data,14);events=data['events'];last=max(i for i,e in enumerate(events) if e['kind']==1 and e['value']==1)
    active={};strokes=[]
    for e in events[last+1:]:
        if e['kind']!=3:continue
        axis=data['axes'].get(str(e['code']))
        if not axis or axis['max']<=axis['min']:raise ValueError('Invalid axis metadata')
        value=(e['value']*2-axis['min']-axis['max'])/(axis['max']-axis['min'])
        was=bool(active)
        if abs(value)>.35:active[e['code']]=1 if value>0 else -1
        elif abs(value)<.20:active.pop(e['code'],None)
        if len(active)>1:raise ValueError('Ambiguous diagonal; use one direction at a time')
        if not was and active:strokes.append(next(iter(active.items())))
    if len(strokes)!=4 or active:raise ValueError('Expected left, right, up, down stick strokes, returning to center each time')
    x1,x2,y1,y2=strokes
    if x1[0]!=x2[0] or x1[1]==x2[1] or y1[0]!=y2[0] or y1[1]==y2[1] or x1[0]==y1[0]:raise ValueError('Stick sequence is inconsistent')
    axes=[]
    for (code,sign),negative,positive in [(x1,'left','right'),(y1,'up','down')]:
        info=data['axes'][str(code)]
        axes.append({'code':code,'min':info['min'],'max':info['max'],'negative':negative if sign<0 else positive,'positive':positive if sign<0 else negative})
    return buttons,axes

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path);p.add_argument('--observed-prefix',action='store_true');p.add_argument('--completion',type=Path);p.add_argument('--confirm-physical-order',action='store_true');a=p.parse_args()
    if not a.confirm_physical_order:raise SystemExit('The operator must confirm the exact announced physical order')
    data=json.loads(a.capture.read_text())
    if a.completion:
        data=complete(data,json.loads(a.completion.read_text()))
    buttons,axes=learn_partial(data) if a.observed_prefix else learn(data)
    text=f'# Derived from guided physical capture; {len(buttons)}/14 buttons and {len(axes)}/2 stick axes.\n# Missing controls are deliberately unmapped.\nenabled = true\ndevice_name = "retrogame_joypad"\n'
    for b in buttons:text+=f'\n# {b["physical_label"]}\n[[mapping.buttons]]\ncode = {b["code"]}\naction = "{b["action"]}"\n'
    for axis in axes:
        text+='\n[[mapping.axes]]\n'
        for key,value in axis.items():text+=f'{key} = {json.dumps(value)}\n'
        text+='engage_percent = 35\nrelease_percent = 20\n'
    with a.output.open('x') as f:f.write(text)
    print(a.output)
def complete(prefix,completion):
    button_prefix(prefix,13)
    if not completion.get('owned_frontend_observed') or completion.get('device_name')!=prefix.get('device_name'):raise ValueError('Completion must be observed on our frontend')
    keys=[i for i,e in enumerate(completion['events']) if e['kind']==1 and e['value']==1]
    if not keys:raise ValueError('Missing R2 press')
    start=keys[0];end=keys[1] if len(keys)>1 else len(completion['events'])
    segment=completion['events'][start:end]
    if segment[0]['code'] in {b['code'] for b in button_prefix(prefix,13)}:raise ValueError('R2 must be a new observed button')
    if any(e['kind']==0 and e['code']==3 for e in completion['events']):raise ValueError('Dropped completion stream')
    result=dict(completion,events=prefix['events']+segment,calibration_provenance='Ordered segments from two real captures; ms values keep their own capture origins and are NOT combined timing measurements.')
    learn(result)
    return result

if __name__=='__main__':main()
