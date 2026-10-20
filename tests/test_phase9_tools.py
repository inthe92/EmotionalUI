import hashlib,json,sys,unittest,wave,struct
from pathlib import Path
from audio_fixtures import sampled_exports,original_study
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'tools'))
import produce_music
class Music(unittest.TestCase):
 def test_real_sample_provenance_and_editable_performance(self):
  base=ROOT/'assets/audio/phase9';score=json.loads((base/'source/score.json').read_text());lock=json.loads((base/'instrument/sources.lock.json').read_text())
  self.assertEqual(lock['license'],'CC0-1.0');self.assertIn('CC0 1.0 Universal',(base/'instrument/LICENSE').read_text());self.assertEqual(len(lock['files']),11)
  self.assertNotEqual(score['variants']['refugio'],score['variants']['silencio'])
  for name,v in score['variants'].items():
   self.assertEqual(produce_music.midi(v),(base/'candidates'/name/'source.mid').read_bytes())
   for motif in v['motifs']:
    self.assertTrue(3<=len(motif['notes'])<=5);self.assertGreater(len({n[2] for n in motif['notes']}),2);self.assertGreater(len({n[3] for n in motif['notes']}),2)
 def test_actual_pcm_no_clipping_and_long_silences(self):
  base=sampled_exports();manifest=json.loads((ROOT/'assets/audio/phase9/candidates/render-manifest.json').read_text())
  for r in manifest['resources']:
   p=base/r['file'];self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),r['sha256'])
   with wave.open(str(p)) as f:
    self.assertEqual((f.getnchannels(),f.getsampwidth(),f.getframerate()),(1,2,22050));b=f.readframes(f.getnframes());samples=struct.unpack('<'+'h'*(len(b)//2),b)
   self.assertGreater(max(abs(v) for v in samples),100);self.assertLess(max(abs(v) for v in samples),20000)
   if '/runtime/' not in r['file']:self.assertGreater(r['zero_percent'],50)
 def test_exports_exist_and_older_studies_remain(self):
  for name in ('refugio','silencio'):
   base=sampled_exports()/name;self.assertTrue((base/(name+'.ogg')).read_bytes().startswith(b'OggS'));self.assertIn(b'ftyp',(base/(name+'.m4a')).read_bytes()[:32]);self.assertTrue((base/'source.mid').read_bytes().startswith(b'MThd'))
  self.assertTrue((original_study()/'assets/audio/previews/la-habitacion-permanece.wav').is_file())
