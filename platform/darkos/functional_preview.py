#!/usr/bin/env python3
"""Private, supervised game-launch test. Dry-run by default; no installation.

The supervisor owns the real game process. There is no game-duration watchdog.
Recovery runs as the current user and restores EmulationStation only after this
transient unit has no other live processes. Unknown ownership fails closed.
"""
import argparse
import hashlib
import json
import os
import pwd
import re
import stat
import struct
import subprocess
import time
import tomllib
import uuid
from pathlib import Path

import phase12_preview as recovery

UNIT_PATTERN = r'emotional-ui-functional-[a-f0-9]{32}'
INPUT_ACTIONS = {'left', 'right', 'up', 'down', 'confirm', 'back', 'refresh',
                 'library', 'menu', 'previous', 'next', 'reduced_motion', 'sound',
                 'details', 'preview_previous', 'preview_next', 'moment_previous', 'moment_next'}
NAVIGATION = {'left', 'right', 'up', 'down'}
MAX_PROFILES = 128
MAX_CONFIGURATION_BYTES = 512 * 1024


def unit_name(unit):
    if not re.fullmatch(UNIT_PATTERN, unit):
        raise ValueError('A fresh functional UUID unit is required')
    return unit


def checked_file(path, digest):
    path = recovery.safe_path(path)
    if not re.fullmatch(r'[a-f0-9]{64}', digest):
        raise ValueError('An explicit SHA-256 is required')
    if path.is_symlink() or not path.is_file():
        raise ValueError('Expected a regular, non-symlink file')
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError('SHA-256 mismatch; no handoff performed')
    return path


def arm64_elf(path):
    with path.open('rb') as source:
        data = source.read(64)
    if (len(data) != 64 or data[:6] != b'\x7fELF\x02\x01'
            or struct.unpack_from('<H', data, 18)[0] != 183
            or struct.unpack_from('<H', data, 16)[0] not in (2, 3)):
        raise ValueError('An ARM64 ELF executable is required; shell wrappers are not accepted')
    if not os.access(path, os.X_OK):
        raise ValueError('Executable permission is missing')


def emulator_elf(path, shared=False):
    """Direct installed ARM binaries only; never execute system shell wrappers."""
    with path.open('rb') as source:
        data = source.read(64)
    if len(data) < 52 or data[:4] != b'\x7fELF' or data[5] != 1:
        raise ValueError('A direct ARM ELF is required; shell wrappers are not accepted')
    architecture = (data[4], struct.unpack_from('<H', data, 18)[0])
    kind = struct.unpack_from('<H', data, 16)[0]
    if architecture not in {(1, 40), (2, 183)} or kind not in ({3} if shared else {2, 3}):
        raise ValueError('Emulator/core must be a compatible ARM ELF')
    if not shared and not os.access(path, os.X_OK):
        raise ValueError('Emulator executable permission is missing')
    return architecture


def verify_inputs(binary, candidate_sha256, profile, profile_sha256, db, library_root, runtime):
    binary = checked_file(binary, candidate_sha256)
    arm64_elf(binary)
    profile = checked_file(profile, profile_sha256)
    if profile.stat().st_size > MAX_CONFIGURATION_BYTES:
        raise ValueError('Launcher profile exceeds the bounded document size')
    config = tomllib.loads(profile.read_text())
    launchers = config.get('launchers', [])
    if (config.get('handoff') != 'process-exit' or not isinstance(launchers, list)
            or not 1 <= len(launchers) <= MAX_PROFILES or not all(isinstance(p, dict) for p in launchers)):
        raise ValueError('An explicit process-exit launcher profile is required')
    library_root = recovery.safe_path(library_root)
    if not library_root.is_dir():
        raise ValueError('The explicit library root is unavailable')
    root = library_root.resolve(strict=True)
    ids = set()
    for launcher in launchers:
        ident = launcher.get('id')
        if not isinstance(ident, str) or not ident or ident in ids:
            raise ValueError('Launcher identifiers must be unique')
        ids.add(ident)
        if launcher.get('waits_for_exit') is not True:
            raise ValueError('The verified foreground-process contract is required')
        allowed = recovery.safe_path(launcher['root']).resolve(strict=True)
        if not allowed.is_dir() or not allowed.is_relative_to(root):
            raise ValueError('Launcher root must belong to the explicit library')
        executable = recovery.safe_path(launcher['executable']).resolve(strict=True)
        architecture = emulator_elf(executable)
        if not recovery.safe_path(launcher['working_directory']).is_dir():
            raise ValueError('Launcher working directory is unavailable')
        args = launcher.get('arguments', [])
        if (not isinstance(args, list) or len(args) > 64
                or args.count('{rom}') != 1
                or any(not isinstance(a, str) or len(a) > 8192 or '\0' in a
                       or ('{' in a and a != '{rom}') for a in args)):
            raise ValueError('Expected bounded argv with one isolated ROM argument')
        extensions = launcher.get('extensions', [])
        if (not isinstance(extensions, list) or len(extensions) > 128
                or any(not isinstance(e, str) or not re.fullmatch(r'\.[A-Za-z0-9]{1,23}', e) for e in extensions)):
            raise ValueError('Expected bounded file extensions for the authorized system')
        if '-L' in args:
            index = args.index('-L')
            if index + 1 >= len(args):
                raise ValueError('Missing explicitly configured libretro core')
            core = recovery.safe_path(args[index+1]).resolve(strict=True)
            if emulator_elf(core, shared=True) != architecture:
                raise ValueError('Emulator and libretro core architectures differ')
    db = recovery.safe_path(db)
    if db.is_symlink() or (db.exists() and not db.is_file()) or not db.parent.is_dir():
        raise ValueError('An explicit own database path with an existing parent is required')
    runtime = recovery.safe_path(runtime)
    if (not runtime.is_dir() or runtime.is_symlink()
            or runtime.stat().st_uid != os.getuid()
            or stat.S_IMODE(runtime.stat().st_mode) & 0o077):
        raise ValueError('A private, current-user-owned runtime directory is required')
    return binary, profile, db, library_root, runtime


def verify_input_profile(path):
    """Mirror the bounded Rust profile contract before any display handoff.

    This validates configuration, not physical controller operation. The app
    still checks evdev identity/capabilities when opening its own input worker.
    Return a digest so a changed profile cannot pass the later preflight.
    """
    if path is None:
        raise ValueError('A verified --input-profile is required for a physical functional test')
    path = recovery.safe_path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 65536:
        raise ValueError('A bounded regular input profile is required')
    data = path.read_bytes()
    profile = tomllib.loads(data.decode('utf-8'))
    if (set(profile) - {'enabled', 'device_name', 'device_path', 'mapping'}
            or profile.get('enabled') is not True
            or not isinstance(profile.get('device_name'), str)
            or not 1 <= len(profile['device_name'].encode('utf-8')) <= 128
            or '\0' in profile['device_name']):
        raise ValueError('Input profile must be enabled and identify the controller')
    if 'device_path' in profile:
        device = recovery.safe_path(profile['device_path'])
        if device.parent != Path('/dev/input') or not re.fullmatch(r'event\d+', device.name):
            raise ValueError('Explicit controller paths must identify an evdev event device')
    mapping = profile.get('mapping')
    if not isinstance(mapping, dict) or set(mapping) - {'buttons', 'axes'}:
        raise ValueError('Invalid input mapping')
    buttons, axes = mapping.get('buttons', []), mapping.get('axes', [])
    if (not isinstance(buttons, list) or not isinstance(axes, list)
            or len(buttons) > 64 or len(axes) > 8):
        raise ValueError('Input mapping exceeds the supported size')
    actions, codes = set(), set()
    for button in buttons:
        if (not isinstance(button, dict) or set(button) != {'code', 'action'}
                or type(button.get('code')) is not int or not 0 <= button['code'] <= 0x2ff
                or button['code'] in codes or button.get('action') not in INPUT_ACTIONS):
            raise ValueError('Invalid or duplicate input button mapping')
        codes.add(button['code'])
        actions.add(button['action'])
    codes.clear()
    for axis in axes:
        if (not isinstance(axis, dict)
                or set(axis) - {'code', 'min', 'max', 'negative', 'positive', 'engage_percent', 'release_percent'}
                or any(type(axis.get(key)) is not int for key in ('code', 'min', 'max'))
                or not 0 <= axis['code'] <= 0x3f or axis['code'] in codes
                or not -(2**31) <= axis['min'] < axis['max'] < 2**31
                or axis.get('negative') not in NAVIGATION or axis.get('positive') not in NAVIGATION
                or axis['negative'] == axis['positive']):
            raise ValueError('Invalid or duplicate input axis mapping')
        engage, release = axis.get('engage_percent', 35), axis.get('release_percent', 20)
        if type(engage) is not int or type(release) is not int or not 0 <= release < engage <= 95:
            raise ValueError('Invalid input axis thresholds')
        codes.add(axis['code'])
        actions.update((axis['negative'], axis['positive']))
    if not (NAVIGATION | {'confirm', 'back', 'menu'}).issubset(actions):
        raise ValueError('Input profile lacks essential navigation actions')
    return hashlib.sha256(data).hexdigest()


def command(binary, profile, db, library_root, runtime, unit, renderer='software',
            input_profile=None, uid=None, home=None, script=None):
    """Build argv only. Call verify_inputs and preflight before executing it."""
    binary, profile, db, library_root, runtime = map(
        recovery.safe_path, (binary, profile, db, library_root, runtime))
    unit = unit_name(unit)
    if renderer not in {'software', 'femtovg'}:
        raise ValueError('An explicit supported renderer is required')
    uid = os.getuid() if uid is None else uid
    if not isinstance(uid, int) or uid <= 0:
        raise ValueError('The frontend and recovery helper must not run as root')
    home = recovery.safe_path(home if home else pwd.getpwuid(uid).pw_dir)
    script = recovery.safe_path(script if script else Path(__file__).resolve())
    # No '+' prefix here: only systemctl's fixed ExecStartPre is privileged.
    restore = f'/usr/bin/python3 {script} recover --unit {unit}'
    argv = [
        '/usr/bin/sudo', '-n', '/usr/bin/systemd-run', '--collect', '--no-block',
        '--unit='+unit,
        '--property=Type=exec', '--property=User='+str(uid),
        '--property=WorkingDirectory='+str(binary.parent),
        '--property=Restart=no', '--property=TimeoutStartSec=20',
        '--property=TimeoutStopSec=infinity', '--property=KillMode=process',
        '--property=SendSIGKILL=no', '--property=Delegate=no',
        '--property=ProtectControlGroups=yes',
        '--property=ExecStartPre=+/usr/bin/systemctl stop '+recovery.UNIT,
        '--property=ExecStopPost='+restore,
        # First launch validation uses isolated saves outside the original ROM
        # root; protect that entire library against accidental emulator writes.
        '--property=ReadOnlyPaths='+str(library_root)+' -'+str(home/'.config/retroarch')+' -'+str(home/'.config/retroarch32')+' -/opt/drastic',
        '--property=UnsetEnvironment=EMOTIONAL_GPU_SWAP EMOTIONAL_GPU_REPLAY EMOTIONAL_PERF EMOTIONAL_PERF_TRACE EMOTIONAL_SCENE_CACHE',
        '--property=StandardOutput=append:'+str(runtime/'stdout.log'),
        '--property=StandardError=append:'+str(runtime/'stderr.log'),
        '--property=Environment=HOME='+str(home)
        +' SLINT_BACKEND=linuxkms-'+renderer+' SLINT_SCALE_FACTOR=1'
        +' XDG_RUNTIME_DIR=/run/user/'+str(uid)
        +' EMOTIONAL_ARTWORK_CACHE='+str(runtime/'artwork-cache'),
        str(binary), '--functional', '--launcher-config', str(profile),
        '--launch-physical-supervised', '--db', str(db),
        '--library-root', str(library_root), '--view', 'emotional',
    ]
    if input_profile:
        argv.extend(['--input-profile', str(recovery.safe_path(input_profile))])
    return argv


def cgroup_directory(unit, membership=Path('/proc/self/cgroup'), root=Path('/sys/fs/cgroup')):
    unit = unit_name(unit)+'.service'
    rows = membership.read_text().splitlines()
    unified = [row[3:] for row in rows if row.startswith('0::')]
    # Do not accept another service, the user slice or a caller-supplied cgroup.
    expected = '/system.slice/'+unit
    if unified != [expected]:
        raise ValueError('Recovery is not running inside its exact unified functional unit')
    root = root.resolve(strict=True)
    group = root/expected.lstrip('/')
    if group.is_symlink() or group.resolve(strict=True) != group:
        raise ValueError('Unexpected cgroup path')
    return group


def live_process(pid, proc=Path('/proc')):
    try:
        text = (proc/str(pid)/'stat').read_text()
    except FileNotFoundError:
        return False
    # Parentheses in the comm field may contain spaces and closing parens.
    end = text.rfind(')')
    if end < 0 or not text[end+1:].strip():
        raise ValueError('Unreadable process state; independent SSH recovery required')
    return text[end+1:].split()[0] not in {'Z', 'X'}


def other_processes(group, self_pid, alive=live_process):
    pids = set()
    # Descendants inherit the unit cgroup; check any nested cgroups too. A read
    # error is never interpreted as an empty group or permission to draw ES.
    for current, directories, files in os.walk(group, followlinks=False,
                                              onerror=lambda e: (_ for _ in ()).throw(e)):
        current = Path(current)
        if any((current/d).is_symlink() for d in directories):
            raise ValueError('Symlink in recovery cgroup')
        if 'cgroup.procs' not in files:
            raise ValueError('Incomplete cgroup process inventory')
        raw = (current/'cgroup.procs').read_text()
        for item in raw.split():
            if not item.isdecimal() or int(item) <= 0:
                raise ValueError('Invalid cgroup PID inventory')
            pids.add(int(item))
    if self_pid not in pids:
        raise ValueError('Recovery PID is not a member of its functional unit')
    return [pid for pid in pids if pid != self_pid and alive(pid)]


def recover(unit, membership=Path('/proc/self/cgroup'), root=Path('/sys/fs/cgroup'),
            self_pid=None, alive=live_process, pause=time.sleep, run=subprocess.run):
    group = cgroup_directory(unit, membership, root)
    self_pid = os.getpid() if self_pid is None else self_pid
    # No timeout and no process killing: a long game must finish normally. The
    # caller's separate SSH session can diagnose an indefinitely blocked unit.
    while other_processes(group, self_pid, alive):
        pause(0.25)
    result = run(['/usr/bin/sudo', '-n', '/usr/bin/systemctl', 'start', recovery.UNIT],
                 capture_output=True, text=True, timeout=20)
    if result.returncode:
        raise ValueError('EmulationStation restoration failed; use independent SSH recovery')


def preflight(binary):
    if os.getuid() == 0 or not os.environ.get('SSH_CONNECTION'):
        raise ValueError('Use the authorized non-root SSH session')
    if not Path('/sys/fs/cgroup/cgroup.controllers').is_file():
        raise ValueError('Unified cgroup v2 is required for safe functional recovery')
    def run(argv):
        result = subprocess.run(argv, capture_output=True, text=True, timeout=15)
        if result.returncode:
            raise ValueError('Read-only preflight command failed; no handoff performed')
        return result.stdout
    state = run(['/usr/bin/systemctl', 'show', recovery.UNIT, '-p', 'ActiveState', '-p', 'MainPID'])
    pid = re.search(r'^MainPID=(\d+)$', state, re.M)
    if 'ActiveState=active' not in state or not pid or int(pid[1]) == 0:
        raise ValueError('EmulationStation must be active before handoff')
    recovery.check_tree(run(['/usr/bin/ps', '-eo', 'pid=,ppid=,comm=']), int(pid[1]))
    recovery.check_backlight([int(p.read_text()) for p in Path('/sys/class/backlight').glob('*/actual_brightness')])
    loader = run(['/lib/ld-linux-aarch64.so.1', '--list', str(binary)])
    if 'not found' in loader:
        raise ValueError('Frontend dependencies are unavailable')
    run(['/usr/bin/sudo', '-n', '/usr/bin/systemctl', 'is-active', recovery.UNIT])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='action', required=True)
    start = commands.add_parser('start')
    for option in ('binary', 'profile', 'db', 'library-root', 'runtime'):
        start.add_argument('--'+option, type=Path, required=True)
    start.add_argument('--candidate-sha256', required=True)
    start.add_argument('--profile-sha256', required=True)
    start.add_argument('--renderer', choices=['software', 'femtovg'], default='software')
    start.add_argument('--input-profile', type=Path)
    start.add_argument('--unit', default='emotional-ui-functional-'+uuid.uuid4().hex)
    for flag in ('execute', 'supervised', 'recovery-confirmed', 'second-session-confirmed', 'isolated-saves-confirmed'):
        start.add_argument('--'+flag, action='store_true')
    restore = commands.add_parser('recover')
    restore.add_argument('--unit', required=True)
    args = parser.parse_args()
    try:
        if args.action == 'recover':
            recover(args.unit)
            return 0
        paths = verify_inputs(args.binary, args.candidate_sha256, args.profile,
                              args.profile_sha256, args.db, args.library_root, args.runtime)
        input_digest = verify_input_profile(args.input_profile) if args.input_profile else None
        argv = command(*paths, args.unit, args.renderer, args.input_profile)
        if not args.execute:
            print(json.dumps({'dry_run': True, 'argv': argv,
                              'physical_validation': 'NOT_PROBED',
                              'save_isolation': 'requires explicit reviewed profile'}))
            return 0
        if not all((args.supervised, args.recovery_confirmed, args.second_session_confirmed,
                    args.isolated_saves_confirmed)):
            raise ValueError('Current supervision, independent recovery and reviewed isolated saves are required')
        if input_digest is None:
            raise ValueError('A verified --input-profile is required for a physical functional test')
        preflight(paths[0])
        # Recheck identities after preflight; never replace an authorized build
        # or profile with changed bytes between the checks and unit creation.
        checked_file(args.binary, args.candidate_sha256)
        checked_file(args.profile, args.profile_sha256)
        checked_file(args.input_profile, input_digest)
        result = subprocess.run(argv, capture_output=True, text=True, timeout=20)
        if result.returncode:
            raise ValueError('Transient functional unit did not start; inspect independent recovery')
        print(json.dumps({'unit': args.unit, 'started': True, 'physical_validation': 'NOT_PROBED'}))
        return 0
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        # Do not echo subprocess stdout/stderr, network metadata or credentials.
        print('Functional test refused: '+str(error), file=__import__('sys').stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
