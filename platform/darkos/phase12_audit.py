#!/usr/bin/env python3
"""Additional read-only Phase 12 inventory; never opens PCM, grabs input, or starts games."""
import glob,json,os,shutil,subprocess,xml.etree.ElementTree as ET
from pathlib import Path

def read(path,limit=65536):
    try:
        with open(path,'rb') as f:return f.read(limit).decode(errors='replace').replace('\0','').strip()
    except OSError:return None

def cmd(args):
    try:
        p=subprocess.run(args,capture_output=True,text=True,timeout=8)
        return dict(code=p.returncode,stdout=p.stdout[:65536],stderr=p.stderr[:2048])
    except (OSError,subprocess.TimeoutExpired) as e:return dict(error=str(e))

def collect():
    home=Path.home()
    result=dict(kind='phase12-read-only-extension',remote_writes=False)
    result['memory']=read('/proc/meminfo');result['cpu']=read('/proc/cpuinfo')
    result['thermal']=[dict(path=p,type=read(str(Path(p).parent/'type')),value=read(p)) for p in glob.glob('/sys/class/thermal/thermal_zone*/temp')]
    result['mounts']=read('/proc/mounts')
    result['space']=cmd(['df','-Pk',str(home),'/tmp'])
    result['release_files']={str(p):read(p,8192) for p in [Path('/etc/darkos-release'),Path('/etc/arkos-release'),Path('/etc/issue'),Path('/etc/version'),Path('/usr/share/plymouth/themes/text.plymouth'),home/'.config/.DEVICE'] if p.is_file()}
    result['service']=cmd(['systemctl','cat','emulationstation.service'])
    result['sudo_readonly']=cmd(['sudo','-n','systemctl','show','emulationstation.service','-p','ActiveState','-p','MainPID'])
    result['wrapper']=read('/usr/bin/emulationstation/emulationstation.sh')
    result['drm_clients']=cmd(['sudo','-n','cat','/sys/kernel/debug/dri/0/clients'])
    result['audio_status']={p:read(p,8192) for p in glob.glob('/proc/asound/card*/pcm*p/sub*/status')}
    result['audio_configs']={str(p):read(p,16384) for p in [home/'.asoundrc',Path('/etc/asound.conf')] if p.is_file()}
    result['alsa_devices']=cmd(['aplay','-l']);result['mixer']=cmd(['amixer','scontents'])
    result['tools']={n:shutil.which(n) for n in ['python3','readelf','timeout','systemctl','systemd-run','retroarch','retroarch32','retroarch64','aplay','amixer','fuser']}
    result['process_tree']=cmd(['ps','-eo','pid,ppid,comm'])
    result['es_configs']=[]
    for path in [Path('/etc/emulationstation/es_systems.cfg'),home/'.emulationstation/es_systems.cfg']:
        if not path.is_file():continue
        data=path.read_bytes()
        if len(data)>2*1024*1024:continue
        root=ET.fromstring(data)
        systems=[]
        for system in root.findall('system'):
            name=system.findtext('name');rom=system.findtext('path')
            item=dict(name=name,path=rom,command=system.findtext('command'))
            if rom:
                p=Path(os.path.expanduser(rom));item['exists']=p.is_dir()
                if p.is_dir():
                    item['gamelist_exists']=(p/'gamelist.xml').is_file()
                    item['gamelist_bytes']=(p/'gamelist.xml').stat().st_size if item['gamelist_exists'] else 0
            systems.append(item)
        result['es_configs'].append(dict(path=str(path),systems=systems))
    result['prior_test_dirs']=[str(p) for p in (home/'emotional-ui-tests').glob('*') if p.is_dir()] if (home/'emotional-ui-tests').is_dir() else []
    return result
if __name__=='__main__':print(json.dumps(collect(),indent=2))
