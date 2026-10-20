#!/usr/bin/env python3
"""Download a pinned Debian ARM64 *reference* SDK. No package scripts/installers run.
This is not a dump of the handheld. Validate against its diagnostic before deployment.
"""
import argparse,hashlib,io,json,lzma,posixpath,subprocess,tarfile,urllib.request
from pathlib import Path,PurePosixPath
REPO='https://deb.debian.org/debian/'
ROOTS=['libinput-dev','libudev-dev','libxkbcommon-dev','libseat-dev','libfontconfig-dev','libfreetype-dev','libexpat1-dev','zlib1g-dev','libpng-dev','libbrotli-dev']
def download(url):
    with urllib.request.urlopen(url,timeout=90) as r:return r.read()
def select_packages():
    raw=download(REPO+'dists/bookworm/main/binary-arm64/Packages.xz')
    packages={}
    for paragraph in lzma.decompress(raw).decode().split('\n\n'):
        record={}
        for line in paragraph.splitlines():
            if line and not line[0].isspace() and ': ' in line:
                k,v=line.split(': ',1);record[k]=v
        if 'Package' in record:packages[record['Package']]=record
    needed=list(ROOTS);selected={}
    while needed:
        name=needed.pop()
        if name in selected:continue
        p=packages[name];selected[name]=p
        for field in ['Depends','Pre-Depends']:
            for group in p.get(field,'').split(','):
                if not group.strip():continue
                alternatives=[part.strip().split()[0].split(':')[0] for part in group.split('|')]
                candidate=next((x for x in alternatives if x in packages),None)
                if candidate and (candidate.startswith('lib') or candidate.startswith('zlib')) and not candidate.endswith(('-dev','-bin','-data')):
                    needed.append(candidate)
    return {'reference':'Debian 12 bookworm arm64; NOT identified device','repository':REPO,'index_sha256':hashlib.sha256(raw).hexdigest(),'packages':[{k:p[k] for k in ('Package','Version','Filename','SHA256')} for _,p in sorted(selected.items())]}
def safe_member_path(name):
    path=PurePosixPath(name)
    if path.is_absolute() or '..' in path.parts:raise ValueError('Unsafe archive path')
    return path

def unpack(archive,root):
    members=subprocess.check_output(['ar','t',str(archive)]).decode().splitlines()
    payload=next(n for n in members if n.startswith('data.tar.'))
    data=subprocess.check_output(['ar','p',str(archive),payload])
    with tarfile.open(fileobj=io.BytesIO(data),mode='r:*') as tar:
        links=[]
        for member in tar:
            rel=safe_member_path(member.name);dest=root/str(rel)
            if str(rel)=='.' and member.isdir():continue
            dest.parent.mkdir(parents=True,exist_ok=True)
            if not dest.parent.resolve().is_relative_to(root):raise ValueError('Parent escape')
            if member.isdir():dest.mkdir(exist_ok=True);continue
            if member.issym():
                target=member.linkname
                normalized=posixpath.normpath(target.lstrip('/') if target.startswith('/') else str(rel.parent/target))
                safe_member_path(normalized)
                target_rel=posixpath.relpath(normalized,str(rel.parent))
                if dest.is_symlink():dest.unlink()
                elif dest.exists():raise ValueError('Refusing to replace non-symlink with symlink')
                dest.symlink_to(target_rel);continue
            if member.islnk():links.append((dest,root/str(safe_member_path(member.linkname))));continue
            if not member.isfile():raise ValueError('Unexpected special file')
            if dest.is_symlink():raise ValueError('Refusing to write through symlink')
            dest.write_bytes(tar.extractfile(member).read());dest.chmod(member.mode&0o777)
        for dest,target in links:
            if not target.resolve().is_relative_to(root):raise ValueError('Hardlink escape')
            dest.write_bytes(target.read_bytes())
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('destination',type=Path);p.add_argument('--lock',type=Path,default=Path(__file__).with_name('sysroot.lock.json'));p.add_argument('--resolve-lock',action='store_true');args=p.parse_args()
    if args.resolve_lock:
        if args.lock.exists():raise SystemExit('Lock exists; do not overwrite it implicitly')
        args.lock.write_text(json.dumps(select_packages(),indent=2)+'\n')
    lock=json.loads(args.lock.read_text());root=args.destination.resolve();root.mkdir(parents=True,exist_ok=True)
    if any(root.iterdir()) and not (root/'.emotional-sdk').exists():raise SystemExit('Destination is not an EmotionalUI SDK')
    (root/'.emotional-sdk').write_text('Reference SDK only; no hardware compatibility claim.\n')
    cache=root.parent/'packages';cache.mkdir(exist_ok=True)
    for package in lock['packages']:
        path=cache/(package['SHA256']+'.deb')
        if not path.exists():path.write_bytes(download(lock['repository']+package['Filename']))
        if hashlib.sha256(path.read_bytes()).hexdigest()!=package['SHA256']:raise ValueError('Package checksum mismatch')
        unpack(path,root);print(package['Package'],package['Version'],flush=True)
    (root/'emotional-sdk-lock.json').write_text(json.dumps(lock,indent=2)+'\n')
if __name__=='__main__':main()
