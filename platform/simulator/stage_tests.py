"""Stage only test executables reported by Cargo, never guessed file names."""
import json
from pathlib import Path
import shutil
import sys
folder = Path(sys.argv[1])
executables = []
for line in (folder / 'tests-build.jsonl').read_text().splitlines():
    event = json.loads(line)
    if event.get('reason') == 'compiler-artifact' and event.get('profile', {}).get('test') and event.get('executable'):
        executables.append(Path(event['executable']))
if len(executables) != 3:
    raise SystemExit(f'Expected all 3 workspace test executables, got {len(executables)}')
destination = folder / 'tests'
destination.mkdir(exist_ok=True)
# This directory is dedicated to staged test binaries, not source files.
for old in destination.iterdir():
    if old.is_file(): old.unlink()
for source in executables:
    shutil.copy2(source, destination / source.name)
print('Staged', len(executables), 'ARM64 test executables')
