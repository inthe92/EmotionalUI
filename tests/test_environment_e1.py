"""Static production/geometry contracts, in addition to actual Slint captures."""
import json
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
import daycycle_assets as assets


class EnvironmentE1(unittest.TestCase):
    def test_four_seasons_and_unknown_never_add_snow(self):
        seasons=json.loads((assets.FOLDER/'seasons.json').read_text())
        self.assertEqual(seasons['order'],['winter','spring','summer','autumn','unknown'])
        self.assertEqual(len(set(seasons['vegetation_tints'][:4])),4)
        self.assertEqual(assets.rgba(seasons['vegetation_tints'][4])[3],0)
        self.assertEqual(len(seasons['daylight_strength']),8)
        self.assertLess(seasons['daylight_strength'][6],seasons['daylight_strength'][3])
        source=(ROOT/'app/ui/integrated.slint').read_text()
        snow=source.split('// Weather only:',1)[1].split('// Original small vector ornament',1)[0]
        self.assertIn('if root.weather==4:',snow)
        self.assertNotIn('root.season',snow)

    def test_generated_palette_has_interruptible_color_inputs(self):
        source=(assets.FOLDER/'assets.slint').read_text()
        self.assertEqual(source,assets.slint_source(assets.catalog()))
        self.assertIn('from-colors',source)
        self.assertIn('.mix(',source)
        motion=(ROOT/'app/src/ambient_motion.rs').read_text()
        self.assertIn('get_ambient_generation() != generation',motion)
        self.assertIn('ui.get_reduced()',motion)

    def test_own_mask_and_private_caches_stay_separate(self):
        mask=(assets.FOLDER/'vegetation-mask.svg').read_text()
        self.assertIn('viewBox="0 0 640 480"',mask)
        self.assertNotIn('<image',mask)
        self.assertNotIn('http:',mask.split('>',1)[1])
        self.assertIn('*.weather-cache.toml',(ROOT/'.gitignore').read_text())


if __name__=='__main__':unittest.main()
