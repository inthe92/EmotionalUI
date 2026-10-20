#!/usr/bin/env python3
"""Private environmental validation entry point. Dry-run unless explicitly executed."""
import argparse,json,os,platform,subprocess,sys,uuid
from pathlib import Path
import phase12_preview as preview

def invocation(bundle,case,view,seconds,library=None,execute=False,supervised=False,second=False,probe=False):
    manifest=preview.verify_bundle(bundle)
    cases=json.loads((bundle/'scenarios.json').read_text())
    if case not in cases:raise ValueError('Unknown scenario')
    row=cases[case]
    config=row['config']
    if config not in manifest['files']:raise ValueError('Unchecked scenario')
    unit='emotional-ui-phase12-'+('probe-' if probe else 'preview-')+uuid.uuid4().hex[:12]
    cmd=[sys.executable,str(bundle/'phase12_preview.py'),str(bundle),'--unit',unit,'--context',config,'--view',view,'--seconds',str(seconds),'--input-profile','--isolated']
    if row.get('unknown_time'):cmd+=['--unknown-time']
    if library:cmd+=['--library-root',str(preview.safe_path(library))]
    if probe:cmd+=['--probe']
    if execute:
        if not supervised or not second:raise ValueError('Fresh presence/authorization and independent SSH session required')
        cmd+=['--execute','--supervised','--second-session-confirmed']
    return cmd

def preflight(bundle):
    manifest=preview.verify_bundle(bundle)
    if platform.system()!='Linux' or platform.machine()!='aarch64':raise ValueError('Linux AArch64 required')
    model=Path('/proc/device-tree/model').read_text().replace('\0','').strip()
    if 'rg353v' not in model.lower():raise ValueError('RG353V model not verified')
    if not Path('/run/systemd/system').is_dir():raise ValueError('systemd runtime required')
    modes=[path.read_text() for path in Path('/sys/class/drm').glob('card*-*/modes')]
    if not any('640x480' in mode.splitlines() for mode in modes):raise ValueError('640x480 DRM connector not verified')
    if 'retrogame_joypad' not in Path('/proc/bus/input/devices').read_text():raise ValueError('Historical controller identity not present; recalibrate before trial')
    subprocess.run([sys.executable,str(bundle/'preflight.py'),str(bundle/'emotional-ui'),'--sha256',manifest['files']['emotional-ui']],check=True,timeout=15)
    # --help returns before Slint initialization: not a display/controller test.
    subprocess.run([str(bundle/'emotional-ui'),'--help'],check=True,timeout=10,stdout=subprocess.DEVNULL)
    print(json.dumps(dict(model=model,ui_started=False,physical_display_verified=False)))

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--case',default='live');p.add_argument('--view',choices=['resume','emotional'],default='emotional');p.add_argument('--seconds',type=int,default=120)
    p.add_argument('--library-root',type=Path);p.add_argument('--execute',action='store_true');p.add_argument('--supervised',action='store_true');p.add_argument('--second-session-confirmed',action='store_true');p.add_argument('--probe',action='store_true');p.add_argument('--preflight',action='store_true');p.add_argument('--list',action='store_true')
    a=p.parse_args();bundle=Path(__file__).resolve().parent
    preview.verify_bundle(bundle)
    if a.list:
        for key,row in json.loads((bundle/'scenarios.json').read_text()).items():print(key,json.dumps(row,ensure_ascii=False))
        return
    if a.preflight:preflight(bundle);return
    if not 2<=a.seconds<=180:raise SystemExit('Use 2..180 seconds; caps only apply to no-game --trial')
    args=invocation(bundle,a.case,a.view,a.seconds,a.library_root,a.execute,a.supervised,a.second_session_confirmed,a.probe)
    if a.execute:
        if not os.environ.get('SSH_CONNECTION'):raise SystemExit('Verified SSH session required')
        preflight(bundle)
        raise SystemExit(subprocess.run(args).returncode)
    # Let the existing helper validate and print the exact systemd argv, without executing it.
    raise SystemExit(subprocess.run(args).returncode)
if __name__=='__main__':main()
