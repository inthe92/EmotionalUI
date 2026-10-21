import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from analyze_pipeline_probe import analyze


def log(stamps,pending=0):
    raw=','.join(f'{i}:{t}' for i,t in enumerate(stamps))
    return ('PIPELINE_MOVE_BEGIN id=1 kernel_us=1000000 duration_ms=360 from=-192 target=640\n'
            'PIPELINE_MOVE_END id=1 kernel_us=1360000 offset=640 source=after_render_before_submit\n'
            f'KMS_DRM present_calls=5 kernel_confirmed={len(stamps)} pending_or_unobserved={pending} submit_errors=0 receive_errors=0 timestamp_monotonic=Some(true)\n'
            f'KMS_DRM_TRACE format=sequence:kernel_us retained={len(stamps)} dropped=0 stamps=[{raw}]\n')


class PipelineProbe(unittest.TestCase):
    def test_presentations_not_requested_frames_or_timer_rate(self):
        r=analyze(log([1001000,1017667,1034334,1361000]))
        self.assertAlmostEqual(r['pooled_update_hz'],1000*3/360)
        self.assertFalse(r['sixty_fps_certified'])
        self.assertIsNone(r['pooled_intervals']['p99_ms'])

    def test_missing_last_confirmation_does_not_complete_motion(self):
        r=analyze(log([1001000,1017667],pending=1))
        self.assertEqual(r['groups'][0]['coverage'],'INCOMPLETE')
        self.assertIsNone(r['pooled_update_hz'])
        self.assertEqual(r['pending_not_counted'],1)

    def test_invalid_timestamps_and_clock_rejected(self):
        for text in [log([20,10]),log([1001000]).replace('Some(true)','Some(false)')]:
            with self.assertRaises(ValueError):analyze(text)

class PipelineSafety(unittest.TestCase):
    def test_manifest_recovery_and_unchanged_swap_policy(self):
        import hashlib,json,tempfile,importlib.util
        from unittest.mock import patch
        darkos=Path(__file__).resolve().parents[1]/'platform/darkos'
        sys.path.insert(0,str(darkos))
        import gpu_probe_preview
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);files={}
            for name in ['emotional-ui','gpu-pipeline-probe']:
                data=b'plan-only fixture';(root/name).write_bytes(data);files[name]=hashlib.sha256(data).hexdigest()
            (root/'phase12-manifest.json').write_text(json.dumps({'kind':'phase12-private-preview','files':files}))
            with patch.dict('os.environ',{},clear=True):
                args=gpu_probe_preview.plan(root,'emotional-ui-phase12-preview-pipeline-test',24,'gpu-pipeline-probe')
            self.assertIn(str(root/'gpu-pipeline-probe'),args)
            self.assertTrue(any('ExecStopPost=' in arg for arg in args))
            self.assertTrue(any('RuntimeMaxSec=39' in arg for arg in args))
            self.assertTrue(any('EMOTIONAL_PERF_TRACE=1' in arg for arg in args))
            with patch.dict('os.environ',{'EMOTIONAL_GPU_SWAP':'immediate'}):
                with self.assertRaisesRegex(ValueError,'unchanged swap'):gpu_probe_preview.plan(root,'emotional-ui-phase12-preview-pipeline-test',24,'gpu-pipeline-probe')
