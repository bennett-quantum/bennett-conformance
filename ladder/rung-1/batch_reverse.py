"""Independent consumer of the published reverse-action-batch/1 binary interface."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import signal
import struct
import subprocess
import sys
import threading
import time

from progress import write_summary

FORMAT = 'bennett.reverse-action-batch/1'


class BatchError(ValueError):
    pass


class BatchTimeout(BatchError):
    pass


def require(condition, message):
    if not condition:
        raise BatchError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()+b'\n'


def read_json(stream, digest):
    raw = stream.readline(65537)
    require(raw.endswith(b'\n') and len(raw) <= 65536, 'missing/oversized JSON frame')
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError) as error:
        raise BatchError('invalid JSON frame') from error
    require(raw == canonical(value), 'JSON frame is not canonical JCS')
    digest.update(raw)
    return value


def expected_header(identity, arity, budget, lo, hi):
    require(identity.get('status') == 'ok' and not identity.get('unresolved_imports'), 'unadmitted object identity')
    symbol = identity['symbols']['rung1_full']
    components = [{'group':'mut', 'name':'t', 'cell':None, 'offset':0, 'width':8},
                  {'group':'mut', 'name':'e', 'cell':None, 'offset':8, 'width':4}]
    components += [{'group':'in', 'name':'x'+str(i), 'cell':None, 'offset':12+8*i, 'width':8}
                   for i in range(arity)]
    bits = 12+8*arity
    return {'type':'header', 'format':FORMAT, 'encoding':'binary', 'action':'rung1_full',
            'adjoint':False, 'object_hash':identity['object_hash'],
            'interface_hash':symbol['interface_hash'], 'closure_hash':symbol.get('closure_hash'),
            'args':['token:'+budget], 'token_aliases':[0], 'domain_bits':bits,
            'record_bytes':2+2*((bits+7)//8), 'mode':'range', 'lo':str(lo), 'hi':str(hi),
            'rank_order':[*range(2,2+arity),0,1], 'components':components}


def read_exact(stream, size):
    pieces = bytearray()
    while len(pieces) < size:
        piece = stream.read(size-len(pieces))
        require(bool(piece), 'truncated case stream or premature footer')
        pieces.extend(piece)
    return pieces


def check_records(data, arity, native, rank):
    layout = '<BB3s3s' if arity == 1 else '<BBII'
    previous = -1
    for marker, flags, forward, reverse in struct.iter_unpack(layout, data):
        if marker != 1 or flags != 127:
            raise BatchError(f'rank {rank}: invalid case tag or phase/cleanup/preservation/reversal flags {marker}/{flags}')
        index = rank >> 12
        if index != previous:
            inputs = index << 12 if arity == 1 else ((index >> 8) << 12) | ((index & 255) << 20)
            payload = native[index]
            previous = index
        entry = inputs | ((rank >> 4) & 255) | ((rank & 15) << 8)
        if arity == 1:
            forward, reverse = int.from_bytes(forward, 'little'), int.from_bytes(reverse, 'little')
        if forward != entry ^ payload:
            raise BatchError(f'rank {rank}: full forward tuple/native mismatch (including padding)')
        if reverse != entry:
            raise BatchError(f'rank {rank}: full reversed tuple differs from entry (including padding)')
        rank += 1
    return rank


def verify_stream(stream, identity, native, arity, budget, lo, hi):
    require(arity in (1,2) and len(native) == 256**arity, 'native table length/arity mismatch')
    require(0 <= lo <= hi <= 256**arity*4096, 'range outside the complete rung-1 domain')
    digest = hashlib.sha256()
    head = read_json(stream, digest)
    expected = expected_header(identity, arity, budget, lo, hi)
    require(canonical(head) == canonical(expected), 'batch header differs from admitted object, interface, invocation or range')
    rank = lo
    while rank < hi:
        count = min(65536, hi-rank)
        data = read_exact(stream, count*expected['record_bytes'])
        digest.update(data)
        rank = check_records(data, arity, native, rank)
    marker = stream.read(1)
    require(marker == b'\0', 'missing footer marker or extra case')
    digest.update(marker)
    tail = read_json(stream, digest)
    wanted = {'type':'footer', 'format':FORMAT, 'complete':True, 'cases':str(hi-lo),
              'failed':'0', 'first_failed':None}
    require(canonical(tail) == canonical(wanted), 'incomplete/failed/count-mismatched footer')
    require(stream.read(1) == b'', 'extra bytes after the complete footer')
    return {'cases':hi-lo, 'header':head, 'footer':tail, 'stream_sha256':digest.hexdigest()}


def ranges(total, size):
    require(total > 0 and size > 0, 'nonpositive domain or shard size')
    return [(lo,min(lo+size,total)) for lo in range(0,total,size)]


def coverage(receipts, total, bindings=None):
    cursor = 0
    for receipt in sorted(receipts, key=lambda r:r['lo']):
        require(receipt['complete'] is True and receipt['lo'] == cursor and receipt['hi'] > cursor,
                'incomplete, overlapping or gapped shard coverage')
        require(receipt['cases'] == receipt['hi']-cursor, 'shard count mismatch')
        if bindings is not None:
            require(all(receipt.get(k) == v for k,v in bindings.items()), 'shards use different native/object/tool/invocation bindings')
        cursor = receipt['hi']
    require(cursor == total, 'domain not completely covered by shards')
    return cursor


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def kill_group(process):
    if process.poll() is None:
        try: os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError: pass


@contextmanager
def output_process(argv, stderr_path, timeout):
    with stderr_path.open('wb') as stderr:
        process = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=stderr, start_new_session=True)
        expired = threading.Event()
        def expire():
            expired.set()
            kill_group(process)
        # Leave time to reap/publish UNKNOWN before the unchanged outer cap.
        timer = threading.Timer(max(0.1,timeout-2), expire)
        timer.start()
        try:
            yield process
        except Exception as error:
            if expired.is_set():
                raise BatchTimeout('batch command timed out (UNKNOWN)') from error
            raise
        finally:
            timer.cancel()
            kill_group(process)
            process.wait()
            process.stdout.close()


def execute_shard(executor, obj, identity, native, arity, budget, lo, hi, evidence,
                  executor_sha256, object_sha256, timeout):
    require(sha(executor) == executor_sha256, 'executor binary hash differs from the pinned build')
    require(sha(obj) == object_sha256, 'object bytes changed after interface admission')
    evidence = Path(evidence)
    evidence.parent.mkdir(parents=True, exist_ok=True)
    argv = [str(executor),'reverse-action-batch',str(obj),'rung1_full','--range',f'{lo}..{hi}',
            '--format','binary','--arg','token:'+budget,'--token-aliases','[0]']
    receipt = {'argv':argv, 'lo':lo, 'hi':hi, 'complete':False,
               'executor_sha256':executor_sha256, 'object_sha256':object_sha256,
               'native_sha256':hashlib.sha256(native).hexdigest(), 'budget':budget}
    write_summary(evidence, receipt)
    started = time.monotonic()
    stderr_path = evidence.with_suffix('.stderr')
    process = None
    try:
        with output_process(argv,stderr_path,timeout) as process:
            checked = verify_stream(process.stdout, identity, native, arity, budget, lo, hi)
            process.wait()
            require(process.returncode == 0, 'batch process failed despite its output footer')
            receipt.update(checked, complete=True, exit=0)
    except Exception as error:
        receipt.update(error=str(error), exit=124 if isinstance(error,BatchTimeout) else 1)
        raise
    finally:
        if process is not None:
            receipt['process_exit'] = process.returncode
        receipt['elapsed_seconds'] = time.monotonic()-started
        receipt['stderr'] = stderr_path.read_text(errors='replace')[-8000:]
        write_summary(evidence, receipt)
    return receipt


def cli(args, parser, native):
    if not all((args.identity,args.executor_sha256,args.object_sha256)):
        parser.error('batch shards require admitted identity and executable/object hashes')
    try:
        bounds = args.range.split('..')
        require(len(bounds) == 2 and all(str(int(v)) == v for v in bounds), 'noncanonical range')
        lo,hi = map(int,bounds)
        identity = json.loads(Path(args.identity).read_text())
        result = execute_shard(args.executor,args.object,identity,native,args.arity,args.budget,
            lo,hi,args.evidence,args.executor_sha256,args.object_sha256,args.timeout)
        print(json.dumps({'cases':result['cases'],'range':[lo,hi],'stream_sha256':result['stream_sha256']}))
        return 0
    except BatchTimeout as error:
        print(str(error),file=sys.stderr)
        return 124
    except (ValueError,KeyError,OSError) as error:
        print(f'batch shard rejected: {error}; receipt: {args.evidence}',file=sys.stderr)
        return 1
