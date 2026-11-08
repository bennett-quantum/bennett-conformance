#!/usr/bin/env python3
"""North Star §8 rung 4: Grover over unmodified SQLite. Red prerequisites are red, never expected passes.
DESIGN.md states what each stage establishes and what no shortcut may replace."""
import argparse
import cmath
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib
import zipfile
sys.path.insert(0, str(Path(__file__).resolve().parents[1]/'common'))
from gate_support import GateError, RecordedGate, sha
from revision_sources import FrozenSources, REPOSITORIES, select_revisions

HERE = Path(__file__).resolve().parent
CONFORMANCE = HERE.parents[1]
WORKSPACE = CONFORMANCE.parent
PINS = json.loads((HERE/'pins.json').read_text())
ORACLE_SIZES = (1 << 4, 1 << 8, 1000, 1 << 10)
GROVER_SIZES = (1 << 4, 1 << 6, 1 << 8)
ESTIMATE_SIZE = 1 << 20
BUILD_SIZES = tuple(sorted({*ORACLE_SIZES, *GROVER_SIZES, ESTIMATE_SIZE}))
AMPLITUDE_TOLERANCE = 1e-9
SQLITE_WITNESSES = ('sqlite3VdbeExec', 'memdbRead', 'sqlite3_deserialize', 'sqlite3BtreeTableMoveto')
EXECUTORS = ('reference', 'VM')
ACCEPT = ('admission', '(a) oracle = native, fault rate zero', '(a) full tuple on D', '(a) literal step check',
          '(b) amplitudes', '(b) outcomes')
STAGES = ('sibling revisions', 'reference build', 'fixtures', 'SQLite source unmodified', 'independent predicate', 'plugin build', 'VM build', 'driver build', 'estimator build',
          'checker build',
          'host image = native oracle = predicate', 'images agree across majors', 'musl C front end', 'runtime bitcode link', 'runtime libc closure', 'post-link LTO',
          'coherent production', 'launch artefacts and oracle shape', 'materialised comparison = native',
          *(f'{e} {a}' for e in EXECUTORS for a in ACCEPT),
          '(a) certificate verdict', '(c) resource estimate at N=2^20',
          'reference reduced control (diagnostic)', 'VM reduced control (diagnostic)',
          'published Grover TU production (diagnostic)',
          'reference --mem-file (diagnostic)', 'VM --mem-file (diagnostic)',
          'reference measured control (diagnostic)', 'VM measured control (diagnostic)',
          'estimator CLI (diagnostic)', 'checker CLI (diagnostic)')

# ---------------------------------------------------------------- independent predicate (no SQLite)
FIRST = 'Ada Brian Carmen Dmitri Elena Frank Grace Hans Ingrid Juan Kenji Lena Morgan Nadia Oscar Priya'.split()
LAST = 'Anders Baker Chandra Diaz Evans Fujita Gallo Hansen'.split()
CITY = 'Berlin Lagos Madrid Boston Lima Mumbai Oslo Paris Quito Rome Seoul Tokyo Utrecht Vienna Warsaw Zagreb'.split()
U64 = (1 << 64) - 1


def _splitmix(x):
    x = (x + 0x9e3779b97f4a7c15) & U64
    x = ((x ^ (x >> 30)) * 0xbf58476d1ce4e5b9) & U64
    x = ((x ^ (x >> 27)) * 0x94d049bb133111eb) & U64
    return x ^ (x >> 31)


def matches(rowid):
    h = _splitmix((rowid * 0x2545F4914F6CDD1D + 7) & U64)
    name = FIRST[h & 15] + ' ' + LAST[(h >> 4) & 7]
    city = CITY[(h >> 7) & 15]
    age = 18 + (h >> 11) % 73
    score = (h >> 20) % 1000
    return 'an' in name.lower() and city[0] in 'BLM' and 25 <= age <= 60 and (score * 7 + age) % 11 == 3


def truth(n_rows, bits):
    return [int(x + 1 <= n_rows and matches(x + 1)) for x in range(1 << bits)]


def bits_for(n_rows):
    return max(1, math.ceil(math.log2(n_rows)))


def region_exponent(length):
    return max(4, math.ceil(math.log2(length)))


def domain_size(bits):
    return 1 << (bits + 1 + 4)


def domain(bits):
    for x in range(1 << bits):
        for t in range(2):
            for e in range(16):
                yield x, t, e


def expected_tuple(x, t, e, f, bits):
    return x | ((t ^ f) << bits) | (e << (bits + 1))

# ---------------------------------------------------------------- Grover by independent simulation


def grover_plan(n_rows, bits=None):
    bits = bits or bits_for(n_rows)
    table = truth(n_rows, bits)
    marked, size = sum(table), 1 << bits
    if not marked:
        return {'N': n_rows, 'M': 0, 'k': 1, 'theta': 0.0, 'success': 0.0}
    theta = math.asin(math.sqrt(marked / size))
    k = math.floor(math.pi / (4 * theta))
    return {'N': n_rows, 'M': marked, 'k': k, 'theta': theta, 'success': math.sin((2 * k + 1) * theta) ** 2}


def simulate(table, k):
    """Row-register amplitudes after k iterations from the oracle's truth table; an ordinary array simulation."""
    size = len(table)
    s = [complex(1 / math.sqrt(size))] * size
    for _ in range(k):
        s = [-a if b else a for a, b in zip(s, table)]
        mean = sum(s) / size
        s = [2 * mean - a for a in s]
    return s


def finite_json_number(text):
    value = float(text)
    if not math.isfinite(value):
        raise ValueError(f'non-finite JSON number {text}')
    return value


def check_state(dump, bits, table, a, k, image, m_expected=None):
    """A pre-measurement state: database constant and unentangled, fault and ancilla clean, target |->,
    row amplitudes equal the independent simulation up to one global phase, within AMPLITUDE_TOLERANCE."""
    regs = dump['registers']
    assert [r['width'] for r in regs] == [8 << a, bits, 1, 4, bits], 'register widths differ from the TU'
    assert dump['constant'].get('0') == image, 'database register changed'
    want = simulate(table, k)
    got = {}
    for row in dump['amps']:
        basis = row['basis']
        assert '0' not in basis, 'database register entangled with a branch'
        assert basis['3'] == '0', 'fault register not clean on a branch'
        assert basis['4'] == '0', 'ancilla not clean on a branch'
        key = (int(basis['1']), int(basis['2']))
        assert key not in got, 'duplicate basis state'
        amplitude = complex(row['re'], row['im'])
        assert math.isfinite(amplitude.real) and math.isfinite(amplitude.imag), f'non-finite amplitude at basis {basis}: {amplitude}'
        got[key] = amplitude
    pivot = max(range(len(want)), key=lambda x: abs(want[x]))
    # A key missing from the sparse dump has amplitude zero, which the comparison below rejects.
    observed = got.get((pivot, 0), 0j)
    assert abs(observed) > 0, 'amplitude missing at the largest branch'
    phase = observed / (want[pivot] / math.sqrt(2))
    phase /= abs(phase)
    assert math.isfinite(phase.real) and math.isfinite(phase.imag), 'non-finite amplitude phase'
    error = 0.0
    for x, v in enumerate(want):
        for t, sign in ((0, 1), (1, -1)):
            error = max(error, abs(got.pop((x, t), 0j) - phase * v * sign / math.sqrt(2)))
    assert not got, 'amplitude outside the row and target registers'
    assert error <= AMPLITUDE_TOLERANCE, f'amplitude error {error} above {AMPLITUDE_TOLERANCE}'
    success = sum(abs(v) ** 2 for v, b in zip(want, table) if b)
    return {'max_amplitude_error': error, 'success': success, 'global_phase': [phase.real, phase.imag]}


def predict_outcome(probs, bits, draws):
    """ISA §14.2 draws: bit-at-a-time on wires 0.. in order; first outcome whose cumulative weight strictly exceeds draw."""
    weights = list(probs)
    value = 0
    for i in range(bits):
        p0 = sum(w for x, w in enumerate(weights) if not (x >> i) & 1)
        total = sum(weights)
        bit = 0 if p0 > draws[i] * total else 1
        value |= bit << i
        weights = [w if ((x >> i) & 1) == bit else 0.0 for x, w in enumerate(weights)]
    return value


def draws_for(probs, bits, x):
    """Draws that select branch x, at the middle of each conditional interval."""
    weights = list(probs)
    out = []
    for i in range(bits):
        total = sum(weights)
        assert total > 0, 'selected branch has no weight'
        p0 = sum(w for y, w in enumerate(weights) if not (y >> i) & 1) / total
        bit = (x >> i) & 1
        out.append(p0 / 2 if not bit else (p0 + 1) / 2)
        weights = [w if ((y >> i) & 1) == bit else 0.0 for y, w in enumerate(weights)]
    return out

# ---------------------------------------------------------------- pure result checks


def tuple_result(value):
    assert value['status'] == 'COMPLETED', 'run did not complete: ' + str(value.get('status'))
    assert value['fault'] is None and not value['faults'], 'faulted execution: ' + str(value['fault'])
    assert not value['host_requests'], 'unmodelled host effect in oracle execution'
    assert value['run_state'] == 'CLOSED', 'launch not closed'
    assert len(value['results']) == 1, 'expected one result'
    return int(value['results'][0]['value'], 0)


def oracle_row(value, want, budget_steps):
    assert value['code'] == 0, f'nonzero fault code {value["code"]}: fault rate must be zero'
    assert value['payload'] == want, f'payload {value["payload"]} differs from native {want}'
    assert value['steps'] <= budget_steps, 'reported steps exceed the budget'


def step_report(value, bits):
    assert value['status'] == 'ok', 'step check failed'
    assert value['mode'] == 'exhaustive', 'not an exhaustive step check'
    assert value['cases'] == 1 << bits, 'wrong number of cases'
    assert value['input_and_memory_bits'] == bits, 'domain differs from the rowid register'
    assert value['fixed_regions'] == ['m0'], 'image not fixed'


def check_reference_object(text, bits, a):
    head = f'oracle @ref (x0:wire[{bits}] in) (tgt:wire[1]) (fault:wire[4]) (m0:region[{a},8] in) (a0:i64)'
    assert head in text, 'reference object is not the rung-4 oracle shape: ' + head
    procs = len(re.findall(r'^lproc @', text, re.M))
    assert procs >= 500, f'closure of {procs} lprocs cannot be SQLite (spike closure: 1,218 functions)'


def check_program_text(text):
    for s in SQLITE_WITNESSES:
        assert re.search(r'^define [^\n]*@' + s + r'\(', text, re.M), 'stock SQLite function absent from program: ' + s


def unresolved(text):
    names = re.findall(r'^declare [^\n]*?@([A-Za-z_][\w.$]*)\(', text, re.M)
    return sorted(n for n in names if not n.startswith(('llvm.', '__bennett_')))


def check_coverage(value):
    def walk(v):
        if isinstance(v, dict):
            assert v.get('status') != 'trapped', 'trapped coverage line (call without bitcode): ' + str(v.get('name'))
            for w in v.values():
                walk(w)
        elif isinstance(v, list):
            for w in v:
                walk(w)
    walk(value)


def check_certificate_verdict(value, elements, a):
    """ISA §12.2 K74: a domain fixing the image is a strict slice: never `proof`; exactly n/|D|."""
    assert value['verdict'] == 'evidence', 'fixed-region claim must be labelled evidence, got ' + str(value['verdict'])
    assert value['elements'] == elements, 'checker replayed a different element count'
    assert value['covered_fraction'] == f'{elements}/{elements * 2 ** (8 << a)}', 'covered fraction wrong'


def check_estimate(value, object_sha):
    assert value['target'], 'no named target'
    assert value['label'], 'no estimate label (exact/bound/...)'
    assert value['object_sha256'] == object_sha, 'estimate is not of the actual 2^20 object'
    assert value['padded'], 'no padded cost reported'
    for field in ('steps', 'heap', 'stack', 'depth'):
        assert value['budget'][field] == PINS['budget'][field], 'estimate budget differs: ' + field


def launch_allowances(launch):
    """ISA §9: these SQLite launches take one token per build budget, in array order."""
    build = Path(launch)/'build.json'
    try:
        metadata = json.loads(build.read_text())
    except (OSError, ValueError) as exc:
        raise GateError(f'cannot read launch budget allowances from {build}: {exc}') from exc
    if not isinstance(metadata, dict) or 'budgets' not in metadata:
        raise GateError(f'missing budgets in {build}; SQLite launches hold oracle regions')
    budgets = metadata['budgets']
    if not isinstance(budgets, list):
        raise GateError(f'expected budgets array in {build}; SQLite launches hold oracle regions')
    allowances = []
    for index, row in enumerate(budgets):
        if not isinstance(row, dict) or 'steps' not in row:
            raise GateError(f'missing steps allowance in budget {index} of {build}')
        value = row['steps']
        if type(value) not in (int, str) or not re.fullmatch(r'0|[1-9][0-9]*', str(value)):
            raise GateError(f'invalid steps allowance in budget {index} of {build}: {value!r}')
        allowances.append(str(value))
    return tuple(allowances)

# ---------------------------------------------------------------- the gate


class Gate(RecordedGate):
    def __init__(self, args):
        self.args = args
        self.tmp = CONFORMANCE/'.tmp'/'rung-4'
        self.out = Path(args.output).resolve() if args.output else self.tmp/'runs'
        self.out.mkdir(parents=True, exist_ok=True)
        self.run = self.out/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
        self.run.mkdir()
        for m in (18, 23):
            (self.run/str(m)).mkdir()
        self.rows = {m: {} for m in (18, 23)}
        self.known = json.loads((HERE/'blockers.json').read_text())
        self.images = {}
        self.manifest = {'time_utc': datetime.now(timezone.utc).isoformat(), 'commands': [], 'pins': dict(PINS), 'dialect': PINS['dialect'], 'isa': PINS['isa'],
                         'budget': PINS['budget'], 'oracle_sizes': ORACLE_SIZES, 'grover_sizes': GROVER_SIZES,
                         'estimate_size': ESTIMATE_SIZE, 'amplitude_tolerance': AMPLITUDE_TOLERANCE, 'evidence': {},
                         'environment': {k: os.environ.get(k) for k in
                                         ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'BROWSER', 'TMPDIR')},
                         'gate_sha256': sha(HERE/'gate.py'), 'revision_sources_sha256': sha(HERE/'revision_sources.py'), 'shared_support_sha256': sha(HERE.parent/'common/gate_support.py'),
                         'blockers_sha256': sha(HERE/'blockers.json')}
        self.sources = FrozenSources(self, WORKSPACE)

    # ------------ sources
    def reference(self):
        if self.args.reference is not None:
            self.reference_exe = Path(self.args.reference).resolve()
            self.manifest['reference_profile'] = 'explicit path'
        else:
            src = self.sources.paths['bennett-isa-spec']
            target = src/'target'
            self.command('build/reference-build', ['cargo', 'build', '--release', '--offline', '--jobs', self.args.jobs,
                         '--manifest-path', src/'reference-interpreter/Cargo.toml', '--target-dir', target])
            self.reference_exe = target/'release/bennett-isa-ref'
            self.manifest['reference_profile'] = 'release'
        assert self.reference_exe.is_file(), 'missing reference interpreter: '+str(self.reference_exe)
        self.manifest['reference_sha256'] = sha(self.reference_exe)

    def fixtures(self):
        pins = json.loads((HERE/'fixtures/sha256.json').read_text())
        for f, digest in pins.items():
            assert sha(HERE/'fixtures'/f) == digest, 'fixture hash mismatch: '+f
        expect = json.loads((HERE/'fixtures/expect.json').read_text())
        ids = set()
        for spec in ('DLT-0.2.10', 'ISA-0.8'):
            ids.update(n['id'] for n in json.loads((CONFORMANCE/f'clauses/{spec}.json').read_text())['nodes'])
        assert expect['clauses'] and set(expect['clauses']) <= ids, 'missing/invalid clause tags'
        self.manifest['fixtures'] = pins
        dialect = self.sources.paths['bennett-dialect-spec']
        self.spec = (dialect/'spec/Bennett-Dialect-Spec.md').read_text()
        self.markers = self.run/'include'
        self.markers.mkdir()
        (self.markers/'bennett_markers.h').write_bytes((dialect/'spec/bennett_markers.h').read_bytes())

    def check_sqlite_tree(self, root):
        p = PINS['sqlite']
        assert sha(root/'sqlite3.c') == p['sqlite3_c_sha256'], 'sqlite3.c is not the unmodified amalgamation'
        assert sha(root/'sqlite3.h') == p['sqlite3_h_sha256'], 'sqlite3.h is not the unmodified amalgamation'

    def sqlite_source(self):
        p = PINS['sqlite']
        name = p['url'].rsplit('/', 1)[1]
        cache = self.tmp/'sqlite'
        path = cache/name
        if not path.is_file():
            home = Path(os.environ.get('BENNETT_SQLITE_HOME', Path.home()/'.local/share/bennett-sqlite'))/p['version']/name
            if not home.is_file():
                raise GateError(f'missing amalgamation {path}: run ladder/rung-4/fixtures/fetch-sqlite.sh')
            cache.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(home, path)
        data = path.read_bytes()
        assert len(data) == p['zip_bytes'], 'amalgamation size differs from published'
        assert hashlib.sha3_256(data).hexdigest() == p['zip_sha3_256'], 'amalgamation SHA3-256 differs from published'
        assert hashlib.sha256(data).hexdigest() == p['zip_sha256'], 'amalgamation SHA-256 differs from pin'
        root = cache/p['version']
        if not root.exists():
            root.mkdir()
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for info in z.infolist():
                    if not info.is_dir():
                        (root/Path(info.filename).name).write_bytes(z.read(info))
        self.check_sqlite_tree(root)
        self.sqlite = root
        self.manifest['sqlite'] = {**p, 'source_id': re.search(r'#define SQLITE_SOURCE_ID\s+"([^"]+)"', (root/'sqlite3.h').read_text())[1]}

    def predicate(self):
        spike = {256: 6, 1024: 13, 1 << 20: 11698}
        self.tables = {}
        for n in BUILD_SIZES:
            self.tables[n] = truth(n, bits_for(n))
        for n, want in spike.items():
            assert sum(self.tables[n]) == want, f'independent predicate lost the spike count at N={n}'
        self.plans = {n: grover_plan(n) for n in BUILD_SIZES}
        self.manifest['evidence']['grover_plans'] = {str(n): {k: v for k, v in p.items()} for n, p in self.plans.items()}

    def plugin(self):
        for m in (18, 23):
            for tool in ('clang', 'opt', 'llvm-link', 'llvm-as'):
                self.command(f'build/llvm{m}-{tool}-version', [f'/usr/lib/llvm-{m}/bin/{tool}', '--version'])
        src = self.sources.paths['bennett-plugin']
        layout = self.sources.plugin_layout()
        build_env = ['env', 'BENNETT_GATE_LAYOUT='+str(layout)]
        self.plugin_build = src/'build'
        self.command('build/plugin-configure', [*build_env, 'cmake', '-S', src, '-B', self.plugin_build])
        self.command('build/plugin-build', [*build_env, 'cmake', '--build', self.plugin_build, '-j', self.args.jobs], timeout=1800)
        self.manifest['plugin_sha256'] = {str(m): sha(self.plugin_build/f'libbennett-plugin-{m}.so') for m in (18, 23)}
        self.pub_runtimes = self.sources.paths['bennett-runtimes']
        self.arith_pin = tomllib.loads((src/'SPEC_VERSIONS.toml').read_text())['arith']
        self.pub_arith = self.sources.paths['bennett-arith']
        for m in (18, 23):
            self.libraries(m)

    def cargo(self, repo, tag, binary):
        src = self.sources.paths[repo]
        self.command(tag, ['cargo', 'build', '--release', '--offline', '--jobs', self.args.jobs, '--manifest-path', src/'Cargo.toml', '--target-dir', src/'target'], timeout=1200)
        return src/'target/release'/binary

    def vm(self):
        self.vm_exe = self.cargo('bennett-vm', 'build/vm-build', 'bennett-vm')
        self.manifest['vm_sha256'] = sha(self.vm_exe)

    def driver(self):
        self.driver_exe = self.cargo('bennett-driver', 'build/driver-build', 'bennett-driver')
        self.manifest['driver_sha256'] = sha(self.driver_exe)

    def estimator(self):
        self.estimate_exe = self.cargo('bennett-estimate', 'build/estimate-build', 'bennett-estimate')
        self.targets = self.sources.paths['bennett-targets']
        self.target = self.targets/'targets'/PINS['target']
        assert self.target.is_file(), 'missing named target '+PINS['target']
        self.manifest['estimate_sha256'] = sha(self.estimate_exe)
        self.manifest['target_sha256'] = sha(self.target)

    def checker(self):
        # The checker ships in this repository; a pinned commit, not the concurrently edited tree.
        src = self.sources.paths['bennett-conformance']
        self.command('build/checker-build', ['cargo', 'build', '--release', '--offline', '--jobs', self.args.jobs, '--manifest-path', src/'checker/Cargo.toml', '--target-dir', src/'target'], timeout=600)
        self.check_exe = src/'target/release/bennett-check'
        self.manifest['checker_sha256'] = sha(self.check_exe)

    def executor(self, name):
        exe = self.vm_exe if name == 'VM' else self.reference_exe
        assert exe.is_file(), 'missing executor: '+str(exe)
        self.manifest.setdefault('executor_sha256', {})[name] = sha(exe)
        return exe

    # ------------ classical before entry
    def tool(self, m, name):
        return f'/usr/lib/llvm-{m}/bin/{name}'

    def host(self, m):
        d = self.run/str(m)/'host'
        d.mkdir()
        p = PINS['sqlite']
        flags = ['-O2', '-g0', *p['flags'], '-I', self.sqlite, '-I', HERE/'fixtures']
        self.command(f'{m}/host-sqlite', [self.tool(m, 'clang'), *flags, '-c', self.sqlite/'sqlite3.c', '-o', d/'sqlite3.o'])
        common = [HERE/'fixtures/os_shim.c', d/'sqlite3.o', '-lm']
        self.command(f'{m}/host-build', [self.tool(m, 'clang'), *flags, HERE/'fixtures/host_build.c', *common, '-o', d/'host_build'])
        self.command(f'{m}/host-oracle', [self.tool(m, 'clang'), *flags, HERE/'fixtures/oracle.c', HERE/'fixtures/native_oracle.c', *common, '-o', d/'native_oracle'])
        images = {}
        for n in BUILD_SIZES:
            image = d/f'image-{n}.bin'
            self.command(f'{m}/host-image-{n}', [d/'host_build', n, image])
            bits = bits_for(n)
            out = self.command(f'{m}/native-oracle-{n}', [d/'native_oracle', image, 1 << bits], timeout=900).decode().strip()
            got = [int(c) for c in out]
            assert got == self.tables[n], f'native SQLite disagrees with the independent predicate at N={n}'
            images[n] = image
        self.images[m] = images
        digests = {str(n): sha(i) for n, i in images.items()}
        self.manifest['evidence'].setdefault('image_sha256', {})[str(m)] = digests
        self.manifest['evidence'].setdefault('image_bytes', {}).update({str(n): i.stat().st_size for n, i in images.items()})

    def images_agree(self):
        # Database bytes are classical data, independent of the LLVM major that compiled the host tool.
        assert 18 in self.images and 23 in self.images, 'host stage did not produce both majors'
        for n in BUILD_SIZES:
            assert sha(self.images[18][n]) == sha(self.images[23][n]), f'image differs across majors at N={n}'

    def plan_flags(self, n, length, a):
        b = PINS['budget']
        return [f'-DRUNG4_BITS={bits_for(n)}', f'-DRUNG4_A={a}', f'-DRUNG4_LEN={length}', f'-DRUNG4_ITERS={self.plans[n]["k"]}',
                f'-DRUNG4_B_STEPS={b["steps"]}', f'-DRUNG4_B_HEAP={b["heap"]}', f'-DRUNG4_B_STACK={b["stack"]}', f'-DRUNG4_B_DEPTH={b["depth"]}']

    def musl_flags(self, m):
        root = self.pub_runtimes/f'dist/llvm{m}/musl-sysroot'
        assert (root/'include/stdio.h').is_file(), 'missing published musl sysroot: '+str(root)
        return ['--target=x86_64-unknown-linux-musl', '--sysroot='+str(root), '-nostdinc', '-isystem', root/'include', '-emit-llvm',
                *PINS['sqlite']['cflags'], *PINS['sqlite']['flags'], '-I', self.sqlite, '-I', HERE/'fixtures', '-I', self.markers]

    def image_module(self, m, n, d):
        data = self.images[m][n].read_bytes()
        a = region_exponent(len(data))
        pad = (1 << a) - len(data)
        text = ''.join(f'\\{b:02X}' for b in data)
        pad_ty = f', [{pad} x i8] zeroinitializer' if pad else ''
        (d/'db_image.ll').write_text(f'@rung4_db_image = constant <{{ [{len(data)} x i8]{f", [{pad} x i8]" if pad else ""} }}> <{{ [{len(data)} x i8] c"{text}"{pad_ty} }}>, align 16\n')
        self.command(f'{m}/{n}-image-module', [self.tool(m, 'llvm-as'), d/'db_image.ll', '-o', d/'db_image.bc'])
        return len(data), a

    def frontend(self, m):
        root = self.run/str(m)
        flags = self.musl_flags(m)
        for name, source in (('sqlite3', self.sqlite/'sqlite3.c'), ('oracle', HERE/'fixtures/oracle.c'), ('os_shim', HERE/'fixtures/os_shim.c')):
            self.command(f'{m}/frontend-{name}', [self.tool(m, 'clang'), *flags, '-c', source, '-o', root/f'{name}.bc'], timeout=600)
        self.manifest['evidence'].setdefault('sqlite3_bc_sha256', {})[str(m)] = sha(root/'sqlite3.bc')
        for n in BUILD_SIZES:
            d = root/f'N{n}'
            d.mkdir()
            length, a = self.image_module(m, n, d)
            self.command(f'{m}/{n}-frontend-grover', [self.tool(m, 'clang'), *flags, *self.plan_flags(n, length, a), '-c', HERE/'fixtures/grover.c', '-o', d/'grover.bc'])
            self.command(f'{m}/{n}-link', [self.tool(m, 'llvm-link'), d/'grover.bc', root/'oracle.bc', root/'os_shim.bc', d/'db_image.bc', root/'sqlite3.bc', '-o', d/'program.bc'])

    def libraries(self, m):
        runtime = self.pub_runtimes/f'dist/llvm{m}/bennett-runtimes.bc'
        expected = (self.pub_runtimes/f'dist/llvm{m}/SHA256SUMS').read_text().split()[0]
        assert sha(runtime) == expected, 'runtimes publication differs from pinned revision'
        arith = self.pub_arith/f'dist/llvm{m}/bennett-arith.bc'
        assert sha(arith) == self.arith_pin[f'llvm{m}_sha256'], 'arith publication differs from plugin pin'
        return runtime, arith

    def runtime_link(self, m):
        runtime, arith = self.libraries(m)
        for n in BUILD_SIZES:
            d = self.run/str(m)/f'N{n}'
            self.command(f'{m}/{n}-runtime-link', [self.tool(m, 'llvm-link'), d/'program.bc', runtime, arith, '--only-needed', '-o', d/'linked.bc'])

    def postlink(self, m):
        for n in BUILD_SIZES:
            d = self.run/str(m)/f'N{n}'
            self.command(f'{m}/{n}-postlink', [self.tool(m, 'opt'), '-passes=lto<O3>', '-S', d/'linked.bc', '-o', d/'postlink.ll'], timeout=1200)
            check_program_text((d/'postlink.ll').read_text())

    def closure(self, m):
        # Dialect §4.5: a call without bitcode is a `trapped` coverage line. The oracle executes these
        # (REPORT.md §5.2(iv)), so unresolved libc means a fault at run time, never a pass.
        names = sorted({u for n in BUILD_SIZES for u in unresolved((self.run/str(m)/f'N{n}'/'postlink.ll').read_text())})
        self.manifest['evidence'].setdefault('unresolved_after_link', {})[str(m)] = names
        if names:
            raise GateError('runtime libc closure incomplete, undefined after link: '+', '.join(names))

    def production(self, m):
        lim = PINS['plugin_limits']
        b = PINS['budget']
        for n in BUILD_SIZES:
            d = self.run/str(m)/f'N{n}'
            self.command(f'{m}/{n}-production', ['env', '-u', 'BENNETT_ARITH_BITCODE', 'BENNETT_ARITH_PINNED_ROOT='+str(self.pub_arith/'dist'), self.tool(m, 'opt'), '-load-pass-plugin='+str(self.plugin_build/f'libbennett-plugin-{m}.so'), '-passes=bennett',
                         '-bennett-dialect='+self.manifest['dialect'], '-bennett-isa='+self.manifest['isa'],
                         f'-bennett-budget-default={b["steps"]},{b["heap"]},{b["stack"]},{b["depth"]},{b["shadow_stack"]}',
                         f'-bennett-analysis-budget-ms={lim["analysis_budget_ms"]}', f'-bennett-analysis-max-rounds={lim["analysis_max_rounds"]}',
                         '-bennett-out='+str(d/'out'), '-disable-output', d/'postlink.ll'], timeout=self.args.production_timeout)

    def artefacts(self, m):
        for n in BUILD_SIZES:
            d = self.run/str(m)/f'N{n}'/'out'
            for f in ('host.bc', 'device.bc', 'launches.json'):
                assert (d/f).is_file(), 'missing launch artefact: '+f
            for kind in ('host', 'device'):
                self.command(f'{m}/{n}-verify-{kind}', [self.tool(m, 'opt'), '-passes=verify', '-disable-output', d/(kind+'.bc')])
            launches = {x['entry'] for x in json.loads((d/'launches.json').read_text())}
            for entry in ('rung4_grover', 'rung4_tuple'):
                assert entry in launches, 'missing launch '+entry
                for f in ('launch.bennett', 'build.json', 'certificate.json', 'coverage.json', 'blockers.json', 'launch-record.json'):
                    assert (d/entry/f).is_file(), f'missing {entry}/{f}'
                assert json.loads((d/entry/'blockers.json').read_text()) == [], 'nonempty production blockers'
                check_coverage(json.loads((d/entry/'coverage.json').read_text()))
            bits = bits_for(n)
            a = region_exponent(self.images[m][n].stat().st_size)
            check_reference_object((d/'rung4_tuple/oracle-0/ref.bennett').read_text(), bits, a)

    def comparison(self, m):
        runtime, arith = self.libraries(m)
        for n in ORACLE_SIZES:
            d = self.run/str(m)/f'N{n}'
            bits = bits_for(n)
            harness = d/'native_compare.c'
            harness.write_text(f'#define RUNG4_BITS {bits}\n#define RUNG4_LEN {self.images[m][n].stat().st_size}\n'+(HERE/'fixtures/native_compare.c').read_text())
            exe = d/'native-compare'
            record = d/'native-comparison.json'
            self.command(f'{m}/{n}-driver-comparison', [self.driver_exe, '-bennett-dialect='+self.manifest['dialect'], '-bennett-isa='+self.manifest['isa'], '-bennett-out='+str(d/'comparison'),
                         f'--bennett-llvm-major={m}', f'--bennett-materialised={d/"out/device.bc"}', f'--bennett-runtimes={runtime}', f'--bennett-arith={arith}',
                         f'--bennett-native-input={harness}', f'--bennett-comparison-record={record}', f'--bennett-comparison-output={exe}'], timeout=600)
            value = json.loads(record.read_text())
            assert value.get('llvm_major') == m, 'comparison record major differs'
            argv = value.get('link_argv') or []
            assert '-nostdlib' in argv and not any(a.startswith(('-l', '-L')) for a in argv), 'host libc forbidden in the §17.7 comparison'
            got = [int(c) for c in self.command(f'{m}/{n}-comparison-run', [exe], timeout=600).decode().strip()]
            assert got == self.tables[n], f'materialised comparison differs from native at N={n}'

    # ------------ executors
    def admission(self, m, name):
        exe = self.executor(name)
        for n in BUILD_SIZES:
            d = self.run/str(m)/f'N{n}'/'out'
            for obj in (d/'rung4_tuple/oracle-0/ref.bennett', d/'rung4_tuple/launch.bennett', d/'rung4_grover/launch.bennett'):
                self.command(f'{m}/{name}-{n}-check-{obj.parent.name}-{obj.stem}', [exe, 'check', obj])

    def region_file(self, m, n):
        data = self.images[m][n].read_bytes()
        a = region_exponent(len(data))
        path = self.run/str(m)/f'N{n}'/'region.bin'
        path.write_bytes(data+bytes((1 << a)-len(data)))
        return path, len(data), a

    def budget_arg(self):
        b = PINS['budget']
        return f'{b["steps"]},{b["heap"]},{b["stack"]},{b["depth"]}'

    def sweep(self, tag, items, action):
        """Run action(item) concurrently; first failure aborts the row with its context; records a compact JSONL."""
        log = self.run/(tag+'.jsonl')
        log.parent.mkdir(parents=True, exist_ok=True)
        started = time.monotonic()
        results = []
        with ThreadPoolExecutor(max_workers=int(self.args.jobs)) as pool:
            for r in pool.map(action, items):
                results.append(r)
        log.write_text(''.join(json.dumps(r)+'\n' for r in results))
        self.manifest['evidence'].setdefault('sweeps', {})[tag] = {'count': len(results), 'elapsed_seconds': round(time.monotonic()-started, 3), 'sha256': sha(log)}
        return results

    def quiet(self, argv, timeout=None):
        """One domain element. Failures are loud; successes are aggregated by sweep, not one file per element."""
        argv = list(map(str, argv))
        try:
            p = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout or self.args.sweep_timeout)
        except subprocess.TimeoutExpired as exc:
            raise GateError(f'element timeout: {" ".join(argv)}') from exc
        if p.returncode:
            raise GateError(f'element exit {p.returncode}: {" ".join(argv)}\n{p.stderr.decode(errors="replace")}')
        return json.loads(p.stdout)

    def oracle_runs(self, m, name):
        exe = self.executor(name)
        for n in ORACLE_SIZES:
            region, length, _ = self.region_file(m, n)
            obj = self.run/str(m)/f'N{n}'/'out/rung4_tuple/oracle-0/ref.bennett'
            table = self.tables[n]
            base = [exe, 'run-oracle', obj, 'ref', '--mem-file', f'm0={region}', '--arg', length, '--budget', self.budget_arg()]
            def one(x):
                v = self.quiet(base+['--input', x])
                oracle_row(v, table[x], PINS['budget']['steps'])
                return {'x': x, **v}
            rows = self.sweep(f'{m}/{name}-oracle-{n}', range(1 << bits_for(n)), one)
            self.manifest['evidence'].setdefault('oracle', {}).setdefault(str(m), {}).setdefault(name, {})[str(n)] = {
                'label': 'exhaustive over the rowid register at the fixed image', 'cases': len(rows), 'matches': sum(r['payload'] for r in rows),
                'max_steps': max(r['steps'] for r in rows), 'min_steps': min(r['steps'] for r in rows), 'faults': 0}

    def tuples(self, m, name):
        exe = self.executor(name)
        for n in ORACLE_SIZES:
            bits = bits_for(n)
            obj = self.run/str(m)/f'N{n}'/'out/rung4_tuple/launch.bennett'
            allowances = launch_allowances(obj.parent)
            table = self.tables[n]
            def one(element):
                x, t, e = element
                got = tuple_result(self.quiet([exe, 'run-program', obj, 'rung4_tuple', '--args', ','.join([str(x), str(t), str(e), *allowances]), '--budget', self.budget_arg()]))
                want = expected_tuple(x, t, e, table[x], bits)
                assert got == want, f'tuple mismatch N={n} (x,t,e)=({x},{t},{e}): got {got:#x}, want {want:#x}'
                return {'x': x, 't': t, 'e': e, 'tuple': got}
            rows = self.sweep(f'{m}/{name}-tuple-{n}', domain(bits), one)
            assert len(rows) == domain_size(bits), 'domain not fully enumerated'
            self.manifest['evidence'].setdefault('tuples', {}).setdefault(str(m), {}).setdefault(name, {})[str(n)] = {
                'label': 'exhaustive over x, t, e at the fixed image (no borrowed ancillae in the object)', 'elements': len(rows)}

    def step_checks(self, m, name):
        exe = self.executor(name)
        for n in ORACLE_SIZES:
            bits = bits_for(n)
            region, length, _ = self.region_file(m, n)
            obj = self.run/str(m)/f'N{n}'/'out/rung4_tuple/oracle-0/ref.bennett'
            out = self.command(f'{m}/{name}-step-check-{n}', [exe, 'step-check', obj, 'ref', '--exhaustive', '--max-bits', bits, '--mem-file', f'm0={region}', '--arg', length, '--budget', self.budget_arg()], timeout=self.args.sweep_timeout)
            step_report(json.loads(out), bits)

    def grover_state(self, m, name, n):
        exe = self.executor(name)
        d = self.run/str(m)/f'N{n}'
        state = d/f'{name}-state.json'
        data = self.images[m][n].read_bytes()
        a = region_exponent(len(data))
        plan = self.plans[n]
        obj = d/'out/rung4_grover/launch.bennett'
        allowances = launch_allowances(obj.parent)
        out = self.command(f'{m}/{name}-{n}-grover-state', [exe, 'run-program', obj, 'rung4_grover', '--args', ','.join(allowances), '--state-out', state, '--budget', self.budget_arg()], timeout=self.args.sweep_timeout)
        self.success_only(json.loads(out))
        dump = json.loads(state.read_text(), parse_float=finite_json_number, parse_constant=finite_json_number)
        image = (data+bytes((1 << a)-len(data))).hex()
        return check_state(dump, bits_for(n), self.tables[n], a, plan['k'], image), plan

    @staticmethod
    def success_only(value):
        assert value['status'] == 'COMPLETED' and value['fault'] is None and not value['faults'], 'Grover run faulted'
        assert not value['host_requests'], 'unmodelled host effect in Grover run'

    def amplitudes(self, m, name):
        for n in GROVER_SIZES:
            result, plan = self.grover_state(m, name, n)
            assert abs(result['success']-plan['success']) < 1e-9, 'success probability differs from the closed form'
            self.manifest['evidence'].setdefault('grover', {}).setdefault(str(m), {}).setdefault(name, {})[str(n)] = {
                'label': 'numerical (binary64 amplitude dump, tolerance %g); branches enumerated by the executor' % AMPLITUDE_TOLERANCE,
                **result, 'M': plan['M'], 'k': plan['k'], 'success_closed_form': plan['success']}

    def outcomes(self, m, name):
        exe = self.executor(name)
        for n in GROVER_SIZES:
            bits = bits_for(n)
            table = self.tables[n]
            probs = [abs(a)**2 for a in simulate(table, self.plans[n]['k'])]
            marked = [x for x, b in enumerate(table) if b]
            vectors = {'half': [0.5]*bits, 'zero': [0.0]*bits}
            for x in marked or [0, (1 << bits)-1]:
                vectors[f'select-{x}'] = draws_for(probs, bits, x)
            obj = self.run/str(m)/f'N{n}'/'out/rung4_grover/launch.bennett'
            allowances = launch_allowances(obj.parent)
            for label, draws in vectors.items():
                value = json.loads(self.command(f'{m}/{name}-{n}-grover-{label}', [exe, 'run-program', obj, 'rung4_grover', '--args', ','.join(allowances), '--draws', ','.join(map(repr, draws)), '--budget', self.budget_arg()], timeout=self.args.sweep_timeout))
                self.success_only(value)
                got = int(value['results'][0]['value'], 0)
                want = predict_outcome(probs, bits, draws)
                assert got == want, f'Grover outcome {got} under draws {label} differs from predicted {want} at N={n}'

    def certificate(self, m):
        for n in ORACLE_SIZES:
            d = self.run/str(m)/f'N{n}'/'out/rung4_tuple'
            bits = bits_for(n)
            a = region_exponent(self.images[m][n].stat().st_size)
            out = self.command(f'{m}/{n}-checker', [self.check_exe, 'check', d/'certificate.json', '--object', d/'oracle-0/ref.bennett'], timeout=self.args.sweep_timeout)
            verdict = json.loads(out)
            check_certificate_verdict(verdict, domain_size(bits), a)
            self.manifest['evidence'].setdefault('certificate', {}).setdefault(str(m), {})[str(n)] = verdict

    def estimate(self, m):
        d = self.run/str(m)/f'N{ESTIMATE_SIZE}'/'out/rung4_grover/launch.bennett'
        value = json.loads(self.command(f'{m}/estimate', [self.estimate_exe, d, '--target', self.target], timeout=600))
        check_estimate(value, sha(d))
        self.manifest['evidence'].setdefault('estimate', {})[str(m)] = {'grover': self.plans[ESTIMATE_SIZE], 'value': value}

    # ------------ diagnostics on the published reduced control; these never satisfy a required row
    def mini(self):
        m = re.search(r'```isa object=rung4-mini\n(.*?)```', self.spec, re.S)
        path = self.run/'rung4-mini.bennett'
        path.write_text(m[1])
        lines = re.search(r'```text run=rung4-mini\n(.*?)```', self.spec, re.S)[1].splitlines()
        return path, lines

    def control(self, name):
        exe = self.executor(name)
        path, lines = self.mini()
        self.command(f'control/{name}-check', [exe, 'check', path])
        for i, line in enumerate(lines):
            f = re.fullmatch(r'input=(\d+) mem=m0:(\w+) arg=(\d+) budget=([\d,]+) -> code=(\d+) payload=(\d+) steps=(\d+)', line)
            v = json.loads(self.command(f'control/{name}-run-{i}', [exe, 'run-oracle', path, 'ref', '--input', f[1], '--mem', 'm0='+f[2], '--arg', f[3], '--budget', f[4]]))
            assert (v['code'], v['payload'], v['steps']) == (int(f[5]), int(f[6]), int(f[7])), 'published rung4-mini run differs: '+line
        s = re.search(r'```text step-check=rung4-mini\nmem=m0:(\w+) arg=(\d+) budget=([\d,]+)', self.spec)
        step_report(json.loads(self.command(f'control/{name}-step', [exe, 'step-check', path, 'ref', '--exhaustive', '--mem', 'm0='+s[1], '--arg', s[2], '--budget', s[3]])), 3)

    def grover_tu(self, m):
        d = self.run/str(m)/'mini'
        d.mkdir()
        for name, fn in (('rung4-minidb', 'db.c'), ('rung4-grover-tu', 'tu.c')):
            (d/fn).write_text(re.search(r'```c example='+name+r' verdict=accept\n(.*?)```', self.spec, re.S)[1])
        for fn in ('db', 'tu'):
            self.command(f'{m}/mini-{fn}', [self.tool(m, 'clang'), '-O2', '-flto', '-emit-llvm', '-I', self.markers, '-c', d/f'{fn}.c', '-o', d/f'{fn}.bc'])
        self.command(f'{m}/mini-link', [self.tool(m, 'llvm-link'), d/'db.bc', d/'tu.bc', '-o', d/'p.bc'])
        self.command(f'{m}/mini-postlink', [self.tool(m, 'opt'), '-passes=lto<O3>', '-S', d/'p.bc', '-o', d/'post.ll'])
        self.command(f'{m}/mini-production', ['env', '-u', 'BENNETT_ARITH_BITCODE', 'BENNETT_ARITH_PINNED_ROOT='+str(self.pub_arith/'dist'), self.tool(m, 'opt'), '-load-pass-plugin='+str(self.plugin_build/f'libbennett-plugin-{m}.so'), '-passes=bennett',
                     '-bennett-dialect='+self.manifest['dialect'], '-bennett-isa='+self.manifest['isa'], '-bennett-budget-default=400,0,512,8,0', '-bennett-out='+str(d/'out'), '-disable-output', d/'post.ll'])
        assert (d/'out/launches.json').is_file(), 'published Grover TU produced no launches'

    def mem_file(self, name):
        exe = self.executor(name)
        path, _ = self.mini()
        region = self.run/'mini-region.bin'
        region.write_bytes(bytes.fromhex('0007000a0165030704c8052a06630796'))
        v = json.loads(self.command(f'probe/{name}-mem-file', [exe, 'run-oracle', path, 'ref', '--input', '4', '--mem-file', f'm0={region}', '--arg', '16', '--budget', '400,0,512,8']))
        assert (v['code'], v['payload'], v['steps']) == (0, 1, 71), 'region file differs from the published run'

    def state_out(self, name):
        exe = self.executor(name)
        fixture = self.sources.paths['bennett-isa-spec']/'reference-interpreter/tests/fixtures/registers.bisa'
        observations = []
        # The published Bell measurement has support only on 00 and 11, each with weight 1/2.
        for draw, bits, expected in (('0.0', '00', 0), ('0.9', '11', 3)):
            value = json.loads(self.command(f'probe/{name}-measured-control-{bits}',
                              [exe, 'run-program', fixture, 'measurement', '--draws', draw]))
            self.success_only(value)
            assert value['run_state'] == 'CLOSED', 'measurement launch not closed'
            measurements = value['measurements']
            assert len(measurements) == 1, 'measurement control must execute exactly one measurement'
            measured = measurements[0]
            assert measured['bits'] == bits and measured['value'] == expected, 'measurement differs from the Bell draw rule'
            assert value['results'] == [{'type': 'i2', 'value': str(expected)}], 'measurement result differs from readout'
            observations.append({'draw': draw, 'measurement': measured})
        state = {'label': 'measured Bell basis states; diagnostic only',
                 'fixture_sha256': sha(fixture), 'observations': observations}
        (self.run/f'{name}-probe-state.json').write_text(json.dumps(state, indent=2)+'\n')
        self.manifest['evidence'].setdefault('reduced_measured_control', {})[name] = state

    def estimator_cli(self, m):
        path, _ = self.mini()
        out = self.command('probe/estimate-cli', [self.estimate_exe, path, '--target', self.target])
        try:
            value = json.loads(out)
        except ValueError:
            raise GateError('estimator returned no JSON estimate: '+out.decode(errors='replace').strip())
        assert 'padded' in value, 'estimator reports no padded cost'

    def checker_cli(self, m):
        # A malformed certificate must be a checker rejection (exit 1 with a diagnosis), not "not implemented".
        bogus = self.run/'probe-bogus-certificate.json'
        bogus.write_text('{}\n')
        p = subprocess.run(list(map(str, [self.check_exe, 'check', bogus])), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
        text = (p.stdout+p.stderr).decode(errors='replace')
        (self.run/'probe-checker.txt').write_text(f'exit {p.returncode}\n{text}')
        if re.search(r'not (yet )?implemented|usage', text, re.I):
            raise GateError('certificate checker has no check command for rung-4 certificates: '+text.strip()[:300])

    # ------------ reporting
    def finish(self):
        table = ['| Stage | LLVM 18 | LLVM 23 |', '|---|---|---|']
        for stage in STAGES:
            cells = []
            for m in (18, 23):
                r = self.rows[m].get(stage, {'status': 'NOT RUN', 'beads': []})
                cells.append(r['status']+(' · '+', '.join(r['beads']) if r['beads'] else ''))
            table.append('| '+stage+' | '+' | '.join(cells)+' |')
        (self.run/'report.json').write_text(json.dumps({'manifest': self.manifest, 'stages': self.rows}, indent=2, default=str)+'\n')
        (self.run/'table.md').write_text('\n'.join(table)+'\n')
        (self.out/'latest').write_text(str(self.run)+'\n')
        print('\n'+'\n'.join(table)+'\n\nEvidence: '+str(self.run), flush=True)
        if getattr(self.args, 'record', False):
            self.record()
        return int(any(self.rows[m].get(s, {}).get('status') != 'PASS' for m in (18, 23) for s in STAGES))

    def record(self):
        dest = HERE/'evidence'/self.run.name
        dest.mkdir(parents=True)
        for f in ('report.json', 'table.md'):
            shutil.copyfile(self.run/f, dest/f)
        inventory = {}
        for p in sorted(self.run.rglob('*')):
            # Generated program text is large and rebuilt from pins; every recorded file is hashed.
            if p.is_file():
                inventory[str(p.relative_to(self.run))] = {'bytes': p.stat().st_size, 'sha256': sha(p)}
        (dest/'raw-evidence-sha256.json').write_text(json.dumps(inventory, indent=0)+'\n')

    def shared(self, stage, action, deps=()):
        self.stage(18, stage, action, deps)
        self.rows[23][stage] = dict(self.rows[18][stage])

    def main(self):
        self.shared('sibling revisions', self.sources.prepare)
        frozen = ('sibling revisions',)
        self.shared('reference build', self.reference, frozen)
        self.shared('fixtures', self.fixtures, frozen)
        self.shared('SQLite source unmodified', self.sqlite_source)
        self.shared('independent predicate', self.predicate)
        self.shared('plugin build', self.plugin, frozen)
        self.shared('VM build', self.vm, frozen)
        self.shared('driver build', self.driver, frozen)
        self.shared('estimator build', self.estimator, frozen)
        self.shared('checker build', self.checker, frozen)
        base = ('fixtures', 'SQLite source unmodified', 'independent predicate')

        def major(m):
            self.stage(m, 'host image = native oracle = predicate', lambda: self.host(m), base)
            self.stage(m, 'musl C front end', lambda: self.frontend(m), ('host image = native oracle = predicate', 'plugin build'))
            self.stage(m, 'runtime bitcode link', lambda: self.runtime_link(m), ('musl C front end',))
            self.stage(m, 'post-link LTO', lambda: self.postlink(m), ('runtime bitcode link',))
            self.stage(m, 'runtime libc closure', lambda: self.closure(m), ('post-link LTO',))
            self.stage(m, 'coherent production', lambda: self.production(m), ('post-link LTO',))
            self.stage(m, 'launch artefacts and oracle shape', lambda: self.artefacts(m), ('coherent production',))
            self.stage(m, 'materialised comparison = native', lambda: self.comparison(m), ('launch artefacts and oracle shape', 'runtime libc closure', 'driver build'))
            for name in EXECUTORS:
                deps = ('launch artefacts and oracle shape', 'runtime libc closure', *(('VM build',) if name == 'VM' else ('reference build',)))
                adm = f'{name} admission'
                self.stage(m, adm, lambda: self.admission(m, name), deps)
                ora = f'{name} (a) oracle = native, fault rate zero'
                self.stage(m, ora, lambda: self.oracle_runs(m, name), (adm, 'materialised comparison = native'))
                self.stage(m, f'{name} (a) full tuple on D', lambda: self.tuples(m, name), (ora,))
                self.stage(m, f'{name} (a) literal step check', lambda: self.step_checks(m, name), (ora,))
                amp = f'{name} (b) amplitudes'
                self.stage(m, amp, lambda: self.amplitudes(m, name), (ora,))
                self.stage(m, f'{name} (b) outcomes', lambda: self.outcomes(m, name), (amp,))
            self.stage(m, '(a) certificate verdict', lambda: self.certificate(m), ('reference (a) full tuple on D', 'checker build'))
            self.stage(m, '(c) resource estimate at N=2^20', lambda: self.estimate(m), ('launch artefacts and oracle shape', 'estimator build'))
            for name in EXECUTORS:
                vmdep = ('VM build',) if name == 'VM' else ('reference build',)
                self.stage(m, f'{name} reduced control (diagnostic)', lambda: self.control(name), ('fixtures', *vmdep))
                self.stage(m, f'{name} --mem-file (diagnostic)', lambda: self.mem_file(name), (f'{name} reduced control (diagnostic)',))
                self.stage(m, f'{name} measured control (diagnostic)', lambda: self.state_out(name), (f'{name} reduced control (diagnostic)',))
            self.stage(m, 'published Grover TU production (diagnostic)', lambda: self.grover_tu(m), ('fixtures', 'plugin build'))
            self.stage(m, 'estimator CLI (diagnostic)', lambda: self.estimator_cli(m), ('estimator build', 'fixtures'))
            self.stage(m, 'checker CLI (diagnostic)', lambda: self.checker_cli(m), ('checker build',))
        with ThreadPoolExecutor(max_workers=2) as pool:
            list(pool.map(major, (18, 23)))
        self.shared('images agree across majors', self.images_agree, ())
        return self.finish()


def main():
    if not __debug__:
        raise GateError('Python optimisation disables acceptance assertions; run gate.sh without -O')
    for key in ('DISPLAY', 'WAYLAND_DISPLAY', 'DBUS_SESSION_BUS_ADDRESS', 'BROWSER', 'PYTHONOPTIMIZE'):
        os.environ.pop(key, None)
    os.environ.setdefault('TMPDIR', str(CONFORMANCE/'.tmp/rung-4/tmp'))
    Path(os.environ['TMPDIR']).mkdir(parents=True, exist_ok=True)
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', help='parent for fresh timestamped run directories')
    for name in REPOSITORIES:
        p.add_argument('--'+name+'-rev', help='override pins.json with a frozen bennett-'+name+' commit/ref')
    p.add_argument('--reference', help='explicit reference binary; conflicts with --isa-spec-rev')
    p.add_argument('--timeout', type=int, default=600, help='seconds per ordinary command')
    p.add_argument('--production-timeout', type=int, default=1800, help='seconds per plugin run')
    p.add_argument('--sweep-timeout', type=int, default=7200, help='seconds per oracle element or Grover run')
    p.add_argument('--jobs', type=int, default=4)
    p.add_argument('--record', action='store_true', help='copy compact evidence into ladder/rung-4/evidence')
    args = p.parse_args()
    select_revisions(p, args, PINS)
    return Gate(args).main()


if __name__ == '__main__':
    raise SystemExit(main())
