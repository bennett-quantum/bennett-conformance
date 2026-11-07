"""Consumer glue only: these tests never establish rung acceptance."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
import gate


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR'))
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.g = gate.Gate(argparse.Namespace(output=str(self.root), timeout=5))

    def test_literal_formatted_oracle(self):
        expected = json.loads((HERE/'fixtures/printf_fmt.expect.json').read_text())
        # Independent arithmetic: signed range -2^31..2^31-1; unsigned 2^32-1.
        self.assertEqual(''.join(row['stdout'] for row in expected['lines']),
            'text=musl signed=-42 unsigned=42 hex=2a char=Z\n'
            'width=[      xy][    -7][     9][    ab][  Q]\n'
            'precision=[abc][-00012][00023][00ab]\n'
            'extremes=-2147483648,2147483647,4294967295,ffffffff zero=[]\n'
            'buffered=tail:-1:0:10:!')
        self.assertTrue(all(row['derivation'] for row in expected['lines']))
        self.assertEqual(expected['exit_status'], 0)
        source = (HERE/'fixtures/printf_fmt.c').read_text()
        for fmt in ('%s', '%d', '%u', '%x', '%c'):
            self.assertIn(fmt, source)
        self.assertIn('int main(void)', source)
        self.assertNotIn('fflush', source)
        self.g.fixtures()

    def test_every_workload_runs_both_executors_and_majors(self):
        calls = []
        for method in ('fixtures', 'plugin', 'vm', 'driver', 'frontend', 'runtime_link',
                       'postlink', 'pipeline', 'artifacts', 'native', 'admission',
                       'execute', 'replay', 'runtime_audit', 'driver_probe',
                       'isa_contract', 'control', 'replay_probe'):
            setattr(self.g, method, lambda *a, _method=method, **kw:
                    calls.append((_method, a, kw)))
        self.g.finish = lambda: 0
        self.assertEqual(self.g.main(), 0)
        for m in (18, 23):
            for workload in ('hello', 'printf_fmt'):
                for executor in ('reference', 'VM'):
                    for suffix in ('forward = native', 'reset', 'tape replay', 'corrupt-tape rejection'):
                        row = gate.workload_stage(workload, executor+' '+suffix)
                        self.assertEqual(self.g.rows[m][row]['status'], 'PASS')
                    self.assertIn(('execute', (m, executor), {'workload':workload}), calls)
                    self.assertIn(('replay', (m, executor), {'workload':workload, 'tampered':True}), calls)
        self.assertNotEqual(self.g.workdir(18, 'hello'), self.g.workdir(18, 'printf_fmt'))

    def test_contract_is_derived_from_pinned_publication_and_producer(self):
        for dialect, isa in (('0.2.10', '0.8'), ('0.2.15', '0.9'), ('0.3', '0.9')):
            pins = {'dialect':dialect, 'isa':isa}
            producer = f'dialect = "{dialect}"\nisa = "{isa}"\n'
            spec = f'**Versioning.** Dialect {dialect}: markers 0.2; targets ISA {isa}.'
            if isa == '0.8':
                spec += '\nOn ISA 0.8 a module has verdict blocked:BD-B-ISA7'
                with self.assertRaisesRegex(gate.GateError, 'BD-B-ISA7'):
                    gate.validate_contract(pins, producer, spec)
            else:
                self.assertEqual(gate.validate_contract(pins, producer, spec), (dialect, isa))
            for bad in ({'dialect':'99', 'isa':isa}, {'dialect':dialect, 'isa':'1.0'}):
                with self.assertRaisesRegex(gate.GateError, 'contract'):
                    gate.validate_contract(bad, producer, spec)

    def test_contract_mismatch_in_publication_or_producer_is_loud(self):
        pins = {'dialect':'0.2.15', 'isa':'0.9'}
        for producer, spec in (
            ('dialect="0.2.15"\nisa="0.8"', '**Versioning.** Dialect 0.2.15: targets ISA 0.9.'),
            ('dialect="0.2.15"\nisa="0.9"', '**Versioning.** Dialect 0.2.15: targets ISA 0.8.'),
            ('dialect="0.2.15"\nisa="0.9"', 'no version declaration'),
        ):
            with self.assertRaisesRegex(gate.GateError, 'contract'):
                gate.validate_contract(pins, producer, spec)

    def test_selected_versions_reach_production_and_driver_probe(self):
        self.g.contract = ('0.2.15', '0.9')
        self.g.plugin_build = self.root/'plugin/build'
        self.g.driver_exe = self.root/'driver'
        self.g.runtime_files = lambda m: (self.root/f'runtime{m}.bc', self.root/f'arith{m}.bc')
        calls = []
        def command(tag, argv, **kwargs):
            calls.append(list(map(str, argv)))
            raise gate.GateError('stop after argv')
        self.g.command = command
        for m in (18, 23):
            for method in ('pipeline', 'driver_probe'):
                with self.assertRaisesRegex(gate.GateError, 'stop after argv'):
                    getattr(self.g, method)(m)
                self.assertIn('-bennett-dialect=0.2.15', calls[-1])
                self.assertIn('-bennett-isa=0.9', calls[-1])
            for arg in (f'--bennett-llvm-major={m}',
                        f'--bennett-runtimes={self.root}/runtime{m}.bc',
                        f'--bennett-arith={self.root}/arith{m}.bc',
                        f'--bennett-plugin={self.root}/plugin/build/libbennett-plugin-{m}.so'):
                self.assertIn(arg, calls[-1])

    def test_blockers_recognize_workload_paths_in_any_checkout(self):
        for workload in ('', 'hello/', 'printf_fmt/'):
            error = f'18/{workload}production: exit 3\nbennett: blocked:BD-B-ISA7 typed fault'
            def fail():
                raise gate.GateError(error)
            self.g.stage(18, 'probe', fail)
            self.assertEqual(self.g.rows[18]['probe']['beads'], ['bp-is0'])
        error = ('23/driver-probe: exit 1\nBD-E-INPUT: hash '+str(gate.CONFORMANCE)+
                 '/.tmp/bennett-runtimes/dist/llvm23/bennett-runtimes.bc: No such file or directory')
        self.g.stage(23, 'probe', fail)
        self.assertEqual(self.g.rows[23]['probe']['beads'], ['bennett-conformance-igc'])

    def test_missing_buffered_tail_fails_every_executor(self):
        for major in (18, 23):
            for workload in ('hello', 'printf_fmt'):
                expected = bytes.fromhex(self.g.expected(workload)['stdout_hex'])
                d = self.g.workdir(major, workload)
                (d/'native.stdout').write_bytes(expected)
                self.g.argv_replies = lambda *a: Path('/argv-replies.json')
                self.g.executor = lambda name: Path('/executor')
                for executor in ('reference', 'VM'):
                    value = {'fault':None, 'faults':[], 'host_requests':[],
                        'results':[{'type':'i32','value':'0'}], 'status':'COMPLETED',
                        'run_state':'CLOSED', 'console_hex':expected.hex(), 'effect_tape_length':1,
                        'tape':[{'dev':'console','op':'write','args':[4096,len(expected)],'bytes':expected.hex()}]}
                    self.g.command = lambda *a, **kw: json.dumps(value).encode()
                    self.g.execute(major, executor, workload=workload)
                    self.assertEqual(self.g.forward_results[(major,executor,workload)], value)
                    truncated = expected.rsplit(b'\n', 1)[0]
                    value['console_hex'] = truncated.hex()
                    value['tape'][0].update(bytes=truncated.hex(), args=[4096,len(truncated)])
                    with self.assertRaisesRegex(AssertionError, 'tape bytes differ'):
                        self.g.execute(major, executor, workload=workload)

    def receipt(self, m):
        d = self.g.workdir(m, 'hello'); out = d/'out'; out.mkdir(exist_ok=True)
        paths = [out/'device.bc', self.root/'publication/runtime.bc', self.root/'publication/arith.bc']
        for path, data in zip(paths, (b'device', b'runtime', b'arith')):
            path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(data)
        self.g.publications = {'bennett-runtimes':self.root/'publication'}
        support = self.root/'publication/native-support.bc'; support.write_bytes(b'support')
        layout = out/'native-layout.ll'
        layout.write_text('target triple = "x86_64-unknown-linux-musl"\n'
            '@__bennett_rt_native_layout = constant { i64, i64, i64 } { i64 1, i64 4096, i64 65536 }, align 8\n')
        script = out/'comparison.ld'
        script.write_text('SECTIONS { . = 4096; .data : { *(.data .data.*) } '
                          'ASSERT(ALIGN(., 4096) == 4096, "heap") . = 4194304; .text : { *(.text*) } }')
        merged = out/'comparison-linked.bc'; merged.write_bytes(b'merged')
        final = out/'comparison.bc'; final.write_bytes(b'closed')
        prefix = f'/usr/lib/llvm-{m}/bin/'
        steps = [
            [prefix+'llvm-link', *map(str, paths), '--only-needed', '-o', str(merged)],
            [prefix+'llvm-as', str(layout), '-o', str(out/'native-layout.bc')],
            [prefix+'llvm-link', str(merged), str(out/'native-layout.bc'),
             '--override='+str(support), '-o', str(out/'comparison-adapted.bc')],
            [prefix+'llvm-link', str(out/'comparison-adapted.bc'), *map(str, paths[1:]),
             '--only-needed', '-o', str(final)],
            [prefix+'opt', '-passes=strip-dead-prototypes,verify', str(final), '-o', str(final)],
        ]
        record = {'llvm_major':m, 'materialised_sha256':gate.sha(paths[0]),
                  'runtimes_sha256':gate.sha(paths[1]), 'arith_sha256':gate.sha(paths[2]),
                  'native_support_sha256':gate.sha(support), 'linked_sha256':gate.sha(merged),
                  'bitcode_link_argv':steps[0],
                  'link_argv':[prefix+'clang', '-O2', '-nostdlib', '-nostdinc', '-static',
                      '-no-pie', '-fno-merge-all-constants', str(final), '-Wl,-T,'+str(script),
                      '-Wl,-e,_start', '-Wl,--no-undefined', '-Wl,--fatal-warnings',
                      '-Wl,-Map,'+str(out/'native.map'), '-o', str(d/'native')]}
        manifest = {'commands':[{'argv':argv, 'success':True} for argv in steps]}
        return paths, record, manifest, d/'native'

    def test_auditable_multistage_receipt(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            calls = []
            self.g.command = lambda tag, argv, **kw: calls.append(list(map(str,argv))) or b''
            self.assertEqual(audit_comparison(self.g, m, *paths, record, manifest, output), record['link_argv'])
            self.assertEqual(calls, [c['argv'] for c in manifest['commands']])

    def test_receipt_rejects_host_libc_missing_runtime_and_hash_substitution(self):
        from native_audit import audit_comparison
        paths, record, manifest, output = self.receipt(18)
        mutations = []
        for extra in ('/usr/lib/libc.a', '-lc', '-Wl,-lc', '-Xlinker', '@response'):
            r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
            r['link_argv'].insert(-2, extra); mutations.append((r, mf))
        for extra in ('/usr/lib/libc.a', '-L/usr/lib', '--override=/usr/lib/libm.bc'):
            r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
            mf['commands'][0]['argv'].insert(1, extra); mutations.append((r, mf))
        r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
        mf['commands'][0]['argv'].remove(str(paths[1])); mutations.append((r, mf))
        r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
        substitute = self.root/'publication/substitute.bc'; substitute.write_bytes(b'wrong input')
        mf['commands'][0]['argv'][2] = str(substitute); mutations.append((r, mf))
        r = copy.deepcopy(record); r['arith_sha256'] = '0'*64; mutations.append((r, manifest))
        self.g.command = lambda *a, **kw: b''
        for r, mf in mutations:
            with self.subTest(receipt=r, manifest=mf), self.assertRaises((AssertionError, gate.GateError)):
                audit_comparison(self.g, 18, *paths, r, mf, output)
        paths[1].write_bytes(b'tampered runtime')
        with self.assertRaisesRegex((AssertionError, gate.GateError), 'hash'):
            audit_comparison(self.g, 18, *paths, record, manifest, output)

    def test_linker_script_cannot_import_libc(self):
        from native_audit import audit_comparison
        paths, record, manifest, output = self.receipt(18)
        script = paths[0].parent/'comparison.ld'
        self.g.command = lambda *a, **kw: b''
        for text in ('INPUT(/usr/lib/libc.a)', 'INCLUDE host.ld', 'SECTIONS { .text : { libc.a(*) } }'):
            script.write_text(text)
            with self.assertRaisesRegex((AssertionError, gate.GateError), 'script'):
                audit_comparison(self.g, 18, *paths, record, manifest, output)


if __name__ == '__main__':
    unittest.main()
