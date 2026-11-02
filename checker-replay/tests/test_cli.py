import json, os, pathlib, subprocess, sys, unittest
from tests.helpers import FIX
from tests.spec_vectors import arith_objects

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(*a):
    env = {k: v for k, v in os.environ.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")}
    return subprocess.run([sys.executable, "-m", "checker_replay", *map(str, a)], cwd=ROOT, env=env, capture_output=True, text=True, timeout=120)


class Cli(unittest.TestCase):
    def test_fixture_is_unchecked_status_2_with_one_json_line(self):
        r = run(FIX / "certificate.json", "--object", FIX / "controlled.bisa", "--target", FIX / "target.json", "--schedule", FIX / "schedule.json")
        self.assertEqual(r.returncode, 2, r.stderr)
        self.assertEqual(r.stdout.count("\n"), 1)
        v = json.loads(r.stdout)
        self.assertEqual(v["summary"], {"proof": 1, "evidence": 0, "reject": 0, "unchecked": 2})

    def test_deterministic(self):
        a = run("--from-object", FIX / "controlled.bisa").stdout
        self.assertEqual(a, run("--from-object", FIX / "controlled.bisa").stdout)

    def test_from_object_exit_0_only_when_all_claims_replayed(self):
        w1 = [p for p in arith_objects() if p.name == "arith.add.cuccaro.w1.bisa"][0]
        r = run("--from-object", w1)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(json.loads(r.stdout)["overall"], "replayed")

    def test_usage_and_input_errors_are_3(self):
        self.assertEqual(run().returncode, 3)
        self.assertEqual(run("--from-object", FIX / "nope.bisa").returncode, 3)
