"""Generate omitted production exports in owned temporary folders, never in assets."""
import atexit
from functools import lru_cache
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def temporary():
    folder = tempfile.TemporaryDirectory(prefix='emotional-audio-test-')
    atexit.register(folder.cleanup)
    return Path(folder.name)


def ffmpeg():
    configured = os.environ.get('EMOTIONAL_FFMPEG')
    full = Path('/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg')
    binary = configured or (str(full) if full.is_file() else shutil.which('ffmpeg'))
    if not binary:
        raise unittest.SkipTest('Optional production export: FFmpeg is not installed; no automatic download')
    return binary


@lru_cache(maxsize=1)
def original_study():
    folder = temporary()
    subprocess.run([sys.executable, str(ROOT/'tools/compose_audio.py'), '--output-root', str(folder)],
                   check=True, stdout=subprocess.DEVNULL)
    return folder


@lru_cache(maxsize=1)
def original_exports():
    preview = original_study()/'assets/audio/previews'
    for ext, codec in [('ogg', ['-c:a', 'libvorbis', '-q:a', '4']), ('m4a', ['-c:a', 'aac', '-b:a', '96k'])]:
        subprocess.run([ffmpeg(), '-nostdin', '-n', '-v', 'error', '-i', str(preview/'la-habitacion-permanece.wav'),
                        *codec, str(preview/('la-habitacion-permanece.'+ext))], check=True)
    return preview


@lru_cache(maxsize=1)
def sampled_exports():
    cache = Path(os.environ.get('EMOTIONAL_PIANO_SAMPLE_CACHE', str(Path.home()/'.cache/emotional-ui/music-vsco-upright')))
    import json
    lock = json.loads((ROOT/'assets/audio/phase9/instrument/sources.lock.json').read_text())
    if any(not (cache/e['cache_file']).is_file() for e in lock['files'] if e['path'].endswith('.wav')):
        raise unittest.SkipTest('Optional sampled-piano production: pinned CC0 samples absent; set EMOTIONAL_PIANO_SAMPLE_CACHE (no automatic download)')
    try:
        import numpy  # noqa: F401 -- production prerequisite, not runtime dependency
    except ImportError:
        raise unittest.SkipTest('Optional sampled-piano production requires NumPy in the development environment')
    output = temporary()/'candidates'
    subprocess.run([sys.executable, str(ROOT/'tools/produce_music.py'), '--sample-cache', str(cache),
                    '--output', str(output), '--ffmpeg', ffmpeg()], check=True, stdout=subprocess.DEVNULL)
    return output
