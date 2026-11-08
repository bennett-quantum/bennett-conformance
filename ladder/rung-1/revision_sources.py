"""Frozen sibling sources for the rung-1 and rung-2 revision selectors."""
import os
from pathlib import Path
import subprocess
import tomllib

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


def apply_recorded_pins(args, pins):
    args.explicit_revisions = frozenset(name for name, value in vars(args).items()
                                        if name.endswith('_rev') and value is not None)
    for name in ('plugin', 'vm', *REVISION_SIBLINGS):
        attribute = name.replace('-', '_') + '_rev'
        if getattr(args, attribute) is None:
            setattr(args, attribute, pins['bennett-' + name])


class RevisionSources:
    def __init__(self, gate, workspace):
        self.gate = gate
        self.workspace = Path(os.environ.get('BENNETT_GATE_WORKSPACE', workspace)).resolve()
        self.paths = {}

    def detached(self, name, revision):
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
        return commit, source

    def freeze(self, name, revision):
        commit, source = self.detached(name, revision)
        self.gate.manifest.setdefault('pins', {})[name] = commit
        self.paths[name] = source
        return source

    def plugin_versions(self, plugin_source):
        """Revision-selected production follows the selected plugin's declaration."""
        declaration = plugin_source / 'SPEC_VERSIONS.toml'
        versions = tomllib.loads(declaration.read_text())
        for name in ('dialect', 'isa'):
            version = versions.get(name)
            if not isinstance(version, str) or not version.strip():
                raise GateError(f'selected plugin declaration {declaration}: missing {name} version')
            self.gate.manifest[name] = version
        self.gate.manifest['plugin_version_declaration_sha256'] = sha(declaration)

    def plugin_layout(self, plugin_source):
        """Expose the plugin's published build pins beside its archived source."""
        config = plugin_source / 'gate.toml'
        pins = (tomllib.loads(config.read_text())['siblings'] if config.is_file() else
                {'bennett-arith': {'sha': self.gate.args.arith_rev}})
        layout = plugin_source / '.tmp/gate-layout'
        layout.mkdir(parents=True, exist_ok=True)
        targets = {'bennett-plugin': plugin_source}
        build_pins = {}
        for name, pin in pins.items():
            revision = pin.get('sha')
            if not isinstance(revision, str) or not revision:
                raise GateError(f'plugin build pin {name}: missing sha in {config}')
            commit, target = self.detached(pin.get('repo', name), revision)
            targets[name] = target
            build_pins[name] = commit
        for name, target in targets.items():
            link = layout / name
            if link.is_symlink():
                if link.resolve() != target:
                    raise GateError('incorrect plugin build layout link: ' + str(link))
            elif link.exists():
                raise GateError('plugin build layout requires a frozen-source link: ' + str(link))
            else:
                link.symlink_to(target, target_is_directory=True)
        self.gate.manifest['build_pins'] = build_pins
        self.gate.manifest['plugin_build_layout'] = str(layout)
        return layout

    def arithmetic_revision(self, plugin_source):
        """An explicit plugin follows its published dependency unless arith is explicit."""
        declared = tomllib.loads((plugin_source / 'SPEC_VERSIONS.toml').read_text())['arith']
        explicit = self.gate.args.explicit_revisions
        if 'plugin_rev' in explicit and 'arith_rev' not in explicit and 'commit' in declared:
            self.gate.manifest['arith_selection'] = 'selected plugin published dependency'
            return declared['commit']
        return self.gate.args.arith_rev

    def prepare(self):
        for name in REVISION_SIBLINGS:
            revision = getattr(self.gate.args, name.replace('-', '_') + '_rev', None)
            if name == 'isa-spec' and self.gate.args.reference is not None:
                continue
            if revision is not None:
                self.freeze('bennett-' + name, revision)
        if 'bennett-driver' in self.paths:
            self.gate.manifest['driver_usage'] = 'revision recorded; gate invokes plugin directly, no driver binary'

    def reference(self):
        if self.gate.args.reference is not None:
            return Path(self.gate.args.reference)
        source = self.paths['bennett-isa-spec']
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
