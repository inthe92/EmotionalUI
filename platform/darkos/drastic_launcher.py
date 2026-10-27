"""Private DraStic workspace and foreground launch; no system configuration writes.

Assets remain links to the installed emulator, never redistributed copies. The
caller must protect the original installation read-only during the session.
"""
import argparse
import hashlib
import os
from pathlib import Path, PurePosixPath
import selectors
import shutil
import stat
import subprocess
import tempfile
import time


ASSETS = ('system', 'microphone', 'drastic_logo_0.raw', 'drastic_logo_1.raw',
          'usrcheat.dat', 'game_database.xml')
MUTABLE = ('backup', 'cheats', 'savestates', 'slot2', 'input_record', 'profiles',
           'scripts', 'unzip_cache')
MAX_ROM = 512 * 1024 * 1024
MAX_LIST = 1024 * 1024


class LaunchError(ValueError):
    pass


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def real_path(path):
    path = Path(path)
    if not path.is_absolute() or '..' in path.parts:
        raise LaunchError('An absolute, unambiguous path is required')
    # Reject aliases in private workspace paths, including parent directories.
    for item in (path, *path.parents):
        if item.is_symlink():
            raise LaunchError('Workspace paths must not be symbolic links')
    return path


def private(path, directory=False):
    path = real_path(path)
    info = path.lstat()
    valid_type = stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)
    if not valid_type or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise LaunchError('Workspace ownership or permissions are unsafe')
    return path


def elf(path):
    if not path.is_file() or path.is_symlink():
        raise LaunchError('Installed emulator must be a regular ELF file')
    with path.open('rb') as source:
        header = source.read(64)
    if (len(header) != 64 or header[:6] != b'\x7fELF\x02\x01'
            or int.from_bytes(header[16:18], 'little') not in (2, 3)
            or int.from_bytes(header[18:20], 'little') != 183):
        raise LaunchError('An ARM64 little-endian emulator ELF is required')


def prepare(source, data, write=False):
    """Plan or create a new private workspace. Existing destinations are refused."""
    source, data = Path(source).resolve(strict=True), real_path(data)
    if source == data or source in data.parents:
        raise LaunchError('Private data must be outside the installed emulator')
    if data.exists():
        raise LaunchError('Destination exists; no files were overwritten')
    if not data.parent.is_dir():
        raise LaunchError('Destination parent must already exist')
    binary = source / 'drastic'
    elf(binary)
    config = source / 'config'
    if not config.is_dir() or config.is_symlink():
        raise LaunchError('Installed emulator configuration is unavailable')
    configs = []
    for current, dirs, files in os.walk(config, followlinks=False):
        if any((Path(current) / name).is_symlink() for name in dirs):
            raise LaunchError('Configuration directory contains an alias')
        for name in files:
            path = Path(current) / name
            if path.suffix.lower() == '.cfg':
                if path.is_symlink() or not path.is_file():
                    raise LaunchError('Configuration must contain regular cfg files')
                configs.append((path, path.relative_to(config), digest(path)))
    if not any(relative == Path('drastic.cfg') for _, relative, _ in configs):
        raise LaunchError('Main emulator configuration is unavailable')
    assets = [name for name in ASSETS if (source / name).exists()]
    before = digest(binary)
    plan = dict(binary_sha256=before, config_files=len(configs), linked_assets=assets,
                private_directories=list(MUTABLE), written=False)
    if not write:
        return plan
    data.mkdir(mode=0o700)
    (data / 'config').mkdir(mode=0o700)
    for name in MUTABLE:
        (data / name).mkdir(mode=0o700)
    target = data / 'drastic'
    with binary.open('rb') as incoming, target.open('xb') as outgoing:
        shutil.copyfileobj(incoming, outgoing, 1024 * 1024)
    target.chmod(0o700)
    if digest(binary) != before or digest(target) != before:
        raise LaunchError('Emulator changed during preparation; do not launch')
    for original, relative, expected in configs:
        destination = data / 'config' / relative
        destination.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        with original.open('rb') as incoming, destination.open('xb') as outgoing:
            shutil.copyfileobj(incoming, outgoing)
        destination.chmod(0o600)
        if digest(original) != expected or digest(destination) != expected:
            raise LaunchError('Configuration changed during preparation; do not launch')
    for name in assets:
        (data / name).symlink_to((source / name).resolve(strict=True))
    # Written last: an incomplete preparation cannot be launched accidentally.
    manifest = data / 'emotional-drastic.sha256'
    with manifest.open('x', encoding='ascii') as output:
        output.write(before + '\n')
    manifest.chmod(0o600)
    return dict(plan, written=True)


def validate_workspace(data):
    data = private(data, True)
    for name in (*MUTABLE, 'config'):
        private(data / name, True)
    for current, dirs, files in os.walk(data / 'config', followlinks=False):
        for name in dirs:
            private(Path(current) / name, True)
        for name in files:
            private(Path(current) / name)
    private(data / 'config' / 'drastic.cfg')
    binary = private(data / 'drastic')
    elf(binary)
    if not os.access(binary, os.X_OK):
        raise LaunchError('Private emulator is not executable')
    expected = private(data / 'emotional-drastic.sha256').read_text(encoding='ascii').strip()
    if len(expected) != 64 or digest(binary) != expected:
        raise LaunchError('Private emulator integrity check failed')
    return data, binary


def bounded_command(argv, maximum, timeout, output=None):
    """Bound preparation output and time; never used for the game process."""
    process = subprocess.Popen(argv, stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    chunks, total = [], 0
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise LaunchError('Archive preparation timed out')
            if not selector.select(remaining):
                raise LaunchError('Archive preparation timed out')
            block = os.read(process.stdout.fileno(), 65536)
            if not block:
                break
            total += len(block)
            if total > maximum:
                raise LaunchError('Archive output exceeds the permitted size')
            if output is None:
                chunks.append(block)
            else:
                output.write(block)
        try:
            result = process.wait(timeout=max(0.001, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as error:
            raise LaunchError('Archive preparation timed out') from error
        if result != 0:
            raise LaunchError('Archive tool rejected the content')
        return b''.join(chunks) if output is None else total
    finally:
        selector.close()
        process.stdout.close()
        if process.poll() is None:
            process.kill()
            process.wait()


def archive_member(listing):
    """Accept exactly one regular unencrypted NDS and no ambiguous metadata."""
    try:
        text = listing.decode('utf-8', errors='strict')
    except UnicodeError as error:
        raise LaunchError('Archive metadata encoding is unsupported') from error
    entries = []
    entry = {}
    for line in [*text.splitlines(), '']:
        if not line.strip():
            if entry:
                entries.append(entry)
                entry = {}
            continue
        if ' = ' not in line:
            raise LaunchError('Malformed archive metadata')
        key, value = line.split(' = ', 1)
        if key in entry:
            raise LaunchError('Duplicate archive metadata')
        entry[key] = value
    candidates, names = [], set()
    for entry in entries:
        name = entry.get('Path', '')
        path = PurePosixPath(name)
        if (not name or '\\' in name or ':' in name or '\x00' in name
                or path.is_absolute() or any(part in ('', '.', '..') for part in name.split('/'))
                or any(ord(char) < 32 for char in name)):
            raise LaunchError('Archive contains an unsafe member path')
        normalized = name.casefold()
        if normalized in names:
            raise LaunchError('Archive contains duplicate member names')
        names.add(normalized)
        # Reject links anywhere, rather than following an archive's reference.
        attributes = entry.get('Attributes', '')
        mode = entry.get('Mode', '')
        if (any(key in entry for key in ('Symbolic Link', 'Hard Link', 'Link', 'Alternate Stream'))
                or mode.startswith(('l', 'b', 'c', 'p', 's'))
                or any(token.startswith(('l', 'b', 'c', 'p', 's')) for token in attributes.split())):
            raise LaunchError('Archive links are unsupported')
        if path.suffix.lower() != '.nds':
            continue
        if entry.get('Folder', '-') == '+' or attributes.startswith('D') or mode.startswith('d'):
            raise LaunchError('NDS member is not a regular file')
        if entry.get('Encrypted', '-') != '-':
            raise LaunchError('Encrypted NDS content is unsupported')
        try:
            size = int(entry['Size'])
        except (KeyError, ValueError) as error:
            raise LaunchError('NDS size is missing or invalid') from error
        if not 0 < size <= MAX_ROM:
            raise LaunchError('NDS content exceeds the permitted size')
        candidates.append((name, size))
    if len(candidates) != 1:
        raise LaunchError('An archive must contain exactly one NDS file')
    return candidates[0]


def run(data, rom_root, rom, sevenzip=Path('/usr/bin/7z')):
    data, binary = validate_workspace(data)
    root = Path(rom_root).resolve(strict=True)
    content = Path(rom).resolve(strict=True)
    if not root.is_dir() or not content.is_file() or root not in content.parents:
        raise LaunchError('Content must be a file inside the authorized library')
    extension = content.suffix.lower()
    if extension not in ('.nds', '.zip', '.7z'):
        raise LaunchError('This profile accepts NDS, ZIP or 7Z only')
    if extension == '.nds':
        if not 0 < content.stat().st_size <= MAX_ROM:
            raise LaunchError('NDS content exceeds the permitted size')
        return subprocess.run([str(binary), str(content)], cwd=data, check=False,
                              umask=0o077).returncode
    tool = Path(sevenzip)
    if not tool.is_absolute() or not tool.is_file() or not os.access(tool, os.X_OK):
        raise LaunchError('The installed archive tool is unavailable')
    before = content.stat()
    listing = bounded_command([str(tool), 'l', '-slt', '-ba', '--', str(content)], MAX_LIST, 10)
    member, size = archive_member(listing)
    # DraStic derives save names from the ROM basename. A constant extracted
    # filename would merge every archived game's SRAM and savestates.
    identity = hashlib.sha256(content.relative_to(root).as_posix().encode('utf-8')).hexdigest()[:24]
    with tempfile.TemporaryDirectory(prefix='emotional-nds-', dir=data / 'unzip_cache') as folder:
        extracted = Path(folder) / ('game-' + identity + '.nds')
        with extracted.open('xb') as output:
            count = bounded_command([str(tool), 'e', '-so', '-spd', '--', str(content), member],
                                    size, 60, output)
        extracted.chmod(0o600)
        after = content.stat()
        if count != size or (before.st_ino, before.st_size, before.st_mtime_ns) != (
                after.st_ino, after.st_size, after.st_mtime_ns):
            raise LaunchError('Archive changed or extraction size did not match')
        return subprocess.run([str(binary), str(extracted)], cwd=data, check=False,
                              umask=0o077).returncode


def main(argv=None):
    parser = argparse.ArgumentParser(description='Private installed DraStic adapter')
    actions = parser.add_subparsers(dest='action', required=True)
    planning = actions.add_parser('prepare')
    planning.add_argument('--source', required=True, type=Path)
    planning.add_argument('--data', required=True, type=Path)
    planning.add_argument('--write', action='store_true')
    launching = actions.add_parser('run')
    launching.add_argument('--data', required=True, type=Path)
    launching.add_argument('--rom-root', required=True, type=Path)
    launching.add_argument('--sevenzip', default=Path('/usr/bin/7z'), type=Path)
    launching.add_argument('rom', type=Path)
    args = parser.parse_args(argv)
    try:
        if args.action == 'prepare':
            result = prepare(args.source, args.data, args.write)
            print('Private DraStic workspace ' + ('created' if result['written'] else 'planned')
                  + '; configuration files: ' + str(result['config_files']))
            return 0
        return run(args.data, args.rom_root, args.rom, args.sevenzip)
    except (LaunchError, OSError, UnicodeError) as error:
        # Do not expose ROM names, private paths or credentials in frontend logs.
        message = str(error) if isinstance(error, LaunchError) else 'Required local file or process is unavailable'
        print('DraStic: ' + message)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
