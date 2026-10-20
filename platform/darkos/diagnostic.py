#!/usr/bin/env python3
"""Read-only inventory. Emits JSON; does not read input events or change any service."""
import fcntl,glob,json,os,platform,shutil,stat,struct,subprocess,time
from pathlib import Path

def read(path):
    try:return Path(path).read_bytes()[:32768].decode(errors='replace').replace('\0','').strip()
    except OSError:return None

def command(args):
    if not shutil.which(args[0]):
        for prefix in ['/usr/sbin','/sbin']:
            candidate=Path(prefix)/args[0]
            if candidate.is_file():args=[str(candidate),*args[1:]];break
    try:
        p=subprocess.run(args,capture_output=True,text=True,timeout=5)
        return {'code':p.returncode,'stdout':p.stdout[:32768],'stderr':p.stderr[:2048]}
    except (OSError,subprocess.TimeoutExpired) as e:return {'error':str(e)}

def input_info(path):
    item={'path':path,'readable':os.access(path,os.R_OK),'writable':os.access(path,os.W_OK)}
    try:
        fd=os.open(path,os.O_RDONLY|os.O_NONBLOCK)
        try:
            buf=bytearray(256);fcntl.ioctl(fd,(2<<30)|(256<<16)|(ord('E')<<8)|0x06,buf);item['name']=buf.split(b'\0')[0].decode(errors='replace')
            bits=bytearray(64);fcntl.ioctl(fd,(2<<30)|(64<<16)|(ord('E')<<8)|0x23,bits)
            item['abs_axes']={}
            for axis in range(64):
                if bits[axis//8]&(1<<(axis%8)):
                    data=bytearray(24);fcntl.ioctl(fd,(2<<30)|(24<<16)|(ord('E')<<8)|(0x40+axis),data)
                    value,lo,hi,fuzz,flat,resolution=struct.unpack('6i',data)
                    # Current value is deliberately not retained; no event stream is read.
                    item['abs_axes'][str(axis)]={'min':lo,'max':hi,'fuzz':fuzz,'flat':flat,'resolution':resolution}
        finally:os.close(fd)
    except OSError as e:item['capability_error']=str(e)
    return item

def collect():
    result={'schema':1,'kind':'device-diagnostic','captured_unix':int(time.time()),'system':{'os':platform.system(),'machine':platform.machine(),'kernel':platform.release(),'model':read('/proc/device-tree/model'),'compatible':read('/proc/device-tree/compatible'),'os_release':read('/etc/os-release'),'glibc':command(['getconf','GNU_LIBC_VERSION']),'uid':os.getuid(),'groups':os.getgroups()},'drm':[],'inputs':[],'framebuffer':[]}
    for path in sorted(glob.glob('/dev/dri/*')):
        s=os.stat(path);result['drm'].append({'path':path,'mode':stat.filemode(s.st_mode),'uid':s.st_uid,'gid':s.st_gid,'readable':os.access(path,os.R_OK),'writable':os.access(path,os.W_OK)})
    result['connectors']=[{'path':p,'status':read(p+'/status'),'modes':read(p+'/modes'),'enabled':read(p+'/enabled')} for p in sorted(glob.glob('/sys/class/drm/card*-*')) if Path(p,'status').exists()]
    result['drivers']=[{'path':p,'target':os.path.realpath(p)} for p in sorted(glob.glob('/sys/class/drm/card*/device/driver'))]
    for path in sorted(glob.glob('/dev/input/event*')):result['inputs'].append(input_info(path))
    for path in sorted(glob.glob('/sys/class/graphics/fb*')):
        result['framebuffer'].append({'path':path,**{n:read(path+'/'+n) for n in ['name','virtual_size','bits_per_pixel','stride','rotate','modes']}})
    result['audio_cards']=read('/proc/asound/cards')
    result['loader']=command(['/lib/ld-linux-aarch64.so.1','--help'])
    result['libraries']=command(['ldconfig','-p'])
    result['packages']=command(['dpkg-query','-W','-f=${Package}\t${Version}\t${Architecture}\n','libc6','libinput10','libseat1','libudev1','libxkbcommon0','libfontconfig1','libfreetype6'])
    result['clock']=command(['timedatectl','show','-p','Timezone','-p','NTPSynchronized','-p','NTP'])
    result['services']={unit:command(['systemctl','show',unit,'--no-pager','-p','ActiveState','-p','SubState','-p','MainPID','-p','ExecStart','-p','Restart','-p','User','-p','TTYPath']) for unit in ['emulationstation.service','display-manager.service','weston.service','seatd.service']}
    result['processes']=command(['ps','-eo','pid,comm'])
    result['service_paths']={p:os.path.realpath(p) for p in ['/usr/bin/emulationstation/emulationstation.sh','/etc/systemd/system/emulationstation.service','/usr/lib/systemd/system/emulationstation.service'] if os.path.lexists(p)}
    return result
if __name__=='__main__':print(json.dumps(collect(),indent=2))
