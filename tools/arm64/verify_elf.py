#!/usr/bin/env python3
"""Audit the complete ELF and its reference SDK closure. Never claims device compatibility."""
import argparse,hashlib,json,re,shutil,struct,subprocess
from pathlib import Path

def header(data):
    if len(data)<64 or data[:6]!=b'\x7fELF\x02\x01':raise ValueError('ELF64 little-endian required')
    if struct.unpack_from('<H',data,18)[0]!=183:raise ValueError('AArch64 required')
    if struct.unpack_from('<H',data,16)[0] not in (2,3):raise ValueError('Executable or PIE required')
    return {'format':'ELF64','machine':'AArch64','endian':'little'}

def tool_path():
    for name in ['llvm-readelf','readelf','/opt/homebrew/opt/llvm/bin/llvm-readelf']:
        p=shutil.which(name)
        if p:return p
    raise RuntimeError('Install LLVM/GNU readelf')

def inspect(path,tool):
    output=subprocess.check_output([tool,'-h','-l','-d','-V',str(path)],text=True)
    needed=re.findall(r'\(NEEDED\).*?\[([^\]]+)\]',output)
    rpaths=re.findall(r'\((?:RPATH|RUNPATH)\).*?\[([^\]]+)\]',output)
    versions=sorted(set(re.findall(r'GLIBC_[A-Za-z0-9_.]+',output)))
    match=re.search(r'Requesting program interpreter:\s*([^\]\n]+)',output)
    return {'needed':needed,'rpaths':rpaths,'glibc_versions':versions,'interpreter':match.group(1).strip() if match else None}

def audit(binary,sysroot):
    data=binary.read_bytes();result=header(data);tool=tool_path();info=inspect(binary,tool)
    if info['interpreter']!='/lib/ld-linux-aarch64.so.1':raise ValueError('Unexpected interpreter')
    if info['rpaths']:raise ValueError('RPATH/RUNPATH is forbidden')
    if any(re.search(r'Qt|X11|wayland|EGL|GLES|gbm',n) for n in info['needed']):raise ValueError('Unexpected GUI/GPU dependency in software build')
    libraries={};pending=list(info['needed']);missing=[]
    while pending:
        name=pending.pop()
        if name in libraries or name in missing:continue
        found=next((p for d in ['lib/aarch64-linux-gnu','usr/lib/aarch64-linux-gnu'] if (p:=sysroot/d/name).is_file()),None)
        if found is None:missing.append(name);continue
        item=inspect(found,tool);libraries[name]={'sha256':hashlib.sha256(found.read_bytes()).hexdigest(),**item};pending.extend(item['needed'])
    if missing:raise ValueError('Reference SDK is missing: '+', '.join(missing))
    versions=set(info['glibc_versions'])
    for item in libraries.values():versions.update(item['glibc_versions'])
    # Dependency version definitions are an upper reference inventory, not minimum requirements.
    result.update({'binary':str(binary),'sha256':hashlib.sha256(data).hexdigest(),'size_bytes':len(data),**info,'reference_libraries':libraries,'reference_sdk':str(sysroot),'device_compatibility':'UNVERIFIED','physical_display_verified':False})
    return result

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('binary',type=Path);p.add_argument('--sysroot',type=Path,required=True);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
    result=audit(args.binary,args.sysroot)
    args.output.write_text(json.dumps(result,indent=2)+'\n');print(result['format'],result['machine'],result['size_bytes'],'bytes');print('NEEDED:',', '.join(result['needed']));print('Device compatibility: UNVERIFIED')
if __name__=='__main__':main()
