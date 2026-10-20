#!/usr/bin/env python3
"""Run separately from builds, on the existing ARM64 userspace simulator."""
import json,resource,subprocess,sys,time
from pathlib import Path
start=time.monotonic();p=subprocess.run([sys.argv[1]],capture_output=True,text=True,check=True,timeout=90)
result=json.loads(p.stdout);usage=resource.getrusage(resource.RUSAGE_CHILDREN)
result['measurement']={'wall_seconds':time.monotonic()-start,'user_cpu_seconds':usage.ru_utime,'system_cpu_seconds':usage.ru_stime,'peak_rss_kib_linux':usage.ru_maxrss,'build_profile':'dev; unoptimized; not RG353V','slow_microSD':'not simulated','physical_input_latency_ms':None,'physical_fps':None}
Path(sys.argv[2]).write_text(json.dumps(result,indent=2)+'\n')
