#!/usr/bin/env python3
"""Zig C compiler/linker for target artifacts only. No SDK path is baked into ELF."""
import os,sys
from pathlib import Path
zig=os.environ['EMOTIONAL_ZIG']
root=Path(os.environ['EMOTIONAL_ARM64_SYSROOT']).resolve()
floor=os.environ.get('EMOTIONAL_GLIBC_FLOOR','2.36')
paths=[root/'usr/lib/aarch64-linux-gnu',root/'lib/aarch64-linux-gnu']
args=[zig,'cc','-target','aarch64-linux-gnu.'+floor]
for p in paths:args+=['-L'+str(p)]
incoming=[]
skip=False
for arg in sys.argv[1:]:
    if skip:skip=False;continue
    if arg in ('-target','--target'):skip=True;continue
    if arg.startswith('--target='):continue
    # rustc adds this A53-only erratum workaround for generic AArch64 GNU.
    # Zig 0.14.1 rejects the flag; our RK3566 target has Cortex-A55 cores.
    # This wrapper is not a toolchain for Cortex-A53 devices.
    if arg == '-Wl,--fix-cortex-a53-843419':continue
    incoming.append(arg)
os.execv(zig,args+incoming)
