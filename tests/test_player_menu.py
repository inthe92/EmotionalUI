"""Private functional entry planning; no console, network, game or tools executed."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
for path in [ROOT/'platform/distribution', ROOT/'tools', ROOT/'platform/darkos']:
    sys.path.insert(0, str(path))
spec = importlib.util.spec_from_file_location('player_menu', ROOT/'platform/darkos/player_menu.py')
player = importlib.util.module_from_spec(spec)
spec.loader.exec_module(player)


class PlayerMenu(unittest.TestCase):
    def test_installed_library_is_data_and_ambiguous_roots_fail(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            cfg = home/'.emulationstation/es_systems.cfg'
            cfg.parent.mkdir()
            ports = home/'roms/ports'
            ports.mkdir(parents=True)
            cfg.write_text('<systemList><system><name>ports</name><path>'+str(ports)+'</path><command>must never execute</command></system></systemList>')
            config, library, _ = player.installed_systems(home)
            self.assertEqual(config,cfg)
            self.assertEqual(library,ports.parent.resolve())
            original = cfg.read_text()
            cfg.write_text(original.replace('<systemList>','<!DOCTYPE dangerous><systemList>'))
            with self.assertRaises(ValueError): player.installed_systems(home)

    def test_owned_configuration_reuses_identical_bytes_and_preserves_modification(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            own = base/'private'
            config = own/'isolation.cfg'
            output = own/'session/launchers.toml'
            plan = {'directories':[str(own)],'append_configs':{str(config):'private=true\n'},'profiles':'handoff="process-exit"\n'}
            player.materialize(plan,output)
            player.materialize(plan,output)
            self.assertEqual(config.stat().st_mode & 0o777,0o600)
            config.write_text('user change')
            with self.assertRaises(ValueError): player.materialize(plan,output)
            self.assertEqual(config.read_text(),'user change')

    def test_functional_plan_has_gpu_recovery_and_no_demo_or_game_timeout(self):
        root = Path('/private-fixture')
        command = player.session_command(root/'bundle',root/'launchers.toml',root/'data',root/'roms',root/'runtime','a'*32)
        self.assertIn('--functional',command)
        self.assertNotIn('--trial',command)
        self.assertIn('--launcher-config',command)
        self.assertFalse(any('RuntimeMaxSec' in a for a in command))
        self.assertTrue(any('ExecStopPost=' in a for a in command))
        self.assertTrue(any('EMOTIONAL_MATERIAL_GPU=1' in a for a in command))
        self.assertEqual(command[command.index('--atlas')+1],str(root/'bundle/emotional-atlas.db'))
        self.assertIn('--atlas-experimental',command)
        self.assertIn('--property=ReadOnlyPaths='+str(root/'roms'),command)
        self.assertEqual(command[-2:],['--quit-file',str(root/'runtime/quit.request')])


if __name__ == '__main__':
    unittest.main()
