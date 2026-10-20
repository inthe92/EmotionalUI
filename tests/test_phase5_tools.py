import importlib.util,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('learn',Path(__file__).resolve().parents[1]/'tools/arm64/learn_input.py')
learn=importlib.util.module_from_spec(spec);spec.loader.exec_module(learn)
def fixture():
    events=[]
    for i in range(14):
        events.extend([{'ms':i*100,'kind':1,'code':304+i,'value':1},{'ms':i*100+50,'kind':1,'code':304+i,'value':0}])
    for i,(axis,value) in enumerate([(0,-1800),(0,1800),(1,-1800),(1,1800)]):
        events.extend([{'ms':1500+i*100,'kind':3,'code':axis,'value':value},{'ms':1550+i*100,'kind':3,'code':axis,'value':0}])
    return {'owned_frontend_observed':True,'device_name':'retrogame_joypad','key_capabilities':list(range(304,318)),'axes':{'0':{'min':-1800,'max':1800},'1':{'min':-1800,'max':1800}},'events':events}
class Calibration(unittest.TestCase):
    def test_partial_profile_does_not_invent_missing_controls(self):
        f=fixture();f['events']=f['events'][:26]
        buttons,axes=learn.learn_partial(f)
        self.assertEqual(len(buttons),13);self.assertEqual(axes,[])
        with self.assertRaises(ValueError):learn.learn(f)
    def test_completion_uses_ordered_segments_not_incompatible_clocks(self):
        f=fixture();prefix=dict(f,events=f['events'][:26])
        completion=dict(f,events=[dict(e,ms=e['ms']-1300) for e in f['events'][26:]])
        buttons,axes=learn.learn(learn.complete(prefix,completion))
        self.assertEqual(len(buttons),14);self.assertEqual(len(axes),2)
        self.assertEqual(prefix['events'],f['events'][:26])
    def test_completion_rejects_existing_button_and_missing_stick(self):
        f=fixture();prefix=dict(f,events=f['events'][:26])
        completion=dict(f,events=f['events'][26:28])
        with self.assertRaises(ValueError):learn.complete(prefix,completion)
        completion=dict(f,events=f['events'][24:])
        with self.assertRaises(ValueError):learn.complete(prefix,completion)
    def test_synthetic_capture_maps_order_and_stick(self):
        buttons,axes=learn.learn(fixture());self.assertEqual(buttons[4]['action'],'confirm');self.assertEqual(axes[0]['negative'],'left');self.assertEqual(len(buttons),14)
    def test_duplicate_press_is_rejected(self):
        f=fixture();f['events'][2]['code']=304
        with self.assertRaises(ValueError):learn.learn(f)
    def test_dropped_stream_is_rejected(self):
        f=fixture();f['events'].append({'kind':0,'code':3,'value':0,'ms':9999})
        with self.assertRaises(ValueError):learn.learn(f)
    def test_incomplete_stick_is_rejected(self):
        f=fixture();f['events']=f['events'][:-2]
        with self.assertRaises(ValueError):learn.learn(f)
    def test_original_frontend_capture_is_rejected(self):
        f=fixture();f['owned_frontend_observed']=False
        with self.assertRaises(ValueError):learn.learn(f)
if __name__=='__main__':unittest.main()
