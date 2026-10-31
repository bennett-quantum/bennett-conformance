"""Tests for suite/run.py against a tiny fake implementation (a shell script).

Run: env -u DISPLAY ... TMPDIR=.tmp python3 -I -m unittest discover -s suite/tests
"""
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
RUN = os.path.join(os.path.dirname(HERE), "run.py")

FAKE = r"""#!/bin/sh
case "$1" in
  hash|check)
    if [ "$1" = check ] && grep -q BAD "$2"; then
      echo "bad object: rule §2 violated" >&2; exit 1
    fi
    echo '{"object_hash":"aa","status":"ok","symbols":{"f":{"closure_hash":"bb","interface_hash":"cc"}},"unresolved_imports":[]}' ;;
  run-oracle) echo '{"payload":9,"code":0,"steps":3,"flag":true}' ;;
  fail) echo "boom §1" >&2; exit 1 ;;
  echo) shift; printf '%s\n' "$@" ;;
  env) echo "DISPLAY=${DISPLAY-unset}" ;;
  *) echo "unknown $1" >&2; exit 2 ;;
esac
"""

GOOD_HASHES = {"object": "aa", "symbols": {"f": {"interface": "cc", "closure": "bb"}}}


def base(**kw):
    e = {"spec": "isa-0.8", "source": "fake@0", "clauses": ["ISA-4.5"]}
    e.update(kw)
    return e


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="suite-test-", dir=os.environ.get("TMPDIR"))
        self.addCleanup(shutil.rmtree, self.tmp)
        self.binary = os.path.join(self.tmp, "fake")
        with open(self.binary, "w") as f:
            f.write(FAKE)
        os.chmod(self.binary, os.stat(self.binary).st_mode | stat.S_IXUSR)
        self.cases = os.path.join(self.tmp, "cases")
        os.mkdir(self.cases)

    def add(self, name, expect, obj="object text\n"):
        d = os.path.join(self.cases, name)
        os.mkdir(d)
        with open(os.path.join(d, "object.bisa"), "w") as f:
            f.write(obj)
        if expect is not None:
            with open(os.path.join(d, "expect.json"), "w") as f:
                f.write(expect if isinstance(expect, str) else json.dumps(expect))
        return d

    def run_suite(self, *extra, binary=None):
        env = dict(os.environ, DISPLAY=":0")
        p = subprocess.run(
            [sys.executable, "-I", RUN, "--binary", binary or self.binary,
             "--cases", self.cases, *extra],
            capture_output=True, text=True, env=env)
        return p.returncode, p.stdout, p.stderr

    def run_oracle(self, **over):
        r = {"cmd": ["run-oracle", "$OBJECT", "x"], "stdout_json":
             {"payload": 9, "code": 0, "steps": 3, "flag": True}, "exit": 0}
        r.update(over)
        return r

    def test_passing_case_prints_table_and_exits_zero(self):
        self.add("a", base(hashes=GOOD_HASHES, check="ok", runs=[self.run_oracle()]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 0, out)
        self.assertIn("case", out.splitlines()[0])
        self.assertIn("hashes", out.splitlines()[0])
        row = out.splitlines()[1]
        for cell in ("a", "ok", "1/1", "PASS"):
            self.assertIn(cell, row)
        self.assertIn("checked 1/1", out)

    def test_wrong_stdout_json_fails(self):
        self.add("a", base(runs=[self.run_oracle(stdout_json={"payload": 8, "code": 0, "steps": 3, "flag": True})]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("FAIL", out)
        self.assertIn("0/1", out)

    def test_json_true_is_not_one(self):
        self.add("a", base(runs=[self.run_oracle(stdout_json={"payload": 9, "code": 0, "steps": 3, "flag": 1})]))
        self.assertEqual(self.run_suite()[0], 1)

    def test_json_key_order_and_whitespace_are_irrelevant(self):
        self.add("a", base(runs=[self.run_oracle(stdout_json={"flag": True, "steps": 3, "code": 0, "payload": 9})]))
        self.assertEqual(self.run_suite()[0], 0)

    def test_wrong_object_hash_fails(self):
        h = json.loads(json.dumps(GOOD_HASHES))
        h["object"] = "zz"
        self.add("a", base(hashes=h))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("object hash", out)

    def test_wrong_symbol_hash_and_extra_symbol_fail(self):
        h = json.loads(json.dumps(GOOD_HASHES))
        h["symbols"]["f"]["closure"] = "zz"
        self.add("a", base(hashes=h))
        h2 = json.loads(json.dumps(GOOD_HASHES))
        h2["symbols"]["g"] = {"interface": "1", "closure": "2"}
        self.add("b", base(hashes=h2))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("checked 0/2", out)

    def test_expected_check_failure_with_stderr_substring(self):
        self.add("bad", base(check={"exit": 1, "stderr_contains": "§2"}), obj="BAD\n")
        self.assertEqual(self.run_suite()[0], 0)
        self.add("bad2", base(check={"exit": 1, "stderr_contains": "§9"}), obj="BAD\n")
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)

    def test_check_ok_on_a_bad_object_fails(self):
        self.add("a", base(check="ok"), obj="BAD\n")
        self.assertEqual(self.run_suite()[0], 1)

    def test_expected_nonzero_exit_needs_matching_stderr(self):
        self.add("a", base(runs=[{"cmd": ["fail"], "stdout": "", "exit": 1, "stderr_contains": "§1"}]))
        self.assertEqual(self.run_suite()[0], 0)
        self.add("b", base(runs=[{"cmd": ["fail"], "stdout": "", "exit": 1, "stderr_contains": "nope"}]))
        self.assertEqual(self.run_suite()[0], 1)

    def test_unexpected_exit_code_fails(self):
        self.add("a", base(runs=[{"cmd": ["fail"], "stdout": "", "exit": 0}]))
        self.assertEqual(self.run_suite()[0], 1)

    def test_nonzero_exit_without_stderr_expectation_is_malformed(self):
        self.add("a", base(runs=[{"cmd": ["fail"], "stdout": "", "exit": 1}]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("stderr_contains", out)

    def test_exact_stdout_with_object_and_case_substitution(self):
        d = self.add("a", base(runs=[{"cmd": ["echo", "$OBJECT", "$CASE"], "stdout": "OBJ\nCASE\n", "exit": 0}]))
        exp = os.path.join(d, "expect.json")
        with open(exp) as f:
            e = json.load(f)
        e["runs"][0]["stdout"] = "%s\n%s\n" % (os.path.join(d, "object.bisa"), d)
        with open(exp, "w") as f:
            json.dump(e, f)
        self.assertEqual(self.run_suite()[0], 0)

    def test_exact_stdout_is_byte_exact(self):
        self.add("a", base(runs=[{"cmd": ["echo", "x"], "stdout": "x", "exit": 0}]))
        self.assertEqual(self.run_suite()[0], 1)

    def test_zero_expectations_is_a_failure_not_a_skip(self):
        self.add("a", base())
        self.add("b", base(runs=[]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("zero expectations", out)
        self.assertIn("checked 0/2", out)

    def test_missing_expect_json_and_malformed_json_fail(self):
        self.add("a", None)
        self.add("b", "{not json")
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("checked 0/2", out)

    def test_missing_object_fails(self):
        d = self.add("a", base(check="ok"))
        os.remove(os.path.join(d, "object.bisa"))
        self.assertEqual(self.run_suite()[0], 1)

    def test_unknown_keys_and_missing_clauses_are_rejected(self):
        self.add("a", base(check="ok", extra=1))
        e = base(check="ok")
        del e["clauses"]
        self.add("b", e)
        self.add("c", base(check="ok", clauses=[]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("checked 0/3", out)

    def test_one_failing_case_fails_the_suite_and_is_counted(self):
        self.add("a", base(check="ok"))
        self.add("b", base(runs=[{"cmd": ["fail"], "stdout": "", "exit": 0}]))
        code, out, _ = self.run_suite()
        self.assertEqual(code, 1)
        self.assertIn("checked 1/2", out)

    def test_tags_json(self):
        self.add("a", base(check="ok", clauses=["ISA-4.5", "ISA-5.4"]))
        self.add("b", base(check="ok", clauses=["ISA-14.1"]))
        code, out, _ = self.run_suite("--tags-json")
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out), {"a": ["ISA-4.5", "ISA-5.4"], "b": ["ISA-14.1"]})

    def test_tags_json_needs_no_binary_but_rejects_bad_cases(self):
        self.add("a", base(check="ok", clauses=["nonsense"]))
        code, _, err = self.run_suite("--tags-json")
        self.assertNotEqual(code, 0)
        self.assertIn("clauses", err)

    def test_missing_binary_and_empty_cases_dir_are_errors(self):
        self.add("a", base(check="ok"))
        self.assertNotEqual(self.run_suite(binary=os.path.join(self.tmp, "nope"))[0], 0)
        shutil.rmtree(os.path.join(self.cases, "a"))
        self.assertNotEqual(self.run_suite()[0], 0)

    def test_runner_strips_display_variables_from_the_binary_environment(self):
        self.add("a", base(runs=[{"cmd": ["env"], "stdout": "DISPLAY=unset\n", "exit": 0}]))
        self.assertEqual(self.run_suite()[0], 0)


if __name__ == "__main__":
    unittest.main()
