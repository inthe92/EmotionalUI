"""Original resources and reproducibility; no physical sound/display claims."""
from pathlib import Path
import array,hashlib,json,subprocess,sys,tempfile,unittest,wave
from audio_fixtures import original_study,original_exports
ROOT=Path(__file__).resolve().parent.parent
class AudioAssets(unittest.TestCase):
    def test_original_score_is_small_spaced_and_reproducible(self):
        score=json.loads((ROOT/'assets/audio/source/score.json').read_text())
        for motif in score['motifs']:
            self.assertTrue(2<=len(motif['notes'])<=5)
            self.assertTrue(all(0<=n[1]<=127 and n[2]<=45 for n in motif['notes']))
            self.assertEqual(sorted(n[0] for n in motif['notes']),[n[0] for n in motif['notes']])
        temporary=original_study()
        original=json.loads((ROOT/'assets/audio/source/render-manifest.json').read_text())
        for record in original['resources']:
            data=(Path(temporary)/record['file']).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(),record['sha256'])
        self.assertEqual((Path(temporary)/'assets/audio/source/la-habitacion-permanece.mid').read_bytes(),(ROOT/'assets/audio/source/la-habitacion-permanece.mid').read_bytes())
    def test_pcm_bounded_and_preview_contains_real_silence(self):
        total=0
        for path in (ROOT/'assets/audio/runtime').glob('*.wav'):
            with wave.open(str(path)) as f:
                self.assertEqual((f.getnchannels(),f.getsampwidth(),f.getframerate()),(1,2,22050));total+=f.getnframes()*2
        self.assertLess(total,4*1024*1024)
        with wave.open(str(original_study()/'assets/audio/previews/la-habitacion-permanece.wav')) as f:
            self.assertEqual(f.getnframes()/f.getframerate(),100)
            samples=array.array('h',f.readframes(f.getnframes()))
        self.assertGreater(sum(v==0 for v in samples)/len(samples),.5)
        self.assertGreater(max(samples),1000);self.assertLess(max(map(abs,samples)),10000)
    def test_export_is_ogg_vorbis_and_midi_is_actual_smf(self):
        ogg=(original_exports()/'la-habitacion-permanece.ogg').read_bytes()
        self.assertEqual(ogg[:4],b'OggS');self.assertIn(b'\x01vorbis',ogg)
        midi=(ROOT/'assets/audio/source/la-habitacion-permanece.mid').read_bytes()
        self.assertEqual(midi[:4],b'MThd');self.assertEqual(midi[14:18],b'MTrk');self.assertIn(b'\xff\x2f\x00',midi)
if __name__=='__main__':unittest.main()
