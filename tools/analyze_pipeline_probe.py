#!/usr/bin/env python3
"""Diagnostic A: confirmed kernel flips inside bounded, actually rendered moves."""
import argparse
import bisect
import json
import re
from pathlib import Path
from analyze_gpu_trial import fields
from analyze_gpu_stages import summarize


def analyze(text):
    trace=[l for l in text.splitlines() if l.startswith('KMS_DRM_TRACE ')]
    if len(trace)!=1:raise ValueError('Exactly one final DRM trace required')
    m=re.fullmatch(r'KMS_DRM_TRACE format=sequence:kernel_us retained=(\d+) dropped=(\d+) stamps=\[(.*)\]',trace[0])
    if not m:raise ValueError('Malformed DRM trace')
    retained,dropped=int(m[1]),int(m[2]); raw=m[3].split(',') if m[3] else []
    if retained>16384 or len(raw)!=retained or any(not re.fullmatch(r'\d+:\d+',r) for r in raw):raise ValueError('Invalid bounded trace')
    rows=[tuple(map(int,r.split(':'))) for r in raw]
    if any(b[1]<=a[1] or not 0<((b[0]-a[0])&0xffffffff)<0x80000000 for a,b in zip(rows,rows[1:])):raise ValueError('Nonmonotonic DRM confirmations')
    counter_lines=[fields(l) for l in text.splitlines() if l.startswith('KMS_DRM present_calls=')]
    if not counter_lines:raise ValueError('Missing DRM counters')
    c=counter_lines[-1]
    if c.get('timestamp_monotonic')!='Some(true)' or int(c['kernel_confirmed'])!=retained+dropped:raise ValueError('Clock/count mismatch')
    if int(c['submit_errors']) or int(c['receive_errors']):raise ValueError('DRM errors')
    begin={};ends={}
    for line in text.splitlines():
        if line.startswith('PIPELINE_MOVE_BEGIN '):
            f=fields(line);i=int(f['id'])
            if i in begin:raise ValueError('Duplicate movement start')
            begin[i]=(int(f['kernel_us']),float(f['target']))
        elif line.startswith('PIPELINE_MOVE_END '):
            f=fields(line);i=int(f['id'])
            if i in ends:raise ValueError('Duplicate movement end')
            ends[i]=(int(f['kernel_us']),float(f['offset']))
    if not begin:raise ValueError('No measured moves')
    stamps=[t for _,t in rows];groups=[];intervals=[]
    ids=list(begin)
    if ids!=list(range(1,len(ids)+1)):raise ValueError('Incomplete movement order')
    for index,i in enumerate(ids):
        at,target=begin[i];end,offset=ends.get(i,(None,None))
        next_start=begin[ids[index+1]][0] if index+1<len(ids) else None
        if end is not None and (end<at or target!=offset):raise ValueError('Invalid rendered endpoint')
        first=bisect.bisect_left(stamps,at);last=bisect.bisect_left(stamps,end) if end else len(stamps)
        covered=end is not None and last<len(stamps) and (next_start is None or stamps[last]<next_start)
        selected=stamps[first:last+1] if covered else []
        values=[(b-a)/1000 for a,b in zip(selected,selected[1:])];intervals.extend(values)
        groups.append(dict(id=i,coverage='COMPLETE' if covered else 'INCOMPLETE',confirmations=len(selected),intervals=summarize(values),model_duration_ms=(end-at)/1000 if end else None))
    result=dict(kind='A',source='confirmed DRM timestamps',groups=groups,
        confirmed_total=retained,dropped=dropped,pending_not_counted=int(c['pending_or_unobserved']),
        pooled_intervals=summarize(intervals),pooled_update_hz=1000*len(intervals)/sum(intervals) if intervals else None,
        physical_input_to_photon='UNMEASURED',gpu_execution_time='UNMEASURED',cpu_rss='EXTERNAL_SAMPLER',
        stability='SHORT_DIAGNOSTIC_ONLY',sixty_fps_certified=False)
    # Deliberately no boolean FPS approval: inspect all groups, incomplete coverage
    # and tails. A median or a timer frequency must never grant certification.
    return result


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('log',type=Path);a=p.parse_args()
    if a.log.stat().st_size>32*1024*1024:p.error('Oversized log')
    print(json.dumps(analyze(a.log.read_text()),indent=2))


if __name__=='__main__':main()
