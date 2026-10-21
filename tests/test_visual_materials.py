"""640x480 material regressions; pixel checks are not physical-LCD approval."""
import importlib.util
from pathlib import Path
import unittest
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('room_materials', ROOT/'tools/room_materials.py')
materials = importlib.util.module_from_spec(spec)
spec.loader.exec_module(materials)


class MaterialPixels(unittest.TestCase):
    def test_dimensions_radii_and_transparent_exterior(self):
        self.assertEqual(materials.STROKE, 2)
        for name, (w, h, radius) in materials.SURFACES.items():
            self.assertTrue(w % 2 == h % 2 == 0)
            if name in ('room-dock', 'room-action', 'room-status'):
                self.assertEqual(radius * 2, h)
            else:
                self.assertIn(radius, (12, 16))
            with Image.open(ROOT/'assets/illustrations'/f'{name}.png') as image:
                self.assertEqual(image.size, (w, h))
                self.assertEqual(image.mode, 'RGBA')
                self.assertEqual(image.getpixel((0, 0))[3], 0)
                edge = [image.getpixel((x, 0))[3] for x in range(radius + 1)]
                self.assertTrue(any(0 < a < 200 for a in edge), (name, edge))

    def test_focus_is_two_pixels_not_a_hairline(self):
        for name, (w, h, _) in materials.SURFACES.items():
            if name == 'room-dock':
                continue  # Dock focus belongs to its destinations, not its frame.
            with Image.open(ROOT/'assets/illustrations'/f'{name}-focus.png') as image:
                for y in (0, 1):
                    self.assertGreater(image.getpixel((w//2, y))[0], 200)
                self.assertLess(image.getpixel((w//2, 4))[0], 100)

    def test_active_and_focused_destinations_remain_distinct(self):
        folder = ROOT/'assets/illustrations'
        with Image.open(folder/'room-dock-selection-active.png') as active, Image.open(folder/'room-dock-selection-focus.png') as focus, Image.open(folder/'room-dock-selection-active-focus.png') as both:
            for image in (active, focus, both):
                self.assertEqual(image.size, (80, 44))
            self.assertGreater(active.getpixel((40, 22))[3], 0)
            self.assertEqual(focus.getpixel((40, 22))[3], 0)
            self.assertGreater(focus.getpixel((40, 0))[3], 200)
            self.assertGreater(both.getpixel((40, 22))[3], 0)

    def test_baked_frames_are_not_overlaid_with_native_contours(self):
        source = (ROOT/'app/ui/integrated.slint').read_text()
        for component, end in [('AmbientPanel', 'component GameTitle'), ('AmbientAction', 'component CompactGame')]:
            body = source.split('component '+component+' inherits Rectangle {', 1)[1].split(end, 1)[0]
            self.assertIn('border-width:0px;', body)
            self.assertNotIn('border-color:', body)

    def test_inset_resume_cover_keeps_compact_radius(self):
        source = (ROOT/'app/ui/integrated.slint').read_text()
        cover = next(line for line in source.splitlines() if 'if self.cover-visible: EditorialArtwork' in line)
        self.assertIn('radius:AtmosphereDesign.radius-s;', cover)
        self.assertNotIn('radius-l - AtmosphereDesign.pad', cover)
