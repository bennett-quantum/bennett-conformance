"""Consumer preparation handoff; the gate also runs a real buffered-return probe."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate
from process_preparation import audit_preparation


class ProcessPreparation(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.g = gate.Gate(argparse.Namespace(output=tmp.name, timeout=5))
        self.g.driver_exe = self.root/'driver'
        self.g.plugin_build = self.root/'plugin/build'
        self.runtime, self.arith = self.root/'runtime.bc', self.root/'arith.bc'
        self.runtime.write_bytes(b'runtime')
        self.arith.write_bytes(b'arith')
        self.g.runtime_files = lambda m: (self.runtime, self.arith)

    def receipt(self, major, workload):
        d = self.g.workdir(major, workload)
        (d/'input.bc').write_bytes(b'ordinary main')
        out = d/'process'
        out.mkdir(exist_ok=True)
        def item(path, data=None):
            if data is not None:
                path.write_bytes(data)
            return {'path':str(path), 'sha256':gate.sha(path)}
        module = item(out/'process-prepared.bc', b'prepared with init/fini')
        preparation = dict(schema='bennett-driver-process-preparation-1', entry='main',
            module=module, runtimes=item(self.runtime),
            wrapper=item(out/'process-entry.bc', b'wrapper'),
            report=item(out/'process-entry.json', b'{}'), tool=item(self.root/'tool', b'tool'))
        manifest = dict(schema='bennett-driver-build-1', status='prepared', llvm_major=major,
            dialect='0.3-r1', isa='0.9', inputs=[item(d/'input.bc')],
            runtimes=item(self.runtime), arith=item(self.arith), process_preparation=preparation,
            outputs={'process_prepared':module}, process_initialisation={
                'module':item(out/'process-init-fini.bc', b'prepared with init/fini'),
                'report':item(out/'process-init-fini.json', b'{}')})
        (out/'manifest.json').write_text(json.dumps(manifest))
        return d, out, manifest

    def test_both_workloads_consume_driver_preparation_before_lto(self):
        for major in (18, 23):
            for workload in gate.WORKLOADS:
                d, out, manifest = self.receipt(major, workload)
                calls = []
                self.g.command = lambda tag, argv, **kw: calls.append(list(map(str, argv)))
                self.g.runtime_link(major, workload)
                self.g.postlink(major, workload)
                self.assertEqual(calls[0], [str(self.g.driver_exe),
                    '--bennett-tier=reversible', '--bennett-entry=main',
                    '--bennett-emit=process-bitcode', '-bennett-dialect=0.3-r1', '-bennett-isa=0.9',
                    *self.g.driver_publications(major, self.runtime, self.arith),
                    '-bennett-out='+str(out), str(d/'input.bc')])
                self.assertEqual(calls[1], [f'/usr/lib/llvm-{major}/bin/opt',
                    '-passes=lto<O3>', '-S', str(out/'process-prepared.bc'), '-o', str(d/'postlink.ll')])
                self.assertEqual(self.g.manifest['process_preparation'][str(major)][workload]
                    ['preparation'], manifest['process_preparation'])

    def test_missing_or_changed_preparation_never_reaches_lto(self):
        d, out, original = self.receipt(18, 'hello')
        for key, value in (('status','running'), ('llvm_major',23), ('dialect','0.2.13'),
                           ('isa','0.8'), ('inputs',[])):
            bad = dict(original, **{key:value})
            (out/'manifest.json').write_text(json.dumps(bad))
            with self.subTest(key=key), self.assertRaises(AssertionError):
                audit_preparation(out, d/'input.bc', self.runtime, self.arith, 18, self.g.contract)
        for field in ('process_preparation', 'process_initialisation'):
            bad = copy.deepcopy(original)
            bad[field]['module']['sha256'] = '0'*64
            (out/'manifest.json').write_text(json.dumps(bad))
            with self.subTest(field=field), self.assertRaisesRegex(AssertionError, 'hash differs'):
                audit_preparation(out, d/'input.bc', self.runtime, self.arith, 18, self.g.contract)
        (out/'manifest.json').write_text(json.dumps(original))
        (out/'process-prepared.bc').write_bytes(b'unprepared source')
        with self.assertRaisesRegex(AssertionError, 'hash differs'):
            audit_preparation(out, d/'input.bc', self.runtime, self.arith, 18, self.g.contract)


if __name__ == '__main__':
    unittest.main()
