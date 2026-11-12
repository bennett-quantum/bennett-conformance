"""Consumer glue only: these tests never establish rung acceptance."""
import argparse
import copy
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
import gate


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR'))
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.g = gate.Gate(argparse.Namespace(output=str(self.root), timeout=5))
        self.g.native_provider = self.root/'provider.so'
        self.g.native_target = self.root/'target.json'

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
        for method in ('fixtures', 'plugin', 'vm', 'driver', 'reference', 'frontend', 'runtime_link',
                       'postlink', 'pipeline', 'artifacts', 'native', 'admission',
                       'execute', 'replay', 'runtime_audit', 'driver_probe',
                       'isa_contract', 'control', 'replay_probe'):
            setattr(self.g, method, lambda *a, _method=method, **kw:
                    calls.append((_method, a, kw)))
        self.g.finish = lambda: 0
        self.assertEqual(self.g.main(), 0)
        for m in (18, 23):
            self.assertEqual(set(self.g.rows[m]), set(gate.STAGES))
            for workload in ('hello', 'printf_fmt'):
                for executor in ('reference', 'VM'):
                    for suffix in ('forward = native', 'reset', 'tape replay', 'corrupt-tape rejection'):
                        row = gate.workload_stage(workload, executor+' '+suffix)
                        self.assertEqual(self.g.rows[m][row]['status'], 'PASS')
                    self.assertIn(('execute', (m, executor), {'workload':workload}), calls)
                    self.assertIn(('replay', (m, executor), {'workload':workload, 'tampered':True}), calls)
        self.assertNotEqual(self.g.workdir(18, 'hello'), self.g.workdir(18, 'printf_fmt'))
        # A complete hello path cannot hide one failed formatted acceptance row.
        self.g.rows[18][gate.workload_stage('printf_fmt', 'native comparison')]['status'] = 'FAIL'
        with redirect_stdout(io.StringIO()):
            self.assertEqual(gate.Gate.finish(self.g), 1)
        self.assertEqual(self.g.rows[18]['native comparison']['status'], 'PASS')

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

    def test_contract_version_declaration_can_wrap_lines(self):
        # Pinned dialect 023f5a7 spec/Bennett-Dialect-Spec.md:14-15 wraps
        # between tdl and effects. Paragraph breaks still end the declaration.
        for dialect, isa in (('0.2.10', '0.8'), ('0.2.15', '0.9')):
            pins = {'dialect':dialect, 'isa':isa}
            producer = f'dialect="{dialect}"\nisa="{isa}"'
            spec = (f'**Versioning.** Dialect {dialect}: markers 0.2, tdl 0.2.4,\n'
                    f'effects 0.2.4, corpus 0.2; targets ISA {isa}.\n\n')
            with self.subTest(isa=isa):
                if isa == '0.8':
                    spec += 'On ISA 0.8 a module has verdict blocked:BD-B-ISA7'
                    with self.assertRaisesRegex(gate.GateError, 'BD-B-ISA7'):
                        gate.validate_contract(pins, producer, spec)
                else:
                    self.assertEqual(gate.validate_contract(pins, producer, spec), (dialect, isa))
                unrelated = spec.replace('; targets ISA ', ';\n\nOther paragraph targets ISA ')
                with self.assertRaisesRegex(gate.GateError, 'unknown or mismatched R contract'):
                    gate.validate_contract(pins, producer, unrelated)

    def test_accepted_r_profile_has_independent_identity(self):
        pins = {'dialect': '0.3-r1', 'isa': '0.9', 'mapping': '0.3',
                'normaliser': '0.2.8', 'replay': 'saved-tape-v1'}
        producer = ('dialect="0.2.13"\nisa="0.8"\n[profiles.r_contract]\n'
                    'dialect="0.3-r1"\nisa="0.9"\nmapping="0.3"\n'
                    'normaliser="0.2.8"\nreplay="saved-tape-v1"')
        spec = ('**Versioning.** Dialect 0.3: norm 0.2.8, map 0.3, asm 0.2; targets ISA 0.9.\n\n'
                '### 1.7 Classical R-contract profile (council-accepted)\n\n'
                '`0.3-r1` names a full dialect compatibility identity under §23 for the classical\n'
                'R-contract slice of this draft targeting ISA 0.9.')
        self.assertEqual(gate.validate_contract(pins, producer, spec), ('0.3-r1', '0.9'))
        for bad_spec in (spec.replace('council-accepted', 'ungraded'),
                         spec.replace('`0.3-r1` names', '`0.3-r2` names'),
                         spec.replace('targets ISA 0.9', 'targets ISA 0.8')):
            with self.subTest(spec=bad_spec), self.assertRaisesRegex(gate.GateError, 'contract'):
                gate.validate_contract(pins, producer, bad_spec)
        for field in ('dialect', 'isa', 'mapping', 'normaliser', 'replay'):
            bad_pins = dict(pins, **{field: 'unknown'})
            with self.subTest(field=field), self.assertRaisesRegex(gate.GateError, 'contract'):
                gate.validate_contract(bad_pins, producer, spec)
        with self.assertRaisesRegex(gate.GateError, 'contract'):
            gate.validate_contract(pins, 'dialect="0.3-r1"\nisa="0.9"', spec)

    def test_contract_mismatch_in_publication_or_producer_is_loud(self):
        pins = {'dialect':'0.2.15', 'isa':'0.9'}
        for producer, spec in (
            ('dialect="0.2.15"\nisa="0.8"', '**Versioning.** Dialect 0.2.15: targets ISA 0.9.'),
            ('dialect="0.2.15"\nisa="0.9"', '**Versioning.** Dialect 0.2.15: targets ISA 0.8.'),
            ('dialect="0.2.15"\nisa="0.9"', 'no version declaration'),
            ('dialect="0.2.15"\nisa="0.9"\n[specs]\n"bennett-isa-spec"="0.8"',
             '**Versioning.** Dialect 0.2.15: targets ISA 0.9.'),
        ):
            with self.assertRaisesRegex(gate.GateError, 'contract'):
                gate.validate_contract(pins, producer, spec)

    def test_archived_plugin_reads_verified_pin_cache(self):
        source = self.root/'archive'
        source.mkdir()
        workspace = self.root/'workspace'
        cache = workspace/'bennett-plugin/.tmp/gate/cache'
        cache.mkdir(parents=True)
        self.g.args.plugin_rev = 'frozen-plugin'
        self.g.freeze = lambda repo, rev: source
        def command(tag, argv, **kwargs):
            if tag == 'build/plugin-configure':
                archive_cache = source/'.tmp/gate/cache'
                self.assertTrue(archive_cache.is_symlink())
                self.assertEqual(archive_cache.resolve(), cache)
                raise gate.GateError('configured archive cache verified')
            return b''
        self.g.command = command
        with patch.object(gate, 'WORKSPACE', workspace):
            with self.assertRaisesRegex(gate.GateError, 'configured archive cache verified'):
                self.g.plugin()

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

    def test_contract_reads_the_effective_frozen_producer(self):
        self.g.args.plugin_rev = 'moving-ref'
        self.g.manifest['pins']['bennett-plugin'] = 'frozen-producer'
        calls = []
        def command(tag, argv, **kw):
            calls.append(list(map(str, argv)))
            return (b'**Versioning.** Dialect 0.2.10: targets ISA 0.8.\n'
                    b'On ISA 0.8 a module has verdict blocked:BD-B-ISA7' if tag.endswith('dialect')
                    else b'dialect="0.2.10"\nisa="0.8"')
        self.g.command = command
        with patch.dict(gate.PINS, dialect='0.2.10', isa='0.8'):
            with self.assertRaisesRegex(gate.GateError, 'BD-B-ISA7'):
                self.g.isa_contract()
        self.assertEqual(calls[1][-1], 'frozen-producer:SPEC_VERSIONS.toml')

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

    def receipt(self, m, workload='hello'):
        d = self.g.workdir(m, workload); out = d/'out'; out.mkdir(exist_ok=True)
        paths = [out/'device.bc', self.root/'publication/runtime.bc', self.root/'publication/arith.bc']
        for path, data in zip(paths, (b'device', b'runtime', b'arith')):
            path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(data)
        self.g.publications = {'bennett-runtimes':self.root/'publication'}
        support = self.root/f'publication/dist/llvm{m}/musl-sysroot/lib/native-support.bc'
        support.parent.mkdir(parents=True, exist_ok=True); support.write_bytes(b'support')
        layout = out/'native-layout.ll'
        layout.write_text('target triple = "x86_64-unknown-linux-musl"\n'
            '@__bennett_rt_native_layout = constant { i64, i64, i64 } { i64 1, i64 4096, i64 65536 }, align 8\n')
        script = out/'comparison.ld'
        script.write_text('SECTIONS { . = 4096; .data : { *(.data .data.*) } '
                          'ASSERT(ALIGN(., 4096) == 4096, "heap") . = 4194304; .text : { *(.text*) } }')
        adapter = self.root/'publication/scripts/native-check'
        adapter.parent.mkdir(parents=True, exist_ok=True)
        adapter.write_bytes(b'fixture native-check; execution mocked outside LLVM integration')
        merged = out/'comparison-linked.bc'; merged.write_bytes(b'merged')
        check_input = out/'comparison-check-input.bc'; check_input.write_bytes(b'check input')
        checked = out/'comparison-checked.bc'; checked.write_bytes(b'checked')
        final = out/'comparison.bc'; final.write_bytes(b'closed')
        prefix = f'/usr/lib/llvm-{m}/bin/'
        # Command oracle: pinned driver 54d32fa src/native.rs:141-186,266-370;
        # close_libraries in src/main.rs:302-332 links at least once, in place.
        # The tiny integration has stable imports after one closure iteration,
        # and initially lacks all three ABI memory definitions (native.rs:309-347).
        steps = [
            [prefix+'llvm-link', *map(str, paths), '--only-needed', '-o', str(merged)],
            [prefix+'llvm-link', str(merged), *map(str, paths[1:]),
             '--only-needed', '-o', str(merged)],
            [prefix+'llvm-link', str(merged), '-o', str(check_input)],
            [str(adapter), str(m), str(check_input), str(checked)],
            [prefix+'llvm-as', str(layout), '-o', str(out/'native-layout.bc')],
            [prefix+'llvm-link', str(checked), str(out/'native-layout.bc'),
             '--override='+str(support), '-o', str(out/'comparison-adapted.bc')],
            [prefix+'llvm-link', str(out/'comparison-adapted.bc'), *map(str, paths[1:]),
             '--only-needed', '-o', str(final)],
            [prefix+'llvm-extract', str(paths[1]), '--func=memcpy', '--func=memmove',
             '--func=memset', '-o', str(out/'native-memory.bc')],
            [prefix+'llvm-link', str(final), str(out/'native-memory.bc'), '-o', str(final)],
            [prefix+'llvm-link', str(final), *map(str, paths[1:]),
             '--only-needed', '-o', str(final)],
            [prefix+'opt', '-passes=strip-dead-prototypes,verify', str(final), '-o', str(final)],
        ]
        record = {'llvm_major':m, 'materialised_sha256':gate.sha(paths[0]),
                  'runtimes_sha256':gate.sha(paths[1]), 'arith_sha256':gate.sha(paths[2]),
                  'native_support_sha256':gate.sha(support),
                  # native.rs:424 hashes the FINAL comparison.bc, after cleanup.
                  'linked_sha256':gate.sha(final),
                  'bitcode_link_argv':steps[0],
                  'link_argv':[prefix+'clang', '-O2', '-nostdlib', '-nostdinc', '-static',
                      '-no-pie', '-fno-merge-all-constants', str(final), '-Wl,-T,'+str(script),
                      '-Wl,-e,_start', '-Wl,--no-undefined', '-Wl,--fatal-warnings',
                      '-Wl,-Map,'+str(out/'native.map'), '-o', str(d/'native')]}
        manifest = {'native_check_adapter':{'path':str(adapter), 'sha256':gate.sha(adapter)},
                    'checked_materialised':{'path':str(checked), 'sha256':gate.sha(checked)},
                    'commands':[{'argv':argv, 'success':True} for argv in steps]}
        return paths, record, manifest, d/'native'

    def test_comparison_steps_validate_order_and_checked_provenance(self):
        from native_audit import comparison_steps
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            expected = set(map(str, paths))
            self.g.command = lambda *a, **kw: self.fail('validation executed a command')
            steps, provenance, checked = comparison_steps(
                self.g, m, record['bitcode_link_argv'], paths, record, manifest, paths[0].parent)
            self.assertEqual(steps, [c['argv'] for c in manifest['commands']])
            self.assertEqual(checked, str(paths[0].parent/'comparison-checked.bc'))
            self.assertEqual(provenance[checked], expected | {checked})
            self.assertEqual(provenance[str(paths[0].parent/'comparison.bc')], expected | {checked})
            for mutation, reason in (('first', 'first bitcode merge must use the exact'),
                                     ('missing', 'missing first bitcode merge')):
                first = list(record['bitcode_link_argv'])
                mf = copy.deepcopy(manifest)
                if mutation == 'first':
                    first[1], first[2] = first[2], first[1]
                else:
                    mf['commands'].pop(0)
                with self.subTest(major=m, mutation=mutation), self.assertRaisesRegex(gate.GateError, reason):
                    comparison_steps(self.g, m, first, paths, record, mf, paths[0].parent)

    def test_final_inputs_require_all_leaves_and_checked_closure(self):
        from native_audit import audit_final_inputs
        device, runtime, arith, checked, final = (self.root/name for name in
            ('device.bc', 'runtime.bc', 'arith.bc', 'checked.bc', 'final.bc'))
        expected = set(map(str, (device, runtime, arith)))
        roots = expected | {str(checked)}
        self.assertIsNone(audit_final_inputs([final], {str(final):roots}, expected, str(checked)))
        cases = (
            ([], {}, 'staged receipt must name one final bitcode closure'),
            ([final, device], {}, 'staged receipt must name one final bitcode closure'),
            ([final], {}, 'final link uses an unrecorded bitcode input'),
            ([final], {str(final):roots - {str(runtime)}}, 'final closure dropped'),
            ([final], {str(final):expected}, 'final closure bypasses checked bitcode'),
            ([device], {str(device):roots}, 'staged receipt bypasses its merged closure'),
        )
        for inputs, provenance, reason in cases:
            with self.subTest(reason=reason), self.assertRaisesRegex(gate.GateError, reason):
                audit_final_inputs(inputs, provenance, expected, str(checked))

    def test_replay_comparison_preserves_order_and_checker_artifacts(self):
        from native_audit import replay_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            steps = [c['argv'] for c in manifest['commands']]
            checked = manifest['checked_materialised']['path']
            calls = []
            self.g.command = lambda tag, argv: calls.append((tag, argv))
            replay_comparison(self.g, steps, manifest, checked, paths[0].parent)
            prefix = str(paths[0].parent.relative_to(self.g.run))
            self.assertEqual(calls, [(prefix+f'/audit-bitcode-{i}', args) for i, args in enumerate(steps)])
            for victim, reason in (('input', 'native-check modified its input'),
                                   ('output', 'checked bitcode hash mismatch')):
                paths, record, manifest, output = self.receipt(m)
                steps = [c['argv'] for c in manifest['commands']]
                def command(tag, argv):
                    if Path(argv[0]).name == 'native-check':
                        Path(argv[2 if victim == 'input' else 3]).write_bytes(b'changed during replay')
                self.g.command = command
                with self.subTest(major=m, victim=victim), self.assertRaisesRegex(gate.GateError, reason):
                    replay_comparison(self.g, steps, manifest, checked, paths[0].parent)

    def test_auditable_multistage_receipt(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            calls = []
            self.g.command = lambda tag, argv, **kw: calls.append(list(map(str,argv))) or b''
            self.assertEqual(audit_comparison(self.g, m, *paths, record, manifest, output), record['link_argv'])
            self.assertEqual(calls, [c['argv'] for c in manifest['commands']])

    def test_receipt_hash_names_final_closure(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            with self.subTest(major=m):
                paths, record, manifest, output = self.receipt(m)
                self.g.command = lambda *a, **kw: b''
                audit_comparison(self.g, m, *paths, record, manifest, output)
                record['linked_sha256'] = gate.sha(Path(record['bitcode_link_argv'][-1]))
                with self.assertRaisesRegex(gate.GateError, 'final bitcode closure hash mismatch'):
                    audit_comparison(self.g, m, *paths, record, manifest, output)

    def test_checked_stage_is_mandatory_and_must_feed_final_closure(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            self.g.command = lambda *a, **kw: b''
            check = next(c for c in manifest['commands'] if Path(c['argv'][0]).name == 'native-check')
            for mutation in ('omitted', 'bypassed', 'failed'):
                mf = copy.deepcopy(manifest)
                if mutation == 'omitted':
                    mf['commands'].remove(check)
                elif mutation == 'bypassed':
                    support = next(c['argv'] for c in mf['commands']
                                   if any(a.startswith('--override=') for a in c['argv']))
                    support[1] = record['bitcode_link_argv'][-1]
                else:
                    next(c for c in mf['commands'] if c['argv'] == check['argv'])['success'] = False
                with self.subTest(major=m, mutation=mutation), self.assertRaisesRegex(
                        gate.GateError, 'checked|native.check'):
                    audit_comparison(self.g, m, *paths, record, mf, output)

    def test_native_check_is_bound_to_pinned_adapter_and_recorded_artifacts(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            self.g.command = lambda *a, **kw: b''
            for mutation, error in (
                ('path', 'pinned native-check'), ('adapter-hash', 'native-check adapter hash'),
                ('major', 'native-check LLVM major'), ('input', 'unrecorded native-check input'),
                ('checked-hash', 'checked bitcode hash'), ('output', 'checked output'),
            ):
                mf = copy.deepcopy(manifest)
                check = next(c['argv'] for c in mf['commands'] if Path(c['argv'][0]).name == 'native-check')
                match mutation:
                    case 'path':
                        check[0] = '/untrusted/scripts/native-check'
                        mf['native_check_adapter']['path'] = check[0]
                    case 'adapter-hash':
                        mf['native_check_adapter']['sha256'] = '0'*64
                    case 'major':
                        check[1] = str(41-m)
                    case 'input':
                        check[2] = str(paths[0].parent/'opaque.bc')
                    case 'checked-hash':
                        mf['checked_materialised']['sha256'] = '0'*64
                    case 'output':
                        check[3] = check[2]
                with self.subTest(major=m, mutation=mutation), self.assertRaisesRegex(gate.GateError, error):
                    audit_comparison(self.g, m, *paths, record, mf, output)

    @unittest.skipUnless(all(Path(f'/usr/lib/llvm-{m}/bin/{tool}').is_file()
                            for m in (18, 23) for tool in ('llvm-as', 'llvm-link', 'llvm-extract', 'opt', 'clang', 'clang++', 'llvm-config')),
                         'tiny receipt replay integration needs LLVM 18 and 23')
    def test_rebuilds_staged_receipt_from_exact_inputs(self):
        from native_audit import audit_comparison
        # Independent closure: main returns rt() + arith() = 7 + 11 = 18.
        # This is an adapter test, never either musl corpus workload.
        self.g.args.timeout = 20  # Includes a cold build of the pinned checker.
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            support = self.root/f'publication/dist/llvm{m}/musl-sysroot/lib/native-support.bc'
            # Materialise only the pinned executable adapter publication, never
            # live sibling sources. The script builds its own same-major checker.
            archive = self.g.command(f'{m}/test-adapter-publication',
                ['git', '-C', gate.WORKSPACE/'bennett-runtimes', 'archive',
                 gate.PINS['bennett-runtimes'], 'scripts/native-check', 'scripts/native-check.cpp'])
            with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
                tar.extractall(self.root/'publication', filter='data')
            adapter = self.root/'publication/scripts/native-check'
            manifest['native_check_adapter']['sha256'] = gate.sha(adapter)
            modules = {
                paths[0]: 'declare i32 @rt()\ndeclare i32 @arith()\n'
                          'define i32 @main() { %r = call i32 @rt()\n'
                          '%a = call i32 @arith()\n%s = add i32 %r, %a\nret i32 %s }\n',
                paths[2]: 'define i32 @arith() { ret i32 11 }\n',
                support: 'declare i32 @main()\ndefine void @_start() { %r = call i32 @main()\n'
                         '%rc = sext i32 %r to i64\ncall void asm sideeffect "syscall", '
                         '"{rax},{rdi},~{rcx},~{r11},~{memory}"(i64 60, i64 %rc)\nunreachable }\n',
            }
            for path, ir in modules.items():
                source = path.with_suffix('.ll')
                source.write_text('target triple = "x86_64-unknown-linux-musl"\n'+ir)
                self.g.command(f'{m}/test-assemble-{path.stem}',
                               [f'/usr/lib/llvm-{m}/bin/llvm-as', source, '-o', path])
            # Real ABI memory bodies for the driver's conditional extraction.
            runtime_c = paths[1].with_suffix('.c')
            runtime_c.write_text('int rt(void) { return 7; }\n'
                'void *memcpy(void *d, const void *s, unsigned long n) { '
                'unsigned char *a=d; const unsigned char *b=s; '
                'for (unsigned long i=0;i<n;i++) a[i]=b[i]; return d; }\n'
                'void *memmove(void *d, const void *s, unsigned long n) { '
                'unsigned char *a=d; const unsigned char *b=s; '
                'if (a<b) for (unsigned long i=0;i<n;i++) a[i]=b[i]; '
                'else while (n) { n--; a[n]=b[n]; } return d; }\n'
                'void *memset(void *d, int c, unsigned long n) { unsigned char *a=d; '
                'for (unsigned long i=0;i<n;i++) a[i]=c; return d; }\n')
            self.g.command(f'{m}/test-runtime', [self.g.clang(m),
                '--target=x86_64-unknown-linux-musl', '-O1', '-ffreestanding',
                '-fno-builtin', '-nostdinc', '-emit-llvm', '-c', runtime_c, '-o', paths[1]])
            for key, path in zip(('materialised', 'runtimes', 'arith'), paths):
                record[key+'_sha256'] = gate.sha(path)
            record['native_support_sha256'] = gate.sha(support)
            # Independently produce the driver's receipt before invoking the audit.
            for i, command in enumerate(manifest['commands']):
                self.g.command(f'{m}/test-driver-stage-{i}', command['argv'])
            final = paths[0].parent/'comparison.bc'
            checked = paths[0].parent/'comparison-checked.bc'
            record['linked_sha256'] = gate.sha(final)
            manifest['checked_materialised']['sha256'] = gate.sha(checked)
            self.assertNotEqual(record['linked_sha256'], gate.sha(paths[0].parent/'comparison-linked.bc'))
            checked_ir = self.g.command(f'{m}/test-checked-ir',
                [f'/usr/lib/llvm-{m}/bin/opt', '-S', checked, '-o', '-'])
            self.assertIn(b'@__bennett_rt_native_objects', checked_ir)
            (paths[0].parent/'comparison.ld').write_text('SECTIONS { . = 4194304; '
                '.text : { *(.text*) } .rodata : { *(.rodata*) } .data : { *(.data*) } '
                '/DISCARD/ : { *(.eh_frame*) *(.comment*) *(.note*) } }')
            # Spoofing an intermediate cannot bypass the replay from exact leaves.
            for name in ('comparison-linked.bc', 'comparison-check-input.bc',
                         'comparison-checked.bc', 'comparison-adapted.bc', 'comparison.bc'):
                (paths[0].parent/name).write_bytes(b'not trusted bitcode')
            argv = audit_comparison(self.g, m, *paths, record, manifest, output)
            self.g.command(f'{m}/test-native-link', argv)
            run = subprocess.run([str(output)], capture_output=True, timeout=5)
            self.assertEqual((run.returncode, run.stdout), (18, b''))
            dynamic = self.g.command(f'{m}/test-dynamic', ['readelf', '-d', output])
            self.assertNotIn(b'NEEDED', dynamic)
            undefined = self.g.command(f'{m}/test-undefined', [f'/usr/lib/llvm-{m}/bin/llvm-nm', '--undefined-only', output])
            self.assertEqual(undefined.strip(), b'')

    def test_native_reads_explicit_adapter_work_manifest(self):
        for m in (18, 23):
            for workload in ('hello', 'printf_fmt'):
                d = self.g.workdir(m,workload)
                out = d/'out'; out.mkdir()
                work = out/'native-work'; work.mkdir()
                (work/'manifest.json').write_text('{}')
                record = {'profile':'bennett-driver-isolated-provider-1','bitcode_link_argv':[],'inputs':{}}
                variants = [dict(record,variant=level,status='linked') for level in ('O2','O0')]
                (out/'native-comparison.json').write_text(json.dumps(variants[0]))
                (work/'native-comparisons.json').write_text(json.dumps(variants))
                self.g.runtime_files = lambda m: (self.root/'runtime.bc',self.root/'arith.bc')
                self.g.driver_exe = self.root/'driver'
                self.g.plugin_build = self.root/'plugin/build'
                self.g.publications = {'bennett-runtimes':self.root/'publication'}
                expected = bytes.fromhex(self.g.expected(workload)['stdout_hex'])
                for name in ('native','native-O0'):
                    Path(str(d/name)+'.runtime.json').write_text('{"status":"stale"}')
                calls = []
                def command(tag, argv, **kw):
                    calls.append(list(map(str,argv)))
                    if '/native-run-' in tag:
                        self.assertFalse(Path(str(argv[0])+'.runtime.json').exists())
                        Path(str(argv[0])+'.runtime.json').write_text('{}')
                        return expected
                    return b''
                self.g.command = command
                with patch.object(gate,'audit_hosted',return_value={}) as audit, \
                     patch.object(gate,'audit_execution') as execution:
                    self.g.native(m,workload)
                    self.assertEqual(audit.call_count,2)
                    self.assertEqual(execution.call_count,2)
                    self.assertEqual(audit.call_args.args[-1],d/'native-O0')
                self.assertEqual(len(calls),3)
                self.assertEqual((d/'native.stdout').read_bytes(),expected)
                self.assertIn('-bennett-out='+str(work),calls[0])
                self.assertIn('--bennett-native-provider='+str(self.g.native_provider),calls[0])
                def truncated(tag, argv, **kw):
                    result = command(tag,argv,**kw)
                    return result[:-1] if '/native-run-' in tag else result
                self.g.command = truncated
                with patch.object(gate,'audit_hosted',return_value={}), patch.object(gate,'audit_execution'), \
                     self.assertRaisesRegex(AssertionError,'native stdout differs'):
                    self.g.native(m,workload)

    def test_saved_and_corrupted_tapes_for_each_workload(self):
        self.g.argv_replies = lambda *a: Path('/argv-replies.json')
        self.g.executor = lambda *a: Path('/executor')
        for major in (18, 23):
            for executor in ('reference', 'VM'):
                for workload in ('hello', 'printf_fmt'):
                    expected = bytes.fromhex(self.g.expected(workload)['stdout_hex'])
                    value = {'fault':None, 'faults':[], 'host_requests':[],
                        'results':[{'type':'i32','value':'0'}], 'status':'COMPLETED',
                        'run_state':'CLOSED', 'console_hex':expected.hex(), 'effect_tape_length':2,
                        'tape':[{'dev':'console','op':'write','args':[4096,0],'bytes':''},
                                {'dev':'console','op':'write','args':[4096,len(expected)],'bytes':expected.hex()}]}
                    self.g.forward_results[(major,executor,workload)] = value
                    calls = []
                    def command(tag, argv, **kw):
                        saved = json.loads(Path(argv[-1]).read_text())
                        calls.append(saved)
                        if tag.endswith('tampered'):
                            self.assertNotEqual(saved, value['tape'])
                            self.assertEqual(saved[1]['args'], value['tape'][1]['args'])
                            return b'{"status":"FAILED","replay_mismatch":true}'
                        self.assertEqual(saved, value['tape'])
                        return json.dumps(value).encode()
                    self.g.command = command
                    self.g.replay(major, executor, workload)
                    self.g.replay(major, executor, workload, tampered=True)
                    self.assertEqual(len(calls), 2)
                    self.g.command = lambda *a, **kw: json.dumps(value).encode()
                    with self.assertRaisesRegex(AssertionError, 'accepted a tampered'):
                        self.g.replay(major, executor, workload, tampered=True)

    def test_receipt_rejects_host_libc_missing_runtime_and_hash_substitution(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            paths, record, manifest, output = self.receipt(m)
            mutations = []
            for extra in ('/usr/lib/libc.a', '-lc', '-Wl,-lc', '-Xlinker', '@response',
                          '-Wl,-Map,'+str(paths[0].parent/'native.map')+',-lc'):
                r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
                r['link_argv'].insert(-2, extra)
                mutations.append((r, mf, 'unapproved comparison input|opaque linker option|opaque comma'))
            for extra in ('/usr/lib/libc.a', '-L/usr/lib', '--override=/usr/lib/libm.bc'):
                r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
                mf['commands'][0]['argv'].insert(1, extra)
                mutations.append((r, mf, 'missing first bitcode merge'))
            r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
            mf['commands'][0]['argv'].remove(str(paths[1]))
            mutations.append((r, mf, 'missing first bitcode merge'))
            r, mf = copy.deepcopy(record), copy.deepcopy(manifest)
            substitute = self.root/'publication/substitute.bc'; substitute.write_bytes(b'wrong input')
            mf['commands'][0]['argv'][2] = str(substitute)
            mutations.append((r, mf, 'missing first bitcode merge'))
            for key in ('materialised', 'runtimes', 'arith'):
                r = copy.deepcopy(record); r[key+'_sha256'] = '0'*64
                mutations.append((r, manifest, key+' input hash mismatch'))
            self.g.command = lambda *a, **kw: b''
            for index, (r, mf, reason) in enumerate(mutations):
                with self.subTest(major=m, mutation=index), self.assertRaisesRegex(gate.GateError, reason):
                    audit_comparison(self.g, m, *paths, r, mf, output)
            paths[1].write_bytes(b'tampered runtime')
            with self.assertRaisesRegex(gate.GateError, 'runtimes input hash mismatch'):
                audit_comparison(self.g, m, *paths, record, manifest, output)

    def test_replay_preserves_checker_input_and_original_leaves(self):
        from native_audit import audit_comparison
        for m in (18, 23):
            for victim in ('checker-input', 'materialised', 'runtimes', 'arith'):
                paths, record, manifest, output = self.receipt(m)
                cleanup = manifest['commands'][-1]['argv']
                def command(tag, argv):
                    if victim == 'checker-input' and Path(argv[0]).name == 'native-check':
                        Path(argv[2]).write_bytes(b'changed during checker replay')
                    elif victim != 'checker-input' and argv == cleanup:
                        index = ('materialised', 'runtimes', 'arith').index(victim)
                        paths[index].write_bytes(b'changed during closure replay')
                    return b''
                self.g.command = command
                reason = ('native-check modified its input' if victim == 'checker-input'
                          else victim+' input hash changed during replay')
                with self.subTest(major=m, victim=victim), self.assertRaisesRegex(gate.GateError, reason):
                    audit_comparison(self.g, m, *paths, record, manifest, output)

    def test_linker_script_cannot_import_libc(self):
        from native_audit import audit_comparison
        paths, record, manifest, output = self.receipt(18)
        script = paths[0].parent/'comparison.ld'
        self.g.command = lambda *a, **kw: b''
        for text in ('INPUT(/usr/lib/libc.a)', 'INCLUDE host.ld', 'SECTIONS { .text : { libc.a(*) } }',
                     'SECTIONS { .text : { "/usr/lib/libc.a"(*) } }'):
            script.write_text(text)
            with self.assertRaisesRegex((AssertionError, gate.GateError), 'script'):
                audit_comparison(self.g, 18, *paths, record, manifest, output)

    def test_support_must_be_the_same_major_pinned_publication(self):
        from native_audit import audit_comparison
        paths, record, manifest, output = self.receipt(18)
        self.g.command = lambda *a, **kw: b''
        for relative in ('untracked/native-support.bc', 'dist/llvm23/musl-sysroot/lib/native-support.bc'):
            substitute = self.root/'publication'/relative
            substitute.parent.mkdir(parents=True, exist_ok=True)
            substitute.write_bytes(b'host libc substitute')
            record['native_support_sha256'] = gate.sha(substitute)
            support_link = next(c['argv'] for c in manifest['commands']
                                if any(a.startswith('--override=') for a in c['argv']))
            support_link[3] = '--override='+str(substitute)
            with self.assertRaisesRegex(gate.GateError, 'native support'):
                audit_comparison(self.g, 18, *paths, record, manifest, output)


if __name__ == '__main__':
    unittest.main()
