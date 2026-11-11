"""Native comparison must retain materialised definitions and link missing helpers."""
import argparse
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location('comparison_gate', HERE / 'gate.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class NativeComparisonLink(unittest.TestCase):
    def test_producer_requests_the_comparison_layout(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
            root = Path(temp)
            g = gate.Gate(argparse.Namespace(output=str(root / 'evidence'), timeout=300))
            directory = g.folder('C', 18) / 'r1_lookup/out'
            directory.mkdir(parents=True)
            for name in ('ref.bennett', 'materialised.bc'):
                (directory / name).write_text('producer output placeholder')
            g.plugin_build = root / 'plugin'
            g.runtime_files = lambda m: (root / 'runtime.bc', root / 'arith.bc')
            commands = []
            g.command = lambda tag, argv: commands.append(list(map(str, argv)))
            g.pipeline('C', 18, 'r1_lookup')
            self.assertIn('--bennett-layout-data-base=131072', commands[0])
            self.assertIn('--bennett-layout-text-base=8388608', commands[0])

    def test_lookup_native_table_matches_the_device_global_base(self):
        for major in (18, 23):
            with self.subTest(major=major), tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
                root = Path(temp)
                source = root / 'materialised.ll'
                source.write_text('''@table = constant [8 x i8] [i8 3, i8 -7, i8 11, i8 0, i8 127, i8 -128, i8 42, i8 -1]
define i8 @r1_lookup(i8 %x) {
  %i = and i8 %x, 7
  %p = getelementptr [8 x i8], ptr @table, i8 0, i8 %i
  %v = load i8, ptr %p
  ret i8 %v
}
''')
                empty = root / 'empty.ll'
                empty.write_text('; no helpers needed\n')
                for path in (source, empty):
                    subprocess.run([f'/usr/lib/llvm-{major}/bin/llvm-as', str(path),
                                    '-o', str(path.with_suffix('.bc'))], check=True, capture_output=True)
                g = gate.Gate(argparse.Namespace(output=str(root / 'evidence'), timeout=300))
                directory = g.folder('C', major) / 'r1_lookup'
                (directory / 'out').mkdir(parents=True)
                (directory / 'out/materialised.bc').write_bytes(source.with_suffix('.bc').read_bytes())
                (directory / 'out/ref.bennett').write_text('section layout\n{"globals":{"G1":{"base":131072}}}\n')
                expected = bytes([3, 249, 11, 0, 127, 128, 42, 255] * 32)
                (g.folder('C', major) / 'r1_lookup-native.bin').write_bytes(expected)
                g.runtime_files = lambda m: (empty.with_suffix('.bc'), empty.with_suffix('.bc'))
                g.entries[('C', major, 'r1_lookup')] = 'r1_lookup'
                g.comparison('C', major, 'r1_lookup')
                self.assertEqual((directory / 'native-comparison-run.stdout').read_bytes(), expected)

    def test_materialised_runtime_definition_does_not_link_twice(self):
        for major in (18, 23):
            with self.subTest(major=major), tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
                root = Path(temp)
                modules = {
                    'materialised': """define i8 @__bennett_rt_ctpop_i8(i8 %x) { ret i8 %x }
declare i8 @needed_delta(i8)
declare i8 @arith_adjust(i8)
define i8 @r1_popcount(i8 %x) {
  %a = call i8 @__bennett_rt_ctpop_i8(i8 %x)
  %b = call i8 @needed_delta(i8 %a)
  %c = call i8 @arith_adjust(i8 %b)
  ret i8 %c
}
""",
                    'runtime': """define i8 @__bennett_rt_ctpop_i8(i8 %x) { ret i8 %x }
define i8 @needed_delta(i8 %x) {
  %a = xor i8 %x, 85
  ret i8 %a
}
""",
                    'arith': """define i8 @arith_adjust(i8 %x) {
  %a = add i8 %x, 3
  ret i8 %a
}
"""}
                for name, text in modules.items():
                    source = root / (name + '.ll')
                    source.write_text(text)
                    subprocess.run([f'/usr/lib/llvm-{major}/bin/llvm-as', str(source),
                                    '-o', str(source.with_suffix('.bc'))], check=True, capture_output=True)
                g = gate.Gate(argparse.Namespace(output=str(root / 'evidence'), timeout=300))
                directory = g.folder('C', major) / 'r1_popcount'
                (directory / 'out').mkdir(parents=True)
                (directory / 'out/materialised.bc').write_bytes((root / 'materialised.bc').read_bytes())
                (directory / 'out/ref.bennett').write_text('section layout\n{"globals":{}}\n')
                # Hand-derived composition: identity, XOR 0x55, then addition mod 256.
                expected = bytes(((x ^ 0x55) + 3) & 0xff for x in range(256))
                (g.folder('C', major) / 'r1_popcount-native.bin').write_bytes(expected)
                g.runtime_files = lambda m: (root / 'runtime.bc', root / 'arith.bc')
                g.entries[('C', major, 'r1_popcount')] = 'r1_popcount'
                g.comparison('C', major, 'r1_popcount')
                self.assertEqual((directory / 'native-comparison-run.stdout').read_bytes(), expected)
                self.assertFalse((directory / 'native-comparison-undefined.stdout').read_bytes().strip())


if __name__ == '__main__':
    unittest.main()
