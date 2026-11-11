"""Independent binary protocol and full-tuple checks; never corpus acceptance."""
import copy
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import batch_reverse as batch


IDENTITY = {'status':'ok', 'object_hash':'a'*64, 'unresolved_imports':[],
            'symbols':{'rung1_full':{'interface_hash':'b'*64, 'closure_hash':'c'*64}}}
BUDGET = '256,0,32,32'


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()+b'\n'


def header(arity, lo, hi):
    components = [{'group':'mut', 'name':'t', 'cell':None, 'offset':0, 'width':8},
                  {'group':'mut', 'name':'e', 'cell':None, 'offset':8, 'width':4}]
    components += [{'group':'in', 'name':'x'+str(i), 'cell':None, 'offset':12+8*i, 'width':8}
                   for i in range(arity)]
    return {'type':'header', 'format':'bennett.reverse-action-batch/1', 'encoding':'binary',
            'action':'rung1_full', 'adjoint':False, 'object_hash':'a'*64,
            'interface_hash':'b'*64, 'closure_hash':'c'*64,
            'args':['token:'+BUDGET], 'token_aliases':None, 'domain_bits':12+8*arity,
            'record_bytes':8 if arity==1 else 10, 'mode':'range', 'lo':str(lo), 'hi':str(hi),
            'rank_order':[*range(2, 2+arity), 0, 1], 'components':components}


def footer(count):
    return {'type':'footer', 'format':'bennett.reverse-action-batch/1', 'complete':True,
            'cases':str(count), 'failed':'0', 'first_failed':None}


class StreamChecks(unittest.TestCase):
    def fixture(self):
        native = bytes([3, 7])+bytes(254)
        # Ranks 4094..4098: x=0,t=255,e=14/15; x=1,t=0,e=0/1.
        # Physical bit order is target, fault, input, each little-endian.
        records = bytes.fromhex('017ffc0e00ff0e00 017ffc0f00ff0f00 '
                               '017f071000001000 017f071100001100')
        return native, canonical(header(1,4094,4098)), records, b'\0'+canonical(footer(4))

    def verify(self, data, native=None, arity=1, lo=4094, hi=4098):
        return batch.verify_stream(io.BytesIO(data), IDENTITY,
            native if native is not None else self.fixture()[0], arity, BUDGET, lo, hi)

    def test_every_forward_and_reversed_tuple_is_checked_across_input_boundary(self):
        native, head, records, tail = self.fixture()
        result = self.verify(head+records+tail)
        self.assertEqual(result['cases'], 4)
        self.assertEqual(result['stream_sha256'], batch.hashlib.sha256(head+records+tail).hexdigest())
        self.assertEqual(result['header'], header(1,4094,4098))

    def test_binary_input_rank_order_and_padding(self):
        native = bytearray(65536); native[255]=6; native[256]=9
        # Last tuple of (x=0,y=255), then first of (x=1,y=0).
        records = bytes.fromhex('017ff90ff00fff0ff00f 017f0910000000100000')
        data = canonical(header(2,1048575,1048577))+records+b'\0'+canonical(footer(2))
        self.assertEqual(self.verify(data, native, 2,1048575,1048577)['cases'],2)
        changed = bytearray(records); changed[5] |= 0x80
        with self.assertRaises(batch.BatchError):
            self.verify(canonical(header(2,1048575,1048577))+changed+b'\0'+canonical(footer(2)),
                        native,2,1048575,1048577)

    def test_each_local_execution_flag_is_mandatory(self):
        _, head, records, tail = self.fixture()
        for bit in range(8):
            changed = bytearray(records); changed[1] ^= 1 << bit
            with self.subTest(bit=bit), self.assertRaises(batch.BatchError):
                self.verify(head+changed+tail)

    def test_target_fault_input_and_reverse_corruption_reject(self):
        _, head, records, tail = self.fixture()
        for byte in range(2,8):
            changed = bytearray(records); changed[byte] ^= 1
            with self.subTest(byte=byte), self.assertRaises(batch.BatchError):
                self.verify(head+changed+tail)

    def test_header_binds_the_complete_selected_interface_and_invocation(self):
        _, _, records, tail = self.fixture()
        good = header(1,4094,4098)
        mutations = {'object_hash':'d'*64, 'interface_hash':'d'*64, 'closure_hash':None,
            'action':'other', 'adjoint':True, 'encoding':'jsonl', 'mode':'jsonl',
            'args':['token:1,0,0,0'], 'token_aliases':[0], 'lo':'4093', 'hi':'4099',
            'domain_bits':21, 'record_bytes':9, 'rank_order':[0,1,2], 'future':0}
        for field,value in mutations.items():
            bad=copy.deepcopy(good); bad[field]=value
            with self.subTest(field=field), self.assertRaises(batch.BatchError):
                self.verify(canonical(bad)+records+tail)
        for field in ('name','group','cell','offset','width'):
            bad=copy.deepcopy(good); bad['components'][0][field]='wrong'
            with self.subTest(component=field), self.assertRaises(batch.BatchError):
                self.verify(canonical(bad)+records+tail)

    def test_strict_framing_footer_counts_and_canonical_json(self):
        _, head, records, tail = self.fixture()
        bad_footer = []
        for key,value in [('complete',False),('cases','3'),('failed','1'),('first_failed','4094'),('cases',4)]:
            value_footer=footer(4); value_footer[key]=value; bad_footer.append(b'\0'+canonical(value_footer))
        malformed = [head+records, head+records[:-1]+tail, head+records+tail+b'x',
                     head+records+b'x'+tail, head+b'\0'+tail,
                     b' '+head+records+tail, head.replace(b'{',b'{"type":"header",',1)+records+tail,
                     head+bytes([2])+records[1:]+tail]
        malformed += [head+records+t for t in bad_footer]
        for i,data in enumerate(malformed):
            with self.subTest(case=i), self.assertRaises(batch.BatchError):self.verify(data)

    def test_native_length_and_ranges_are_not_accepted_as_partial_coverage(self):
        _, head, records, tail=self.fixture()
        with self.assertRaises(batch.BatchError):self.verify(head+records+tail, bytes(255))
        self.assertEqual(batch.ranges(10,4),[(0,4),(4,8),(8,10)])
        receipts=[{'lo':0,'hi':4,'cases':4,'complete':True},
                  {'lo':4,'hi':8,'cases':4,'complete':True},
                  {'lo':8,'hi':10,'cases':2,'complete':True}]
        self.assertEqual(batch.coverage(receipts,10),10)
        receipts[0]['native_sha256']='different'
        with self.assertRaises(batch.BatchError):batch.coverage(receipts,10,{'native_sha256':'expected'})
        for bad in (receipts[:-1], receipts+[receipts[-1]],
                    [dict(receipts[0],hi=5),*receipts[1:]],
                    [dict(receipts[0],complete=False),*receipts[1:]]):
            with self.assertRaises(batch.BatchError):batch.coverage(bad,10)


class ProcessChecks(unittest.TestCase):
    fixture = StreamChecks.fixture
    def execute(self, output, exit_code=0, hang=False, timeout=5, tamper=None, forbid_alias=False):
        directory=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(directory.cleanup)
        root=Path(directory.name)
        executor=root/'executor'
        body='import sys,time\n'
        if forbid_alias:
            body+="if '--token-aliases' in sys.argv: sys.exit('unsupported --token-aliases')\n"
        body+='time.sleep(30)\n' if hang else f'sys.stdout.buffer.write(bytes.fromhex({output.hex()!r}))\nsys.exit({exit_code})\n'
        executor.write_text('#!'+sys.executable+'\n'+body); executor.chmod(0o755)
        obj=root/'object.bennett'; obj.write_bytes(b'test object')
        receipt=root/'receipt.json'
        self.last_receipt=receipt
        executable_hash,object_hash=batch.sha(executor),batch.sha(obj)
        if tamper=='executor':executor.write_text(executor.read_text()+'# changed\n')
        if tamper=='object':obj.write_bytes(b'changed object')
        return batch.execute_shard(executor,obj,IDENTITY,self.fixture()[0],1,BUDGET,
            4094,4098,receipt,executable_hash,object_hash,timeout)

    def test_pinned_tool_and_admitted_object_bytes_cannot_be_substituted(self):
        _,head,records,tail=self.fixture()
        for kind in ('executor','object'):
            with self.subTest(kind=kind), self.assertRaises(batch.BatchError):
                self.execute(head+records+tail,tamper=kind)
            self.assertFalse(self.last_receipt.exists())

    def test_single_token_wrapper_uses_the_common_default_alias_binding(self):
        _,head,records,tail=self.fixture()
        self.assertTrue(self.execute(head+records+tail,forbid_alias=True)['complete'])

    def test_process_must_succeed_even_with_valid_footer(self):
        _,head,records,tail=self.fixture()
        self.assertTrue(self.execute(head+records+tail)['complete'])
        with self.assertRaises(batch.BatchError):self.execute(head+records+tail,exit_code=7)
        result=json.loads(self.last_receipt.read_text())
        self.assertFalse(result['complete']); self.assertEqual(result['process_exit'],7)

    def test_timeout_and_truncation_never_publish_a_complete_shard(self):
        with self.assertRaises(batch.BatchTimeout):self.execute(b'',hang=True,timeout=0.2)
        result=json.loads(self.last_receipt.read_text())
        self.assertFalse(result['complete']); self.assertEqual(result['exit'],124)
        _,head,records,_=self.fixture()
        with self.assertRaises(batch.BatchError):self.execute(head+records)
        self.assertFalse(json.loads(self.last_receipt.read_text())['complete'])


if __name__ == '__main__':unittest.main()
