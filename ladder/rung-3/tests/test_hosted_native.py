"""Isolated-provider audit regressions; these controls do not accept the corpus."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys
import struct
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate
import hosted_native as audit


class HostedAudit(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.g = gate.Gate(argparse.Namespace(output=str(self.root/'runs'), timeout=5, jobs=2))
        self.g.native_provider = self.root/'provider.so'
        self.g.native_target = self.root/'target.json'

    def artifact(self, name, data=b'input'):
        path = self.root/name
        path.write_bytes(data)
        return dict(path=str(path), sha256=gate.sha(path))

    def execution(self):
        provider = self.artifact('provider.so')
        inputs = {name:self.artifact(name) for name in ('component','bootstrap','object','target')}
        config = dict(schema=audit.PROFILE, inputs=inputs, provider=provider,
                      dependencies=[provider], bootstrap_dependencies=[inputs['bootstrap']],
                      materialised_sha256='device', launch_sha256='launch')
        life = dict(schema='bennett-driver-native-lifetime-1', namespace=1, load=True,
                    binding_calls=1, entry_calls=1, unload_status=0, status=0, signal=0,
                    last_provider_status=0, dependencies=[dict(path=provider['path'],base=1234)])
        receipt = dict(schema='bennett-driver-native-execution-1', profile=audit.PROFILE,
            execution_id='a'*32, lifetime=life, exit_status=0, inputs=inputs, provider=provider,
            provider_dependencies=config['dependencies'], bootstrap_dependencies=config['bootstrap_dependencies'],
            materialised_sha256='device', launch_sha256='launch')
        return config, receipt

    def test_completed_execution_and_all_lifecycle_obligations(self):
        config, receipt = self.execution()
        audit.audit_execution(config, receipt, set(), 0)
        for field, value in [('namespace',0), ('load',False), ('binding_calls',0),
                             ('entry_calls',0), ('unload_status',1), ('signal',4),
                             ('status',7), ('last_provider_status',7), ('dependencies',[])]:
            broken = copy.deepcopy(receipt)
            broken['lifetime'][field] = value
            with self.subTest(field=field), self.assertRaises(gate.GateError):
                audit.audit_execution(config, broken, set(), 0)
        for field in ('inputs','provider','provider_dependencies','bootstrap_dependencies',
                      'materialised_sha256','launch_sha256','exit_status','schema','profile'):
            broken = copy.deepcopy(receipt)
            broken[field] = 'wrong'
            with self.subTest(field=field), self.assertRaises(gate.GateError):
                audit.audit_execution(config, broken, set(), 0)

    def test_stale_receipt_and_changed_execution_inputs_are_rejected(self):
        config, receipt = self.execution()
        seen = set()
        audit.audit_execution(config, receipt, seen, 0)
        with self.assertRaisesRegex(gate.GateError, 'stale execution'):
            audit.audit_execution(config, receipt, seen, 0)
        for row in list(config['inputs'].values())+[config['provider']]:
            path = Path(row['path'])
            original = path.read_bytes()
            path.write_bytes(b'tampered')
            with self.subTest(path=path), self.assertRaisesRegex(gate.GateError, 'hash mismatch'):
                audit.audit_execution(config, receipt, set(), 0)
            path.write_bytes(original)

    def test_component_elf_is_audited_without_relinking_or_launcher_checks(self):
        d = self.g.workdir(18)
        executable = d/'native'
        component = Path(str(executable)+'.component')
        component.write_bytes(b'\x7fELF component')
        component.chmod(0o644)
        out = d/'out/native-work'; out.mkdir(parents=True)
        def artifact(name,data):
            path = out/name
            path.write_bytes(data)
            return dict(path=str(path),sha256=gate.sha(path))
        inputs = dict(comparison=artifact('comparison.bc',b'closure'),
            codegen_object=artifact('native-O2.o',b'codegen'),
            linker_script=artifact('comparison.ld',b'SECTIONS { .text : { *(.text) } }'),
            layout=artifact('layout.json',b'{"native_bindings":[]}'),
            layout_ir=artifact('native-layout.ll',b'target triple = "x86_64-unknown-linux-musl"\n'
                b'@__bennett_rt_native_layout = constant { i64, i64, i64 } { i64 1, i64 69632, i64 1024 }, align 8\n'))
        argv = [self.g.clang(18),'-O2','-nostdlib','-nostdinc','-static','-no-pie',
            '-fno-merge-all-constants',inputs['codegen_object']['path'],'--sysroot='+str(self.g.sysroots[18]),
            '-Wl,-T,'+inputs['linker_script']['path'],'-Wl,-e,_start','-Wl,--fatal-warnings',
            '-Wl,--no-undefined','-Wl,-Map,'+str(out/'native-O2.map'),'-o',str(component)]
        codegen = ['/usr/lib/llvm-18/bin/llc','-O2','-filetype=obj','-data-sections','-jump-table-density=101',
            '-optsize-jump-table-density=101',inputs['comparison']['path'],'-o',inputs['codegen_object']['path']]
        record = dict(component=dict(path=str(component),sha256=gate.sha(component)),inputs=inputs,
                      variant='O2',codegen_object=inputs['codegen_object'],link_argv=argv)
        manifest = dict(commands=[dict(argv=a,success=True) for a in (argv,codegen)])
        calls = []
        self.g.command = lambda tag,args: calls.append(list(map(str,args))) or b''
        with patch.object(audit,'audit_ranges'):
            audit.audit_component(self.g,18,record,manifest,executable)
            self.assertEqual(len(calls),3)
            self.assertTrue(all(str(component) in args for args in calls))
            self.g.command = gate.RecordedGate.command.__get__(self.g)
            with patch('subprocess.run',return_value=argparse.Namespace(returncode=0,stdout=b'',stderr=b'')):
                audit.audit_component(self.g,18,record,manifest,executable)
            for response in (b'NEEDED libc.so',b'INTERP',b' U malloc'):
                self.g.command = lambda *a: response
                with self.subTest(response=response),self.assertRaises(gate.GateError):
                    audit.audit_component(self.g,18,record,manifest,executable)
            self.g.command = lambda *a: self.fail('relinked substituted component')
            broken = copy.deepcopy(record)
            broken['link_argv'].insert(2,'-lc')
            with self.assertRaisesRegex(gate.GateError,'host libraries'):
                audit.audit_component(self.g,18,broken,manifest,executable)

    def test_global_identity_and_actual_elf_ranges_are_preserved(self):
        original = dict(null_size=4096,ptr_width=64,heap_base=69632,heap_limit_max=1024,
                        globals={'G1':dict(base=65536,size=4,image='G1')})
        layout = copy.deepcopy(original)
        layout['globals']['G1'].update(native_symbol='counter',alignment=4)
        entry = self.root/'entry'; entry.mkdir()
        (entry/'launch.bennett').write_text('header\nsection layout\n'+json.dumps(original)+'\n')
        path = entry/'launch-record.json'; path.write_bytes(b'record')
        record = dict(path=str(path),sha256=gate.sha(path))
        self.g.driver_source = self.root/'driver-source'
        def check(value,symbols):
            row = self.artifact('layout.json',json.dumps(value).encode())
            self.g.command = lambda tag,args: symbols if tag.endswith('/symbols') else b''
            audit.audit_ranges(self.g,18,self.root/'native.component',dict(layout=row,launch_record=record),'test')
        symbols = (b'0000000000010000 0000000000000004 D counter\n'
                   b'0000000000010004 0000000000000000 D __bennett_comparison_data_end\n')
        component = self.root/'native.component'
        header = bytearray(120); header[:6] = b'\x7fELF\x02\x01'
        struct.pack_into('<Q',header,32,64); struct.pack_into('<HH',header,54,56,1)
        struct.pack_into('<IIQQQQQQ',header,64,1,6,0,65536,65536,4,4,4096)
        component.write_bytes(header)
        check(layout,symbols)
        for field in ('base','size','image'):
            broken = copy.deepcopy(layout)
            broken['globals']['G1'][field] = 1
            with self.subTest(field=field),self.assertRaises(gate.GateError):
                check(broken,symbols)
        for broken in (symbols.replace(b'10000',b'10004'),symbols.replace(b'0004 D',b'0008 D'),
                       symbols.replace(b'D counter',b'T counter')):
            with self.assertRaises(gate.GateError):
                check(layout,broken)

    def test_component_segments_leave_the_device_heap_vacant(self):
        layout = dict(heap_base=69632,heap_limit_max=4096,globals={'G1':dict(base=65536,size=4)})
        path = self.root/'component'
        for address,length,valid in ((65536,4,True),(65536,5000,False),(69632,4,False),(73728,4,False)):
            data = bytearray(120); data[:6] = b'\x7fELF\x02\x01'
            struct.pack_into('<Q',data,32,64); struct.pack_into('<HH',data,54,56,1)
            struct.pack_into('<IIQQQQQQ',data,64,1,6,0,address,address,length,length,4096)
            path.write_bytes(data)
            if valid:
                audit.audit_segments(path,layout)
            else:
                with self.subTest(address=address,length=length),self.assertRaises(gate.GateError):
                    audit.audit_segments(path,layout)

    def test_hosted_dependencies_use_separate_evidence_paths(self):
        d = self.g.workdir(18); out = d/'out'; (out/'main').mkdir(parents=True)
        device = out/'device.bc'; device.write_bytes(b'device')
        launch = out/'main/launch.bennett'; launch.write_bytes(b'launch')
        record_path = out/'main/launch-record.json'; record_path.write_bytes(b'record')
        self.g.driver_source = self.root/'driver'; (self.g.driver_source/'scripts').mkdir(parents=True)
        template = 'config DRIVER_CONFIG_SHA256'
        (self.g.driver_source/'scripts/native-provider-run.py').write_text(template)
        config, _ = self.execution()
        component = Path(str(d/'native')+'.component'); component.write_bytes(b'component')
        config['inputs']['component'] = dict(path=str(component),sha256=gate.sha(component))
        config['inputs']['object']['sha256'] = gate.sha(launch)
        Path(config['inputs']['object']['path']).write_bytes(b'launch')
        config.update(entry='main',binding_symbol='bind_main',record_sha256=gate.sha(record_path),
                      materialised_sha256=gate.sha(device),launch_sha256=gate.sha(launch))
        self.g.native_target = Path(config['inputs']['target']['path'])
        executable = d/'native'; config_path = Path(str(executable)+'.provider-config.json')
        config_path.write_text(json.dumps(config))
        executable.write_text(template.replace('DRIVER_CONFIG_SHA256',gate.sha(config_path)))
        record = dict(profile=audit.PROFILE,status='linked',llvm_major=18,variant='O2',residual_markers=[],
            provider_config=dict(path=str(config_path),sha256=gate.sha(config_path)),sha256=gate.sha(executable))
        self.g.command = gate.RecordedGate.command.__get__(self.g)
        with patch.object(audit,'audit_sources',return_value=dict(binding_symbol='bind_main')), \
             patch.object(audit,'audit_component'), \
             patch('subprocess.run',return_value=argparse.Namespace(returncode=0,stdout=b'',stderr=b'')) as run:
            result = audit.audit_hosted(self.g,18,device,self.root/'rt',self.root/'arith',record,{},executable)
            self.assertEqual(result,config)
            self.assertEqual(run.call_count,2)
            self.assertTrue(all(c.args[0][0]=='ldd' for c in run.call_args_list))

    def test_gateway_is_bound_to_pinned_source_and_replayed_bitcode(self):
        out = self.g.workdir(18)/'out/native-work'; out.mkdir(parents=True)
        self.g.driver_source = self.root/'driver'
        source = self.g.driver_source/'src'; source.mkdir(parents=True)
        template = '\n'.join(('/* DRIVER_ENTRY_DECL */','/* DRIVER_MAIN_STUB */','DRIVER_BINDING_SYMBOL',
            '/* DRIVER_INPUTS */','/* DRIVER_ENTRY_CALL */','/* DRIVER_FINISH */'))
        (source/'native_component.c').write_text(template)
        for name in ('native_provider_api.h','native_bootstrap.c'):
            (source/name).write_text('pinned support')
            (out/name.replace('_','-')).write_text('pinned support')
        generated = ('extern int driver_entry(void) __asm__("native_main");\n\nbind_main\n'
                     '(void)argc;(void)argv;(void)envp;\ndriver_entry()\nextern void exit(int); exit(result);')
        (out/'native-component.c').write_text(generated)
        (out/'native-component.bc').write_bytes(b'gateway')
        inputs = dict(comparison=dict(path=str(out/'comparison.bc')))
        report = dict(symbols=[dict(kind='function',llvm_name='main',native_symbol='native_main')],binding_symbol='bind_main')
        manifest = dict(commands=[dict(argv=[self.g.clang(18),str(out/'native-component.c'),'-o',str(out/'native-component.bc')])])
        def command(tag,argv):
            Path(argv[-1]).write_bytes(b'gateway')
        self.g.command = command
        audit.audit_gateway(self.g,18,inputs,report,manifest)
        (out/'native-component.c').write_text(generated+' fake binding')
        with self.assertRaisesRegex(gate.GateError,'gateway differs'):
            audit.audit_gateway(self.g,18,inputs,report,manifest)
        (out/'native-component.c').write_text(generated)
        (out/'native-component.bc').write_bytes(b'wrong gateway')
        with self.assertRaisesRegex(gate.GateError,'gateway bitcode'):
            audit.audit_gateway(self.g,18,inputs,report,manifest)

    def test_original_hashes_are_rechecked_after_execution(self):
        config, receipt = self.execution()
        original = self.artifact('device.bc')
        Path(original['path']).write_bytes(b'changed')
        with self.assertRaisesRegex(gate.GateError,'hash mismatch'):
            audit.audit_execution(config,receipt,set(),0,[original])

    def test_shared_layout_is_explicit_on_every_production_path(self):
        self.g.plugin_build = self.root/'plugin/build'
        self.g.driver_exe = self.root/'driver'
        self.g.runtime_files = lambda m: (self.root/'runtime.bc',self.root/'arith.bc')
        for major in (18,23):
            for method in ('pipeline','driver_probe'):
                calls = []
                def command(tag, argv, **kw):
                    calls.append(list(map(str,argv)))
                    raise gate.GateError('stop')
                self.g.command = command
                with self.assertRaisesRegex(gate.GateError,'stop'):
                    getattr(self.g,method)(major)
                self.assertIn('--bennett-layout-data-base=65536',calls[0])
                self.assertIn('--bennett-layout-text-base=4194304',calls[0])

    def test_provider_is_built_from_the_pin_and_target_is_unmodified_jcs(self):
        self.g.args.driver_rev, self.g.args.vm_rev = 'driver-pin','executor-pin'
        self.g.manifest['pins']['bennett-vm']='executor-pin'
        frozen = []
        def freeze(repo, rev):
            frozen.append((repo,rev))
            source = self.root/(repo+'-'+rev)
            source.mkdir(exist_ok=True)
            return source
        self.g.freeze = freeze
        calls = []
        def command(tag, argv, **kw):
            calls.append(list(map(str,argv)))
            dest = Path(argv[-1])/'debug'
            dest.mkdir(parents=True,exist_ok=True)
            (dest/('libbennett_vm.so' if '--lib' in argv else 'bennett-driver')).write_bytes(b'binary')
        self.g.command = command
        targets = freeze('bennett-targets',gate.PINS['bennett-targets'])
        target = targets/gate.PINS['native_target_path']
        target.parent.mkdir()
        target.write_bytes(b'{"a":1}')
        self.g.driver()
        self.assertIn(('bennett-vm',gate.PINS['bennett-native-provider']),frozen)
        self.assertEqual(self.g.manifest['pins']['bennett-vm'],'executor-pin')
        self.assertIn('--lib',calls[1])
        self.assertEqual(calls[1][calls[1].index('-j')+1],'2')
        self.assertEqual(self.g.manifest['native_target']['sha256'],gate.sha(target))
        target.write_bytes(b'{ "a":1 }')
        with self.assertRaisesRegex(AssertionError,'canonical JCS'):
            self.g.driver()


if __name__ == '__main__':
    unittest.main()
