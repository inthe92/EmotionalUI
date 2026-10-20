#!/usr/bin/env python3
"""Supervised preview only: transient PID1 unit, exit-triggered ES restoration, no games.
Dry-run by default. Never edit or enable units, change boot, or use a game watchdog.
"""
import argparse,hashlib,json,os,re,subprocess,time,threading
from pathlib import Path
UNIT='emulationstation.service'
RESTORE='+/usr/bin/systemctl start '+UNIT
ALLOWED={'emulationstatio','emulationstation','bash','sh','sleep','ogage'}

def safe_path(path):
    path=Path(path)
    if not path.is_absolute() or not re.fullmatch(r'/[A-Za-z0-9_./-]+',str(path)) or '..' in path.parts:
        raise ValueError('Absolute simple path required (no systemd specifiers/control characters)')
    return path

def verify_bundle(bundle):
    bundle=safe_path(bundle)
    if bundle.is_symlink():raise ValueError('Symlink bundle is forbidden')
    manifest=json.loads((bundle/'phase12-manifest.json').read_text())
    if manifest.get('kind')!='phase12-private-preview':raise ValueError('Unexpected manifest')
    for name,digest in manifest['files'].items():
        if not re.fullmatch(r'[A-Za-z0-9_.-]+',name):raise ValueError('Unsafe payload name')
        p=bundle/name
        if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:raise ValueError('Payload checksum mismatch')
    if 'emotional-ui' not in manifest['files']:raise ValueError('Missing frontend')
    return manifest

def check_backlight(values):
    if not values or not any(value>0 for value in values):
        raise ValueError('Backlight is off or unknown; do not cede the display')

def check_audio_bank(manifest):
    if not all(name+'.wav' in manifest['files'] for name in ('umbral','brasa','rastro','rain','air')):
        raise ValueError('Calibrated bank must be covered by payload checksums')

def check_tree(text,main):
    rows=[line.split(None,2) for line in text.splitlines() if len(line.split(None,2))==3]
    parsed=[(int(pid),int(parent),name.strip()) for pid,parent,name in rows]
    if not any(pid==main and name in {'emulationstatio','emulationstation','bash','sh'} for pid,_,name in parsed):raise ValueError('Frontend PID disappeared or changed')
    descendants={main}
    while True:
        newer=descendants|{pid for pid,parent,_ in parsed if parent in descendants}
        if newer==descendants:break
        descendants=newer
    if any(pid in descendants and name not in ALLOWED for pid,_,name in parsed):raise ValueError('Unrecognized frontend child; cancel')
    if any(name in {'retroarch','retroarch32','retroarch64','Xorg','Xwayland','weston','ppsspp','ppsspp64'} for _,_,name in parsed):raise ValueError('Emulator/other compositor active; cancel')

def command(bundle,unit,probe,seconds=10,input_profile=False,library_root=None,audio_mode="silence",audio_volume=0,calibrated_audio=False):
    bundle=safe_path(bundle)
    library_root=safe_path(library_root) if library_root else bundle/'empty'
    if audio_mode not in {'silence','minimal','ambient'} or not 0<=audio_volume<=25:raise ValueError('Quiet preview audio only')
    if not re.fullmatch(r'emotional-ui-phase12-(?:probe|preview)-[a-z0-9-]+',unit):raise ValueError('Unexpected transient unit')
    if not 2<=seconds<=180:raise ValueError('Preview duration must be 2..180 seconds')
    args=['sudo','-n','systemd-run','--wait','--unit='+unit,
          '--property=Type=exec','--property=User=ark','--property=WorkingDirectory='+str(bundle),
          '--property=Restart=no','--property=TimeoutStartSec=20','--property=TimeoutStopSec=30',
          '--property=ExecStopPost='+RESTORE]
    if probe:return args+['/usr/bin/true']
    # Runtime cap applies ONLY to a --trial preview that cannot start any emulator.
    args += ['--property=ExecStartPre=+/usr/bin/systemctl stop '+UNIT,
             '--property=RuntimeMaxSec='+str(seconds+15),
             '--property=ReadOnlyPaths='+str(library_root),
             '--property=Environment=SLINT_BACKEND=linuxkms-software SLINT_SCALE_FACTOR=1 XDG_RUNTIME_DIR=/run/user/1000',
             '--property=StandardOutput=append:'+str(bundle/'preview.stdout'),
             '--property=StandardError=append:'+str(bundle/'preview.stderr'),
             str(bundle/'emotional-ui'),'--trial','--library-root',str(library_root),
             '--db',str(bundle/'preview.sqlite'),'--view','emotional','--audio-mode',audio_mode,'--audio-volume',str(audio_volume),
             '--smoke-log',str(bundle/'preview-events.toml'),'--quit-file',str(bundle/'quit.request')]
    if seconds<=30:args += ['--smoke-test','--smoke-seconds',str(seconds)]
    if input_profile:args += ['--input-profile',str(bundle/'input.toml')]
    if calibrated_audio:args += ['--audio-assets',str(bundle)]
    return args

def run(args):
    return subprocess.run(args,capture_output=True,text=True,timeout=95)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('bundle',type=Path);p.add_argument('--unit',required=True);p.add_argument('--probe',action='store_true');p.add_argument('--input-profile',action='store_true');p.add_argument('--library-root',type=Path);p.add_argument('--audio-mode',default='silence');p.add_argument('--audio-volume',type=int,default=0);p.add_argument('--calibrated-audio',action='store_true');p.add_argument('--seconds',type=int,default=10);p.add_argument('--execute',action='store_true');p.add_argument('--supervised',action='store_true');p.add_argument('--second-session-confirmed',action='store_true');a=p.parse_args()
    manifest=verify_bundle(a.bundle)
    if a.calibrated_audio:check_audio_bank(manifest)
    args=command(a.bundle,a.unit,a.probe,a.seconds,a.input_profile,a.library_root,a.audio_mode,a.audio_volume,a.calibrated_audio)
    if not a.execute:print(json.dumps({'dry_run':True,'argv':args}));return
    if not a.supervised or not a.second_session_confirmed or not os.environ.get('SSH_CONNECTION'):raise SystemExit('Current supervision and independently tested SSH session required')
    state=run(['systemctl','show',UNIT,'-p','ActiveState','-p','MainPID'])
    if state.returncode or 'ActiveState=active' not in state.stdout:raise SystemExit('Original frontend is not active')
    main=re.search(r'MainPID=(\d+)',state.stdout)
    if not main:raise SystemExit('Missing frontend PID')
    tree=run(['ps','-eo','pid=,ppid=,comm=']);check_tree(tree.stdout,int(main.group(1)))
    if not a.probe and not (a.library_root or a.bundle/'empty').is_dir():raise SystemExit('Missing own empty library')
    if not a.probe:
        check_backlight([int(path.read_text().strip()) for path in Path('/sys/class/backlight').glob('*/actual_brightness')])
    # Long previews ask the application to close gracefully; PID1's trial-only cap
    # remains independent if the SSH caller disappears. No emulator can run here.
    deadline=None
    if not a.probe and a.seconds>30:
        def request_quit():
            try:
                with (a.bundle/'quit.request').open('x') as f:f.write('preview deadline\n')
            except FileExistsError:pass
        deadline=threading.Timer(a.seconds,request_quit);deadline.daemon=True;deadline.start()
    start=time.monotonic()
    try:result=subprocess.run(args,capture_output=True,text=True,timeout=a.seconds+65)
    finally:
        if deadline:deadline.cancel()
    after=run(['systemctl','show',UNIT,'-p','ActiveState','-p','MainPID'])
    properties=run(['systemctl','show',a.unit,'-p','Result','-p','ExecMainStatus','-p','ExecStopPost','-p','ActiveState'])
    report=dict(kind='phase12-probe' if a.probe else 'phase12-preview',unit=a.unit,exit_code=result.returncode,elapsed_ms=round((time.monotonic()-start)*1000,3),stdout=result.stdout,stderr=result.stderr,after=after.stdout,unit_properties=properties.stdout,physical_image_verified=False,restoration_visible_verified=False)
    target=a.bundle/('probe-result.json' if a.probe else 'preview-result.json')
    with target.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
    print(json.dumps(report,indent=2))
    if result.returncode or 'ActiveState=active' not in after.stdout:raise SystemExit(1)
if __name__=='__main__':main()
