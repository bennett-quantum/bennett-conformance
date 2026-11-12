"""Consumer audit for dialect §17.7's two isolated native execution domains."""
import json
from pathlib import Path
import re
import struct

from gate_support import sha
from native_audit import require, audit_script, layout_constant, native_support
from hosted_closure import audit_closure, replay_closure

PROFILE = 'bennett-driver-isolated-provider-1'


def checked(row, expected=None):
    path = Path(row['path'])
    require(expected is None or path.resolve() == Path(expected).resolve(), 'substituted input: '+str(path))
    require(sha(path) == row['sha256'], 'input hash mismatch: '+str(path))
    return path


def inventory(rows):
    paths = [str(checked(row).resolve()) for row in rows]
    require(len(paths) == len(set(paths)), 'duplicate dependency identity')
    return set(paths)


def command_present(manifest, argv):
    require(any(c['argv'] == list(map(str, argv)) and c['success'] is True
                for c in manifest['commands']), 'missing successful closure command: '+str(argv))


def audit_sources(gate, major, device, runtime, arith, record, manifest):
    """Bind original device/producer/checker and matching-major library leaves."""
    inputs = record['inputs']
    for row in inputs.values():
        checked(row)
    fixed = {'materialised':device, 'runtimes':runtime, 'base_arith':arith,
             'arith':arith, 'plugin':gate.plugin_build/f'libbennett-plugin-{major}.so',
             'launch_record':device.parent/'main/launch-record.json',
             'native_check':gate.publications['bennett-runtimes']/'scripts/native-check'}
    for name, path in fixed.items():
        checked(inputs[name], path)
    for name in ('materialised', 'runtimes', 'arith'):
        require(record[name+'_sha256'] == inputs[name]['sha256'], name+' receipt hash mismatch')
    native_support(gate, major, checked(inputs['native_support']), record)
    require(record['base_arith_sha256'] == sha(arith), 'base arithmetic hash mismatch')
    require(record['linked_sha256'] == inputs['comparison']['sha256'], 'final closure hash mismatch')
    report = json.loads(checked(inputs['materialisation_report']).read_text())
    for name, path in (('materialised', device), ('record', fixed['launch_record']),
                       ('launch', device.parent/'main/launch.bennett')):
        require(report[name+'_sha256'] == sha(path), name+' producer identity mismatch')
    require(report['entries'] == ['main'] and report['binding_symbol'], 'wrong selected entry or binding')
    audit_gateway(gate, major, inputs, report, manifest)
    require(record['provenance'] == 'selected-entry launch record verified', 'unverified producer receipt')
    first = [f'/usr/lib/llvm-{major}/bin/llvm-link', inputs['prepared']['path'], str(runtime), str(arith),
             '--only-needed', '-o', inputs['linked']['path']]
    require(record['bitcode_link_argv'] == first, 'substituted initial closure')
    command_present(manifest, first)
    replay = audit_closure(gate, major, record, manifest)
    checker = [str(fixed['native_check']), str(major), inputs['check_input']['path'], inputs['checked']['path']]
    command_present(manifest, checker)
    replay_closure(gate,record,replay)
    for row in inputs.values():
        checked(row)
    detection = json.loads(checked(manifest['native_detection']).read_text())
    require(detection['materialised_sha256'] == sha(device) and
            detection['checked_sha256'] == inputs['checked']['sha256'], 'stale detection coverage')
    for field in ('general_memory', 'use_after_free', 'out_of_live_allocation', 'memory_function_ranges',
                  'invalid_free', 'double_free', 'division_by_zero', 'signed_division_overflow',
                  'explicit_fault', 'unreachable'):
        require(detection[field] is True, 'missing fault coverage: '+field)
    require(detection['source_ub_erased_by_optimisation'] is False, 'invalid independent definedness claim')
    return report


def audit_gateway(gate, major, inputs, report, manifest):
    out = Path(inputs['comparison']['path']).parent
    aliases = [s['native_symbol'] for s in report['symbols']
               if s.get('llvm_name') == 'main' and s['kind'] == 'function']
    require(len(aliases) == 1, 'ambiguous native entry')
    replacements = {'/* DRIVER_ENTRY_DECL */':'extern int driver_entry(void) __asm__('+json.dumps(aliases[0])+');',
        '/* DRIVER_MAIN_STUB */':'', 'DRIVER_BINDING_SYMBOL':report['binding_symbol'],
        '/* DRIVER_INPUTS */':'(void)argc;(void)argv;(void)envp;',
        '/* DRIVER_ENTRY_CALL */':'driver_entry()', '/* DRIVER_FINISH */':'extern void exit(int); exit(result);'}
    expected = (gate.driver_source/'src/native_component.c').read_text()
    for key, value in replacements.items():
        expected = expected.replace(key,value)
    require((out/'native-component.c').read_text() == expected, 'gateway differs from pinned main(void) adapter')
    for name in ('native_provider_api.h','native_bootstrap.c'):
        require((out/name.replace('_','-')).read_bytes() == (gate.driver_source/'src'/name).read_bytes(),
                'hosted support differs from pinned driver')
    commands = [c['argv'] for c in manifest['commands'] if c['argv'] and
                c['argv'][0] == gate.clang(major) and str(out/'native-component.c') in c['argv']]
    require(len(commands)==1, 'missing/duplicate gateway compiler command')
    replay = list(commands[0])
    replay[replay.index('-o')+1] = str(out/'audit-gateway.bc')
    gate.command(str(out.relative_to(gate.run))+'/audit-gateway',replay)
    require(sha(out/'audit-gateway.bc') == sha(out/'native-component.bc'), 'gateway bitcode differs from pinned source')


def audit_component(gate, major, record, manifest, executable):
    """Audit the existing component, never relink it as a replacement baseline."""
    component = checked(record['component'], str(executable)+'.component')
    require(component.read_bytes()[:4] == b'\x7fELF', 'component is not ELF')
    require(not component.stat().st_mode & 0o111, 'component must not be a standalone executable')
    inputs, level = record['inputs'], record['variant']
    out = Path(inputs['comparison']['path']).parent
    script = checked(inputs['linker_script'])
    bindings = json.loads(checked(inputs['layout']).read_text())['native_bindings']
    identifiers = {b['native_symbol'] for b in bindings} | {s for b in bindings for s in b['sections']}
    audit_script(str(script), out, ('SIZEOF','DEFINED','FLAGS'), identifiers)
    layout_constant(checked(inputs['layout_ir']), out)
    obj = checked(record['codegen_object'], inputs['codegen_object']['path'])
    argv = [gate.clang(major), '-'+level, '-nostdlib', '-nostdinc', '-static', '-no-pie',
            '-fno-merge-all-constants', str(obj), '--sysroot='+str(gate.sysroots[major]),
            '-Wl,-T,'+str(script), '-Wl,-e,_start', '-Wl,--fatal-warnings', '-Wl,--no-undefined',
            '-Wl,-Map,'+str(out/f'native-{level}.map'), '-o', str(component)]
    require(record['link_argv'] == argv, 'component link imports host libraries or substitutes codegen')
    command_present(manifest, argv)
    command_present(manifest, [f'/usr/lib/llvm-{major}/bin/llc', '-'+level, '-filetype=obj',
        '-data-sections', '-jump-table-density=101', '-optsize-jump-table-density=101',
        inputs['comparison']['path'], '-o', str(obj)])
    tag = str(component.relative_to(gate.run))+'-audit'
    dynamic = gate.command(tag+'/dynamic', ['readelf', '-d', component])
    headers = gate.command(tag+'/headers', ['readelf', '-l', component])
    require(b'NEEDED' not in dynamic and b'INTERP' not in headers, 'component imports hosted dependencies')
    require(not gate.command(tag+'/undefined', [f'/usr/lib/llvm-{major}/bin/llvm-nm', '--undefined-only', component]).strip(),
            'component has unresolved imports')
    audit_ranges(gate, major, component, inputs, tag)


def audit_ranges(gate, major, component, inputs, tag):
    layout = json.loads(checked(inputs['layout']).read_text())
    obj = checked(inputs['launch_record']).parent/'launch.bennett'
    original = json.loads(obj.read_text().split('\nsection layout\n')[1].splitlines()[0])
    for field in ('null_size', 'ptr_width', 'heap_base', 'heap_limit_max'):
        require(layout[field] == original[field], 'comparison relocated '+field)
    require(layout['globals'].keys() == original['globals'].keys(), 'comparison changed global identities')
    symbols = gate.command(tag+'/symbols', [f'/usr/lib/llvm-{major}/bin/llvm-nm', '-S', '--defined-only', component]).decode()
    rows = {m[3]:(int(m[0],16),int(m[1],16),m[2]) for m in
            re.findall(r'^([0-9a-fA-F]+)\s+([0-9a-fA-F]+)\s+(\w)\s+(\S+)$', symbols, re.M)}
    data_end = rows['__bennett_comparison_data_end'][0]
    require((data_end+4095)//4096*4096 == original['heap_base'], 'component heap boundary mismatch')
    audit_segments(component, original)
    for name, value in original['globals'].items():
        actual = layout['globals'][name]
        require(all(actual[k] == value[k] for k in ('base', 'size', 'image')), 'comparison relocated global '+name)
        address, size, kind = rows[actual['native_symbol']]
        require((address,size) == (value['base'],value['size']) and kind.lower() in ('b','d','r','s','g')
                and address % actual['alignment'] == 0, 'component fixed range mismatch: '+name)
    # Also run the frozen public auditor over the complete binding inventory.
    gate.command(tag+'/layout', ['python3', gate.driver_source/'scripts/native-layout.py', '--audit', str(major),
                               component, inputs['layout']['path']])


def audit_segments(component, layout):
    data = component.read_bytes()
    require(data[:6] == b'\x7fELF\x02\x01' and len(data)>=64, 'expected ELF64 little-endian component')
    offset = struct.unpack_from('<Q',data,32)[0]
    size, count = struct.unpack_from('<HH',data,54)
    require(size==56 and offset+size*count<=len(data), 'malformed component program headers')
    loads = [struct.unpack_from('<IIQQQQQQ',data,offset+i*size) for i in range(count)]
    begin, end = layout['heap_base'], layout['heap_base']+layout['heap_limit_max']
    for kind, flags, _, address, _, _, length, _ in loads:
        if kind == 1 and length:
            require(address+length<=begin or address>=end, 'component segment overlaps device heap')
    for value in layout['globals'].values():
        require(any(p[0]==1 and p[3]<=value['base'] and value['base']+value['size']<=p[3]+p[6] for p in loads),
                'device global has no component backing segment')


def audit_hosted(gate, major, device, runtime, arith, record, manifest, executable):
    require(record['profile'] == PROFILE and record['status'] == 'linked', 'wrong isolated profile/status')
    require(record['llvm_major'] == major and record['variant'] in ('O0','O2'), 'wrong LLVM variant')
    require(not record['residual_markers'], 'unresolved marker imports')
    report = audit_sources(gate, major, device, runtime, arith, record, manifest)
    audit_component(gate, major, record, manifest, executable)
    config_path = checked(record['provider_config'], str(executable)+'.provider-config.json')
    config = json.loads(config_path.read_text())
    require(config['schema'] == PROFILE and config['entry'] == 'main' and
            config['binding_symbol'] == report['binding_symbol'], 'wrong hosted binding/entry')
    for name, path in (('materialised',device), ('launch',device.parent/'main/launch.bennett'),
                       ('record',device.parent/'main/launch-record.json')):
        require(config[name+'_sha256'] == sha(path), 'hosted '+name+' identity mismatch')
    checked(config['provider'], gate.native_provider)
    checked(config['inputs']['target'])
    require(config['inputs']['target']['sha256'] == sha(gate.native_target), 'substituted canonical target')
    checked(config['inputs']['component'], str(executable)+'.component')
    checked(config['inputs']['object'])
    require(config['inputs']['object']['sha256'] == config['launch_sha256'], 'substituted ISA object')
    for row in config['inputs'].values():
        checked(row)
    for key, root in (('dependencies',gate.native_provider), ('bootstrap_dependencies',Path(config['inputs']['bootstrap']['path']))):
        expected = inventory(config[key])
        require(str(root.resolve()) in expected, 'missing dependency root')
        actual = gate.command(str(executable.relative_to(gate.run))+'-audit/'+key, ['ldd', root]).decode()
        paths = {str(root.resolve())} | {str(Path(p).resolve()) for p in re.findall(r'(?:=>\s*)?(/\S+)\s+\(', actual)}
        require(expected == paths, 'hosted dependency closure mismatch')
    template = (gate.driver_source/'scripts/native-provider-run.py').read_text()
    require(executable.read_text() == template.replace('DRIVER_CONFIG_SHA256',sha(config_path)), 'launcher/config mismatch')
    require(record['sha256'] == sha(executable), 'launcher hash mismatch')
    return config


def audit_execution(config, receipt, seen, status, originals=()):
    require(receipt['schema'] == 'bennett-driver-native-execution-1' and receipt['profile'] == PROFILE,
            'missing completed execution receipt')
    identity = receipt['execution_id']
    require(isinstance(identity,str) and re.fullmatch('[0-9a-f]{32}',identity) and identity not in seen, 'stale execution receipt')
    seen.add(identity)
    for field, source in (('inputs','inputs'), ('provider','provider'), ('provider_dependencies','dependencies'),
                          ('bootstrap_dependencies','bootstrap_dependencies'), ('materialised_sha256','materialised_sha256'),
                          ('launch_sha256','launch_sha256')):
        require(receipt[field] == config[source], 'execution changed '+field)
    for row in list(originals)+list(receipt['inputs'].values())+[receipt['provider']]:
        checked(row)
    expected = inventory(receipt['provider_dependencies'])
    inventory(receipt['bootstrap_dependencies'])
    life = receipt['lifetime']
    require({str(Path(row['path']).resolve()) for row in life['dependencies']} == expected,
            'loaded namespace dependency mismatch')
    require(life['schema'] == 'bennett-driver-native-lifetime-1' and life['namespace'] > 0 and life['load'] is True
            and life['binding_calls'] == 1 and life['entry_calls'] == 1 and life['unload_status'] == 0,
            'incomplete load/binding/entry/unload lifecycle')
    require(life['signal'] == 0 and life['status'] == status and receipt['exit_status'] == status
            and life['last_provider_status'] == 0, 'native fault or status mismatch')
