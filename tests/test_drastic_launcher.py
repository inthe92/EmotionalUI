"""Synthetic adapter tests; no installed emulator, ROM, BIOS or graphics used."""
import importlib.util
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import shutil
import zipfile
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('drastic_launcher', ROOT / 'platform/darkos/drastic_launcher.py')
drastic = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drastic)


def metadata(name='folder/Game.NDS', size=4, extra=''):
    return ('Path = ' + name + '\nSize = ' + str(size)
            + '\nAttributes = A -rw-r--r--\nEncrypted = -\n' + extra + '\n').encode()


class DrasticAdapter(unittest.TestCase):
    def fixture(self, base):
        source = base / 'installed'
        source.mkdir()
        header = bytearray(64)
        header[:6] = b'\x7fELF\x02\x01'
        struct.pack_into('<HH', header, 16, 3, 183)
        (source / 'drastic').write_bytes(header)
        (source / 'config').mkdir()
        (source / 'config/drastic.cfg').write_text('menu_button=11\n')
        (source / 'system').mkdir()
        (source / 'system/private-bios').write_bytes(b'synthetic-not-bios')
        (source / 'drastic_logo_0.raw').write_bytes(b'fake')
        original = base / 'original-saves'
        original.mkdir()
        (original / 'never-touch').write_bytes(b'protected')
        for name in ('backup', 'cheats', 'savestates', 'slot2'):
            (source / name).symlink_to(original)
        data = base / 'private'
        library = base / 'library'
        library.mkdir()
        rom = library / 'synthetic.NDS'
        rom.write_bytes(b'test')
        return source, data, library, rom

    def prepared(self, base):
        source, data, library, rom = self.fixture(base)
        drastic.prepare(source, data, True)
        return source, data, library, rom

    def test_dry_run_writes_nothing_and_copy_is_private_and_independent(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            source, data, _, _ = self.fixture(base)
            before = drastic.digest(source / 'drastic')
            plan = drastic.prepare(source, data)
            self.assertFalse(plan['written'])
            self.assertFalse(data.exists())
            result = drastic.prepare(source, data, True)
            self.assertTrue(result['written'])
            self.assertEqual(before, drastic.digest(data / 'drastic'))
            self.assertFalse((data / 'drastic').is_symlink())
            self.assertEqual((data / 'drastic').stat().st_mode & 0o777, 0o700)
            self.assertEqual((data / 'config/drastic.cfg').stat().st_mode & 0o777, 0o600)
            self.assertEqual((source / 'config/drastic.cfg').read_text(),
                             (data / 'config/drastic.cfg').read_text())
            self.assertTrue((data / 'system').is_symlink())
            for name in drastic.MUTABLE:
                self.assertFalse((data / name).is_symlink())
                self.assertEqual(list((data / name).iterdir()), [])
            self.assertEqual((base / 'original-saves/never-touch').read_bytes(), b'protected')
            with self.assertRaises(drastic.LaunchError):
                drastic.prepare(source, data, True)

    def test_wrong_elf_and_config_alias_refused_before_writes(self):
        with tempfile.TemporaryDirectory() as folder:
            source, data, _, _ = self.fixture(Path(folder).resolve())
            header = (source / 'drastic').read_bytes()
            (source / 'drastic').write_bytes(b'invalid')
            with self.assertRaises(drastic.LaunchError): drastic.prepare(source, data, True)
            self.assertFalse(data.exists())
            (source / 'drastic').write_bytes(header)
            (source / 'config/alias.cfg').symlink_to(source / 'config/drastic.cfg')
            with self.assertRaises(drastic.LaunchError): drastic.prepare(source, data, True)
            self.assertFalse(data.exists())

    def test_copy_integrity_change_prevents_complete_manifest(self):
        with tempfile.TemporaryDirectory() as folder:
            source, data, _, _ = self.fixture(Path(folder).resolve())
            original = drastic.digest
            calls = 0
            def changing(path):
                nonlocal calls
                if path == source / 'drastic':
                    calls += 1
                    if calls > 1:
                        return '0' * 64
                return original(path)
            with patch.object(drastic, 'digest', side_effect=changing):
                with self.assertRaises(drastic.LaunchError): drastic.prepare(source, data, True)
            self.assertFalse((data / 'emotional-drastic.sha256').exists())

    def test_direct_nds_foreground_exact_exit_code_no_timeout(self):
        with tempfile.TemporaryDirectory() as folder:
            _, data, library, rom = self.prepared(Path(folder).resolve())
            with patch.object(drastic.subprocess, 'run', return_value=subprocess.CompletedProcess([], 17)) as run:
                self.assertEqual(drastic.run(data, library, rom), 17)
                run.assert_called_once_with([str(data / 'drastic'), str(rom)], cwd=data,
                                            check=False, umask=0o077)
                self.assertNotIn('timeout', run.call_args.kwargs)
                self.assertNotIn('shell', run.call_args.kwargs)

    def test_untrusted_workspace_and_tampered_binary_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            _, data, library, rom = self.prepared(base)
            with patch.object(drastic.subprocess, 'run') as run:
                data.chmod(0o755)
                with self.assertRaises(drastic.LaunchError): drastic.run(data, library, rom)
                data.chmod(0o700)
                (data / 'drastic').write_bytes(b'tampered')
                with self.assertRaises(drastic.LaunchError): drastic.run(data, library, rom)
                run.assert_not_called()

    def test_private_directory_symlink_refused_and_no_source_changed(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            source, data, library, rom = self.prepared(base)
            (data / 'backup').rmdir()
            (data / 'backup').symlink_to(base / 'original-saves')
            with self.assertRaises(drastic.LaunchError): drastic.run(data, library, rom)
            self.assertTrue((source / 'backup').is_symlink())

    def test_outside_library_alias_and_unknown_extensions_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            _, data, library, rom = self.prepared(base)
            outside = base / 'outside.nds'
            outside.write_bytes(b'test')
            alias = library / 'alias.nds'
            alias.symlink_to(outside)
            script = library / 'danger.sh'
            script.write_text('not executable')
            with patch.object(drastic.subprocess, 'run') as run:
                for content in (outside, alias, script):
                    with self.subTest(content=content.name):
                        with self.assertRaises(drastic.LaunchError): drastic.run(data, library, content)
                run.assert_not_called()

    def test_archive_member_accepts_uppercase_and_non_nds_companion(self):
        listing = metadata() + metadata('readme.txt', 2)
        self.assertEqual(drastic.archive_member(listing), ('folder/Game.NDS', 4))

    def test_archive_rejects_multiple_missing_duplicate_and_bad_encoding(self):
        for listing in (b'', metadata('readme.txt'), metadata() + metadata('second.nds'),
                        metadata() + metadata('FOLDER/game.nds'), b'Path = \xff\n\n',
                        b'malformed\n', metadata(extra='Size = 4\n')):
            with self.subTest(listing=listing):
                with self.assertRaises(drastic.LaunchError): drastic.archive_member(listing)

    def test_archive_rejects_traversal_absolute_options_and_external_references(self):
        # A leading dash is safe after --; absolute/ambiguous paths are not.
        self.assertEqual(drastic.archive_member(metadata('-game.nds'))[0], '-game.nds')
        for name in ('../game.nds', '/game.nds', 'folder/../game.nds', 'C:/game.nds',
                     'folder\\game.nds', './game.nds', 'folder//game.nds', 'a\x00.nds'):
            with self.subTest(name=name):
                with self.assertRaises(drastic.LaunchError): drastic.archive_member(metadata(name))
        for key in ('Symbolic Link', 'Hard Link', 'Link', 'Alternate Stream'):
            with self.assertRaises(drastic.LaunchError):
                drastic.archive_member(metadata(extra=key + ' = outside\n'))

    def test_archive_rejects_nonregular_encrypted_and_oversized(self):
        cases = [metadata(size=0), metadata(size=drastic.MAX_ROM + 1), metadata(size='bad'),
                 metadata().replace(b'Encrypted = -', b'Encrypted = +'),
                 metadata().replace(b'A -rw-r--r--', b'A lrwxrwxrwx'),
                 metadata(extra='Folder = +\n')]
        for listing in cases:
            with self.subTest(listing=listing):
                with self.assertRaises(drastic.LaunchError): drastic.archive_member(listing)

    def test_archive_extracts_only_member_and_keeps_it_until_game_exits(self):
        with tempfile.TemporaryDirectory() as folder:
            _, data, library, _ = self.prepared(Path(folder).resolve())
            archive = library / 'content.ZIP'
            archive.write_bytes(b'synthetic archive')
            calls = []
            def command(argv, maximum, timeout, output=None):
                calls.append(argv)
                if output is None:
                    return metadata()
                self.assertEqual(maximum, 4)
                output.write(b'game')
                return 4
            def game(argv, **kwargs):
                self.assertEqual(Path(argv[1]).read_bytes(), b'game')
                self.assertEqual(kwargs, dict(cwd=data, check=False, umask=0o077))
                return subprocess.CompletedProcess(argv, 9)
            with patch.object(drastic, 'bounded_command', side_effect=command), \
                    patch.object(drastic.subprocess, 'run', side_effect=game):
                self.assertEqual(drastic.run(data, library, archive, Path(sys.executable)), 9)
            self.assertEqual(calls[0][1:5], ['l', '-slt', '-ba', '--'])
            self.assertEqual(calls[1][1:6], ['e', '-so', '-spd', '--', str(archive)])
            self.assertEqual(calls[1][-1], 'folder/Game.NDS')
            self.assertEqual(list((data / 'unzip_cache').iterdir()), [])

    def test_bad_extract_never_starts_game_and_only_own_temp_is_removed(self):
        with tempfile.TemporaryDirectory() as folder:
            _, data, library, _ = self.prepared(Path(folder).resolve())
            archive = library / 'content.7z'
            archive.write_bytes(b'archive')
            keep = data / 'unzip_cache/keep'
            keep.write_bytes(b'untouched')
            with patch.object(drastic, 'bounded_command', side_effect=[metadata(), 2]), \
                    patch.object(drastic.subprocess, 'run') as run:
                with self.assertRaises(drastic.LaunchError):
                    drastic.run(data, library, archive, Path(sys.executable))
                run.assert_not_called()
            self.assertEqual(list((data / 'unzip_cache').iterdir()), [keep])
            self.assertEqual(keep.read_bytes(), b'untouched')

    def test_archived_games_have_distinct_and_stable_save_basenames(self):
        with tempfile.TemporaryDirectory() as folder:
            _, data, library, _ = self.prepared(Path(folder).resolve())
            archives = [library / 'first.zip', library / 'second.zip']
            for archive in archives:
                archive.write_bytes(b'archive')
            names, paths = [], []
            def command(argv, maximum, timeout, output=None):
                if output is None:
                    return metadata()
                output.write(b'game')
                return 4
            def game(argv, **kwargs):
                paths.append(Path(argv[1]))
                names.append(paths[-1].name)
                return subprocess.CompletedProcess(argv, 0)
            with patch.object(drastic, 'bounded_command', side_effect=command), \
                    patch.object(drastic.subprocess, 'run', side_effect=game):
                for archive in [archives[0], archives[1], archives[0]]:
                    self.assertEqual(drastic.run(data, library, archive, Path(sys.executable)), 0)
            self.assertNotEqual(names[0], names[1])
            self.assertEqual(names[0], names[2])
            self.assertNotEqual(paths[0].parent, paths[2].parent)
            self.assertTrue(all(name.startswith('game-') and name.endswith('.nds') for name in names))

    @unittest.skipUnless(shutil.which('7z') or shutil.which('7zz'), 'No local 7z/7zz installed')
    def test_real_archive_tool_lists_and_extracts_synthetic_zip_and_7z(self):
        tool = Path(shutil.which('7z') or shutil.which('7zz'))
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            _, data, library, _ = self.prepared(base)
            member = base / 'Synthetic.NDS'
            member.write_bytes(b'synthetic content, not a ROM')
            zip_archive = library / 'test.zip'
            with zipfile.ZipFile(zip_archive, 'w') as archive:
                archive.write(member, arcname='folder/Synthetic.NDS')
            seven_archive = library / 'test.7z'
            subprocess.run([str(tool), 'a', '-t7z', str(seven_archive), member.name],
                           cwd=base, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            for archive in (zip_archive, seven_archive):
                listing = drastic.bounded_command([str(tool), 'l', '-slt', '-ba', '--', str(archive)],
                                                  drastic.MAX_LIST, 10)
                self.assertEqual(drastic.archive_member(listing)[1], member.stat().st_size)
                def game(argv, **kwargs):
                    self.assertEqual(Path(argv[1]).read_bytes(), member.read_bytes())
                    return subprocess.CompletedProcess(argv, 0)
                with patch.object(drastic.subprocess, 'run', side_effect=game):
                    self.assertEqual(drastic.run(data, library, archive, tool), 0)

    def test_bounded_command_success_exit_failure_limit_and_timeout(self):
        command = [sys.executable, '-c']
        self.assertEqual(drastic.bounded_command(command + ["print('ok')"], 4, 2), b'ok\n')
        for code, limit, timeout in [('raise SystemExit(1)', 32, 2),
                                     ("print('x'*10000)", 8, 2),
                                     ('import time; time.sleep(2)', 32, 0.05)]:
            with self.subTest(code=code):
                with self.assertRaises(drastic.LaunchError):
                    drastic.bounded_command(command + [code], limit, timeout)

    def test_cli_error_does_not_expose_private_path(self):
        with patch('builtins.print') as output:
            result = drastic.main(['run', '--data', '/nonexistent-private-folder',
                                   '--rom-root', '/private-library', '/private-game.nds'])
        self.assertEqual(result, 2)
        self.assertNotIn('private-folder', str(output.call_args))
        self.assertNotIn('private-game', str(output.call_args))


if __name__ == '__main__':
    unittest.main()
