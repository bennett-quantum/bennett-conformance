"""Select and record immutable inputs for the SQLite ladder gate."""
import hashlib
import os
from pathlib import Path
import subprocess
import tomllib

from gate_support import GateError, git, sha

REPOSITORIES = ('plugin', 'vm', 'runtimes', 'driver', 'estimate', 'targets',
                'isa-spec', 'dialect-spec', 'conformance', 'arith')


def select_revisions(parser, args, pins):
    args.explicit_revisions = {name for name in REPOSITORIES
                               if getattr(args, name.replace('-', '_')+'_rev') is not None}
    if args.reference is not None and 'isa-spec' in args.explicit_revisions:
        parser.error('--isa-spec-rev conflicts with --reference; select a revision or an explicit path')
    for name in REPOSITORIES:
        attribute = name.replace('-', '_')+'_rev'
        if getattr(args, attribute) is None:
            setattr(args, attribute, pins['bennett-'+name])


class FrozenSources:
    def __init__(self, gate, workspace):
        self.gate = gate
        self.workspace = Path(os.environ.get('BENNETT_GATE_WORKSPACE', workspace)).resolve()
        self.paths = {}

    def detached(self, repo, revision):
        origin = self.workspace/repo
        try:
            commit = git(origin, 'rev-parse', '--verify', '--end-of-options', revision+'^{commit}')
        except subprocess.CalledProcessError as exc:
            raise GateError(f'{repo}: unknown revision {revision}') from exc
        source = self.gate.tmp/'sources'/f'{repo}-{commit}'
        if not source.exists():
            source.parent.mkdir(parents=True, exist_ok=True)
            self.gate.command('build/'+repo+'-worktree',
                              ['git', '-C', origin, 'worktree', 'add', '--detach', source, commit])
        if (not (source/'.git').is_file() or git(source, 'rev-parse', '--abbrev-ref', 'HEAD') != 'HEAD'
                or git(source, 'rev-parse', 'HEAD') != commit
                or git(source, 'status', '--porcelain', '--untracked-files=no')):
            raise GateError(f'{repo}: dirty or incorrect detached cache: {source}')
        return commit, source

    def freeze(self, repo, revision):
        commit, source = self.detached(repo, revision)
        archive = self.gate.command('build/'+repo+'-archive', ['git', '-C', source, 'archive', commit])
        (self.gate.run/f'build/{repo}-archive.stdout').unlink()
        self.gate.manifest.setdefault('archive_sha256', {})[repo] = hashlib.sha256(archive).hexdigest()
        self.gate.manifest['pins'][repo] = commit
        self.gate.manifest.setdefault('frozen', {})[repo] = commit
        self.paths[repo] = source
        return source

    def prepare(self):
        plugin = self.freeze('bennett-plugin', self.gate.args.plugin_rev)
        versions = tomllib.loads((plugin/'SPEC_VERSIONS.toml').read_text())
        for field in ('dialect', 'isa'):
            if not isinstance(versions.get(field), str) or not versions[field].strip():
                raise GateError(f'selected plugin declaration missing {field}: {plugin}/SPEC_VERSIONS.toml')
            self.gate.manifest[field] = versions[field]
        self.gate.manifest['plugin_version_declaration_sha256'] = sha(plugin/'SPEC_VERSIONS.toml')
        arithmetic = self.gate.args.arith_rev
        explicit = self.gate.args.explicit_revisions
        if 'plugin' in explicit and 'arith' not in explicit:
            arithmetic = versions['arith']['commit']
            self.gate.manifest['arith_selection'] = 'selected plugin published dependency'
        for name in REPOSITORIES:
            if name == 'plugin' or (name == 'isa-spec' and self.gate.args.reference is not None):
                continue
            revision = arithmetic if name == 'arith' else getattr(self.gate.args, name.replace('-', '_')+'_rev')
            self.freeze('bennett-'+name, revision)

    def plugin_layout(self):
        source = self.paths['bennett-plugin']
        config = source/'gate.toml'
        dependencies = (tomllib.loads(config.read_text())['siblings'] if config.is_file() else
                        {'bennett-arith': {'sha': self.gate.manifest['frozen']['bennett-arith']}})
        layout = source/'.tmp/gate-layout'
        layout.mkdir(parents=True, exist_ok=True)
        targets = {'bennett-plugin': source}
        build_pins = {}
        for name, pin in dependencies.items():
            revision = pin.get('sha')
            if not isinstance(revision, str) or not revision:
                raise GateError(f'plugin build pin {name}: missing sha in {config}')
            commit, target = self.detached(pin.get('repo', name), revision)
            targets[name] = target
            build_pins[name] = commit
        for name, target in targets.items():
            link = layout/name
            if link.is_symlink():
                if link.resolve() != target:
                    raise GateError('incorrect plugin build layout link: '+str(link))
            elif link.exists():
                raise GateError('plugin build layout requires a frozen-source link: '+str(link))
            else:
                link.symlink_to(target, target_is_directory=True)
        self.gate.manifest['build_pins'] = build_pins
        self.gate.manifest['plugin_build_layout'] = str(layout)
        return layout
