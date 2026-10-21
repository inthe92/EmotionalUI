#!/usr/bin/env python3
"""Bounded local software-renderer comparison; no LCD/FPS certification or network.
Use a release --no-default-features --features simulator binary. No screenshots
are encoded during timing; CPU phases exclude startup and use measured intervals.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def process_sample(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'time=,rss='], capture_output=True, text=True)
    if result.returncode or not result.stdout.strip():
        return None
    cpu, rss = result.stdout.split()
    seconds = 0.
    for part in cpu.split(':'):
        seconds = seconds * 60 + float(part)
    return dict(cpu_seconds=seconds, rss_kib=int(rss))


def phase(samples, lo, hi):
    rows = [s for s in samples if lo <= s['wall_seconds'] <= hi]
    if len(rows) < 2:
        return None
    elapsed = rows[-1]['wall_seconds'] - rows[0]['wall_seconds']
    return dict(wall_seconds=elapsed, one_core_cpu_percent=100*(rows[-1]['cpu_seconds']-rows[0]['cpu_seconds'])/elapsed,
                max_rss_kib=max(s['rss_kib'] for s in rows))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    args.out.mkdir(parents=True, exist_ok=False)
    args.out.chmod(0o700)
    config = args.out/'context.toml'
    config.write_text('at="2026-12-25T16:00"\nweather="snow"\n[ambient]\ntimezone="Etc/UTC"\nhemisphere="north"\n')
    replay = args.out/'navigation.toml'
    lines = ['keyboard=true', '[mapping]']
    for index, at in enumerate([2000, 2600, 3200, 3800, 4400, 5000]):
        for when, pressed in [(at, True), (at+30, False)]:
            lines += ['[[steps]]', f'at_ms={when}', 'text="'+('e' if index % 2 == 0 else 'q')+'"',
                      'pressed='+str(pressed).lower()]
    lines += ['[[steps]]', 'at_ms=7400', 'resume=true']
    replay.write_text('\n'.join(lines)+'\n')
    results = []
    # Interleave resources with the same mode to avoid comparing different layouts.
    for mode in ['full', 'static', 'simple']:
        for resource in ['svg', 'cached']:
            name = mode+'-'+resource
            cmd = [str(binary), '--db', str(args.out/(name+'.sqlite')), '--demo-history', '--visual-case',
                   'resume-three', '--sim-replay', str(replay), '--config', str(config), '--view', 'resume',
                   '--audio-mode', 'silence', '--smoke-test', '--smoke-seconds', '9']
            env = dict(os.environ, EMOTIONAL_PERF=mode, EMOTIONAL_PERF_RESOURCES=resource)
            # Full-speed metrics would force redraws and invalidate idle measurements.
            env.pop('SLINT_DEBUG_PERFORMANCE', None)
            samples = []
            with (args.out/(name+'.log')).open('x') as log:
                started = time.monotonic()
                child = subprocess.Popen(cmd, env=env, stdout=log, stderr=subprocess.STDOUT)
                while child.poll() is None:
                    elapsed = time.monotonic()-started
                    if elapsed > 20:
                        child.terminate()
                        child.wait(timeout=5)
                        raise RuntimeError('Local diagnostic exceeded deadline')
                    row = process_sample(child.pid)
                    if row:
                        samples.append(dict(wall_seconds=time.monotonic()-started, **row))
                    time.sleep(.25)
                if child.returncode:
                    raise RuntimeError(f'{name} failed; inspect its log')
            text = (args.out/(name+'.log')).read_text()
            if '13 raw-input replay steps passed' not in text:
                raise RuntimeError('Navigation did not complete')
            metrics = {}
            for line in text.splitlines():
                match = re.match(r'PERF metric=(\S+) n=(\d+) median=([\d.]+) p95=([\d.]+) max=([\d.]+) total=([\d.]+)', line)
                if match:
                    metrics[match[1]] = dict(zip(['n', 'median_ms', 'p95_ms', 'max_ms', 'total_ms'], map(float, match.groups()[1:])))
            row = dict(mode=mode, resources=resource, metrics=metrics, samples=samples,
                       idle=phase(samples,.7,1.8), navigation=phase(samples,2.3,5.4), settled=phase(samples,6.1,8.6),
                       frames_by_second=next((line for line in text.splitlines() if 'raster_frames_by_second=' in line), None))
            results.append(row)
            print(name, json.dumps({k:row[k] for k in ['metrics','idle','navigation','settled','frames_by_second']}), flush=True)
    report = dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), platform=os.uname().sysname,
                  backend='Slint software offscreen 640x480', physical_tested=False, lcd_latency_measured=False,
                  comparisons=results)
    (args.out/'measurements.json').write_text(json.dumps(report, indent=2)+'\n')


if __name__ == '__main__':
    main()
