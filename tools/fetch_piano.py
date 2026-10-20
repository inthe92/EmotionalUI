#!/usr/bin/env python3
"""Fetch only pinned CC0 WAV data; never installs or executes downloaded code."""
from pathlib import Path
import argparse,hashlib,json,urllib.request
ROOT=Path(__file__).resolve().parent.parent
p=argparse.ArgumentParser();p.add_argument('--cache',type=Path,required=True);a=p.parse_args();a.cache.mkdir(parents=True,exist_ok=True)
for entry in json.loads((ROOT/'assets/audio/phase9/instrument/sources.lock.json').read_text())['files']:
 if not entry['path'].endswith('.wav'):continue
 path=a.cache/entry['cache_file']
 if path.exists():data=path.read_bytes()
 else:data=urllib.request.urlopen(entry['url'],timeout=60).read()
 if len(data)!=entry['bytes'] or hashlib.sha256(data).hexdigest()!=entry['sha256']:raise SystemExit('Sample checksum mismatch: '+str(path))
 if not path.exists():
  with path.open('xb') as f:f.write(data)
 print(path.name,entry['sha256'])
