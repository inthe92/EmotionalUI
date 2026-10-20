#!/usr/bin/env python3
"""Summarize actual /proc samples and journal timestamps; never infers presented FPS."""
import argparse,json,re
from pathlib import Path

def summarize(rows,journal):
    if not rows:raise ValueError('No process samples')
    identities={r['pid_start_ticks'] for r in rows if 'pid_start_ticks' in r}
    if len(identities)>1:raise ValueError('PID reuse invalidates this measurement')
    hz={r['clock_ticks_per_second'] for r in rows if 'clock_ticks_per_second' in r}
    ticks=rows[-1]['cpu_ticks']-rows[0]['cpu_ticks'];seconds=(rows[-1]['monotonic_ns']-rows[0]['monotonic_ns'])/1e9
    if ticks<0 or seconds<=0:raise ValueError('Invalid process sample interval')
    events={}
    for block in journal.split('[[events]]')[1:]:
        name=re.search(r'^event = "([a-z_]+)"$',block,re.M);elapsed=re.search(r'^elapsed_ms = (\d+)$',block,re.M)
        if name and elapsed:events[name.group(1)]=int(elapsed.group(1))
    frequency=next(iter(hz)) if len(hz)==1 else None
    close=events.get('process_exit',0)-events.get('smoke_deadline',0) if 'smoke_deadline' in events and 'process_exit' in events else None
    return {'kind':'process-smoke-measurement','samples':len(rows),'sample_span_seconds':seconds,'peak_rss_mib':max(r['peak_rss_kib'] for r in rows)/1024,'last_rss_mib':rows[-1]['rss_kib']/1024,'cpu_ticks_observed':ticks,'clock_ticks_per_second':frequency,'cpu_one_core_percent':100*ticks/frequency/seconds if frequency else None,'window_created_elapsed_ms':events.get('window_created'),'event_loop_alive_elapsed_ms':events.get('event_loop_alive'),'close_request_to_journal_exit_ms':close,'presented_fps':None,'frame_times_ms':None,'input_to_photon_ms':None,'limitations':'/proc interval and app journal only; no presentation/latency measurement. Older captures may omit CLK_TCK. Not a navigation benchmark.'}
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('samples',type=Path);p.add_argument('journal',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
    report=summarize([json.loads(line) for line in a.samples.read_text().splitlines() if line.strip()],a.journal.read_text())
    with a.output.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
    print(json.dumps(report,indent=2))
if __name__=='__main__':main()
