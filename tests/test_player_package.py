"""Regression for a delivered launcher missing its transitive menu module."""
from pathlib import Path
import importlib.util
import shutil
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('player_package',ROOT/'tools/build_player_package.py')
package=importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PlayerPackage(unittest.TestCase):
    def test_entrypoints_import_only_from_payload_and_missing_menu_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            payload=Path(tmp)
            files=[ROOT/'platform/distribution'/name for name in ('manage.py','menu.py')]
            files += [ROOT/'platform/darkos'/name for name in ('phase12_preview.py','functional_preview.py','functional_menu.py','player_menu.py','player_bootstrap.py','rom_drop.py')]
            files += [ROOT/'tools/darkos_launcher_profiles.py',ROOT/'platform/darkos/steamgriddb_rpc.py']
            for source in files:shutil.copyfile(source,payload/source.name)
            shutil.copyfile(ROOT/'tools/steamgriddb_artwork.py',payload/'steamgriddb_client.py')
            package.verify_entrypoint_imports(payload)
            (payload/'menu.py').unlink()
            with self.assertRaisesRegex(RuntimeError,'Incomplete player package'):
                package.verify_entrypoint_imports(payload)

    def test_private_candidate_has_no_public_release_claim(self):
        import json
        import zipfile
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); binary=root/'candidate';binary.write_bytes(b'synthetic candidate, not a console binary')
            result=package.build(binary,root/'output',private=True)
            bundle=Path(result['bundle'])
            manifest=json.loads((bundle/'phase12-manifest.json').read_text())
            package_manifest=json.loads((bundle/'package.json').read_text())
            self.assertEqual(manifest['kind'],'phase12-private-preview')
            self.assertTrue(manifest['private_demo'])
            self.assertFalse(manifest['public_release'])
            self.assertFalse(package_manifest['public_release'])
            self.assertFalse(package_manifest['main_install_enabled'])
            self.assertEqual(Path(result['launcher']).name,'EmotionalUI - Prueba.sh')
            self.assertIn('prueba privada', (bundle/'README.txt').read_text())
            with zipfile.ZipFile(result['zip']) as archive:
                self.assertIsNone(archive.testzip())
