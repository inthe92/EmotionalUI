"""Synthetic Ports-entry guards. No console, game, network or graphics calls."""
import hashlib
import importlib.util
import json
import os
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'platform/distribution'))
sys.path.insert(0, str(ROOT/'platform/darkos'))
spec = importlib.util.spec_from_file_location('functional_menu', ROOT/'platform/darkos/functional_menu.py')
functional = importlib.util.module_from_spec(spec)
spec.loader.exec_module(functional)


class FunctionalMenu(unittest.TestCase):
    def fixture(self, root):
        binary = root/'frontend'
        data = bytearray(64)
        data[:6] = b'\x7fELF\x02\x01'
        struct.pack_into('<H', data, 16, 3)
        struct.pack_into('<H', data, 18, 183)
        binary.write_bytes(data)
        binary.chmod(0o700)
        library = root/'library'
        library.mkdir()
        profile = root/'launcher.toml'
        profile.write_text('handoff="process-exit"\n[[launchers]]\nid="synthetic"\n'
                           +'root='+json.dumps(str(library))+'\n'
                           +'executable='+json.dumps(str(binary))+'\n'
                           +'working_directory='+json.dumps(str(root))+'\n'
                           +'arguments=["{rom}"]\nwaits_for_exit=true\n')
        controls = root/'input.toml'
        controls.write_bytes((ROOT/'platform/distribution/rg353v-input.toml').read_bytes())
        store = root/'data'
        store.mkdir(mode=0o700)
        config = dict(binary=str(binary), binary_sha256=hashlib.sha256(data).hexdigest(),
                      profile=str(profile), profile_sha256=hashlib.sha256(profile.read_bytes()).hexdigest(),
                      input_profile=str(controls), db=str(store/'library.sqlite'), library_root=str(library),
                      data=str(store), renderer='software')
        path = root/'functional.json'
        self.save(path, config)
        return path, config

    def save(self, path, config):
        path.write_text(json.dumps(config))
        path.chmod(0o600)

    def test_private_config_requires_exact_schema_owner_permissions_and_safe_paths(self):
        with tempfile.TemporaryDirectory() as folder:
            path, config = self.fixture(Path(folder).resolve())
            functional.read_config(path)
            path.chmod(0o644)
            with self.assertRaises(ValueError): functional.read_config(path)
            path.chmod(0o600)
            with patch.object(functional.os, 'getuid', return_value=os.getuid()+1):
                with self.assertRaises(ValueError): functional.read_config(path)
            for field, value in [('db', '/tmp/%n'), ('renderer', 'unknown'), ('binary_sha256', 'bad')]:
                changed = dict(config, **{field: value})
                self.save(path, changed)
                with self.assertRaises(ValueError): functional.read_config(path)
            self.save(path, dict(config, unexpected='not accepted'))
            with self.assertRaises(ValueError): functional.read_config(path)

    def test_dry_run_verifies_hardware_free_and_keeps_functional_contract(self):
        with tempfile.TemporaryDirectory() as folder:
            path, config = self.fixture(Path(folder).resolve())
            with patch.object(functional.subprocess, 'run') as run:
                result = functional.launch(path)
                run.assert_not_called()
            self.assertTrue(result['dry_run'])
            argv = result['argv']
            self.assertIn('--functional', argv)
            self.assertIn('--launcher-config', argv)
            self.assertIn('--input-profile', argv)
            self.assertNotIn('--trial', argv)
            self.assertFalse(any('RuntimeMaxSec=' in argument for argument in argv))
            self.assertFalse(Path(result['runtime']).exists())
            self.assertFalse(Path(config['db']).exists())

    def test_symlink_config_and_changed_binary_refused_without_commands(self):
        with tempfile.TemporaryDirectory() as folder:
            path, config = self.fixture(Path(folder).resolve())
            alias = path.parent/'alias.json'
            alias.symlink_to(path)
            with self.assertRaises(ValueError): functional.read_config(alias)
            Path(config['binary']).write_bytes(b'changed')
            with patch.object(functional.subprocess, 'run') as run:
                with self.assertRaises(ValueError): functional.launch(path, True)
                run.assert_not_called()

    def environment(self, tree, functional_active=False, missing_library=False):
        def invoke(argv, **kwargs):
            if argv[:2] == ['/usr/bin/systemctl', 'list-units']:
                return SimpleNamespace(returncode=0, stdout='active functional unit' if functional_active else '', stderr='')
            if argv[0] == '/usr/bin/ps':
                return SimpleNamespace(returncode=0, stdout=tree, stderr='')
            if argv[0] == '/lib/ld-linux-aarch64.so.1':
                return SimpleNamespace(returncode=0, stdout='libexample.so => not found' if missing_library else 'all resolved', stderr='')
            return SimpleNamespace(returncode=0, stdout='', stderr='')
        return invoke

    def test_direct_ssh_cannot_start_a_functional_menu_session(self):
        with tempfile.TemporaryDirectory() as folder:
            path, config = self.fixture(Path(folder).resolve())
            tree = '10 1 bash\n11 10 emulationstatio\n30 22 python3\n22 1 sshd\n'
            with patch.object(functional.menu, 'check_no_session'), patch.object(functional.menu, 'service_state', return_value=10), patch.object(functional.os, 'getpid', return_value=30), patch.object(functional.subprocess, 'run', side_effect=self.environment(tree)) as run:
                with self.assertRaisesRegex(ValueError, 'menu'):
                    functional.launch(path, True)
            self.assertFalse(any('/usr/bin/systemd-run' in call.args[0] for call in run.call_args_list))
            self.assertEqual(list(Path(config['data']).iterdir()), [])

    def test_active_trial_guard_is_reused(self):
        with patch.object(functional.menu, 'check_no_session', side_effect=ValueError('trial active')), patch.object(functional.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'trial active'):
                functional.check_no_session()
            run.assert_not_called()

    def test_active_functional_unit_also_prevents_start(self):
        with patch.object(functional.menu, 'check_no_session'), patch.object(functional.subprocess, 'run', side_effect=self.environment('', functional_active=True)):
            with self.assertRaisesRegex(ValueError, 'wait'):
                functional.check_no_session()

    def test_graphical_menu_ancestry_and_all_preflight_checks_precede_unit_creation(self):
        with tempfile.TemporaryDirectory() as folder:
            path, config = self.fixture(Path(folder).resolve())
            tree = '10 1 bash\n11 10 emulationstatio\n20 11 sh\n30 20 python3\n'
            with patch.object(functional.menu, 'check_no_session'), patch.object(functional.menu, 'service_state', return_value=10), patch.object(functional.os, 'getpid', return_value=30), patch.object(functional, 'require_cgroup_v2'), patch.object(functional, 'backlight_values', return_value=[130]), patch.object(functional.subprocess, 'run', side_effect=self.environment(tree)) as run:
                result = functional.launch(path, True)
            argv = [call.args[0] for call in run.call_args_list]
            self.assertTrue(result['started'])
            self.assertEqual(argv[-1], result['argv'])
            self.assertTrue(any(a[0] == '/lib/ld-linux-aarch64.so.1' for a in argv[:-1]))
            self.assertTrue(any(a[:3] == ['/usr/bin/sudo', '-n', '/usr/bin/systemctl'] for a in argv[:-1]))
            runtime = Path(result['runtime'])
            self.assertEqual(runtime.parent, Path(config['data']))
            self.assertEqual(runtime.stat().st_mode & 0o777, 0o700)

    def test_dark_screen_or_missing_dependency_never_start_unit(self):
        tree = '10 1 bash\n11 10 emulationstatio\n30 11 python3\n'
        for brightness, missing in [([0], False), ([130], True)]:
            with tempfile.TemporaryDirectory() as folder:
                path, config = self.fixture(Path(folder).resolve())
                with patch.object(functional.menu, 'check_no_session'), patch.object(functional.menu, 'service_state', return_value=10), patch.object(functional.os, 'getpid', return_value=30), patch.object(functional, 'require_cgroup_v2'), patch.object(functional, 'backlight_values', return_value=brightness), patch.object(functional.subprocess, 'run', side_effect=self.environment(tree, missing_library=missing)) as run:
                    with self.assertRaises(ValueError): functional.launch(path, True)
                self.assertFalse(any('/usr/bin/systemd-run' in call.args[0] for call in run.call_args_list))
                self.assertEqual(list(Path(config['data']).iterdir()), [])


if __name__ == '__main__':
    unittest.main()
