#!/usr/bin/env python3
"""Sample /proc for one process. CPU/RSS only, never FPS or input-to-photon latency."""
import argparse,json,os,time
from pathlib import Path

def sample(pid):
    text=Path(f'/proc/{pid}/stat').read_text();fields=text[text.rfind(')')+2:].split()
    status=Path(f'/proc/{pid}/status').read_text();values={line.split(':',1)[0]:line.split(':',1)[1].strip() for line in status.splitlines() if ':' in line}
    return {'unix_ns':time.time_ns(),'monotonic_ns':time.monotonic_ns(),'cpu_ticks':int(fields[11])+int(fields[12]),'clock_ticks_per_second':os.sysconf('SC_CLK_TCK'),'pid_start_ticks':int(fields[19]),'rss_kib':int(values.get('VmRSS','0 kB').split()[0]),'peak_rss_kib':int(values.get('VmHWM','0 kB').split()[0]),'threads':int(values.get('Threads','0'))}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('pid',type=int);p.add_argument('--seconds',type=int,default=10);args=p.parse_args()
    if args.pid<=0 or not 1<=args.seconds<=60:raise SystemExit('Invalid PID or duration')
    if not Path('/proc').exists():raise SystemExit('Linux /proc required')
    previous=None;hz=os.sysconf('SC_CLK_TCK')
    for _ in range(args.seconds):
        try:row=sample(args.pid)
        except FileNotFoundError:print(json.dumps({'event':'process_exited','pid':args.pid}));break
        if previous and previous['pid_start_ticks']!=row['pid_start_ticks']:raise SystemExit('PID was reused; sampling stopped')
        if previous:row['cpu_one_core_percent']=100*(row['cpu_ticks']-previous['cpu_ticks'])/hz/((row['monotonic_ns']-previous['monotonic_ns'])/1e9)
        print(json.dumps(row),flush=True);previous=row;time.sleep(1)
if __name__=='__main__':main()
