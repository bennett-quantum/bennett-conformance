"""Published launch-adapter protocols; these are not corpus acceptance results."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate


class ProcessArguments(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(directory.cleanup)
        self.g = gate.Gate(argparse.Namespace(output=directory.name, timeout=5))

    def publication(self, major, workload, width, byte_import):
        out = self.g.workdir(major, workload)/'out/main'
        out.mkdir(parents=True, exist_ok=True)
        build = dict(layout=dict(ptr_width=width), entry_initialisation={})
        if byte_import:
            build['entry_initialisation']['arguments'] = 'bennett-argv-bytes-1'
        service, version = ('bennett.argv.bytes', '1') if byte_import else ('bennett.argv', '0.2.10')
        (out/'build.json').write_text(json.dumps(build))
        self.g.obj(major, workload).write_text(
            f'bennett-object isa=0.9 ptr_width={width}\n\nsection devices\n' +
            json.dumps(dict(services={service:dict(version=version)})) + '\n\nsection layout\n{}\n')
        return out, build

    def test_empty_context_matches_published_protocols_without_driver_file(self):
        for major in (18, 23):
            for workload in gate.WORKLOADS:
                for width in (32, 64):
                    for byte_import in (False, True):
                        with self.subTest(major=major, workload=workload, width=width, bytes=byte_import):
                            out, _ = self.publication(major, workload, width, byte_import)
                            path = self.g.argv_replies(major, workload)
                            self.assertFalse((out/'argv-replies.json').exists())
                            replies = json.loads(path.read_text())
                            if byte_import:
                                # Independent literal size/table oracle: 24-byte header + two null pointers.
                                total = {32:32, 64:40}[width]
                                reply = bytes(16) + bytes([total]) + bytes(total - 17)
                                expected = [dict(service='bennett.argv.bytes', payload='00'*size,
                                                 reply=reply.hex()) for size in (24, total)]
                            else:
                                expected = [dict(service='bennett.argv', payload='00'*{32:12,64:20}[width],
                                                 reply='00'*{32:12,64:20}[width])]
                            self.assertEqual(replies, expected)
                            self.assertEqual(path.read_bytes(), self.g.argv_replies(major, workload).read_bytes())
                            record = self.g.manifest['process_arguments'][str(major)][workload]
                            self.assertEqual(record['argv'], [])
                            self.assertEqual(record['envp'], [])
                            self.assertEqual(record['replies_sha256'], gate.sha(path))

    def test_unknown_or_inconsistent_publication_stops_without_replies(self):
        for mutation in ('width', 'protocol', 'version', 'service', 'object-width'):
            with self.subTest(mutation=mutation):
                out, build = self.publication(18, 'hello', 64, True)
                if mutation in ('width', 'protocol'):
                    if mutation == 'width':
                        build['layout']['ptr_width'] = 16
                    else:
                        build['entry_initialisation']['arguments'] = 'bennett-argv-bytes-2'
                    (out/'build.json').write_text(json.dumps(build))
                else:
                    path = self.g.obj(18)
                    text = path.read_text()
                    text = text.replace('"1"', '"2"') if mutation == 'version' else (
                        text.replace('bennett.argv.bytes', 'libc.printf') if mutation == 'service' else
                        text.replace('ptr_width=64', 'ptr_width=32'))
                    path.write_text(text)
                with self.assertRaisesRegex((AssertionError, gate.GateError), 'argv|argument|pointer'):
                    self.g.argv_replies(18)

    def test_actual_exchanges_must_equal_selected_replies(self):
        for byte_import in (False, True):
            self.publication(18, 'hello', 64, byte_import)
            replies = self.g.argv_replies(18)
            exchanges = [dict(row, address=4096, capacity=len(bytes.fromhex(row['payload'])))
                         for row in json.loads(replies.read_text())]
            self.g.check_argv_exchanges(dict(host_requests=exchanges), replies)
            for field in ('service', 'payload', 'reply', 'capacity'):
                bad = copy.deepcopy(exchanges)
                bad[0][field] = 0 if field == 'capacity' else 'unexpected'
                with self.subTest(bytes=byte_import, field=field), self.assertRaises(AssertionError):
                    self.g.check_argv_exchanges(dict(host_requests=bad), replies)
            for bad in ([], exchanges + exchanges, exchanges[:-1]):
                with self.assertRaises(AssertionError):
                    self.g.check_argv_exchanges(dict(host_requests=bad), replies)

    def test_forward_both_reverse_paths_and_replay_check_the_actual_exchanges(self):
        for byte_import in (False, True):
            for major in (18, 23):
                for workload in gate.WORKLOADS:
                    self.publication(major, workload, 64, byte_import)
                    d = self.g.workdir(major, workload)
                    native = bytes.fromhex(self.g.expected(workload)['stdout_hex'])
                    (d/'native.stdout').write_bytes(native)
                    replies = self.g.argv_replies(major, workload)
                    exchanges = [dict(row, address=4096, capacity=len(bytes.fromhex(row['payload'])))
                                 for row in json.loads(replies.read_text())]
                    forward = dict(status='COMPLETED', fault=None, faults=[], host_requests=exchanges,
                        results=[dict(type='i32', value='0')], run_state='CLOSED',
                        console_hex=native.hex(), effect_tape_length=1,
                        tape=[dict(dev='console', op='write', args=[4096,len(native)], bytes=native.hex())])
                    reversed_ = dict(forward, status='REVERSED', reversed=True, equals_entry=True,
                        memory_equals_entry=True, first_difference=None, entry_memory_digest='a',
                        memory_digest='a', console_hex='', effect_tape_length=0)
                    self.g.executor = lambda name: Path('/executor')
                    calls = []
                    def command(tag, argv):
                        self.assertEqual(Path(argv[argv.index('--replies')+1]).read_bytes(), replies.read_bytes())
                        calls.append(argv)
                        result = reversed_ if '--reverse' in argv or argv[1]=='reverse' else forward
                        return json.dumps(result).encode()
                    self.g.command = command
                    for name in ('reference', 'VM'):
                        self.g.execute(major, name, workload=workload)
                        self.g.execute(major, name, True, workload=workload)
                        self.g.replay(major, name, workload)
                        self.assertEqual(len(calls),4)
                        calls.clear()
                        # The status/output alone must not hide a substituted process reply.
                        exchanges[0]['reply'] = 'ff'
                        for operation in (lambda:self.g.execute(major,name,workload=workload),
                                          lambda:self.g.execute(major,name,True,workload=workload),
                                          lambda:self.g.replay(major,name,workload)):
                            with self.assertRaisesRegex(AssertionError, 'argv exchange'):
                                operation()
                        exchanges[0]['reply'] = json.loads(replies.read_text())[0]['reply']
                        calls.clear()
