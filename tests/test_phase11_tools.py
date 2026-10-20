import copy,hashlib,importlib.util,json,os,sqlite3,sys,tempfile,unittest
from pathlib import Path
from contextlib import closing
from unittest.mock import patch
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'tools'))
import atlas,build_atlas_experiment
spec=importlib.util.spec_from_file_location('distribution',ROOT/'platform/distribution/manage.py');distribution=importlib.util.module_from_spec(spec);spec.loader.exec_module(distribution)
class ExperimentalAtlas(unittest.TestCase):
 def setUp(self):self.d=atlas.load(ROOT/'assets/atlas/source/snes-experimental.json')
 def test_twenty_one_sourced_interpretations_no_human_approval(self):
  atlas.validate(self.d);self.assertEqual(len(self.d['experiment']['selected_ids']),21);self.assertIsNone(self.d['experiment']['human_review'])
  for g in self.d['games']:
   self.assertTrue(all(a['status']!='approved' for a in g['editorial'].values()))
   self.assertTrue(all(a.get('reviewer') is None for a in g['editorial'].values()))
   self.assertIsNone(g['objective']['duration'])
 def test_default_cannot_activate_proposals(self):
  for g in self.d['games']:self.assertEqual(atlas.profile(g)[1],{})
  self.assertEqual(sum(bool(atlas.profile(g,True)[1]) for g in self.d['games']),21)
 def test_missing_source_reason_evidence_or_forged_human_label_rejected(self):
  for mutation in ('facts','note','mechanics','human'):
   d=copy.deepcopy(self.d);g=d['games'][0]
   if mutation=='facts':d['sources'][g['editorial']['calm']['source']].pop('facts')
   elif mutation=='note':g['editorial']['calm'].pop('note')
   elif mutation=='mechanics':g['objective']['mechanics']=None
   else:d['experiment']['human_review']='automatic human review'
   with self.assertRaises(ValueError):atlas.validate(d)
 def test_experiment_id_and_explanation_bounds(self):
  for key,value in [('selected_ids',['absent']),('selected_ids',[self.d['games'][0]['id']]*2),('explanations',{})]:
   d=copy.deepcopy(self.d);d['experiment'][key]=value
   with self.assertRaises(ValueError):atlas.validate(d)
 def test_external_import_revokes_activation(self):
  d=atlas.import_results(self.d,[{'id':self.d['games'][0]['id'],'editorial':{'calm':dict(value=12,confidence=99,source='research-000001',note='unreviewed external proposal')}}])
  self.assertNotIn('experiment',d)
 def test_research_rebuild_and_sqlite_reproducible(self):
  with tempfile.TemporaryDirectory() as t:
   p=Path(t);build_atlas_experiment.build(p/'a.json');self.assertEqual(atlas.load(p/'a.json'),self.d)
   atlas.build(self.d,p/'a.db');atlas.build(self.d,p/'b.db');self.assertEqual((p/'a.db').read_bytes(),(p/'b.db').read_bytes())
   with closing(sqlite3.connect(p/'a.db')) as c:
    self.assertEqual(c.execute('SELECT count(*) FROM profiles_experimental').fetchone()[0],21)
    self.assertTrue(all(r[0]=='version=1\n' for r in c.execute('SELECT profile FROM profiles')))
 def test_work_target_must_exist(self):
  d=copy.deepcopy(self.d);d['games'][0]['identity']['work_id']='atlas:snes:999999'
  with self.assertRaises(ValueError):atlas.validate(d)
class Installer(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name).resolve();self.pkg=self.root/'package';self.pkg.mkdir();self.dest=self.root/'EmotionalUI'
  (self.pkg/'emotional-ui').write_bytes(b'owned fixture binary, not executed');(self.pkg/'config').mkdir();(self.pkg/'config/default.txt').write_text('owned defaults')
  self.files={p.relative_to(self.pkg).as_posix():distribution.digest(p) for p in self.pkg.rglob('*') if p.is_file()}
  (self.pkg/'package.json').write_text(json.dumps(dict(schema=1,kind='emotional-ui-trial',architecture='aarch64',main_install_enabled=False,files=self.files)))
 def install(self):return distribution.install(self.pkg,self.dest,True)
 def test_confirmation_and_first_install(self):
  with self.assertRaises(ValueError):distribution.install(self.pkg,self.dest)
  self.assertFalse(self.dest.exists());self.assertIn('own-files',self.install());self.assertEqual(distribution.read_state(self.dest)['system_changes'],[])
 def test_idempotent_no_duplicate_and_no_unknown_backup(self):
  self.install();before={p.relative_to(self.dest).as_posix():distribution.digest(p) for p in self.dest.rglob('*') if p.is_file()};self.assertEqual(self.install(),'already-installed');self.assertEqual(before,{p.relative_to(self.dest).as_posix():distribution.digest(p) for p in self.dest.rglob('*') if p.is_file()})
 def test_platform_wrong_and_main_disabled(self):
  with patch.object(distribution.platform,'system',return_value='Darwin'):
   with self.assertRaises(ValueError):distribution.compatible(self.pkg,distribution.manifest(self.pkg))
  self.assertFalse(self.dest.exists())
 def test_missing_dependency_fails_before_install(self):
  binary=bytearray(64);binary[:6]=b'\x7fELF\x02\x01';binary[18:20]=(183).to_bytes(2,'little');(self.pkg/'emotional-ui').write_bytes(binary)
  original=Path.read_bytes
  def read_bytes(path):return b'Anbernic RG353V\0' if str(path)=='/proc/device-tree/model' else original(path)
  with patch.object(distribution.platform,'system',return_value='Linux'),patch.object(distribution.platform,'machine',return_value='aarch64'),patch.object(Path,'is_file',lambda p:str(p)=='/proc/device-tree/model'),patch.object(Path,'read_bytes',read_bytes),patch.object(Path,'read_text',lambda p:'ID=darkos'):
   with self.assertRaisesRegex(ValueError,'glibc loader unavailable'):distribution.compatible(self.pkg,{})
  self.assertFalse(self.dest.exists())
 def test_similar_model_is_not_assumed_compatible(self):
  with patch.object(distribution.platform,'system',return_value='Linux'),patch.object(distribution.platform,'machine',return_value='aarch64'),patch.object(Path,'is_file',return_value=True),patch.object(Path,'read_bytes',return_value=b'Anbernic RG353VS\0'):
   with self.assertRaisesRegex(ValueError,'identity not established'):distribution.compatible(self.pkg,{})
 def test_recoverable_interrupted_manifest_resumes_only_verified_files(self):
  stage=self.dest.with_name(self.dest.name+'.installing');stage.mkdir()
  files=dict(self.files);files['package.json']=distribution.digest(self.pkg/'package.json')
  state=dict(schema=1,kind='emotional-ui-owned-files',destination=str(stage.resolve()),files=files,status='copying',system_changes=[],backup_required=False)
  distribution.write_state(stage,state);(stage/'emotional-ui').write_bytes((self.pkg/'emotional-ui').read_bytes())
  self.assertIn('installed-own-files',self.install());self.assertFalse(stage.exists())
 def test_disk_insufficient(self):
  with self.assertRaises(ValueError):distribution.install(self.pkg,self.dest,True,free_bytes=0)
  self.assertFalse(self.dest.exists())
 def test_restore_and_uninstall_preserve_personal_files(self):
  original=self.root/'gamelist.xml';original.write_text('not owned');saves=self.root/'save.sav';saves.write_bytes(b'keep');self.install();distribution.restore(self.dest);self.assertEqual(distribution.restore(self.dest),'already-restored');distribution.uninstall(self.dest);self.assertFalse(self.dest.exists());self.assertEqual(saves.read_bytes(),b'keep');self.assertEqual(original.read_text(),'not owned')
 def test_restore_required(self):
  self.install()
  with self.assertRaises(ValueError):distribution.uninstall(self.dest)
  self.assertTrue(self.dest.is_dir())
 def test_conflict_is_not_overwritten_or_removed(self):
  self.install();(self.dest/'config/default.txt').write_text('user changed this')
  for action in (self.install,lambda:distribution.restore(self.dest),lambda:distribution.uninstall(self.dest)):
   with self.assertRaises(ValueError):action()
  self.assertEqual((self.dest/'config/default.txt').read_text(),'user changed this')
 def test_unowned_file_blocks_cleanup(self):
  self.install();(self.dest/'mine.txt').write_text('keep')
  with self.assertRaises(ValueError):distribution.restore(self.dest)
  self.assertEqual((self.dest/'mine.txt').read_text(),'keep')
 def test_package_tampering_no_overwrite(self):
  (self.pkg/'emotional-ui').write_text('changed')
  with self.assertRaises(ValueError):self.install()
  self.assertFalse(self.dest.exists())
 def test_symlink_and_path_traversal_rejected(self):
  with self.assertRaises(ValueError):distribution.relative('../roms/game')
  self.dest.symlink_to(self.pkg,target_is_directory=True)
  with self.assertRaises(ValueError):self.install()
 def test_interrupted_copy_is_preserved_and_fails_closed(self):
  real=distribution.shutil.copyfileobj
  def interrupted(src,dst):dst.write(b'partial');raise OSError('simulated power failure')
  with patch.object(distribution.shutil,'copyfileobj',side_effect=interrupted):
   with self.assertRaises(OSError):self.install()
  stage=self.dest.with_name(self.dest.name+'.installing');self.assertTrue(stage.is_dir());self.assertFalse(self.dest.exists())
  with self.assertRaises(ValueError):self.install()
  self.assertTrue(stage.is_dir())
 def test_trial_is_manual_and_disabled_after_restore(self):
  self.install()
  with self.assertRaises(ValueError):distribution.trial(self.dest)
  distribution.restore(self.dest)
  with self.assertRaises(ValueError):distribution.trial(self.dest,display_released=True,recovery=True)
 def test_foreign_installation_untouched(self):
  self.dest.mkdir();(self.dest/'original.txt').write_text('keep')
  with self.assertRaises(OSError):self.install()
  self.assertEqual((self.dest/'original.txt').read_text(),'keep')
if __name__=='__main__':unittest.main()
