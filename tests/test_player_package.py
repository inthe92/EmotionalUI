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
            files += [ROOT/'tools/darkos_launcher_profiles.py']
            for source in files:shutil.copyfile(source,payload/source.name)
            package.verify_entrypoint_imports(payload)
            (payload/'menu.py').unlink()
            with self.assertRaisesRegex(RuntimeError,'Incomplete player package'):
                package.verify_entrypoint_imports(payload)
