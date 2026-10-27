#!/usr/bin/env python3
"""Isolated GPU probe, reusing the supervised PID1 recovery contract. Dry-run default."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time
import phase12_preview as preview


def plan(bundle,unit,seconds,binary='gpu-probe'):
    if binary not in {'gpu-probe','gpu-pipeline-probe'}:raise ValueError('Unsupported isolated probe')
    manifest=preview.verify_bundle(bundle)
    if binary not in manifest['files']:raise ValueError('Unchecked probe executable')
    args=preview.command(bundle,unit,False,seconds=seconds,isolated=True)
    executable=args.index(str(bundle/'emotional-ui'))
    args=args[:executable]+[str(bundle/binary),'--seconds',str(seconds)]
    args=[x.replace('SLINT_BACKEND=linuxkms-software','SLINT_BACKEND=linuxkms-femtovg') for x in args]
    env_index=next(i for i,x in enumerate(args) if x.startswith('--property=Environment='))
    args[env_index]+=' EMOTIONAL_PERF=full'
    if binary=='gpu-pipeline-probe':
        if 'EMOTIONAL_GPU_SWAP=' in args[env_index]:raise ValueError('Pipeline A requires unchanged swap policy')
        args[env_index]+=' EMOTIONAL_PERF_TRACE=1'
    return args


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('bundle',type=Path);p.add_argument('--unit',required=True)
    p.add_argument('--seconds',type=int,default=10)
    p.add_argument('--binary',choices=['gpu-probe','gpu-pipeline-probe'],default='gpu-probe')
    for flag in ['execute','supervised','second-session-confirmed']:p.add_argument('--'+flag,action='store_true')
    a=p.parse_args();a.bundle=preview.safe_path(a.bundle)
    if not 2<=a.seconds<=30:raise SystemExit('Probe requires 2..30 seconds')
    args=plan(a.bundle,a.unit,a.seconds,a.binary)
    if not a.execute:print(json.dumps({'dry_run':True,'argv':args}));return
    if not a.supervised or not a.second_session_confirmed or not os.environ.get('SSH_CONNECTION'):
        raise SystemExit('Fresh authorization/presence and independent SSH required')
    state=subprocess.check_output(['systemctl','show',preview.UNIT,'-p','ActiveState','-p','MainPID'],text=True)
    if 'ActiveState=active' not in state:raise SystemExit('Original frontend inactive')
    pid=re.search(r'MainPID=(\d+)',state)
    if not pid:raise SystemExit('Frontend PID unknown')
    preview.check_tree(subprocess.check_output(['ps','-eo','pid=,ppid=,comm='],text=True),int(pid[1]))
    preview.check_backlight([int(x.read_text()) for x in Path('/sys/class/backlight').glob('*/actual_brightness')])
    started=time.monotonic();result=subprocess.run(args,capture_output=True,text=True,timeout=a.seconds+65)
    report={'unit':a.unit,'exit_code':result.returncode,'wall_seconds':time.monotonic()-started,
            'stdout':result.stdout,'stderr':result.stderr,
            'after':subprocess.check_output(['systemctl','show',preview.UNIT,'-p','ActiveState','-p','MainPID'],text=True),
            'physical_image_verified':False,'restoration_visible_verified':False}
    (a.bundle/(a.unit+'-result.json')).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2));raise SystemExit(result.returncode)

if __name__=='__main__':main()
