#!/usr/bin/env python3
"""Temporary manual test only. NEVER stops/starts graphical services or changes a VT."""
import argparse,hashlib,json,os,platform,signal,struct,subprocess,sys,threading,time
from pathlib import Path
from sample_process import sample

def journal(path,value):path.write_text(json.dumps(value,indent=2)+'\n')
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('binary',type=Path);p.add_argument('log_dir',type=Path);p.add_argument('--sha256',required=True);p.add_argument('--seconds',type=int,default=2);p.add_argument('--input-profile',type=Path);p.add_argument('--recovery-confirmed',action='store_true');args=p.parse_args()
    if not 1<=args.seconds<=30:raise SystemExit('Smoke duration must be 1..30 seconds')
    if platform.system()!='Linux' or platform.machine()!='aarch64':raise SystemExit('Linux aarch64 required')
    if not args.recovery_confirmed or not os.environ.get('SSH_CONNECTION'):raise SystemExit('Use SSH and independently verify a second session/recovery before this manual test')
    data=args.binary.read_bytes()
    if hashlib.sha256(data).hexdigest()!=args.sha256:raise SystemExit('Checksum mismatch')
    if len(data)<64 or data[:6]!=b'\x7fELF\x02\x01' or struct.unpack_from('<H',data,18)[0]!=183:raise SystemExit('ELF64 AArch64 required')
    running=subprocess.run(['ps','-eo','comm='],capture_output=True,text=True,check=True).stdout.splitlines()
    forbidden={'emulationstatio','emulationstation','Xorg','Xwayland','weston'}
    if any(name.strip() in forbidden for name in running):raise SystemExit('A graphical frontend is active; no process was stopped')
    args.log_dir.mkdir(parents=False,exist_ok=False)
    folder=args.log_dir.resolve();script=Path(__file__).resolve().parent
    preflight=subprocess.run([sys.executable,str(script/'preflight.py'),str(args.binary),'--sha256',args.sha256],capture_output=True,text=True,timeout=15)
    (folder/'preflight.stdout').write_text(preflight.stdout);(folder/'preflight.stderr').write_text(preflight.stderr)
    if preflight.returncode:
        journal(folder/'result.json',{'preflight_exit_code':preflight.returncode,'ui_started':False,'physical_display_verified':False});raise SystemExit(preflight.returncode)
    env=dict(os.environ,SLINT_BACKEND='linuxkms-software',SLINT_SCALE_FACTOR='1')
    # Keep the bounded watchdog alive if the initiating SSH connection drops.
    signal.signal(signal.SIGHUP,signal.SIG_IGN)
    def interrupted(signum,frame):raise SystemExit(128+signum)
    signal.signal(signal.SIGTERM,interrupted)
    start=time.monotonic_ns();timed_out=False;done=threading.Event()
    with (folder/'stdout.log').open('w') as out,(folder/'stderr.log').open('w') as err:
        command=[str(args.binary.resolve()),'--db',':memory:','--smoke-test','--smoke-seconds',str(args.seconds),'--smoke-log',str(folder/'events.toml'),'--quit-file',str(folder/'quit.request')]
        if args.input_profile:command+=['--input-profile',str(args.input_profile.resolve())]
        proc=subprocess.Popen(command,env=env,stdout=out,stderr=err,start_new_session=True)
        journal(folder/'started.json',{'pid':proc.pid,'binary_sha256':args.sha256,'unix_ns':time.time_ns(),'quit_request':str(folder/'quit.request')})
        def monitor():
            with (folder/'resources.jsonl').open('w') as f:
                while not done.is_set():
                    try:f.write(json.dumps(sample(proc.pid))+'\n');f.flush()
                    except (OSError,ValueError):break
                    done.wait(.2)
        thread=threading.Thread(target=monitor);thread.start()
        try:
            try:code=proc.wait(timeout=args.seconds+10)
            except subprocess.TimeoutExpired:
                timed_out=True;os.killpg(proc.pid,signal.SIGTERM)
                try:code=proc.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);code=proc.wait()
        finally:
            if proc.poll() is None:
                os.killpg(proc.pid,signal.SIGTERM)
                try:proc.wait(timeout=3)
                except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
            done.set();thread.join()
    journal(folder/'result.json',{'exit_code':code,'timed_out':timed_out,'elapsed_ms':(time.monotonic_ns()-start)/1e6,'ui_process_started':True,'physical_display_verified':False,'system_restoration_verified':False,'recovery':'Restore the original frontend from the independent session and verify the LCD manually.'})
    print('Logs:',folder);print('Process exit:',code,'timeout:',timed_out)
    raise SystemExit(code if code>=0 else 128-code)
if __name__=='__main__':main()
