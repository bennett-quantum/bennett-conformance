#!/usr/bin/env python3
"""Rung 1: ordinary scalars, exhaustive Int8 inputs and the full North Star §5.1 D."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import io
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import threading
import tomllib
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'common'))
from gate_support import GateError, RecordedGate, git, sha
from revision_sources import RevisionSources, add_revision_options, validate_revision_options, apply_recorded_pins
import batch_reverse
import native_layout
from progress import write_summary

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
WORKSPACE=ROOT.parent
PINS=json.loads((HERE/'pins.json').read_text())
CASES={'r1_sat_add':2,'r1_gcd':2,'r1_popcount':1,'r1_sign_abs':1,'r1_lookup':1}
NATIVE_COUNT=sum(256**n for n in CASES.values())
BUDGET='256,0,32,32'
STAGES=('sibling revisions','reference build','toolchain','fixtures','plugin build','VM build','native O0 = O2','frontend',
        'runtime bitcode link','post-link LTO','pipeline','materialised native comparison',
        'reference admission','VM admission','reference input exhaustive = native (diagnostic)',
        'VM input exhaustive = native (diagnostic)','reference input zero faults (diagnostic)',
        'VM input zero faults (diagnostic)','reference input byte-exact reversal (diagnostic)',
        'VM input byte-exact reversal (diagnostic)','reference full D interface','VM full D interface',
        'reference exhaustive full D = native','VM exhaustive full D = native',
        'reference zero faults on D','VM zero faults on D',
        'reference byte-exact reversal on D','VM byte-exact reversal on D')

def frontend_applicability(header,major):
    """Read llvm-bcanalyzer's identification record, not module format VERSION."""
    block=re.search(rb'<IDENTIFICATION_BLOCK_ID\b[^>]*>(.*?)</IDENTIFICATION_BLOCK_ID>',header,re.S)
    record=re.search(rb'<STRING\b([^>]*)/>',block[1]) if block else None
    producer=''.join(chr(int(n)) for n in re.findall(rb'op\d+=(\d+)',record[1])) if record else ''
    version=re.match(r'LLVM(\d+(?:\.\d+){1,2})',producer)
    if not version:raise GateError('missing LLVM bitcode producer identification: '+producer)
    if int(version[1].split('.')[0])>major:
        return {'status':'N/A','beads':[],
                'error':f'§19.8 (frontend IR {version[1]} newer than LLVM {major})'}
    return None


def cell(row):
    detail=row['error'] if row['status']=='N/A' else ', '.join(row['beads'])
    return row['status']+(' · '+detail if detail else '')


def case_budget(case):
    # The linked ctpop helper has a larger retained call frame and eight
    # bit-iterations; reserve its full finite body without changing D.
    return '1024,0,512,32' if case=='r1_popcount' else BUDGET

def inputs(arity):
    return itertools.product(range(256),repeat=arity)

def domain(arity):
    return {'input_bits':8*arity,'target_bits':8,'fault_bits':4,'dirty_bits':0,
            'cases':256**arity*256*16,'input_preservation':True,
            'clean_workspace':'all zero on exit; byte-exact entry after reversal',
            'claim':'exhaustive proof only after the entire tuple is checked on every element'}

def split_native(data):
    assert len(data)==NATIVE_COUNT, f'native count {len(data)} != {NATIVE_COUNT}'
    result={}; start=0
    for case,n in CASES.items():
        result[case]=data[start:start+256**n]; start+=256**n
    return result

def result(v):
    assert int(v['code'])==0, 'nonzero oracle fault: '+str(v)
    value=int(v['payload'])
    assert 0<=value<256, 'payload outside Int8 encoding'
    return value

def restored(v):
    assert v['equals_entry'] is True and v['frames_clean'] is True, 'state/workspace not restored'
    assert v['first_difference'] is None, 'first differing component: '+str(v['first_difference'])
    assert v['status']=='REVERSED', 'reversal not completed'
    # Oracle CLI equals_entry compares the full §3.2 snapshot, including memory.
    if 'memory_equals_entry' in v:
        assert v['memory_equals_entry'] is True, 'memory differs'
        assert v['memory_digest']==v['entry_memory_digest'], 'entry/final memory digest differs'

def check_action(v,x,t,e,native):
    """Compare the entire public quantum tuple, including phase and reversal."""
    def basis(values,widths):
        return ''.join(''.join(str((q>>i)&1) for i in range(w)) for q,w in zip(values,widths))
    widths=[8,4]+[8]*len(x)
    before=basis([t,e,*x],widths);after=basis([t^native,e,*x],widths)
    assert v['forward_state']=={after:[1,0]}, 'entire forward register tuple or phase differs'
    assert v['wires']==sum(widths), 'residual workspace wires'
    assert v['status']=='REVERSED' and v['equals_entry'] is True and v['first_difference'] is None
    assert v['state']=={before:[1,0]} and v['in']==list(x) and v['mut']==[t,e], 'entry tuple differs after reversal'

class Gate(RecordedGate):
    def __init__(self,args):
        self.args=args
        self.tmp=Path(os.environ.get('TMPDIR',ROOT/'.tmp')).resolve();self.tmp.mkdir(parents=True,exist_ok=True)
        self.out=Path(args.output).resolve() if args.output else self.tmp/'rung-1'
        self.out.mkdir(parents=True,exist_ok=True)
        self.run=self.out/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ');self.run.mkdir()
        self.rows={f'{lang}/{m}':{} for lang in ('C','Rust','Julia') for m in (18,23)}
        self.known=json.loads((HERE/'blockers.json').read_text())
        self.manifest={'time_utc':datetime.now(timezone.utc).isoformat(),'commands':[],
            'pins':dict(PINS),'dialect':'0.2.10','isa':'0.8','budget':BUDGET,
            'domain':{c:domain(n) for c,n in CASES.items()},'case_budgets':{c:case_budget(c) for c in CASES},
            'environment':{k:os.environ.get(k) for k in ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','BROWSER','TMPDIR')},
            'worker_sha256':{f:sha(HERE/f) for f in ('input_worker.py','full_domain_worker.py','progress.py','batch_reverse.py')},
            'gate_sha256':sha(HERE/'gate.py'),'shared_support_sha256':sha(HERE.parent/'common/gate_support.py')}
        self.manifest['native_layout_sha256']=sha(HERE/'native_layout.py')
        self.paths={};self.entries={};self.children={};self.inapplicable={}
        self.batch_capabilities={};self.batch_lock=threading.Lock()
        self.sources=RevisionSources(self,WORKSPACE)

    def freeze(self,repo,rev):
        rev=git(WORKSPACE/repo,'rev-parse',rev)
        self.manifest['pins'][repo]=rev
        src=self.tmp/('rung1-'+repo+'-'+rev)
        data=self.command('build/'+repo+'-archive',['git','-C',WORKSPACE/repo,'archive',rev])
        if not src.exists():
            src.mkdir()
            with tarfile.open(fileobj=io.BytesIO(data)) as t:t.extractall(src,filter='data')
        with tarfile.open(fileobj=io.BytesIO(data)) as t:
            for member in t.getmembers():
                if member.isfile():assert (src/member.name).read_bytes()==t.extractfile(member).read(), 'dirty build cache: '+member.name
        return src

    def plugin(self):
        src=self.freeze('bennett-plugin',self.args.plugin_rev)
        self.sources.plugin_versions(src)
        layout=self.sources.plugin_layout(src)
        self.plugin_build=src/'build'
        self.command('build/plugin-configure',['env','BENNETT_GATE_LAYOUT='+str(layout),'cmake','-S',src,'-B',self.plugin_build])
        self.command('build/plugin-build',['env','BENNETT_GATE_LAYOUT='+str(layout),'cmake','--build',self.plugin_build,'-j',self.args.jobs],timeout=1800)
        self.manifest['plugin_sha256']={str(m):sha(self.plugin_build/f'libbennett-plugin-{m}.so') for m in (18,23)}
        self.publications={
            'bennett-runtimes':self.sources.paths['bennett-runtimes'],
            'bennett-arith':self.sources.freeze('bennett-arith',self.sources.arithmetic_revision(src))}

    def vm(self):
        repo=WORKSPACE/'bennett-vm'; rev=git(repo,'rev-parse',self.args.vm_rev)
        self.manifest['pins']['bennett-vm']=rev
        src=self.tmp/('vm-'+rev)
        if not src.exists():self.command('build/vm-worktree',['git','-C',repo,'worktree','add','--detach',src,rev])
        assert (src/'.git').is_file() and git(src,'rev-parse','--abbrev-ref','HEAD')=='HEAD', 'VM not detached'
        assert git(src,'rev-parse','HEAD')==rev and not git(src,'status','--porcelain','--untracked-files=no'), 'VM worktree dirty or wrong HEAD'
        self.command('build/vm-build',['cargo','build','--release','--offline','--jobs',self.args.jobs,'--manifest-path',src/'Cargo.toml','--target-dir',src/'target'],timeout=600)
        self.vm_exe=src/'target/release/bennett-vm'
        self.manifest['vm_profile']='release'
        self.manifest['vm_sha256']=sha(self.vm_exe)

    def build_reference(self):
        self.reference_exe=self.sources.reference()

    def toolchain(self,lang,m):
        tools=[f'/usr/lib/llvm-{m}/bin/{t}' for t in ('clang','opt','llvm-link','llvm-dis')]
        if lang=='Rust':tools+=['rustc']
        if lang=='Julia':tools+=['julia']
        for tool in tools:
            self.command(f'{lang}/{m}/version-'+Path(tool).name,[tool,'--version'])
        if lang=='Rust':self.command(f'{lang}/{m}/rustc-verbose',['rustc','-vV'])
        if lang=='Julia':self.command(f'{lang}/{m}/julia-llvm',['julia','--startup-file=no','-e','println(Base.libllvm_version)'])

    def fixtures(self):
        exp=json.loads((HERE/'fixtures/expect.json').read_text())
        valid=set()
        for spec in ('DLT-0.2.10','ISA-0.8'):
            valid.update(n['id'] for n in json.loads((ROOT/f'clauses/{spec}.json').read_text())['nodes'])
        for c,n in CASES.items():
            assert exp['cases'][c]['arity']==n
            assert exp['cases'][c]['clauses'] and set(exp['cases'][c]['clauses'])<=valid, 'missing clause tags'
        pins=json.loads((HERE/'fixtures/sha256.json').read_text())
        for f,h in pins.items():assert sha(HERE/'fixtures'/f)==h,'fixture digest mismatch: '+f
        for lang,file in [('c','scalars.c'),('rust','scalars.rs'),('julia','scalars.jl')]:
            assert '__bennett_' not in (HERE/'fixtures'/lang/file).read_text(), 'marker-aware corpus'
        self.manifest['fixtures_sha256']=pins

    def folder(self,lang,m):
        d=self.run/lang/str(m); d.mkdir(parents=True,exist_ok=True);return d

    def native(self,lang,m):
        d=self.folder(lang,m); outputs=[]
        for level in (0,2):
            tag=f'{lang}/{m}/native-O{level}';exe=d/f'native-O{level}'
            if lang=='C':
                self.command(tag+'-build',[f'/usr/lib/llvm-{m}/bin/clang',f'-O{level}',HERE/'fixtures/c/scalars.c',HERE/'fixtures/c/native.c','-o',exe])
                argv=[exe]
            elif lang=='Rust':
                lib=d/f'libr1-O{level}.rlib'
                self.command(tag+'-lib',['rustc','--edition=2024','--crate-type=rlib','--crate-name=r1','-C',f'opt-level={level}',HERE/'fixtures/rust/scalars.rs','-o',lib])
                self.command(tag+'-build',['rustc','--edition=2024','--extern',f'r1={lib}','-C',f'opt-level={level}',HERE/'fixtures/rust/native.rs','-o',exe])
                argv=[exe]
            else:argv=['julia','--startup-file=no',f'--optimize={level}',HERE/'fixtures/julia/native.jl']
            outputs.append(self.command(tag+'-run',argv))
        assert outputs[0]==outputs[1], 'native O0/O2 disagree'
        values=split_native(outputs[1])
        for c,data in values.items():(d/(c+'-native.bin')).write_bytes(data)
        self.manifest.setdefault('native',{})[f'{lang}/{m}']={'sha256':sha(d/'native-O2-run.stdout'),'cases':NATIVE_COUNT,
            'excluded_undefined':0,'excluded_layout_dependent':0,
            'definedness':'widened signed arithmetic, guarded divisors, bounded indices; all Int8 bit patterns retained'}

    def frontend(self,lang,m):
        d=self.folder(lang,m); src=HERE/'fixtures'
        if lang=='C':
            self.command(f'{lang}/{m}/frontend',[f'/usr/lib/llvm-{m}/bin/clang','-O2','-flto','-emit-llvm','-c',src/'c/scalars.c','-o',d/'input.bc'])
        elif lang=='Rust':
            self.command(f'{lang}/{m}/frontend',['rustc','--edition=2024','--crate-type=lib','--emit=llvm-bc','-C','opt-level=2',src/'rust/scalars.rs','-o',d/'input.bc'])
        else:
            self.command(f'{lang}/{m}/frontend',['julia','--startup-file=no','--optimize=2',src/'julia/emit.jl',d/'ir'])
            # Serialize the exact text module with Julia's own LLVM, preserving
            # its producer version rather than stamping it with the reader's.
            self.command(f'{lang}/{m}/bitcode',['julia','--startup-file=no',HERE/'julia_bitcode.jl',d/'ir'])
        files=sorted((d/'ir').glob('*.bc')) if lang=='Julia' else [d/'input.bc']
        assert len(files)==(len(CASES) if lang=='Julia' else 1), 'missing frontend bitcode'
        for path in files:
            # bcanalyzer reads the bitstream structure without interpreting IR.
            header=self.command(f'{lang}/{m}/producer-'+path.stem,
                ['/usr/lib/llvm-23/bin/llvm-bcanalyzer','-dump',path])
            row=frontend_applicability(header,m)
            if row:self.inapplicable[f'{lang}/{m}']=row
        # Native frontend IR is never rewritten/downgraded for an older reader.

    def runtime_files(self,m):
        rt=Path(getattr(self.args,f'runtimes{m}',None) or self.publications['bennett-runtimes']/f'dist/llvm{m}/bennett-runtimes.bc').resolve()
        expected=sha(self.publications['bennett-runtimes']/f'dist/llvm{m}/bennett-runtimes.bc')
        assert sha(rt)==expected, 'runtimes publication differs from selected revision'
        pins=tomllib.loads((self.plugin_build.parent/'SPEC_VERSIONS.toml').read_text())['arith']
        ar=Path(getattr(self.args,f'arith{m}',None) or self.publications['bennett-arith']/f'dist/llvm{m}/bennett-arith.bc').resolve()
        assert sha(ar)==pins[f'llvm{m}_sha256'], 'arith differs from plugin SHA pin'
        self.manifest.setdefault('publications',{})[str(m)]={'runtime':{'path':str(rt),'sha256':sha(rt)},'arith':{'path':str(ar),'sha256':sha(ar)}}
        return rt,ar

    def each(self,lang,m,stage,action):
        errors=[];key=f'{lang}/{m}'
        for case in CASES:
            try:action(case)
            except (GateError,ValueError,KeyError,AssertionError,OSError) as e:errors.append(f'{case}: {e}')
        if errors:raise GateError('\n'.join(errors))

    def link(self,lang,m,c):
        d=self.folder(lang,m)/c;d.mkdir(exist_ok=True)
        rt,ar=self.runtime_files(m)
        src=self.folder(lang,m)/('ir/'+c+'.ll' if lang=='Julia' else 'input.bc')
        self.command(f'{lang}/{m}/{c}/link',[f'/usr/lib/llvm-{m}/bin/llvm-link',src,rt,ar,'--only-needed','-o',d/'linked.bc'])

    def lto(self,lang,m,c):
        d=self.folder(lang,m)/c
        self.command(f'{lang}/{m}/{c}/lto',[f'/usr/lib/llvm-{m}/bin/opt','-passes=lto<O3>','-S',d/'linked.bc','-o',d/'postlink.ll'])

    def pipeline(self,lang,m,c):
        d=self.folder(lang,m)/c;entry=c
        if lang=='Julia':
            ir=(d/'postlink.ll').read_text()
            match=re.search(r'define[^\n]*@("?julia_'+c+r'_\d+"?)\(',ir)
            assert match, 'Julia specialised function missing: '+c
            entry=match[1].strip('"')
        self.entries[(lang,m,c)]=entry
        _,arith=self.runtime_files(m)
        self.command(f'{lang}/{m}/{c}/pipeline',['env','BENNETT_ARITH_BITCODE='+str(arith),f'/usr/lib/llvm-{m}/bin/opt',
            '-load-pass-plugin='+str(self.plugin_build/f'libbennett-plugin-{m}.so'),'-passes=bennett',
            '-bennett-dialect='+self.manifest['dialect'],'-bennett-isa='+self.manifest['isa'],'-bennett-oracle='+entry,
            '-bennett-scalar-shape='+('ARG(64),' if lang=='Julia' else '')+','.join(['IN(8,sext)']*CASES[c])+';tgt=8',
            '-bennett-scalar-stack='+case_budget(c).split(',')[2],
            '--bennett-layout-data-base=131072','--bennett-layout-text-base=8388608',
            '-bennett-out='+str(d/'out'),'-disable-output',d/'postlink.ll'])
        for file in ('ref.bennett','materialised.bc'):assert (d/'out'/file).is_file(), 'missing scalar artefact: '+file

    def comparison(self,lang,m,c):
        d=self.folder(lang,m)/c;out=d/'out';rt,ar=self.runtime_files(m)
        entry=self.entries[(lang,m,c)]
        assert lang!='Julia', 'Julia native C ABI adapter required for specialised gcstack entry'
        arity=CASES[c]
        runner=d/'comparison.c'
        runner.write_text("""typedef signed char i8;
extern i8 ENTRY(PARAMS);
static long output(unsigned char *p, long n) {
    long r; __asm__ volatile("syscall" : "=a"(r) : "a"(1L), "D"(1L), "S"(p), "d"(n) : "rcx", "r11", "memory"); return r;
}
__attribute__((noreturn)) void comparison_main(void) {
    unsigned char buf[256];
    for(unsigned a=0;a<256;++a) {
        for(unsigned b=0;b<COUNT;++b) buf[b]=(unsigned char)ENTRY(ARGS);
        if(output(buf,COUNT)!=COUNT) __asm__ volatile("ud2");
    }
    __asm__ volatile("syscall" :: "a"(60L), "D"(0L) : "rcx", "r11", "memory");
    __builtin_unreachable();
}
__attribute__((naked,noreturn)) void _start(void) { __asm__("andq $-16, %rsp; call comparison_main; ud2"); }
""".replace('ENTRY',entry).replace('PARAMS',','.join(['i8']*arity))
            .replace('ARGS','(i8)a,(i8)b' if arity==2 else '(i8)a').replace('COUNT',str(256 if arity==2 else 1)))
        layout=json.loads(self.obj(lang,m,c).read_text().split('section layout\n')[1])
        layout_flags='-Wl,--gc-sections,--build-id=none,-e,_start,--section-start=.text=0x800000'
        if layout['globals']:
            layout_flags+=',--section-start=.rodata=0x20000,--section-start=.data=0x21000'
        linked=d/'comparison-linked.bc'
        bitcode_argv=[f'/usr/lib/llvm-{m}/bin/llvm-link',out/'materialised.bc',rt,ar,
            '--only-needed','-o',linked]
        self.command(f'{lang}/{m}/{c}/native-comparison-bitcode-link',bitcode_argv)
        argv=[f'/usr/lib/llvm-{m}/bin/clang','-O2','-nostdlib','-static','-no-pie',
            '-ffunction-sections','-fdata-sections',linked,runner,
            layout_flags,
            '-o',d/'comparison']
        (d/'native-comparison.json').write_text(json.dumps({'materialised_sha256':sha(out/'materialised.bc'),
            'runtime_sha256':sha(rt),'arith_sha256':sha(ar),
            'bitcode_link_argv':list(map(str,bitcode_argv)),'linked_sha256':sha(linked),
            'link_argv':list(map(str,argv)),
            'notes':'freestanding scalar-only oracle harness; fixed non-PIE text/data, no host libc/libm'},indent=2)+'\n')
        self.command(f'{lang}/{m}/{c}/native-comparison-link',argv)
        symbols=self.command(f'{lang}/{m}/{c}/native-comparison-symbols',[f'/usr/lib/llvm-{m}/bin/llvm-nm',d/'comparison'])
        assert not self.command(f'{lang}/{m}/{c}/native-comparison-undefined',[f'/usr/lib/llvm-{m}/bin/llvm-nm','--undefined-only',d/'comparison']).strip()
        # Lookup's sole read-only table is the scalar object's sole global, G1.
        if c=='r1_lookup':
            self.comparison_layout(lang,m,c,layout,symbols)
        data=self.command(f'{lang}/{m}/{c}/native-comparison-run',[d/'comparison'])
        assert data==(self.folder(lang,m)/(c+'-native.bin')).read_bytes(), 'materialised native differs from O0/O2'

    def comparison_layout(self,lang,m,c,layout,symbols):
        d=self.folder(lang,m)/c
        readelf=lambda flag:self.command(f'{lang}/{m}/{c}/native-comparison-elf{flag}',
            ['readelf','-W',flag,d/'comparison']).decode()
        audit=native_layout.check_globals(d/'comparison',layout,self.obj(lang,m,c).read_text(),symbols.decode(),readelf)
        record=json.loads((d/'native-comparison.json').read_text());record['global_layout_audit']=audit
        write_summary(d/'native-comparison.json',record)

    def executor(self,name):
        exe=self.vm_exe if name=='VM' else self.reference_exe
        assert exe.is_file(), 'missing executor: '+str(exe)
        self.manifest.setdefault('executors',{})[name]={'path':str(exe),'sha256':sha(exe)}
        return exe

    def obj(self,lang,m,c):return self.folder(lang,m)/c/'out/ref.bennett'

    def admission(self,lang,m,c,name):
        for verb in ('check','hash'):
            self.command(f'{lang}/{m}/{c}/{name}-{verb}',[self.executor(name),verb,self.obj(lang,m,c)])

    def full_domain_interface(self,lang,m,c,name):
        # Public ISA action adapter leaves the compiler's oracle/lproc text and
        # hashes untouched. It exposes all input, target and fault wires through
        # run-action/reverse, avoiding a payload-only interface assumption.
        obj=self.obj(lang,m,c);wrapped=obj.with_name('full-domain.bennett')
        if not wrapped.exists():
            text=obj.read_text();sections=text.split('section symbols\n')
            raw,tail=sections[1].split('\n\nsection functions\n',1)
            symbols=json.loads(raw);ref=next(v['interface_hash'] for v in symbols.values() if v['name']=='ref')
            n=CASES[c]
            sig={'abi':'0.8','args':['token'],'bounds':[],'in':[8]*n,'kind':'action','mem':[],'mut':[8,4],'tier':'q'}
            body='action @rung1_full (t:wire[8] mut, e:wire[4] mut) ('+', '.join(f'x{i}:wire[8] in' for i in range(n))+') () (tk:token) ()\n^entry():\noracle_call @ref tk ('+', '.join('x'+str(i) for i in range(n))+') t e () ()\n'
            canonical=body.replace('@ref','@'+ref)
            h=hashlib.sha256(('isa='+self.manifest['isa']+'\nptr_width=64\n'+json.dumps(sig,sort_keys=True,separators=(',',':'))+'\n'+canonical+ref+'\n').encode()).hexdigest()
            symbols[str(max(map(int,symbols))+1)]={'interface_hash':h,'kind':'action','name':'rung1_full','signature':sig}
            functions,sep,rest=tail.partition('\n\nsection ')
            wrapped.write_text(sections[0]+'section symbols\n'+json.dumps(symbols,sort_keys=True,separators=(',',':'))+'\n\nsection functions\n'+functions.rstrip()+'\n'+body.rstrip()+sep+rest)
        identity=json.loads(self.command(f'{lang}/{m}/{c}/{name}-full-D-check',[self.executor(name),'check',wrapped]))
        write_summary(self.folder(lang,m)/c/(name+'-full-D-identity.json'),identity)
        # A boundary tuple demonstrates the adapter; exhaustive acceptance is a
        # separate bounded worker over the entire product, never this probe.
        value=json.loads(self.command(f'{lang}/{m}/{c}/{name}-full-D-interface',[self.executor(name),'reverse',wrapped,'rung1_full',
            '--mut','255,15','--in',','.join(['255']*CASES[c]),'--arg','token:'+case_budget(c)]))
        check_action(value,(255,)*CASES[c],255,15,(self.folder(lang,m)/(c+'-native.bin')).read_bytes()[-1])

    def exhaustive_D(self,lang,m,c,name):
        if self.batch_available(name):
            return self.batch_domain(lang,m,c,name)
        if name=='reference':
            raise GateError('pinned reference lacks the published reverse-action-batch interface')
        self.command(f'{lang}/{m}/{c}/{name}-full-D-exhaustive',[
            sys.executable,HERE/'full_domain_worker.py','--executor',self.executor(name),
            '--object',self.obj(lang,m,c).with_name('full-domain.bennett'),
            '--native',self.folder(lang,m)/(c+'-native.bin'),'--arity',CASES[c],
            '--evidence',self.folder(lang,m)/c/(name+'-full-D.jsonl'),'--budget',case_budget(c)])

    def batch_available(self,name):
        with self.batch_lock:
            if name not in self.batch_capabilities:
                help_text=self.command(f'build/{name}-batch-help',[self.executor(name),'--help']).decode()
                available=any(' reverse-action-batch ' in line for line in help_text.splitlines())
                self.batch_capabilities[name]=available
                self.manifest.setdefault('batch_capabilities',{})[name]=available
            return self.batch_capabilities[name]

    def batch_domain(self,lang,m,c,name):
        total=domain(CASES[c])['cases']
        # 16 workers measured ~714k cases/s. These execution shards leave ample
        # room below 300s; unary shards are smaller for uncached memory helpers.
        shard_size=1 << (16 if CASES[c]==1 else 20)
        shards=batch_reverse.ranges(total,shard_size)
        root=self.folder(lang,m)/c
        evidence=root/(name+'-full-D-shards');evidence.mkdir(exist_ok=True)
        obj=self.obj(lang,m,c).with_name('full-domain.bennett')
        executable=self.executor(name);executable_hash=sha(executable);object_hash=sha(obj)
        bindings=self.batch_bindings(lang,m,c,name)
        def one(bounds):
            lo,hi=bounds;receipt=evidence/f'{lo}-{hi}.json'
            self.command(f'{lang}/{m}/{c}/{name}-full-D-{lo}-{hi}',[
                sys.executable,HERE/'full_domain_worker.py','--executor',executable,'--object',obj,
                '--native',self.folder(lang,m)/(c+'-native.bin'),'--arity',CASES[c],
                '--evidence',receipt,'--budget',case_budget(c),'--range',f'{lo}..{hi}',
                '--identity',root/(name+'-full-D-identity.json'),
                '--executor-sha256',executable_hash,'--object-sha256',object_hash,'--timeout',self.args.timeout])
            return json.loads(receipt.read_text())
        summary={'cases':0,'faults':0,'reversed':0,'domain':domain(CASES[c]),'complete':False,'transport':batch_reverse.FORMAT,
                 'shards':[],'bindings':bindings,'jobs':getattr(self.args,'full_d_jobs',16),'shard_cases':shard_size}
        summary_path=root/(name+'-full-D-summary.json')
        write_summary(summary_path,summary)
        pool=ThreadPoolExecutor(max_workers=getattr(self.args,'full_d_jobs',16))
        try:
            for receipt in pool.map(one,shards):
                summary['shards'].append(receipt)
                summary['cases']+=receipt['cases'];summary['reversed']+=receipt['cases']
                write_summary(summary_path,summary)
        finally:
            pool.shutdown(wait=True,cancel_futures=True)
        batch_reverse.coverage(summary['shards'],total,bindings)
        summary['complete']=True
        write_summary(summary_path,summary)

    def batch_bindings(self,lang,m,c,name):
        return {'executor_sha256':sha(self.executor(name)),
            'object_sha256':sha(self.obj(lang,m,c).with_name('full-domain.bennett')),
            'native_sha256':sha(self.folder(lang,m)/(c+'-native.bin')),'budget':case_budget(c)}

    def domain_summary(self,lang,m,c,name):
        v=json.loads((self.folder(lang,m)/c/(name+'-full-D-summary.json')).read_text())
        assert v['cases']==domain(CASES[c])['cases'] and v['faults']==0 and v['reversed']==v['cases']
        if v.get('transport')==batch_reverse.FORMAT:
            assert v['complete'] is True
            assert v['bindings']==self.batch_bindings(lang,m,c,name)
            batch_reverse.coverage(v['shards'],domain(CASES[c])['cases'],v['bindings'])

    def diagnostic(self,lang,m,c,name):
        # Bounded worker records every exact child command and response, flushing
        # progress per input so a 300s host timeout never masquerades as exhaustion.
        self.command(f'{lang}/{m}/{c}/{name}-input-exhaustive',[
            sys.executable,HERE/'input_worker.py','--executor',self.executor(name),'--object',self.obj(lang,m,c),
            '--native',self.folder(lang,m)/(c+'-native.bin'),'--arity',CASES[c],
            '--evidence',self.folder(lang,m)/c/(name+'-inputs.jsonl'),'--budget',case_budget(c)])

    def diagnostic_summary(self,lang,m,c,name,reverse=False):
        path=self.folder(lang,m)/c/(name+'-inputs-summary.json')
        v=json.loads(path.read_text());assert v['cases']==256**CASES[c]
        assert v['faults']==0 and v['mismatches']==0
        if reverse:assert v['reversed']==v['cases']

    def finish(self):
        table=['| Language / stage | LLVM 18 | LLVM 23 |','|---|---|---|']
        for lang in ('C','Rust','Julia'):
            for stage in STAGES:
                cells=[]
                for m in (18,23):
                    v=self.rows[f'{lang}/{m}'][stage]
                    cells.append(cell(v))
                table.append('| '+lang+' / '+stage+' | '+' | '.join(cells)+' |')
        self.manifest['blockers_sha256']=sha(HERE/'blockers.json')
        self.manifest['child_commands']='per-case *-inputs.jsonl; every subprocess argv/output and progress retained'
        (self.run/'report.json').write_text(json.dumps({'manifest':self.manifest,'stages':self.rows,'cases':self.children},indent=2)+'\n')
        (self.run/'table.md').write_text('\n'.join(table)+'\n')
        (self.out/'latest').write_text(str(self.run)+'\n')
        print('\n'.join(table)+'\nEvidence: '+str(self.run),flush=True)
        return int(any(r['status'] not in ('PASS','N/A') for stages in self.rows.values() for r in stages.values()))

    def main(self):
        for s,action,deps in [
            ('sibling revisions',self.sources.prepare,()),
            ('reference build',self.build_reference,('sibling revisions',)),
            ('fixtures',self.fixtures,()),
            ('plugin build',self.plugin,('sibling revisions',)),
            ('VM build',self.vm,('sibling revisions',))]:
            self.stage('C/18',s,action,deps)
            for key in self.rows:self.rows[key][s]=dict(self.rows['C/18'][s])
        def major(pair):
            lang,m=pair;key=f'{lang}/{m}'
            self.stage(key,'toolchain',lambda:self.toolchain(lang,m))
            self.stage(key,'native O0 = O2',lambda:self.native(lang,m),('toolchain','fixtures'))
            self.stage(key,'frontend',lambda:self.frontend(lang,m),('toolchain','fixtures'))
            if key in self.inapplicable and self.rows[key]['frontend']['status']=='PASS':
                for s in STAGES:
                    if s not in self.rows[key]:self.rows[key][s]=dict(self.inapplicable[key])
                self.children[key]={c:{s:dict(r) for s,r in self.rows[key].items()} for c in CASES}
                return
            # Per-case dependencies preserve successful functions even when another
            # ordinary scalar is refused. Aggregate rows are conjunctions only.
            childkeys=[]
            for c in CASES:
                child=key+'/'+c;childkeys.append(child);self.rows[child]={}
                for s in ('sibling revisions','reference build','fixtures','plugin build','VM build','toolchain','native O0 = O2','frontend'):
                    self.rows[child][s]=dict(self.rows[key][s])
                def stage(s,f,deps):self.stage(child,s,f,deps)
                stage('runtime bitcode link',lambda:self.link(lang,m,c),('frontend','plugin build'))
                stage('post-link LTO',lambda:self.lto(lang,m,c),('runtime bitcode link',))
                stage('pipeline',lambda:self.pipeline(lang,m,c),('post-link LTO',))
                stage('materialised native comparison',lambda:self.comparison(lang,m,c),('pipeline','native O0 = O2'))
                for name in ('reference','VM'):
                    stage(name+' admission',lambda:self.admission(lang,m,c,name),('pipeline',)+(('VM build',) if name=='VM' else ('reference build',)))
                    stage(name+' input exhaustive = native (diagnostic)',lambda:self.diagnostic(lang,m,c,name),('native O0 = O2',name+' admission'))
                    for suffix,rev in [('input zero faults (diagnostic)',False),('input byte-exact reversal (diagnostic)',True)]:
                        stage(name+' '+suffix,lambda rev=rev:self.diagnostic_summary(lang,m,c,name,rev),(name+' input exhaustive = native (diagnostic)',))
                    stage(name+' full D interface',lambda:self.full_domain_interface(lang,m,c,name),(name+' admission',))
            for s in STAGES:
                if s in self.rows[key] or s.endswith(('exhaustive full D = native','zero faults on D','byte-exact reversal on D')):continue
                values=[self.rows[k][s] for k in childkeys]
                failures=[r for r in values if r['status']!='PASS']
                self.rows[key][s]={'status':('FAIL' if any(r['status']=='FAIL' for r in failures) else 'BLOCKED') if failures else 'PASS',
                    'beads':sorted({b for r in failures for b in r['beads']}),
                    'error':'\n'.join(k+': '+self.rows[k][s]['error'] for k in childkeys if self.rows[k][s]['status']!='PASS')}
            for name in ('reference','VM'):
                self.stage(key,name+' exhaustive full D = native',
                    lambda:self.each(lang,m,'full D',lambda c:self.exhaustive_D(lang,m,c,name)),
                    (name+' full D interface','materialised native comparison',name+' input exhaustive = native (diagnostic)'))
                for suffix in ('zero faults on D','byte-exact reversal on D'):
                    self.stage(key,name+' '+suffix,
                        lambda:self.each(lang,m,'D summary',lambda c:self.domain_summary(lang,m,c,name)),
                        (name+' exhaustive full D = native',))
            self.children[key]={c:self.rows.pop(key+'/'+c) for c in CASES}
        with ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(major,[(l,m) for l in ('C','Rust','Julia') for m in (18,23)]))
        return self.finish()

def main():
    if not __debug__:raise GateError('Python -O disables acceptance checks')
    for k in ('DISPLAY','WAYLAND_DISPLAY','DBUS_SESSION_BUS_ADDRESS','BROWSER','PYTHONOPTIMIZE'):os.environ.pop(k,None)
    os.environ.setdefault('TMPDIR',str(ROOT/'.tmp'));Path(os.environ['TMPDIR']).mkdir(parents=True,exist_ok=True)
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output');p.add_argument('--timeout',type=int,default=300);p.add_argument('--jobs',type=int,default=4)
    p.add_argument('--full-d-jobs',type=int,default=16,help='concurrent complete range checks; each retains --timeout')
    p.add_argument('--plugin-rev');p.add_argument('--vm-rev')
    add_revision_options(p)
    p.add_argument('--reference')
    for m in (18,23):
        p.add_argument('--arith'+str(m));p.add_argument('--runtimes'+str(m))
    args=p.parse_args()
    validate_revision_options(p,args)
    apply_recorded_pins(args,PINS)
    if not 0<args.timeout<=300:p.error('corpus timeout must be in 1..300')
    if args.full_d_jobs<=0:p.error('full-D jobs must be positive')
    return Gate(args).main()

if __name__=='__main__':raise SystemExit(main())
