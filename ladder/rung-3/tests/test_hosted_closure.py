"""Closed comparison transform provenance, without trusting a merged filename."""
import copy
import os
from pathlib import Path
import sys
import tempfile
import unittest
from types import SimpleNamespace

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'common'))
from gate_support import GateError, sha
from hosted_closure import audit_closure, plugin_step, replay_closure
from native_audit import audit_script


class ClosureAudit(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.directory.cleanup)
        self.out = Path(self.directory.name)
        self.g = SimpleNamespace(sysroots={18:self.out/'sysroot'},tmp=self.out,driver_source=self.out)
        (self.out/'src').mkdir()
        (self.out/'src/native_component_adapt.cpp').write_text('pinned adapter')

    def fixture(self):
        names = dict(materialised='device.bc',runtimes='runtime.bc',arith='arith.bc',native_support='support.bc',
            layout_bitcode='native-layout.bc',plugin='plugin.so',launch_record='launch-record.json',prepared='prepared.bc',
            preparation_report='preparation.json',native_check='native-check',linked='linked.bc',check_input='check-input.bc',
            checked='checked.bc',finalised='finalised.bc',materialisation_report='report.json',host_imports='native-host.bc',
            layout='resolved-layout.json',comparison='comparison-sectioned.bc')
        inputs = {k:dict(path=str(self.out/v)) for k,v in names.items()}
        path = lambda key:inputs[key]['path']
        link = ['/usr/lib/llvm-18/bin/llvm-link',path('prepared'),path('runtimes'),path('arith'),
                '--only-needed','-o',path('linked')]
        gateway = ['/usr/lib/llvm-18/bin/clang','-O0','-ffreestanding','-fno-builtin','-fno-pie',
            '-emit-llvm','-c',str(self.out/'native-component.c'),'-o',str(self.out/'native-component.bc'),
            '--sysroot='+str(self.out/'sysroot'),'-nostdinc','-isystem',str(self.out/'sysroot/include')]
        commands = [plugin_step(self.g,18,inputs,'prepare',path('materialised'),path('prepared'),path('preparation_report')),
            link, ['/usr/lib/llvm-18/bin/llvm-link',path('linked'),'-o',path('check_input')],
            [path('native_check'),'18',path('check_input'),path('checked')],
            plugin_step(self.g,18,inputs,'finalise',path('checked'),path('finalised'),path('materialisation_report')),
            gateway, ['/usr/lib/llvm-18/bin/llvm-link',path('finalised'),str(self.out/'native-component.bc'),
                path('layout_bitcode'),'--override='+path('native_support'),'-o',str(self.out/'comparison.bc')],
            plugin_step(self.g,18,inputs,'host',str(self.out/'comparison.bc'),path('host_imports')),
            [str(self.out/'bennett-driver-tools'/('native-component-adapt-18-'+sha(self.out/'src/native_component_adapt.cpp'))),
                str(self.out/'comparison.bc'),path('layout'),path('comparison')]]
        return dict(inputs=inputs,bitcode_link_argv=link),dict(commands=[dict(argv=a,success=True) for a in commands])

    def test_closed_chain_and_rejected_substitutions(self):
        record, manifest = self.fixture()
        audit_closure(self.g,18,record,manifest)
        cases = [('host-leaf',6,lambda a:a.insert(2,'/usr/lib/libc.a')),
                 ('checker-bypass',4,lambda a:a.__setitem__(-3,record['inputs']['linked']['path'])),
                 ('library-drop',6,lambda a:a.__setitem__(1,record['inputs']['materialised']['path'])),
                 ('opaque-pass',4,lambda a:a.__setitem__(2,'-passes=default<O2>')),
                 ('wrong-major',6,lambda a:a.__setitem__(0,'/usr/lib/llvm-23/bin/llvm-link')),
                 ('support-substitute',6,lambda a:a.__setitem__(4,'--override=/usr/lib/libm.a'))]
        for name,index,change in cases:
            broken = copy.deepcopy(manifest)
            change(broken['commands'][index]['argv'])
            with self.subTest(name=name),self.assertRaises(GateError):
                audit_closure(self.g,18,record,broken)
        for index in (0,3,4,5,7,8):
            broken = copy.deepcopy(manifest)
            broken['commands'].pop(index)
            with self.subTest(missing=index),self.assertRaises(GateError):
                audit_closure(self.g,18,record,broken)

    def test_replay_preserves_originals_and_rejects_different_bitcode(self):
        record, manifest = self.fixture()
        self.g.run = self.out
        for row in record['inputs'].values():
            path = Path(row['path'])
            path.write_bytes(b'closure')
            row['sha256'] = sha(path)
        record['linked_sha256'] = record['inputs']['comparison']['sha256']
        originals = {p:p.read_bytes() for p in self.out.iterdir() if p.is_file()}
        commands = audit_closure(self.g,18,record,manifest)
        def command(tag,args):
            dest = args[args.index('-o')+1] if '-o' in args else args[-1]
            Path(dest).write_bytes(b'closure')
        self.g.command = command
        replay_closure(self.g,record,commands)
        self.assertTrue(all(path.read_bytes()==data for path,data in originals.items()))
        for victim in ('checked.bc','comparison-sectioned.bc'):
            def broken(tag,args):
                command(tag,args)
                dest = args[args.index('-o')+1] if '-o' in args else args[-1]
                if Path(dest).name==victim:
                    Path(dest).write_bytes(b'substituted')
            self.g.command = broken
            with self.subTest(victim=victim),self.assertRaisesRegex(GateError,'replayed'):
                replay_closure(self.g,record,commands)

    def test_hosted_linker_script_identifiers_are_not_file_inputs(self):
        path = self.out/'comparison.ld'
        valid = '''PHDRS { data PT_LOAD FLAGS(6); }
SECTIONS { .mapped 65536 : { *(".mapped") } :data
ASSERT(SIZEOF(.mapped) == 4, "extent")
ASSERT(DEFINED("counter"), "defined")
ASSERT("counter" == 65536, "address") }
'''
        identifiers = {'.mapped','counter'}
        path.write_text(valid)
        audit_script(str(path),self.out,('SIZEOF','DEFINED','FLAGS'),identifiers)
        for injection in ('INPUT("counter")','INCLUDE host.ld','"counter"(.text)',
                          'libc.a(*)','GROUP(libc.a)','SEARCH_DIR(/usr/lib)'):
            path.write_text(valid+injection)
            with self.subTest(injection=injection),self.assertRaises(GateError):
                audit_script(str(path),self.out,('SIZEOF','DEFINED','FLAGS'),identifiers)


if __name__ == '__main__':
    unittest.main()
