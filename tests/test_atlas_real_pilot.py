import copy
import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import atlas
from atlas_store import Store


class Reconciliation(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        self.store=Store(self.root/'research.db');self.addCleanup(self.store.close)
        self.store.import_atlas(atlas.load(ROOT/'assets/atlas/source/snes-evidence-experimental.json'))
        self.store.import_records(atlas.load(ROOT/'assets/atlas/imports/wikidata-20261024.json'))
        self.receipt=dict(basis='Explicit comparison of Nintendo SNES title and Wikidata P400; original ID retained.',
            sources=['https://www.wikidata.org/wiki/Q1194825','https://www.nintendo.com/us/store/products/super-metroid-snes/'],
            platform='snes',edition_scope='SNES implementation only; region/revision remain unspecified.',human_review=None)

    def test_explicit_mapping_preserves_profiles_and_all_original_ids(self):
        before=[tuple(r) for r in self.store.db.execute('SELECT id,stable,experimental,explanation,legacy FROM implementations WHERE legacy IS NOT NULL ORDER BY id')]
        self.assertTrue(self.store.reconcile('atlas:snes:000053','atlas:snes:000005',self.receipt))
        self.assertFalse(self.store.reconcile('atlas:snes:000053','atlas:snes:000005',self.receipt))
        after=[tuple(r) for r in self.store.db.execute('SELECT id,stable,experimental,explanation,legacy FROM implementations WHERE legacy IS NOT NULL ORDER BY id')]
        self.assertEqual(before,after)
        self.assertEqual(self.store.stats()['implementations'],52)
        self.assertEqual(self.store.stats()['unresolved_candidates'],0)
        self.assertEqual(self.store.db.execute("SELECT implementation_id FROM external_ids WHERE external_id='Q1194825'").fetchone()[0],'atlas:snes:000005')
        p=self.root/'one.db';q=self.root/'two.db'
        self.store.export(p);self.store.export(q)
        self.assertEqual(p.read_bytes(),q.read_bytes())
        with sqlite3.connect(f'file:{p}?mode=ro',uri=True) as db:
            self.assertEqual(db.execute('SELECT count(*) FROM profiles_experimental').fetchone()[0],21)
            self.assertEqual(db.execute('SELECT game_id FROM identity_aliases').fetchone()[0],'atlas:snes:000005')
            refs=' '.join(r[0] for r in db.execute("SELECT url FROM source_refs WHERE game_id='atlas:snes:000005'"))
            self.assertIn('gamespot',refs)
        self.assertEqual(self.store.import_records(atlas.load(ROOT/'assets/atlas/imports/wikidata-20261024.json')),0)
        self.assertEqual(self.store.stats()['implementations'],52)

    def test_wrong_platform_or_approval_rejected_atomically(self):
        before=self.store.stats()
        for change in [dict(platform='gba'),dict(human_review='invented approval')]:
            with self.assertRaises(ValueError):self.store.reconcile('atlas:snes:000053','atlas:snes:000005',dict(self.receipt,**change))
        self.assertEqual(self.store.stats(),before)
        self.assertEqual(self.store.db.execute('SELECT count(*) FROM identity_reconciliations').fetchone()[0],0)

    def test_conflicting_mapping_never_silently_replaces_receipt(self):
        self.store.reconcile('atlas:snes:000053','atlas:snes:000005',self.receipt)
        with self.assertRaisesRegex(ValueError,'Conflicting'):
            self.store.reconcile('atlas:snes:000053','atlas:snes:000005',dict(self.receipt,basis='Different assertion'))


if __name__=='__main__':unittest.main()
