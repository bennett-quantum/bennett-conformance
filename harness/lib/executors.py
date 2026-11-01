"""Black-box executors: any binary speaking the ISA command-line contract, with an environment selection."""
import hashlib
import os
import resource
import subprocess
import time

from compare import classify

STRIP_ENV = ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")
ADDRESS_SPACE_LIMIT = 8 * 1024 ** 3


def _limit():
    resource.setrlimit(resource.RLIMIT_AS, (ADDRESS_SPACE_LIMIT, ADDRESS_SPACE_LIMIT))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


class Outcome:
    __slots__ = ("code", "stdout", "stderr", "timed_out", "kind", "elapsed", "tape", "replay_ok", "replay_note", "audit")

    def __init__(self, code, stdout, stderr, timed_out, elapsed):
        self.code, self.stdout, self.stderr, self.timed_out, self.elapsed = code, stdout, stderr, timed_out, elapsed
        self.kind = classify(code, stdout, stderr, timed_out)
        self.tape = None
        self.replay_ok = None
        self.replay_note = ""
        self.audit = False


class Executor:
    def __init__(self, name, argv, env=None, timeout=60.0):
        self.name, self.argv, self.env, self.timeout = name, list(argv), dict(env or {}), timeout

    def fingerprint(self):
        path = self.argv[-1]
        with open(path, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()

    def describe(self):
        return {"argv": self.argv, "env": self.env, "sha256": self.fingerprint()}

    def run(self, args, cwd=None):
        env = {k: v for k, v in os.environ.items() if k not in STRIP_ENV}
        env.update(self.env)
        started = time.monotonic()
        try:
            p = subprocess.run(self.argv + list(args), cwd=cwd, env=env, capture_output=True,
                               timeout=self.timeout, preexec_fn=_limit)
        except subprocess.TimeoutExpired as e:
            return Outcome(None, e.stdout or b"", (e.stderr or b"").decode("utf-8", "replace"), True,
                           time.monotonic() - started)
        return Outcome(p.returncode, p.stdout, p.stderr.decode("utf-8", "replace"), False, time.monotonic() - started)


def parse_spec(text, timeout):
    """`name=/path/to/binary[,ENV=VALUE...]`, the command-line form of an executor."""
    if "=" not in text:
        raise ValueError("executor spec must be name=path[,ENV=VALUE...]: %r" % text)
    name, rest = text.split("=", 1)
    parts = rest.split(",")
    env = {}
    for item in parts[1:]:
        k, _, v = item.partition("=")
        if not k or not _:
            raise ValueError("bad environment item %r in %r" % (item, text))
        env[k] = v
    if not os.access(parts[0], os.X_OK):
        raise ValueError("executor %s: %s is not an executable file" % (name, parts[0]))
    return Executor(name, [parts[0]], env, timeout)
