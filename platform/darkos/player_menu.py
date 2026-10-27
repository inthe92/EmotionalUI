#!/usr/bin/env python3
"""Private functional Ports entry using installed emulators and original Options.

User launches this entry locally; all read-only preflight checks precede handoff.
No boot replacement, drivers, ES/XML writes or automatic game duration limit.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid
import xml.etree.ElementTree as ET

import darkos_launcher_profiles as profiles
import functional_menu
import functional_preview
import phase12_preview as recovery


def installed_systems(home=None):
    home = Path.home() if home is None else home
    local = home / '.emulationstation/es_systems.cfg'
    config = local if local.is_file() else Path('/etc/emulationstation/es_systems.cfg')
    raw = config.read_bytes()
    if len(raw) > 8*1024*1024 or b'<!DOCTYPE' in raw.upper() or b'<!ENTITY' in raw.upper():
        raise ValueError('Invalid installed system inventory')
    document = ET.fromstring(raw)
    roots = set()
    options = set()
    for system in document.findall('system'):
        value = system.findtext('path') or ''
        path = Path(str(home)+value[1:] if value.startswith('~/') else value)
        if not path.is_absolute() or not path.is_dir():
            continue
        path = path.resolve()
        if system.findtext('name') == 'ports' and path.name == 'ports':
            roots.add(path.parent)
        if path == Path('/opt/system') and '.sh' in (system.findtext('extension') or '').lower():
            options.add(path)
    if not options and Path('/opt/system').is_dir():
        options.add(Path('/opt/system').resolve())
    if len(roots) != 1 or len(options) > 1:
        raise ValueError('The installed library/Options paths are ambiguous')
    return config, roots.pop(), next(iter(options), None)


def write_private(path, data):
    if path.exists() or path.is_symlink():
        if path.is_symlink() or path.stat().st_uid != os.getuid() or path.read_bytes() != data:
            raise ValueError('Existing private configuration changed; preserved without overwriting')
        return
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'wb') as out:
        out.write(data)
        out.flush()
        os.fsync(out.fileno())


def materialize(plan, output):
    for directory in sorted(set(plan['directories']) | {str(output.parent)}, key=lambda p: len(Path(p).parts)):
        profiles.private_directory(Path(directory))
    for name, text in plan['append_configs'].items():
        write_private(Path(name), text.encode())
    write_private(output, plan['profiles'].encode())


def session_command(destination, profile, data, library, runtime, token):
    command = functional_preview.command(destination/'emotional-ui', profile,
        data/'library.sqlite', library, runtime, 'emotional-ui-functional-'+token,
        renderer='femtovg', input_profile=destination/'input.toml')
    # Original Option tools must be able to use their own configuration. The
    # game's private config disables writing back; original ROMs stay read-only.
    command = [arg for arg in command if not arg.startswith('--property=ReadOnlyPaths=')]
    command.insert(4, '--property=ReadOnlyPaths='+str(library))
    index = next(i for i,arg in enumerate(command) if arg.startswith('--property=Environment='))
    command[index] += ' EMOTIONAL_MATERIAL_GPU=1'
    command.extend(['--atlas', str(destination/'emotional-atlas.db'), '--atlas-experimental'])
    command.extend(['--quit-file', str(runtime/'quit.request')])
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    destination = recovery.safe_path(Path(__file__).resolve().parent)
    private = recovery.verify_bundle(destination)
    if not recovery.functional_bundle(private):
        raise ValueError('Expected a functional EmotionalUI package')
    systems, library, options = installed_systems()
    data = recovery.safe_path(Path.home()/'.local/share/emotional-ui-player')
    plan = profiles.build_plan(systems, Path.home(), library, data/'progress', functional=True)
    if not plan['report']['included']:
        raise ValueError('No installed compatible emulator/core found; the menu was not stopped')
    if options:
        plan['profiles'] = 'system_options_root = '+json.dumps(str(options))+'\n'+plan['profiles']
    if args.check:
        print(json.dumps({'profiles':len(plan['report']['included']),
            'skipped':len(plan['report']['skipped']), 'options':bool(options),
            'physical_validated':False, 'graphics_started':False}))
        return
    functional_menu.preflight(destination/'emotional-ui')
    try:
        synced = subprocess.run(['/usr/bin/timedatectl','show','-p','NTPSynchronized','--value'],capture_output=True,text=True,timeout=5)
        if synced.returncode == 0 and synced.stdout.strip() == 'yes':
            plan['profiles'] = plan['profiles'].replace('clock_confirmed = false','clock_confirmed = true',1)
    except (OSError, subprocess.SubprocessError):
        pass
    profiles.private_directory(data)
    token = uuid.uuid4().hex
    runtime = data/('session-'+token)
    profiles.private_directory(runtime)
    profile = runtime/'launchers.toml'
    materialize(plan, profile)
    functional_preview.verify_inputs(destination/'emotional-ui', private['binary_sha256'],
        profile, hashlib.sha256(profile.read_bytes()).hexdigest(), data/'library.sqlite', library, runtime)
    functional_preview.verify_input_profile(destination/'input.toml')
    functional_menu.check_no_session()
    subprocess.run(session_command(destination,profile,data,library,runtime,token),
                   check=True,capture_output=True,text=True,timeout=20)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, ET.ParseError, subprocess.SubprocessError) as error:
        print('EmotionalUI: '+str(error),file=sys.stderr,flush=True)
        time.sleep(5)
        sys.exit(1)
