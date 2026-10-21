"""Targeted guards for Hito 2.5; pixel checks do not certify the physical LCD."""
import json,re,sys,unittest,xml.etree.ElementTree as ET
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools'))
from daycycle_assets import catalog,FOLDER,rgba
from room_materials import render

def polygons(path):
    tokens=re.findall(r'[MLHVZ]|-?\d+(?:\.\d+)?',path)
    out=[];current=[];i=0;x=y=0
    while i<len(tokens):
        cmd=tokens[i];i+=1
        if cmd in ('M','L'):x,y=map(float,tokens[i:i+2]);i+=2;current.append((x,y))
        elif cmd=='H':x=float(tokens[i]);i+=1;current.append((x,y))
        elif cmd=='V':y=float(tokens[i]);i+=1;current.append((x,y))
        elif cmd=='Z':out.append(current);current=[]
        else:raise AssertionError(cmd)
    return out

def inside(point,polygon):
    x,y=point;hit=False
    for (a,b),(c,d) in zip(polygon,polygon[1:]+polygon[:1]):
        if (b>y)!=(d>y) and x<(c-a)*(y-b)/(d-b)+a:hit=not hit
    return hit

class AmbientPolish(unittest.TestCase):
    def test_room_light_mask_protects_glass_and_lantern_core(self):
        tree=ET.fromstring((FOLDER/'interior-light-mask.svg').read_text())
        path=tree.find('{http://www.w3.org/2000/svg}path')
        self.assertEqual(path.attrib['fill-rule'],'evenodd')
        regions=polygons(path.attrib['d'])
        def painted(p):return sum(inside(p,r) for r in regions)%2==1
        for point in [(400,150),(316,150),(575,298)]:self.assertFalse(painted(point),point)
        for point in [(560,260),(470,365),(120,200)]:self.assertTrue(painted(point),point)

    def test_panels_reveal_more_without_changing_focus_or_corner_geometry(self):
        for row in catalog():
            colors=row['colors'];palette=tuple(rgba(colors[k]) for k in ['surface_tint','surface_bottom','surface_border','focus_accent'])
            old=render(64,64,16,palette=palette)
            focused=render(64,64,16,palette=palette,focused=True)
            with Image.open(FOLDER/'materials'/row['id']/'room-material.png') as panel:
                self.assertLess(panel.getpixel((32,8))[3],old.getpixel((32,8))[3])
                self.assertGreater(panel.getpixel((32,32))[3],210)
                self.assertEqual(panel.getpixel((0,0))[3],0)
            with Image.open(FOLDER/'materials'/row['id']/'room-material-focus.png') as panel:
                # Lanczos mixes the changed fill into the boundary by one 8-bit
                # quantum; keep both effective focus rows opaque and unchanged in hue.
                for y in [0,1]:
                    self.assertGreater(panel.getpixel((32,y))[3],250)
                    self.assertLessEqual(max(abs(a-b) for a,b in zip(panel.getpixel((32,y)),focused.getpixel((32,y)))),1)

    def test_weather_has_distinct_bounded_density_and_no_extra_animation(self):
        r=json.loads((FOLDER/'responses.json').read_text())
        cloudy,rain,fog,snow=map(rgba,r['weather_window_tints'][1:])
        self.assertLess(cloudy[3],rain[3]);self.assertLess(rain[3],snow[3]);self.assertLess(snow[3],fog[3])
        self.assertEqual(rgba(r['interior_day_tints'][7])[3],0)
        assets=['interior-light-mask','weather-window','rain-window','snow-window','halloween-table','christmas-table','new-year-table']
        self.assertLess(sum((FOLDER/(n+'.svg')).stat().st_size for n in assets),12000)
        for name in assets:
            source=(FOLDER/(name+'.svg')).read_text();ET.fromstring(source)
            self.assertNotIn('<animate',source);self.assertNotIn('<image',source)
        source=(ROOT/'app/ui/integrated.slint').read_text()
        self.assertNotIn('background:root.tint;',source)

if __name__=='__main__':unittest.main()
