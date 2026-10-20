#!/usr/bin/env python3
"""Prepare a private trial candidate, never deploy. Explicit payload allowlist.
Main frontend installation remains disabled. Public redistribution is NOT approved.
"""
import argparse,gzip,hashlib,json,shutil,struct,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent

def build(binary,destination):
 if destination.exists():raise ValueError('Refusing to overwrite package')
 data=binary.read_bytes()
 if len(data)<64 or data[:6]!=b'\x7fELF\x02\x01' or struct.unpack_from('<H',data,18)[0]!=183:raise ValueError('Requires complete ELF64 AArch64 binary')
 destination.mkdir(parents=True)
 shutil.copy2(binary,destination/'emotional-ui');(destination/'emotional-ui').chmod(0o755)
 for n in ('manage.py','install.sh','restore.sh','uninstall.sh','trial.sh'):shutil.copy2(ROOT/'platform/distribution'/n,destination/n)
 for d in ('assets','config','licenses','source'): (destination/d).mkdir()
 shutil.copy2(ROOT/'assets/atlas/emotional-atlas-0.2.0-experimental.1.db',destination/'assets/emotional-atlas-experimental.db')
 # Art and default audio are embedded already. Do not copy another runtime set.
 for n in ('LICENSE','THIRD_PARTY_LICENSES.md','AUDIO_ASSET_PROVENANCE.md'):
  shutil.copy2(ROOT/n,destination/'licenses'/n)
 shutil.copy2(ROOT/'assets/fonts/DejaVu-COPYRIGHT.txt',destination/'licenses/DejaVu-COPYRIGHT.txt')
 # Matching source, including vendored notices and original editable resources.
 names=['Cargo.toml','Cargo.lock','rust-toolchain.toml','LICENSE','THIRD_PARTY_LICENSES.md','README.md']
 for folder in ('app','core','services','assets','tools','platform/distribution'):
  names += [p.relative_to(ROOT).as_posix() for p in (ROOT/folder).rglob('*') if p.is_file() and not any(x in p.parts for x in ('__pycache__','.DS_Store'))]
 with (destination/'source/corresponding-source.tar.gz').open('xb') as raw:
  with gzip.GzipFile(filename='',fileobj=raw,mode='wb',mtime=0) as zipped:
   with tarfile.open(fileobj=zipped,mode='w') as tar:
    for n in sorted(names):
     p=ROOT/n
     if p.is_symlink():raise ValueError('Unexpected source symlink')
     info=tar.gettarinfo(str(p),arcname='emotional-ui/'+n)
     info.uid=info.gid=0;info.uname=info.gname='';info.mtime=0
     with p.open('rb') as source:tar.addfile(info,source)
 (destination/'config/README.md').write_text('No physical launcher profile is bundled. Atlas is experimental and requires explicit identity links and opt-in. Main installation is unavailable.\n')
 (destination/'README.md').write_text('''# EmotionalUI 0.2.0 · private trial candidate

Linux AArch64, intended RG353V/dArkOS. NOT physically certified. Not a public release.
All scripts require Python >=3.8. `install.sh --yes` copies only verified own files;
platform/device/loader checks can refuse unknown dArkOS identifiers conservatively.
No boot changes, ES entries, service changes or main frontend replacement are made.

`trial.sh --display-released --recovery-confirmed` requires an independently verified,
supervised display handoff performed separately. These flags are acknowledgements,
not a recovery mechanism. This package does NOT stop/start EmulationStation.
The mode shows real local XML data, remains silent initially, and blocks emulators.
Do not run this over an active DRM owner. Exit using Start → Salir.
Safe trial entry from the system menu and physical return are STILL PENDING.

`restore.sh` disables our trial marker, preserving personal data; there are no system
changes to roll back. `uninstall.sh --yes` requires restore first, removes only hash-
verified owned files and refuses conflicts. User data is outside the install folder.
No SSH password is needed by these scripts; no credential is included.

The experimental Atlas is optional; no AI proposal is human-approved. No ROMs/BIOS.
Corresponding project source is included; Cargo dependencies remain version locked.
Public release is blocked by physical validation and room-reference rights review.
Checksum verifies integrity, not identity of a publisher. No automatic updates.
''')
 files={p.relative_to(destination).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(destination.rglob('*')) if p.is_file()}
 manifest=dict(schema=1,kind='emotional-ui-trial',version='0.2.0-phase11-candidate',architecture='aarch64',target='RG353V/dArkOS-unverified',main_install_enabled=False,public_release=False,files=files)
 (destination/'package.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
 hashes=files|{'package.json':hashlib.sha256((destination/'package.json').read_bytes()).hexdigest()}
 (destination/'SHA256SUMS').write_text(''.join(f'{h}  {n}\n' for n,h in sorted(hashes.items())))
 return manifest
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();build(a.binary,a.output)
