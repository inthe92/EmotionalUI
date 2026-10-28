"""Focused glass-card assets; pixel evidence does not certify the physical LCD."""
import importlib.util
import json
from pathlib import Path
import unittest

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('card_materials', ROOT / 'tools/room_materials.py')
materials = importlib.util.module_from_spec(spec)
spec.loader.exec_module(materials)


class CardFocus(unittest.TestCase):
    def variants(self):
        yield ROOT / 'assets/illustrations'
        yield from sorted((ROOT / 'assets/scenes/daycycle/materials').iterdir())

    def test_transparent_interior_is_preserved_in_every_palette(self):
        for folder in self.variants():
            for name in ('room-material', 'room-tile'):
                with self.subTest(folder=folder.name, card=name):
                    with Image.open(folder / f'{name}.png') as normal, Image.open(folder / f'{name}-focus.png') as focused:
                        self.assertEqual(normal.size, focused.size)
                        self.assertEqual(normal.crop((20, 20, 44, 44)).tobytes(), focused.crop((20, 20, 44, 44)).tobytes())

    def test_focus_is_visible_colored_and_two_pixels_wide(self):
        for folder in self.variants():
            for name in ('room-material', 'room-tile'):
                with self.subTest(folder=folder.name, card=name):
                    with Image.open(folder / f'{name}-focus.png') as focused:
                        for y in (0, 1):
                            self.assertGreater(focused.getpixel((32, y))[3], 220)
                        left, right = focused.getpixel((0, 32)), focused.getpixel((63, 32))
                        self.assertGreater(left[1], left[0] + 60)
                        self.assertGreater(right[2], right[1] + 60)
                        self.assertGreater(right[0], left[0] + 40)

    def test_rounded_exterior_has_no_corner_spikes(self):
        for folder in self.variants():
            for name in ('room-material', 'room-tile'):
                with Image.open(folder / f'{name}-focus.png') as focused:
                    alpha = focused.getchannel('A')
                    for xy in ((0, 0), (63, 0), (0, 63), (63, 63)):
                        self.assertEqual(alpha.getpixel(xy), 0)
                    self.assertIsNone(ImageChops.difference(alpha, alpha.transpose(Image.Transpose.FLIP_LEFT_RIGHT)).getbbox())

    def test_generated_palette_matches_runtime_asset(self):
        folder = ROOT / 'assets/scenes/daycycle'
        profile = json.loads((folder / 'responses.json').read_text())['panel_profile']
        for row in json.loads((folder / 'palettes.json').read_text())['moments']:
            def rgba(key):
                value = row['colors'][key].lstrip('#')
                return tuple(bytes.fromhex(value + ('ff' if len(value) == 6 else '')))
            palette = materials.panel_palette(tuple(rgba(key) for key in ('surface_tint', 'surface_bottom', 'surface_border', 'focus_accent')), profile)
            for name, radius in (('room-material', 16), ('room-tile', 12)):
                generated = materials.render(64, 64, radius, palette=palette, focused=True, stroke_width=2, fantasy_focus=True)
                with Image.open(folder / 'materials' / row['id'] / f'{name}-focus.png') as actual:
                    self.assertEqual(generated.tobytes(), actual.tobytes())
