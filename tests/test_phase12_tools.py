import hashlib,importlib.util,json,tempfile,unittest
import array,wave
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location('preview',ROOT/'platform/darkos/phase12_preview.py');preview=importlib.util.module_from_spec(spec);spec.loader.exec_module(preview)
spec=importlib.util.spec_from_file_location('calibration',ROOT/'platform/darkos/calibrate_audio.py');calibration=importlib.util.module_from_spec(spec);spec.loader.exec_module(calibration)
class PhysicalPreview(unittest.TestCase):
 def test_recovery_probe_never_stops_frontend(self):
  args=preview.command('/tmp/own','emotional-ui-phase12-probe-test',True)
  self.assertEqual(args[-1],'/usr/bin/true');self.assertFalse(any('ExecStartPre' in a for a in args));self.assertIn('--property=ExecStopPost='+preview.RESTORE,args)
 def test_preview_cannot_launch_game_or_play_sound(self):
  args=preview.command('/tmp/own','emotional-ui-phase12-preview-test',False)
  self.assertIn('--trial',args);self.assertNotIn('--launcher-config',args);self.assertEqual(args[args.index('--audio-mode')+1],'silence');self.assertEqual(args[args.index('--library-root')+1],'/tmp/own/empty');self.assertIn('--property=Restart=no',args)
 def test_motion_and_source_write_protection_in_preview(self):
  args=preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,30,True,'/roms2','ambient',18)
  self.assertNotIn('--reduced-motion',args);self.assertIn('--property=ReadOnlyPaths=/roms2',args);self.assertIn('--input-profile',args)
 def test_long_preview_keeps_trial_guard_and_independent_cap(self):
  args=preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,120)
  self.assertIn('--trial',args);self.assertIn('--property=RuntimeMaxSec=135',args)
  self.assertIn('--quit-file',args);self.assertNotIn('--smoke-test',args);self.assertNotIn('--launcher-config',args)
 def test_high_audio_volume_is_rejected(self):
  with self.assertRaises(ValueError):preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,audio_mode='ambient',audio_volume=100)
 def test_calibrated_bank_is_explicit_and_cannot_change_trial_guard(self):
  default=preview.command('/tmp/own','emotional-ui-phase12-preview-test',False)
  self.assertNotIn('--audio-assets',default)
  args=preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,calibrated_audio=True)
  self.assertEqual(args[args.index('--audio-assets')+1],'/tmp/own');self.assertIn('--trial',args)
 def test_calibrated_bank_requires_all_files_in_checked_manifest(self):
  with self.assertRaises(ValueError):preview.check_audio_bank({'files':{'umbral.wav':'digest'}})
  preview.check_audio_bank({'files':{name+'.wav':'digest' for name in ('umbral','brasa','rastro','rain','air')}})
 def test_systemd_injection_and_unit_rejected(self):
  for path in ['/tmp/%n','/tmp/a\nExecStart=x','relative','/tmp/../etc']:
   with self.assertRaises(ValueError):preview.command(path,'emotional-ui-phase12-preview-test',False)
  with self.assertRaises(ValueError):preview.command('/tmp/own','emulationstation.service',False)
 def test_dark_or_unknown_backlight_prevents_handoff(self):
  for values in [[],[0],[0,0]]:
   with self.assertRaises(ValueError):preview.check_backlight(values)
  preview.check_backlight([129])
 def test_game_and_unknown_children_cancel_handoff(self):
  preview.check_tree('10 1 emulationstatio\n11 10 emulationstatio\n',10)
  for tree in ['10 1 emulationstatio\n11 10 retroarch\n','10 1 emulationstatio\n11 10 custom-game\n','20 1 emulationstatio\n','10 1 emulationstatio\n12 1 retroarch\n']:
   with self.assertRaises(ValueError):preview.check_tree(tree,10)
 def test_payload_tamper_and_symlink_rejected(self):
  with tempfile.TemporaryDirectory() as t:
   p=Path(t);binary=p/'emotional-ui';binary.write_bytes(b'owned fixture, never executed')
   (p/'phase12-manifest.json').write_text(json.dumps(dict(kind='phase12-private-preview',files={'emotional-ui':hashlib.sha256(binary.read_bytes()).hexdigest()})))
   preview.verify_bundle(p);binary.write_bytes(b'changed')
   with self.assertRaises(ValueError):preview.verify_bundle(p)
   link=p/'alias';link.symlink_to(p,target_is_directory=True)
   with self.assertRaises(ValueError):preview.verify_bundle(link)
 def test_calibration_preserves_family_dynamics_sources_and_ambience(self):
  with tempfile.TemporaryDirectory() as t:
   root=Path(t);source=root/'source';source.mkdir()
   for index,name in enumerate(calibration.PIANO+calibration.AMBIENCE):
    with wave.open(str(source/(name+'.wav')),'wb') as w:
     w.setparams((1,2,22050,0,'NONE','not compressed'));w.writeframes(array.array('h',[0,1000*(index+1),-1000*(index+1),0]).tobytes())
   before={p.name:p.read_bytes() for p in source.iterdir()};out=root/'output'
   report=calibration.calibrate(source,out)
   self.assertEqual(before,{p.name:p.read_bytes() for p in source.iterdir()})
   peaks=[]
   for name in calibration.PIANO:
    with wave.open(str(out/(name+'.wav')),'rb') as w:peaks.append(max(map(abs,array.array('h',w.readframes(w.getnframes())))))
   self.assertLessEqual(abs(peaks[0]*3-peaks[2]),2);self.assertLessEqual(max(peaks),26214)
   for name in calibration.AMBIENCE:self.assertEqual((out/(name+'.wav')).read_bytes(),before[name+'.wav'])
   self.assertEqual(report['application_master_limit'],25)
   with self.assertRaises(FileExistsError):calibration.calibrate(source,out)
 def test_calibration_rejects_silent_bank_without_creating_output(self):
  with tempfile.TemporaryDirectory() as t:
   root=Path(t)
   for name in calibration.PIANO+calibration.AMBIENCE:
    with wave.open(str(root/(name+'.wav')),'wb') as w:
     w.setparams((1,2,22050,0,'NONE','not compressed'));w.writeframes(bytes(8))
   with self.assertRaises(ValueError):calibration.calibrate(root,root/'output')
   self.assertFalse((root/'output').exists())
if __name__=='__main__':unittest.main()
