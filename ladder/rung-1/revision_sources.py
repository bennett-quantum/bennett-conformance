"""Frozen sibling sources for the rung-1 and rung-2 revision selectors."""
from pathlib import Path
import subprocess

from gate_support import GateError, git, sha


REVISION_SIBLINGS = {
    'runtimes': 'use published runtime bitcode from this frozen revision',
    'driver': 'record a frozen driver revision; these rungs invoke the plugin directly',
    'isa-spec': 'build the release reference interpreter at this revision',
    'arith': 'use published arithmetic bitcode from this frozen revision',
}
# Same build hang guard as VM builds. Last rung-1 build timings are recorded in
# evidence/20261101T141544.771577Z/report.json; see README.md for outer run guards.
RUST_BUILD_HANG_GUARD_SECONDS = 600


def add_revision_options(parser):
    for name, description in REVISION_SIBLINGS.items():
        parser.add_argument('--' + name + '-rev', help=description)


def validate_revision_options(parser, args):
    for name, paths in (('runtimes', ('runtimes18', 'runtimes23')),
                        ('arith', ('arith18', 'arith23')),
                        ('isa-spec', ('reference',))):
        if getattr(args, name.replace('-', '_') + '_rev') is not None:
            for path in paths:
                if getattr(args, path) is not None:
                    parser.error(f'--{name}-rev conflicts with --{path}; select a revision or an explicit path')


class RevisionSources:
    def __init__(self, gate, workspace):
        self.gate = gate
        self.workspace = workspace
        self.paths = {}

    def freeze(self, name, revision):
        repo = self.workspace / name
        try:
            commit = git(repo, 'rev-parse', '--verify', '--end-of-options', revision + '^{commit}')
        except subprocess.CalledProcessError as exc:
            raise GateError(f'{name}: unknown revision {revision}') from exc
        source = self.gate.tmp / (name + '-' + commit)
        if not source.exists():
            self.gate.command('build/' + name + '-worktree',
                              ['git', '-C', repo, 'worktree', 'add', '--detach', source, commit])
        if (not (source / '.git').is_file()
                or git(source, 'rev-parse', '--abbrev-ref', 'HEAD') != 'HEAD'
                or git(source, 'rev-parse', 'HEAD') != commit
                or git(source, 'status', '--porcelain', '--untracked-files=no')):
            raise GateError(f'{name}: dirty or incorrect detached cache: {source}')
        self.gate.manifest.setdefault('pins', {})[name] = commit
        self.paths[name] = source
        return source

    def prepare(self):
        for name in REVISION_SIBLINGS:
            revision = getattr(self.gate.args, name.replace('-', '_') + '_rev', None)
            if revision is not None:
                self.freeze('bennett-' + name, revision)
        if 'bennett-driver' in self.paths:
            self.gate.manifest['driver_usage'] = 'revision recorded; gate invokes plugin directly, no driver binary'

    def reference(self):
        source = self.paths.get('bennett-isa-spec')
        if source is None:
            return Path(self.gate.args.reference)
        target = source / 'target'
        self.gate.command('build/reference-build', [
            'cargo', 'build', '--release', '--offline', '--jobs', self.gate.args.jobs,
            '--manifest-path', source / 'reference-interpreter/Cargo.toml', '--target-dir', target],
            timeout=RUST_BUILD_HANG_GUARD_SECONDS)
        executable = target / 'release/bennett-isa-ref'
        if not executable.is_file():
            raise GateError('missing built reference interpreter: ' + str(executable))
        self.gate.manifest['reference_profile'] = 'release'
        self.gate.manifest['reference_sha256'] = sha(executable)
        return executable
