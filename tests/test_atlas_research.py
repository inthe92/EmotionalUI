import copy
import hashlib
import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import atlas
import atlas_research as research
import build_atlas_experiment


class EvidenceBatches(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base = atlas.load(ROOT / 'assets/atlas/source/snes-experimental.json')
        cls.batches = [atlas.load(p) for p in sorted((ROOT / 'assets/atlas/research/batches').glob('*.json'))]
        cls.enriched = copy.deepcopy(cls.base)
        for b in cls.batches:
            cls.enriched = research.apply_batch(cls.enriched, b)

    def test_twenty_one_covered_and_thirty_one_explicit_gaps(self):
        c = research.coverage(self.enriched)
        self.assertEqual((c['total'], c['enriched']), (52, 21))
        self.assertEqual(c['with_two_independent_editorial_groups'], 3)
        self.assertTrue(all(r['gaps'] and r['unknowns'] for r in c['games']))
        self.assertEqual(len(self.enriched['research']['source_ids']), 30)
        self.assertEqual(len(self.enriched['research']['claims']), 66)

    def test_no_review_or_personal_nostalgia_manufactured(self):
        self.assertIsNone(self.enriched['experiment']['human_review'])
        confidence = set()
        for g in self.enriched['games']:
            if g['id'] not in self.enriched['research']['records']:
                continue
            self.assertNotIn('nostalgia', g['editorial'])
            self.assertNotIn('session_minutes', g['editorial'])
            self.assertIsNone(g['objective']['duration'])
            for a in g['editorial'].values():
                self.assertEqual(a['status'], 'proposed')
                self.assertNotIn('reviewer', a)
                self.assertTrue(a['evidence'] and a['note'])
                confidence.add(a['confidence'])
        self.assertEqual(confidence, {60, 65, 70})

    def test_batch_resume_is_idempotent_and_preserves_input(self):
        before = copy.deepcopy(self.base)
        once = research.apply_batch(self.base, self.batches[0])
        self.assertEqual(once, research.apply_batch(once, self.batches[0]))
        self.assertEqual(self.base, before)

    def test_changed_receipt_rejected(self):
        b = copy.deepcopy(self.batches[0]); b['profiles'][0]['explanation'] += '!'
        with self.assertRaisesRegex(ValueError, 'batch changed'):
            research.apply_batch(self.enriched, b)

    def test_duplicate_profile_or_unknown_work_rejected(self):
        for mutation in ('duplicate', 'unknown'):
            b = copy.deepcopy(self.batches[0])
            if mutation == 'duplicate': b['profiles'].append(copy.deepcopy(b['profiles'][0]))
            else: b['profiles'][0]['id'] = 'atlas:snes:999999'
            with self.assertRaises(ValueError): research.apply_batch(self.base, b)

    def test_applied_source_conflict_rejected(self):
        b = copy.deepcopy(self.batches[0]); b['batch_id'] = 'different'
        b['sources']['review-000001']['author'] = 'another attribution'
        with self.assertRaisesRegex(ValueError, 'Source conflict'):
            research.apply_batch(self.enriched, b)

    def test_no_overwriting_approved_metadata(self):
        d = copy.deepcopy(self.base)
        d['games'][0]['editorial']['calm'].update(status='approved', reviewer='synthetic fixture only')
        with self.assertRaisesRegex(ValueError, 'approved'):
            research.apply_batch(d, self.batches[0])

    def test_dates_metadata_private_urls_and_cross_platform_rejected(self):
        for key, value in [('platform', 'gba'), ('accessed', None), ('published', '2026-12-31'),
                           ('url', 'https://user:secret@example.com/page'), ('title', '')]:
            b = copy.deepcopy(self.batches[0]); b['sources']['review-000001'][key] = value
            with self.assertRaises(ValueError): research.apply_batch(self.base, b)

    def test_duplicate_canonical_article_is_not_an_independent_source(self):
        b = copy.deepcopy(self.batches[0])
        other = copy.deepcopy(b['sources']['review-000001'])
        other['url'] += '/'
        b['sources']['copied-review'] = other
        with self.assertRaisesRegex(ValueError, 'Duplicate source'):
            research.apply_batch(self.base, b)

    def test_unknown_citation_and_cross_work_claim_rejected(self):
        for mode in ('missing', 'different-work'):
            b = copy.deepcopy(self.batches[0])
            cid = 'missing' if mode == 'missing' else 'snes-000002-mechanics'
            b['profiles'][0]['attributes']['exploration']['evidence'] = [cid]
            with self.assertRaises(ValueError): research.apply_batch(self.base, b)

    def test_disputed_claim_cannot_feed_numeric_profile(self):
        b = copy.deepcopy(self.batches[0]); b['claims']['snes-000001-experience']['status'] = 'disputed'
        with self.assertRaisesRegex(ValueError, 'Disputed'):
            research.apply_batch(self.base, b)

    def test_syndication_cannot_increase_confidence(self):
        b = copy.deepcopy(self.batches[0])
        b['sources']['gamespot-metroid']['independence_key'] = 'hookshot-editorial'
        # exploration is supported by Nintendo; tension relies on two editorial voices.
        with self.assertRaisesRegex(ValueError, 'Corroboration'):
            research.apply_batch(self.base, b)

    def test_player_comment_not_a_fact_or_automatic_consensus(self):
        b = copy.deepcopy(self.batches[0]); b['claims']['player-metroid-walljump']['kind'] = 'fact'
        with self.assertRaisesRegex(ValueError, 'player opinion'):
            research.apply_batch(self.base, b)
        self.assertEqual(self.enriched['research']['claims']['player-harvest-routine']['supports'], [])
        self.assertIn('player-harvest-routine', self.enriched['research']['claims']['snes-000047-experience']['contrasts'])

    def test_unsupported_scores_or_missing_evidence_rejected(self):
        for mode in ('nostalgia', 'precision', 'missing'):
            b = copy.deepcopy(self.batches[0]); attrs = b['profiles'][0]['attributes']
            if mode == 'nostalgia': attrs['nostalgia'] = copy.deepcopy(attrs['exploration'])
            elif mode == 'precision': attrs['exploration']['value'] = 77
            else: attrs['exploration'].pop('evidence')
            with self.assertRaises(ValueError): research.apply_batch(self.base, b)

    def test_unknown_research_and_batch_versions_rejected(self):
        d = copy.deepcopy(self.enriched); d['research']['schema'] = 2
        with self.assertRaises(ValueError): atlas.validate(d)
        b = copy.deepcopy(self.batches[0]); b['version'] = '1.0.0'
        with self.assertRaises(ValueError): research.apply_batch(self.base, b)

    def test_dangling_registry_rejected(self):
        d = copy.deepcopy(self.enriched); d.pop('research')
        with self.assertRaisesRegex(ValueError, 'registry'): atlas.validate(d)

    def test_json_duplicate_keys_and_missing_batch_directory_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'duplicate.json'
            p.write_text('{"source":"first", "source":"second"}')
            with self.assertRaisesRegex(ValueError, 'Duplicate JSON key'): atlas.load(p)
            with self.assertRaisesRegex(ValueError, 'no batches'):
                build_atlas_experiment.build(Path(tmp) / 'out.json', Path(tmp) / 'missing')
            self.assertFalse((Path(tmp) / 'out.json').exists())

    def test_rebuild_reproducible_readonly_and_channels_isolated(self):
        protected = [ROOT / 'assets/atlas' / p for p in ('emotional-atlas.db', 'emotional-atlas-0.1.0-pilot.db', 'emotional-atlas-0.2.0-experimental.1.db')]
        before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in protected}
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            build_atlas_experiment.build(out / 'source.json', ROOT / 'assets/atlas/research/batches')
            self.assertEqual(atlas.load(out / 'source.json'), self.enriched)
            atlas.build(self.enriched, out / 'one.db'); atlas.build(self.enriched, out / 'two.db')
            self.assertEqual((out / 'one.db').read_bytes(), (out / 'two.db').read_bytes())
            with sqlite3.connect((out / 'one.db').as_uri() + '?mode=ro', uri=True) as db:
                self.assertEqual(db.execute('PRAGMA user_version').fetchone()[0], 1)
                self.assertEqual(db.execute('SELECT count(*) FROM profiles_experimental').fetchone()[0], 21)
                self.assertTrue(all(r[0] == 'version=1\n' for r in db.execute('SELECT profile FROM profiles')))
                self.assertEqual(db.execute('SELECT count(*) FROM research_sources').fetchone()[0], 30)
                self.assertEqual(db.execute('SELECT count(*) FROM research_claims').fetchone()[0], 66)
                with self.assertRaises(sqlite3.OperationalError): db.execute('DELETE FROM profiles')
            with self.assertRaises(ValueError): atlas.build(self.enriched, out / 'one.db')
        self.assertEqual(before, {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in protected})


if __name__ == '__main__':
    unittest.main()
