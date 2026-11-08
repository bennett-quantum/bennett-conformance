#!/usr/bin/env python3
"""Rung 2 acceptance gate. Missing implementations remain red, never xfail."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tarfile
import time
import tomllib
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "common"))
from gate_support import GateError, RecordedGate, git, sha
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'rung-1'))
from revision_sources import RevisionSources, add_revision_options, validate_revision_options
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
CONFORMANCE = HERE.parents[1]
WORKSPACE = CONFORMANCE.parent
DIALECT = '023f5a771a814270634db3be9cfde88ded69f975'
INPUTS = (0, 1, 7, 64, 1000)
DRIVERS = ('basic', 'grow')
BUDGET = '10000000,1048576,65536,256'
STAGES = ('sibling revisions', 'reference build', 'fixtures', 'plugin build', 'VM build', 'native reference', 'C front end',
          'runtime bitcode link', 'post-link LTO', 'S1–S9 production', 'launch artefacts',
          'reference admission', 'VM admission', 'reference forward = native',
          'VM forward = native', 'reference byte-exact reverse', 'VM byte-exact reverse')


def marked(source):
    source = source.replace('#include <stdlib.h>', '#include <stdlib.h>\n\n'
        '/* Dialect 0.2.10 §3.8: direct C calls; §4.2: mark both driver closures CC. */\n'
        'extern void *__bennett_qalloc(uint64_t, uint64_t);\n'
        'extern void __bennett_qfree(void *);')
    for driver in DRIVERS:
        old = f'int64_t ht_demo_{driver}(int64_t n) {{'
        assert source.count(old) == 1
        source = source.replace(old, old + '\n    void *rung2_marker = __bennett_qalloc(0, 1);'
            '\n    __bennett_qfree(rung2_marker);')
    return source


class Gate(RecordedGate):
    def __init__(self, args):
        self.args = args
        self.stages = STAGES + (('unlinked diagnostic probe',) if getattr(args, 'probe_unlinked', False) else ())
        self.tmp = Path(os.environ['TMPDIR']).resolve()
        self.out = Path(args.output).resolve() if args.output else self.tmp / 'rung-2'
        # Each invocation owns a fresh run directory; never reuse old plugin outputs.
        self.out.mkdir(parents=True, exist_ok=True)
        self.run = self.out / datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
        self.run.mkdir()
        self.rows = {m: {} for m in (18, 23)}
        self.manifest = {'time_utc': datetime.now(timezone.utc).isoformat(),
                         'dialect_commit': DIALECT, 'isa': '0.8', 'budget': BUDGET,
                         'inputs': list(INPUTS), 'drivers': list(DRIVERS), 'commands': [],
                         'environment': {k: os.environ.get(k) for k in
                             ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','BROWSER','TMPDIR')}}
        self.known = json.loads((HERE / 'blockers.json').read_text())
        self.sources = RevisionSources(self, WORKSPACE)

    def fixtures(self):
        pins = json.loads((HERE / 'fixtures/sha256.json').read_text())
        for name, digest in pins.items():
            if sha(HERE / 'fixtures' / name) != digest:
                raise GateError(f'fixture digest mismatch: {name}')
        self.manifest['fixture_sha256'] = pins
        assert marked((HERE / 'fixtures/hashtable.c').read_text()) == (HERE / 'fixtures/hashtable_marked.c').read_text(), 'marker-only variant differs'

    def plugin(self):
        repo = WORKSPACE / 'bennett-plugin'
        rev = self.args.plugin_rev or git(repo, 'rev-parse', 'HEAD')
        rev = git(repo, 'rev-parse', rev)
        self.manifest['plugin_commit'] = rev
        self.manifest.setdefault('pins', {})['bennett-plugin'] = rev
        src = self.tmp / ('rung2-plugin-' + rev)
        archive = self.command('build/plugin-archive', ['git', '-C', repo, 'archive', rev])
        if not src.exists():
            src.mkdir()
            with tarfile.open(fileobj=io.BytesIO(archive)) as t:
                t.extractall(src, filter='data')
        with tarfile.open(fileobj=io.BytesIO(archive)) as t:
            for member in t.getmembers():
                if member.isfile():
                    assert (src/member.name).read_bytes() == t.extractfile(member).read(), 'dirty plugin cache: ' + member.name
        self.plugin_build = src / 'build'
        self.command('build/plugin-configure', ['cmake','-S',src,'-B',self.plugin_build])
        self.command('build/plugin-build', ['cmake','--build',self.plugin_build,'-j',str(self.args.jobs)], timeout=1800)
        self.manifest['plugin_sha256'] = {str(m):sha(self.plugin_build / f'libbennett-plugin-{m}.so') for m in (18,23)}
        if self.args.arith_rev or self.args.plugin_rev:
            pins = tomllib.loads((src/'SPEC_VERSIONS.toml').read_text())['arith']
            self.sources.freeze('bennett-arith', self.args.arith_rev or pins['commit'])

    def vm(self):
        repo = WORKSPACE / 'bennett-vm'
        rev = git(repo, 'rev-parse', self.args.vm_rev or 'HEAD')
        self.manifest['vm_commit'] = rev
        self.manifest.setdefault('pins', {})['bennett-vm'] = rev
        src = self.tmp / ('vm-' + rev)
        if not src.exists():
            self.command('build/vm-worktree', ['git','-C',repo,'worktree','add','--detach',src,rev])
        assert git(src, 'rev-parse','HEAD') == rev and git(src, 'rev-parse','--abbrev-ref','HEAD') == 'HEAD', 'VM worktree HEAD mismatch or not detached'
        assert not git(src, 'status','--porcelain','--untracked-files=no'), 'VM worktree is dirty'
        self.command('build/vm-build', ['cargo','build','--offline','--jobs',self.args.jobs,'--manifest-path',src/'Cargo.toml','--target-dir',src/'target'], timeout=600)
        self.vm_exe = src / 'target/debug/bennett-vm'
        self.manifest['vm_sha256'] = sha(self.vm_exe)

    def build_reference(self):
        self.reference_exe = self.sources.reference()

    def native(self, major):
        d = self.run / str(major)
        d.mkdir(exist_ok=True)
        clang = f'/usr/lib/llvm-{major}/bin/clang'
        outputs = []
        for level in ('O2','O0'):
            exe = d / ('native-' + level)
            self.command(f'{major}/native-build-{level}', [clang, '-' + level, *(['-fsanitize=address,undefined','-fno-omit-frame-pointer'] if level == 'O0' else []), HERE/'fixtures/hashtable.c', HERE/'fixtures/main_golden.c', '-o',exe])
            outputs.append(self.command(f'{major}/native-run-{level}', [exe]))
        assert outputs[0] == outputs[1], 'native O2/O0 disagree'
        expected = [(driver,n) for driver in DRIVERS for n in INPUTS]
        lines = outputs[0].decode().splitlines()
        assert len(lines) == len(expected), 'native must emit all ten cases'
        values = []
        for line, (driver,n) in zip(lines, expected):
            fields = line.split()
            assert fields[:2] == [driver,str(n)], 'native case order differs'
            values.append(int(fields[2]))
        (d/'native.txt').write_bytes(outputs[0])
        (d/'native.bin').write_bytes(b''.join(struct.pack('<q',v) for v in values))

    def frontend(self, major):
        self.command(f'{major}/frontend', [f'/usr/lib/llvm-{major}/bin/clang','-O2','-flto','-emit-llvm','-c',HERE/'fixtures/hashtable_marked.c','-o',self.run/str(major)/'input.bc'])

    def runtime_link(self, major):
        runtime = Path(getattr(self.args, f'runtimes{major}') or
                       self.sources.paths.get('bennett-runtimes', WORKSPACE/'bennett-runtimes') / f'dist/llvm{major}/bennett-runtimes.bc')
        if not runtime.is_file():
            raise GateError(f'missing runtimes bitcode for LLVM {major}: {runtime} (dialect §19.1)')
        pins = tomllib.loads((self.plugin_build.parent/'SPEC_VERSIONS.toml').read_text())['arith']
        source = self.sources.paths.get('bennett-arith')
        default_arith = source / f'dist/llvm{major}/bennett-arith.bc' if source else pins[f'llvm{major}_path']
        arith = Path(getattr(self.args, f'arith{major}', None) or default_arith)
        if sha(arith) != pins[f'llvm{major}_sha256']:
            raise GateError(f'arith LLVM {major} published bitcode digest differs from plugin pin')
        self.manifest.setdefault('runtimes_sha256', {})[str(major)] = sha(runtime)
        self.manifest.setdefault('arith_sha256', {})[str(major)] = sha(arith)
        self.command(f'{major}/runtime-link', [f'/usr/lib/llvm-{major}/bin/llvm-link',
                     self.run/str(major)/'input.bc', runtime, arith, '--only-needed',
                     '-o', self.run/str(major)/'linked.bc'])

    def postlink(self, major):
        self.command(f'{major}/postlink', [f'/usr/lib/llvm-{major}/bin/opt','-passes=lto<O3>','-S',self.run/str(major)/'linked.bc','-o',self.run/str(major)/'postlink.ll'])

    def pipeline(self, major, probe=False):
        d = self.run / str(major)
        argv = [f'/usr/lib/llvm-{major}/bin/opt', '-load-pass-plugin='+str(self.plugin_build/f'libbennett-plugin-{major}.so'),'-passes=bennett','-bennett-dialect=0.2.10','-bennett-isa=0.8',
                '-bennett-budget-default='+BUDGET+',65536','-bennett-out='+str(d/('probe-out' if probe else 'out')),'-disable-output',d/('unlinked.ll' if probe else 'postlink.ll')]
        self.command(f'{major}/'+('production-probe' if probe else 'production'), argv)

    def probe(self, major):
        self.command(f'{major}/unlinked-postlink', [f'/usr/lib/llvm-{major}/bin/opt',
                     '-passes=lto<O3>', '-S', self.run/str(major)/'input.bc',
                     '-o', self.run/str(major)/'unlinked.ll'])
        self.pipeline(major, probe=True)

    def artifacts(self, major):
        d = self.run / str(major) / 'out'
        for filename in ('device.bc','host.bc','launches.json'):
            assert (d/filename).is_file(), 'missing ' + filename
        for kind in ('device','host'):
            self.command(f'{major}/verify-{kind}', [f'/usr/lib/llvm-{major}/bin/opt','-passes=verify','-disable-output',d/(kind+'.bc')])
        launches = json.loads((d/'launches.json').read_text())
        for driver in DRIVERS:
            entry = 'ht_demo_' + driver
            assert any(x['entry'] == entry for x in launches), 'missing launch entry ' + entry
            for filename in ('launch.bennett','build.json','certificate.json','coverage.json','blockers.json','launch-record.json'):
                assert (d/entry/filename).is_file(), 'missing ' + entry + '/' + filename
            assert json.loads((d/entry/'blockers.json').read_text()) == [], 'nonempty blocker report'

    def executor(self, name):
        if name == 'VM':
            return self.vm_exe
        exe = self.reference_exe
        if not exe.is_file():
            raise GateError('missing accepted reference executable: ' + str(exe))
        self.manifest['reference_sha256'] = sha(exe)
        return exe

    def admission(self, major, name):
        exe = self.executor(name)
        for driver in DRIVERS:
            obj = self.run / str(major)/'out'/('ht_demo_'+driver)/'launch.bennett'
            self.command(f'{major}/{name}-check-{driver}', [exe,'check',obj])
            self.command(f'{major}/{name}-hash-{driver}', [exe,'hash',obj])

    @staticmethod
    def result(value):
        assert value['fault'] is None and not value['faults'], 'execution fault: ' + str(value)
        assert not value['host_requests'], 'unmodelled external calls are not native equality'
        assert len(value['results']) == 1 and value['results'][0]['type'] == 'i64', 'expected one i64 result'
        v = int(value['results'][0]['value'], 0) if value['results'][0]['value'].startswith('0x') else int(value['results'][0]['value'])
        return (v + (1 << 63)) % (1 << 64) - (1 << 63)

    def execute(self, major, name, reverse=False):
        exe = self.executor(name)
        output, binary = [], bytearray()
        for driver in DRIVERS:
            entry = 'ht_demo_' + driver
            obj = self.run / str(major)/'out'/entry/'launch.bennett'
            for n in INPUTS:
                tag = f'{major}/{name}-'+('reverse' if reverse else 'forward')+f'-{driver}-{n}'
                argv = [exe,'run-program',obj,entry,'--args',str(n),'--budget',BUDGET]
                if reverse:
                    argv.append('--reverse')
                result = json.loads(self.command(tag, argv))
                assert result['status'] == ('REVERSED' if reverse else 'COMPLETED'), 'run did not complete: ' + str(result)
                assert result['run_state'] == 'CLOSED', 'launch not closed'
                v = self.result(result)
                output.append(f'{driver} {n} {v}\n')
                binary.extend(struct.pack('<q', v))
                if reverse:
                    assert result['reversed'] is True
                    self.restored(result)
                    # Check the retained-history CLI independently of one-shot launch.
                    r = json.loads(self.command(tag+'-history', [exe,'reverse',obj,entry,'--args',str(n),'--budget',BUDGET]))
                    assert r['status'] == 'REVERSED'
                    assert self.result(r) == v
                    self.restored(r)
        suffix = 'reverse' if reverse else 'forward'
        d = self.run / str(major)
        (d/f'{name}-{suffix}.txt').write_text(''.join(output))
        (d/f'{name}-{suffix}.bin').write_bytes(binary)
        assert (d/f'{name}-{suffix}.txt').read_bytes() == (d/'native.txt').read_bytes(), 'text output differs from native'
        assert bytes(binary) == (d/'native.bin').read_bytes(), 'result bytes differ from native'

    @staticmethod
    def restored(value):
        assert value['equals_entry'] is True, 'full-state mismatch: ' + str(value.get('first_difference'))
        assert value['memory_equals_entry'] is True, 'memory not restored'
        assert value['first_difference'] is None, 'first differing byte/component: ' + str(value['first_difference'])
        # Digests are compared within one executor only; serialization is executor-local.
        assert value['memory_digest'] == value['entry_memory_digest'], 'entry/final memory digest mismatch'

    def finish(self):
        table = ['| Stage | LLVM 18 | LLVM 23 |', '|---|---|---|']
        for stage in self.stages:
            cells = []
            for m in (18,23):
                r = self.rows[m][stage]
                cells.append(r['status'] + (' · ' + ', '.join(r['beads']) if r['beads'] else ''))
            table.append('| ' + stage + ' | ' + ' | '.join(cells) + ' |')
        report = {'manifest':self.manifest, 'stages':self.rows}
        (self.run/'report.json').write_text(json.dumps(report, indent=2)+'\n')
        (self.run/'table.md').write_text('\n'.join(table)+'\n')
        (self.out/'latest').write_text(str(self.run)+'\n')
        print('\n'+'\n'.join(table)+'\n\nEvidence: '+str(self.run), flush=True)
        return int(any(r['status'] != 'PASS' for rows in self.rows.values() for r in rows.values()))

    def main(self):
        self.stage(18,'sibling revisions',self.sources.prepare)
        self.rows[23]['sibling revisions'] = dict(self.rows[18]['sibling revisions'])
        self.stage(18,'reference build',self.build_reference,('sibling revisions',))
        self.rows[23]['reference build'] = dict(self.rows[18]['reference build'])
        self.stage(18,'fixtures',self.fixtures)
        self.rows[23]['fixtures'] = dict(self.rows[18]['fixtures'])
        self.stage(18,'plugin build',self.plugin,('sibling revisions',))
        self.rows[23]['plugin build'] = dict(self.rows[18]['plugin build'])
        self.stage(18,'VM build',self.vm,('sibling revisions',))
        self.rows[23]['VM build'] = dict(self.rows[18]['VM build'])
        def run_major(m):
            self.stage(m,'native reference',lambda:self.native(m),('fixtures',))
            self.stage(m,'C front end',lambda:self.frontend(m),('fixtures',))
            self.stage(m,'runtime bitcode link',lambda:self.runtime_link(m),('C front end','plugin build'))
            self.stage(m,'post-link LTO',lambda:self.postlink(m),('runtime bitcode link',))
            self.stage(m,'S1–S9 production',lambda:self.pipeline(m),('post-link LTO','plugin build'))
            self.stage(m,'launch artefacts',lambda:self.artifacts(m),('S1–S9 production',))
            for name in ('reference','VM'):
                self.stage(m,name+' admission',lambda:self.admission(m,name),('launch artefacts',)+(('VM build',) if name=='VM' else ('reference build',)))
                self.stage(m,name+' forward = native',lambda:self.execute(m,name),('native reference',name+' admission'))
                self.stage(m,name+' byte-exact reverse',lambda:self.execute(m,name,True),(name+' forward = native',))
            if self.args.probe_unlinked:
                self.stage(m,'unlinked diagnostic probe',lambda:self.probe(m),('C front end','plugin build'))
        with ThreadPoolExecutor(max_workers=2) as pool:
            list(pool.map(run_major, (18,23)))
        if all(self.rows[m]['native reference']['status'] == 'PASS' for m in (18,23)):
            if (self.run/'18/native.txt').read_bytes() != (self.run/'23/native.txt').read_bytes():
                for m in (18,23):
                    self.rows[m]['native reference'] = {'status':'FAIL', 'beads':['UNFILED'], 'error':'native LLVM 18/23 outputs differ'}
        return self.finish()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', help='parent for fresh timestamped evidence directories')
    p.add_argument('--plugin-rev', help='plugin commit; defaults to HEAD at invocation')
    p.add_argument('--vm-rev', help='VM commit; defaults to HEAD at invocation')
    p.add_argument('--runtimes18', help='published LLVM18 runtimes bitcode')
    p.add_argument('--arith18', help='LLVM18 arith bitcode to use instead of the live sibling dist (must still match the plugin pin digest)')
    p.add_argument('--arith23', help='LLVM23 arith bitcode to use instead of the live sibling dist (must still match the plugin pin digest)')
    p.add_argument('--runtimes23', help='published LLVM23 runtimes bitcode')
    p.add_argument('--probe-unlinked', action='store_true', help='also probe the corpus without runtimes; never substitutes for production')
    add_revision_options(p)
    p.add_argument('--reference', help='reference interpreter binary (release build; the debug build is far too slow for the corpus, bead b-isa-spec-n8j)')
    p.add_argument('--timeout', type=int, default=300, help='seconds per corpus command (default 300)')
    p.add_argument('--jobs', type=int, default=4, help='plugin build parallelism')
    args = p.parse_args()
    validate_revision_options(p, args)
    args.reference = args.reference or str(WORKSPACE/'bennett-isa-spec-review/reference-interpreter/target/release/bennett-isa-ref')
    return Gate(args).main()

if __name__ == '__main__':
    raise SystemExit(main())
