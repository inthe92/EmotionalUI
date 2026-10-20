#!/usr/bin/env python3
"""Verify ARM64 ELF without installing the optional `file` package."""
from pathlib import Path
import hashlib,struct,sys
for name in sys.argv[1:]:
    p=Path(name);data=p.read_bytes()
    assert data[:6]==b'\x7fELF\x02\x01', 'ELF64 little endian required'
    assert struct.unpack_from('<H',data,18)[0]==183, 'AArch64 required'
    print(p.name,'ELF64 AArch64',len(data),hashlib.sha256(data).hexdigest())
