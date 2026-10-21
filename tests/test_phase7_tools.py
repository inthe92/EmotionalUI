"""Cross-file resource and design contracts; no hardware claims."""
import struct,tomllib,unittest,os
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
class ResourceContracts(unittest.TestCase):
    @unittest.skipUnless(os.environ.get("EMOTIONAL_PRIVATE_SCENE_TESTS")=="1", "Pending optional legacy-art validation: original PNG excluded; requires authorized private resources and EMOTIONAL_PRIVATE_SCENE_TESTS=1")
    def test_one_opaque_master_and_six_moments(self):
        folder=ROOT/'assets/scenes/room-v1';m=tomllib.loads((folder/'manifest.toml').read_text())
        self.assertEqual((m['width'],m['height']),(640,480))
        phases=m['moments'];self.assertEqual(sorted(p['phase'] for p in phases),list(range(6)))
        self.assertEqual(len({p['id'] for p in phases}),6)
        self.assertEqual(len(list(folder.glob('*.png'))),1)
        data=(folder/m['master']).read_bytes();self.assertEqual(struct.unpack('>II',data[16:24]),(640,480));self.assertEqual(data[25],2)
        self.assertLessEqual(len(data),1024*1024)
        window=m['window'];self.assertLessEqual(window['x']+window['width'],640);self.assertLessEqual(window['y']+window['height'],m['viewport_height'])
    def test_original_checkpoint_resource(self):
        import importlib.util,zlib
        folder=ROOT/'assets/scenes/checkpoint-room-v1';m=tomllib.loads((folder/'manifest.toml').read_text())
        spec=importlib.util.spec_from_file_location('checkpoint_background',ROOT/'tools/checkpoint_background.py')
        generator=importlib.util.module_from_spec(spec);spec.loader.exec_module(generator)
        data=(folder/m['master']).read_bytes()
        self.assertEqual(data,generator.render())
        self.assertEqual(struct.unpack('>II',data[16:24]),(640,480));self.assertEqual(data[24:26],bytes((8,2)))
        self.assertEqual(sorted(p['phase'] for p in m['moments']),list(range(6)))
        self.assertTrue((folder/m['reading_mask']).is_file())
        chunks=[];i=8
        while i<len(data):
            size=struct.unpack('>I',data[i:i+4])[0];kind=data[i+4:i+8];payload=data[i+8:i+8+size]
            self.assertEqual(zlib.crc32(kind+payload)&0xffffffff,struct.unpack('>I',data[i+8+size:i+12+size])[0])
            if kind==b'IDAT':chunks.append(payload)
            i+=12+size
        self.assertEqual(len(zlib.decompress(b''.join(chunks))),480*(640*3+1))
    def test_active_original_room_is_embedded_opaque_and_documented(self):
        import hashlib
        folder=ROOT/'assets/scenes/original-room-v1'
        manifest=tomllib.loads((folder/'manifest.toml').read_text())
        source=folder/manifest['master']
        data=source.read_bytes()
        self.assertEqual(data[:8],b'\x89PNG\r\n\x1a\n')
        self.assertEqual(struct.unpack('>II',data[16:24]),(640,480))
        self.assertEqual(data[24:26],bytes((8,2)))
        self.assertIn(hashlib.sha256(source.read_bytes()).hexdigest(),(folder/'PROVENANCE.md').read_text())
        assets=(ROOT/'assets/scenes/daycycle/assets.slint').read_text()
        self.assertIn('selected-room-v1/anochecer.png',assets)
        self.assertEqual(assets.split('backgrounds:[',1)[1].split('];',1)[0].count('@image-url('),8) # seven bases + neutral; material arrays are separate
        self.assertIn('DaycycleAssets.backgrounds',(ROOT/'app/ui/integrated.slint').read_text())
        self.assertEqual(sorted(p['phase'] for p in manifest['moments']),list(range(6)))
    def test_quiet_text_and_focus_contrast(self):
        # Explicit text surface, independent of scene brightness and palette interpolation.
        import re
        s=(ROOT/'docs/visual-correction/before/integrated.slint').read_text()
        c={key:value for key,value in re.findall(r'property <color> (paper|ink|muted|mark):#([0-9a-f]{6})',s)}
        def lum(h):
            rgb=[int(h[i:i+2],16)/255 for i in (0,2,4)]
            return sum((v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4)*w for v,w in zip(rgb,(.2126,.7152,.0722)))
        for key in ('ink','muted','mark'):
            a,b=sorted((lum(c[key]),lum(c['paper'])))
            self.assertGreaterEqual((b+.05)/(a+.05),4.5)
    @unittest.skipUnless(os.environ.get("EMOTIONAL_PRIVATE_SCENE_TESTS")=="1", "Pending optional legacy-art validation: original PNG excluded; requires authorized private resources and EMOTIONAL_PRIVATE_SCENE_TESTS=1")
    def test_reference_master_retains_six_moment_contract(self):
        folder=ROOT/'assets/scenes/lofi-room-v2';m=tomllib.loads((folder/'manifest.toml').read_text())
        self.assertEqual(m['viewport_height'],480)
        self.assertEqual(sorted(p['phase'] for p in m['moments']),list(range(6)))
        self.assertEqual(len(list(folder.glob('*.png'))),1)
        data=(folder/m['master']).read_bytes();self.assertEqual(struct.unpack('>II',data[16:24]),(640,480));self.assertEqual(data[25],2)
        self.assertTrue((folder/m['reading_mask']).is_file())
    def test_warm_glass_contrast_over_brightest_background(self):
        import json
        palettes=json.loads((ROOT/'assets/scenes/daycycle/palettes.json').read_text())['moments']
        def rgb(h):return [int(h[i:i+2],16)/255 for i in (1,3,5)]
        def lum(v):return sum((c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4)*w for c,w in zip(v,(.2126,.7152,.0722)))
        def ratio(a,b):
            lo,hi=sorted((lum(a),lum(b)));return (hi+.05)/(lo+.05)
        for row in palettes:
            colors=row['colors']
            alpha=int(colors['glass'][7:9],16)/255
            backgrounds=[rgb(colors['base'])]+[[c*alpha+b*(1-alpha) for c in rgb(colors['glass'])] for b in (0,1)]
            for key in ('primary_text','secondary_text','focus_accent'):
                for bg in backgrounds:self.assertGreaterEqual(ratio(rgb(colors[key]),bg),4.5,(row['id'],key))
            self.assertGreaterEqual(ratio(rgb(colors['base']),rgb(colors['focus_accent'])),4.5)
            self.assertGreaterEqual(ratio(rgb(colors['secondary_text']),[c*.8+.2 for c in rgb('#111916')]),4.5)
if __name__=='__main__':unittest.main()
