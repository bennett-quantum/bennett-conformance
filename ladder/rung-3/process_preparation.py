"""Pinned driver preparation receipts and a non-corpus §19.2 regression."""
import copy
import json
from pathlib import Path

from gate_support import sha


def artifact(record, expected=None):
    path = Path(record['path'])
    assert expected is None or path == expected, 'process preparation path differs'
    assert sha(path) == record['sha256'], 'process preparation hash differs: '+path.name
    return record


def audit_preparation(out, source, runtime, arith, major, contract):
    manifest = json.loads((out/'manifest.json').read_text())
    assert manifest['schema'] == 'bennett-driver-build-1', 'unknown preparation manifest'
    assert manifest['status'] == 'prepared', 'driver did not complete process preparation'
    assert manifest['llvm_major'] == major, 'process preparation major differs'
    assert (manifest['dialect'], manifest['isa']) == contract, 'process preparation contract differs'
    assert manifest['inputs'] == [{'path':str(source), 'sha256':sha(source)}], 'process input differs'
    artifact(manifest['runtimes'], runtime)
    artifact(manifest['arith'], arith)
    prep = manifest['process_preparation']
    assert prep['schema'] == 'bennett-driver-process-preparation-1', 'unknown process preparation'
    assert prep['entry'] == 'main', 'process entry differs'
    artifact(prep['runtimes'], runtime)
    prepared = out/'process-prepared.bc'
    artifact(prep['module'], prepared)
    assert manifest['outputs']['process_prepared'] == prep['module'], 'prepared output differs'
    for field in ('wrapper', 'report', 'tool'):
        artifact(prep[field])
    init = manifest['process_initialisation']
    artifact(init['module'], out/'process-init-fini.bc')
    artifact(init['report'], out/'process-init-fini.json')
    assert init['module']['sha256'] == prep['module']['sha256'], 'init/fini binding lost'
    return {'manifest_sha256':sha(out/'manifest.json'), 'preparation':prep, 'initialisation':init}


def buffered_return_probe(gate, major):
    # Ordinary C, intentionally outside WORKLOADS: this diagnostic cannot pass
    # either acceptance workload. It exercises the actual consumer pipeline.
    probe = copy.copy(gate)
    probe.run = gate.run/'buffered-return'
    d = probe.run/str(major)
    d.mkdir(parents=True)
    probe.manifest = {'commands':[]}
    source = d/'return.c'
    source.write_text('#include <stdio.h>\nint main(void) {\n'
                      'fputs("one\\n", stdout); fputs("two\\n", stdout);\n'
                      'fputs("tail", stdout); return 0; }\n')
    root = gate.sysroots[major]
    try:
        probe.command(f'{major}/hello/frontend', [gate.clang(major),
            '--target=x86_64-unknown-linux-musl', '--sysroot='+str(root),
            '-nostdinc', '-isystem', root/'include', '-O2', '-flto', '-emit-llvm',
            '-c', source, '-o', d/'input.bc'])
        probe.runtime_link(major)
        probe.postlink(major)
        probe.pipeline(major)
        probe.artifacts(major)
        replies = probe.argv_replies(major)
        for executor in ('reference', 'VM'):
            probe.admission(major, executor)
            value = json.loads(probe.command(f'{major}/hello/{executor}-buffered-return',
                [gate.executor(executor), 'run-program', probe.obj(major), 'main',
                 '--budget', gate.manifest['budget'], '--replies', replies]))
            probe.successful(value)
            assert value['status'] == 'COMPLETED' and value['run_state'] == 'CLOSED', 'buffered return did not complete'
            probe.check_argv_exchanges(value, replies)
            assert value['console_hex'] == b'one\ntwo\ntail'.hex(), 'buffered return console differs'
            probe.console_tape(value, b'one\ntwo\ntail')
    finally:
        (d/'report.json').write_text(json.dumps(probe.manifest, indent=2)+'\n')
        gate.manifest.setdefault('buffered_return', {})[str(major)] = {
            'report_sha256':sha(d/'report.json'), 'commands':probe.manifest['commands']}
