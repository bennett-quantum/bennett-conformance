#!/usr/bin/env python3
"""Revision selection regressions; synthetic inputs do not establish acceptance."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

import gate
from gate_support import git

REPOSITORIES = ('plugin', 'vm', 'runtimes', 'driver', 'estimate', 'targets',
                'isa-spec', 'dialect-spec', 'conformance', 'arith')


def arguments(*argv):
    with patch.object(sys, 'argv', ['gate.py', *argv]), patch.object(gate, 'Gate') as implementation:
        gate.main()
        return implementation.call_args.args[0]


class RevisionInputs(unittest.TestCase):
    def test_help_advertises_every_frozen_input(self):
        help_text = subprocess.check_output(['bash', str(gate.HERE/'gate.sh'), '--help'], text=True)
        for name in REPOSITORIES:
            with self.subTest(name=name):
                self.assertIn('--'+name+'-rev', help_text)

    def test_arithmetic_pin_is_the_plugin_digest_publication(self):
        self.assertEqual(gate.PINS.get('bennett-arith'), '0dea42d001ffe832493fa0711da4a59b8cc1dd10')
        repo = gate.WORKSPACE/'bennett-arith'
        if not (repo/'.git').exists():
            self.skipTest('Bennett workspace object databases unavailable')
        declaration = gate.tomllib.loads(git(gate.WORKSPACE/'bennett-plugin', 'show',
                                                 gate.PINS['bennett-plugin']+':SPEC_VERSIONS.toml'))['arith']
        for major in (18, 23):
            publication = subprocess.check_output(['git', '-C', str(repo), 'show',
                                                  gate.PINS['bennett-arith']+f':dist/llvm{major}/bennett-arith.bc'])
            self.assertEqual(gate.hashlib.sha256(publication).hexdigest(), declaration[f'llvm{major}_sha256'])

    def test_default_reference_comes_from_isa_revision(self):
        selected = arguments()
        self.assertIsNone(selected.reference)
        self.assertEqual(selected.isa_spec_rev, gate.PINS['bennett-isa-spec'])
        self.assertNotIn('bennett-isa-spec-review', gate.PINS)

    def test_defaults_and_all_overrides(self):
        defaults = arguments()
        for name in REPOSITORIES:
            attr = name.replace('-', '_')+'_rev'
            with self.subTest(name=name):
                self.assertEqual(getattr(defaults, attr), gate.PINS['bennett-'+name])
                selected = arguments('--'+name+'-rev', 'origin/main')
                self.assertEqual(getattr(selected, attr), 'origin/main')

    def test_explicit_reference_conflicts_with_isa_revision(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as failure:
            arguments('--reference', '/reference', '--isa-spec-rev', 'main')
        self.assertEqual(failure.exception.code, 2)

    def test_production_uses_frozen_arithmetic_without_bypassing_digest_check(self):
        selected = arguments()
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            selected.output = tmp
            runner = gate.Gate(selected)
            runner.plugin_build = Path(tmp)/'plugin/build'
            runner.pub_arith = Path(tmp)/'frozen-arith'
            runner.command = Mock(return_value=b'')
            runner.production(18)
            for call in runner.command.call_args_list:
                argv = list(map(str, call.args[1]))
                self.assertIn('BENNETT_ARITH_PINNED_ROOT='+str(runner.pub_arith/'dist'), argv)
                self.assertIn('-u', argv)
                self.assertIn('BENNETT_ARITH_BITCODE', argv)
                self.assertFalse(any(s.startswith('BENNETT_ARITH_BITCODE=') for s in argv))

    def test_main_health_argv_accepts_real_help(self):
        path = gate.WORKSPACE/'bennett/tools/main_health.py'
        if not path.is_file():
            self.skipTest('read-only main-health source unavailable')
        spec = importlib.util.spec_from_file_location('rung4_main_health', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        help_text = subprocess.check_output(['bash', str(gate.HERE/'gate.sh'), '--help'], text=True)
        revisions = {name: gate.PINS[name] for name in module.LADDER_REPOS}
        argv = module.ladder_argv(gate.HERE/'gate.sh', revisions, help_text, '/output')
        selected = arguments(*argv[2:])
        for name, revision in revisions.items():
            self.assertEqual(getattr(selected, name.removeprefix('bennett-').replace('-', '_')+'_rev'), revision)


class FrozenInputs(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.workspace = self.root/'workspace'
        self.workspace.mkdir()
        self.commits = {}
        for name in REPOSITORIES:
            repo = self.workspace/('bennett-'+name)
            repo.mkdir()
            self.git(repo, 'init', '-q', '-b', 'main')
            self.git(repo, 'config', 'user.name', 'Revision fixture')
            self.git(repo, 'config', 'user.email', 'fixture@example.invalid')
            (repo/'publication').write_text('recorded '+name)
            if name == 'plugin':
                (repo/'SPEC_VERSIONS.toml').write_text('dialect="0.2.10"\nisa="0.8"\n[arith]\n')
            self.git(repo, 'add', '.')
            self.git(repo, 'commit', '-qm', 'recorded input')
            self.commits[name] = self.git(repo, 'rev-parse', 'HEAD')
        selected = arguments()
        selected.output = str(self.root/'output')
        for name, commit in self.commits.items():
            setattr(selected, name.replace('-', '_')+'_rev', commit)
        self.runner = gate.Gate(selected)
        self.runner.tmp = self.root/'cache'
        self.runner.sources.workspace = self.workspace

    def git(self, repo, *argv):
        return subprocess.check_output(['git', '-C', str(repo), *argv], text=True).strip()

    def test_every_input_is_detached_and_live_changes_do_not_change_the_receipt(self):
        self.runner.sources.prepare()
        for name, commit in self.commits.items():
            repo = 'bennett-'+name
            source = self.runner.sources.paths[repo]
            with self.subTest(repo=repo):
                self.assertEqual(self.git(source, 'rev-parse', '--abbrev-ref', 'HEAD'), 'HEAD')
                self.assertEqual(self.runner.manifest['frozen'][repo], commit)
                self.assertEqual(self.runner.manifest['pins'][repo], commit)
                (self.workspace/repo/'publication').write_text('live edit')
                self.assertEqual((source/'publication').read_text(), 'recorded '+name)
                self.assertIn(repo, self.runner.manifest['archive_sha256'])
        self.assertEqual(self.runner.manifest['dialect'], '0.2.10')

    def test_override_freezes_the_requested_commit_and_dirty_cache_is_rejected(self):
        repo = self.workspace/'bennett-runtimes'
        (repo/'publication').write_text('current runtime')
        self.git(repo, 'commit', '-am', 'new publication', '-q')
        current = self.git(repo, 'rev-parse', 'HEAD')
        self.runner.args.runtimes_rev = 'main'
        self.runner.sources.prepare()
        source = self.runner.sources.paths['bennett-runtimes']
        self.assertEqual(self.runner.manifest['pins']['bennett-runtimes'], current)
        self.assertEqual((source/'publication').read_text(), 'current runtime')
        (source/'publication').write_text('dirty cache')
        with self.assertRaisesRegex(gate.GateError, 'dirty or incorrect detached cache'):
            self.runner.sources.freeze('bennett-runtimes', current)

    def test_unknown_revision_fails_with_repository_context(self):
        with self.assertRaisesRegex(gate.GateError, 'bennett-runtimes: unknown revision'):
            self.runner.sources.freeze('bennett-runtimes', 'missing-revision')

    def test_explicit_plugin_follows_its_arithmetic_unless_arithmetic_is_explicit(self):
        repo = self.workspace/'bennett-arith'
        (repo/'publication').write_text('plugin dependency')
        self.git(repo, 'commit', '-am', 'new arithmetic', '-q')
        arithmetic = self.git(repo, 'rev-parse', 'HEAD')
        plugin = self.workspace/'bennett-plugin'
        declaration = plugin/'SPEC_VERSIONS.toml'
        declaration.write_text(declaration.read_text()+'commit="'+arithmetic+'"\n')
        self.git(plugin, 'commit', '-am', 'new plugin', '-q')
        self.runner.args.plugin_rev = 'main'
        self.runner.args.explicit_revisions = {'plugin'}
        self.runner.sources.prepare()
        self.assertEqual(self.runner.manifest['pins']['bennett-arith'], arithmetic)
        self.runner.args.explicit_revisions.add('arith')
        self.runner.sources.prepare()
        self.assertEqual(self.runner.manifest['pins']['bennett-arith'], self.commits['arith'])

    def test_reference_build_is_release_and_uses_frozen_isa_source(self):
        self.runner.sources.prepare()
        source = self.runner.sources.paths['bennett-isa-spec']
        executable = source/'target/release/bennett-isa-ref'
        def build(tag, argv):
            self.assertEqual(tag, 'build/reference-build')
            command = list(map(str, argv))
            self.assertIn('--release', command)
            self.assertIn('--offline', command)
            self.assertIn(str(source/'reference-interpreter/Cargo.toml'), command)
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b'fixture reference binary')
        self.runner.command = Mock(side_effect=build)
        self.runner.reference()
        self.runner.command.assert_called_once()
        self.assertEqual(self.runner.reference_exe, executable)
        self.assertEqual(self.runner.manifest['reference_profile'], 'release')
        self.assertEqual(self.runner.manifest['reference_sha256'], gate.sha(executable))

    def test_plugin_layout_has_frozen_build_pins_including_aliases(self):
        self.runner.sources.prepare()
        source = self.runner.sources.paths['bennett-plugin']
        (source/'gate.toml').write_text('[siblings.arithmetic]\nrepo="bennett-arith"\nsha="'+self.commits['arith']+'"\n')
        layout = self.runner.sources.plugin_layout()
        self.assertEqual((layout/'bennett-plugin').resolve(), source)
        self.assertEqual((layout/'arithmetic').resolve(), self.runner.sources.paths['bennett-arith'])
        self.assertEqual(self.runner.manifest['build_pins'], {'arithmetic': self.commits['arith']})
        self.runner.plugin_build = source/'build'
        self.runner.pub_runtimes = self.runner.sources.paths['bennett-runtimes']
        self.runner.pub_arith = self.runner.sources.paths['bennett-arith']
        self.runner.libraries = Mock()
        self.runner.command = Mock(return_value=b'')
        with patch.object(gate, 'sha', return_value='fixture digest'):
            self.runner.plugin()
        for call in self.runner.command.call_args_list:
            if call.args[0] in ('build/plugin-configure', 'build/plugin-build'):
                self.assertIn('BENNETT_GATE_LAYOUT='+str(layout), list(map(str, call.args[1])))


if __name__ == '__main__':
    unittest.main()
