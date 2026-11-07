#!/usr/bin/env python3
"""North Star §8 rung 3; incomplete prerequisites are red, never expected passes."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tomllib
sys.path.insert(0, str(Path(__file__).resolve().parents[1]/'common'))
from gate_support import GateError, RecordedGate, git, sha
from native_audit import audit_comparison

HERE = Path(__file__).resolve().parent
CONFORMANCE = HERE.parents[1]
WORKSPACE = CONFORMANCE.parent
PINS = json.loads((HERE/'pins.json').read_text())
BUDGET = '10000000,1048576,65536,256'
WORKLOADS = ('hello', 'printf_fmt')
CORPUS_STAGES = ('musl C front end',
          'runtime bitcode link', 'post-link LTO', 'reversible production',
          'launch artefacts', 'native comparison',
          'reference admission', 'reference forward = native', 'reference reset', 'reference tape replay', 'reference corrupt-tape rejection',
          'VM admission', 'VM forward = native', 'VM reset', 'VM tape replay', 'VM corrupt-tape rejection')

def workload_stage(workload, stage):
    return stage if workload == 'hello' else workload+': '+stage

STAGES = ('fixtures', 'plugin build', 'VM build', 'driver build', 'R contract (prerequisite)',
          *(workload_stage(w,s) for w in WORKLOADS for s in CORPUS_STAGES),
          'runtime libc/console audit (diagnostic)', 'R-mode probe (diagnostic)',
          'driver probe (diagnostic)',
          'reference device/reset control (diagnostic)', 'reference replay API (diagnostic)',
          'VM device/reset control (diagnostic)', 'VM replay API (diagnostic)')

def validate_contract(pins, producer_text, spec):
    """Check the full compatibility unit against the pinned producer and publication."""
    pair = pins['dialect'], pins['isa']
    producer = tomllib.loads(producer_text)
    declared = producer['dialect'], producer['isa']
    for field, version in (('bennett-dialect-spec', declared[0]), ('bennett-isa-spec', declared[1])):
        if field in producer.get('specs', {}) and producer['specs'][field] != version:
            raise GateError('producer contract declarations disagree: '+field)
    declaration = next((p for p in re.split(r'\n\s*\n', spec)
                        if p.startswith('**Versioning.**')), '')
    versioning = re.search(r'\*\*Versioning\.\*\* Dialect ([\d.]+):.*?targets ISA ([\d.]+)\.',
                           declaration, flags=re.S)
    if pair[1] not in ('0.8', '0.9') or not versioning or pair != versioning.groups() or pair != declared:
        raise GateError(f'unknown or mismatched R contract: pins {pair}, producer {declared}, '
                        f'publication {versioning.groups() if versioning else "undeclared"}')
    if pair[1] == '0.8':
        if 'On ISA 0.8 a module has verdict blocked:BD-B-ISA7' not in spec:
            raise GateError('unknown ISA-0.8 R contract: missing §11.5/17.6 prerequisite')
        raise GateError('BD-B-ISA7: dialect §11.5/17.6 blocks every ISA-0.8 whole-program R entry; typed pre-instruction budget fault requires ISA-7')
    return pair

class Gate(RecordedGate):
    def __init__(self, args):
        self.args=args
        self.contract=PINS['dialect'],PINS['isa']
        self.tmp=CONFORMANCE/'.tmp'
        self.out=Path(args.output).resolve() if args.output else self.tmp/'rung-3'
        self.out.mkdir(parents=True, exist_ok=True)
        self.run=self.out/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
        self.run.mkdir()
        for m in (18,23):
            (self.run/str(m)).mkdir()
            (self.run/str(m)/'printf_fmt').mkdir()
        self.rows={m:{} for m in (18,23)}
        self.known=json.loads((HERE/'blockers.json').read_text())
        self.stages=STAGES
        self.forward_results={}
        self.sysroots={m:Path(getattr(args,f'sysroot{m}',None) or
                         WORKSPACE/f'bennett-runtimes/dist/llvm{m}/musl-sysroot').resolve() for m in (18,23)}
        self.manifest={'time_utc':datetime.now(timezone.utc).isoformat(), 'commands':[],
                       'pins':dict(PINS), 'dialect':self.contract[0], 'isa':self.contract[1], 'budget':BUDGET,
                       'environment':{k:os.environ.get(k) for k in
                           ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','BROWSER','TMPDIR')},
                       'sysroots':{str(m):str(p) for m,p in self.sysroots.items()},
                       'gate_sha256':sha(HERE/'gate.py'),
                       'native_audit_sha256':sha(HERE/'native_audit.py'),
                       'shared_support_sha256':sha(HERE.parent/'common/gate_support.py'),
                       'blockers_sha256':sha(HERE/'blockers.json')}

    def workdir(self,m,workload='hello'):
        assert workload in WORKLOADS, 'unknown workload: '+workload
        return self.run/str(m) if workload=='hello' else self.run/str(m)/workload

    def expected(self,workload):
        if workload=='hello':
            return json.loads((HERE/'fixtures/expect.json').read_text())
        value=json.loads((HERE/'fixtures'/f'{workload}.expect.json').read_text())
        return {'stdout_hex':''.join(line['stdout'] for line in value['lines']).encode('ascii').hex(),
                'exit_status':value['exit_status']}

    def freeze(self, repo, rev):
        rev=git(WORKSPACE/repo,'rev-parse',rev)
        self.manifest['pins'][repo]=rev
        src=self.tmp/(f'rung3-{repo}-'+rev)
        # Verify immutable source cache, excluding its untracked build outputs.
        archive=self.command(f'build/{repo}-archive',['git','-C',WORKSPACE/repo,'archive',rev])
        if not src.exists():
            src.mkdir()
            with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
                tar.extractall(src,filter='data')
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            for member in tar.getmembers():
                if member.isfile():
                    assert (src/member.name).read_bytes()==tar.extractfile(member).read(), 'dirty source cache: '+member.name
        return src

    def plugin(self):
        for m in (18,23):
            for tool in ('clang','opt','llvm-link'):
                self.command(f'build/llvm{m}-{tool}-version',[f'/usr/lib/llvm-{m}/bin/{tool}','--version'])
        src=self.freeze('bennett-plugin',self.args.plugin_rev)
        self.plugin_build=src/'build'
        self.command('build/plugin-configure',['cmake','-S',src,'-B',self.plugin_build])
        self.command('build/plugin-build',['cmake','--build',self.plugin_build,'-j',self.args.jobs],timeout=1800)
        self.manifest['plugin_sha256']={str(m):sha(self.plugin_build/f'libbennett-plugin-{m}.so') for m in (18,23)}
        self.publications={r:self.freeze(r,PINS[r]) for r in ('bennett-runtimes','bennett-arith')}

    def vm(self):
        repo=WORKSPACE/'bennett-vm'
        rev=git(repo,'rev-parse',self.args.vm_rev)
        self.manifest['pins']['bennett-vm']=rev
        src=self.tmp/('vm-'+rev)
        if not src.exists():
            self.command('build/vm-worktree',['git','-C',repo,'worktree','add','--detach',src,rev])
        assert git(src,'rev-parse','HEAD')==rev, 'VM HEAD differs from pin'
        assert (src/'.git').is_file(), 'VM must be a detached worktree'
        head=self.command('build/vm-head',['git','-C',src,'status','--porcelain','--untracked-files=no'])
        assert not head, 'VM detached worktree dirty'
        # symbolic-ref returns 1 for detached HEAD; inspect rev-parse instead.
        assert git(src,'rev-parse','--abbrev-ref','HEAD')=='HEAD', 'VM is not detached'
        self.command('build/vm-build',['cargo','build','--offline','--manifest-path',src/'Cargo.toml','--target-dir',src/'target'],timeout=600)
        self.vm_exe=src/'target/debug/bennett-vm'
        self.manifest['vm_sha256']=sha(self.vm_exe)

    def driver(self):
        src=self.freeze('bennett-driver',self.args.driver_rev)
        self.command('build/driver-build',['cargo','build','--offline','--manifest-path',src/'Cargo.toml','--target-dir',src/'target'],timeout=600)
        self.driver_exe=src/'target/debug/bennett-driver'
        self.manifest['driver_sha256']=sha(self.driver_exe)

    def fixtures(self):
        pins=json.loads((HERE/'fixtures/sha256.json').read_text())
        for f,digest in pins.items():
            assert sha(HERE/'fixtures'/f)==digest, 'fixture hash mismatch: '+f
        expect=json.loads((HERE/'fixtures/expect.json').read_text())
        ids=set()
        for spec in (f'DLT-{self.contract[0]}',f'ISA-{self.contract[1]}'):
            ids.update(n['id'] for n in json.loads((CONFORMANCE/f'clauses/{spec}.json').read_text())['nodes'])
        assert expect['clauses'] and set(expect['clauses'])<=ids, 'missing/invalid clause tags'
        for workload in WORKLOADS:
            assert workload+'.c' in pins, 'unhashed workload: '+workload
            value=self.expected(workload)
            assert value['exit_status']==0, 'fixture requires a zero return status'
            assert bytes.fromhex(value['stdout_hex']), 'empty workload oracle'
        assert 'printf_fmt.expect.json' in pins, 'unhashed formatted oracle'
        self.manifest['fixtures']=pins

    def clang(self,m):
        return f'/usr/lib/llvm-{m}/bin/clang'

    def frontend(self,m,workload='hello'):
        root=self.sysroots[m]
        self.command(f'{m}/{workload}/frontend',[self.clang(m),'--target=x86_64-unknown-linux-musl',
            '--sysroot='+str(root),'-nostdinc','-isystem',root/'include','-O2','-flto','-emit-llvm','-c',
            HERE/'fixtures'/f'{workload}.c','-o',self.workdir(m,workload)/'input.bc'])
        assert (root/'include/stdio.h').is_file(), 'missing musl stdio.h: '+str(root)
        self.manifest.setdefault('musl_stdio_sha256',{})[str(m)]=sha(root/'include/stdio.h')

    def runtime_files(self,m):
        runtime=Path(getattr(self.args,f'runtimes{m}',None) or self.publications['bennett-runtimes']/f'dist/llvm{m}/bennett-runtimes.bc').resolve()
        if not runtime.is_file():
            raise GateError('missing runtime publication: '+str(runtime))
        if not getattr(self.args,f'runtimes{m}',None):
            expected=git(WORKSPACE/'bennett-runtimes','show',PINS['bennett-runtimes']+f':dist/llvm{m}/SHA256SUMS').split()[0]
            assert sha(runtime)==expected, 'runtimes publication differs from pinned revision'
        pins=tomllib.loads((self.plugin_build.parent/'SPEC_VERSIONS.toml').read_text())['arith']
        arith=self.publications['bennett-arith']/f'dist/llvm{m}/bennett-arith.bc'
        assert sha(arith)==pins[f'llvm{m}_sha256'], 'arith publication differs from plugin pin'
        self.manifest.setdefault('runtimes_sha256',{})[str(m)]=sha(runtime)
        self.manifest.setdefault('arith_sha256',{})[str(m)]=sha(arith)
        return runtime,arith

    def runtime_link(self,m,workload='hello'):
        runtime,arith=self.runtime_files(m)
        d=self.workdir(m,workload)
        self.command(f'{m}/{workload}/runtime-link',[f'/usr/lib/llvm-{m}/bin/llvm-link',d/'input.bc',runtime,arith,'--only-needed','-o',d/'linked.bc'])

    def postlink(self,m,workload='hello'):
        d=self.workdir(m,workload)
        self.command(f'{m}/{workload}/postlink',[f'/usr/lib/llvm-{m}/bin/opt','-passes=lto<O3>','-S',d/'linked.bc','-o',d/'postlink.ll'])

    def pipeline(self,m,probe=False,workload='hello'):
        d=self.workdir(m,workload)
        source=HERE/'fixtures/effect-console-r.ll' if probe else d/'postlink.ll'
        self.command(f'{m}/{workload}/'+('mode-probe' if probe else 'production'),[f'/usr/lib/llvm-{m}/bin/opt',
            '-load-pass-plugin='+str(self.plugin_build/f'libbennett-plugin-{m}.so'),'-passes=bennett',
            '-bennett-dialect='+self.contract[0],'-bennett-isa='+self.contract[1],'--bennett-tier=reversible','--bennett-entry=main',
            '-bennett-budget-default='+BUDGET+',65536','-bennett-out='+str(d/('probe-out' if probe else 'out')),
            '-disable-output',source])
        if probe:
            assert (d/'probe-out/launches.json').is_file(), 'R-mode probe produced no launches'

    def artifacts(self,m,workload='hello'):
        d=self.workdir(m,workload)/'out'
        for f in ('host.bc','device.bc','launches.json'):
            assert (d/f).is_file(), 'missing launch artefact: '+f
        for kind in ('host','device'):
            self.command(f'{m}/{workload}/verify-{kind}',[f'/usr/lib/llvm-{m}/bin/opt','-passes=verify','-disable-output',d/(kind+'.bc')])
        launches=json.loads((d/'launches.json').read_text())
        assert any(x['entry']=='main' for x in launches), 'missing main launch'
        for f in ('launch.bennett','build.json','certificate.json','coverage.json','blockers.json','launch-record.json'):
            assert (d/'main'/f).is_file(), 'missing main/'+f
        assert json.loads((d/'main/blockers.json').read_text())==[], 'nonempty production blockers'
        text=(d/'main/launch.bennett').read_text()
        assert 'fn r @' in text and 'effect console.write ' in text, 'no reversible console body'
        assert 'bennett.argv' in text, 'missing §19.2 CC argv launch stub'
        assert '"console"' in text, 'missing declared console model'

    def driver_probe(self,m):
        out=self.run/str(m)/'driver-probe'
        runtime,arith=self.runtime_files(m)
        self.command(f'{m}/driver-probe',[self.driver_exe,'--bennett-tier=reversible','--bennett-entry=main',
            *self.driver_publications(m,runtime,arith),
            '-bennett-dialect='+self.contract[0],'-bennett-isa='+self.contract[1],'-bennett-out='+str(out),HERE/'fixtures/effect-console-r.ll'])
        assert (out/'launches.json').is_file(), 'driver returned success without pipeline outputs'

    def driver_publications(self,m,runtime,arith):
        # Archived checkouts have no sibling publications; never use driver defaults.
        return ['--bennett-llvm-major='+str(m),
            '--bennett-plugin='+str(self.plugin_build/f'libbennett-plugin-{m}.so'),
            '--bennett-runtimes='+str(runtime),'--bennett-arith='+str(arith)]

    def native(self,m,workload='hello'):
        # This consumer manifest is gate glue, not an addition to the dialect contract.
        # Driver integration must publish an auditable comparison link; no host-libc fallback.
        d=self.workdir(m,workload); out=d/'out'; work=out/'native-work'
        runtime,arith=self.runtime_files(m)
        self.command(f'{m}/{workload}/driver-comparison',[self.driver_exe,'--bennett-tier=reversible','--bennett-entry=main',
            '-bennett-dialect='+self.contract[0],'-bennett-isa='+self.contract[1],
            '--bennett-materialised='+str(out/'device.bc'),*self.driver_publications(m,runtime,arith),
            '-bennett-out='+str(work),
            '--bennett-sysroot='+str(self.sysroots[m]),
            '--bennett-native-check='+str(self.publications['bennett-runtimes']/'scripts/native-check'),
            '--bennett-comparison-record='+str(out/'native-comparison.json'),
            '--bennett-comparison-output='+str(d/'native')])
        if not (out/'native-comparison.json').is_file():
            raise GateError('missing native comparison link record from driver')
        record=json.loads((out/'native-comparison.json').read_text())
        manifest=json.loads((work/'manifest.json').read_text()) if 'bitcode_link_argv' in record else {}
        argv=audit_comparison(self,m,out/'device.bc',runtime,arith,record,manifest,d/'native')
        self.command(f'{m}/{workload}/native-link',argv)
        dynamic=self.command(f'{m}/{workload}/native-dynamic',['readelf','-d',d/'native'])
        assert b'NEEDED' not in dynamic, 'comparison unexpectedly imports a shared library'
        headers=self.command(f'{m}/{workload}/native-headers',['readelf','-l',d/'native'])
        assert b'INTERP' not in headers, 'comparison unexpectedly has a dynamic interpreter'
        undefined=self.command(f'{m}/{workload}/native-undefined',[f'/usr/lib/llvm-{m}/bin/llvm-nm','--undefined-only',d/'native'])
        assert not undefined.strip(), 'native comparison unresolved imports'
        output=self.command(f'{m}/{workload}/native-run',[d/'native'])
        assert output==bytes.fromhex(self.expected(workload)['stdout_hex']), 'native stdout differs from corpus: '+workload
        (d/'native.stdout').write_bytes(output)

    def executor(self,name):
        exe=self.vm_exe if name=='VM' else Path(self.args.reference)
        assert exe.is_file(), 'missing executor: '+str(exe)
        self.manifest.setdefault('executor_sha256',{})[name]=sha(exe)
        return exe

    def obj(self,m,workload='hello'):
        return self.workdir(m,workload)/'out/main/launch.bennett'

    def admission(self,m,name,workload='hello'):
        for verb in ('check','hash'):
            self.command(f'{m}/{workload}/{name}-{verb}',[self.executor(name),verb,self.obj(m,workload)])

    def argv_replies(self,m,workload='hello'):
        # Gate adapter file from the driver: exact public executor --replies format.
        path=self.workdir(m,workload)/'out/main/argv-replies.json'
        replies=json.loads(path.read_text())
        assert len(replies)==1 and replies[0]['service']=='bennett.argv', 'need exactly one argv/envp reply'
        assert all(k in replies[0] for k in ('payload','reply')), 'incomplete argv/envp reply'
        return path

    @staticmethod
    def successful(value):
        assert value['fault'] is None and not value['faults'], 'faulted execution'
        # §19.2 requires exactly the argv request at launch; reject unmodelled console/libc calls.
        assert all(x.get('service')=='bennett.argv' for x in value['host_requests']), 'unmodelled host effect'
        assert value['results']==[{'type':'i32','value':'0'}], 'expected exit status i32 0'

    @staticmethod
    def console_tape(value,expected):
        tape=value['tape']
        assert tape and all(e['dev']=='console' and e['op']=='write' for e in tape), 'not a console-only tape'
        assert all(len(e['args'])==2 and e['args'][1]==len(bytes.fromhex(e['bytes'])) for e in tape), 'invalid console tape operands'
        assert b''.join(bytes.fromhex(e['bytes']) for e in tape)==expected, 'tape bytes differ from native'

    def execute(self,m,name,reverse=False,workload='hello'):
        tag=f'{m}/{workload}/{name}-'+('reset' if reverse else 'forward')
        replies=self.argv_replies(m,workload)
        argv=[self.executor(name),'run-program',self.obj(m,workload),'main','--budget',BUDGET,'--replies',replies]
        if reverse:
            argv+=['--reverse']
        value=json.loads(self.command(tag,argv))
        self.successful(value)
        assert value['status']==('REVERSED' if reverse else 'COMPLETED')
        assert value['run_state']=='CLOSED', 'launch not closed'
        native=(self.workdir(m,workload)/'native.stdout').read_bytes()
        self.console_tape(value,native)
        if reverse:
            assert value['reversed'] is True
            self.restored(value)
            r=json.loads(self.command(tag+'-history',[self.executor(name),'reverse',self.obj(m,workload),'main','--budget',BUDGET,'--replies',replies]))
            self.successful(r); self.restored(r)
            assert r['tape']==value['tape'], 'one-shot and retained-history tapes differ'
        else:
            assert bytes.fromhex(value['console_hex'])==native, 'console differs from native'
            assert value['effect_tape_length']==len(value['tape'])
            self.forward_results[(m,name,workload)]=value

    @staticmethod
    def restored(value):
        assert value['status']=='REVERSED'
        assert value['equals_entry'] is True, 'full state not reset'
        assert value['memory_equals_entry'] is True
        assert value['first_difference'] is None
        assert value['entry_memory_digest']==value['memory_digest']
        assert value['console_hex']=='', 'console not reset'
        assert value['effect_tape_length']==0, 'tape not reset'

    def replay(self,m,name,workload='hello',tampered=False):
        value=self.forward_results[(m,name,workload)]
        tape=self.workdir(m,workload)/(name+'-saved-tape.json')
        tape.write_text(json.dumps(value['tape'],indent=2)+'\n')
        base=[self.executor(name),'run-program',self.obj(m,workload),'main','--budget',BUDGET,'--entropy','',
              '--replies',self.argv_replies(m,workload)]
        if not tampered:
            r=json.loads(self.command(f'{m}/{workload}/{name}-replay',base+['--replay-tape',tape]))
            self.successful(r)
            for field in ('status','results','console_hex','tape','effect_tape_length','run_state'):
                assert r[field]==value[field], 'replay differs in '+field
            return
        corrupt=json.loads(tape.read_text())
        event=next(e for e in corrupt if e['bytes'])
        first=bytes.fromhex(event['bytes'])
        event['bytes']=bytes([first[0]^1]).hex()+first[1:].hex()
        bad=tape.with_name(name+'-tampered-tape.json')
        bad.write_text(json.dumps(corrupt)+'\n')
        try:
            result=json.loads(self.command(f'{m}/{workload}/{name}-replay-tampered',base+['--replay-tape',bad]))
        except GateError as exc:
            # A crash/timeout/unsupported option is not evidence of tape validation.
            assert 'replay' in str(exc).lower() and ('mismatch' in str(exc).lower() or 'diverg' in str(exc).lower()), str(exc)
        else:
            assert result['status']=='FAILED' and result.get('replay_mismatch'), 'executor accepted a tampered tape'

    def runtime_audit(self,m):
        runtime,_=self.runtime_files(m)
        nm=self.command(f'{m}/runtime-symbols',[f'/usr/lib/llvm-{m}/bin/llvm-nm','--defined-only',runtime]).decode()
        symbols={line.split()[-1] for line in nm.splitlines() if line.split()}
        # Publication audit, not a claim every symbol survives optimisation in hello.c.
        missing=sorted({'printf','puts','write','exit','__bennett_dev_console_write'}-symbols)
        if missing:
            raise GateError('runtime libc/console publication missing: '+', '.join(missing))

    def isa_contract(self):
        spec=self.command('contract/dialect',['git','-C',WORKSPACE/'bennett-dialect-spec','show',
            PINS['bennett-dialect-spec']+':'+PINS['dialect_spec_path']]).decode()
        producer=self.command('contract/producer',['git','-C',WORKSPACE/'bennett-plugin','show',
            self.manifest['pins']['bennett-plugin']+':SPEC_VERSIONS.toml']).decode()
        self.contract=validate_contract(PINS,producer,spec)

    def control(self,m,name):
        exe=self.executor(name); obj=HERE/'fixtures/executor-control.bisa'
        self.command(f'{m}/{name}-control-check',[exe,'check',obj])
        v=json.loads(self.command(f'{m}/{name}-control-forward',[exe,'run-cc',obj,'effects','--entropy','0b1621']))
        assert v['status']=='COMPLETED' and v['fault'] is None
        assert v['console_hex']=='0b1621' and len(v['tape'])==5
        r=json.loads(self.command(f'{m}/{name}-control-reset',[exe,'reverse',obj,'effects','--entropy','0b1621']))
        self.restored(r)
        assert r['tape']==v['tape'], 'control retained history changed tape'
        tape=self.run/str(m)/(name+'-control-tape.json')
        tape.write_text(json.dumps(v['tape'])+'\n')

    def replay_probe(self,m,name):
        tape=self.run/str(m)/(name+'-control-tape.json')
        self.command(f'{m}/{name}-replay-probe',[self.executor(name),'run-cc',HERE/'fixtures/executor-control.bisa','effects','--entropy','0b1621','--replay-tape',tape])

    def finish(self):
        table=['| Stage | LLVM 18 | LLVM 23 |','|---|---|---|']
        for stage in STAGES:
            cells=[]
            for m in (18,23):
                r=self.rows[m][stage]
                cells.append(r['status']+(' · '+', '.join(r['beads']) if r['beads'] else ''))
            table.append('| '+stage+' | '+' | '.join(cells)+' |')
        (self.run/'report.json').write_text(json.dumps({'manifest':self.manifest,'stages':self.rows},indent=2)+'\n')
        (self.run/'table.md').write_text('\n'.join(table)+'\n')
        (self.out/'latest').write_text(str(self.run)+'\n')
        print('\n'+'\n'.join(table)+'\n\nEvidence: '+str(self.run),flush=True)
        return int(any(r['status']!='PASS' for row in self.rows.values() for r in row.values()))

    def main(self):
        for stage,action in [('fixtures',self.fixtures),('plugin build',self.plugin),('VM build',self.vm),('driver build',self.driver)]:
            self.stage(18,stage,action)
            self.rows[23][stage]=dict(self.rows[18][stage])
        self.stage(18,'R contract (prerequisite)',self.isa_contract)
        self.rows[23]['R contract (prerequisite)']=dict(self.rows[18]['R contract (prerequisite)'])
        def major(m):
            for workload in WORKLOADS:
                row=lambda name:workload_stage(workload,name)
                self.stage(m,row('musl C front end'),lambda:self.frontend(m,workload),('fixtures',))
                self.stage(m,row('runtime bitcode link'),lambda:self.runtime_link(m,workload),(row('musl C front end'),'plugin build'))
                self.stage(m,row('post-link LTO'),lambda:self.postlink(m,workload),(row('runtime bitcode link'),))
                self.stage(m,row('reversible production'),lambda:self.pipeline(m,workload=workload),(row('post-link LTO'),'plugin build','R contract (prerequisite)'))
                self.stage(m,row('launch artefacts'),lambda:self.artifacts(m,workload),(row('reversible production'),))
                self.stage(m,row('native comparison'),lambda:self.native(m,workload),(row('launch artefacts'),'driver build'))
                for name in ('reference','VM'):
                    self.stage(m,row(name+' admission'),lambda:self.admission(m,name,workload),(row('launch artefacts'),)+(('VM build',) if name=='VM' else ()))
                    self.stage(m,row(name+' forward = native'),lambda:self.execute(m,name,workload=workload),(row('native comparison'),row(name+' admission')))
                    self.stage(m,row(name+' reset'),lambda:self.execute(m,name,True,workload=workload),(row(name+' forward = native'),))
                    self.stage(m,row(name+' tape replay'),lambda:self.replay(m,name,workload=workload),(row(name+' reset'),))
                    self.stage(m,row(name+' corrupt-tape rejection'),lambda:self.replay(m,name,workload=workload,tampered=True),(row(name+' tape replay'),))
            self.stage(m,'runtime libc/console audit (diagnostic)',lambda:self.runtime_audit(m),('plugin build',))
            self.stage(m,'R-mode probe (diagnostic)',lambda:self.pipeline(m,True),('fixtures','plugin build'))
            self.stage(m,'driver probe (diagnostic)',lambda:self.driver_probe(m),('driver build','fixtures','plugin build'))
            for name in ('reference','VM'):
                self.stage(m,name+' device/reset control (diagnostic)',lambda:self.control(m,name),('fixtures',)+(('VM build',) if name=='VM' else ()))
                self.stage(m,name+' replay API (diagnostic)',lambda:self.replay_probe(m,name),(name+' device/reset control (diagnostic)',))
        with ThreadPoolExecutor(max_workers=2) as pool:
            list(pool.map(major,(18,23)))
        return self.finish()


def main():
    if not __debug__:
        raise GateError('Python optimisation disables acceptance assertions; run gate.sh without -O')
    for key in ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','BROWSER','PYTHONOPTIMIZE'):
        os.environ.pop(key,None)
    os.environ['TMPDIR']=str(CONFORMANCE/'.tmp')
    Path(os.environ['TMPDIR']).mkdir(exist_ok=True)
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',help='parent for fresh timestamped run directories')
    for short,repo in [('plugin','bennett-plugin'),('vm','bennett-vm'),('driver','bennett-driver')]:
        p.add_argument('--'+short+'-rev',default=PINS[repo],help='frozen commit (default: pins.json)')
    for m in (18,23):
        p.add_argument('--sysroot'+str(m),help='Bennett musl sysroot with include/ and lib/; never host headers')
        p.add_argument('--runtimes'+str(m),help='explicit major-matched runtime publication override; hash recorded')
    p.add_argument('--reference',default=str(WORKSPACE/'bennett-isa-spec-review/reference-interpreter/target/release/bennett-isa-ref'))
    p.add_argument('--timeout',type=int,default=300)
    p.add_argument('--jobs',type=int,default=4)
    args=p.parse_args()
    if args.timeout<=0 or args.timeout>300:
        p.error('corpus timeout must be in 1..300 seconds')
    return Gate(args).main()

if __name__=='__main__':
    raise SystemExit(main())
