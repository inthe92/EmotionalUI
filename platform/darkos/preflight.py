#!/usr/bin/env python3
"""Validate ELF and ask the system loader to resolve libraries, without starting the UI."""
import argparse,hashlib,json,os,platform,struct,subprocess
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('binary',type=Path);p.add_argument('--sha256',required=True);args=p.parse_args()
    data=args.binary.read_bytes();digest=hashlib.sha256(data).hexdigest()
    if digest!=args.sha256:raise SystemExit('Checksum mismatch')
    if len(data)<64 or data[:6]!=b'\x7fELF\x02\x01' or struct.unpack_from('<H',data,18)[0]!=183:raise SystemExit('Not an ELF64 little-endian AArch64 executable')
    if platform.system()!='Linux' or platform.machine()!='aarch64':raise SystemExit('Preflight requires Linux aarch64')
    binary=str(args.binary.resolve());loader='/lib/ld-linux-aarch64.so.1'
    result=subprocess.run([loader,'--list',binary],capture_output=True,text=True,timeout=10)
    report={'kind':'loader-preflight','binary_sha256':digest,'system_glibc':os.confstr('CS_GNU_LIBC_VERSION'),'code':result.returncode,'stdout':result.stdout,'stderr':result.stderr,'ui_started':False,'physical_display_verified':False}
    print(json.dumps(report,indent=2))
    raise SystemExit(result.returncode)
if __name__=='__main__':main()
