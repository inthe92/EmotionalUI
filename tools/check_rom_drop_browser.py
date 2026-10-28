#!/usr/bin/env python3
"""Exercise the mobile page with synthetic files and a temporary library."""
import argparse
import importlib.util
import os
import subprocess
import tempfile
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--node', required=True)
parser.add_argument('--node-path', required=True)
parser.add_argument('--browser', required=True)
parser.add_argument('--screenshot')
args = parser.parse_args()
spec = importlib.util.spec_from_file_location('rom_drop', ROOT/'platform/darkos/rom_drop.py')
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as tmp:
    root = Path(tmp); (root/'nes').mkdir(); (root/'snes').mkdir()
    systems = [{'id':'s0', 'name':'NES', 'root':root/'nes', 'extensions':['.nes','.zip']},
               {'id':'s1', 'name':'SNES', 'root':root/'snes', 'extensions':['.sfc','.zip']}]
    server = module.DropServer(('127.0.0.1', 0), systems, 'synthetic-token')
    worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    env = os.environ.copy()
    env.update(NODE_PATH=args.node_path, EMOTIONAL_BROWSER=args.browser,
               DROP_TEST_URL='http://127.0.0.1:'+str(server.server_port)+'/#synthetic-token')
    if args.screenshot: env['DROP_TEST_SCREENSHOT'] = args.screenshot
    try:
        subprocess.run([args.node, str(ROOT/'tests/browser/rom-drop.cjs')], env=env, check=True, timeout=40)
        assert (root/'nes/primero.nes').read_bytes() == b'synthetic-nes'
        assert (root/'snes/segundo.zip').read_bytes() == b'synthetic-zip'
    finally:
        server.shutdown(); server.server_close(); worker.join()
