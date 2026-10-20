import copy,hashlib,json,sqlite3,sys,tempfile,unittest
from pathlib import Path
from contextlib import closing
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'tools'))
import atlas
class Atlas(unittest.TestCase):
 def setUp(self):self.d=atlas.load(ROOT/'assets/atlas/source/snes-pilot.json')
 def test_pilot_valid_and_unknowns_stay_unknown(self):
  atlas.validate(self.d);self.assertEqual(len(self.d['games']),52)
  self.assertTrue(all(g['objective']['duration'] is None for g in self.d['games']))
  self.assertTrue(all('calm=' not in atlas.profile(g)[0] for g in self.d['games']))
 def test_schema_versions_and_missing_fields(self):
  for field in ('schema','version','games'):
   d=copy.deepcopy(self.d);d.pop(field)
   with self.assertRaises((ValueError,KeyError)):atlas.validate(d)
 def test_missing_entry_fields_and_platform_contradiction(self):
  d=copy.deepcopy(self.d);d['games'][0]['objective'].pop('year')
  with self.assertRaises(ValueError):atlas.validate(d)
  d=copy.deepcopy(self.d);d['games'][0]['objective']['platform']['value']='nes'
  with self.assertRaises(ValueError):atlas.validate(d)
 def test_duplicate_id_identity_and_invalid_relation(self):
  d=copy.deepcopy(self.d);d['games'].append(copy.deepcopy(d['games'][0]))
  with self.assertRaises(ValueError):atlas.validate(d)
  d['games'][-1]['id']='atlas:snes:009999'
  with self.assertRaises(ValueError):atlas.validate(d)
  d=copy.deepcopy(self.d);d['relations'][0]['to']='absent'
  with self.assertRaises(ValueError):atlas.validate(d)
 def test_bad_values_confidence_and_provenance(self):
  for key,value in [('value',101),('confidence',101),('source','absent'),('status','approved')]:
   d=copy.deepcopy(self.d);d['games'][0]['editorial']['intensity'][key]=value
   with self.assertRaises(ValueError):atlas.validate(d)
 def test_ai_import_cannot_self_approve_or_overwrite_review(self):
  a=dict(value=45,confidence=75,source='pilot-editorial-proposals',status='approved',reviewer='fake')
  d=atlas.import_results(self.d,[dict(id=self.d['games'][0]['id'],editorial={'calm':a})]);self.assertEqual(d['games'][0]['editorial']['calm']['status'],'proposed');self.assertNotIn('reviewer',d['games'][0]['editorial']['calm'])
  d['games'][0]['editorial']['calm'].update(status='approved',reviewer='synthetic-test-review')
  with self.assertRaises(ValueError):atlas.import_results(d,[dict(id=d['games'][0]['id'],editorial={'calm':a})])
 def test_unknown_ai_value_is_not_zero(self):
  d=atlas.import_results(self.d,[dict(id=self.d['games'][0]['id'],editorial={'calm':dict(value=None,confidence=0,source='pilot-editorial-proposals')})]);self.assertIsNone(d['games'][0]['editorial']['calm']['value']);self.assertNotIn('calm=',atlas.profile(d['games'][0])[0])
 def test_reproducible_sqlite_and_no_overwrite(self):
  with tempfile.TemporaryDirectory() as t:
   a=Path(t)/'a.db';b=Path(t)/'b.db';atlas.build(self.d,a);atlas.build(self.d,b);self.assertEqual(a.read_bytes(),b.read_bytes())
   with self.assertRaises(ValueError):atlas.build(self.d,a)
   with closing(sqlite3.connect(f'file:{a}?mode=ro',uri=True)) as db:
    self.assertEqual(db.execute('SELECT count(*) FROM games').fetchone()[0],52);self.assertEqual(db.execute('PRAGMA user_version').fetchone()[0],1)
    self.assertEqual(db.execute('SELECT count(*) FROM candidates').fetchone()[0],52*11)
    with self.assertRaises(sqlite3.OperationalError):db.execute("DELETE FROM games")
 def test_region_revision_translation_are_explicit_and_not_fuzzy(self):
  g=copy.deepcopy(self.d['games'][0]);g['id']='atlas:snes:999999';g['identity']['region']='JP';self.d['games'].append(g);self.assertEqual(len(atlas.resolve(self.d,g['objective']['title']['value'])),2);self.assertFalse(atlas.resolve(self.d,'Super Mario World (USA) [hack]'))
 def test_requests_do_not_invent_provider_responses(self):
  r=atlas.requests(self.d);self.assertEqual(len(r),52);self.assertIn('proposal',r[0]['instruction']);self.assertNotIn('editorial',r[0])
