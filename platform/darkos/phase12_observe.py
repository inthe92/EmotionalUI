#!/usr/bin/env python3
"""Read-only owned-preview observer. No EVIOCGRAB, input injection or FB writes."""
import argparse,base64,binascii,fcntl,json,os,re,select,struct,subprocess,time,zlib
from pathlib import Path

def fb_readback():
    fd=os.open('/dev/fb0',os.O_RDONLY|os.O_CLOEXEC)
    try:
        var=bytearray(160);fcntl.ioctl(fd,0x4600,var);v=struct.unpack('40I',var)
        w,h,x,y,bpp=v[0],v[1],v[4],v[5],v[6]
        if (w,h,bpp)!=(640,480,32):raise ValueError('Unsupported readback geometry')
        stride=int(Path('/sys/class/graphics/fb0/stride').read_text())
        if not 2560<=stride<=16384:raise ValueError('Unexpected stride')
        raw=os.pread(fd,stride*h,y*stride);rows=[]
        for row in range(h):
            line=bytearray()
            for col in range(w):
                pixel=struct.unpack_from('<I',raw,row*stride+(col+x)*4)[0]
                for offset,length in [(v[8],v[9]),(v[11],v[12]),(v[14],v[15])]:
                    if length!=8:raise ValueError('Unsupported RGB channels')
                    line.append((pixel>>offset)&255)
            rows.append(b'\0'+line)
        def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',binascii.crc32(kind+data)&0xffffffff)
        png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(rows),3))+chunk(b'IEND',b'')
        return dict(kind='device-fb0-readback-not-LCD-photo',width=w,height=h,xoffset=x,yoffset=y,stride=stride,png_base64=base64.b64encode(png).decode())
    finally:os.close(fd)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--unit',required=True);p.add_argument('--binary',type=Path,required=True);p.add_argument('--no-readback',action='store_true');a=p.parse_args()
    if not re.fullmatch(r'emotional-ui-phase12-preview-[a-z0-9-]+',a.unit):raise SystemExit('Owned preview unit required')
    start=time.monotonic();pid=0
    while time.monotonic()-start<20:
        r=subprocess.run(['systemctl','show',a.unit,'-p','MainPID','--value'],capture_output=True,text=True)
        if r.stdout.strip().isdigit():pid=int(r.stdout.strip())
        if pid and Path('/proc',str(pid),'exe').resolve()==a.binary.resolve():break
        time.sleep(.1)
    else:raise SystemExit('Owned frontend not observed')
    result=dict(kind='physical-readonly-observer',pid=pid,events=[],samples=[],ticks_per_second=os.sysconf('SC_CLK_TCK'),readback=None)
    fd=None
    for device in sorted(Path('/dev/input').glob('event*')):
        candidate=os.open(device,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC);name=bytearray(256)
        fcntl.ioctl(candidate,(2<<30)|(256<<16)|(ord('E')<<8)|6,name)
        if name.split(b'\0')[0]==b'retrogame_joypad':fd=candidate;result['device']=str(device);break
        os.close(candidate)
    start=time.monotonic();captured=a.no_readback;eventsize=struct.calcsize('llHHi')
    try:
        while time.monotonic()-start<40:
            try:
                if Path('/proc',str(pid),'exe').resolve()!=a.binary.resolve():break
                stat=Path('/proc',str(pid),'stat').read_text().rsplit(')',1)[1].split()
                result['samples'].append(dict(ms=round((time.monotonic()-start)*1000,2),utime=int(stat[11]),stime=int(stat[12]),rss_kib=int(stat[21])*os.sysconf('SC_PAGE_SIZE')//1024))
            except OSError:break
            if not captured and time.monotonic()-start>=3:
                captured=True
                try:result['readback']=fb_readback()
                except (OSError,ValueError,struct.error) as e:result['readback_error']=str(e)
            if fd is not None:
                if select.select([fd],[],[],.05)[0]:
                    data=os.read(fd,eventsize*64)
                    for _,_,kind,code,value in struct.iter_unpack('llHHi',data):
                        if kind in (1,3) and len(result['events'])<5000:result['events'].append(dict(ms=round((time.monotonic()-start)*1000,2),kind=kind,code=code,value=value))
            else:time.sleep(.05)
        print(json.dumps(result))
    finally:
        if fd is not None:os.close(fd)
if __name__=='__main__':main()
