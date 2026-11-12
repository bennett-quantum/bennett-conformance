"""Closed transform vocabulary for the published isolated component adapter."""
from pathlib import Path
from native_audit import require, layout_constant
from gate_support import sha


def plugin_step(gate, major, inputs, phase, source, output, report=None):
    names = {'prepare':'bennett-native-comparison', 'finalise':'bennett-native-comparison-finalise',
             'host':'bennett-native-host-imports'}
    args = [f'/usr/lib/llvm-{major}/bin/opt', '-load-pass-plugin='+inputs['plugin']['path'],
            '-passes='+names[phase], '-bennett-native-record='+inputs['launch_record']['path'],
            '-bennett-native-entry=main']
    if phase == 'prepare':
        args.append('-bennett-native-prepare-only')
    if report:
        args.append('-bennett-native-report='+report)
    return [*args, source, '-o', output]


def audit_closure(gate, major, record, manifest):
    inputs = record['inputs']
    out = Path(inputs['comparison']['path']).parent
    leaf_roles = ('materialised','runtimes','arith','native_support','layout_bitcode')
    roots = {inputs[k]['path']:{k} for k in leaf_roles}
    prepare = plugin_step(gate, major, inputs, 'prepare', inputs['materialised']['path'],
                          inputs['prepared']['path'], inputs['preparation_report']['path'])
    commands = [c['argv'] for c in manifest['commands'] if c['success'] is True]
    require(prepare in commands, 'missing pinned prepare transform')
    roots[inputs['prepared']['path']] = {'materialised'}
    gateway = [f'/usr/lib/llvm-{major}/bin/clang','-O0','-ffreestanding','-fno-builtin','-fno-pie',
               '-emit-llvm','-c',str(out/'native-component.c'),'-o',str(out/'native-component.bc'),
               '--sysroot='+str(gate.sysroots[major]),
               '-nostdinc','-isystem',str(gate.sysroots[major]/'include')]
    require(gateway in commands, 'missing pinned gateway compilation')
    roots[str(out/'native-component.bc')] = {'gateway'}
    replay = [prepare, gateway]
    start = commands.index(record['bitcode_link_argv'])
    for args in commands[start:]:
        if Path(args[0]).name == 'native-check' or Path(args[0]).name.startswith('native-component-adapt-'):
            destination = args[-1]
            values = adapter_inputs(gate, major, inputs, args, out)
        elif len(args)>=3 and args[-2] == '-o' and args[-1].endswith('.bc'):
            destination = args[-1]
            values = transform_inputs(gate, major, inputs, args, out)
        else:
            continue
        require(Path(destination).resolve().is_relative_to(out.resolve()), 'closure writes outside comparison')
        replay.append(args)
        require(all(value in roots for value in values), 'unrecorded closure leaf: '+str(values))
        roots[destination] = set().union(*(roots[value] for value in values))
        if destination == inputs['checked']['path']:
            roots[destination].add('checked')
        if destination == inputs['layout_bitcode']['path']:
            roots[destination].add('layout_bitcode')
        if destination == inputs['host_imports']['path']:
            roots[destination].add('host_closed')
            # Published driver copies the host-import output back over its input.
            roots[args[-3]] = roots[destination]
    final = roots.get(inputs['comparison']['path'],set())
    require(set(leaf_roles) | {'checked','gateway','host_closed'} <= final, 'final component bypassed checked/library/support closure')
    return replay


def replay_closure(gate, record, commands):
    """Rebuild bitcode only, without modifying the published or linked baseline."""
    inputs = record['inputs']
    out = Path(inputs['comparison']['path']).parent
    scratch = out/'audit-closure'
    scratch.mkdir(exist_ok=True)
    paths = {}
    for index, args in enumerate(commands):
        destination = args[args.index('-o')+1] if '-o' in args else args[-1]
        output = str(scratch/Path(destination).name)
        replay = [paths.get(value,value) for value in args]
        for i, value in enumerate(replay):
            if value.startswith('-bennett-native-report='):
                replay[i] = '-bennett-native-report='+str(scratch/Path(value.split('=',1)[1]).name)
        position = replay.index('-o')+1 if '-o' in replay else len(replay)-1
        replay[position] = output
        gate.command(str(out.relative_to(gate.run))+f'/audit-closure-{index}',replay)
        paths[destination] = output
        if destination == inputs['checked']['path']:
            require(sha(Path(output)) == inputs['checked']['sha256'], 'replayed checker output differs')
        if destination == inputs['host_imports']['path']:
            paths[str(out/'comparison.bc')] = output
    require(sha(Path(paths[inputs['comparison']['path']])) == record['linked_sha256'],
            'replayed final bitcode closure differs')


def adapter_inputs(gate, major, inputs, args, out):
    tool = Path(args[0]).name
    if tool == 'native-check':
        require(args == [inputs['native_check']['path'],str(major),inputs['check_input']['path'],
                         inputs['checked']['path']], 'unapproved checker transform')
        return [args[2]]
    if tool.startswith('native-component-adapt-'):
        source = gate.driver_source/'src/native_component_adapt.cpp'
        adapter = gate.tmp/'bennett-driver-tools'/f'native-component-adapt-{major}-{sha(source)}'
        require(args == [str(adapter),str(out/'comparison.bc'),inputs['layout']['path'],
                         inputs['comparison']['path']], 'unapproved section adaptation')
        return [args[1]]


def transform_inputs(gate, major, inputs, args, out):
    tool = Path(args[0]).name
    require(args[0] == f'/usr/lib/llvm-{major}/bin/{tool}', 'mixed-major or unknown closure transform')
    if tool == 'opt' and '-passes=strip-dead-prototypes,verify' not in args:
        for phase, source, dest, report in (
            ('finalise','checked','finalised','materialisation_report'),
            ('host',None,'host_imports',None)):
            source_path = inputs[source]['path'] if source else str(out/'comparison.bc')
            expected = plugin_step(gate,major,inputs,phase,source_path,inputs[dest]['path'],
                                   inputs[report]['path'] if report else None)
            if args == expected:
                return [source_path]
        require(False, 'unapproved plugin transformation')
    operands, options = [], []
    for value in args[1:-2]:
        if value.startswith('--override='):
            require(value == '--override='+inputs['native_support']['path'], 'unapproved support override')
            operands.append(value.split('=',1)[1])
        elif value.startswith('-'):
            options.append(value)
        else:
            operands.append(value)
    allowed = {'llvm-link':{'--only-needed'},'llvm-extract':{'--func=memcpy','--func=memmove','--func=memset'},
               'opt':{'-passes=strip-dead-prototypes,verify'},'llvm-as':set()}
    require(tool in allowed and set(options)<=allowed[tool], 'opaque comparison transform')
    if tool == 'llvm-as':
        require(len(operands)==1, 'opaque layout assembly')
        layout_constant(Path(operands[0]),out)
        return []
    require(bool(operands), 'closure transform has no input')
    return operands
