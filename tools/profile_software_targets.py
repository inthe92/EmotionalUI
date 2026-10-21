#!/usr/bin/env python3
"""Isolate RAM pixel-format and full-repaint costs; never DRM/LCD certification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
from profile_rendering import process_sample, phase


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    a.out.mkdir(parents=True, exist_ok=False)
    a.out.chmod(0o700)
    binary = a.binary.resolve()
    config = a.out/'context.toml'
    config.write_text('at="2026-12-25T16:00"\nweather="snow"\n[ambient]\ntimezone="Etc/UTC"\nhemisphere="north"\n')
    replay = a.out/'navigation.toml'
    lines = ['keyboard=true', '[mapping]']
    for i, at in enumerate([2000, 2600, 3200, 3800, 4400, 5000]):
        for when, pressed in [(at, True), (at+30, False)]:
            lines += ['[[steps]]', f'at_ms={when}', 'text="'+('e' if i % 2 == 0 else 'q')+'"',
                      'pressed='+str(pressed).lower()]
    lines += ['[[steps]]', 'at_ms=7400', 'resume=true']
    replay.write_text('\n'.join(lines)+'\n')
    results = []
    for target, buffer in [('rgb', 'reused'), ('rgb', 'new'), ('xrgb', 'new'), ('xrgb', 'reused')]:
        name = f'{target}-{buffer}'
        env = dict(os.environ, EMOTIONAL_PERF='full', EMOTIONAL_PERF_RESOURCES='cached',
                   EMOTIONAL_PERF_TARGET=target, EMOTIONAL_PERF_BUFFER=buffer)
        env.pop('SLINT_DEBUG_PERFORMANCE', None)
        cmd = [str(binary), '--db', str(a.out/(name+'.sqlite')), '--demo-history', '--visual-case',
               'resume-three', '--sim-replay', str(replay), '--config', str(config), '--view', 'resume',
               '--audio-mode', 'silence', '--smoke-test', '--smoke-seconds', '9']
        samples = []
        with (a.out/(name+'.log')).open('x') as log:
            start = time.monotonic()
            child = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT)
            while child.poll() is None:
                if time.monotonic()-start > 20:
                    child.terminate()
                    child.wait(timeout=5)
                    raise RuntimeError('Diagnostic deadline exceeded')
                row = process_sample(child.pid)
                if row:
                    samples.append(dict(wall_seconds=time.monotonic()-start, **row))
                time.sleep(.25)
            if child.returncode:
                raise RuntimeError(f'{name} failed: inspect log')
        content = (a.out/(name+'.log')).read_text()
        if '13 raw-input replay steps passed' not in content:
            raise RuntimeError('Navigation replay not completed')
        metrics = {}
        for line in content.splitlines():
            m = re.match(r'PERF metric=(\S+) n=(\d+) median=([\d.]+) p95=([\d.]+) max=([\d.]+) total=([\d.]+)', line)
            if m:
                metrics[m[1]] = dict(zip(['n', 'median_ms', 'p95_ms', 'max_ms', 'total_ms'], map(float, m.groups()[1:])))
        results.append(dict(target=target, buffer=buffer, metrics=metrics, samples=samples,
                            navigation=phase(samples, 2.3, 5.4), settled=phase(samples, 6.1, 8.6)))
        print(name, json.dumps(metrics.get('software_raster_ms')), flush=True)
    result = dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), platform=os.uname().sysname,
                  physical_tested=False, drm_mapping_tested=False, lcd_frame_times_measured=False,
                  comparisons=results)
    (a.out/'measurements.json').write_text(json.dumps(result, indent=2)+'\n')


if __name__ == '__main__':
    main()
