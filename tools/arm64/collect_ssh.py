#!/usr/bin/env python3
"""Run only the read-only diagnostic using existing SSH authentication and host trust."""
import argparse,json,re,subprocess
from pathlib import Path

def valid_host(host):return bool(re.fullmatch(r'[A-Za-z0-9_.@:\[\]-]+',host)) and not host.startswith('-')
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('host');p.add_argument('output',type=Path);args=p.parse_args()
    if not valid_host(args.host):raise SystemExit('Invalid SSH host/alias')
    if args.output.exists():raise SystemExit('Refusing to overwrite diagnostic')
    script=Path(__file__).resolve().parents[2]/'platform/darkos/diagnostic.py'
    result=subprocess.run(['ssh','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=5','--',args.host,'python3 -'],input=script.read_bytes(),capture_output=True,timeout=90)
    if result.returncode:raise SystemExit(result.stderr.decode(errors='replace'))
    diagnostic=json.loads(result.stdout)
    if diagnostic.get('kind')!='device-diagnostic':raise SystemExit('Invalid diagnostic output')
    with args.output.open('x') as f:json.dump(diagnostic,f,indent=2);f.write('\n')
    print('Diagnostic saved:',args.output)
    print('Reported system:',diagnostic['system']['os'],diagnostic['system']['machine'],diagnostic['system']['model'])
if __name__=='__main__':main()
