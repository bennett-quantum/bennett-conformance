"""A killed worker must leave its last complete checkpoint readable."""
import json
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]

INTERRUPTED_WORKER = r'''
import json, os, runpy, signal, subprocess, sys
from pathlib import Path
from types import SimpleNamespace
worker, evidence, native = map(Path, sys.argv[1:])
sys.path.insert(0, str(worker.parent))
response = {'payload':0, 'code':0, 'status':'REVERSED', 'equals_entry':True,
            'frames_clean':True, 'first_difference':None,
            'forward_state':{'0'*20:[1,0]}, 'state':{'0'*20:[1,0]},
            'wires':20, 'in':[0], 'mut':[0,0]}
# Only the transport is stubbed: worker equality/reversal assertions still run.
subprocess.run = lambda *a, **kw: SimpleNamespace(returncode=0,
    stdout=json.dumps(response).encode(), stderr=b'')
write_text = Path.write_text
checkpoint = evidence.stem+'-summary.json'
def interrupted_write(path, data, *args, **kwargs):
    if path.name.startswith(checkpoint):
        write_text(path, data[:5], *args, **kwargs)
        os.kill(os.getpid(), signal.SIGKILL)
    return write_text(path, data, *args, **kwargs)
Path.write_text = interrupted_write
sys.argv = [str(worker), '--executor', 'stubbed-executor', '--object', 'unused',
            '--native', str(native), '--evidence', str(evidence),
            '--budget', '256,0,32,32', '--arity', '1']
runpy.run_path(str(worker), run_name='__main__')
'''


class CheckpointPublication(unittest.TestCase):
    def test_completed_publication_replaces_the_checkpoint(self):
        spec = importlib.util.spec_from_file_location('worker_progress', HERE/'progress.py')
        progress = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(progress)
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
            checkpoint = Path(temp)/'summary.json'
            progress.write_summary(checkpoint, {'cases':7})
            progress.write_summary(checkpoint, {'cases':8})
            self.assertEqual(json.loads(checkpoint.read_text()), {'cases':8})
            self.assertEqual(list(checkpoint.parent.iterdir()), [checkpoint])

    def test_interrupted_workers_preserve_the_previous_checkpoint(self):
        for worker in ('full_domain_worker.py', 'input_worker.py'):
            with self.subTest(worker=worker), tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
                root = Path(temp)
                native = root/'native.bin'
                native.write_bytes(bytes(256))
                evidence = root/'cases.jsonl'
                checkpoint = root/'cases-summary.json'
                previous = json.dumps({'cases':7, 'faults':0, 'reversed':7})+'\n'
                checkpoint.write_text(previous)
                result = subprocess.run([sys.executable, '-B', '-c', INTERRUPTED_WORKER,
                    str(HERE/worker), str(evidence), str(native)], capture_output=True, timeout=10)
                self.assertEqual(result.returncode, -signal.SIGKILL, result.stderr.decode())
                self.assertEqual(checkpoint.read_text(), previous)


if __name__ == '__main__':
    unittest.main()
