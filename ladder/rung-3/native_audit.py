"""Audit and reproduce the native closure; receipts are consumer adapter data.

Only LLVM bitcode transformations with known inputs are replayed. Generated layout
data is restricted to its published constant, and linker scripts cannot load files.
The gate separately audits the final ELF for imports and an interpreter.
"""
from pathlib import Path
import re

from gate_support import GateError, sha


def require(condition, message):
    if not condition:
        raise GateError('native comparison audit: '+message)


def generated_path(value, out):
    path = Path(value)
    require(',' not in value, 'opaque comma-separated linker path: '+value)
    require(path.is_absolute() and path.resolve().is_relative_to(out.resolve()),
            'unapproved comparison input/output: '+value)
    return path


def audit_script(path, out):
    path = generated_path(path, out)
    text = path.read_text()
    code = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    # A quoted filename is also an input selector in ld. Only ASSERT's final
    # diagnostic operand may be quoted; never erase arbitrary quoted tokens.
    for quoted in re.finditer(r'"[^"\n]*"', code):
        require(re.search(r'\bASSERT\((?:[^()"]|\([^()"]*\))*,\s*$', code[:quoted.start()]),
                'linker script contains a quoted input')
    code = re.sub(r'"[^"\n]*"', '', code)
    require(not re.search(r'\b(INPUT|GROUP|AS_NEEDED|INCLUDE|STARTUP|SEARCH_DIR|OUTPUT)\b', code, re.I),
            'linker script may not import files: '+str(path))
    require('/' not in code.replace('/DISCARD/', ''), 'linker script contains a file path')
    calls = re.findall(r'([\w.]+)\s*\(', code)
    require(all(call in ('SECTIONS', 'ASSERT', 'ALIGN', 'KEEP', 'PROVIDE') for call in calls),
            'linker script has an opaque file selector or expression')
    require('SECTIONS' in code, 'linker script lacks SECTIONS')


def audit_link_argv(gate, major, argv, out, output):
    require(argv and argv[0] == gate.clang(major), 'wrong LLVM clang major')
    require(all(flag in argv for flag in ('-O2', '-nostdlib', '-nostdinc', '-static')),
            'missing O2/static/no-host-library flags')
    require(argv.count('-o') == 1 and argv[-2:] == ['-o', str(output)], 'wrong native output')
    inputs = []
    flags = {'-O2', '-nostdlib', '-nostdinc', '-static', '-no-pie', '-fno-merge-all-constants',
             '-Wl,-e,_start', '-Wl,--no-undefined', '-Wl,--fatal-warnings'}
    for arg in argv[1:-2]:
        if arg in flags:
            continue
        if arg == '--sysroot='+str(gate.sysroots[major]):
            continue
        if arg.startswith('-Wl,-T,'):
            audit_script(arg.removeprefix('-Wl,-T,'), out)
        elif arg.startswith('-Wl,-Map,'):
            generated_path(arg.removeprefix('-Wl,-Map,'), out)
        elif re.fullmatch(r'-Wl,--section-start=\.[\w.]+=0x[0-9a-fA-F]+', arg):
            continue
        else:
            require(not arg.startswith(('-', '@')), 'host libc/libm or opaque linker option: '+arg)
            require(Path(arg).is_absolute() and Path(arg).suffix == '.bc',
                    'unapproved comparison input: '+arg)
            inputs.append(Path(arg))
    require(inputs, 'no comparison bitcode input')
    return inputs


def layout_constant(path, out):
    path = generated_path(str(path), out)
    require(path.name == 'native-layout.ll', 'unknown generated layout input')
    grammar = (r'\s*target triple = "x86_64-unknown-linux-musl"\s*'
               r'@__bennett_rt_native_layout = constant \{ i64, i64, i64 \} '
               r'\{ i64 1, i64 [0-9]+, i64 [0-9]+ \}, align 8\s*')
    require(re.fullmatch(grammar, path.read_text()), 'layout input must contain only the published constant')


def native_support(gate, major, path, record):
    publication = gate.publications['bennett-runtimes'].resolve()
    path = path.resolve()
    pinned = publication/f'dist/llvm{major}/musl-sysroot/lib/native-support.bc'
    require(path in (pinned.resolve(), (gate.sysroots[major]/'lib/native-support.bc').resolve()),
            'native support is not the same-major pinned publication/sysroot')
    require(sha(path) == sha(pinned), 'native support differs from the pinned runtime publication')
    require(record['native_support_sha256'] == sha(path), 'native support hash mismatch')


def bitcode_step(gate, major, argv, provenance, record, out):
    """Parse a closed transform vocabulary; reject all unrecorded input leaves."""
    tool = Path(argv[0]).name
    require(argv[0] == f'/usr/lib/llvm-{major}/bin/{tool}', 'mixed LLVM tool major')
    require(argv.count('-o') == 1 and argv[-2] == '-o', 'opaque bitcode output')
    dest = generated_path(argv[-1], out)
    operands = argv[1:-2]
    inputs = []
    roots = set()
    options = []
    for arg in operands:
        if arg.startswith('--override='):
            require(tool == 'llvm-link', 'override on a non-link step')
            support = Path(arg.removeprefix('--override='))
            native_support(gate, major, support, record)
            inputs.append(support)
        elif arg.startswith('-'):
            options.append(arg)
        else:
            path = Path(arg)
            require(path.is_absolute(), 'relative bitcode input: '+arg)
            if tool == 'llvm-as':
                layout_constant(path, out)
            else:
                require(str(path) in provenance, 'unrecorded comparison input: '+arg)
                roots.update(provenance[str(path)])
            inputs.append(path)
    require(inputs, 'bitcode step has no inputs')
    match tool:
        case 'llvm-link':
            require(all(option == '--only-needed' for option in options), 'opaque bitcode link option')
        case 'llvm-as':
            require(len(inputs) == 1 and not options, 'opaque layout assembly')
        case 'llvm-extract':
            require(len(inputs) == 1 and bool(options) and
                    set(options) <= {'--func=memcpy', '--func=memmove', '--func=memset'},
                    'opaque runtime memory extraction')
        case 'opt':
            require(len(inputs) == 1 and options == ['-passes=strip-dead-prototypes,verify'],
                    'unapproved native bitcode pass')
        case _:
            raise GateError('native comparison audit: unknown bitcode transform '+tool)
    provenance[str(dest)] = roots


def audit_comparison(gate, major, device, runtime, arith, record, manifest, output):
    """Return the checked final argv, rebuilding a staged bitcode closure first.

    Legacy direct links still require the three exact inputs. Staged links consume
    the driver's public bitcode_link_argv plus ordered successful manifest commands.
    Rebuilding, rather than trusting a merged filename/hash claim, binds the final
    clang input to the exact device/runtime/arith bytes and rejects hidden leaves.
    """
    out = device.parent
    argv = record['link_argv']
    final_inputs = audit_link_argv(gate, major, argv, out, output)
    require(record['llvm_major'] == major, 'receipt LLVM major mismatch')
    for key, path in (('materialised', device), ('runtimes', runtime), ('arith', arith)):
        require(record[key+'_sha256'] == sha(path), key+' input hash mismatch')
    expected = {str(device), str(runtime), str(arith)}
    if 'bitcode_link_argv' not in record:
        require({str(p) for p in final_inputs} == expected, 'direct link dropped/substituted a required input')
        return argv

    first = record['bitcode_link_argv']
    require(first[:4] == [f'/usr/lib/llvm-{major}/bin/llvm-link', *map(str, (device, runtime, arith))]
            and first[4:-2] == ['--only-needed'] and first[-2] == '-o',
            'first bitcode merge must use the exact device, runtime and arith')
    commands = manifest['commands']
    start = next((i for i, c in enumerate(commands) if c['argv'] == first), None)
    require(start is not None, 'missing first bitcode merge in process receipt')
    provenance = {path:{path} for path in expected}
    steps = []
    tools = {'llvm-link', 'llvm-as', 'llvm-extract', 'opt'}
    for command in commands[start:]:
        args = command['argv']
        if args and Path(args[0]).name in tools and '-o' in args:
            require(command['success'] is True, 'failed bitcode transform in receipt')
            bitcode_step(gate, major, args, provenance, record, out)
            steps.append(args)
    roots = set()
    for path in final_inputs:
        require(str(path) in provenance, 'final link uses an unrecorded bitcode input: '+str(path))
        roots.update(provenance[str(path)])
    require(expected <= roots, 'final closure dropped the device, runtime or arith')
    require(all(str(p) not in expected for p in final_inputs), 'staged receipt bypasses its merged closure')
    for i, args in enumerate(steps):
        tag = str(out.relative_to(gate.run))+f'/audit-bitcode-{i}'
        gate.command(tag, args)
        if i == 0:
            require(record['linked_sha256'] == sha(Path(first[-1])), 'first bitcode merge hash mismatch')
    return argv
