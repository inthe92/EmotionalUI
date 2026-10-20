#!/usr/bin/env python3
"""Stage a new private test directory. Does not deploy or start the frontend."""
import argparse,hashlib,json,shutil
from pathlib import Path
from verify_elf import audit
ROOT=Path(__file__).resolve().parents[2]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('destination',type=Path);p.add_argument('--sysroot',required=True,type=Path)
    p.add_argument('--binary',type=Path,default=ROOT/'target/aarch64-unknown-linux-gnu/release/emotional-ui')
    args=p.parse_args();report=audit(args.binary,args.sysroot)
    if args.destination.exists():raise SystemExit('Refusing to overwrite an existing directory')
    folder=args.destination.resolve();folder.mkdir(parents=True)
    (folder/'lib').mkdir();(folder/'licenses').mkdir();(folder/'platform').mkdir()
    shutil.copy2(args.binary,folder/'emotional-ui')
    # The identified device has every direct dependency except libseat.
    if 'libseat.so.1' in report['needed']:
        seat=args.sysroot/'usr/lib/aarch64-linux-gnu/libseat.so.1'
        shutil.copy2(seat.resolve(),folder/'lib/libseat.so.1')
        shutil.copy2(args.sysroot/'usr/share/doc/libseat1/copyright',folder/'licenses/libseat-copyright')
    shutil.copy2(ROOT/'LICENSE',folder/'licenses/EmotionalUI-LICENSE')
    shutil.copy2(ROOT/'tools/arm64/sysroot.lock.json',folder/'licenses/reference-sdk-lock.json')
    for name in ['preflight.py','manual_smoke.py','sample_process.py','capture_controls.py','supervised_trial.py']:
        shutil.copy2(ROOT/'platform/darkos'/name,folder/'platform'/name)
    for name,entry in [('preflight','preflight.py'),('smoke','manual_smoke.py')]:
        extra='' if name=='preflight' else ' "$@"'
        script='#!/bin/sh\nset -eu\nbase=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n'
        script+='export LD_LIBRARY_PATH="$base/lib"\n'
        script+=f'exec python3 "$base/platform/{entry}" "$base/emotional-ui"{extra} --sha256 {report["sha256"]}\n'
        path=folder/f'run-{name}.sh';path.write_text(script);path.chmod(0o755)
    (folder/'elf-audit.json').write_text(json.dumps(report,indent=2)+'\n')
    manifest={str(path.relative_to(folder)):hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(folder.rglob('*')) if path.is_file()}
    (folder/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(folder);print('UI has NOT been started. Manual smoke requires independent recovery.')
if __name__=='__main__':main()
