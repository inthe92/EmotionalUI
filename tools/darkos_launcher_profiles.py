#!/usr/bin/env python3
"""Build private, explicit RetroArch launch profiles from ES *data*.

Never executes an emulator, a ROM, XML command text or a shell. Dry-run is the
default. Generated profiles require a separate supervised physical validation.
"""
import argparse
import hashlib
import json
import os
import re
import stat
import struct
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

MAX_PROFILES = 128
RETROARCH = {'retroarch', 'retroarch32'}
FIXED_CORES = {
    'advision': ['mess'], 'astrocde': ['mess'], 'vmac': ['minivmac'],
    'channelf': ['freechaf'], 'mame2003': ['mame2003_plus'], 'mame': ['mame2010'],
    'cdimono1': ['same_cdi'], '3do': ['opera'], 'pokemonmini': ['pokemini'],
    'atari800': ['atari800'], 'atarixegs': ['atari800'], 'atari7800': ['prosystem'],
    'odyssey2': ['o2em'], 'videopac': ['o2em'], 'satellaview': ['snes9x'],
    'snesmsu1': ['snes9x'], 'gameandwatch': ['gw'], 'sgb': ['mgba'],
    'sg-1000': ['genesis_plus_gx'], 'msumd': ['genesis_plus_gx'], 'vmu': ['vemulator'],
    'intellivision': ['freeintv'], 'tic80': ['tic80'], 'vectrex': ['vecx'],
    'zx81': ['81'], 'zxspectrum': ['fuse'], 'x1': ['x1'], 'x68000': ['px68k'],
    'c16': ['vice_xplus4'], 'c64': ['vice_x64'], 'c128': ['vice_x128'],
    'vic20': ['vice_xvic'], 'pcfx': ['mednafen_pcfx'], 'supervision': ['potator'],
    'scv': ['emuscv'], 'palm': ['mu'], 'puzzlescript': ['puzzlescript'],
    'enterprise': ['ep128emu_core'], 'tvc': ['ep128emu_core'],
    'vircon32': ['vircon32'], 'thomson': ['theodore'], 'wasm4': ['wasm4'],
    'lowresnx': ['lowresnx'], 'tigerlcd': ['mametiger'], 'uzebox': ['uzem'],
    'psp': ['ppsspp'], 'pspminis': ['ppsspp'], 'easyrpg': ['easyrpg'],
    'scummvm': ['scummvm'], 'pico8': ['fake08'], 'pico-8': ['fake08'],
    'carts': ['fake08'], 'nds': ['melonds', 'desmume'],
}
BLOCKED_SYSTEMS = {'ports', 'movies', 'cmds'}
BLOCKED_EXTENSIONS = {'.sh', '.bash', '.py', '.pl', '.rb', '.bat', '.cmd', '.ps1',
                      '.so', '.dll', '.elf', '.appimage', '.com', '.exe'}
# Small, explicitly verified additions where this installation's ES list omits
# a supported format. These are per-core, never a global suffix launcher.
CORE_EXTENSIONS = {
    'mednafen_pce_fast': {'.cue', '.chd', '.m3u'},
    'pcsx_rearmed': {'.bin'}, 'pcsx_rearmed_rumble': {'.bin'},
    'easyrpg': {'.ldb', '.lzh'},
}


def absolute(path):
    path = Path(path)
    if not path.is_absolute() or '..' in path.parts or any(c in str(path) for c in ('\0', '\n', '\r', '"')):
        raise ValueError('Explicit absolute paths without traversal or control characters are required')
    return path


def elf(path, core=False):
    path = absolute(path).resolve(strict=True)
    with path.open('rb') as source:
        header = source.read(64)
    if len(header) < 52 or header[:4] != b'\x7fELF' or header[5] != 1:
        raise ValueError('not a supported little-endian ELF')
    kind, machine = struct.unpack_from('<HH', header, 16)
    architecture = {(1, 40): 'arm32', (2, 183): 'aarch64'}.get((header[4], machine))
    if architecture is None or (core and kind != 3) or (not core and kind not in (2, 3)):
        raise ValueError('unsupported ELF architecture/type')
    if not core and (not path.is_file() or not os.access(path, os.X_OK)):
        raise ValueError('RetroArch is not executable')
    return architecture, path


def validate_base(path):
    path = absolute(path).resolve(strict=True)
    if not path.is_file() or path.stat().st_size > 8 * 1024 * 1024:
        raise ValueError('missing or oversized RetroArch base config')
    content = path.read_text()
    if '\0' in content:
        raise ValueError('invalid RetroArch base config')
    for line in content.splitlines():
        line = line.strip()
        if line and not line.startswith('#') and not re.fullmatch(r'[A-Za-z0-9_]+\s*=\s*.+', line):
            raise ValueError('invalid RetroArch base config syntax')
    return path


def extensions(text, folder, core):
    accepted, rejected = set(), set()
    for raw in text.split():
        suffix = raw.lower()
        # These suffixes are *guest/core data* passed as argv to an explicit
        # installed RetroArch core. They never become a Linux executable.
        cave_story = suffix == '.exe' and folder == 'cavestory' and core in {'nxengine', 'doukutsu_rs'}
        dos_guest = suffix in {'.exe', '.com', '.bat'} and folder == 'dos' and core in {'dosbox', 'dosbox_pure'}
        core_commands = suffix == '.cmd' and (core.startswith('vice_') or core in {
            'px68k', 'np2kai', 'nekop2', 'atari800', 'minivmac'})
        if (not re.fullmatch(r'\.[a-z0-9]{1,23}', suffix)
                or (core == 'fbneo' and suffix == '.chd')
                or (suffix in BLOCKED_EXTENSIONS and not (cave_story or dos_guest or core_commands))):
            rejected.add(raw)
        else:
            accepted.add(suffix)
    accepted.update(CORE_EXTENSIONS.get(core, ()))
    return sorted(accepted), sorted(rejected)


def source_path(value, home):
    if value == '~' or value.startswith('~/'):
        value = str(home)+value[1:]
    return absolute(value).resolve(strict=True)


def isolation(data, ident, functional=False):
    base = data/ident
    directories = [base/name for name in ('saves', 'states', 'cache', 'remaps', 'screenshots',
                                         'playlists', 'history', 'recordings', 'recording-config', 'logs', 'config')]
    settings = {key: 'false' for key in (
        'config_save_on_exit', 'auto_overrides_enable', 'auto_remaps_enable',
        'auto_shaders_enable', 'game_specific_options', 'savestate_auto_save',
        'savestate_auto_load', 'savefiles_in_content_dir', 'savestates_in_content_dir',
        'content_runtime_log', 'content_runtime_log_aggregate', 'cheevos_enable',
        'netplay_enable', 'network_cmd_enable', 'network_remote_enable', 'stdin_cmd_enable',
        'builtin_imageviewer_enable', 'builtin_mediaplayer_enable')}
    settings.update({
        'savefile_directory': str(base/'saves'), 'savestate_directory': str(base/'states'),
        'cache_directory': str(base/'cache'), 'input_remapping_directory': str(base/'remaps'),
        'screenshot_directory': str(base/'screenshots'), 'playlist_directory': str(base/'playlists'),
        'recording_output_directory': str(base/'recordings'),
        'recording_config_directory': str(base/'recording-config'), 'log_dir': str(base/'logs'),
        'rgui_config_directory': str(base/'config'),
        'core_options_path': str(base/'core-options.cfg'),
        'content_history_path': str(base/'history/content.lpl'),
        'content_favorites_path': str(base/'history/favorites.lpl'),
        'content_image_history_path': str(base/'history/images.lpl'),
        'content_music_history_path': str(base/'history/music.lpl'),
        'content_video_history_path': str(base/'history/video.lpl'),
    })
    if functional:
        settings['savestate_auto_save'] = 'true'
        for key in ('sort_savefiles_enable', 'sort_savestates_enable',
                    'sort_savefiles_by_content_enable', 'sort_savestates_by_content_enable'):
            settings[key] = 'false'
    # No system_directory/BIOS rewrite: retain the exact original base values.
    text = '# EmotionalUI private isolation; original base remains read-only.\n'
    text += ''.join(key+' = '+json.dumps(value)+'\n' for key, value in sorted(settings.items()))
    return base/'isolation.cfg', text, directories


def read_preferences(path):
    """ES writes XML sibling nodes, not necessarily a single document root.

    Read the native format without rewriting it. Invalid/ambiguous preferences
    must not silently reset a user's emulator selection to our fallback.
    """
    raw = path.read_bytes()
    if len(raw) > 1024*1024 or b'<!DOCTYPE' in raw.upper() or b'<!ENTITY' in raw.upper():
        raise ValueError('Invalid EmulationStation preferences; preserved without fallback')
    text = raw.decode('utf-8-sig')
    text = re.sub(r'^\s*<\?xml[^>]*\?>', '', text, count=1)
    try:
        document = ET.fromstring('<emotional-settings-fragment>'+text+'</emotional-settings-fragment>')
    except ET.ParseError as error:
        raise ValueError('Unreadable EmulationStation preferences; preserved without fallback') from error
    preferences = {}
    for element in document.iter('string'):
        name, value = element.get('name'), element.get('value')
        if not name or value is None or not value.strip():
            continue
        if name in preferences:
            raise ValueError('Ambiguous EmulationStation preferences; preserved without fallback')
        preferences[name] = value
    return preferences


def build_plan(systems, home, library_root, data, retroarch_bin_dir=Path('/opt/retroarch/bin'), functional=False):
    """Read-only planning. This never launches content or creates directories."""
    systems = absolute(systems).resolve(strict=True)
    home = absolute(home).resolve(strict=True)
    library_root = absolute(library_root).resolve(strict=True)
    data = absolute(data)
    retroarch_bin_dir = absolute(retroarch_bin_dir).resolve(strict=True)
    if not home.is_dir() or not library_root.is_dir() or not retroarch_bin_dir.is_dir():
        raise ValueError('Existing explicit home, library and emulator directories are required')
    data = data.resolve(strict=False)
    if data.is_relative_to(library_root) or data == home:
        raise ValueError('Own isolation data must be outside the original ROM library and home root')
    raw = systems.read_bytes()
    if len(raw) > 8 * 1024 * 1024 or b'<!DOCTYPE' in raw.upper() or b'<!ENTITY' in raw.upper():
        raise ValueError('Oversized XML and DTD/entity declarations are forbidden')
    document = ET.fromstring(raw)
    nodes = document.findall('system')
    if document.tag != 'systemList' or len(nodes) > 4096:
        raise ValueError('Expected a bounded EmulationStation systemList')
    report = {'kind': 'private-darkos-launcher-plan', 'physical_tested': False,
              'included': [], 'skipped': [], 'limitations': [
                  'Installed ELF/core compatibility is static, not a gameplay certification.',
                  'Original RetroArch configuration and ROM roots must remain read-only in the supervisor unit.',
                  'Guest programs and PICO-8 PNG carts are passed only as core content; corrupt content remains unvalidated.']}
    preferences = {}
    settings = home/'.emulationstation/es_settings.cfg'
    if functional and settings.is_file():
        preferences = read_preferences(settings)
    system_names = {(node.findtext('name') or '').strip() for node in nodes}
    report['unmatched_preferences'] = sorted(key for key in preferences
        if key.endswith(('.emulator', '.core')) and key.rsplit('.', 1)[0] not in system_names)
    profiles, append_configs, directories = [], {}, {data}
    seen = set()
    for node in nodes:
        name = (node.findtext('name') or '').strip()
        raw_root = (node.findtext('path') or '').strip()
        entry = {'system': name, 'root': raw_root}
        def skip(reason):
            report['skipped'].append(dict(entry, reason=reason))
        try:
            root = source_path(raw_root, home)
            folder = root.name.lower()
            entry['root'] = str(root)
            if folder in BLOCKED_SYSTEMS or name.lower() in BLOCKED_SYSTEMS:
                skip('non-game/script system excluded'); continue
            if not root.is_dir() or root == library_root or not root.is_relative_to(library_root):
                skip('root absent or outside explicit library'); continue
            if root in seen:
                skip('duplicate canonical root'); continue
            emulators = node.findall('./emulators/emulator')
            default = next((e.get('name', '') for e in emulators if e.get('default') == 'true'),
                           emulators[0].get('name', '') if emulators else '')
            candidates = []
            ra_nodes = [e for e in emulators if e.get('name') in RETROARCH]
            preferred_emulator = preferences.get(name+'.emulator', default)
            preferred_core = preferences.get(name+'.core')
            explicit_emulator = preferences.get(name+'.emulator')
            if explicit_emulator and explicit_emulator not in RETROARCH:
                skip('explicit standalone emulator selection is not supported; preserved without substitution'); continue
            if explicit_emulator:
                ra_nodes = [e for e in ra_nodes if e.get('name') == explicit_emulator]
            ra_nodes.sort(key=lambda e: e.get('name') != preferred_emulator)
            declared = any(e.findall('./cores/core') for e in ra_nodes)
            if declared:
                for emulator in ra_nodes:
                    candidates.append((emulator.get('name'),
                                       [(c.text or '').strip() for c in sorted(emulator.findall('./cores/core'), key=lambda c: ((c.text or '').strip() != preferred_core if preferred_core else c.get('default') != 'true'))], 'xml'))
            else:
                fixed = FIXED_CORES.get(folder, FIXED_CORES.get(name.lower(), []))
                if fixed:
                    candidates = [(e.get('name'), fixed, 'fixed-map') for e in ra_nodes]
                    candidates += [(e, fixed, 'fixed-map') for e in ('retroarch', 'retroarch32')
                                   if e not in [item[0] for item in candidates]]
            if preferred_core:
                candidates = [(emu, [core for core in cores if core == preferred_core], origin)
                              for emu, cores, origin in candidates]
            if explicit_emulator:
                candidates = [candidate for candidate in candidates if candidate[0] == explicit_emulator]
            selected = None
            for emulator, cores, origin in candidates:
                try:
                    architecture, executable = elf(retroarch_bin_dir/emulator)
                    cfg_dir = home/'.config'/emulator
                    base = validate_base(cfg_dir/'retroarch.cfg')
                except (OSError, ValueError, UnicodeError):
                    continue
                for core in cores:
                    if not re.fullmatch(r'[A-Za-z0-9_+-]{1,96}', core):
                        continue
                    try:
                        core_architecture, core_path = elf(cfg_dir/'cores'/(core+'_libretro.so'), core=True)
                        if core_architecture != architecture:
                            continue
                    except (OSError, ValueError):
                        continue
                    selected = (emulator, core, origin, architecture, executable, core_path, base)
                    break
                if selected:
                    break
            if not selected:
                skip('no declared/fixed installed compatible RetroArch executable, core and base config'); continue
            emulator, core, origin, architecture, executable, core_path, base = selected
            suffixes, rejected = extensions(node.findtext('extension') or '', folder, core)
            slug = re.sub(r'[^a-z0-9_-]', '-', folder)[:48] or 'system'
            ident = 'darkos-'+slug+'-'+hashlib.sha256(str(root).encode()).hexdigest()[:8]
            routes = [(ident, selected, suffixes, rejected)] if suffixes else []
            if folder == 'neogeo' and core == 'fbneo':
                # Neo Geo CD collections can live beside arcade ZIPs. FBNeo
                # does not accept CHD; reserve that suffix for an installed
                # compatible NeoCD core, with its own saves/configuration.
                neocd = None
                for cd_emulator in dict.fromkeys([emulator, 'retroarch', 'retroarch32']):
                    try:
                        cd_arch, cd_executable = elf(retroarch_bin_dir/cd_emulator)
                        cfg_dir = home/'.config'/cd_emulator
                        cd_base = validate_base(cfg_dir/'retroarch.cfg')
                        core_arch, cd_core = elf(cfg_dir/'cores/neocd_libretro.so', core=True)
                        if core_arch != cd_arch:
                            continue
                        neocd = (cd_emulator, 'neocd', 'explicit-neogeo-cd-route',
                                 cd_arch, cd_executable, cd_core, cd_base)
                        break
                    except (OSError, ValueError, UnicodeError):
                        continue
                if neocd:
                    routes.append((ident+'-neocd', neocd, ['.chd'], []))
                else:
                    report['skipped'].append(dict(entry, extensions=['.chd'],
                        reason='Neo Geo CD .chd omitted: no installed compatible neocd core/base'))
            if not routes:
                skip('no safe declared content extensions'); continue
            for route_id, selected_route, suffixes, rejected in routes:
                if len(profiles) >= MAX_PROFILES:
                    skip('128-profile safety limit'); break
                route_emulator, route_core, route_origin, route_arch, route_exe, route_core_path, route_base = selected_route
                append, content, own_dirs = isolation(data, route_id, functional)
                append_configs[str(append)] = content
                directories.update(own_dirs)
                directories.add(append.parent)
                profiles.append(dict(id=route_id, root=str(root), executable=str(route_exe),
                                     working_directory=str(data/route_id),
                                     arguments=['--config', str(route_base), '--appendconfig', str(append),
                                                '-L', str(route_core_path), '{rom}'],
                                     waits_for_exit=True, extensions=suffixes))
                if functional:
                    resume_config = append.with_name('resume.cfg')
                    append_configs[str(resume_config)] = content.replace('savestate_auto_load = "false"', 'savestate_auto_load = "true"')
                    profiles[-1]['resume_arguments'] = ['--config', str(route_base), '--appendconfig', str(resume_config), '-L', str(route_core_path), '{rom}']
                    profiles[-1]['progress_root'] = str(append.parent)
                    # Copy only this game's recognized save/state files at launch.
                    # Missing/default directories mean the original content folder.
                    base_text = route_base.read_text()
                    for setting, flag, field in [
                        ('savefile_directory', 'savefiles_in_content_dir', 'save_source_directory'),
                        ('savestate_directory', 'savestates_in_content_dir', 'state_source_directory'),
                    ]:
                        configured = re.search(r'^\s*' + setting + r'\s*=\s*"([^"\n]*)"', base_text, re.M)
                        in_content = re.search(r'^\s*' + flag + r'\s*=\s*"?true"?', base_text, re.M)
                        if configured and not in_content and configured[1] not in ('', 'default'):
                            try:
                                original = source_path(configured[1], home)
                                if original.is_dir(): profiles[-1][field] = str(original)
                            except (OSError, ValueError):
                                pass
                seen.add(root)
                record = dict(entry, profile=route_id, emulator=route_emulator, core=route_core,
                              architecture=route_arch, core_source=route_origin,
                              default_emulator=default,
                              fallback_from_non_retroarch=bool(default and default not in RETROARCH),
                              excluded_extensions=rejected, extensions=suffixes)
                if route_origin == 'explicit-neogeo-cd-route':
                    record['routing'] = 'Neo Geo CD .chd only; other formats remain on the FBNeo profile'
                if route_core in CORE_EXTENSIONS:
                    record['documented_core_extensions'] = sorted(CORE_EXTENSIONS[route_core])
                report['included'].append(record)
        except (OSError, ValueError, TypeError):
            skip('unavailable or invalid system path/data')
    text = 'handoff = "process-exit"\nclock_confirmed = false\n'
    for profile in profiles:
        text += '\n[[launchers]]\n'
        for key, value in profile.items():
            text += key+' = '+json.dumps(value, ensure_ascii=False)+'\n'
    if len(text.encode('utf-8')) > 512 * 1024:
        raise ValueError('Launcher document exceeds the 512 KiB runtime limit')
    return {'profiles': text, 'append_configs': append_configs,
            'directories': sorted(map(str, directories)), 'report': report}


def private_directory(path):
    path = absolute(path)
    if path.exists():
        info = path.lstat()
        if (not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid()
                or stat.S_IMODE(info.st_mode) != 0o700):
            raise ValueError('Existing output directories must be current-user-owned 0700 directories')
        return
    # Existing ancestors (for example /home/ark) need not be 0700. Never chmod
    # them; only the directories created by this operation are private-owned.
    missing = []
    parent = path
    while not parent.exists():
        if parent.is_symlink():
            raise ValueError('Symlink output ancestors are forbidden')
        missing.append(parent)
        parent = parent.parent
    if not parent.is_dir() or parent.is_symlink():
        raise ValueError('Expected a real output ancestor directory')
    for own in reversed(missing):
        own.mkdir(mode=0o700)


def write_plan(plan, output, report):
    output, report = absolute(output), absolute(report)
    files = dict(plan['append_configs'])
    files[str(output)] = plan['profiles']
    if str(report) in files or output == report:
        raise ValueError('Output paths collide')
    files[str(report)] = json.dumps(plan['report'], indent=2, ensure_ascii=False)+'\n'
    if len(files) != len(plan['append_configs'])+2:
        raise ValueError('Output paths collide')
    for name in files:
        path = absolute(name)
        if path.exists() or path.is_symlink():
            raise FileExistsError('Refusing to overwrite an existing private file')
    for directory in sorted({*plan['directories'], *(str(Path(p).parent) for p in files)}, key=lambda p: len(Path(p).parts)):
        private_directory(Path(directory))
    for name, text in files.items():
        descriptor = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, 'w') as target:
            target.write(text)
            target.flush()
            os.fsync(target.fileno())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('systems', 'home', 'library-root', 'data', 'output', 'report'):
        parser.add_argument('--'+field, type=Path, required=True)
    parser.add_argument('--retroarch-bin-dir', type=Path, default=Path('/opt/retroarch/bin'))
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    try:
        plan = build_plan(args.systems, args.home, args.library_root, args.data, args.retroarch_bin_dir)
        if args.write:
            write_plan(plan, args.output, args.report)
        print(json.dumps({'dry_run': not args.write, 'profiles': len(plan['report']['included']),
                          'skipped': len(plan['report']['skipped']), 'physical_tested': False}))
        return 0
    except (OSError, ValueError, ET.ParseError) as error:
        print('Launcher plan refused: '+str(error), file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
