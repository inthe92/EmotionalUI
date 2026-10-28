#!/usr/bin/env python3
"""Private functional Ports entry; never installs or changes the boot frontend.

Deploy beside menu.py, manage.py, functional_preview.py and phase12_preview.py.
The reviewed private JSON selects exact paths and hashes. Dry-run is the default;
execution must originate from the running EmulationStation menu.
"""
import argparse
import json
import os
import re
import stat
import subprocess
import sys
import uuid
from pathlib import Path

import menu
import functional_preview
import phase12_preview as recovery

FIELDS = {'binary', 'binary_sha256', 'profile', 'profile_sha256', 'input_profile',
          'db', 'library_root', 'data', 'renderer'}
PATH_FIELDS = {'binary', 'profile', 'input_profile', 'db', 'library_root', 'data'}


def read_config(path):
    path = recovery.safe_path(path)
    info = path.lstat()
    if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid()
            or stat.S_IMODE(info.st_mode) != 0o600 or info.st_size > 65536):
        raise ValueError('Configuration must be a private 0600 regular file owned by the current user')
    config = json.loads(path.read_text())
    if not isinstance(config, dict) or set(config) != FIELDS:
        raise ValueError('Expected the exact reviewed functional configuration fields')
    for field in PATH_FIELDS:
        if not isinstance(config[field], str):
            raise ValueError('Explicit absolute paths are required')
        config[field] = recovery.safe_path(config[field])
    for field in ('binary_sha256', 'profile_sha256'):
        if not isinstance(config[field], str) or not re.fullmatch(r'[a-f0-9]{64}', config[field]):
            raise ValueError('Explicit binary and launcher profile SHA-256 values are required')
    if config['renderer'] not in {'software', 'femtovg'}:
        raise ValueError('Renderer must be software or femtovg')
    return config


def checked_inputs(config, runtime):
    paths = functional_preview.verify_inputs(
        config['binary'], config['binary_sha256'], config['profile'],
        config['profile_sha256'], config['db'], config['library_root'], runtime)
    digest = functional_preview.verify_input_profile(config['input_profile'])
    return paths, digest


def run(argv, timeout=8):
    result = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError('A required system check failed; the display was not handed off')
    return result


def check_no_session():
    # Reuse the public preview guard and additionally exclude functional units,
    # including a recovery helper still waiting for a real game to terminate.
    menu.check_no_session()
    state = run(['/usr/bin/systemctl', 'list-units',
                 '--state=active,activating,deactivating,reloading', '--no-legend',
                 'emotional-ui-functional-*'])
    if state.stdout.strip():
        raise ValueError('Close EmotionalUI and wait for its recovery before starting another session')


def require_cgroup_v2():
    if not Path('/sys/fs/cgroup/cgroup.controllers').is_file():
        raise ValueError('Unified cgroup v2 is required for safe functional recovery')


def backlight_values():
    return [int(path.read_text()) for path in Path('/sys/class/backlight').glob('*/actual_brightness')]


def preflight(binary):
    if os.getuid() == 0:
        raise ValueError('Do not run the functional frontend as root')
    check_no_session()
    main = menu.service_state()
    tree = run(['/usr/bin/ps', '-eo', 'pid=,ppid=,comm='])
    # A shell/SSH caller is rejected; selecting the Ports entry is the explicit
    # local action. Merely setting or clearing SSH_CONNECTION cannot bypass it.
    menu.check_menu_tree(tree.stdout, main, os.getpid())
    require_cgroup_v2()
    recovery.check_backlight(backlight_values())
    loader = run(['/lib/ld-linux-aarch64.so.1', '--list', str(binary)], timeout=15)
    if 'not found' in loader.stdout+loader.stderr:
        raise ValueError('Frontend dependencies are unavailable')
    run(['/usr/bin/sudo', '-n', '/usr/bin/systemctl', 'is-active', recovery.UNIT])


def launch(config_path, execute=False):
    config = read_config(config_path)
    # Verify against the existing private data directory before creating any
    # session files. Dry-run performs no commands and leaves no new directory.
    _, input_digest = checked_inputs(config, config['data'])
    token = uuid.uuid4().hex
    runtime = config['data']/('functional-session-'+token)
    unit = 'emotional-ui-functional-'+token
    argv = functional_preview.command(
        config['binary'], config['profile'], config['db'], config['library_root'],
        runtime, unit, renderer=config['renderer'], input_profile=config['input_profile'])
    result = {'dry_run': not execute, 'unit': unit, 'runtime': str(runtime), 'argv': argv}
    if not execute:
        return result
    preflight(config['binary'])
    # All required menu, loader and recovery checks precede the first write.
    # Retain the private runtime if a later start fails: it may contain useful
    # logs, and removing a directory is not part of normal launch recovery.
    runtime.mkdir(mode=0o700)
    checked_inputs(config, runtime)
    functional_preview.checked_file(config['input_profile'], input_digest)
    # Reject a second trial/functional session that appeared during preflight.
    check_no_session()
    started = subprocess.run(argv, capture_output=True, text=True, timeout=20)
    if started.returncode:
        raise ValueError('Functional session could not be started; EmulationStation recovery is external')
    result['started'] = True
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    try:
        print(json.dumps(launch(args.config, args.execute)))
        return 0
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        # Never echo subprocess output or private configuration contents.
        print('EmotionalUI: '+str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
