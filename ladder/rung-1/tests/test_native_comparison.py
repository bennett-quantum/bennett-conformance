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
