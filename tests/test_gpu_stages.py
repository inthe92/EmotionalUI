import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from analyze_gpu_stages import analyze


def log(rows):
    return ('GPU_REPLAY plan=fixed-v1 epoch_kernel_us=1000000;\n'
            'KMS_GPU_STAGES format=request:begin:swap:waited:swapped:submitted unit=monotonic_us redraws=2 coalesced=0 dropped=0 rows='+repr(rows)+'\n')


class GpuStages(unittest.TestCase):
    def test_separates_cpu_wall_stages_without_claiming_gpu_completion(self):
        a=analyze(log([[11000000,11001000,11011000,11012000,11032000,11033000]]))
        m=a['windows']['interaction']['metrics']
        self.assertEqual(m['traversal_commands_until_swap']['median_ms'],10)
        self.assertEqual(m['previous_flip_wait']['median_ms'],1)
        self.assertEqual(m['egl_swap']['median_ms'],20)
        self.assertEqual(a['scope'],'CPU_WALL_TIME_NOT_GPU_EXECUTION')
        self.assertEqual(a['windows']['idle']['frames'],0)

    def test_rejects_missing_duplicate_or_nonmonotonic_records(self):
        for value in ['',log([])+log([]),log([[11000000,11001000,11011000,11010000,11032000,11033000]])]:
            with self.assertRaises(ValueError):analyze(value)

    def test_no_request_and_sparse_idle_do_not_fabricate_latency(self):
        a=analyze(log([[0,31001000,31011000,31012000,31032000,31033000]]))
        self.assertEqual(a['windows']['idle']['metrics']['request_to_render']['n'],0)
        self.assertIsNone(a['windows']['idle']['metrics']['egl_swap']['p99_ms'])
