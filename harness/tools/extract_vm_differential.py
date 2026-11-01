#!/usr/bin/env python3
"""Extract literal CLI invocations from bennett-vm's tests/differential.rs into harness/corpora as data.

No code crosses the boundary: only argv lists whose every element is a string literal or fx("<fixture>"),
naming exactly one fixture. Anything else is recorded as a counted skip with its source line.
Usage: extract_vm_differential.py <differential.rs> <source-commit> <out.json>
"""
import json
import re
import sys

STR = re.compile(r'"((?:[^"\\]|\\.)*)"')


def blocks(text):
    for m in re.finditer(r"\bs\(&\[", text):
        depth, i = 1, m.end()
        while depth:
            ch = text[i]
            if ch == '"':
                i += 1
                while text[i] != '"':
                    i += 2 if text[i] == "\\" else 1
            elif ch == "[":
                depth += 1
            elif ch == "]":
                depth -= 1
            i += 1
        yield text.count("\n", 0, m.start()) + 1, text[m.end():i - 1]


def elements(body):
    out, depth, cur, i = [], 0, "", 0
    while i < len(body):
        ch = body[i]
        if ch == '"':
            j = i + 1
            while body[j] != '"':
                j += 2 if body[j] == "\\" else 1
            cur += body[i:j + 1]
            i = j + 1
            continue
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
        i += 1
    if cur.strip():
        out.append(cur.strip())
    return out


def main():
    src, commit, dest = sys.argv[1:]
    with open(src, encoding="utf-8") as f:
        text = f.read()
    invocations, skipped = [], []
    for line, body in blocks(text):
        argv, fixture, why = [], None, None
        for el in elements(body):
            m = re.fullmatch(r'&fx\("([^"]+)"\)', el)
            if m:
                if fixture is not None:
                    why = "more than one fixture in one invocation"
                fixture = m.group(1)
                argv.append("$FIXTURE")
            elif STR.fullmatch(el):
                argv.append(bytes(STR.fullmatch(el).group(1), "utf-8").decode("unicode_escape"))
            else:
                why = "non-literal argv element (computed in the test): %s" % el.split("(")[0][:40]
                break
        if fixture is None and why is None:
            why = "no fixture argument (scratch or helper list)"
        if why:
            skipped.append({"line": line, "reason": why})
        else:
            invocations.append({"line": line, "fixture": fixture, "argv": argv})
    with open(dest, "w", encoding="utf-8") as f:
        json.dump({"source": "bennett-vm tests/differential.rs @ " + commit, "invocations": invocations,
                   "skipped": skipped}, f, indent=0, sort_keys=True)
        f.write("\n")
    print("%d invocations, %d skipped" % (len(invocations), len(skipped)))


if __name__ == "__main__":
    main()
