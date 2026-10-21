"""Focused daily art/token contracts; no hardware or human approval claims."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
import daycycle_assets as cycle
import room_materials as materials


class DaycycleArt(unittest.TestCase):
    def test_seven_ids_neutral_and_unique_token_families(self):
        rows = cycle.catalog()
        self.assertEqual([r['id'] for r in rows], cycle.IDS)
        for token in ('focus_accent', 'surface_tint', 'primary_text', 'secondary_text'):
            self.assertEqual(len({r['colors'][token] for r in rows[:7]}), 7)
        self.assertEqual(rows[7]['scene'], rows[5]['scene'])

    def test_generated_bindings_are_current_and_clamped(self):
        source = (cycle.FOLDER/'assets.slint').read_text()
        self.assertEqual(source, cycle.slint_source(cycle.catalog()))
        self.assertIn('max(0,min(7,self.palette-index))', source)
        self.assertIn('crate::ambient_motion::update(', (ROOT/'app/src/state.rs').read_text())
        self.assertIn('.set_palette_index(blend.from as i32)', (ROOT/'app/src/ambient_motion.rs').read_text())
        integrated = (ROOT/'app/ui/integrated.slint').read_text()
        for name in ('ink', 'muted', 'mark', 'material', 'dock', 'status'):
            self.assertIn('DaycycleAssets.'+name, integrated)

    def test_materials_match_palette_and_keep_geometry(self):
        for row in cycle.catalog():
            c = row['colors']
            palette = tuple(cycle.rgba(c[k]) for k in ('surface_tint', 'surface_bottom', 'surface_border', 'focus_accent'))
            for name, (w, h, radius) in materials.SURFACES.items():
                effective = (cycle.rgba(c['capsule_tint']), *palette[1:]) if name == 'room-status' else palette
                if name=='room-material':
                    profile=json.loads((cycle.FOLDER/'responses.json').read_text())['panel_profile']
                    effective=materials.panel_palette(palette,profile)
                expected = materials.render(w, h, radius, palette=effective)
                with Image.open(cycle.FOLDER/'materials'/row['id']/(name+'.png')) as actual:
                    self.assertEqual(actual.tobytes(), expected.tobytes(), (row['id'], name))
            folder = cycle.FOLDER/'materials'/row['id']
            with Image.open(folder/'room-action-focus.png') as focused:
                for y in (0, 1):
                    self.assertGreater(focused.getpixel((94, y))[3], 200)
            self.assertNotEqual((folder/'room-dock-selection-active.png').read_bytes(),
                                (folder/'room-dock-selection-focus.png').read_bytes())

    def test_private_room_exports_and_provenance(self):
        folder = cycle.FOLDER/'selected-room-v1'
        manifest = json.loads((folder/'manifest.json').read_text())
        if not (folder/'anochecer.png').exists():
            self.skipTest('Private reference-derived room withheld; requires authorized local assets')
        self.assertEqual(len(manifest['resources']), 7)
        self.assertEqual(manifest['master'], 'anochecer.png')
        signatures = set()
        for item in manifest['resources']:
            path = folder/(item['id']+'.png')
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), item['sha256'])
            with Image.open(path) as image:
                self.assertEqual(image.size, (640, 480))
                self.assertEqual(image.mode, 'RGB')
            signatures.add(item['sha256'])
        self.assertEqual(len(signatures), 7)
        self.assertIn('redistribución', (folder/'PROVENANCE.md').read_text())


if __name__ == '__main__':
    unittest.main()
