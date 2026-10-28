#!/usr/bin/env python3
"""Rebuild the bounded real metadata pilot offline from saved CC0 snapshots.

No network, model calls, personal library or device access. Original artifacts
are inputs only. The Super Metroid mapping is explicit, not a title resolver.
"""
import argparse
import json
from pathlib import Path
import atlas
from atlas_store import Store

ROOT=Path(__file__).resolve().parents[1]


def build(store_path, output):
    if store_path.exists() or output.exists() or output.with_name(output.name+'.building').exists():
        raise ValueError('New research store and export paths required')
    store=Store(store_path)
    try:
        store.import_atlas(atlas.load(ROOT/'assets/atlas/source/snes-evidence-experimental.json'))
        store.import_records(atlas.load(ROOT/'assets/atlas/imports/wikidata-20261024.json'))
        mapping=atlas.load(ROOT/'assets/atlas/imports/super-metroid-reconciliation-20261024.json')
        store.reconcile(mapping['alias'],mapping['canonical'],mapping['receipt'])
        store.import_records(atlas.load(ROOT/'assets/atlas/imports/wikidata-real-pilot-20261024.json'))
        if store.stats()['implementations']!=300:raise ValueError('Expected 300 real implementation identities')
        export=store.export(output,'0.3.0-real-pilot.1')
        return dict(stats=store.stats(),export=export)
    finally:store.close()


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--store',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();print(json.dumps(build(a.store,a.output),indent=2))
