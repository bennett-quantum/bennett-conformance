"""Tests for the statement layer of clauses/clausegraph.py (stdlib unittest; python3 -I)."""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
CLAUSES = os.path.dirname(HERE)
REPO = os.path.dirname(CLAUSES)
WORKSPACE = os.path.dirname(REPO)
sys.path.insert(0, CLAUSES)

import clausegraph as cg  # noqa: E402


def sents(p):
    return [cg.norm_ws(p[a:b]) for a, b in cg.split_sentences(p)]


def build(text, spec="ISA", version="0.8", xspec=None):
    return cg.build_graph(text, spec=spec, version=version, filename="t.md", xspec=xspec)


def by_id(st):
    return {s["id"]: s for s in st["statements"]}


class TestSplitter(unittest.TestCase):
    def test_plain(self):
        self.assertEqual(sents("A is x. B is y! C is z?"), ["A is x.", "B is y!", "C is z?"])

    def test_abbreviations(self):
        self.assertEqual(sents("Use a tag, e.g. Foo or i.e. Bar. Next one."),
                         ["Use a tag, e.g. Foo or i.e. Bar.", "Next one."])
        self.assertEqual(sents("Compare cf. Smith here. Then vs. Jones."),
                         ["Compare cf. Smith here.", "Then vs. Jones."])

    def test_section_refs_and_decimals(self):
        self.assertEqual(sents("See §4.5. The rule in §4.5.2 holds. Version 0.8 is 1.5x. "
                               "Ends at 2."),
                         ["See §4.5.", "The rule in §4.5.2 holds.", "Version 0.8 is 1.5x.",
                          "Ends at 2."])
        self.assertEqual(sents("Ref §3. §4 is next."), ["Ref §3.", "§4 is next."])

    def test_lowercase_continuation_is_not_a_boundary(self):
        self.assertEqual(sents("It ends. then continues."), ["It ends. then continues."])

    def test_code_spans_and_parens(self):
        self.assertEqual(sents("Write `a. B` here. And `x`. `y` is code."),
                         ["Write `a. B` here.", "And `x`.", "`y` is code."])
        self.assertEqual(sents("A (see X. Then Y.) is one. B is two."),
                         ["A (see X. Then Y.) is one.", "B is two."])
        self.assertEqual(sents("Use ``a `. B`` now. Next."), ["Use ``a `. B`` now.", "Next."])

    def test_caption_joins_next(self):
        self.assertEqual(sents("**Requirement.** A region MUST be x. More."),
                         ["**Requirement.** A region MUST be x.", "More."])
        self.assertEqual(sents("**Hardware handling** (§3.3). The rest is y."),
                         ["**Hardware handling** (§3.3). The rest is y."])

    def test_deps_marker_closes_sentence(self):
        self.assertEqual(sents("A is x. [deps: ISA-1-P1] B is y [deps: ISA-2]. C."),
                         ["A is x. [deps: ISA-1-P1]", "B is y [deps: ISA-2].", "C."])
        self.assertEqual(sents("| K | A is x. [deps: ISA-1] | ticket |"),
                         ["| K | A is x. [deps: ISA-1]", "| ticket |"])

    def test_non_normative_marker_starts_a_sentence(self):
        self.assertEqual(sents("Every field is clean. *(non-normative)* The fixup is x."),
                         ["Every field is clean.", "*(non-normative)* The fixup is x."])
        self.assertEqual(sents("It holds. (deferred) later."), ["It holds.", "(deferred) later."])

    def test_ellipsis_and_closers(self):
        self.assertEqual(sents("Lists a, b, ... and c. *Italic end.* Next."),
                         ["Lists a, b, ... and c.", "*Italic end.*", "Next."])

    def test_enumeration(self):
        t = "The steps are (1) read x, (2) write y, and (3) stop."
        items = [t[a:b] for a, b in cg.split_enumeration(t)]
        self.assertEqual(items, ["(1) read x, ", "(2) write y, and ", "(3) stop."])
        t = "Either (a) one or (b) two; (i) and (ii) too."
        self.assertEqual(len(cg.split_enumeration(t)), 2)

    def test_enumeration_references_are_not_enumerators(self):
        self.assertEqual(cg.split_enumeration("Per §9.2 (1) and §9.2 (2) it holds."), [])
        self.assertEqual(cg.split_enumeration("Replaced in part (1), listed in part (2)."), [])
        self.assertEqual(cg.split_enumeration("Only (1) here and `(2)` in code."), [])
        self.assertEqual(cg.split_enumeration("Starts at (2) then (3)."), [])


class TestSplitterBoldEnd(unittest.TestCase):
    def test_sentence_ending_inside_bold(self):
        # STATEMENT-NOTES ISA-3.3-P2.S2: `.**` + space + capital/bold opener ends a sentence
        self.assertEqual(
            sents("**Regions are threaded the same way.** **Indexed operand** `w[i]` (literal "
                  "i) borrows wire i for one instruction."),
            ["**Regions are threaded the same way.**",
             "**Indexed operand** `w[i]` (literal i) borrows wire i for one instruction."])
        self.assertEqual(sents("**A rule is here.** Then more."),
                         ["**A rule is here.**", "Then more."])
        # a caption (no finite verb in the bold span) still joins the next sentence
        self.assertEqual(sents("**Reference body and binding.** Every oracle carries it."),
                         ["**Reference body and binding.** Every oracle carries it."])
        self.assertEqual(sents("**Width rules** (a mismatch is ill-formed). Every x is y."),
                         ["**Width rules** (a mismatch is ill-formed). Every x is y."])


class TestClassifierFixes(unittest.TestCase):
    def k(self, text, **kw):
        return cg.classify_statement(text, **kw)[:2]

    def test_k5_only_when_term_is_introduced(self):
        d = ("definition", "K5")
        self.assertEqual(self.k("**Interface regions** are caller-owned.",
                                defines=["Interface regions"]), d)
        self.assertEqual(self.k("**Budget**: classical (B, h).", defines=["Budget"]), d)
        self.assertEqual(self.k("**Program-point id** = `(a, b)`.",
                                defines=["Program-point id"]), d)
        self.assertEqual(self.k("Its size is the **effective control count**.",
                                defines=["effective control count"]), d)
        self.assertEqual(self.k("The **Choi matrix** is J(x).", defines=["Choi matrix"]), d)
        self.assertEqual(self.k("**Adjoint table** (reverse order, renumbered):",
                                defines=["Adjoint table"]), d)
        self.assertEqual(self.k("**Interval arithmetic** at `p` means every op rounds out.",
                                defines=["Interval arithmetic"]), d)
        self.assertEqual(self.k("**Tier.** CC, R, Q.", defines=["Tier"]), d)
        self.assertEqual(self.k("**C1 capture**: `d ^= a` for i = 1..m;", defines=["C1"],
                                labels=["C1"]), d)

    def test_k13_caption_or_inline_term_with_a_rule(self):
        r = ("requirement", "K13")
        # STATEMENT-NOTES "K5 overriding a requirement"
        self.assertEqual(self.k("**`in` wires in actions** appear only as controls.",
                                defines=["`in` wires in actions"]), r)
        self.assertEqual(self.k("**Rule (suite F).** Borrowed wires are touched only inside "
                                "**borrow frames**: maximal runs.",
                                defines=["borrow frames"]), r)
        self.assertEqual(self.k("**T3 (permutation theorem).** If V is a permutation, the "
                                "tuple is P(x).", defines=["T3"], labels=["T3"]), r)
        self.assertEqual(self.k("**T5.** As §6.3.", defines=["T5"], labels=["T5"]), r)
        # a verbless id-table row names what its label stands for (definition, K5)
        self.assertEqual(self.k("DP | Platform | the platform profile (§20)", row=True,
                                defines=["DP"], labels=["DP"]), ("definition", "K5"))
        self.assertEqual(self.k("**Data symbol**: parts (5)–(6) are empty.",
                                defines=["Data symbol"]), r)
        self.assertEqual(self.k("Every `qalloc` is matched by **explicit cleanup**; the "
                                "schedule chooses only realisations.",
                                defines=["explicit cleanup"]), r)

    def test_k14_equation_and_k15_colon_label(self):
        self.assertEqual(self.k("Budget = (B, heap_limit), classical."), ("definition", "K14"))
        self.assertEqual(self.k("`params_hash` = SHA-256 of the bytes."), ("definition", "K14"))
        self.assertEqual(self.k("`NAME := [A-Z]+`; `INT := -?[0-9]+`."), ("definition", "K14"))
        self.assertEqual(self.k("`adj f …` ≡ `f …`."), ("definition", "K14"))
        self.assertEqual(self.k("Forms: `x a`, `cx c t`; all controlled."),
                         ("requirement", "K15"))
        self.assertEqual(self.k("**Body**: gates; `qcopy`; `qalloc`."), ("requirement", "K15"))
        self.assertEqual(self.k("Frames:"), ("requirement", "K15"))

    def test_k16_inherits_and_k12_fence_kind(self):
        self.assertEqual(self.k("No fault outcome.", prev_kind="requirement"),
                         ("requirement", "K16"))
        self.assertEqual(self.k("No fault outcome.", prev_kind="example"), ("uncertain", "K10"))
        self.assertEqual(self.k("```\nx = y\n```", code=True, code_example=False),
                         ("requirement", "K12"))
        self.assertEqual(self.k("```\nx = y\n```", code=True, code_example=False,
                                intro_kind="definition"), ("definition", "K12"))


FENCES = """# T

## 1. Gates

```
x = [[0,1],[1,0]]
```

## 2. Tables

**Adjoint table** (reverse order):

```
s <-> sdg
```

Metadata sections are one line each, an object:

```
symbols: {}
```

For example, a call:

```
call @f
```

The checked witness:

```text state=x
{"a":1}
```

## 3. Worked examples

The object:

```
bennett-object
```

## 4. Notes

*(non-normative)* A sketch:

```
sketch
```

## 5. Example objects (normative test vectors)

**Example 1** (`inc8`): one oracle. The function has six lines.

```
bennett-object
```
"""


class TestFenceRule(unittest.TestCase):
    # STATEMENT-NOTES "K1: every fenced code block is example"
    def test_k12(self):
        st = by_id(cg.build_statements(build(FENCES)))
        got = {k: (v["kind"], v["rule"]) for k, v in st.items() if v["text"].startswith("```")}
        self.assertEqual(got, {
            "ISA-1-P1.S1": ("requirement", "K12"),
            "ISA-2-P1.S2": ("definition", "K12"),
            "ISA-2-P2.S2": ("requirement", "K12"),
            "ISA-2-P3.S2": ("example", "K1"),
            "ISA-2-P4.S2": ("example", "K1"),
            "ISA-3-P1.S2": ("example", "K1"),
            "ISA-4-P1.S2": ("example", "K1"),
            "ISA-5-P1.S3": ("requirement", "K12")})  # normative vectors (ISA §16.3)


class TestClassifier(unittest.TestCase):
    def k(self, text, **kw):
        return cg.classify_statement(text, **kw)[:2]

    def test_rules(self):
        self.assertEqual(self.k("```\nx\n```", code=True), ("example", "K1"))
        self.assertEqual(self.k("A MUST be x.", nn_section=True), ("non-normative", "K2"))
        self.assertEqual(self.k("*(non-normative)* This explains.", nn_passage=True),
                         ("non-normative", "K3"))
        self.assertEqual(self.k("A region MUST NOT contain `br`."), ("requirement", "K4"))
        self.assertEqual(self.k("**Lproc**: a lifted procedure.", defines=["Lproc"]),
                         ("definition", "K5"))
        self.assertEqual(self.k("e.g. a loop over rows."), ("example", "K6"))
        self.assertEqual(self.k("Rows of the table.", example_section=True), ("example", "K6"))
        self.assertEqual(self.k("A producer must check x."), ("requirement", "K7"))
        self.assertEqual(self.k("The VM rejects the program."), ("requirement", "K8"))
        self.assertEqual(self.k("`load` | `r = load` | bytes", row=True), ("requirement", "K9"))

    def test_uncertain(self):
        kind, rule, _ = cg.classify_statement("Predicate stack empty at entry and exit.")
        self.assertEqual((kind, rule), ("uncertain", "K10"))
        kind, rule, modal = cg.classify_statement("*(non-normative)* X MUST hold.",
                                                  nn_passage=True)
        self.assertEqual((kind, rule, modal), ("uncertain", "K3", ["MUST"]))

    def test_precedence_and_code_spans(self):
        # RFC keyword beats a definition; keywords inside code spans do not count
        self.assertEqual(self.k("**Frames** MUST be clean.", defines=["Frames"]),
                         ("requirement", "K4"))
        self.assertEqual(self.k("Flag `MUST` set."), ("uncertain", "K10"))


SPEC = """# T

## 1. Terminology

- **lproc**: a lifted procedure. It resolves names locally.
- **region**: a byte range.

## 2. Rules

**Requirement.** An lproc MUST NOT touch a region. Stack empty at exit.

The steps are (1) read the region, (2) write it, and (3) stop (§1).

| Id | Rule |
|---|---|
| PT3 | casts keep witnesses |

Every lproc is checked. [deps: ISA-1-P2.S1, ISA-2-P1, PT3, ISA-9-P9, DLT-4] Plain sentence uses an lproc.

### 2.1 Notes

*(non-normative)* This explains a region. It MUST still hold.

Closing remark is fine. [deps: none]
"""


class TestStatementLayer(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.g = build(SPEC)
        cls.st = cg.build_statements(cls.g)
        cls.s = by_id(cls.st)

    def test_ids_numbering_and_kinds(self):
        s = self.s
        self.assertEqual(s["ISA-1-P1.S1"]["kind"], "definition")
        self.assertEqual(s["ISA-1-P1.S1"]["defines"], ["lproc"])
        self.assertEqual(s["ISA-1-P1.S2"]["n"], "<1>2")
        self.assertEqual(s["ISA-1-P1.S1"]["text"], "**lproc**: a lifted procedure.")
        self.assertEqual(s["ISA-2-P1.S1"]["kind"], "requirement")
        self.assertTrue(s["ISA-2-P1.S1"]["text"].startswith("**Requirement.** An lproc"))
        # STATEMENT-NOTES "uncertain that should be decided" (ISA-5.1-P1.S4): a verbless
        # fragment after a requirement inherits it (K16); this test used to expect uncertain
        self.assertEqual((s["ISA-2-P1.S2"]["kind"], s["ISA-2-P1.S2"]["rule"]),
                         ("requirement", "K16"))
        self.assertNotIn("uncertain", s["ISA-2-P1.S2"])
        # STATEMENT-NOTES "K5 overriding a requirement" (T3/T5): a labelled rule row is a
        # requirement carrying its label (K13); this test used to expect definition
        self.assertEqual((s["ISA-2-P3.S1"]["kind"], s["ISA-2-P3.S1"]["rule"]),
                         ("requirement", "K13"))
        self.assertEqual(s["ISA-2-P3.S1"]["defines"], ["PT3"])
        self.assertEqual(s["ISA-2.1-P1.S1"]["kind"], "non-normative")
        # STATEMENT-NOTES ISA-6.4-P1.S4: the marker scopes its own sentence; this test used to
        # expect uncertain (K3 propagated to the paragraph end)
        self.assertEqual((s["ISA-2.1-P1.S2"]["kind"], s["ISA-2.1-P1.S2"]["rule"]),
                         ("requirement", "K4"))
        self.assertEqual(len(s["ISA-2-P1.S1"]["hash"]), 12)

    def test_enumerated_sub_statements(self):
        s = self.s
        subs = [x for x in self.st["statements"] if x["parent"] == "ISA-2-P2.S1"]
        self.assertEqual([x["id"] for x in subs], ["ISA-2-P2.S1.1", "ISA-2-P2.S1.2",
                                                   "ISA-2-P2.S1.3"])
        self.assertEqual([x["n"] for x in subs], ["<2>1", "<2>2", "<2>3"])
        self.assertEqual(subs[1]["level"], 2)
        self.assertEqual(s["ISA-2-P2.S1.1"]["deps"]["inferred"][0]["dst"], "ISA-1-P2.S1")

    def test_inferred_deps(self):
        inf = {d["dst"]: d for d in self.s["ISA-2-P1.S1"]["deps"]["inferred"]}
        self.assertEqual(inf["ISA-1-P1.S1"]["via"], "term")
        self.assertEqual(inf["ISA-1-P2.S1"]["name"], "region")
        refs = {d["dst"]: d["via"] for d in self.s["ISA-2-P2.S1"]["deps"]["inferred"]}
        self.assertEqual(refs.get("ISA-1"), "refs")
        # a statement does not depend on its own clause's defining statement for itself
        self.assertNotIn("ISA-1-P1.S1", [d["dst"] for d in
                                         self.s["ISA-1-P1.S1"]["deps"]["inferred"]])

    def test_declared_deps(self):
        x = self.s["ISA-2-P4.S1"]
        self.assertEqual(x["text"], "Every lproc is checked.")  # marker stripped
        self.assertEqual([d["dst"] for d in x["deps"]["declared"]],
                         ["ISA-1-P2.S1", "ISA-2-P1", "ISA-2-P3.S1"])
        self.assertEqual({d["dst"]: d["status"] for d in x["deps"]["inferred"]},
                         {"ISA-1-P1.S1": "superseded"})
        un = {u["ref"]: u["reason"] for u in self.st["unresolved"]}
        self.assertEqual(un, {"ISA-9-P9": "no-such-id", "DLT-4": "xspec-not-loaded"})
        y = self.s["ISA-2-P4.S2"]
        self.assertEqual(y["marker"], None)
        self.assertEqual(y["deps"]["inferred"][0]["status"], "inferred")
        z = self.s["ISA-2.1-P2.S1"]  # [deps: none]: every inferred edge superseded
        self.assertEqual(z["marker"], ["none"])
        self.assertEqual(z["deps"]["declared"], [])

    def test_stats_and_markdown(self):
        t = self.st["stats"]
        self.assertEqual(t["statements"], len(self.st["statements"]))
        self.assertEqual(sum(t["by_kind"].values()), t["statements"])
        self.assertEqual(t["markers"], 2)
        self.assertIn("ISA-2", t["by_section"])
        md = cg.statements_markdown(self.st, self.g)
        self.assertIn("### §2 Rules   [ISA-2]", md)
        self.assertIn("<1>1 [req] **Requirement.** An lproc MUST NOT", md)
        self.assertIn("declared: ISA-1-P2.S1, ISA-2-P1, ISA-2-P3.S1", md)

    def test_deterministic(self):
        self.assertEqual(json.dumps(cg.build_statements(self.g)), json.dumps(self.st))


class TestStatementQueries(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.g = build(SPEC)
        cls.st = cg.build_statements(cls.g)
        cls.m = cg.Merged([cls.g], [cls.st])

    def test_impact_statement_vs_clause(self):
        lv = self.m.impact("ISA-1-P1.S1")
        flat = [x for v in lv for x in v]
        self.assertIn("ISA-2-P1.S1", flat)   # uses "lproc"
        self.assertIn("ISA-2-P2.S1", flat)  # cites §1, which encloses the query
        self.assertNotIn("ISA-2-P2.S1.1", flat)  # "(1) read the region": region only
        lv2 = [x for v in self.m.impact("ISA-1-P2.S1") for x in v]
        self.assertIn("ISA-2-P2.S1.1", lv2)
        # a clause id falls back to the union of its statements
        clause = {x for v in self.m.impact("ISA-1-P1") for x in v}
        self.assertTrue(set(flat) <= clause)

    def test_independent_statements(self):
        ok, fwd, rev = self.m.independent("ISA-1-P1.S1", "ISA-1-P2.S1")
        self.assertFalse(ok)  # ISA-2-P1.S1 uses both terms
        self.assertIn("ISA-2-P1.S1", rev)
        ok, _, rev = self.m.independent("ISA-2-P1.S2", "ISA-2-P2.S1.2")
        self.assertTrue(ok, rev)
        # §1 is cited by ISA-2-P2.S1: strict mode counts it for both §1 statements
        ok, _, rev = self.m.independent("ISA-1-P1.S2", "ISA-1-P2.S1")
        self.assertFalse(ok)
        ok, _, rev = self.m.independent("ISA-1-P1.S2", "ISA-1-P2.S1", containers="ignore")
        self.assertTrue(ok, rev)

    def test_resolve_hint_without_layer(self):
        with self.assertRaises(cg.UsageError) as e:
            cg.Merged([self.g]).resolve("ISA-1-P1.S1")
        self.assertIn("--statements", str(e.exception))

    def test_stale_layer_rejected(self):
        g2 = build(SPEC + "\nMore.\n")
        with self.assertRaises(cg.UsageError):
            cg.Merged([g2], [self.st])

    def test_coverage_statement_granularity(self):
        rep = cg.coverage(self.g, {"t1": "ISA-1-P1.S1", "t2": "ISA-2-P2", "t3": "ISA-2.1",
                                   "t4": "DLT-3-P1.S1"}, self.st)
        sc = rep["statements"]
        self.assertEqual(sc["totals"]["strong"], 1)
        n22 = sum(1 for x in self.st["statements"] if x["clause"] in ("ISA-2-P2",)
                  or x["section"] == "ISA-2.1")
        self.assertEqual(sc["totals"]["weak"], n22)
        self.assertEqual(sc["totals"]["statements"], len(self.st["statements"]))
        self.assertIn("ISA-1-P1", rep["witnesses"])  # a statement tag is a clause witness
        self.assertEqual(rep["foreign"], ["DLT-3-P1.S1"])
        with self.assertRaises(cg.UsageError):
            cg.coverage(self.g, {"t": "ISA-1-P1.S1"})  # statement id without the layer
        with self.assertRaises(cg.UsageError):
            cg.coverage(self.g, {"t": "ISA-1-P1.S9"}, self.st)

    def test_diff_statements(self):
        b_text = SPEC.replace("Stack empty at exit.", "Stack empty at entry and exit.") \
            .replace("Plain sentence uses an lproc.", "Plain sentence uses an lproc. New "
                                                      "sentence is here.")
        gb = build(b_text, version="0.9")
        sb = cg.build_statements(gb)
        d = cg.diff_statements(self.g, gb, self.st, sb)
        s = d["summary"]
        self.assertEqual(d["added"], ["ISA-2-P4.S3"])
        self.assertEqual(d["removed"], [])
        self.assertEqual(d["changed"], [["ISA-2-P1.S2", "ISA-2-P1.S2", "similar"]])
        self.assertEqual(s["unchanged"], len(self.st["statements"]) - 1)


def run_cli(*args):
    env = {k: v for k, v in os.environ.items()
           if k not in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")}
    return subprocess.run([sys.executable, "-I", os.path.join(CLAUSES, "clausegraph.py")]
                          + list(args), capture_output=True, text=True, env=env, timeout=300)


class TestStatementCli(unittest.TestCase):
    def test_statements_command_and_queries(self):
        base = os.environ.get("TMPDIR") or os.path.join(REPO, ".tmp")
        os.makedirs(base, exist_ok=True)
        tmp = tempfile.mkdtemp(dir=base)
        spec = os.path.join(tmp, "s.md")
        with open(spec, "w", encoding="utf-8") as f:
            f.write(SPEC)
        r = run_cli("build", "--spec", "ISA", "--version", "0.8", "--file", spec,
                    "--out-dir", tmp)
        self.assertEqual(r.returncode, 0, r.stderr)
        gj = os.path.join(tmp, "ISA-0.8.json")
        r = run_cli("impact", gj, "ISA-1-P1.S1")
        self.assertEqual(r.returncode, 2)
        self.assertIn("statements", r.stderr)
        r = run_cli("statements", gj)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("unresolved declared dependency 'ISA-9-P9'", r.stderr)
        self.assertTrue(os.path.exists(os.path.join(tmp, "ISA-0.8.statements.md")))
        r = run_cli("impact", gj, "ISA-1-P1.S1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("ISA-2-P1.S1 [req]", r.stdout)
        r = run_cli("independent", gj, "ISA-2-P1.S2", "ISA-2-P2.S1.2")
        self.assertEqual((r.returncode, r.stdout.strip()), (0, "independent"))
        tags = os.path.join(tmp, "tags.json")
        with open(tags, "w") as f:
            json.dump({"t": ["ISA-2-P1"]}, f)
        r = run_cli("coverage", gj, "--tags", tags, "--statements")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("statements of ISA-0.8:", r.stdout)
        r = run_cli("diff", gj, gj, "--statements")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("statements ISA-0.8 -> ISA-0.8:", r.stdout)


COMMITTED = ("ISA-0.8", "ISA-0.9-draft", "DLT-0.2.10")


class TestCommittedStatements(unittest.TestCase):
    """The committed statement layers: fresh, deterministic, sane on the real specs."""

    @classmethod
    def setUpClass(cls):
        cls.g, cls.st = {}, {}
        for name in COMMITTED:
            cls.g[name] = cg.load_graph(os.path.join(CLAUSES, name + ".json"))
            cls.st[name] = cg.load_statements(os.path.join(CLAUSES, name + ".json"))

    def test_regenerates_identically(self):
        for name in COMMITTED:
            g = self.g[name]
            xg = self.g["ISA-0.8"] if g.get("xspec") else None
            again = cg.build_statements(g, xg)
            self.assertEqual(json.dumps(again, indent=1, ensure_ascii=False),
                             json.dumps(self.st[name], indent=1, ensure_ascii=False),
                             "%s statements are stale; rerun `statements`" % name)

    def test_counts_in_range(self):
        for name, lo, hi in (("ISA-0.8", 400, 1500), ("ISA-0.9-draft", 900, 4000),
                             ("DLT-0.2.10", 2000, 9000)):
            t = self.st[name]["stats"]
            self.assertTrue(lo <= t["statements"] <= hi, (name, t["statements"]))
            self.assertGreater(t["statements"], t["clauses"])
            self.assertGreater(t["by_kind"]["requirement"], t["statements"] * 0.4, name)
            self.assertLess(t["by_kind"]["uncertain"], t["statements"] * 0.15, name)

    def test_isa_5_1_p1(self):
        s = [x for x in self.st["ISA-0.8"]["statements"] if x["clause"] == "ISA-5.1-P1"]
        self.assertGreaterEqual(len(s), 4)
        kinds = {x["kind"] for x in s}
        self.assertIn("requirement", kinds)
        self.assertIn("definition", kinds)
        req = by_id(self.st["ISA-0.8"])["ISA-5.1-P1.S3"]
        self.assertTrue(req["text"].startswith("A region MUST NOT contain"), req["text"])
        self.assertEqual(req["modal"], ["MUST NOT"])

    def test_known_definitions_and_deps(self):
        s = by_id(self.st["ISA-0.8"])
        lproc = [x for x in s.values() if "lproc" in x["defines"]]
        self.assertEqual(len(lproc), 1)
        self.assertTrue(lproc[0]["clause"].startswith("ISA-2-"))
        users = [x for x in s.values() if any(d["dst"] == lproc[0]["id"]
                                               for d in x["deps"]["inferred"])]
        self.assertGreaterEqual(len(users), 3)
        d = by_id(self.st["DLT-0.2.10"])
        xs = [x for x in d.values() if any(e["via"] == "xspec" for e in x["deps"]["inferred"])]
        self.assertGreaterEqual(len(xs), 100)

    def test_declared_markers_in_09(self):
        t = self.st["ISA-0.9-draft"]["stats"]
        # the 0.9 editor's [deps: ...] markers (none in 0.8 or the dialect)
        self.assertEqual(self.st["ISA-0.8"]["stats"]["markers"], 0)
        if t["markers"]:
            self.assertGreater(t["declared_edges"], 0)
            self.assertEqual(t["unresolved_markers"], len(self.st["ISA-0.9-draft"]["unresolved"]))


if __name__ == "__main__":
    unittest.main()
