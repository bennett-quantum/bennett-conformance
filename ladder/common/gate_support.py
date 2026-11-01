"""Shared command evidence and red dependency propagation for ladder gates."""
import hashlib
import re
import subprocess
import time

class GateError(Exception):
    pass


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class RecordedGate:
    def command(self, tag, argv, timeout=None):
        argv = list(map(str, argv))
        dest = self.run / tag
        dest.parent.mkdir(parents=True, exist_ok=True)
        print('+ ' + ' '.join(argv), flush=True)
        started = time.monotonic()
        limit = timeout or self.args.timeout
        try:
            p = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               timeout=limit)
            code, out, err = p.returncode, p.stdout, p.stderr
        except subprocess.TimeoutExpired as exc:
            code, out, err = 124, exc.stdout or b'', (exc.stderr or b'') + f'\ntimeout after {limit} seconds\n'.encode()
        except OSError as exc:
            code, out, err = 127, b'', str(exc).encode()
        dest.with_suffix('.stdout').write_bytes(out)
        dest.with_suffix('.stderr').write_bytes(err)
        self.manifest['commands'].append({'tag':tag, 'argv':argv, 'exit':code, 'elapsed_seconds':round(time.monotonic()-started,3), 'timeout_seconds':limit})
        if code:
            raise GateError(f'{tag}: exit {code}\n{err.decode(errors="replace")}')
        return out

    def stage(self, major, name, action, deps=()):
        blockers = [self.rows[major][dep] for dep in deps
                    if self.rows[major][dep]['status'] != 'PASS']
        if blockers:
            self.rows[major][name] = {'status':'BLOCKED', 'beads':sorted({b for r in blockers for b in r['beads']}),
                                      'error':'Not run; requires ' + ', '.join(deps)}
            return
        try:
            action()
            self.rows[major][name] = {'status':'PASS', 'beads':[], 'error':''}
        except (GateError, ValueError, KeyError, AssertionError, OSError) as exc:
            error = str(exc) or repr(exc)
            beads = [b['id'] for b in self.known if re.search(b['pattern'], error)]
            self.rows[major][name] = {'status':'FAIL', 'beads':beads or ['UNFILED'], 'error':error}
            print(error, flush=True)

