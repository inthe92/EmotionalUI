#!/usr/bin/env python3
"""Fetch the pinned official host Zig archive, check SHA-256, extract in a new cache."""
import hashlib,json,platform,posixpath,tarfile,urllib.request
from pathlib import Path
import argparse

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('cache',type=Path);args=p.parse_args()
    lock=json.loads(Path(__file__).with_name('zig.lock.json').read_text())
    machine={'arm64':'aarch64','AMD64':'x86_64'}.get(platform.machine(),platform.machine())
    host=machine+'-'+{'Darwin':'macos','Linux':'linux'}[platform.system()]
    info=lock['hosts'][host];root=args.cache.resolve();root.mkdir(parents=True,exist_ok=True)
    archive=root/Path(info['tarball']).name
    if not archive.exists():
        with urllib.request.urlopen(info['tarball'],timeout=90) as r:archive.write_bytes(r.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest()!=info['shasum']:raise SystemExit('Zig checksum mismatch')
    with tarfile.open(archive) as tar:
        top=Path(tar.getmembers()[0].name).parts[0];destination=root/top
        if destination.exists():
            if not (destination/'.emotional-zig').exists():raise SystemExit('Refusing to overwrite an existing tool directory')
        else:
            for m in tar.getmembers():
                path=Path(m.name)
                if path.is_absolute() or '..' in path.parts:raise ValueError('Unsafe tool archive')
                if m.issym() or m.islnk():
                    target=posixpath.normpath(str(path.parent/m.linkname))
                    if not (target==top or target.startswith(top+'/')):raise ValueError('Tool symlink escape')
            tar.extractall(root)
            (destination/'.emotional-zig').write_text(info['shasum']+'\n')
    print(destination/'zig')
if __name__=='__main__':main()
