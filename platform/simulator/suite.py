#!/usr/bin/env python3
"""Execute real ARM64 programs in a container; do not impersonate DRM or evdev."""
import hashlib
import json
import os
from pathlib import Path
import platform
import struct
import subprocess
import time

ROOT = Path('/workspace')
OUT = Path('/results')
SIM = ROOT / 'dist/simulator/emotional-ui-simulator'
KMS = ROOT / 'dist/simulator/emotional-ui-kms'
def run(name, args, expected=0, contains=None):
    # Preserve the exact legacy interaction contracts; integrated coverage is separate below.
    if args and args[0]==SIM and '--concept' not in args:
        args=[args[0],'--concept','legacy',*args[1:]]
    start = time.monotonic()
    p = subprocess.run([str(x) for x in args], capture_output=True, text=True, timeout=180 if name in ("phase7-integrated", "phase8-sensory", "phase9-launch", "phase10-discovery", "phase10-library-perf", "phase11-trial", "phase11-xml-perf") else 40)
    (OUT / (name + '.stdout')).write_text(p.stdout)
    (OUT / (name + '.stderr')).write_text(p.stderr)
    ok = p.returncode == expected and (contains is None or contains in p.stdout + p.stderr)
    checks.append({'case': name, 'exit_code': p.returncode, 'passed': ok,
                   'elapsed_seconds': time.monotonic() - start})
    print(name, 'PASS' if ok else 'FAIL', flush=True)
    return p

def main():
    if platform.machine() not in ('aarch64', 'arm64'):
        raise RuntimeError('ARM64 runtime required')
    for binary in (SIM, KMS):
        data = binary.read_bytes()
        if data[:6] != b'\x7fELF\x02\x01' or struct.unpack_from('<H', data, 18)[0] != 183:
            raise RuntimeError(f'Not an ARM64 ELF: {binary}')
    inventory = {
        'kind': 'container-userspace-simulation', 'architecture': platform.machine(),
        'kernel': platform.release(), 'os_release': Path('/etc/os-release').read_text(),
        'glibc': platform.libc_ver(), 'uid': os.getuid(),
        'drm_present': Path('/dev/dri').exists(),
        'artifacts': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (SIM, KMS)},
        'physical_display_verified': False, 'physical_input_verified': False,
        'presented_fps': None, 'input_to_photon_ms': None,
    }
    (OUT / 'environment.json').write_text(json.dumps(inventory, indent=2) + '\n')
    for binary in sorted((ROOT / 'dist/simulator/tests').iterdir()):
        run('linux-' + binary.name, [binary, '--test-threads=2'])
    run('kms-cli', [KMS, '--help'])
    # Same actual KMS binary: missing hardware must fail clearly, not be faked.
    p = subprocess.run([str(KMS), '--db', ':memory:', '--smoke-test'], capture_output=True, text=True, timeout=20)
    (OUT / 'kms-no-device.stderr').write_text(p.stderr)
    checks.append({'case': 'kms-refuses-absent-drm', 'passed': p.returncode != 0 and not inventory['drm_present'], 'exit_code': p.returncode})
    for count in (6, 0, 1, 2):
        run(f'ui-catalog-{count}', [SIM, '--db', ':memory:', '--at', '2026-10-18T21:30', '--demo-count', count, '--ui-test'], contains='UI ')
    for mood, hour in [('despertar', '08:00'), ('horizonte', '15:00'), ('refugio', '21:30'), ('silencio', '02:00')]:
        path = OUT / (mood + '.png')
        run('palette-' + mood, [SIM, '--db', ':memory:', '--at', '2026-10-18T' + hour, '--reduced-motion', '--snapshot', path], contains='rasterized_frames=')
        dimensions = struct.unpack('>II', path.read_bytes()[16:24]) if path.exists() else None
        checks.append({'case':'opaque-'+mood,'passed':path.exists() and path.read_bytes()[25]==2})
        checks.append({'case': 'size-' + mood, 'passed': dimensions == (640, 480), 'dimensions': dimensions})
    for moment in ['summer-morning','autumn-rain','halloween','christmas','winter-sunday','quiet-dawn']:
        path=OUT/(moment+'.png')
        run('moment-'+moment,[SIM,'--db',':memory:','--dev-moment',moment,'--snapshot',path],contains='rasterized_frames=')
        checks.append({'case':'moment-size-'+moment,'passed':path.exists() and struct.unpack('>II',path.read_bytes()[16:24])==(640,480) and path.read_bytes()[25]==2})
    run('unknown-time',[SIM,'--db',':memory:','--unknown-time','--snapshot',OUT/'unknown-time.png'])
    run('stale-weather',[SIM,'--db',':memory:','--dev-moment','autumn-rain','--dev-weather-age','181','--snapshot',OUT/'stale-weather.png'])
    # Entire import -> own SQLite -> GUI -> actual library navigation pipeline.
    import_dir=OUT/'fixture-library';import_dir.mkdir()
    xml='<gameList>'
    for i in range(12):
        (import_dir/f'fiction-{i:02}.test').write_text('Original synthetic test data; not a ROM.')
        xml+=f'<game><path>fiction-{i:02}.test</path><name>Camino de papel {i:02}</name><desc>Un lugar para perder la prisa.</desc><favorite>{str(i==0).lower()}</favorite><playcount>4</playcount></game>'
    xml+='</gameList>'
    source=import_dir/'gamelist.xml';source.write_text(xml)
    before={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in import_dir.iterdir()}
    db=OUT/'library.sqlite'
    run('import-library',[SIM,'--import-gamelist',source,'--rom-root',import_dir,'--db',db])
    scenario=OUT/'library-navigation.toml'
    replay='keyboard=true\n[mapping]\n'
    steps=[dict(at_ms=80,text='a',pressed=True,library=True),dict(at_ms=88,text='a',pressed=False)]
    for i in range(9):
        steps.extend([dict(at_ms=120+i*32,text='down',pressed=True,library_index=i+1),dict(at_ms=128+i*32,text='down',pressed=False)])
    steps.extend([dict(at_ms=500,text='s',pressed=True,menu=True),dict(at_ms=508,text='s',pressed=False),dict(at_ms=540,text='x',pressed=True,menu=False,library=True,library_index=9),dict(at_ms=548,text='x',pressed=False)])
    for step in steps:
        replay+='\n[[steps]]\n'+''.join(f'{k}={json.dumps(v)}\n' for k,v in step.items())
    scenario.write_text(replay)
    run('imported-gui',[SIM,'--db',db,'--library-root',import_dir,'--at','2026-10-18T21:30','--sim-replay',scenario,'--snapshot',OUT/'imported-library.png'],contains='raw-input replay steps passed')
    after={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in import_dir.iterdir()}
    checks.append({'case':'import-preserves-originals','passed':before==after})
    for scenario in sorted((ROOT / 'platform/simulator/scenarios').glob('*.toml')):
        run('raw-' + scenario.stem, [SIM, '--db', ':memory:', '--at', '2026-10-18T21:30', '--smoke-test', '--smoke-seconds', '3', '--sim-replay', scenario], contains='raw-input replay steps passed')
    run('phase7-integrated', ['python3', ROOT/'platform/simulator/phase7.py',SIM,OUT/'phase7'])
    run('phase8-sensory', ['python3', ROOT/'platform/simulator/phase8.py',SIM,OUT/'phase8'])
    run('phase9-launch', ['python3', ROOT/'platform/simulator/phase9.py',SIM,OUT/'phase9'])
    run('phase10-discovery', ['python3', ROOT/'platform/simulator/phase10.py', SIM, OUT/'phase10'])
    run('phase11-trial', ['python3', ROOT/'platform/simulator/phase11.py', SIM, OUT/'phase11'])
    run('phase11-xml-perf', ['python3', ROOT/'platform/simulator/phase11_performance.py', SIM, OUT/'phase11-performance'])
    run('phase10-library-perf', ['python3', ROOT/'platform/simulator/phase10_performance.py', SIM, OUT/'phase10-performance'])
    run('invalid-date', [SIM, '--db', ':memory:', '--at', '2026-02-30T20:00'], expected=1)
    run('invalid-replay', [SIM, '--sim-replay', ROOT / 'platform/simulator/fixtures/invalid-replay.toml'], expected=1)
    # Real external cancellation, bounded independently of the UI loop.
    quit_file = OUT / 'cancel.quit'
    journal = OUT / 'cancel.toml'
    p = subprocess.Popen([str(SIM), '--db', ':memory:', '--smoke-test', '--smoke-seconds', '30', '--smoke-log', str(journal), '--quit-file', str(quit_file)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            if journal.exists() and 'event_loop_alive' in journal.read_text(): break
            if p.poll() is not None: raise RuntimeError('Exited before cancellation')
            time.sleep(.05)
        else: raise RuntimeError('Event loop not ready')
        quit_file.touch()
        stdout, stderr = p.communicate(timeout=5)
        (OUT / 'cancel.stdout').write_text(stdout)
        (OUT / 'cancel.stderr').write_text(stderr)
        checks.append({'case': 'external-cancellation', 'passed': p.returncode == 0 and 'external_quit_requested' in journal.read_text(), 'exit_code': p.returncode})
    finally:
        if p.poll() is None: p.kill(); p.communicate()

checks = []
if __name__ == '__main__':
    if (OUT/'report.json').exists():
        raise SystemExit('Use a new output folder; preserve previous evidence')
    error = None
    try: main()
    except Exception as e: error = repr(e)
    report = {'checks': checks, 'error': error, 'passed': not error and bool(checks) and all(c['passed'] for c in checks), 'scope': 'Linux ARM64 container. No Rockchip DRM, physical buttons, touchscreen, EmulationStation recovery or hardware performance validation.'}
    (OUT / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['passed'] else 1)
