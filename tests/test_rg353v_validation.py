"""Packaging/recovery tests only. No SSH, systemd mutation, device or emulator."""
import hashlib,importlib.util,json,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'));sys.path.insert(0,str(ROOT/'platform/darkos'))
import package_rg353v_validation as package
import phase12_preview as preview
import validation_preview as launcher

class RG353VValidation(unittest.TestCase):
    def test_all_twenty_eight_day_season_keys_exist(self):
        cases=package.scenarios()
        for season,date in package.SEASONS.items():
            for moment,hour in package.MOMENTS.items():
                self.assertEqual(cases[season+'-'+moment]['at'],date+'T'+hour)
                self.assertEqual(cases[season+'-'+moment]['weather'],'unknown')
        self.assertEqual(len(cases),36)
        self.assertEqual(cases['winter-snow-christmas']['at'],'2026-12-25T16:00')
        self.assertTrue(cases['unknown-clock']['unknown_time'])
        self.assertNotIn('at',cases['live'])

    def test_case_arguments_remain_trial_silent_and_restore_owned_es(self):
        args=preview.command('/tmp/owned','emotional-ui-phase12-preview-case',False,120,True,None,context='demo-winter-snow-christmas.toml',view='resume',isolated=True)
        self.assertIn('--trial',args);self.assertEqual(args[args.index('--audio-mode')+1],'silence')
        self.assertEqual(args[args.index('--view')+1],'resume')
        self.assertEqual(args[args.index('--config')+1],'/tmp/owned/demo-winter-snow-christmas.toml')
        self.assertIn('--property=ExecStopPost='+preview.RESTORE,args)
        self.assertIn('--property=RuntimeMaxSec=135',args)
        self.assertNotIn('--demo-history',args);self.assertNotIn('--launcher-config',args)
        self.assertTrue(args[args.index('--quit-file')+1].endswith('preview-case.quit.request'))

    def test_repeated_runs_never_reuse_exclusive_journal_or_quit_file(self):
        a=preview.command('/tmp/owned','emotional-ui-phase12-preview-one',False,isolated=True)
        b=preview.command('/tmp/owned','emotional-ui-phase12-preview-two',False,isolated=True)
        for flag in ['--smoke-log','--quit-file']:
            self.assertNotEqual(a[a.index(flag)+1],b[b.index(flag)+1])
        self.assertEqual(a[a.index('--db')+1],b[b.index('--db')+1]) # Own warm cache survives.

    def test_invalid_context_or_view_never_becomes_command(self):
        for name in ['../private.toml','demo-x%p.toml','demo-foo\n.toml','/etc/passwd']:
            with self.assertRaises(ValueError):preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,context=name)
        with self.assertRaises(ValueError):preview.command('/tmp/own','emotional-ui-phase12-preview-test',False,view='arbitrary')

    def fixture_package(self,root):
        binary=root/'fixture-elf';binary.write_bytes(b'own inert payload, never executed')
        profile=root/'owned-input.toml';profile.write_text('enabled=false\ndevice_name="retrogame_joypad"\n[mapping]\nbuttons=[]\naxes=[]\n')
        report=dict(format='ELF64',machine='AArch64',size_bytes=binary.stat().st_size,sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),needed=['libc.so.6'],glibc_versions=['GLIBC_2.36'],interpreter='/lib/ld-linux-aarch64.so.1',rpaths=[],device_compatibility='UNVERIFIED')
        folder=root/'bundle'
        with patch.object(package,'audit',return_value=report):package.build(binary,folder,root,profile)
        return folder

    def test_packaged_payload_hashes_and_archive_are_recoverable(self):
        import tarfile
        with tempfile.TemporaryDirectory() as t:
            root=Path(t);folder=self.fixture_package(root)
            manifest=preview.verify_bundle(folder)
            self.assertFalse(manifest['physical_verified']);self.assertFalse(manifest['public_redistribution'])
            for line in (folder/'SHA256SUMS').read_text().splitlines():
                digest,name=line.split('  ');self.assertEqual(hashlib.sha256((folder/name).read_bytes()).hexdigest(),digest)
            restore=root/'restore';restore.mkdir()
            with tarfile.open(folder.with_suffix('.tar.gz')) as tar:
                # Own archive only; reject traversal/symlinks before extraction.
                for item in tar.getmembers():
                    self.assertFalse(item.issym() or item.islnk());self.assertNotIn('..',Path(item.name).parts)
                tar.extractall(restore)
            self.assertEqual(preview.verify_bundle(restore/'bundle')['files'],manifest['files'])
            self.assertTrue((restore/'bundle/emotional-ui').stat().st_mode&0o111)

    def test_manifest_tamper_and_overwrite_are_rejected(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t);folder=self.fixture_package(root)
            (folder/'demo-live.toml').write_text('changed')
            with self.assertRaises(ValueError):preview.verify_bundle(folder)
            with self.assertRaises(ValueError):package.build(root/'fixture-elf',folder,root,root/'owned-input.toml')

    def test_launcher_requires_fresh_acknowledgements_before_execution(self):
        with tempfile.TemporaryDirectory() as t:
            folder=self.fixture_package(Path(t))
            with self.assertRaises(ValueError):launcher.invocation(folder,'live','emotional',120,execute=True)
            cmd=launcher.invocation(folder,'unknown-clock','resume',30)
            self.assertNotIn('--execute',cmd);self.assertIn('--unknown-time',cmd);self.assertIn('--input-profile',cmd)
            with self.assertRaises(ValueError):launcher.invocation(folder,'missing','resume',30)

    def test_package_does_not_install_system_or_download_weather(self):
        with tempfile.TemporaryDirectory() as t:
            folder=self.fixture_package(Path(t));names=[p.name for p in folder.iterdir()]
            self.assertNotIn('install.sh',names);self.assertNotIn('restore.sh',names)
            self.assertNotIn('--dev-season',(ROOT/'platform/darkos/validation_preview.py').read_text())
            config=(folder/'demo-live.toml').read_text();self.assertNotIn('hemisphere',config)
            self.assertNotIn('location',config);self.assertNotIn('at=',config)

if __name__=='__main__':unittest.main()
