"""Claim-level replay: published fixture, §12 witnesses, and mutation per claim field."""
import copy, hashlib, json, unittest
from checker_replay import hashes, jcs
from checker_replay.objparse import parse_object
from checker_replay.replay import check_certificate
from tests.helpers import cert_for, claim_of, controlled, rng, source, tup
from tests.spec_vectors import example_objects

EX = example_objects()
ROOT = "df131e972083b8d3b3b7d9341342b55252ca7a1963c4b7b6f702b9cdf3fbde12"


def reject_cited(tc, c, code=None):
    tc.assertEqual(c["outcome"], "reject", c)
    for e in c["errors"]:
        tc.assertTrue(e["section"], e)
    if code:
        tc.assertEqual(c["errors"][0]["code"], code, c)


class PublishedFixture(unittest.TestCase):
    def setUp(self):
        self.text, self.cert, self.target, self.schedule = controlled()

    def go(self, cert=None, **kw):
        kw.setdefault("target", self.target)
        return claim_of(self.text, cert or self.cert, ROOT, **kw)

    def test_proof_full_domain(self):
        c = self.go()
        self.assertEqual((c["outcome"], c["elements"], c["covered_fraction"]), ("proof", 4, "1"), c)

    def test_non_exhaustive_claims_are_unchecked_with_reason(self):
        v = check_certificate(self.cert, parse_object(self.text), self.text, target=self.target)
        self.assertEqual(v["overall"], "unchecked")
        c = v["regions"][ROOT]["claims"][1]
        self.assertEqual((c["kind"], c["outcome"]), ("numerical", "unchecked"))
        self.assertTrue(c["reason"])
        self.assertTrue(v["unchecked_scope"])
        self.assertEqual(v["summary"], {"proof": 1, "evidence": 0, "reject": 0, "unchecked": 2})

    def test_without_target_realisation_is_unchecked_never_accepted(self):
        c = claim_of(self.text, self.cert, ROOT)
        self.assertEqual(c["outcome"], "unchecked", c)
        self.assertNotIn("covered_fraction", c)

    def test_host_limit_is_unchecked_with_no_fraction(self):
        c = self.go(max_elements=2)
        self.assertEqual((c["outcome"], c["section"]), ("unchecked", "12.2"), c)
        self.assertNotIn("covered_fraction", c)
        self.assertNotIn("elements", c)

    def mutate(self, fn):
        cert = copy.deepcopy(self.cert)
        fn(cert["regions"][ROOT]["claims"][0])
        return cert

    def test_value_mutations_per_claim_field(self):
        t = lambda c: c["expected"]["table"]
        cases = {
            "kind": lambda c: c.update(kind="exhaustive-typo"),
            "unknown_field": lambda c: c.update(extra=1),
            "label_invalid": lambda c: c.update(label="maybe"),
            "proof_label_on_slice": lambda c: (c["domain"].update(bundles=c["domain"]["bundles"][:1]),
                                               c["expected"].update(table=t(c)[:2])),
            "subject_missing": lambda c: c.pop("subject"),
            "expected_missing": lambda c: c.pop("expected"),
            "subject_closure": lambda c: c["subject"].update(closure_hash="0" * 64),
            "subject_variant": lambda c: c["subject"].update(variant=""),
            "subject_inherited_extra": lambda c: c["subject"].update(inherited_controls=1),
            "subject_schedule": lambda c: c["subject"].update(schedule_hash="0" * 64),
            "domain_name": lambda c: c["domain"]["bundles"][0].update(name="nope"),
            "domain_width": lambda c: c["domain"]["bundles"][0].update(width=2),
            "domain_lo_gt_hi": lambda c: c["domain"]["bundles"][0].update(lo=1, hi=0),
            "domain_hi_out_of_width": lambda c: c["domain"]["bundles"][0].update(hi=2),
            "domain_duplicate": lambda c: c["domain"]["bundles"].append(dict(c["domain"]["bundles"][0])),
            "domain_borrowed_beyond": lambda c: c["domain"].update(borrowed=[rng("b", 1)]),
            "domain_args_on_table": lambda c: c["domain"].update(args=[]),
            "domain_region_unknown": lambda c: c["domain"].update(regions=[{"name": "nope", "cells": "all"}]),
            "table_missing_row": lambda c: t(c).pop(),
            "table_duplicate_row": lambda c: t(c).append(copy.deepcopy(t(c)[0])),
            "table_wrong_order": lambda c: t(c).reverse(),
            "table_wrong_after": lambda c: t(c)[1]["after"].update(bundles=["0", "0"]),
            "table_wrong_before": lambda c: t(c)[0]["before"].update(bundles=["1", "1"]),
            "table_noncanonical": lambda c: t(c)[0]["before"].update(bundles=["00", "0"]),
            "table_number_not_string": lambda c: t(c)[0]["before"].update(bundles=[0, 0]),
            "table_out_of_width": lambda c: t(c)[0]["after"].update(bundles=["2", "0"]),
            "table_short_tuple": lambda c: t(c)[0]["after"].update(bundles=["0"]),
            "table_row_extra": lambda c: t(c)[0].update(extra={}),
            "program_unknown": lambda c: c.update(expected={"program": "0" * 64}),
            "expected_both": lambda c: c["expected"].update(program="0" * 64),
            "expected_oracle_on_action": lambda c: c.update(expected="oracle"),
            "invocation_conflict": lambda c: c.update(invocation={"args": [], "budget": ["1", "0", "0", "0"]}),
        }
        for name, fn in cases.items():
            with self.subTest(name=name):
                reject_cited(self, self.go(self.mutate(fn)))

    def test_type_mutation_sweep_every_leaf(self):
        base = self.cert["regions"][ROOT]["claims"][0]
        paths = []

        def walk(x, p):
            if isinstance(x, dict):
                for k, v in x.items():
                    walk(v, p + [k])
            elif isinstance(x, list):
                for i, v in enumerate(x):
                    walk(v, p + [i])
            else:
                paths.append(p)
        walk(base, [])
        self.assertEqual(len(paths), 30)
        for p in paths:
            def fn(c, p=p):
                node = c
                for k in p[:-1]:
                    node = node[k]
                node[p[-1]] = {"mutated": True}
            with self.subTest(path="/".join(map(str, p))):
                self.assertEqual(self.go(self.mutate(fn))["outcome"], "reject")

    def test_document_bindings(self):
        for field in ("object_hash", "closure_hash", "interface_hash", "reference_program_hash", "target_hash", "schedule_hash"):
            cert = copy.deepcopy(self.cert)
            cert["bindings"][field] = "0" * 64
            with self.subTest(field=field):
                v = check_certificate(cert, parse_object(self.text), self.text, target=self.target)
                self.assertEqual(v["overall"], "reject", v["errors"])
                self.assertTrue(v["errors"][0]["section"])

    def rebound(self, edit):
        """Mutate the schedule, then rebind every hash so only the semantic check can object."""
        sched = copy.deepcopy(self.schedule)
        edit(sched)
        sh = hashlib.sha256(jcs.canonical(sched)).hexdigest()
        cert = copy.deepcopy(self.cert)
        cert["bindings"]["schedule_hash"] = sh
        for rec in cert["regions"].values():
            for c in rec["claims"]:
                c["subject"]["schedule_hash"] = sh
        return claim_of(self.text, cert, ROOT, target=self.target, schedule=sched)

    def test_schedule_root_must_be_the_certificate_root(self):
        """A rebound schedule whose root closure is not the certificate's root is a binding error,
        not a nested invocation context (differential witness object:x_extra)."""
        for bad in ("0" * 64, "f" * 64):
            sched = copy.deepcopy(self.schedule)
            sched["bindings"]["closure_hash"] = bad
            sh = hashlib.sha256(jcs.canonical(sched)).hexdigest()
            cert = copy.deepcopy(self.cert)
            cert["bindings"]["schedule_hash"] = sh
            for rec in cert["regions"].values():
                for c in rec["claims"]:
                    c["subject"]["schedule_hash"] = sh
            v = check_certificate(cert, parse_object(self.text), self.text, target=self.target, schedule=sched)
            with self.subTest(bad=bad):
                self.assertEqual(v["overall"], "reject", v)
                self.assertEqual(v["errors"][0]["path"], "schedule.bindings.closure_hash", v["errors"])

    def test_schedule_scheme_selection_is_unchecked(self):
        c = self.rebound(lambda s: s["entries"][0]["gate"].update(realisation="toffoli7"))
        self.assertEqual(c["outcome"], "unchecked", c)

    def test_schedule_duplicate_entry_ties(self):
        c = self.rebound(lambda s: s["entries"].append(copy.deepcopy(s["entries"][0])))
        reject_cited(self, c, "E_SCHEDULE_INFEASIBLE")

    def test_schedule_more_specific_entry_breaks_the_tie(self):
        def edit(s):
            lo = copy.deepcopy(s["entries"][0])
            lo["generated"] = "*"
            s["entries"] += [lo, copy.deepcopy(lo)]  # equal, but below the existing generated:0 entry
        self.assertEqual(self.rebound(edit)["outcome"], "proof")

    def test_schedule_budget_and_params_binding(self):
        reject_cited(self, self.rebound(lambda s: s["bindings"].update(budget=[1, 0, 0, 0])), "E_SCHEDULE_INFEASIBLE")
        reject_cited(self, self.rebound(lambda s: s["bindings"].update(params_hash="0" * 64)), "E_SCHEDULE_INFEASIBLE")
        reject_cited(self, self.rebound(lambda s: s["bindings"].update(variant="other")), "E_SCHEDULE_INFEASIBLE")

    def test_schedule_foreign_library_is_unchecked(self):
        c = self.rebound(lambda s: s["bindings"]["expansion"].update(hash="0" * 64))
        self.assertEqual(c["outcome"], "unchecked", c)

    def test_schedule_unknown_path_rejects(self):
        c = self.rebound(lambda s: s["entries"][0]["path"][1].update(point="entry:9"))
        reject_cited(self, c, "E_SCHEDULE_INFEASIBLE")

    def test_native_entry_with_wrong_control_count(self):
        target = copy.deepcopy(self.target)
        target["gates"][0]["controls"] = {"min": 2, "max": 2}
        tgt_hash = hashlib.sha256(jcs.canonical(target)).hexdigest()
        cert = copy.deepcopy(self.cert)
        cert["bindings"]["target_hash"] = tgt_hash
        sched = copy.deepcopy(self.schedule)
        sched["bindings"]["target_hash"] = tgt_hash
        sh = hashlib.sha256(jcs.canonical(sched)).hexdigest()
        cert["bindings"]["schedule_hash"] = sh
        for rec in cert["regions"].values():
            for c in rec["claims"]:
                c["subject"]["schedule_hash"] = sh
        reject_cited(self, claim_of(self.text, cert, ROOT, target=target, schedule=sched), "E_SCHEDULE_INFEASIBLE")


INC8 = EX[1]


def inc8_claim(label="evidence", budget="3"):
    rows = [{"before": tup([x, 0, 0]), "after": tup([x, (x + 1) % 256, 0])} for x in range(256)]
    c = {"kind": "exhaustive", "subject": {"closure_hash": None, "variant": "forward", "inherited_controls": 0},
         "invocation": {"args": [], "budget": [budget, "0", "0", "0"]},
         "domain": {"bundles": [rng("x", 8)], "borrowed": [], "regions": []}, "expected": {"table": rows}}
    if label:
        c["label"] = label
    return c


class OracleTable(unittest.TestCase):
    def build(self, claim):
        cert, ch, _h = cert_for(INC8, "inc8", [claim])
        cert["regions"][ch]["claims"][0]["subject"]["closure_hash"] = ch
        return cert, ch

    def test_omitted_target_and_fault_are_zero_slice(self):
        cert, ch = self.build(inc8_claim())
        c = claim_of(INC8, cert, ch)
        self.assertEqual((c["outcome"], c["elements"], c["covered_fraction"]), ("evidence", 256, "1/4096"), c)

    def test_requested_proof_on_slice_rejects(self):
        cert, ch = self.build(inc8_claim("proof"))
        reject_cited(self, claim_of(INC8, cert, ch), "E_REPLAY_COVERAGE_LABEL")

    def test_slice_fraction_counts_individual_components(self):
        c = inc8_claim()
        c["domain"]["bundles"] = [rng("x", 8, 0, 3), rng("t", 8, 0, 3), rng("f", 4)]
        c["expected"]["table"] = [{"before": tup([x, t, f]), "after": tup([x, t ^ (x + 1), f])}
                                  for x in range(4) for t in range(4) for f in range(16)]
        cert, ch = self.build(c)
        out = claim_of(INC8, cert, ch)
        self.assertEqual((out["outcome"], out["elements"], out["covered_fraction"]), ("evidence", 256, "1/4096"), out)

    def test_full_domain_with_arbitrary_initial_target_and_fault_is_proof(self):
        src = source("oracle @o (x:wire in) (t:wire) (f:wire[4]) () ()\nregs { r:reg[1] }\n^entry():\n"
                     "r = lift.mov x\nlift.retval r\n")
        rows = [{"before": tup([x, t, f]), "after": tup([x, t ^ x, f])}
                for x in (0, 1) for t in (0, 1) for f in range(16)]
        cl = {"kind": "exhaustive", "label": "proof", "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 0},
              "invocation": {"args": [], "budget": ["2", "0", "0", "0"]},
              "domain": {"bundles": [rng("x", 1), rng("t", 1), rng("f", 4)], "borrowed": [], "regions": []},
              "expected": {"table": rows}}
        cert, ch, _h = cert_for(src, "o", [cl])
        cl["subject"]["closure_hash"] = ch
        out = claim_of(src, cert, ch)
        self.assertEqual((out["outcome"], out["elements"], out["covered_fraction"]), ("proof", 64, "1"), out)
        # the fault bundle preserved in the row but faulted under B=1: full tuple compare catches it
        cl["invocation"]["budget"] = ["1", "0", "0", "0"]
        reject_cited(self, claim_of(src, cert, ch), "E_REPLAY_MISMATCH")

    def test_budget_fault_row(self):
        c = inc8_claim(budget="2")
        c["expected"]["table"] = [{"before": tup([x, 0, 0]), "after": tup([x, 0, 1])} for x in range(256)]
        cert, ch = self.build(c)
        self.assertEqual(claim_of(INC8, cert, ch)["outcome"], "evidence")

    def test_wrong_budget_cannot_replay_the_same_table(self):
        cert, ch = self.build(inc8_claim(budget="2"))
        reject_cited(self, claim_of(INC8, cert, ch), "E_REPLAY_MISMATCH")

    def test_invocation_mutations(self):
        muts = {
            "missing": lambda c: c.pop("invocation"),
            "short_budget": lambda c: c["invocation"].update(budget=["3", "0", "0"]),
            "number_budget": lambda c: c["invocation"].update(budget=[3, 0, 0, 0]),
            "noncanonical": lambda c: c["invocation"].update(budget=["03", "0", "0", "0"]),
            "memoryless_heap": lambda c: c["invocation"].update(budget=["3", "1", "0", "0"]),
            "extra_arg": lambda c: c["invocation"].update(args=["1"]),
            "extra_field": lambda c: c["invocation"].update(x=1),
        }
        for name, fn in muts.items():
            c = inc8_claim()
            fn(c)
            cert, ch = self.build(c)
            with self.subTest(name=name):
                reject_cited(self, claim_of(INC8, cert, ch))


class K13Witness(unittest.TestCase):
    """§12.1 CLAIM-REPLAY witness relation, minus the borrowed component (see bead)."""
    SRC = None

    @classmethod
    def setUpClass(cls):
        cls.SRC = source("oracle @o (x:wire[2] in) (t:wire[2]) (f:wire[4]) (m:region[1,8] in) (a:i2)\n"
                         "regs { r:reg[2] }\n^entry():\nr = lift.add x a\nlift.retval r\n",
                         layout={"A_tab": 1, "H": 2, "S_cells": 0, "globals": {}, "heap_base": 3, "heap_limit_max": 0,
                                 "interface_regions": {"o": {"m": {"base": 1}}}, "null_size": 1, "ptr_width": 32})

    def claim(self, after, args="2", budget="2"):
        return {"kind": "exhaustive", "label": "evidence",
                "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 0},
                "invocation": {"args": [args], "budget": [budget, "0", "0", "0"]},
                "domain": {"bundles": [rng("x", 2, 1, 1), rng("t", 2, 2, 2), rng("f", 4, 3, 3)], "borrowed": [],
                           "regions": [{"name": "m", "cells": [{"index": 0, "lo": 2, "hi": 2}, {"index": 1, "lo": 1, "hi": 1}]}]},
                "expected": {"table": [{"before": tup([1, 2, 3], [], [[2, 1]]), "after": after}]}}

    def run_(self, claim):
        cert, ch, _h = cert_for(self.SRC, "o", [claim])
        claim["subject"]["closure_hash"] = ch
        return claim_of(self.SRC, cert, ch)

    def test_row(self):
        # The spec states 1/33554432 with its one-bit dirty bundle; without it |D| is 2^24.
        c = self.run_(self.claim(tup([1, 1, 3], [], [[2, 1]])))
        self.assertEqual((c["outcome"], c["elements"], c["covered_fraction"]), ("evidence", 1, "1/16777216"), c)

    def test_identity_wrong_row_fails(self):
        reject_cited(self, self.run_(self.claim(tup([1, 2, 3], [], [[2, 1]]))), "E_REPLAY_MISMATCH")

    def test_changed_arg_and_short_budget_differ(self):
        c = self.run_(self.claim(tup([1, 0, 3], [], [[2, 1]]), args="1"))
        self.assertEqual(c["outcome"], "evidence", c)
        c = self.run_(self.claim(tup([1, 2, 2], [], [[2, 1]]), budget="1"))
        self.assertEqual(c["outcome"], "evidence", c)
        reject_cited(self, self.run_(self.claim(tup([1, 1, 3], [], [[2, 1]]), args="1")), "E_REPLAY_MISMATCH")
        reject_cited(self, self.run_(self.claim(tup([1, 1, 3], [], [[2, 1]]), budget="1")), "E_REPLAY_MISMATCH")

    def test_changed_region_input_is_not_zero_filled(self):
        claim = self.claim(tup([1, 1, 3], [], [[2, 1]]))
        claim["domain"]["regions"][0]["cells"].pop()
        reject_cited(self, self.run_(claim))


SRC_T = source(
    "action @a (t:wire mut, u:wire mut) () () () ()\n^entry():\nb = qborrow any 1\nqreturn b\ncx t u\n"
    "action @i (x:wire mut, y:wire mut) () () () ()\n^entry():\nb = qborrow any 1\nqreturn b\n"
    "action @n (p:wire mut) () () () ()\n^entry():\nb = qborrow any 1\nqreturn b\n"
    "action @sw (u:wire mut, t:wire mut) () () () ()\n^entry():\nb = qborrow any 1\nqreturn b\ncx u t\n"
    "action @cx2 (u:wire mut, t:wire mut) () () () ()\n^entry():\nb = qborrow any 1\nqreturn b\ncx t u\n"
    "action @noborrow (t:wire mut, u:wire mut) () () () ()\n^entry():\ncx t u\n")


class ComponentCoverageWitness(unittest.TestCase):
    """§12.2 COMPONENT-COVERAGE witness: exact counts, fraction and first mismatch."""

    def claim(self, program, domain, label="evidence"):
        return {"kind": "exhaustive", "label": label, "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 0},
                "invocation": {"args": [], "budget": ["0", "0", "0", "0"]}, "domain": domain,
                "expected": {"program": program}}

    def run_(self, name, domain, label="evidence", root="a"):
        cert, ch, h = cert_for(SRC_T, root, [self.claim(h_of(name), domain, label)])
        cert["regions"][ch]["claims"][0]["subject"]["closure_hash"] = ch
        return claim_of(SRC_T, cert, ch)

    def test_borrowed_named_t_is_not_the_mut_t(self):
        dom = {"bundles": [rng("u", 1)], "borrowed": [rng("t", 1)], "regions": []}
        c = self.run_("i", dom)
        self.assertEqual((c["outcome"], c["elements"], c["covered_fraction"]), ("evidence", 4, "1/2"), c)
        reject_cited(self, self.run_("i", dom, "proof"), "E_REPLAY_COVERAGE_LABEL")

    def test_renaming_the_borrowed_label_changes_nothing(self):
        dom = {"bundles": [rng("u", 1)], "borrowed": [rng("b", 1)], "regions": []}
        self.assertEqual(self.run_("i", dom)["covered_fraction"], "1/2")

    def test_full_order_first_mismatch_at_element_4(self):
        dom = {"bundles": [rng("t", 1), rng("u", 1)], "borrowed": [rng("b", 1)], "regions": []}
        c = self.run_("i", dom)
        reject_cited(self, c, "E_REPLAY_MISMATCH")
        self.assertEqual(c["errors"][0]["witness"]["element"], 4)
        self.assertEqual(c["errors"][0]["witness"]["before"], tup([1, 0], [0]))
        self.assertEqual(c["errors"][0]["witness"]["replayed"], tup([1, 1], [0]))
        self.assertEqual(c["errors"][0]["witness"]["expected"], tup([1, 0], [0]))

    def test_K73_position_not_name(self):
        # @sw has mut names u,t swapped relative to @a; it computes cx slot0 slot1 as well
        dom = {"bundles": [rng("t", 1), rng("u", 1)], "borrowed": [rng("b", 1)], "regions": []}
        c = self.run_("sw", dom, "proof")
        self.assertEqual((c["outcome"], c["covered_fraction"]), ("proof", "1"), c)
        # same local names, opposite gate direction by position: must disagree
        reject_cited(self, self.run_("cx2", dom), "E_REPLAY_MISMATCH")

    def test_K73_incompatible_signatures(self):
        dom = {"bundles": [rng("t", 1), rng("u", 1)], "borrowed": [], "regions": []}
        reject_cited(self, self.run_("n", dom), "E_REPLAY_EXPECTED")
        reject_cited(self, self.run_("noborrow", {"bundles": [rng("t", 1)], "borrowed": [], "regions": []}), "E_REPLAY_EXPECTED")

    def test_borrowed_beyond_requirement_rejects(self):
        dom = {"bundles": [rng("t", 1)], "borrowed": [rng("b", 1), rng("c", 1)], "regions": []}
        reject_cited(self, self.run_("i", dom), "E_REPLAY_DOMAIN")

    def test_inherited_controls_positional_and_required(self):
        cert, ch, h = cert_for(SRC_T, "a", [{
            "kind": "exhaustive", "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 1},
            "invocation": {"args": [], "budget": ["0", "0", "0", "0"]},
            "domain": {"bundles": [rng("whatever", 1), rng("t", 1), rng("u", 1)], "borrowed": [rng("b", 1)], "regions": []},
            "expected": {"table": [{"before": tup([w, t, u], [b]), "after": tup([w, t, u ^ (w & t)], [b])}
                                   for w in (0, 1) for t in (0, 1) for u in (0, 1) for b in (0, 1)]}}])
        c0 = cert["regions"][ch]["claims"][0]
        c0["subject"]["closure_hash"] = ch
        out = claim_of(SRC_T, cert, ch)
        self.assertEqual((out["outcome"], out["elements"]), ("proof", 16), out)
        c0["domain"]["bundles"] = c0["domain"]["bundles"][1:]
        reject_cited(self, claim_of(SRC_T, cert, ch))


def h_of(name):
    return hashes.compute_hashes(parse_object(SRC_T))[name][1]


class RegionCoverage(unittest.TestCase):
    SRC = source("action @a (t:wire mut) () (m:region[1,1] in) () ()\n^entry():\nd = qload m\n".replace("qload m", "qalloc any 1") +
                 "qfree d\n")

    def claim(self, cells, rows):
        return {"kind": "exhaustive", "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 0},
                "invocation": {"args": [], "budget": ["0", "0", "0", "0"]},
                "domain": {"bundles": [rng("t", 1)], "borrowed": [], "regions": [{"name": "m", "cells": cells}]},
                "expected": {"table": rows}}

    def go(self, cells, rows):
        cert, ch, _h = cert_for(self.SRC, "a", [self.claim(cells, rows)])
        cert["regions"][ch]["claims"][0]["subject"]["closure_hash"] = ch
        return claim_of(self.SRC, cert, ch)

    def test_reversed_explicit_full_list_covers_like_all(self):
        rows = [{"before": tup([t], [], [[c0, c1]]), "after": tup([t], [], [[c0, c1]])}
                for t in (0, 1) for c1 in (0, 1) for c0 in (0, 1)]
        c = self.go([{"index": 1, "lo": 0, "hi": 1}, {"index": 0, "lo": 0, "hi": 1}], rows)
        self.assertEqual((c["outcome"], c["covered_fraction"], c["elements"]), ("proof", "1", 8), c)

    def test_repeat_and_out_of_range_index_are_ill_formed(self):
        reject_cited(self, self.go([{"index": 1, "lo": 0, "hi": 1}, {"index": 1, "lo": 0, "hi": 1}], []), "E_REPLAY_DOMAIN")
        reject_cited(self, self.go([{"index": 2, "lo": 0, "hi": 1}], []), "E_REPLAY_DOMAIN")

    def test_huge_region_is_host_limit_not_illegal(self):
        src = source("action @a (t:wire mut) () (m:region[63,1] in) () ()\n^entry():\nx t\nx t\n")
        cert, ch, _h = cert_for(src, "a", [{
            "kind": "exhaustive", "subject": {"closure_hash": None, "variant": "v", "inherited_controls": 0},
            "invocation": {"args": [], "budget": ["0", "0", "0", "0"]},
            "domain": {"bundles": [rng("t", 1)], "borrowed": [], "regions": [{"name": "m", "cells": "all"}]},
            "expected": {"table": []}}])
        cert["regions"][ch]["claims"][0]["subject"]["closure_hash"] = ch
        c = claim_of(src, cert, ch)
        self.assertEqual((c["outcome"], c["section"]), ("unchecked", "12.2"), c)


class InversePair(unittest.TestCase):
    TEXT = EX[7] if 7 in EX else None

    def build(self, ginv="ginv", mutate=None, g="g", subject=None):
        h = hashes.compute_hashes(parse_object(self.TEXT))
        claim = {"kind": "exhaustive", "expected": "inverse_pair",
                 "subject": {"closure_hash": h[subject or g][1], "variant": "forward", "inherited_controls": 0},
                 "domain": {"args": [rng("a", 2)], "borrowed": [], "bundles": [rng("x", 2), rng("y", 2)], "regions": []}}
        entry = {"g": h[g][1], "ginv": h[ginv][1], "claim": 0, "budget": ["3", "0", "0", "0"]}
        ver = {"inverse_pairs": [entry]}
        cert, ch, _ = cert_for(self.TEXT, "addya", [claim], ver)
        if mutate:
            mutate(claim, entry, ver)
        return claim_of(self.TEXT, cert, ch)

    def test_example7_is_proof_over_64(self):
        c = self.build()
        self.assertEqual((c["outcome"], c["elements"]), ("proof", 64), c)

    def test_swapped_orientation_also_covers(self):
        self.assertEqual(self.build(g="ginv", ginv="g")["outcome"], "proof")

    def test_wrong_inverse_rejects_with_witness(self):
        c = self.build(ginv="g")
        reject_cited(self, c, "E_REPLAY_INVERSE")
        self.assertIn("witness", c["errors"][0])

    def test_budget_too_small_faults(self):
        c = self.build(mutate=lambda cl, e, v: e.update(budget=["2", "0", "0", "0"]))
        reject_cited(self, c, "E_REPLAY_INVERSE")
        self.assertIn("faults", c["errors"][0]["message"])

    def test_field_mutations(self):
        h = hashes.compute_hashes(parse_object(self.TEXT))
        muts = {
            "unindexed": lambda cl, e, v: v.update(inverse_pairs=[]),
            "subject_not_g": lambda cl, e, v: cl["subject"].update(closure_hash=h["ginv"][1]),
            "partial_x": lambda cl, e, v: cl["domain"]["bundles"][0].update(hi=2),
            "omitted_ins": lambda cl, e, v: cl["domain"]["bundles"].pop(),
            "target_slot": lambda cl, e, v: cl["domain"]["bundles"].append(rng("t", 2)),
            "args_missing": lambda cl, e, v: cl["domain"].pop("args"),
            "args_unknown": lambda cl, e, v: cl["domain"]["args"][0].update(name="zz"),
            "args_width": lambda cl, e, v: cl["domain"]["args"][0].update(width=3),
            "borrowed": lambda cl, e, v: cl["domain"].update(borrowed=[rng("b", 1)]),
            "budget_short": lambda cl, e, v: e.update(budget=["3", "0", "0"]),
            "budget_memoryless": lambda cl, e, v: e.update(budget=["3", "1", "0", "0"]),
            "ginv_unknown": lambda cl, e, v: e.update(ginv="0" * 64),
            "g_unknown": lambda cl, e, v: e.update(g="0" * 64),
            "unknown_entry_field": lambda cl, e, v: e.update(x=1),
            "arg_tokens_spurious": lambda cl, e, v: e.update(arg_tokens={"a": ["1", "0", "0", "0"]}),
            "inherited": lambda cl, e, v: cl["subject"].update(inherited_controls=1),
            "invocation": lambda cl, e, v: cl.update(invocation={"args": ["0"], "budget": ["3", "0", "0", "0"]}),
            "label_invalid": lambda cl, e, v: cl.update(label=1),
        }
        for name, fn in muts.items():
            with self.subTest(name=name):
                reject_cited(self, self.build(mutate=fn))

    def test_args_slice_reports_declared_elements(self):
        c = self.build(mutate=lambda cl, e, v: cl["domain"]["args"][0].update(lo=1, hi=2))
        self.assertEqual((c["outcome"], c["elements"]), ("proof", 32), c)


class QinplaceCoverage(unittest.TestCase):
    """§4.5: every qinplace of a certified region is carried by a covering inverse_pairs entry."""
    TEXT = EX[7] if 7 in EX else None

    def verdict(self, entry_mut=None, claim_mut=None, root_inv=None, entries=None):
        h = hashes.compute_hashes(parse_object(self.TEXT))
        claim = {"kind": "exhaustive", "expected": "inverse_pair",
                 "subject": {"closure_hash": h["g"][1], "variant": "forward", "inherited_controls": 0},
                 "domain": {"args": [rng("a", 2)], "borrowed": [], "bundles": [rng("x", 2), rng("y", 2)], "regions": []}}
        entry = {"g": h["g"][1], "ginv": h["ginv"][1], "claim": 0, "budget": ["3", "0", "0", "0"]}
        ver = {"inverse_pairs": [entry] if entries is None else entries}
        cert, ch, _ = cert_for(self.TEXT, "addya", [claim], ver)
        if entry_mut:
            entry_mut(entry)
        if claim_mut:
            claim_mut(claim)
        if root_inv is not None:
            cert["bindings"]["invocation"] = root_inv
        v = check_certificate(cert, parse_object(self.TEXT), self.TEXT)
        return v, v["regions"][ch]

    def inv(self, tk=("3", "0", "0", "0"), a="1"):
        return {"args": [list(tk), a], "budget": ["3", "0", "0", "0"]}

    def test_root_invocation_covered(self):
        v, r = self.verdict(root_inv=self.inv())
        self.assertEqual(r["errors"], [], r)
        self.assertEqual([s["status"] for s in r["qinplace_coverage"]], ["covered"], r)
        self.assertEqual(v["overall"], "replayed", v)

    def test_larger_tuple_is_covered(self):
        _v, r = self.verdict(root_inv=self.inv(tk=("9", "0", "0", "0")))
        self.assertEqual(r["qinplace_coverage"][0]["status"], "covered", r)

    def test_tuple_below_entry_budget_rejects(self):
        v, r = self.verdict(root_inv=self.inv(tk=("2", "0", "0", "0")))
        self.assertEqual(r["errors"][0]["code"], "E_REPLAY_COVERAGE", r)
        self.assertIn("4.5", r["errors"][0]["section"])
        self.assertEqual(v["overall"], "reject")

    def test_arg_outside_claim_args_domain_rejects(self):
        _v, r = self.verdict(root_inv=self.inv(a="3"),
                             claim_mut=lambda c: c["domain"]["args"][0].update(lo=0, hi=2))
        self.assertEqual(r["errors"][0]["code"], "E_REPLAY_COVERAGE", r)

    def test_swapped_entry_orientation_covers(self):
        h = hashes.compute_hashes(parse_object(self.TEXT))
        _v, r = self.verdict(root_inv=self.inv(),
                             entry_mut=lambda e: e.update(g=h["ginv"][1], ginv=h["g"][1]),
                             claim_mut=lambda c: c["subject"].update(closure_hash=h["ginv"][1]))
        self.assertEqual(r["qinplace_coverage"][0]["status"], "covered", r)

    def test_no_entry_for_the_pair_rejects(self):
        h = hashes.compute_hashes(parse_object(self.TEXT))
        _v, r = self.verdict(entry_mut=lambda e: e.update(ginv=h["g"][1]))
        codes = [e["code"] for e in r["errors"]]
        self.assertIn("E_REPLAY_COVERAGE", codes, r)
        self.assertEqual(r["qinplace_coverage"][0]["status"], "uncovered", r)

    def test_no_inverse_pairs_at_all_rejects(self):
        _v, r = self.verdict(entries=[])
        self.assertIn("E_REPLAY_COVERAGE", [e["code"] for e in r["errors"]], r)

    def test_unbound_tuple_is_unchecked_never_covered(self):
        v, r = self.verdict()
        self.assertEqual(r["errors"], [], r)
        self.assertEqual(r["qinplace_coverage"][0]["status"], "unresolved", r)
        self.assertTrue(r["unchecked"] and r["unchecked"][0]["section"], r)
        self.assertEqual(v["overall"], "unchecked", v)
