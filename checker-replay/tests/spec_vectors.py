"""Published contract material, read at test time (spec is the source of truth)."""
import json, pathlib, re

WS = pathlib.Path(__file__).resolve().parents[3]
SPEC = WS / "bennett-isa-spec/spec/Bennett-ISA-Spec-0.9.md"
FIXTURES = WS / "bennett-isa-spec/conformance/fixtures"
ARITH = WS / "bennett-arith/adders/objects"


def example_objects():
    """Every `bennett-object` code block of §16.3, in order, keyed by example number."""
    text = SPEC.read_text()
    sec = text[text.index("### 16.3 Example objects"):text.index("## 17. Decisions deferred")]
    out = {}
    for m in re.finditer(r"\*\*Example (\d+)\*\*.*?```\n(bennett-object .*?)```", sec, re.S):
        out[int(m.group(1))] = m.group(2)
    assert len(out) >= 7, f"§16.3 extraction found only {sorted(out)}"
    return out


def k78_k80_vectors():
    d = FIXTURES / "accepted-k78-k80-09"
    for name in ("k78.json", "k79.json", "k80.json"):
        for v in json.loads((d / name).read_text())["vectors"]:
            yield name, v


def arith_objects():
    paths = sorted(p for p in ARITH.parent.parent.rglob("*.bisa")
                   if not {".tmp", "target", ".git"} & set(p.parts) and "section certificate" in p.read_text())
    assert paths, f"no published arith objects at {ARITH}"
    return paths


def embedded_certificate(text):
    lines = text.split("\n")
    return json.loads(lines[lines.index("section certificate") + 1])
