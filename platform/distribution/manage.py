#!/usr/bin/env python3
"""Own-files-only trial installer. Never edits services, ES, ROMs, saves or boot.
Python >=3.8 standard library. A checksum is integrity, not publisher authentication.
"""
import argparse,hashlib,json,os,platform,re,shutil,stat,struct,subprocess,sys
from pathlib import Path,PurePosixPath
STATE='.emotional-install.json'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def relative(name):
 p=PurePosixPath(name)
 if not name or p.is_absolute() or '..' in p.parts or str(p)!=name or name==STATE or '\\' in name:raise ValueError('Invalid owned path')
 return p
def plain_tree(root):
 if root.is_symlink():raise ValueError('Symbolic installation root refused')
 # Reject symlink parents too; do not follow redirected user directories while mutating.
 if any(p.is_symlink() for p in [root,*root.parents]):raise ValueError('Symbolic parent refused')
 return root

def manifest(package):
 plain_tree(package)
 d=json.loads((package/'package.json').read_text())
 if d.get('schema')!=1 or d.get('kind')!='emotional-ui-trial' or d.get('architecture')!='aarch64' or d.get('main_install_enabled') is not False:raise ValueError('Unsupported package')
 if not isinstance(d.get('files'),dict) or not d['files']:raise ValueError('Empty package')
 for name,sha in d['files'].items():
  relative(name);p=package/name;plain_tree(p)
  if not p.is_file() or digest(p)!=sha:raise ValueError('Package checksum mismatch: '+name)
 return d

def compatible(package,d):
 if platform.system()!='Linux' or platform.machine() not in ('aarch64','arm64'):raise ValueError('Requires Linux ARM64; no installation performed')
 model=Path('/proc/device-tree/model')
 if not model.is_file() or not re.search(rb'(?<![A-Z0-9])RG353V(?![A-Z0-9])',model.read_bytes().upper()):raise ValueError('RG353V identity not established; do not install blindly')
 distro=Path('/etc/os-release').read_text().lower()
 if 'darkos' not in distro:raise ValueError('dArkOS identity not established; generic ArkOS is not certification')
 data=(package/'emotional-ui').read_bytes()
 if data[:6]!=b'\x7fELF\x02\x01' or len(data)<64 or struct.unpack_from('<H',data,18)[0]!=183:raise ValueError('Requires ELF64 AArch64')
 loader=Path('/lib/ld-linux-aarch64.so.1')
 if not loader.is_file():raise ValueError('Required glibc loader unavailable')
 p=subprocess.run([str(loader),'--list',str(package/'emotional-ui')],capture_output=True,text=True,timeout=15)
 if p.returncode or 'not found' in p.stdout+p.stderr:raise ValueError('Native dependencies unresolved; '+p.stderr)
 # Installing own files is not permission to replace the frontend or release DRM.
 return {'scope':'loader-only; graphics/input/return not certified','loader_output':p.stdout}

def write_state(root,state):
 p=root/STATE;tmp=root/(STATE+'.new')
 if tmp.exists() or tmp.is_symlink():raise ValueError('Unresolved state write; inspect '+str(tmp))
 with tmp.open('x') as f:
  json.dump(state,f,sort_keys=True,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
 os.replace(tmp,p)

def read_state(root):
 plain_tree(root);p=root/STATE;plain_tree(p)
 if p.stat().st_size>1024*1024:raise ValueError('Oversized install state')
 s=json.loads(p.read_text())
 if s.get('schema')!=1 or s.get('kind')!='emotional-ui-owned-files' or s.get('destination')!=str(root.resolve()):raise ValueError('Unowned installation')
 if not isinstance(s.get('files'),dict):raise ValueError('Invalid manifest')
 for name in s['files']:relative(name)
 return s

def check_owned(root,s,allow_missing=False):
 known=set(s['files'])|{STATE}
 for p in root.rglob('*'):
  if p.is_symlink():raise ValueError('Symlink conflict: '+str(p))
  if p.is_file() and p.relative_to(root).as_posix() not in known:raise ValueError('Unowned file conflict: '+str(p))
 for name,sha in s['files'].items():
  p=root/name
  if not p.exists() and allow_missing:continue
  if not p.is_file() or digest(p)!=sha:raise ValueError('Modified/missing owned file: '+name)

def install(package,destination,confirmed=False,free_bytes=None):
 if not confirmed:raise ValueError('Confirm own-files trial installation with --yes; main frontend replacement is unavailable')
 package=plain_tree(package.absolute());destination=plain_tree(destination.absolute());d=manifest(package)
 if package==destination or package in destination.parents or destination in package.parents:raise ValueError('Package and destination must be separate')
 files=dict(d['files']);files['package.json']=digest(package/'package.json')
 if destination.exists():
  old=read_state(destination);check_owned(destination,old)
  if old['files']!=files:raise ValueError('Different release exists; preserve it and choose a new own-files destination')
  if old['status']!='installed':raise ValueError('Restored installation retained; use a new destination or uninstall it explicitly')
  return 'already-installed'
 parent=destination.parent
 if not parent.is_dir():raise ValueError('Create/choose the destination parent first')
 required=sum((package/n).stat().st_size for n in files)+16*1024*1024
 if (shutil.disk_usage(parent).free if free_bytes is None else free_bytes)<required:raise ValueError('Insufficient free space')
 stage=destination.with_name(destination.name+'.installing');plain_tree(stage)
 if stage.exists():
  s=read_state(stage)
  if s['files']!=files or s['status']!='copying':raise ValueError('Unknown interrupted installation')
  check_owned(stage,s,allow_missing=True)
 else:
  stage.mkdir(mode=0o700)
  s={'schema':1,'kind':'emotional-ui-owned-files','destination':str(stage.resolve()),'files':files,'status':'copying','system_changes':[],'backup_required':False}
  write_state(stage,s)
 for name in sorted(files):
  p=stage/name
  if p.exists():continue
  p.parent.mkdir(parents=True,exist_ok=True)
  with (package/name).open('rb') as source,p.open('xb') as out:
   shutil.copyfileobj(source,out);out.flush();os.fsync(out.fileno())
  os.chmod(p,stat.S_IMODE((package/name).stat().st_mode)&0o755)
 check_owned(stage,s)
 # Final state records the intended location before atomic promotion. A crash here
 # is detected rather than overwriting it on retry; independent uninstall can inspect it.
 s['status']='installed';s['destination']=str(destination.resolve());write_state(stage,s)
 os.rename(stage,destination)
 return 'installed-own-files-only'

def restore(destination):
 destination=plain_tree(destination.absolute());s=read_state(destination);check_owned(destination,s)
 if s.get('system_changes')!=[]:raise ValueError('Unsupported restoration journal; no unknown system changes will be applied')
 if s['status']=='restored':return 'already-restored'
 s['status']='restored';write_state(destination,s)
 return 'restored-no-system-changes; trial disabled; personal database retained separately'

def uninstall(destination):
 destination=plain_tree(destination.absolute());s=read_state(destination)
 if s['status']!='restored':raise ValueError('Run restore.sh before uninstalling')
 check_owned(destination,s)
 for name in sorted(s['files']): (destination/name).unlink()
 (destination/STATE).unlink()
 for p in sorted(destination.rglob('*'),key=lambda p:len(p.parts),reverse=True):
  if p.is_dir():p.rmdir()
 destination.rmdir();return 'uninstalled-owned-files; personal data preserved'

def trial(destination,root=None,display_released=False,recovery=False):
 destination=plain_tree(destination.absolute());s=read_state(destination);check_owned(destination,s)
 if s['status']!='installed':raise ValueError('Trial is not enabled')
 if not display_released or not recovery:raise ValueError('Manual supervised trial requires verified independent recovery and display release. No service is stopped by this tool.')
 data=Path(os.environ.get('XDG_DATA_HOME',str(Path.home()/'.local/share')))/'emotional-ui'
 data.mkdir(parents=True,exist_ok=True,mode=0o700)
 args=[str(destination/'emotional-ui'),'--trial','--db',str(data/'trial.sqlite'),'--audio-mode','silence']
 if root is not None:args+=['--library-root',str(root)]
 # No shell, no ES stop/start and no game launcher; caller owns verified handoff.
 return subprocess.call(args,cwd=destination)

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['install','restore','uninstall','trial','install-main','verify']);p.add_argument('--package',type=Path,default=Path(__file__).resolve().parent);p.add_argument('--destination',type=Path,default=Path.home()/'.local/opt/EmotionalUI');p.add_argument('--library-root',type=Path);p.add_argument('--yes',action='store_true');p.add_argument('--display-released',action='store_true');p.add_argument('--recovery-confirmed',action='store_true');a=p.parse_args()
 try:
  if a.action=='install-main':raise ValueError('Main installation unavailable: independent physical recovery and frontend integration have not been verified. Nothing changed.')
  if a.action in ('install','verify'):
   d=manifest(a.package);evidence=compatible(a.package,d)
   if a.action=='verify':print(json.dumps(evidence,indent=2));return
   if not a.yes:raise ValueError('Confirm own-files installation with --yes; nothing changed')
   plain_tree(a.destination.absolute())
   a.destination.parent.mkdir(parents=True,exist_ok=True)
   print(install(a.package,a.destination,a.yes))
  elif a.action=='restore':print(restore(a.destination))
  elif a.action=='uninstall':
   if not a.yes:raise ValueError('Confirm uninstall with --yes; only verified owned files will be removed')
   print(uninstall(a.destination))
  else:raise SystemExit(trial(a.destination,a.library_root,a.display_released,a.recovery_confirmed))
 except (ValueError,OSError,KeyError,subprocess.SubprocessError) as e:raise SystemExit(str(e))
if __name__=='__main__':main()
