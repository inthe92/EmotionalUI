#!/usr/bin/env python3
"""Offline diagnostic bank, never edits sources or the system mixer.

One shared gain preserves the piano family's relative dynamics. This is not an
approved final mix. Use only with application master <=25 and music <=100.
"""
import argparse
import array
import hashlib
import json
import shutil
import sys
import wave
from pathlib import Path

PIANO = ('umbral', 'brasa', 'rastro')
AMBIENCE = ('rain', 'air')

def calibrate(source, destination):
    source, destination = Path(source), Path(destination)
    clips, hashes = {}, {}
    for name in PIANO + AMBIENCE:
        path = source / (name + '.wav')
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 2 * 1024 * 1024:
            raise ValueError('Bounded regular source WAV required')
        hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
        with wave.open(str(path), 'rb') as wav:
            if wav.getparams()[:3] != (1, 2, 22050) or wav.getcomptype() != 'NONE':
                raise ValueError('Expected mono PCM16 22050 Hz')
            samples = array.array('h', wav.readframes(wav.getnframes()))
            if sys.byteorder != 'little':
                samples.byteswap()
            clips[name] = samples
    peak = max((max(map(abs, clips[n]), default=0) for n in PIANO), default=0)
    if peak == 0:
        raise ValueError('Silent piano bank cannot be calibrated')
    gain = 0.8 * 32767 / peak
    destination.mkdir(mode=0o700)  # Existing directory, including sources, fails closed.
    for name in PIANO:
        samples = array.array('h', (round(v * gain) for v in clips[name]))
        if sys.byteorder != 'little':
            samples.byteswap()
        with wave.open(str(destination / (name + '.wav')), 'wb') as wav:
            wav.setparams((1, 2, 22050, 0, 'NONE', 'not compressed'))
            wav.writeframes(samples.tobytes())
    for name in AMBIENCE:
        shutil.copyfile(source / (name + '.wav'), destination / (name + '.wav'))
    result = dict(status='diagnostic-only; human listening pending', shared_piano_gain=gain,
                  source_hashes=hashes, family_peak=round(0.8 * 32767),
                  application_master_limit=25, maximum_music_peak_full_scale=0.2)
    (destination / 'calibration.json').write_text(json.dumps(result, indent=2) + '\n')
    return result

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--source', type=Path, default=Path(__file__).resolve().parents[2] / 'assets/audio/runtime')
    args = parser.parse_args()
    print(json.dumps(calibrate(args.source, args.destination), indent=2))
