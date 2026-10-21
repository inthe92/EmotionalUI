#!/usr/bin/env python3
"""Bounded CPU wall-time stages, not GPU execution time or input-to-photon."""
import argparse
import ast
import json
import math
from pathlib import Path


def summarize(values):
    values = sorted(values)
    if not values:
        return dict(n=0)
    def p(fraction):
        return values[math.ceil(len(values) * fraction) - 1]
    return dict(n=len(values), median_ms=p(.5), p95_ms=p(.95),
                p99_ms=p(.99) if len(values) >= 100 else None, max_ms=values[-1])


def analyze(text):
    lines = [l for l in text.splitlines() if l.startswith('KMS_GPU_STAGES ')]
    if len(lines) != 1 or len(lines[0]) > 3_000_000:
        raise ValueError('Exactly one bounded stage trace required')
    header, raw = lines[0].split(' rows=', 1)
    if 'format=request:begin:swap:waited:swapped:submitted unit=monotonic_us ' not in header:
        raise ValueError('Unknown stage format/clock')
    rows = ast.literal_eval(raw)
    if not isinstance(rows, list) or len(rows) > 16384:
        raise ValueError('Stage row limit exceeded')
    previous_end = 0
    for row in rows:
        if not isinstance(row, list) or len(row) != 6 or any(type(v) is not int or v < 0 for v in row):
            raise ValueError('Malformed stage row')
        if any(b < a for a, b in zip(row[1:], row[2:])) or row[1] < previous_end or row[0] > row[1]:
            raise ValueError('Nonmonotonic stage timestamps')
        previous_end = row[-1]
    result = dict(scope='CPU_WALL_TIME_NOT_GPU_EXECUTION', counters=header, frames=len(rows), windows={})
    epoch_line = next((l for l in text.splitlines() if l.startswith('GPU_REPLAY plan=fixed-v1 ')), None)
    if not epoch_line:
        raise ValueError('Missing fixed-v1 epoch')
    epoch = int(epoch_line.split('epoch_kernel_us=')[1].split(';')[0])
    for label, lo, hi in [('interaction', 10, 24), ('idle', 30, 55)]:
        selected = [r for r in rows if epoch + lo * 1_000_000 <= r[1] < epoch + hi * 1_000_000]
        metrics = {}
        for name, a, b in [('request_to_render',0,1), ('traversal_commands_until_swap',1,2),
                           ('previous_flip_wait',2,3), ('egl_swap',3,4), ('gbm_lock_fb_submit',4,5),
                           ('render_through_submission',1,5)]:
            metrics[name] = summarize([(r[b]-r[a])/1000 for r in selected if r[a]])
        result['windows'][label] = dict(frames=len(selected), metrics=metrics,
            no_render_after_previous_submission=summarize([(r[1]-rows[i-1][-1])/1000
                for i,r in enumerate(rows) if i and r in selected]))
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('log',type=Path);a=p.parse_args()
    if a.log.stat().st_size > 32*1024*1024:
        p.error('Oversized log')
    print(json.dumps(analyze(a.log.read_text()),indent=2))


if __name__ == '__main__':
    main()
