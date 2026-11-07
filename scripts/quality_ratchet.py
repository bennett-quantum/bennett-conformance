#!/usr/bin/env python3
"""Repository-local structural ratchet; no external thresholds or coverage gate."""
import argparse
import ast
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BEGIN = "<!-- quality-baseline -->"
END = "<!-- /quality-baseline -->"
PROCESS_ID = re.compile(r"\bbennett-conformance-[a-z0-9]{3}\b|\b(?:job|wip)/[a-z0-9_-]+|\bshift [0-9]+")
RUST_TOKEN = re.compile(r'//[^\n]*|/\*[\s\S]*?\*/|(?:br|r)(?P<h>\#*)"[\s\S]*?"(?P=h)|b?"(?:\\.|[^"\\])*"|b?\'(?:\\.|[^\'\\\n])\'')
CONTROLS = (ast.If, ast.For, ast.AsyncFor, ast.While, ast.ExceptHandler,
            ast.IfExp, ast.comprehension, ast.match_case)


def mask_rust(text):
    parts, index = [], 0
    while index < len(text):
        if text.startswith("/*", index):
            stop, depth = index + 2, 1
            while depth and stop < len(text):
                if text.startswith("/*", stop):
                    depth += 1
                    stop += 2
                elif text.startswith("*/", stop):
                    depth -= 1
                    stop += 2
                else:
                    stop += 1
            if depth:
                raise ValueError("unterminated Rust block comment")
            parts.append(re.sub(r"[^\n]", " ", text[index:stop]))
            index = stop
            continue
        match = RUST_TOKEN.match(text, index)
        if match:
            parts.append(re.sub(r"[^\n]", " ", match.group()))
            index = match.end()
        else:
            parts.append(text[index])
            index += 1
    return "".join(parts)


def rust_functions(text):
    clean = mask_rust(text)
    for match in re.finditer(r"\bfn\s+(\w+)", clean):
        start = clean.find("{", match.end())
        if start < 0:
            continue
        brackets = 0
        abstract = False
        for char in clean[match.end():start]:
            brackets += (char == "[") - (char == "]")
            abstract |= char == ";" and brackets == 0
        if abstract:
            continue
        depth, end = 1, start + 1
        while depth and end < len(clean):
            depth += (clean[end] == "{") - (clean[end] == "}")
            end += 1
        if depth:
            raise ValueError("unbalanced Rust function " + match.group(1))
        yield match.group(1), clean.count("\n", 0, match.start()) + 1, clean.count("\n", match.start(), end) + 1, clean[start + 1:end - 1]


def rust_complexity(body):
    stack, pending, mass = [], False, 0
    for token in re.findall(r"\b(?:if|for|while|loop|match)\b|&&|\|\||[{}]", body):
        if token == "{":
            stack.append(pending)
            pending = False
        elif token == "}":
            stack.pop()
        elif token in ("&&", "||"):
            mass += 1
        else:
            mass += 1 + sum(stack)
            pending = True
    return mass


def python_complexity(node, depth=0):
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
        return 0  # Nested functions are measured independently.
    control = isinstance(node, CONTROLS)
    own = (1 + depth) if control else 0
    if isinstance(node, ast.BoolOp):
        own += len(node.values) - 1
    return own + sum(python_complexity(child, depth + control)
                     for child in ast.iter_child_nodes(node))


def file_metrics(path, text):
    lengths, complexity, wrappers = [], 0, 0
    if path.suffix == ".rs":
        for _, _, length, body in rust_functions(text):
            lengths.append(length)
            complexity += rust_complexity(body)
            wrappers += bool(re.fullmatch(r"\s*(?:return\s+)?[\w:.]+!?\([^{};]*\)\??;?\s*", body))
    else:
        tree = ast.parse(text, filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                lengths.append(node.end_lineno - node.lineno + 1)
                complexity += sum(python_complexity(child) for child in node.body)
                wrappers += len(node.body) == 1 and isinstance(node.body[0], ast.Return) and isinstance(node.body[0].value, ast.Call)
    return {
        "functions": len(lengths),
        "length_le_50": sum(n <= 50 for n in lengths),
        "length_51_100": sum(50 < n <= 100 for n in lengths),
        "length_101_300": sum(100 < n <= 300 for n in lengths),
        "length_gt_300": sum(n > 300 for n in lengths),
        "over_50": sum(n > 50 for n in lengths),
        "over_100": sum(n > 100 for n in lengths),
        "max_lines": max(lengths, default=0),
        "cognitive_mass": complexity,
        "trivial_wrappers": wrappers,
        "process_identifiers": len(PROCESS_ID.findall(text)),
    }


def is_litter(name):
    path = Path(name)
    return ("__pycache__" in path.parts or path.suffix in (".pyc", ".log", ".darc")
            or path.name == "LOCK" or ".gate.lock" in path.name
            or path.name.startswith(("PROGRESS", "LAST-RUN")))


def measure():
    names = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True).splitlines()
    files = {}
    for name in names:
        path = Path(name)
        rust = name.startswith("checker/src/") and path.suffix == ".rs"
        python = (path.suffix == ".py" and "tests" not in path.parts
                  and not path.name.startswith(("test_", "check_")))
        if rust or python:
            files[name] = file_metrics(path, (ROOT / path).read_text())
    return {"files": files, "litter": sorted(name for name in names if is_litter(name))}


def regressions(baseline, current):
    errors = ["new tracked litter: " + name for name in
              sorted(set(current["litter"]) - set(baseline["litter"]))]
    guarded = ("over_50", "over_100", "length_gt_300", "max_lines",
               "cognitive_mass", "trivial_wrappers", "process_identifiers")
    for name, metrics in current["files"].items():
        previous = baseline["files"].get(name, {})
        for key in guarded:
            if metrics[key] > previous.get(key, 0):
                errors.append(f"{name}: {key} {previous.get(key, 0)} -> {metrics[key]}")
    return errors


def self_test():
    sample = 'fn array(x: &[u8; 4]) { if x[0] > 0 { call(); } }'
    metrics = file_metrics(Path("sample.rs"), sample)
    assert metrics["functions"] == 1 and metrics["cognitive_mass"] == 1
    raw = 'fn f() { let x = r###"{braces}"###; /* } */ call(); }'
    assert len(list(rust_functions(raw))) == 1
    nested = "fn f() { /* outer /* inner */ } ignored */ call(); }"
    assert len(list(rust_functions(nested))) == 1
    long = "fn f() {\n" + " call();\n" * 50 + "}"
    assert file_metrics(Path("long.rs"), long)["over_50"] == 1
    before = {"files": {"sample.rs": metrics}, "litter": []}
    for key in ("over_50", "over_100", "length_gt_300", "max_lines",
                "cognitive_mass", "trivial_wrappers", "process_identifiers"):
        changed = json.loads(json.dumps(before))
        changed["files"]["sample.rs"][key] += 1
        assert regressions(before, changed), key
    assert regressions(before, {"files": before["files"], "litter": ["probe.log"]})
    assert not regressions(before, before)
    py = file_metrics(Path("sample.py"), "def f(x):\n if x:\n  for y in x:\n   call(y)\n")
    assert py["cognitive_mass"] == 3
    print("quality ratchet self-tests passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    current = measure()
    doc = ROOT / "docs/quality.md"
    text = doc.read_text()
    if args.record:
        fields = list(next(iter(current["files"].values())))
        rows = [json.dumps(name) + ": " + json.dumps([metrics[k] for k in fields])
                for name, metrics in sorted(current["files"].items())]
        encoded = ('{"fields": ' + json.dumps(fields) + ',\n"files": {\n'
                   + ",\n".join(rows) + '\n},\n"litter": '
                   + json.dumps(current["litter"], indent=2) + "\n}")
        payload = BEGIN + "\n```json\n" + encoded + "\n```\n" + END
        if BEGIN in text:
            start, stop = text.index(BEGIN), text.index(END) + len(END)
            text = text[:start] + payload + text[stop:]
        else:
            text += "\n" + payload + "\n"
        doc.write_text(text)
        print("recorded quality baseline in docs/quality.md")
        return
    if BEGIN not in text or END not in text:
        raise SystemExit("missing quality baseline in docs/quality.md; review and record it")
    recorded = text.split(BEGIN, 1)[1].split(END, 1)[0].strip()
    baseline = json.loads(recorded.removeprefix("```json\n").removesuffix("\n```"))
    baseline["files"] = {name: dict(zip(baseline["fields"], row))
                         for name, row in baseline["files"].items()}
    errors = regressions(baseline, current)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"quality ratchet passed: {len(current['files'])} files; {len(current['litter'])} inherited litter paths")


if __name__ == "__main__":
    main()
