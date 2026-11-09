"""Self-check the diagnostic R input; these tests do not establish rung acceptance."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import unittest


FIXTURES = Path(__file__).resolve().parents[1] / 'fixtures'
FIXTURE = FIXTURES / 'effect-console-r.ll'


class EffectConsoleFixtureTests(unittest.TestCase):
    def test_target_matches_both_musl_frontends(self):
        # Pinned dialect 0.3-r1 §§1.7/19.8: each input declares the link target.
        source = FIXTURE.read_text()
        for major in (18, 23):
            with self.subTest(major=major):
                frontend = subprocess.run([
                    f'/usr/lib/llvm-{major}/bin/clang',
                    '--target=x86_64-unknown-linux-musl', '-x', 'c',
                    '-S', '-emit-llvm', '-o', '-', '-'],
                    input='', text=True, capture_output=True, check=True).stdout
                for field in ('triple', 'datalayout'):
                    pattern = rf'^target {field} = "([^"]+)"$'
                    expected = re.findall(pattern, frontend, re.MULTILINE)
                    self.assertEqual(len(expected), 1, f'clang {major}: {field}')
                    self.assertEqual(re.findall(pattern, source, re.MULTILINE),
                                     expected, f'LLVM {major}: missing/mismatched {field}')

    def test_verifies_with_both_llvm_majors(self):
        for major in (18, 23):
            with self.subTest(major=major):
                result = subprocess.run([
                    f'/usr/lib/llvm-{major}/bin/opt', '-passes=verify',
                    '-disable-output', str(FIXTURE)], text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_console_body_is_unchanged(self):
        body = [line for line in FIXTURE.read_text().splitlines()
                if line and not line.startswith((';', 'target '))]
        self.assertEqual(body, [
            '@msg = private unnamed_addr constant [3 x i8] c"hi\\0A"',
            'declare void @__bennett_effect_console_write(ptr, i64)',
            'define i32 @main() {',
            '  call void @__bennett_effect_console_write(ptr @msg, i64 3)',
            '  ret i32 0',
            '}',
        ])

    def test_fixture_hash_matches_manifest(self):
        hashes = json.loads((FIXTURES / 'sha256.json').read_text())
        self.assertEqual(hashlib.sha256(FIXTURE.read_bytes()).hexdigest(),
                         hashes[FIXTURE.name])


if __name__ == '__main__':
    unittest.main()
