#!/usr/bin/env python3
"""Experimental ES Ports entry. Own-files installer + independent transient recovery.

Not physically certified as a distribution entry until the exact release ZIP is tested.
Never enables a persistent unit or installs a driver. Software is the default.
"""
import argparse
import os
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

import manage
import phase12_preview as recovery


def location():
    return Path.home()/'.local/opt/EmotionalUI-preview-1'


def service_state():
    p = subprocess.run(['systemctl', 'show', recovery.UNIT, '-p', 'ActiveState', '-p', 'MainPID'],
                       check=True, capture_output=True, text=True, timeout=8)
    main = re.search(r'^MainPID=(\d+)$', p.stdout, re.M)
    if 'ActiveState=active' not in p.stdout or not main or int(main[1]) == 0:
        raise ValueError('EmulationStation must be active before starting the trial')
    return int(main[1])


def check_menu_tree(text, main, caller):
    # ES launches this entry as its own child. Exempt only our actual ancestry,
    # not all Python/shell programs; unrelated emulator/compositor checks remain.
    rows = [line.split(None, 2) for line in text.splitlines() if len(line.split(None, 2)) == 3]
    parents = {int(pid): int(parent) for pid, parent, _ in rows}
    names = {int(pid): name for pid, _, name in rows}
    chain = set()
    pid = caller
    while pid != main and pid in parents and pid not in chain:
        chain.add(pid)
        pid = parents[pid]
    if pid != main:
        raise ValueError('Trial entry must be started from the running EmulationStation menu')
    descendants = {caller}
    while True:
        newer = descendants | {pid for pid, parent in parents.items() if parent in descendants}
        if newer == descendants:
            break
        descendants = newer
    ignored = descendants | {pid for pid in chain if names[pid] not in {'emulationstatio', 'emulationstation'}}
    normalized = []
    for pid, parent, name in rows:
        pid, parent = int(pid), int(parent)
        if pid in ignored:
            parent = 0
        else:
            seen = set()
            while parent in ignored and parent not in seen:
                seen.add(parent)
                parent = parents.get(parent, 0)
        # Retain even detached names so recovery's global emulator check still runs.
        normalized.append(f'{pid} {parent} {name}')
    recovery.check_tree('\n'.join(normalized), main)


def session_command(destination, data, runtime, library, renderer='software', token=None):
    destination, data, runtime, library = map(recovery.safe_path, (destination, data, runtime, library))
    if renderer not in {'software', 'femtovg'}:
        raise ValueError('Unsupported renderer')
    token = token or uuid.uuid4().hex
    if not re.fullmatch(r'[a-f0-9]{32}', token):
        raise ValueError('Unsafe session identifier')
    binary = destination/('emotional-ui-gpu' if renderer == 'femtovg' else 'emotional-ui')
    uid = os.getuid()
    if uid == 0:
        raise ValueError('Do not run EmotionalUI as root')
    # PID1 owns this unit even when stopping ES kills its launching process.
    # ExecStopPost runs on successful exit, crash or the trial-only time limit.
    # No diagnostic environment (including GPU_SWAP) is inherited by the app.
    return ['sudo', '-n', 'systemd-run', '--collect', '--no-block',
            '--unit=emotional-ui-trial-'+token,
            '--property=Type=exec', '--property=User='+str(uid),
            '--property=WorkingDirectory='+str(destination),
            '--property=Restart=no', '--property=TimeoutStartSec=20',
            '--property=TimeoutStopSec=15', '--property=RuntimeMaxSec=900',
            '--property=ExecStartPre=+/usr/bin/systemctl stop '+recovery.UNIT,
            '--property=ExecStopPost='+recovery.RESTORE,
            '--property=ReadOnlyPaths='+str(library),
            '--property=UnsetEnvironment=EMOTIONAL_GPU_SWAP EMOTIONAL_GPU_REPLAY EMOTIONAL_PERF EMOTIONAL_PERF_TRACE EMOTIONAL_SCENE_CACHE',
            '--property=StandardOutput=append:'+str(runtime/'stdout.log'),
            '--property=StandardError=append:'+str(runtime/'stderr.log'),
            '--property=Environment=HOME='+str(recovery.safe_path(Path.home()))+
            ' SLINT_BACKEND=linuxkms-'+renderer+' SLINT_SCALE_FACTOR=1 XDG_RUNTIME_DIR=/run/user/'+str(uid)+
            ' EMOTIONAL_ARTWORK_CACHE='+str(data/'artwork-cache'),
            str(binary), '--trial', '--db', str(data/'trial.sqlite'),
            '--library-root', str(library), '--view', 'emotional',
            '--audio-mode', 'minimal', '--audio-volume', '15',
            '--input-profile', str(destination/'input.toml')]


def prepare(package, destination):
    d = manage.manifest(package)
    # Draft/private packages must not become an end-user graphical installation.
    if d.get('public_release') is not True or d.get('version') != '0.1.0-preview.1':
        raise ValueError('Public preview is not cleared; private test bundles cannot be installed by this entry')
    manage.compatible(package, d)
    destination.parent.mkdir(parents=True, exist_ok=True)
    manage.install(package, destination, confirmed=True)
    s = manage.read_state(destination)
    manage.check_owned(destination, s)
    if s['status'] != 'installed':
        raise ValueError('Installation is disabled; restore/uninstall before reinstalling')


def check_no_session():
    state = subprocess.run(['systemctl', 'list-units', '--state=active,activating,deactivating,reloading',
                            '--no-legend', 'emotional-ui-trial-*'], capture_output=True,
                           text=True, check=True, timeout=8)
    if state.stdout.strip():
        raise ValueError('Close EmotionalUI and wait for recovery before starting, restoring or uninstalling')


def launch(package, destination, renderer):
    # All checks before asking PID1 to stop EmulationStation.
    check_no_session()
    prepare(package, destination)
    binary = destination/('emotional-ui-gpu' if renderer == 'femtovg' else 'emotional-ui')
    p = subprocess.run(['/lib/ld-linux-aarch64.so.1', '--list', str(binary)],
                       capture_output=True, text=True, timeout=15)
    if p.returncode or 'not found' in p.stdout+p.stderr:
        raise ValueError('Renderer dependencies unavailable; choose the software entry. '+p.stderr)
    main = service_state()
    tree = subprocess.run(['ps', '-eo', 'pid=,ppid=,comm='], check=True, capture_output=True, text=True, timeout=8)
    check_menu_tree(tree.stdout, main, os.getpid())
    recovery.check_backlight([int(p.read_text()) for p in Path('/sys/class/backlight').glob('*/actual_brightness')])
    # Match the configured Ports library; this only reads the existing ES config.
    import xml.etree.ElementTree as ET
    config = Path.home()/'.emulationstation/es_systems.cfg'
    if not config.is_file():
        config = Path('/etc/emulationstation/es_systems.cfg')
    roots = []
    for system in ET.parse(config).getroot().findall('system'):
        if system.findtext('name') == 'ports':
            ports = Path(os.path.expanduser(system.findtext('path') or ''))
            if ports.is_absolute() and ports.is_dir() and ports.name == 'ports':
                roots.append(ports.parent)
    if len(set(roots)) != 1:
        raise ValueError('Unambiguous existing Ports library required; no guessed ROM location')
    data = recovery.safe_path(Path.home()/'.local/share/emotional-ui')
    runtime = data/('session-'+uuid.uuid4().hex)
    runtime.mkdir(parents=True, mode=0o700)
    # Noninteractive privilege check before screen release. No passwords stored.
    subprocess.run(['sudo', '-n', '/usr/bin/systemctl', 'is-active', recovery.UNIT], check=True, capture_output=True, timeout=8)
    subprocess.run(session_command(destination, data, runtime, roots[0], renderer), check=True,
                   capture_output=True, text=True, timeout=20)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['trial', 'gpu', 'restore', 'uninstall'])
    a = p.parse_args()
    package = Path(__file__).resolve().parent
    destination = location()
    try:
        if a.action in {'trial', 'gpu'}:
            launch(package, destination, 'femtovg' if a.action == 'gpu' else 'software')
        else:
            # These entries do not interrupt a running graphics session.
            check_no_session()
            print(manage.restore(destination))
            if a.action == 'uninstall':
                print(manage.uninstall(destination))
    except (ValueError, OSError, subprocess.SubprocessError) as e:
        print('EmotionalUI: '+str(e)+'\nNo boot, ROM, save or system-library changes were requested.', file=sys.stderr)
        time.sleep(5)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
