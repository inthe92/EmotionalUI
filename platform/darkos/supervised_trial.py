#!/usr/bin/env python3
"""A supervised, temporary display hand-off. Never enables/disables/edits services.
Requires a separately armed systemd recovery timer and a different SSH connection.
"""
import argparse,json,os,re,subprocess,time
from pathlib import Path
UNIT='emulationstation.service'
def command(args,timeout=15):
    try:
        p=subprocess.run(args,capture_output=True,text=True,timeout=timeout)
        return {'code':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
    except subprocess.TimeoutExpired:return {'code':124,'stdout':'','stderr':'Command timed out'}
    except OSError as error:return {'code':127,'stdout':'','stderr':str(error)}
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('bundle',type=Path);p.add_argument('logs',type=Path)
    p.add_argument('--seconds',type=int,default=2);p.add_argument('--input-profile',type=Path);p.add_argument('--recovery-timer',required=True);p.add_argument('--recovery-connection',required=True);p.add_argument('--supervised',action='store_true')
    a=p.parse_args()
    if not 1<=a.seconds<=30:raise SystemExit('Duration must be 1..30 seconds')
    if not a.supervised or not os.environ.get('SSH_CONNECTION') or os.environ['SSH_CONNECTION']==a.recovery_connection:raise SystemExit('Supervision and independent SSH recovery required')
    if not re.fullmatch(r'emotional-ui-recovery-[a-z0-9-]+\.timer',a.recovery_timer):raise SystemExit('Unexpected recovery timer')
    timer=command(['systemctl','show',a.recovery_timer,'-p','ActiveState','-p','SubState','-p','NextElapseUSecMonotonic'])
    if 'ActiveState=active' not in timer['stdout'] or 'SubState=waiting' not in timer['stdout']:raise SystemExit('Independent recovery timer is not armed')
    running=command(['ps','-eo','comm='])['stdout'].splitlines()
    if any(x.strip() in {'retroarch','retroarch32','retroarch64','ppsspp','ppsspp64','Xorg','Xwayland','weston'} for x in running):raise SystemExit('An emulator or another display owner is active')
    before=command(['systemctl','show',UNIT,'-p','ActiveState','-p','MainPID'])
    if 'ActiveState=active' not in before['stdout']:raise SystemExit('Original frontend must be active before the trial')
    preflight=command([str(a.bundle/'run-preflight.sh')])
    if preflight['code']:raise SystemExit(preflight)
    # Re-check after preflight: never stop the frontend with a nearly expired timer.
    bus_path='/org/freedesktop/systemd1/unit/'+''.join(c if c.isascii() and c.isalnum() else f'_{ord(c):02x}' for c in a.recovery_timer)
    deadline=command(['busctl','--system','get-property','org.freedesktop.systemd1',bus_path,'org.freedesktop.systemd1.Timer','NextElapseUSecMonotonic'])
    match=re.fullmatch(r't (\d+)\s*',deadline['stdout'])
    if not match or int(match.group(1))/1e6-time.monotonic()<a.seconds+30:raise SystemExit('Recovery timer has insufficient time remaining')
    # Do not stop an emulator launched under the existing frontend's service.
    main=int(re.search(r'MainPID=(\d+)',before['stdout']).group(1))
    tree=command(['ps','-eo','pid=,ppid=,comm='])['stdout']
    children=[line.split(None,2) for line in tree.splitlines() if len(line.split(None,2))==3]
    descendants={main};changed=True
    while changed:
        changed=False
        for pid,parent,name in children:
            if int(parent) in descendants and int(pid) not in descendants:descendants.add(int(pid));changed=True
    allowed={'emulationstatio','emulationstation','bash','sh','sleep','ogage'}
    if any(int(pid) in descendants and name not in allowed for pid,parent,name in children):raise SystemExit('An unrecognized child of EmulationStation is active; inspect it first')
    a.logs.mkdir(exist_ok=False)
    report={'kind':'supervised-display-trial','before':before,'recovery_timer':timer,'preflight':preflight,'physical_display_verified':False,'physical_restoration_verified':False}
    start=time.monotonic()
    try:
        report['stop']=command(['sudo','-n','systemctl','stop',UNIT],timeout=8)
        if report['stop']['code']:raise RuntimeError('Could not temporarily stop original frontend')
        launch=[str(a.bundle/'run-smoke.sh'),str(a.logs/'smoke'),'--recovery-confirmed','--seconds',str(a.seconds)]
        if a.input_profile:launch+=['--input-profile',str(a.input_profile)]
        report['smoke']=command(launch,timeout=a.seconds+15)
    except Exception as e:report['error']=str(e)
    finally:
        report['restore']=command(['sudo','-n','systemctl','start',UNIT],timeout=20)
        report['after']=command(['systemctl','show',UNIT,'-p','ActiveState','-p','MainPID'])
        report['elapsed_ms']=(time.monotonic()-start)*1000
        (a.logs/'trial.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
    if report.get('error') or report.get('smoke',{}).get('code') or report['restore']['code'] or 'ActiveState=active' not in report['after']['stdout']:raise SystemExit(1)
if __name__=='__main__':main()
