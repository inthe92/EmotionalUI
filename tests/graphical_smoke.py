#!/usr/bin/env python3
"""Desktop event-loop smoke and external cancellation; no physical LCD claims."""
import json,os,subprocess,tempfile,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
binary=ROOT/'target/debug/emotional-ui'
env=dict(os.environ,SLINT_SCALE_FACTOR='1')
results=[]
with tempfile.TemporaryDirectory(prefix='emotional-smoke-') as temp:
    folder=Path(temp)
    for name,seconds in [('deadline',1),('external',30)]:
        journal=folder/(name+'.toml');quit_file=folder/(name+'.quit')
        start=time.monotonic()
        proc=subprocess.Popen([str(binary),'--db',':memory:','--smoke-test','--smoke-seconds',str(seconds),'--smoke-log',str(journal),'--quit-file',str(quit_file)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            if name=='external':
                deadline=time.monotonic()+8
                while time.monotonic()<deadline:
                    if journal.exists() and 'event_loop_alive' in journal.read_text():break
                    if proc.poll() is not None:raise RuntimeError('UI exited before cancellation')
                    time.sleep(.05)
                else:raise RuntimeError('Event loop did not become ready')
                quit_file.touch()
            out,err=proc.communicate(timeout=10)
        except BaseException:
            proc.kill();proc.communicate();raise
        events=journal.read_text();expected='external_quit_requested' if name=='external' else 'smoke_deadline'
        assert proc.returncode==0,(out,err)
        assert expected in events and 'event_loop_returned' in events and 'process_exit' in events,events
        (ROOT/f'docs/phase3/desktop-{name}-events.toml').write_text(events)
        results.append({'case':name,'exit_code':proc.returncode,'elapsed_ms':round((time.monotonic()-start)*1000,1),'physical_display_verified':False})
    protected=folder/'protected';protected.write_text('preserve me')
    for args in [['--smoke-seconds','0'],['--smoke-seconds','31'],['--smoke-log',str(protected)]]:
        p=subprocess.run([str(binary),*args],env=env,capture_output=True,timeout=5)
        assert p.returncode!=0,args
    assert protected.read_text()=='preserve me'
(ROOT/'docs/phase3/desktop-smoke.json').write_text(json.dumps(results,indent=2)+'\n')
print('Desktop deadline, external quit, invalid durations and exclusive journal checks passed.')
