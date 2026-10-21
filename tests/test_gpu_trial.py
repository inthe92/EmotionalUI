"""Synthetic trace-parser tests; these do not claim hardware performance."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from analyze_gpu_trial import analyze, statistics


def log(stamps, pending=0, dropped=0, completions='', actions=''):
    trace = ','.join(f'{i}:{t}' for i, t in enumerate(stamps))
    return (f'GPU_REPLAY plan=fixed-v1 epoch_kernel_us=1000000\n{actions}{completions}'
            f'KMS_DRM present_calls=3 kernel_confirmed={len(stamps)+dropped} pending_or_unobserved={pending} submit_errors=0 receive_errors=0 timestamp_monotonic=Some(true)\n'
            f'KMS_DRM_TRACE format=sequence:kernel_us retained={len(stamps)} dropped={dropped} stamps=[{trace}]\n')


class GpuTrial(unittest.TestCase):
    def test_statistics_never_treat_empty_idle_as_sixty_fps(self):
        self.assertIsNone(statistics([])['confirmed_update_hz'])
        self.assertEqual(statistics([0, 20000, 40000])['confirmed_update_hz'], 50)
        self.assertIsNone(statistics([0, 20000])['p99_ms'])
        self.assertEqual(statistics(list(range(0, 101000, 1000)))['p99_ms'], 1)

    def test_idle_and_interaction_windows_are_separate(self):
        result = analyze(log([11000000, 11020000, 31000000, 31010000]))
        self.assertEqual(result['windows']['interaction']['confirmations'], 2)
        self.assertEqual(result['windows']['interaction']['confirmed_update_hz'], 50)
        self.assertEqual(result['windows']['stable_idle']['confirmations'], 2)

    def test_pending_and_dropped_events_are_not_fabricated(self):
        result = analyze(log([11000000, 11020000], pending=1, dropped=3))
        self.assertEqual(result['retained_confirmations'], 2)
        self.assertEqual(result['status'], 'PARTIAL')
        self.assertTrue(any('unconfirmed' in w for w in result['warnings']))

    def test_malformed_timestamp_and_interrupted_trace_are_invalid(self):
        self.assertEqual(analyze(log([120, 110]))['status'], 'INVALID')
        self.assertEqual(analyze(log([120]).replace('0:120]', '0:oops]'))['status'], 'INVALID')
        self.assertEqual(analyze(log([120]).replace('0:120]', '0:120'))['status'], 'INVALID')

    def test_interrupted_actions_use_latest_generation_completion(self):
        actions = ('GPU_REPLAY event=Action(Next) scheduled_ms=10000 observed_ms=10000 motion_generation=1\n'
                   'GPU_REPLAY event=Action(Previous) scheduled_ms=10080 observed_ms=10080 motion_generation=2\n')
        complete = 'GPU_MOTION_COMPLETE generation=2 kernel_us=11440000 requested_ms=360 observed_ms=360\n'
        result = analyze(log([11010000, 11020000, 11450000], completions=complete, actions=actions))
        group = result['animation_groups'][0]
        self.assertEqual(group['actions'], 2)
        self.assertEqual(group['coverage'], 'COMPLETE')
        self.assertEqual(group['confirmations'], 3)

    def test_absent_final_flip_keeps_animation_unmeasured(self):
        actions = 'GPU_REPLAY event=Action(Next) scheduled_ms=10000 observed_ms=10000 motion_generation=1\n'
        complete = 'GPU_MOTION_COMPLETE generation=1 kernel_us=11360000 requested_ms=360 observed_ms=360\n'
        result = analyze(log([11010000], pending=1, completions=complete, actions=actions))
        self.assertEqual(result['animation_groups'][0]['coverage'], 'INCOMPLETE')
        self.assertEqual(result['animation_fps'], 'UNMEASURED')

    def test_early_timer_is_disclosed_and_actual_time_is_preserved(self):
        actions = 'GPU_REPLAY event=Action(Next) scheduled_ms=10000 observed_ms=9998 motion_generation=1\n'
        result = analyze(log([10999000, 11010000], actions=actions))
        self.assertEqual(result['action_schedule_error_ms'], [-2])
        self.assertEqual(result['status'], 'PARTIAL')
        self.assertTrue(any('before scheduled deadline' in w for w in result['warnings']))
        self.assertEqual(result['errors'], [])


class PreviewRendererSelection(unittest.TestCase):
    def setUp(self):
        import importlib.util
        spec=importlib.util.spec_from_file_location('gpu_preview',Path(__file__).resolve().parents[1]/'platform/darkos/phase12_preview.py')
        self.preview=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.preview)

    def test_software_remains_default_and_gpu_is_explicit_external_binary(self):
        a=self.preview.command('/tmp/own','emotional-ui-phase12-preview-gpu-test',False)
        b=self.preview.command('/tmp/own','emotional-ui-phase12-preview-gpu-test',False,renderer='femtovg')
        self.assertIn('/tmp/own/emotional-ui',a)
        self.assertNotIn('/tmp/own/emotional-ui-gpu',a)
        self.assertIn('/tmp/own/emotional-ui-gpu',b)
        self.assertIn('--trial',b)
        self.assertTrue(any('ExecStopPost=' in x for x in b))
        self.assertTrue(any('SLINT_BACKEND=linuxkms-femtovg ' in x for x in b))

    def test_unknown_renderer_and_environment_are_not_executed(self):
        from unittest.mock import patch
        with self.assertRaises(ValueError):
            self.preview.command('/tmp/own','emotional-ui-phase12-preview-gpu-test',False,renderer='unknown')
        with patch.dict('os.environ',{'EMOTIONAL_GPU_REPLAY':'bad;value','EMOTIONAL_PERF_TRACE':'1'}):
            a=self.preview.command('/tmp/own','emotional-ui-phase12-preview-gpu-test',False)
        self.assertFalse(any('bad;value' in x for x in a))
        self.assertTrue(any('EMOTIONAL_PERF_TRACE=1' in x for x in a))


if __name__ == '__main__':
    unittest.main()
