"""Private harness checks only. No console, display, real games or system writes."""
import hashlib
import io
import importlib.util
import json
import os
import struct
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from contextlib import redirect_stdout, redirect_stderr

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'platform/darkos'))
spec = importlib.util.spec_from_file_location('functional_preview', ROOT/'platform/darkos/functional_preview.py')
functional = importlib.util.module_from_spec(spec)
spec.loader.exec_module(functional)
UNIT = 'emotional-ui-functional-'+'a'*32


def elf(path):
    data = bytearray(64)
    data[:6] = b'\x7fELF\x02\x01'
    struct.pack_into('<H', data, 16, 3)
    struct.pack_into('<H', data, 18, 183)
    path.write_bytes(data)
    path.chmod(0o700)
    return hashlib.sha256(data).hexdigest()


class FunctionalPreview(unittest.TestCase):
    def command(self, **kwargs):
        return functional.command('/tmp/owned/frontend', '/tmp/owned/profile.toml',
                                  '/tmp/owned/catalog.sqlite', '/tmp/owned/library',
                                  '/tmp/owned/runtime', UNIT, uid=1000, home='/home/ark',
                                  script='/tmp/owned/functional_preview.py', **kwargs)

    def test_functional_contract_has_no_game_watchdog_or_trial_permission(self):
        argv = self.command()
        self.assertIn('--functional', argv)
        self.assertIn('--launcher-config', argv)
        self.assertIn('--launch-physical-supervised', argv)
        self.assertNotIn('--trial', argv)
        self.assertFalse(any('RuntimeMaxSec=' in a for a in argv))
        for expected in ('--property=KillMode=process', '--property=TimeoutStopSec=infinity',
                         '--property=SendSIGKILL=no', '--property=User=1000'):
            self.assertIn(expected, argv)
        restore = next(a for a in argv if 'ExecStopPost=' in a)
        self.assertIn('ExecStopPost=/usr/bin/python3', restore)
        self.assertNotIn('ExecStopPost=+', restore)
        self.assertFalse(any(' enable ' in a or 'install-main' in a for a in argv))

    def test_default_software_and_explicit_gpu_are_separate(self):
        self.assertTrue(any('SLINT_BACKEND=linuxkms-software ' in a for a in self.command()))
        self.assertTrue(any('SLINT_BACKEND=linuxkms-femtovg ' in a for a in self.command(renderer='femtovg')))
        self.assertFalse(any('GPU_SWAP=immediate' in a for a in self.command()))
        with self.assertRaises(ValueError):
            self.command(renderer='unknown')

    def test_original_library_and_retroarch_configuration_are_read_only(self):
        argv = self.command()
        self.assertIn('--property=ReadOnlyPaths=/tmp/owned/library -/home/ark/.config/retroarch -/home/ark/.config/retroarch32 -/opt/drastic', argv)
        self.assertEqual(sum(a.startswith('--property=ReadOnlyPaths=') for a in argv), 1)

    def test_paths_and_units_cannot_inject_systemd_commands(self):
        for path in ('/tmp/%n', '/tmp/../etc', '/tmp/owned\nExecStart=evil', 'relative'):
            with self.assertRaises(ValueError):
                functional.command(path, '/tmp/p', '/tmp/d', '/tmp/l', '/tmp/r', UNIT, uid=1000, home='/home/ark')
        with self.assertRaises(ValueError):
            functional.unit_name('emulationstation.service')

    def fixture(self, root):
        binary = root/'frontend'
        digest = elf(binary)
        emulator = root/'synthetic-arm64-header'
        elf(emulator)  # Never executed; loader/graphics are not tested here.
        library = root/'library'
        library.mkdir()
        runtime = root/'runtime'
        runtime.mkdir(mode=0o700)
        profile = root/'profile.toml'
        profile.write_text('handoff="process-exit"\n[[launchers]]\nid="synthetic"\n'
                           +'root='+json.dumps(str(library))+'\n'
                           +'executable='+json.dumps(str(emulator))+'\n'
                           +'working_directory='+json.dumps(str(root))+'\n'
                           +'arguments=["{rom}"]\nwaits_for_exit=true\n')
        return (binary, digest, profile, hashlib.sha256(profile.read_bytes()).hexdigest(),
                root/'catalog.sqlite', library, runtime)

    def test_exact_elf_and_profile_hashes_checked_without_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = self.fixture(Path(directory).resolve())
            functional.verify_inputs(*paths)
            paths[0].write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                functional.verify_inputs(*paths)

    def test_arm32_emulator_allowed_but_frontend_remains_arm64_and_core_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = list(self.fixture(Path(directory).resolve()))
            emulator = paths[0].parent/'synthetic-arm64-header'
            header = bytearray(emulator.read_bytes())
            header[4] = 1
            struct.pack_into('<H', header, 18, 40)
            emulator.write_bytes(header)
            functional.verify_inputs(*paths)
            core = emulator.parent/'core.so'
            elf(core)  # AArch64 core cannot be loaded into a 32-bit ARM process.
            paths[2].write_text(paths[2].read_text().replace('arguments=["{rom}"]', 'arguments=["-L",'+json.dumps(str(core))+',"{rom}"]'))
            paths[3] = hashlib.sha256(paths[2].read_bytes()).hexdigest()
            with self.assertRaisesRegex(ValueError, 'architectures differ'):
                functional.verify_inputs(*paths)
            core.write_bytes(header)
            functional.verify_inputs(*paths)
            paths[0].write_bytes(header)
            paths[1] = hashlib.sha256(header).hexdigest()
            with self.assertRaisesRegex(ValueError, 'ARM64'):
                functional.verify_inputs(*paths)

    def test_system_extensions_reject_injected_or_unbounded_values(self):
        with tempfile.TemporaryDirectory() as directory:
            original = list(self.fixture(Path(directory).resolve()))
            text = original[2].read_text()
            for extension in ['[".ZIP", ".7z"]', '[".zip;sh"]', '".zip"']:
                paths = list(original)
                paths[2].write_text(text+'extensions='+extension+'\n')
                paths[3] = hashlib.sha256(paths[2].read_bytes()).hexdigest()
                if extension == '[".ZIP", ".7z"]':
                    functional.verify_inputs(*paths)
                else:
                    with self.assertRaisesRegex(ValueError, 'extensions'):
                        functional.verify_inputs(*paths)

    def test_profile_must_keep_foreground_and_cannot_use_system_wrappers(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = list(self.fixture(Path(directory).resolve()))
            text = paths[2].read_text().replace('waits_for_exit=true', 'waits_for_exit=false')
            paths[2].write_text(text)
            paths[3] = hashlib.sha256(paths[2].read_bytes()).hexdigest()
            with self.assertRaisesRegex(ValueError, 'foreground'):
                functional.verify_inputs(*paths)
            text = text.replace('waits_for_exit=false', 'waits_for_exit=true')
            paths[2].write_text(text)
            paths[3] = hashlib.sha256(paths[2].read_bytes()).hexdigest()
            (paths[0].parent/'synthetic-arm64-header').write_text('#!/bin/sh\nexit 0\n')
            with self.assertRaisesRegex(ValueError, 'shell wrappers'):
                functional.verify_inputs(*paths)

    def cgroup(self, root, content=None):
        membership = root/'self-cgroup'
        membership.write_text('0::/system.slice/'+UNIT+'.service\n')
        group = root/'cgroup/system.slice'/(UNIT+'.service')
        group.mkdir(parents=True)
        (group/'cgroup.procs').write_text(content if content is not None else '100\n')
        return membership, root/'cgroup', group

    def test_recovery_cannot_restore_from_another_unit_or_unreadable_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            membership, root, group = self.cgroup(Path(directory))
            restore = unittest.mock.Mock()
            membership.write_text('0::/user.slice/other.service\n')
            with self.assertRaises(ValueError):
                functional.recover(UNIT, membership, root, self_pid=100, run=restore)
            restore.assert_not_called()
            membership.write_text('0::/system.slice/'+UNIT+'.service\n')
            (group/'cgroup.procs').unlink()
            with self.assertRaises(ValueError):
                functional.recover(UNIT, membership, root, self_pid=100, run=restore)
            restore.assert_not_called()

    def test_recovery_waits_for_game_and_descendants_without_killing_them(self):
        with tempfile.TemporaryDirectory() as directory:
            membership, root, group = self.cgroup(Path(directory), '100\n200\n')
            calls = []
            def pause(_):
                calls.append('game-finished')
                (group/'cgroup.procs').write_text('100\n')
            def restore(argv, **kwargs):
                self.assertEqual(calls, ['game-finished'])
                self.assertEqual(argv, ['/usr/bin/sudo', '-n', '/usr/bin/systemctl', 'start', 'emulationstation.service'])
                calls.append('restore')
                return SimpleNamespace(returncode=0)
            functional.recover(UNIT, membership, root, self_pid=100, alive=lambda _: True, pause=pause, run=restore)
            self.assertEqual(calls, ['game-finished', 'restore'])

    def test_nested_child_cgroup_also_delays_restoration(self):
        with tempfile.TemporaryDirectory() as directory:
            membership, root, group = self.cgroup(Path(directory))
            nested = group/'children'
            nested.mkdir()
            (nested/'cgroup.procs').write_text('200\n')
            blocked = []
            def pause(_):
                blocked.append(True)
                (nested/'cgroup.procs').write_text('')
            restore = unittest.mock.Mock(return_value=SimpleNamespace(returncode=0))
            functional.recover(UNIT, membership, root, self_pid=100, alive=lambda _: True, pause=pause, run=restore)
            self.assertEqual(blocked, [True])
            restore.assert_called_once()

    def test_unreadable_pid_state_is_not_treated_as_a_finished_game(self):
        with tempfile.TemporaryDirectory() as directory:
            membership, root, group = self.cgroup(Path(directory), '100\n200\n')
            def unreadable(_):
                raise PermissionError('synthetic proc denial')
            restore = unittest.mock.Mock()
            with self.assertRaises(PermissionError):
                functional.recover(UNIT, membership, root, self_pid=100, alive=unreadable, run=restore)
            restore.assert_not_called()

    def test_synthetic_process_must_exit_before_recovery(self):
        # Real child lifecycle, synthetic content. No Slint, emulator or SSH.
        with tempfile.TemporaryDirectory() as directory:
            child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(0.15)'])
            try:
                membership, root, _ = self.cgroup(Path(directory), '100\n'+str(child.pid)+'\n')
                observed_wait = []
                restored = []
                def pause(seconds):
                    observed_wait.append(True)
                    time.sleep(min(seconds, .02))
                def restore(*args, **kwargs):
                    self.assertIsNotNone(child.poll())
                    restored.append(True)
                    return SimpleNamespace(returncode=0)
                functional.recover(UNIT, membership, root, self_pid=100,
                                   alive=lambda pid: child.poll() is None, pause=pause, run=restore)
                self.assertTrue(observed_wait)
                self.assertEqual(restored, [True])
            finally:
                child.wait(timeout=5)

    def test_execute_without_current_acknowledgements_never_hands_off(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = self.fixture(Path(directory).resolve())
            argv = ['functional_preview.py', 'start', '--binary', str(paths[0]), '--candidate-sha256', paths[1],
                    '--profile', str(paths[2]), '--profile-sha256', paths[3], '--db', str(paths[4]),
                    '--library-root', str(paths[5]), '--runtime', str(paths[6]), '--execute']
            with patch.object(sys, 'argv', argv), patch.object(functional, 'preflight') as preflight, patch.object(functional.subprocess, 'run') as run:
                with redirect_stderr(io.StringIO()):
                    self.assertEqual(functional.main(), 1)
                preflight.assert_not_called()
                run.assert_not_called()

    def test_default_dry_run_verifies_inputs_without_commands_or_runtime_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = self.fixture(Path(directory).resolve())
            argv = ['functional_preview.py', 'start', '--binary', str(paths[0]), '--candidate-sha256', paths[1],
                    '--profile', str(paths[2]), '--profile-sha256', paths[3], '--db', str(paths[4]),
                    '--library-root', str(paths[5]), '--runtime', str(paths[6])]
            stdout = io.StringIO()
            with patch.object(sys, 'argv', argv), patch.object(functional.subprocess, 'run') as run, redirect_stdout(stdout):
                self.assertEqual(functional.main(), 0)
                run.assert_not_called()
            self.assertTrue(json.loads(stdout.getvalue())['dry_run'])
            self.assertFalse(paths[4].exists())
            self.assertEqual(list(paths[6].iterdir()), [])

    def test_installed_input_profile_is_valid_and_forwarded_in_harness_argv(self):
        profile = ROOT/'platform/distribution/rg353v-input.toml'
        self.assertEqual(functional.verify_input_profile(profile), hashlib.sha256(profile.read_bytes()).hexdigest())
        argv = self.command(input_profile=str(profile))
        self.assertEqual(argv[argv.index('--input-profile')+1], str(profile))

    def test_invalid_input_profile_is_rejected_before_display_preflight(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = self.fixture(Path(directory).resolve())
            profile = paths[0].parent/'input.toml'
            base = ['functional_preview.py', 'start', '--binary', str(paths[0]), '--candidate-sha256', paths[1],
                    '--profile', str(paths[2]), '--profile-sha256', paths[3], '--db', str(paths[4]),
                    '--library-root', str(paths[5]), '--runtime', str(paths[6]), '--execute',
                    '--supervised', '--recovery-confirmed', '--second-session-confirmed', '--isolated-saves-confirmed']
            cases = [None,
                     'enabled=false\ndevice_name="retrogame_joypad"\n[mapping]\n',
                     'enabled=true\ndevice_name="retrogame_joypad"\n[mapping]\nbuttons=[]\n',
                     'not valid toml']
            for content in cases:
                argv = list(base)
                if content is not None:
                    profile.write_text(content)
                    argv.extend(['--input-profile', str(profile)])
                with self.subTest(content=content), patch.object(sys, 'argv', argv), patch.object(functional, 'preflight') as preflight, patch.object(functional.subprocess, 'run') as run, redirect_stderr(io.StringIO()):
                    self.assertEqual(functional.main(), 1)
                    preflight.assert_not_called()
                    run.assert_not_called()

    def test_duplicate_button_and_unknown_action_do_not_pass_input_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory).resolve()/'input.toml'
            original = (ROOT/'platform/distribution/rg353v-input.toml').read_text()
            for text in (original.replace('code = 545', 'code = 544'),
                         original.replace('action = "up"', 'action = "unrecognized"')):
                profile.write_text(text)
                with self.assertRaises(ValueError):
                    functional.verify_input_profile(profile)


if __name__ == '__main__':
    unittest.main()
