"""Read-only XML/profile planning and own-file writes; no emulators or ROMs run."""
import importlib.util
import os
import stat
import struct
import tempfile
import tomllib
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('darkos_profiles', ROOT/'tools/darkos_launcher_profiles.py')
profiles = importlib.util.module_from_spec(spec)
spec.loader.exec_module(profiles)


def elf(path, architecture='aarch64', kind=3, executable=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    header = bytearray(64)
    header[:6] = b'\x7fELF'+bytes([2 if architecture == 'aarch64' else 1, 1])
    struct.pack_into('<HH', header, 16, kind, 183 if architecture == 'aarch64' else 40)
    path.write_bytes(header)
    path.chmod(0o700 if executable else 0o600)


class DarkosProfiles(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        self.home, self.library, self.binary = [self.root/name for name in ('home', 'roms', 'bin')]
        for path in (self.home, self.library, self.binary): path.mkdir()
        self.data = self.root/'own'/'data'
        self.systems = self.root/'es_systems.cfg'
        self.document = ET.Element('systemList')

    def tearDown(self):
        self.temporary.cleanup()

    def installed(self, core='snes9x', emulator='retroarch', architecture='aarch64', core_arch=None):
        elf(self.binary/emulator, architecture, executable=True)
        cfg = self.home/'.config'/emulator
        cfg.mkdir(parents=True, exist_ok=True)
        (cfg/'retroarch.cfg').write_text('system_directory = "/original/BIOS"\nconfig_save_on_exit = "true"\n')
        elf(cfg/'cores'/(core+'_libretro.so'), core_arch or architecture)

    def system(self, name='snes', folder='snes', cores=('snes9x',), emulator='retroarch', suffixes='.sfc .SFC .zip .ZIP', path=None, default='retroarch'):
        root = self.library/folder
        root.mkdir(parents=True, exist_ok=True)
        system = ET.SubElement(self.document, 'system')
        for key, value in [('name', name), ('path', str(path or root)), ('extension', suffixes),
                           ('command', 'touch /tmp/NEVER_EXECUTED; sudo dangerous %ROM%')]:
            ET.SubElement(system, key).text = value
        emulators = ET.SubElement(system, 'emulators')
        if default != emulator:
            ET.SubElement(emulators, 'emulator', name=default, default='true')
        node = ET.SubElement(emulators, 'emulator', name=emulator)
        if cores is not None:
            core_node = ET.SubElement(node, 'cores')
            for core in cores: ET.SubElement(core_node, 'core').text = core
        return system

    def plan(self):
        self.systems.write_bytes(ET.tostring(self.document))
        return profiles.build_plan(self.systems, self.home, self.library, self.data, self.binary)

    def test_functional_plan_separates_launch_resume_and_original_progress(self):
        self.installed()
        self.system()
        self.systems.write_bytes(ET.tostring(self.document))
        base = self.home/'.config/retroarch/retroarch.cfg'
        original = base.read_bytes()
        plan = profiles.build_plan(self.systems,self.home,self.library,self.data,self.binary,functional=True)
        parsed = tomllib.loads(plan['profiles'])
        launcher = parsed['launchers'][0]
        normal = Path(launcher['arguments'][launcher['arguments'].index('--appendconfig')+1])
        resume = Path(launcher['resume_arguments'][launcher['resume_arguments'].index('--appendconfig')+1])
        self.assertNotEqual(normal,resume)
        normal_cfg = tomllib.loads(plan['append_configs'][str(normal)])
        resume_cfg = tomllib.loads(plan['append_configs'][str(resume)])
        self.assertEqual(normal_cfg['savestate_auto_load'],'false')
        self.assertEqual(resume_cfg['savestate_auto_load'],'true')
        self.assertEqual(normal_cfg['savestate_auto_save'],'true')
        self.assertEqual(normal_cfg['config_save_on_exit'],'false')
        self.assertFalse(Path(launcher['progress_root']).is_relative_to(self.library))
        self.assertEqual(base.read_bytes(),original)
        self.assertFalse(self.data.exists())

    def test_command_shell_is_ignored_and_plan_is_read_only_with_extensions(self):
        self.installed()
        self.system()
        before = sorted(str(p) for p in self.root.rglob('*'))
        plan = self.plan()
        parsed = tomllib.loads(plan['profiles'])
        self.assertEqual(parsed['handoff'], 'process-exit')
        self.assertFalse(parsed['clock_confirmed'])
        launcher = parsed['launchers'][0]
        self.assertEqual(launcher['extensions'], ['.sfc', '.zip'])
        self.assertEqual(launcher['executable'], str(self.binary/'retroarch'))
        self.assertNotIn('touch', repr(launcher))
        self.assertNotIn('sudo', repr(launcher))
        self.assertFalse(self.data.exists())
        self.assertFalse(plan['report']['physical_tested'])
        after = sorted(str(p) for p in self.root.rglob('*') if p != self.systems)
        self.assertEqual(before, after)

    def test_chooses_first_declared_installed_compatible_core_and_reports_fallback(self):
        self.installed('snes9x')
        self.system(cores=('not-installed', 'snes9x'), default='standalone')
        plan = self.plan()
        record = plan['report']['included'][0]
        self.assertEqual(record['core'], 'snes9x')
        self.assertTrue(record['fallback_from_non_retroarch'])
        self.assertEqual(record['default_emulator'], 'standalone')

    def test_duplicate_system_names_keep_distinct_folder_roots_and_deduplicate_alias(self):
        self.installed()
        self.system(name='snes', folder='snes')
        self.system(name='snes', folder='sufami')
        self.system(name='duplicate', folder='duplicate', path=self.library/'snes')
        plan = self.plan()
        self.assertEqual(len(plan['report']['included']), 2)
        self.assertEqual(len({p['root'] for p in tomllib.loads(plan['profiles'])['launchers']}), 2)
        self.assertEqual(plan['report']['skipped'][0]['reason'], 'duplicate canonical root')

    def test_escape_symlink_root_and_non_game_systems_are_excluded(self):
        self.installed()
        outside = self.root/'outside'
        outside.mkdir()
        (self.library/'alias').symlink_to(outside, target_is_directory=True)
        self.system(folder='alias', path=self.library/'alias')
        for name in ('ports', 'movies', 'cmds'): self.system(name=name, folder=name)
        plan = self.plan()
        self.assertEqual(plan['report']['included'], [])
        self.assertEqual(len(plan['report']['skipped']), 4)

    def test_missing_core_and_wrong_elf_architecture_are_not_routed(self):
        self.installed(core_arch='arm32')
        self.system()
        self.assertEqual(len(self.plan()['report']['included']), 0)
        (self.home/'.config/retroarch/cores/snes9x_libretro.so').unlink()
        self.assertEqual(len(self.plan()['report']['included']), 0)

    def test_32bit_executable_and_matching_et_dyn_core_are_supported(self):
        self.installed('genesis_plus_gx', 'retroarch32', 'arm32')
        self.system('megadrive', 'megadrive', ('genesis_plus_gx',), 'retroarch32')
        self.assertEqual(self.plan()['report']['included'][0]['architecture'], 'arm32')
        elf(self.home/'.config/retroarch32/cores/genesis_plus_gx_libretro.so', 'arm32', kind=2)
        self.assertEqual(self.plan()['report']['included'], [])

    def test_fixed_map_requires_existing_core_and_does_not_start_standalone_nds(self):
        self.installed('gw')
        self.system('gameandwatch', 'gameandwatch', None)
        self.system('nds', 'nds', None, default='drastic', suffixes='.nds .zip')
        plan = self.plan()
        self.assertEqual([p['core'] for p in plan['report']['included']], ['gw'])
        self.assertEqual(plan['report']['included'][0]['core_source'], 'fixed-map')
        self.assertEqual(plan['report']['skipped'][0]['system'], 'nds')

    def test_scripts_libraries_and_invalid_extensions_are_filtered_per_root(self):
        self.installed()
        self.system(suffixes='.SFC .ZIP .7Z .sh .SO .dll .exe *.sfc .tar.gz')
        plan = self.plan()
        self.assertEqual(tomllib.loads(plan['profiles'])['launchers'][0]['extensions'], ['.7z', '.sfc', '.zip'])
        self.assertIn('.sh', plan['report']['included'][0]['excluded_extensions'])

    def test_pico8_nested_carts_alias_routes_png_to_fixed_fake08_core(self):
        self.installed('fake08')
        self.system(name='pico-8', folder='pico-8/carts', cores=None, suffixes='.p8 .PNG .sh')
        plan = self.plan()
        launcher = tomllib.loads(plan['profiles'])['launchers'][0]
        self.assertEqual(launcher['extensions'], ['.p8', '.png'])
        self.assertIn(str(self.home/'.config/retroarch/cores/fake08_libretro.so'), launcher['arguments'])
        self.assertEqual(launcher['executable'], str(self.binary/'retroarch'))
        self.assertEqual(launcher['arguments'][-1], '{rom}')
        self.assertFalse(plan['report']['physical_tested'])

    def test_guest_executables_and_core_cmd_formats_never_become_host_commands(self):
        self.assertEqual(profiles.extensions('.exe .COM .bat .sh .dll .so', 'dos', 'dosbox_pure')[0], ['.bat', '.com', '.exe'])
        for folder, core in [('other', 'dosbox_pure'), ('dos', 'unrecognized')]:
            self.assertEqual(profiles.extensions('.exe .com .bat', folder, core)[0], [])
        for core in ('vice_x64', 'px68k', 'np2kai', 'nekop2', 'atari800', 'minivmac'):
            self.assertEqual(profiles.extensions('.cmd .sh .dll .so', 'system', core)[0], ['.cmd'])
        self.assertEqual(profiles.extensions('.cmd', 'system', 'unrecognized')[0], [])
        for core in ('doukutsu_rs', 'nxengine'):
            self.assertEqual(profiles.extensions('.exe .dll .so', 'cavestory', core)[0], ['.exe'])

    def test_documented_core_formats_cover_actual_collection_omissions(self):
        for name, core, declared, expected in [
                ('pcengine', 'mednafen_pce_fast', '.pce', {'.pce', '.cue', '.chd', '.m3u'}),
                ('psx', 'pcsx_rearmed', '.cue .chd', {'.cue', '.chd', '.bin'}),
                ('easyrpg', 'easyrpg', '.zip', {'.zip', '.ldb', '.lzh'})]:
            self.installed(core)
            self.system(name, name, (core,), suffixes=declared)
        plan = self.plan()
        launchers = tomllib.loads(plan['profiles'])['launchers']
        by_folder = {Path(p['root']).name: set(p['extensions']) for p in launchers}
        self.assertEqual(by_folder['pcengine'], {'.pce', '.cue', '.chd', '.m3u'})
        self.assertIn('.bin', by_folder['psx'])
        self.assertTrue({'.ldb', '.lzh'}.issubset(by_folder['easyrpg']))
        self.assertIn('.bin', profiles.extensions('.cue', 'psx', 'pcsx_rearmed_rumble')[0])
        self.assertNotIn('.ldb', profiles.extensions('.zip', 'other', 'snes9x')[0])

    def test_neogeo_cd_chd_uses_disjoint_neocd_profile_on_the_same_root(self):
        self.installed('fbneo')
        self.installed('neocd')
        self.system('neogeo', 'neogeo', ('fbneo',), suffixes='.ZIP .7Z .CUE .CHD')
        plan = self.plan()
        launchers = tomllib.loads(plan['profiles'])['launchers']
        self.assertEqual(len(launchers), 2)
        primary, cd = launchers
        self.assertEqual(primary['root'], cd['root'])
        self.assertEqual(primary['extensions'], ['.7z', '.cue', '.zip'])
        self.assertEqual(cd['extensions'], ['.chd'])
        self.assertEqual(set(primary['extensions']) & set(cd['extensions']), set())
        self.assertEqual(cd['id'], primary['id']+'-neocd')
        self.assertNotEqual(primary['working_directory'], cd['working_directory'])
        self.assertIn(str(self.home/'.config/retroarch/cores/neocd_libretro.so'), cd['arguments'])
        self.assertIn('Neo Geo CD', plan['report']['included'][1]['routing'])
        self.assertFalse(plan['report']['physical_tested'])

    def test_missing_neocd_never_sends_chd_to_fbneo_and_reports_omission(self):
        self.installed('fbneo')
        self.system('neogeo', 'neogeo', ('fbneo',), suffixes='.zip .chd')
        plan = self.plan()
        launchers = tomllib.loads(plan['profiles'])['launchers']
        self.assertEqual(len(launchers), 1)
        self.assertEqual(launchers[0]['extensions'], ['.zip'])
        self.assertEqual(plan['report']['skipped'][0]['extensions'], ['.chd'])
        self.assertIn('neocd', plan['report']['skipped'][0]['reason'])

    def test_isolation_keeps_bios_inherited_and_disables_global_writes_and_network(self):
        self.installed()
        self.system()
        plan = self.plan()
        config = tomllib.loads(next(iter(plan['append_configs'].values())))
        self.assertNotIn('system_directory', config)
        for key in ('config_save_on_exit', 'auto_overrides_enable', 'auto_remaps_enable',
                    'savestate_auto_save', 'savestate_auto_load', 'cheevos_enable',
                    'builtin_imageviewer_enable', 'builtin_mediaplayer_enable'):
            self.assertEqual(config[key], 'false')
        for key, value in config.items():
            if value != 'false': self.assertTrue(Path(value).is_relative_to(self.data))

    def test_private_materialization_refuses_overwrite_before_any_other_file_is_written(self):
        self.installed()
        self.system()
        plan = self.plan()
        output, report = self.root/'profiles.toml', self.root/'report.json'
        output.write_text('preserve original')
        with self.assertRaises(FileExistsError): profiles.write_plan(plan, output, report)
        self.assertFalse(self.data.exists())
        self.assertEqual(output.read_text(), 'preserve original')
        output.unlink()
        profiles.write_plan(plan, output, report)
        for path in (output, report, *map(Path, plan['append_configs'])):
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
        for path in map(Path, plan['directories']):
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700)
        with self.assertRaises(FileExistsError): profiles.write_plan(plan, output, report)

    def test_invalid_base_config_or_non_executable_frontend_is_skipped(self):
        self.installed()
        self.system()
        (self.home/'.config/retroarch/retroarch.cfg').write_text('not a config assignment')
        self.assertFalse(self.plan()['report']['included'])
        self.installed()
        (self.binary/'retroarch').chmod(0o600)
        self.assertFalse(self.plan()['report']['included'])

    def test_dtd_xml_and_data_under_rom_root_are_rejected(self):
        self.systems.write_text('<!DOCTYPE systemList [<!ENTITY secret SYSTEM "file:///etc/passwd">]><systemList/>')
        with self.assertRaises(ValueError):
            profiles.build_plan(self.systems, self.home, self.library, self.data, self.binary)
        self.systems.write_text('<systemList/>')
        with self.assertRaises(ValueError):
            profiles.build_plan(self.systems, self.home, self.library, self.library/'unsafe', self.binary)


if __name__ == '__main__':
    unittest.main()
