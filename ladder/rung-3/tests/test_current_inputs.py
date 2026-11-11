"""Rung inputs must select frozen implementations and published native support."""
import argparse
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate


class CurrentInputs(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.g = gate.Gate(argparse.Namespace(output=str(self.root/'runs'), timeout=5,
            jobs=2, isa_spec_rev='frozen-isa', reference=None))

    def test_default_reference_builds_the_frozen_isa_release_binary(self):
        source = self.root/'isa-publication'
        executable = source/'target/release/bennett-isa-ref'
        frozen, commands = [], []
        def freeze(repo, revision):
            frozen.append((repo, revision))
            self.g.manifest['pins'][repo] = revision
            return source
        def command(tag, argv, **kwargs):
            commands.append(list(map(str, argv)))
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b'frozen reference binary')
        self.g.freeze, self.g.command = freeze, command
        self.g.reference()
        self.assertEqual(frozen, [('bennett-isa-spec', 'frozen-isa')])
        self.assertEqual(commands, [['cargo', 'build', '--offline', '--release',
            '--jobs', '2', '--manifest-path', str(source/'reference-interpreter/Cargo.toml'),
            '--target-dir', str(source/'target')]])
        self.assertEqual(self.g.executor('reference'), executable)
        self.assertEqual(self.g.manifest['reference']['sha256'], gate.sha(executable))
        self.assertEqual(self.g.manifest['reference']['selection'], 'pinned source')

    def test_explicit_reference_override_is_recorded_without_a_build(self):
        executable = self.root/'explicit-reference'
        executable.write_bytes(b'explicit reference')
        self.g.args.reference = str(executable)
        self.g.freeze = lambda *a: self.fail('explicit reference froze a default source')
        self.g.command = lambda *a, **kw: self.fail('explicit reference triggered a build')
        self.g.reference()
        self.assertEqual(self.g.executor('reference'), executable)
        self.assertEqual(self.g.manifest['reference']['selection'], 'explicit binary')
        self.assertEqual(self.g.manifest['reference']['sha256'], gate.sha(executable))

    def test_native_comparison_receives_the_pinned_same_major_support(self):
        self.g.plugin_build = self.root/'plugin/build'
        self.g.driver_exe = self.root/'driver'
        self.g.publications = {'bennett-runtimes': self.root/'published-runtime'}
        self.g.runtime_files = lambda major: (self.root/'runtime.bc', self.root/'arith.bc')
        for major in (18, 23):
            for workload in gate.WORKLOADS:
                calls = []
                def command(tag, argv, **kwargs):
                    calls.append(list(map(str, argv)))
                    raise gate.GateError('stop at driver invocation')
                self.g.command = command
                with self.assertRaisesRegex(gate.GateError, 'stop at driver invocation'):
                    self.g.native(major, workload)
                support = self.g.publications['bennett-runtimes']/f'dist/llvm{major}/musl-sysroot/lib/native-support.bc'
                self.assertIn('--bennett-native-support='+str(support), calls[0])

    def test_default_sysroot_is_from_the_frozen_runtime_publication(self):
        for major in (18, 23):
            expected = self.g.tmp/('rung3-bennett-runtimes-'+gate.PINS['bennett-runtimes'])/f'dist/llvm{major}/musl-sysroot'
            self.assertEqual(self.g.sysroots[major], expected)


if __name__ == '__main__':
    unittest.main()
