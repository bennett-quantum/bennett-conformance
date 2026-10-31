"""Tests for clauses/clausegraph.py (stdlib unittest; run with python3 -I)."""
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
CLAUSES = os.path.dirname(HERE)
REPO = os.path.dirname(CLAUSES)
WORKSPACE = os.path.dirname(REPO)
sys.path.insert(0, CLAUSES)

import clausegraph as cg  # noqa: E402


def build(text, spec="ISA", version="0.8", xspec=None):
    return cg.build_graph(text, spec=spec, version=version, filename="t.md", xspec=xspec)


def node(g, nid):
    for n in g["nodes"]:
        if n["id"] == nid:
            return n
    raise AssertionError("no node %s; have %s" % (nid, [n["id"] for n in g["nodes"]]))


def ids(g, kind="clause"):
    return [n["id"] for n in g["nodes"] if n["kind"] == kind]


def edges(g, kind):
    return {(e["src"], e["dst"]) for e in g["edges"] if e["kind"] == kind}


BASIC = """# Title

Front matter paragraph.

## 1. Scope

Intro to one,
wrapped.

### 1.1 What

First para.

- item one
  continued
- item two

1. numbered one;
2. numbered two.

```
code attached to numbered two
```

## 2. Terminology

- **Tier.** CC, R, Q. **Wire**, **bundle** `wire[N]`.

---

## Appendix A. Extra

Appendix text.
"""


class TestStructure(unittest.TestCase):
    def test_sections_and_root(self):
        g = build(BASIC)
        self.assertEqual(ids(g, "root"), ["ISA"])
        self.assertEqual(ids(g, "section"), ["ISA-front", "ISA-1", "ISA-1.1", "ISA-2", "ISA-A"])
        self.assertEqual(node(g, "ISA-1.1")["parent"], "ISA-1")
        self.assertEqual(node(g, "ISA-1.1")["title"], "What")
        self.assertEqual(node(g, "ISA-1")["line"], 5)

    def test_clause_segmentation(self):
        g = build(BASIC)
        self.assertEqual(
            ids(g),
            ["ISA-front-P1", "ISA-1-P1", "ISA-1.1-P1", "ISA-1.1-P2", "ISA-1.1-P3",
             "ISA-1.1-P4", "ISA-1.1-P5", "ISA-2-P1", "ISA-A-P1"])
        self.assertEqual(node(g, "ISA-1.1-P2")["text"], "- item one\n  continued")
        self.assertEqual(node(g, "ISA-1.1-P2")["clause_kind"], "item")
        self.assertEqual(node(g, "ISA-1-P1")["clause_kind"], "para")
        self.assertEqual(node(g, "ISA-1-P1")["line"], 7)
        self.assertEqual(node(g, "ISA-1-P1")["end_line"], 8)

    def test_code_attaches_to_preceding_clause(self):
        g = build(BASIC)
        c = node(g, "ISA-1.1-P5")
        self.assertIn("code attached to numbered two", c["text"])
        self.assertEqual(c["end_line"], 23)

    def test_code_without_preceding_clause_is_own_clause(self):
        g = build("## 1. A\n\n```\nx\n```\n\ntext\n")
        self.assertEqual(node(g, "ISA-1-P1")["clause_kind"], "code")
        self.assertEqual(ids(g), ["ISA-1-P1", "ISA-1-P2"])

    def test_hash_whitespace_normalised(self):
        a = build("## 1. A\n\nsome  text\nhere\n")
        b = build("## 1. A\n\nsome text here\n")
        ha, hb = node(a, "ISA-1-P1")["hash"], node(b, "ISA-1-P1")["hash"]
        self.assertEqual(ha, hb)
        self.assertEqual(len(ha), 12)
        int(ha, 16)

    def test_table_rows_are_clauses_header_is_context(self):
        txt = "## 1. A\n\n| Col | X |\n|---|---|\n| a | b |\n| c | d |\nafter\n"
        g = build(txt)
        self.assertEqual(ids(g), ["ISA-1-P1", "ISA-1-P2", "ISA-1-P3"])
        self.assertEqual(node(g, "ISA-1-P1")["clause_kind"], "row")
        self.assertEqual(node(g, "ISA-1-P1")["table_header"], ["Col", "X"])
        self.assertEqual(node(g, "ISA-1-P3")["clause_kind"], "para")

    def test_pipe_line_without_separator_is_paragraph(self):
        g = build("## 1. A\n\n|0⟩ after H and X.\nmore\n")
        self.assertEqual(node(g, "ISA-1-P1")["clause_kind"], "para")

    def test_plus_line_is_not_a_list(self):
        g = build("## 1. A\n\nfoo bar\n+ heap_limit_max or\n")
        self.assertEqual(ids(g), ["ISA-1-P1"])

    def test_list_item_multi_paragraph_continuation(self):
        g = build("## 1. A\n\n- item\n\n  still item\n\nnew para\n")
        self.assertEqual(ids(g), ["ISA-1-P1", "ISA-1-P2"])
        self.assertIn("still item", node(g, "ISA-1-P1")["text"])

    def test_item_markers(self):
        txt = ("## 9. S\n\n### 9.2 Id\n\nEnvelope:\n\n```\n  (1) a\n  (5b) b\n```\n\n"
               "- (i) roman item\n- **(a) Bold.** lettered\n\n3. numbered three\n")
        g = build(txt)
        self.assertIn("5b", node(g, "ISA-9.2-P1")["markers"])
        self.assertEqual(node(g, "ISA-9.2-P2")["item_marker"], "i")
        self.assertEqual(node(g, "ISA-9.2-P3")["item_marker"], "a")
        self.assertEqual(node(g, "ISA-9.2-P4")["item_marker"], "3")


class TestMalformed(unittest.TestCase):
    def assertSpecError(self, text, line, fragment):
        with self.assertRaises(cg.SpecError) as cm:
            build(text)
        self.assertEqual(cm.exception.line, line, str(cm.exception))
        self.assertIn(fragment, str(cm.exception))

    def test_unterminated_fence(self):
        self.assertSpecError("## 1. A\n\n```\ncode\n", 3, "unterminated")

    def test_unnumbered_h2(self):
        self.assertSpecError("## 1. A\n\n## Foo\n", 3, "heading")

    def test_h3_under_wrong_h2(self):
        self.assertSpecError("## 1. A\n\n### 2.1 B\n", 3, "heading")

    def test_h4_unsupported(self):
        self.assertSpecError("## 1. A\n\n#### deep\n", 3, "heading")

    def test_duplicate_section(self):
        self.assertSpecError("## 1. A\n\n## 1. B\n", 3, "duplicate section")

    def test_hash_inside_code_is_not_heading(self):
        g = build("## 1. A\n\n```\n#include <x.h>\n## not a heading\n```\n")
        self.assertEqual(ids(g, "section"), ["ISA-1"])


class TestAliases(unittest.TestCase):
    def test_defining_table_labels_and_merge(self):
        txt = ("## 5. R\n\n| Rule | Effect |\n|---|---|\n| PT1 | one |\n| PT3 | three a |\n"
               "| PT3 | three b |\n| `BD-E-X` | code |\n")
        g = build(txt, spec="DLT")
        self.assertEqual(g["aliases"]["PT1"], "DLT-5-P1")
        self.assertEqual(g["aliases"]["PT3"], "DLT-5-P2")
        self.assertEqual(g["aliases"]["BD-E-X"], "DLT-5-P3")
        self.assertIn("three b", node(g, "DLT-5-P2")["text"])
        self.assertEqual(node(g, "DLT-5-P2")["aliases"], ["PT3"])

    def test_non_defining_table(self):
        # header not a definition header, and a mixed table: no aliases
        txt = ("## 5. R\n\n| Tier | x |\n|---|---|\n| CC | a |\n\n"
               "| Rule | Count |\n|---|---|\n| N0: passes | 3 |\n| N11 | 5 |\n")
        g = build(txt, spec="DLT")
        self.assertNotIn("CC", g["aliases"])
        self.assertNotIn("N11", g["aliases"])

    def test_bold_lead_and_list_lead_labels(self):
        txt = ("## 10. T\n\n**TDL-5 Stores to outliving objects.** An object.\n\n"
               "- MC1: the state\n\n1. **C1 capture**: x\n")
        g = build(txt, spec="DLT")
        self.assertEqual(g["aliases"]["TDL-5"], "DLT-10-P1")
        self.assertEqual(g["aliases"]["MC1"], "DLT-10-P2")
        self.assertEqual(g["aliases"]["C1"], "DLT-10-P3")

    def test_table_definition_wins_over_bold_elaboration(self):
        txt = ("## 5. R\n\n| Rule | Effect |\n|---|---|\n| PT3 | three |\n\n"
               "**PT3 witnesses.** More about PT3.\n")
        g = build(txt, spec="DLT")
        self.assertEqual(g["aliases"]["PT3"], "DLT-5-P1")
        self.assertIn(("DLT-5-P2", "DLT-5-P1"), edges(g, "uses"))

    def test_bold_lead_elaboration_with_different_text(self):
        txt = "## 1. A\n\n**BLOCK-IMAGE** def.\n\n**BLOCK-IMAGE witness** more.\n"
        g = build(txt)
        self.assertEqual(g["aliases"]["BLOCK-IMAGE"], "ISA-1-P1")

    def test_duplicate_label_fails_with_both_locations(self):
        txt = ("## 5. R\n\n| Rule | Effect |\n|---|---|\n| PT3 | three |\n\ntext\n\n"
               "| Rule | Effect |\n|---|---|\n| PT3 | again |\n")
        with self.assertRaises(cg.SpecError) as cm:
            build(txt, spec="DLT")
        msg = str(cm.exception)
        self.assertIn("PT3", msg)
        self.assertIn("5", msg)
        self.assertIn("11", msg)

    def test_identical_bold_leads_fail(self):
        with self.assertRaises(cg.SpecError):
            build("## 1. A\n\n**T1 (x).** a\n\n**T1 (x).** b\n")

    def test_terms_from_terminology_and_inline(self):
        txt = ("## 2. Terminology\n\n- **Tier.** CC. **Wire**, **bundle** `wire[N]`.\n"
               "- **lproc**: a lifted procedure.\n\n"
               "## 5. Regions\n\nOnly inside **borrow frames**: maximal subcircuits.\n\n"
               "Uses an **lproc** again.\n\n**Requirement.** A caption, not a term.\n\n"
               "**Frame layout**: one.\n\n**Frame layout**: two.\n\nSay **every** thing.\n")
        g = build(txt)
        a = g["aliases"]
        self.assertEqual(a["Tier"], "ISA-2-P1")
        self.assertEqual(a["Wire"], "ISA-2-P1")
        self.assertEqual(a["bundle"], "ISA-2-P1")
        self.assertEqual(a["lproc"], "ISA-2-P2")  # terminology wins over re-bolding
        self.assertEqual(a["borrow frames"], "ISA-5-P1")
        self.assertNotIn("Requirement", a)
        self.assertNotIn("Frame layout", a)  # ambiguous
        self.assertNotIn("every", a)  # stopword
        amb = {t["term"] for t in g["ambiguous_terms"]}
        self.assertIn("Frame layout", amb)


    def test_single_plain_word_terms_only_in_terminology(self):
        txt = ("## 2. Terminology\n\n- **register**: n qubits. **C**: a set.\n\n"
               "## 3. M\n\nA **constant** operand. **Binding**: one. **self-disjoint** s.\n\n"
               "**Closure set**, by kind. **no `measure` frame** here.\n")
        a = build(txt)["aliases"]
        self.assertIn("register", a)
        self.assertNotIn("C", a)
        self.assertNotIn("constant", a)
        self.assertNotIn("Binding", a)
        self.assertIn("self-disjoint", a)
        self.assertIn("Closure set", a)
        self.assertNotIn("no `measure` frame", a)

class TestRefs(unittest.TestCase):
    SPEC = ("## 4. I\n\n### 4.5 Copy\n\nCopy.\n\n### 4.6 Mem\n\nMem.\n\n### 4.7 Calls\n\nCalls.\n\n"
            "## 5. Regions\n\n### 5.6 Borrow\n\nBorrow.\n\n"
            "## 9. Obj\n\n### 9.2 Id\n\nEnvelope:\n\n```\n (5a) x\n (5b) y\n```\n\n"
            "## 17. Deferred\n\n1. first\n2. second\n\n"
            "## 20. Users\n\n")

    def g(self, body, **kw):
        return build(self.SPEC + body, **kw)

    def test_simple_section_ref(self):
        g = self.g("See §5.6 here.\n")
        self.assertIn(("ISA-20-P1", "ISA-5.6"), edges(g, "refs"))

    def test_double_section_list(self):
        g = self.g("See §§4.5, 4.7 and more.\n")
        r = edges(g, "refs")
        self.assertIn(("ISA-20-P1", "ISA-4.5"), r)
        self.assertIn(("ISA-20-P1", "ISA-4.7"), r)
        self.assertNotIn(("ISA-20-P1", "ISA-4.6"), r)

    def test_range(self):
        g = self.g("See §4.5–§4.7.\n")
        r = edges(g, "refs")
        for s in ("ISA-4.5", "ISA-4.6", "ISA-4.7"):
            self.assertIn(("ISA-20-P1", s), r)

    def test_item_marker_refs(self):
        g = self.g("Part §9.2 (5b) and §17 item 2.\n")
        r = edges(g, "refs")
        self.assertIn(("ISA-20-P1", "ISA-9.2-P1"), r)
        self.assertIn(("ISA-20-P1", "ISA-17-P2"), r)

    def test_item_not_found_falls_back_to_section(self):
        g = self.g("Part §17 item 9.\n")
        self.assertIn(("ISA-20-P1", "ISA-17"), edges(g, "refs"))
        self.assertTrue(any(u["reason"] == "item-not-found" for u in g["unresolved"]))

    def test_unresolved_section(self):
        g = self.g("See §8.3 there.\n")
        u = [x for x in g["unresolved"] if x["reason"] == "no-such-section"]
        self.assertEqual(len(u), 1)
        self.assertEqual(u[0]["src"], "ISA-20-P1")
        self.assertEqual(u[0]["ref"], "§8.3")
        self.assertEqual(u[0]["line"], 39)

    def test_ref_across_line_break_keeps_line(self):
        g = self.g("First line\nsee §8.3.\n")
        u = [x for x in g["unresolved"] if x["reason"] == "no-such-section"][0]
        self.assertEqual(u["line"], 40)

    def test_external_prefixes(self):
        g = self.g("NS §3.1 and north star §5 and (SQLite §5.2(i)).\n")
        reasons = sorted(u["reason"] for u in g["unresolved"])
        self.assertEqual(reasons, ["external:NS", "external:SQLite", "external:north-star"])
        self.assertEqual(edges(g, "refs"), set())

    def test_xspec_without_graph_is_unresolved(self):
        g = self.g("(ISA §5.6, §4.7); §5.6\n", spec="DLT", version="0.2.10")
        r = [u for u in g["unresolved"] if u["reason"] == "xspec-not-loaded"]
        self.assertEqual(sorted(u["ref"] for u in r), ["ISA §4.7", "ISA §5.6"])
        self.assertIn(("DLT-20-P1", "DLT-5.6"), edges(g, "refs"))

    def test_xspec_resolves_against_graph(self):
        isa = build(self.SPEC)
        d = build(self.SPEC + "Per ISA §5.6 and `bennett-isa-spec` §9.2 (5b); ISA §8.8.\n",
                  spec="DLT", version="0.2.10", xspec=isa)
        x = edges(d, "xspec")
        self.assertIn(("DLT-20-P1", "ISA-5.6"), x)
        self.assertIn(("DLT-20-P1", "ISA-9.2-P1"), x)
        self.assertTrue(any(u["reason"] == "no-such-section" and u["ref"] == "ISA §8.8"
                            for u in d["unresolved"]))

    def test_prior_version_is_external(self):
        g = self.g("Motivation (0.8 §5.6).\n", version="0.9-draft")
        self.assertEqual([u["reason"] for u in g["unresolved"]], ["external:ISA-0.8"])

    def test_section_number_before_paren_is_not_a_version_prefix(self):
        g = self.g("per §4.5\n(§5.6: yes) and outside §4.6 (§17 item 1).\n")
        r = edges(g, "refs")
        self.assertIn(("ISA-20-P1", "ISA-5.6"), r)
        self.assertIn(("ISA-20-P1", "ISA-17-P1"), r)
        self.assertEqual(g["unresolved"], [])

    def test_possessive_and_draft_prefixes(self):
        g = self.g("the dialect 0.2 outline's §25 list; dialect draft 0.2.1 §25.1; DL §17.6.\n")
        reasons = sorted(u["reason"] for u in g["unresolved"])
        self.assertEqual(reasons, ["external:dialect-outline", "external:outline",
                                   "xspec-not-loaded"])

    def test_item_range(self):
        g = build("## 6. H\n\n1. a\n2. b\n3. c\n4. d\n\n## 7. U\n\nSee §6 items 2–4.\n")
        r = edges(g, "refs")
        for p in ("ISA-6-P2", "ISA-6-P3", "ISA-6-P4"):
            self.assertIn(("ISA-7-P1", p), r)
        self.assertNotIn(("ISA-7-P1", "ISA-6-P1"), r)

    def test_letter_suffix_item(self):
        txt = "## 7. R\n\n### 7.5 W\n\n- **(a) M.** x\n- **(b) R.** y\n\n## 8. U\n\nSee §7.5b and §7.5(a).\n"
        g = build(txt)
        r = edges(g, "refs")
        self.assertIn(("ISA-8-P1", "ISA-7.5-P2"), r)
        self.assertIn(("ISA-8-P1", "ISA-7.5-P1"), r)


class TestUses(unittest.TestCase):
    def test_label_mentions_word_boundaries(self):
        txt = ("## 12. N\n\n| Id | What |\n|---|---|\n| N2 | two |\n| N21 | twenty-one |\n"
               "| TDL-5 | five |\n\n"
               "Use N21 here, TDL-50 not, and N2.\n\nAlso TDL-5(b) and xN2 no.\n\nNone: N2a N2-x.\n")
        g = build(txt, spec="DLT")
        u = edges(g, "uses")
        self.assertIn(("DLT-12-P4", "DLT-12-P2"), u)
        self.assertIn(("DLT-12-P4", "DLT-12-P1"), u)
        self.assertNotIn(("DLT-12-P4", "DLT-12-P3"), u)
        self.assertIn(("DLT-12-P5", "DLT-12-P3"), u)
        self.assertNotIn(("DLT-12-P5", "DLT-12-P1"), u)
        self.assertFalse(any(s == "DLT-12-P6" for s, _ in u))

    def test_label_range(self):
        txt = ("## 5. R\n\n| Rule | E |\n|---|---|\n| PT3 | a |\n| PT4 | b |\n| PT5 | c |\n\n"
               "See PT3–PT5.\n")
        g = build(txt, spec="DLT")
        u = edges(g, "uses")
        for p in ("DLT-5-P1", "DLT-5-P2", "DLT-5-P3"):
            self.assertIn(("DLT-5-P4", p), u)

    def test_term_mentions_case_sensitive_plural_and_self_skip(self):
        txt = ("## 2. Terminology\n\n- **lproc**: a lifted procedure; an lproc has params.\n\n"
               "## 3. M\n\nEvery lproc here.\n\nSeveral lprocs.\n\nLproc capital no.\n\nmylproc no.\n")
        g = build(txt)
        u = edges(g, "uses")
        self.assertIn(("ISA-3-P1", "ISA-2-P1"), u)
        self.assertIn(("ISA-3-P2", "ISA-2-P1"), u)
        self.assertNotIn(("ISA-3-P3", "ISA-2-P1"), u)
        self.assertNotIn(("ISA-3-P4", "ISA-2-P1"), u)
        self.assertNotIn(("ISA-2-P1", "ISA-2-P1"), u)

    def test_terms_not_matched_in_code(self):
        txt = ("## 2. Terminology\n\n- **commit**: the commit step.\n\n## 3. M\n\nText.\n\n"
               "```\ncommit x\n```\n")
        g = build(txt)
        self.assertNotIn(("ISA-3-P1", "ISA-2-P1"), edges(g, "uses"))


QSPEC = """## 1. Base

Def of **borrow frames**: things.

Other text.

| Rule | Effect |
|---|---|
| PT3 | copies |

## 2. Mid

Uses borrow frames here.

See §1.

## 3. Top

See §2 and stuff.

Independent text.

## 4. Iso

Lone.
"""


def qgraph():
    return build(QSPEC)


class TestQueries(unittest.TestCase):
    def setUp(self):
        self.m = cg.Merged([qgraph()])

    def test_resolve(self):
        self.assertEqual(self.m.resolve("ISA-1-P1"), "ISA-1-P1")
        self.assertEqual(self.m.resolve("PT3"), "ISA-1-P3")
        self.assertEqual(self.m.resolve("ISA:PT3"), "ISA-1-P3")
        self.assertEqual(self.m.resolve("ISA-2"), "ISA-2")
        with self.assertRaises(cg.UsageError):
            self.m.resolve("ISA-9-P1")

    def test_impact_depths(self):
        lv = self.m.impact("ISA-1-P1")
        self.assertEqual(lv[0], ["ISA-2-P1", "ISA-2-P2"])
        self.assertEqual(lv[1], ["ISA-3-P1"])
        self.assertEqual(len(lv), 2)

    def test_impact_via_section_ref_only(self):
        lv = self.m.impact("ISA-1-P2")
        self.assertEqual(lv, [["ISA-2-P2"], ["ISA-3-P1"]])

    def test_impact_of_section(self):
        lv = self.m.impact("ISA-2")
        self.assertEqual(lv, [["ISA-3-P1"]])

    def test_independent(self):
        ok, fwd, rev = self.m.independent("ISA-3-P2", "ISA-4-P1")
        self.assertTrue(ok)
        ok, fwd, rev = self.m.independent("ISA-1-P1", "ISA-1-P2")
        self.assertFalse(ok)
        self.assertEqual(rev, ["ISA-2-P2", "ISA-3-P1"])
        ok, fwd, rev = self.m.independent("ISA-2-P1", "ISA-2-P2")
        self.assertFalse(ok)
        self.assertIn("ISA-1-P1", fwd)

    def test_top(self):
        top = self.m.top("ISA", 2)
        self.assertEqual(top[0][0], "ISA-1-P1")
        self.assertEqual(top[0][1], 3)

    def test_coverage(self):
        g = qgraph()
        rep = cg.coverage(g, {"t1": ["ISA-1-P1"], "t2": "ISA-2", "t3": ["PT3"],
                              "t4": ["DLT-5-P1"]})
        sec = {s["section"]: s for s in rep["sections"]}
        self.assertEqual(sec["ISA-1"]["strong"], ["ISA-1-P1", "ISA-1-P3"])
        self.assertEqual(sec["ISA-1"]["uncovered"], ["ISA-1-P2"])
        self.assertEqual(sec["ISA-2"]["weak"], ["ISA-2-P1", "ISA-2-P2"])
        self.assertEqual(sec["ISA-3"]["uncovered"], ["ISA-3-P1", "ISA-3-P2"])
        self.assertEqual(rep["foreign"], ["DLT-5-P1"])
        self.assertEqual(rep["totals"]["uncovered"], 4)
        with self.assertRaises(cg.UsageError):
            cg.coverage(g, {"t": ["ISA-7-P1"]})
        with self.assertRaises(cg.UsageError):
            cg.coverage(g, {"t": 5})


class TestXspecMerged(unittest.TestCase):
    def test_cross_spec_impact_and_stale_check(self):
        isa = qgraph()
        dlt = build("## 1. D\n\nPer ISA §1 (borrow).\n\nAnd DLT-local.\n", spec="DLT",
                    version="0.2.10", xspec=isa)
        m = cg.Merged([isa, dlt])
        self.assertEqual(m.impact("ISA-1-P2"), [["DLT-1-P1", "ISA-2-P2"], ["ISA-3-P1"]])
        stale = build(QSPEC + "\nextra\n")
        with self.assertRaises(cg.UsageError):
            cg.Merged([stale, dlt])
        with self.assertRaises(cg.UsageError):
            cg.Merged([dlt]).resolve("ISA-1-P2")


class TestDiff(unittest.TestCase):
    A = """## 1. A

Alpha paragraph stays.

| Id | What |
|---|---|
| K1 | first meaning |

Beta paragraph goes away.

The quick brown fox jumps over the lazy dog near the river bank today.
"""
    B = """## 1. A

Inserted paragraph.

Alpha paragraph stays.

| Id | What |
|---|---|
| K1 | second meaning |

The quick brown fox jumps over the lazy cat near the river bank today.

## 2. New

Brand new.
"""

    def test_diff(self):
        d = cg.diff_graphs(build(self.A), build(self.B, version="0.9-draft"))
        self.assertEqual(sorted(d["added"]), ["ISA-1-P1", "ISA-2-P1"])
        self.assertEqual(d["removed"], ["ISA-1-P3"])
        moved = {(a, b, h) for a, b, h in d["moved"]}
        self.assertIn(("ISA-1-P1", "ISA-1-P2", "hash"), moved)
        self.assertIn(("ISA-1-P2", "ISA-1-P3", "alias"), moved)
        changed = {(a, b, h) for a, b, h in d["changed"]}
        self.assertIn(("ISA-1-P2", "ISA-1-P3", "alias"), changed)
        self.assertIn(("ISA-1-P4", "ISA-1-P4", "similar"), changed)
        self.assertEqual(d["summary"]["unchanged"], 0)
        self.assertEqual(d["summary"]["added"], 2)

    def test_diff_rejects_different_specs(self):
        with self.assertRaises(cg.UsageError):
            cg.diff_graphs(build(self.A), build(self.A, spec="DLT"))


class TestDot(unittest.TestCase):
    def test_dot(self):
        dot = cg.to_dot(qgraph())
        self.assertTrue(dot.startswith('digraph "ISA-0.8" {'))
        self.assertIn('subgraph "cluster_ISA-1"', dot)
        self.assertIn('"ISA-2-P1" -> "ISA-1-P1"', dot)
        self.assertNotIn("contains", dot)


def run_cli(*args, cwd=None):
    env = {k: v for k, v in os.environ.items()
           if k not in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")}
    return subprocess.run([sys.executable, "-I", os.path.join(CLAUSES, "clausegraph.py")]
                          + list(args), capture_output=True, text=True, env=env, cwd=cwd)


class TestCli(unittest.TestCase):
    def setUp(self):
        base = os.environ.get("TMPDIR") or os.path.join(REPO, ".tmp")
        os.makedirs(base, exist_ok=True)
        self.tmp = tempfile.mkdtemp(dir=base)
        self.spec = os.path.join(self.tmp, "s.md")
        with open(self.spec, "w", encoding="utf-8") as f:
            f.write(QSPEC)

    def test_build_and_queries(self):
        r = run_cli("build", "--spec", "ISA", "--version", "0.8", "--file", self.spec,
                    "--out-dir", self.tmp)
        self.assertEqual(r.returncode, 0, r.stderr)
        gj = os.path.join(self.tmp, "ISA-0.8.json")
        self.assertTrue(os.path.exists(gj))
        self.assertTrue(os.path.exists(os.path.join(self.tmp, "ISA-0.8.dot")))
        with open(gj, encoding="utf-8") as f:
            g = json.load(f)
        self.assertEqual(g["stats"]["clauses"], 8)
        r = run_cli("impact", gj, "ISA-1-P1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("depth 1", r.stdout)
        self.assertIn("ISA-3-P1", r.stdout)
        r = run_cli("independent", gj, "ISA-3-P2", "ISA-4-P1")
        self.assertEqual((r.returncode, r.stdout.strip()), (0, "independent"))
        r = run_cli("independent", gj, "ISA-1-P1", "ISA-1-P2")
        self.assertEqual(r.returncode, 1)
        self.assertIn("ISA-3-P1", r.stdout)
        tags = os.path.join(self.tmp, "tags.json")
        with open(tags, "w") as f:
            json.dump({"t": ["ISA-1-P1"], "s": "ISA-2"}, f)
        r = run_cli("coverage", gj, "--tags", tags)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("weak", r.stdout)
        r = run_cli("diff", gj, gj)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("unchanged", r.stdout)

    def test_malformed_input_exit_code_and_line(self):
        bad = os.path.join(self.tmp, "bad.md")
        with open(bad, "w") as f:
            f.write("## 1. A\n\n```\nnever closed\n")
        r = run_cli("build", "--spec", "ISA", "--version", "0.8", "--file", bad,
                    "--out-dir", self.tmp)
        self.assertEqual(r.returncode, 2)
        self.assertIn("bad.md:3:", r.stderr)

    def test_unknown_id_is_an_error(self):
        run_cli("build", "--spec", "ISA", "--version", "0.8", "--file", self.spec,
                "--out-dir", self.tmp)
        r = run_cli("impact", os.path.join(self.tmp, "ISA-0.8.json"), "NOPE-1")
        self.assertEqual(r.returncode, 2)
        self.assertIn("NOPE-1", r.stderr)


if __name__ == "__main__":
    unittest.main()
