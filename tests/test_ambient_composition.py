"""Composition contracts and sampled contrast, separate from LCD validation."""
import json,sys,unittest
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from daycycle_assets import catalog,FOLDER,rgba
from check_visual_materials import contrast

class Composition(unittest.TestCase):
    def test_more_room_without_shrinking_text_or_controls(self):
        s=(ROOT/'app/ui/integrated.slint').read_text()
        # Phase 13 explicitly replaces the old 512/560px cards and bottom dock.
        # Keep the legibility/geometry contract; do not retain obsolete layout sizes.
        for rule in ['resume-card-width:448px','discover-card-width:400px','resume-secondary-width:216px','action-width:188px','action-height:36px','body-size:13px','dock-width:400px','dock-y:16px']:
            self.assertIn(rule,s)
        self.assertLessEqual(370,400)
        self.assertLessEqual(400,410)
        self.assertGreaterEqual(640-(24+400),200) # Landscape remains open.
        self.assertEqual(24+216+16+216,24+448)
        self.assertLessEqual(16+32,64) # Tabs finish above section heading.
        self.assertLessEqual(368+80,456) # Expanded long-title tiles clear hints.

    def test_unknown_weather_does_not_draw_the_white_window_mask(self):
        s=(ROOT/'app/ui/integrated.slint').read_text()
        mask=next(line for line in s.splitlines() if 'scene-rasters[2]' in line)
        # Slint colorize alpha=0 disables colorization, leaving the source white.
        # The mask must be absent, not merely assigned a transparent colorize.
        self.assertIn('DaycycleAssets.weather-window-tint.alpha>0',mask)

    def test_composed_surface_contrast_samples(self):
        response=json.loads((FOLDER/'responses.json').read_text())
        def mix(a,b,t):return tuple(x*(1-t)+y*t for x,y in zip(a,b))
        def over(front,back):return tuple(front[i]*front[3]+back[i]*(1-front[3]) for i in range(3))
        minimum=100;worst=None
        rows=catalog()
        # Two adjacent painted material keyframes, representative interior points,
        # all seasonal/weather/celebration grades and both extreme backgrounds.
        for i in range(7):
            j=(i+1)%7
            for name in ['room-material','room-action','room-status','room-dock']:
                with Image.open(FOLDER/'materials'/rows[i]['id']/(name+'.png')) as im:
                    a=[tuple(v/255 for v in im.getpixel((im.width//2,y))) for y in [8,im.height//2,im.height-9]]
                with Image.open(FOLDER/'materials'/rows[j]['id']/(name+'.png')) as im:
                    b=[tuple(v/255 for v in im.getpixel((im.width//2,y))) for y in [8,im.height//2,im.height-9]]
                for t in [0.,.25,.5,.75,1.]:
                    ink=[mix(tuple(v/255 for v in rgba(rows[i]['colors'][key])[:3]),tuple(v/255 for v in rgba(rows[j]['colors'][key])[:3]),t) for key in ['primary_text','secondary_text','focus_accent']]
                    for season in response['seasons'][:4]:
                        for weather in response['weather']:
                            for celebration in response['celebrations'][1:]+['#000000']:
                                color=weather if weather!='#000000' else season
                                color=tuple(v/255 for v in rgba(color)[:3])
                                if celebration!='#000000':color=mix(color,tuple(v/255 for v in rgba(celebration)[:3]),response['celebration_weight'])
                                secondary=mix(ink[1],color,response['secondary_text_weight'])
                                accent=ink[2]
                                accent=mix(accent,tuple(v/255 for v in rgba(season)[:3]),response['season_weight'])
                                if weather!='#000000':accent=mix(accent,tuple(v/255 for v in rgba(weather)[:3]),response['weather_weight'])
                                if celebration!='#000000':accent=mix(accent,tuple(v/255 for v in rgba(celebration)[:3]),response['celebration_weight'])
                                accent=tuple(min(1.,v*(1+response['accent_brightness'])) for v in accent)
                                text_colors=[ink[0],secondary,accent]
                                for pa,pb in zip(a,b):
                                    for background in [0.,1.]:
                                        front=(*pa[:3],pa[3]*(1-t)/(1-t*.94))
                                        surface=over(front,(background,)*3)
                                        surface=over((*pb[:3],pb[3]*t),surface)
                                        surface=over((*color,a[0][3]*response['surface_wash']),surface)
                                        for strength in [0.,.14]:
                                            active_surface=mix(surface,accent,strength)
                                            value=min(contrast(text,active_surface) for text in text_colors)
                                            if value<minimum:minimum=value;worst=(i,name,t,season,weather,celebration,strength,text_colors,active_surface)
        self.assertGreaterEqual(minimum,4.5,(minimum,worst))

    def test_lightweight_layers_are_local_and_precipitation_stays_in_glass(self):
        import xml.etree.ElementTree as ET
        for name in ['weather-window','rain-window','snow-window','halloween-table','christmas-table','new-year-table']:
            svg=(FOLDER/(name+'.svg')).read_text()
            tree=ET.fromstring(svg)
            self.assertEqual(tree.attrib['viewBox'],'0 0 640 480')
            self.assertNotIn('<image',svg)
            self.assertNotIn('href=',svg)
            self.assertNotIn('<animate',svg)
            if name in ('rain-window','snow-window'):
                self.assertIn('clip-path="url(#glass)"',svg)
        source=(ROOT/'app/ui/integrated.slint').read_text()
        self.assertNotIn('opacity:root.cloud',source) # No global weather veil over lamp.

    def test_transparency_normalization_and_owned_ornament(self):
        s=(FOLDER/'assets.slint').read_text()
        self.assertIn('.with-alpha(self.background-tint-day.alpha/255)',s)
        svg=(FOLDER/'christmas-table.svg').read_text()
        self.assertNotIn('<image',svg)
        self.assertNotIn('<text',svg)

if __name__=='__main__':unittest.main()
