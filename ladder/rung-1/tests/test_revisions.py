"""Revision-selection contract shared by both ladder gates and main-health."""
import argparse
import importlib.util
import os
import sys
import tempfile
from unittest.mock import patch
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'ladder/rung-1'))
sys.path.insert(0, str(ROOT / 'ladder/common'))
from gate_support import GateError, sha

META_HEALTH = ROOT.parent / 'bennett/tools/main_health.py'
spec = importlib.util.spec_from_file_location('main_health', META_HEALTH)
health = importlib.util.module_from_spec(spec)
spec.loader.exec_module(health)


class RevisionCLI(unittest.TestCase):
    def test_help_exposes_all_main_health_revisions(self):
        for rung in ('rung-1', 'rung-2'):
            with self.subTest(rung=rung):
                stub = ROOT / 'ladder' / rung / 'gate.sh'
                help_text = subprocess.check_output(['bash', str(stub), '--help'], text=True)
                for name in health.LADDER_REPOS:
                    self.assertIn('--' + name.removeprefix('bennett-') + '-rev', help_text)

    def test_shell_help_keeps_the_callers_tmpdir(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
            # A python shim observes the environment after gate.sh, before gate.py.
            shim = Path(temp) / 'python3'
            shim.write_text('#!/bin/sh\nprintf \"%s\\n\" \"$TMPDIR\"\n')
            shim.chmod(0o755)
            env = dict(os.environ, PATH=temp + os.pathsep + os.environ['PATH'])
            for rung in ('rung-1', 'rung-2'):
                with self.subTest(rung=rung):
                    output = subprocess.check_output(['bash', str(ROOT / 'ladder' / rung / 'gate.sh'), '--help'], text=True, env=env)
                    self.assertEqual(output.strip(), os.environ['TMPDIR'])

    def test_main_health_can_construct_the_actual_gate_command(self):
        revisions = {name: '1' * 40 for name in health.LADDER_REPOS}
        for rung in ('rung-1', 'rung-2'):
            with self.subTest(rung=rung):
                stub = ROOT / 'ladder' / rung / 'gate.sh'
                help_text = subprocess.check_output(['bash', str(stub), '--help'], text=True)
                argv = health.ladder_argv(stub, revisions, help_text, ROOT / '.tmp/ladder-revs/output')
                for name, rev in revisions.items():
                    option = '--' + name.removeprefix('bennett-') + '-rev'
                    self.assertEqual(argv[argv.index(option) + 1], rev)


def load_gate(rung):
    spec = importlib.util.spec_from_file_location(rung.replace('-', '_'), ROOT / 'ladder' / rung / 'gate.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FrozenRevisions(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name)
        self.revisions = {}
        for name in ('runtimes', 'arith', 'isa-spec', 'driver', 'plugin'):
            repo = self.workspace / ('bennett-' + name)
            repo.mkdir()
            self.git(repo, 'init', '-q')
            self.git(repo, 'config', 'user.name', 'Revision test')
            self.git(repo, 'config', 'user.email', 'test@example.invalid')
            old, new = [], []
            for value, destination in (('old', old), ('new', new)):
                (repo / 'version').write_text(value)
                if name in ('arith', 'runtimes'):
                    for major in (18, 23):
                        artifact = repo / f'dist/llvm{major}/bennett-{name}.bc'
                        artifact.parent.mkdir(parents=True, exist_ok=True)
                        artifact.write_text(value + name + str(major))
                if name == 'plugin':
                    arith = self.workspace / 'bennett-arith'
                    pin = self.revisions['arith'][0 if value == 'old' else 1]
                    (repo / 'SPEC_VERSIONS.toml').write_text(
                        '[arith]\ncommit = "' + pin + '"\n' +
                        '\n'.join(f'llvm{m}_sha256 = "{sha(arith / f"dist/llvm{m}/bennett-arith.bc")}"'
                                  for m in (18, 23)) + '\n')
                self.git(repo, 'add', '.')
                self.git(repo, 'commit', '-qm', value)
                destination.append(self.git(repo, 'rev-parse', 'HEAD'))
            self.revisions[name] = (old[0], new[0])
            (repo / 'version').write_text('dirty live tree')
        self.commands = []

    @staticmethod
    def git(repo, *args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()

    def command(self, tag, argv, **kwargs):
        argv = list(map(str, argv))
        self.commands.append((tag, argv))
        if argv[0] == 'git':
            return subprocess.check_output(argv, stderr=subprocess.PIPE)
        if argv[0] == 'cmake' and '-S' in argv:
            # Model the plugin's configure-time requirement, before a build can run.
            import tomllib
            source = Path(argv[argv.index('-S') + 1])
            commit = tomllib.loads((source / 'SPEC_VERSIONS.toml').read_text())['arith']['commit']
            cache = source / '.tmp/gate/cache' / ('bennett-arith@' + commit[:12]) / 'bennett-arith'
            self.assertTrue(cache.is_dir(), 'plugin arithmetic pin must exist before CMake configures')
            self.assertEqual(self.git(cache, 'rev-parse', 'HEAD'), commit)
        if argv[0] == 'cmake' and '--build' in argv:
            build = Path(argv[argv.index('--build') + 1])
            build.mkdir(exist_ok=True)
            for major in (18, 23):
                (build / f'libbennett-plugin-{major}.so').write_text('plugin')
        if argv[0] == 'cargo':
            target = Path(argv[argv.index('--target-dir') + 1])
            executable = target / 'release/bennett-isa-ref'
            executable.parent.mkdir(parents=True, exist_ok=True)
            source = Path(argv[argv.index('--manifest-path') + 1]).parents[1]
            executable.write_text((source / 'version').read_text())
        return b''

    def make_gate(self, rung, **overrides):
        module = load_gate(rung)
        args = dict(output=str(self.workspace / (rung + '-out')), timeout=300, jobs=3,
                    plugin_rev=self.revisions['plugin'][1], vm_rev=None,
                    runtimes_rev=self.revisions['runtimes'][0],
                    arith_rev=self.revisions['arith'][1],
                    isa_spec_rev=self.revisions['isa-spec'][0],
                    driver_rev=self.revisions['driver'][0],
                    reference='/explicit/default/reference', runtimes18=None, runtimes23=None,
                    arith18=None, arith23=None)
        args.update(overrides)
        with patch.object(module, 'WORKSPACE', self.workspace):
            gate = module.Gate(argparse.Namespace(**args))
        gate.tmp = self.workspace / (rung + '-cache')
        gate.tmp.mkdir(exist_ok=True)
        gate.command = self.command
        return gate

    def test_both_gates_link_selected_publications_and_build_frozen_release_reference(self):
        for rung in ('rung-1', 'rung-2'):
            with self.subTest(rung=rung):
                g = self.make_gate(rung)
                g.sources.prepare()
                # Plugin archive resolution also uses the fake sibling object source.
                with patch.dict(g.plugin.__globals__, WORKSPACE=self.workspace):
                    g.plugin()
                g.build_reference()
                self.assertEqual(g.reference_exe.read_text(), 'old')
                self.assertEqual(g.manifest['reference_profile'], 'release')
                self.assertEqual(g.manifest['pins']['bennett-driver'], self.revisions['driver'][0])
                self.assertIn('no driver binary', g.manifest['driver_usage'])
                self.assertEqual(self.git(g.sources.paths['bennett-isa-spec'], 'rev-parse', 'HEAD'),
                                 self.revisions['isa-spec'][0])
                for major in (18, 23):
                    if rung == 'rung-1':
                        runtime, arith = g.runtime_files(major)
                    else:
                        g.runtime_link(major)
                        argv = self.commands[-1][1]
                        runtime, arith = map(Path, argv[2:4])
                    self.assertEqual(runtime.read_text(), 'oldruntimes' + str(major))
                    self.assertEqual(arith.read_text(), 'newarith' + str(major))
                    self.assertNotEqual(runtime.parent.parent.parent, self.workspace / 'bennett-runtimes')
                cargo = next(argv for tag, argv in self.commands if tag == 'build/reference-build')
                self.assertIn('--release', cargo)
                self.assertIn('--offline', cargo)
                self.assertEqual(cargo[cargo.index('--jobs') + 1], '3')

    def test_plugin_pin_supplies_arith_when_no_arith_revision_is_given(self):
        for rung in ('rung-1', 'rung-2'):
            with self.subTest(rung=rung):
                g = self.make_gate(rung, arith_rev=None)
                g.sources.prepare()
                with patch.dict(g.plugin.__globals__, WORKSPACE=self.workspace):
                    g.plugin()
                self.assertEqual(g.manifest['pins']['bennett-arith'], self.revisions['arith'][1])
                self.assertEqual((g.sources.paths['bennett-arith'] / 'version').read_text(), 'new')

    def test_main_health_workspace_resolves_arithmetic_object_source(self):
        # main-health links its five LADDER_REPOS beside a detached conformance.
        # It provides BENNETT_GATE_WORKSPACE for additional pinned object sources.
        nested = self.workspace / 'health-checkouts'
        nested.mkdir()
        for name in health.LADDER_REPOS:
            (nested / name).symlink_to(self.workspace / name, target_is_directory=True)
        g = self.make_gate('rung-1')
        with patch.dict(os.environ, BENNETT_GATE_WORKSPACE=str(self.workspace)):
            sources = g.sources.__class__(g, nested)
            source = sources.freeze('bennett-arith', self.revisions['arith'][1])
        self.assertEqual((source / 'version').read_text(), 'new')

    def test_cached_sources_are_rechecked_and_dirty_cache_is_rejected(self):
        g = self.make_gate('rung-1')
        source = g.sources.freeze('bennett-runtimes', self.revisions['runtimes'][0])
        self.assertEqual(g.sources.freeze('bennett-runtimes', self.revisions['runtimes'][0]), source)
        (source / 'version').write_text('dirty frozen tree')
        with self.assertRaisesRegex(GateError, 'dirty or incorrect detached cache'):
            g.sources.freeze('bennett-runtimes', self.revisions['runtimes'][0])

    def test_unknown_revisions_fail_before_building(self):
        for name in ('runtimes', 'driver', 'isa-spec', 'arith'):
            with self.subTest(sibling=name):
                g = self.make_gate('rung-1')
                self.commands.clear()
                with self.assertRaisesRegex(GateError, 'unknown revision'):
                    g.sources.freeze('bennett-' + name, 'f' * 40)
                self.assertEqual(self.commands, [])

    def test_default_reference_is_preserved_without_revision(self):
        g = self.make_gate('rung-1', isa_spec_rev=None)
        self.assertEqual(g.sources.reference(), Path('/explicit/default/reference'))
        self.assertEqual(self.commands, [])

    def test_arith_revision_still_requires_plugin_digest(self):
        for rung in ('rung-1', 'rung-2'):
            with self.subTest(rung=rung):
                g = self.make_gate(rung, arith_rev=self.revisions['arith'][0])
                g.sources.prepare()
                with patch.dict(g.plugin.__globals__, WORKSPACE=self.workspace):
                    g.plugin()
                with self.assertRaisesRegex((GateError, AssertionError), 'arith'):
                    if rung == 'rung-1':
                        g.runtime_files(18)
                    else:
                        g.runtime_link(18)


class RevisionConflicts(unittest.TestCase):
    def test_cli_defaults_keep_existing_revision_and_reference_choices(self):
        for rung in ('rung-1', 'rung-2'):
            module = load_gate(rung)
            with self.subTest(rung=rung), patch.object(module, 'Gate') as constructor:
                with patch.object(sys, 'argv', ['gate.py']):
                    module.main()
                args = constructor.call_args.args[0]
                self.assertEqual(args.plugin_rev, module.PINS['bennett-plugin'] if rung == 'rung-1' else None)
                self.assertEqual(args.vm_rev, module.PINS['bennett-vm'] if rung == 'rung-1' else None)
                self.assertEqual(Path(args.reference), module.WORKSPACE / 'bennett-isa-spec-review/reference-interpreter/target/release/bennett-isa-ref')
                for name in ('runtimes_rev', 'arith_rev', 'isa_spec_rev', 'driver_rev'):
                    self.assertIsNone(getattr(args, name))

    def test_explicit_paths_conflict_with_revisions_before_any_run(self):
        pairs = [('runtimes', 'runtimes18'), ('runtimes', 'runtimes23'),
                 ('arith', 'arith18'), ('arith', 'arith23'), ('isa-spec', 'reference')]
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
            output = Path(temp) / 'must-not-be-created'
            for rung in ('rung-1', 'rung-2'):
                for sibling, path in pairs:
                    with self.subTest(rung=rung, sibling=sibling, path=path):
                        result = subprocess.run(['bash', str(ROOT / 'ladder' / rung / 'gate.sh'),
                            '--output', str(output), '--' + sibling + '-rev', 'f' * 40,
                            '--' + path, '/explicit/path'], capture_output=True, text=True)
                        self.assertEqual(result.returncode, 2)
                        self.assertIn('conflicts with --' + path, result.stderr)
                        self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
