import importlib.util
import json
import os
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'platform/distribution'))
sys.path.insert(0, str(ROOT/'platform/darkos'))
import manage
import menu

spec = importlib.util.spec_from_file_location('package_preview', ROOT/'tools/package_preview.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class Preview(unittest.TestCase):
    def test_observed_debian_darkos_identity_without_generic_debian_guess(self):
        self.assertTrue(manage.darkos_identity('ID=debian', 'title=dArkOS (08272026)\n', 'RG353V\n'))
        self.assertTrue(manage.darkos_identity('ID="darkos"\n'))
        for splash, device in [('', 'RG353V'), ('title=ArkOS (08272026)', 'RG353V'),
                               ('title=dArkOS (08272026)', 'RG353VS')]:
            self.assertFalse(manage.darkos_identity('ID=debian', splash, device))

    def test_graphical_entry_rejects_private_bundle_before_copy(self):
        with patch.object(manage, 'manifest', return_value={'public_release': False}), \
             patch.object(manage, 'install') as install:
            with self.assertRaisesRegex(ValueError, 'not cleared'):
                menu.prepare(Path('/package'), Path('/destination'))
            install.assert_not_called()

    def test_recovery_in_progress_blocks_other_entry_actions(self):
        import subprocess
        with patch.object(subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, 'trial activating', '')) as run:
            with self.assertRaises(ValueError):
                menu.check_no_session()
            self.assertIn('--state=active,activating,deactivating,reloading', run.call_args.args[0])

    def test_recovery_is_external_software_default_and_trial_only(self):
        with patch.object(os, 'getuid', return_value=1000), patch.dict(os.environ,
                {'EMOTIONAL_GPU_SWAP': 'immediate', 'EMOTIONAL_GPU_REPLAY': 'fixed-v1'}):
            args = menu.session_command(Path('/home/ark/app'), Path('/home/ark/data'),
                                        Path('/home/ark/data/session'), Path('/roms2'), token='a'*32)
        self.assertIn('--property=ExecStopPost=+/usr/bin/systemctl start emulationstation.service', args)
        self.assertIn('--property=RuntimeMaxSec=900', args)
        self.assertIn('--trial', args)
        env = next(a for a in args if a.startswith('--property=Environment='))
        self.assertIn('SLINT_BACKEND=linuxkms-software', env)
        self.assertNotIn('EMOTIONAL_GPU_SWAP=', env)
        self.assertNotIn('EMOTIONAL_GPU_REPLAY=', env)
        self.assertTrue(any(a.startswith('--property=UnsetEnvironment=EMOTIONAL_GPU_SWAP ') for a in args))
        self.assertFalse(any('enable' in a or '--launch' in a for a in args))

    def test_gpu_is_explicit_and_recovers_without_gpu_process(self):
        with patch.object(os, 'getuid', return_value=1000):
            args = menu.session_command(Path('/home/ark/app'), Path('/home/ark/data'),
                    Path('/home/ark/data/session'), Path('/roms2'), 'femtovg', 'a'*32)
        self.assertIn('/home/ark/app/emotional-ui-gpu', args)
        self.assertTrue(any('SLINT_BACKEND=linuxkms-femtovg' in a for a in args))
        self.assertTrue(any('ExecStopPost=' in a and 'emotional-ui-gpu' not in a for a in args))

    def test_refuse_root_and_systemd_specifiers(self):
        with patch.object(os, 'getuid', return_value=0), self.assertRaisesRegex(ValueError, 'root'):
            menu.session_command(Path('/home/ark/app'), Path('/home/ark/data'), Path('/home/ark/run'), Path('/roms'))
        with self.assertRaises(ValueError):
            menu.session_command(Path('/home/ark/%n'), Path('/home/ark/data'), Path('/home/ark/run'), Path('/roms'))

    def test_menu_ancestry_exception_does_not_allow_another_emulator(self):
        text = '10 1 bash\n11 10 emulationstatio\n12 11 nice\n13 12 sh\n14 13 python3\n15 14 ps\n'
        menu.check_menu_tree(text, 10, 14)
        with self.assertRaises(ValueError):
            menu.check_menu_tree(text+'20 11 retroarch\n', 10, 14)
        with self.assertRaises(ValueError):
            menu.check_menu_tree(text+'21 11 python3\n', 10, 14)
        with self.assertRaises(ValueError):
            menu.check_menu_tree(text+'22 14 retroarch\n', 10, 14)
        with self.assertRaisesRegex(ValueError, 'menu'):
            menu.check_menu_tree(text, 10, 999)

    def fixture(self, root):
        # Synthetic package tests never execute these header-only ELF fixtures.
        names = ['Cargo.toml', 'vendor/backend.rs', 'platform/distribution/manage.py',
                 'platform/distribution/menu.py', 'platform/distribution/rg353v-input.toml',
                 'platform/darkos/phase12_preview.py', 'LICENSE', 'THIRD_PARTY_LICENSES.md',
                 'AUDIO_ASSET_PROVENANCE.md', 'assets/fonts/DejaVu-COPYRIGHT.txt',
                 'docs/dependency-notices.txt', 'PREVIEW_INSTALL.md']
        for n in names:
            p = root/n
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text('original public fixture: '+n+'\n')
        plan = dict(version=package.VERSION,
                    clearance={k: 'cleared' for k in ('room_redistribution', 'dependency_notices', 'public_source_privacy')},
                    clearance_evidence={k: 'synthetic fixture, no real permission' for k in ('room_redistribution', 'dependency_notices', 'public_source_privacy')},
                    source_files={n: package.sha(root/n) for n in names})
        # Unlisted data must never leak into the corresponding-source archive.
        (root/'private.env').write_text('PRIVATE FIXTURE MUST NEVER BE COPIED')
        snapshot = package.review(plan, root)
        binaries = []
        for n in ('cpu', 'gpu'):
            p = root/n
            data = bytearray(64)
            data[:6] = b'\x7fELF\x02\x01'
            data[18:20] = (183).to_bytes(2, 'little')
            p.write_bytes(data)
            binaries.append(p)
        receipt = dict(source_snapshot_sha256=snapshot,
                       binaries=dict(software=package.sha(binaries[0]), femtovg=package.sha(binaries[1])))
        return plan, receipt, binaries

    def test_pending_rights_or_notices_cannot_generate_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            plan, receipt, binaries = self.fixture(root)
            for gate in plan['clearance']:
                plan['clearance'][gate] = 'pending'
                with self.assertRaisesRegex(ValueError, 'blocked'):
                    package.build(plan, receipt, *binaries, root/'out.zip', root)
                self.assertFalse((root/'out.zip').exists())
                plan['clearance'][gate] = 'cleared'

    def test_receipt_mismatch_refuses_before_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            plan, receipt, binaries = self.fixture(root)
            receipt['binaries']['femtovg'] = '0'*64
            with self.assertRaisesRegex(ValueError, 'receipt'):
                package.build(plan, receipt, *binaries, root/'out.zip', root)
            self.assertFalse((root/'out.zip').exists())

    def test_exact_zip_extraction_hashes_permissions_and_owned_lifecycle(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            plan, receipt, binaries = self.fixture(root)
            report = package.build(plan, receipt, *binaries, root/'out.zip', root)
            self.assertEqual(report['sha256'], package.sha(root/'out.zip'))
            extracted = root/'extracted'
            with zipfile.ZipFile(root/'out.zip') as z:
                self.assertIsNone(z.testzip())
                for entry in package.ENTRIES:
                    self.assertTrue((z.getinfo(entry).external_attr >> 16) & 0o111)
                self.assertTrue((z.getinfo(package.PAYLOAD+'/emotional-ui').external_attr >> 16) & 0o111)
                z.extractall(extracted)
            payload = extracted/package.PAYLOAD
            manifest = manage.manifest(payload)
            self.assertEqual(manifest['default_renderer'], 'software')
            self.assertFalse(manifest['main_install_enabled'])
            with tarfile.open(payload/'corresponding-source.tar.gz') as tar:
                self.assertEqual(set(tar.getnames()), {'emotional-ui/'+n for n in plan['source_files']})
                self.assertNotIn('emotional-ui/private.env', tar.getnames())
            destination = root/'installed'
            manage.install(payload, destination, True)
            self.assertTrue((destination/'emotional-ui').stat().st_mode & 0o111)
            self.assertTrue((destination/'emotional-ui-gpu').stat().st_mode & 0o111)
            self.assertIn('restored', manage.restore(destination))
            manage.uninstall(destination)
            self.assertFalse(destination.exists())
            self.assertTrue((root/'private.env').is_file())

    def test_source_change_or_symlink_cannot_enter_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            plan, _, _ = self.fixture(root)
            (root/'Cargo.toml').write_text('changed')
            with self.assertRaisesRegex(ValueError, 'Changed'):
                package.review(plan, root)
            (root/'Cargo.toml').unlink()
            (root/'Cargo.toml').symlink_to(root/'private.env')
            with self.assertRaises(ValueError):
                package.review(plan, root)


if __name__ == '__main__':
    unittest.main()
