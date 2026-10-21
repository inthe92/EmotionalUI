#!/usr/bin/env python3
"""Analyze confirmed DRM events; never interpret timers or CPU draw calls as FPS."""
import argparse
import bisect
import json
import math
import re
from pathlib import Path


def fields(line):
    return dict(re.findall(r'(\w+)=([^\s;]+)', line))


def statistics(stamps):
    intervals = sorted((b - a) / 1000 for a, b in zip(stamps, stamps[1:]))
    def percentile(p):
        return intervals[max(0, math.ceil(len(intervals) * p) - 1)] if intervals else None
    return dict(confirmations=len(stamps), intervals=len(intervals), median_ms=percentile(.5),
                p95_ms=percentile(.95), p99_ms=percentile(.99) if len(intervals) >= 100 else None,
                confirmed_update_hz=(len(stamps)-1)*1e6/(stamps[-1]-stamps[0]) if intervals else None,
                observed_span_ms=(stamps[-1]-stamps[0])/1000 if intervals else 0)


def analyze(text):
    errors, warnings, traces, actions, completions, epoch = [], [], [], [], {}, None
    counters, monotonic = {}, False
    for line in text.splitlines():
        f = fields(line)
        if line.startswith('GPU_REPLAY plan='):
            if epoch is not None:
                errors.append('Multiple replay epochs')
            epoch = int(f.get('epoch_kernel_us', '-1'))
        elif line.startswith('GPU_REPLAY event=Action('):
            actions.append(dict(action=re.search(r'Action\((\w+)\)', line)[1],
                                at_ms=int(f['observed_ms']), scheduled_ms=int(f['scheduled_ms']),
                                generation=int(f['motion_generation'])))
        elif line.startswith('GPU_MOTION_COMPLETE '):
            generation = int(f['generation'])
            if generation in completions:
                errors.append('Duplicate motion completion generation')
            completions[generation] = int(f['kernel_us'])
        elif line.startswith('KMS_DRM present_calls='):
            counters = {k: int(f[k]) for k in ('kernel_confirmed', 'pending_or_unobserved', 'submit_errors', 'receive_errors')}
            monotonic = f.get('timestamp_monotonic') == 'Some(true)'
        elif line.startswith('KMS_DRM_TRACE '):
            match = re.fullmatch(r'KMS_DRM_TRACE format=sequence:kernel_us retained=(\d+) dropped=(\d+) stamps=\[(.*)\]', line)
            if not match:
                errors.append('Malformed or interrupted DRM trace')
                continue
            retained, dropped = map(int, match.group(1, 2))
            raw = match[3].split(',') if match[3] else []
            if retained > 16384 or len(raw) != retained or any(not re.fullmatch(r'\d+:\d+', item) for item in raw):
                errors.append('Invalid trace count or stamp')
                continue
            traces.append(([tuple(map(int, item.split(':'))) for item in raw], dropped))
    if epoch is None or epoch < 0:
        errors.append('Missing valid monotonic replay epoch')
    if len(traces) != 1:
        errors.append('Exactly one final DRM trace required')
    rows, dropped = traces[0] if len(traces) == 1 else ([], 0)
    if not monotonic:
        errors.append('Kernel timestamp clock not confirmed monotonic')
    if any(seq > 0xffffffff for seq, _ in rows) or any(b[1] <= a[1] or not 0 < ((b[0]-a[0]) & 0xffffffff) < 0x80000000 for a, b in zip(rows, rows[1:])):
        errors.append('Nonincreasing kernel timestamp or sequence')
    if counters.get('kernel_confirmed') != len(rows) + dropped:
        errors.append('Trace and confirmed-event count disagree')
    for key in ('submit_errors', 'receive_errors'):
        if counters.get(key, 0):
            errors.append(f'{key}={counters[key]}')
    if dropped:
        warnings.append(f'Bounded trace dropped {dropped} confirmations')
    if counters.get('pending_or_unobserved', 0):
        warnings.append('Last submitted flip(s) remain unconfirmed; not counted as presented')
    if any(a['at_ms'] < 0 for a in actions) or any(b['at_ms'] < a['at_ms'] for a, b in zip(actions, actions[1:])):
        errors.append('Invalid replay observation timestamp')
    schedule_errors = [a['at_ms'] - a['scheduled_ms'] for a in actions]
    if any(delta < 0 for delta in schedule_errors):
        warnings.append('Timer delivered action(s) before scheduled deadline; actual observed times retained')
    if len(actions) != 13 or [a['scheduled_ms'] for a in actions] != [10000,11000,12000,13000,15000,16000,17000,18000,20000,20080,20160,22000,23000] or [a['action'] for a in actions] != ['Next']*4+['Previous']*4+['Next','Previous','Next','Details','Back']:
        warnings.append('Replay action sequence incomplete or different from fixed-v1')
    result = dict(status='INVALID' if errors else 'PARTIAL' if warnings else 'VALID', errors=errors,
                  warnings=warnings, counters=counters, retained_confirmations=len(rows), dropped=dropped,
                  input_to_photon='UNMEASURED', gpu_execution_time='UNMEASURED', cpu_ram='EXTERNAL_SAMPLER',
                  action_schedule_error_ms=schedule_errors, animation_groups=[], animation_fps='UNMEASURED')
    if errors:
        return result
    stamps = [r[1] for r in rows]
    result['windows'] = {}
    for name, lo, hi in [('interaction', 10, 24), ('stable_idle', 30, 55)]:
        selected = [t for t in stamps if epoch+lo*1000000 <= t < epoch+hi*1000000]
        result['windows'][name] = dict(statistics(selected), start_s=lo, end_s=hi,
                                      updates_per_window_second=len(selected)/(hi-lo), includes_idle=True)
    spatial = [a for a in actions if a['action'] in ('Next', 'Previous')]
    index = 0
    while index < len(spatial):
        first = spatial[index]
        begin = epoch + first['at_ms']*1000
        last = first
        grouped = 1
        while True:
            end = completions.get(last['generation'])
            next_at = epoch+spatial[index+1]['at_ms']*1000 if index+1 < len(spatial) else None
            if next_at is not None and (end is None or next_at < end):
                index += 1
                last = spatial[index]
                grouped += 1
                continue
            break
        end_index = bisect.bisect_left(stamps, end) if end is not None else len(stamps)
        covered = end is not None and end >= epoch+last['at_ms']*1000 and end_index < len(stamps)
        selected = stamps[bisect.bisect_left(stamps, begin):end_index+1] if covered else []
        result['animation_groups'].append(dict(statistics(selected), actions=grouped,
            first_generation=first['generation'], final_generation=last['generation'],
            coverage='COMPLETE' if covered else 'INCOMPLETE', model_completion_kernel_us=end,
            first_confirmed_kernel_us=selected[0] if selected else None,
            last_confirmed_kernel_us=selected[-1] if selected else None))
        index += 1
    if any(g['coverage'] == 'COMPLETE' and g['intervals'] for g in result['animation_groups']):
        result['animation_fps'] = 'CONFIRMED_UPDATE_HZ_PER_COVERED_GROUP; NOT_INPUT_TO_PHOTON'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path)
    args = parser.parse_args()
    if args.log.stat().st_size > 32*1024*1024:
        parser.error('Log exceeds diagnostic size limit')
    try:
        result = analyze(args.log.read_text())
    except (ValueError, KeyError, TypeError) as error:
        result = dict(status='INVALID', errors=[f'Malformed diagnostic record: {error}'])
    print(json.dumps(result, indent=2))
    return result['status'] == 'INVALID'


if __name__ == '__main__':
    raise SystemExit(main())
