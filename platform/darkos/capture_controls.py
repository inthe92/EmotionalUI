#!/usr/bin/env python3
"""Bounded read-only gamepad observation. No EVIOCGRAB, keyboard or injected events."""
import argparse,fcntl,json,os,select,struct,subprocess,time
from pathlib import Path

def name(fd):
    b=bytearray(256);fcntl.ioctl(fd,(2<<30)|(256<<16)|(ord('E')<<8)|6,b);return b.split(b'\0')[0].decode(errors='replace')
def collect(seconds,trial,binary):
    started=trial/'smoke/started.json';deadline=time.monotonic()+12
    while not started.exists():
        if time.monotonic()>deadline:raise RuntimeError('The supervised frontend did not start')
        time.sleep(.05)
    state=json.loads(started.read_text());pid=int(state['pid']);proc=Path(f'/proc/{pid}')
    if (proc/'exe').resolve()!=binary.resolve():raise RuntimeError('Unexpected frontend identity')
    identity=(proc/'stat').read_text().split(')')[-1].split()[19]
    running=subprocess.check_output(['ps','-eo','comm='],text=True).splitlines()
    if any(name.strip() in ['emulationstatio','emulationstation','retroarch'] for name in running):raise RuntimeError('Original frontend/emulator is still active; do not capture')
    fd=None;path=None
    for i in range(64):
        candidate=f'/dev/input/event{i}'
        try:
            f=os.open(candidate,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC)
            if name(f)=='retrogame_joypad':fd=f;path=candidate;break
            os.close(f)
        except OSError:continue
    if fd is None:raise SystemExit('retrogame_joypad is unavailable')
    try:
        bits=bytearray(96);fcntl.ioctl(fd,(2<<30)|(96<<16)|(ord('E')<<8)|0x21,bits)
        keys=[i for i in range(768) if bits[i//8]&(1<<(i%8))]
        axes={};abits=bytearray(8);fcntl.ioctl(fd,(2<<30)|(8<<16)|(ord('E')<<8)|0x23,abits)
        for code in range(64):
            if abits[code//8]&(1<<(code%8)):
                b=bytearray(24);fcntl.ioctl(fd,(2<<30)|(24<<16)|(ord('E')<<8)|(0x40+code),b);_,lo,hi,fuzz,flat,res=struct.unpack('6i',b);axes[str(code)]={'min':lo,'max':hi,'fuzz':fuzz,'flat':flat,'resolution':res}
        events=[];start=time.monotonic();size=struct.calcsize('llHHi')
        while time.monotonic()-start<seconds and len(events)<5000:
            try:
                if (proc/'stat').read_text().split(')')[-1].split()[19]!=identity:break
            except OSError:break
            ready,_,_=select.select([fd],[],[],.1)
            if not ready:continue
            raw=os.read(fd,size*64)
            if not raw:raise RuntimeError('Gamepad disconnected')
            for sec,usec,kind,code,value in struct.iter_unpack('llHHi',raw):
                if kind in [1,3] or (kind==0 and code==3):events.append({'ms':round((time.monotonic()-start)*1000,3),'kind':kind,'code':code,'value':value})
        return {'kind':'guided-gamepad-observation','device':path,'device_name':'retrogame_joypad','key_capabilities':keys,'axes':axes,'seconds':seconds,'events':events,'physical_labels_confirmed':False,'owned_frontend_observed':True,'frontend_pid':pid}
    finally:os.close(fd)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--trial-dir',type=Path,required=True);p.add_argument('--binary',type=Path,required=True);p.add_argument('--seconds',type=int,default=34);a=p.parse_args()
    if not 1<=a.seconds<=45:raise SystemExit('Duration must be 1..45 seconds')
    print(json.dumps(collect(a.seconds,a.trial_dir,a.binary),indent=2))
