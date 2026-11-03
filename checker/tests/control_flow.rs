mod common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::Value,
};
use common::*;

fn table_claim(widths: &[(&str, u32)], rows: Vec<Value>) -> Value {
    let bundles = widths
        .iter()
        .map(|(name, width)| {
            json(&format!(
                r#"{{"name":"{name}","width":{width},"lo":0,"hi":{}}}"#,
                (1u32 << width) - 1
            ))
        })
        .collect();
    let mut domain = json(r#"{"bundles":[],"borrowed":[],"regions":[]}"#);
    set(&mut domain, "bundles", Value::Arr(bundles));
    let mut claim = json(
        r#"{"kind":"exhaustive","label":"proof","invocation":{"args":[],"budget":["0","0","0","0"]},"domain":{},"expected":{"table":[]}}"#,
    );
    set(&mut claim, "domain", domain);
    set(&mut claim, "expected", {
        let mut e = json("{}");
        set(&mut e, "table", Value::Arr(rows));
        e
    });
    claim
}
fn row(before: &[u32], after: &[u32]) -> Value {
    let tuple = |values: &[u32]| {
        let mut t = json(r#"{"bundles":[],"borrowed":[],"regions":[]}"#);
        set(
            &mut t,
            "bundles",
            Value::Arr(values.iter().map(|v| Value::Str(v.to_string())).collect()),
        );
        t
    };
    let mut r = json("{}");
    set(&mut r, "before", tuple(before));
    set(&mut r, "after", tuple(after));
    r
}
fn nested(outer: u32, inner: u32, continuation: bool, wrong: bool) -> Inputs {
    let suffix = if continuation { "\nx t" } else { "" };
    let source=format!("perm_action @a (t:wire mut) (c:wire[{outer}] in, d:wire[{inner}] in) () () ()\nregs {{  }} bounds {{{outer}, {inner}}}\n^entry():\nqloop {outer} while c {{\nqloop {inner} while d {{\nx t\n}}{suffix}\n}}");
    let mut rows = vec![];
    for t in 0..2 {
        for c in 0..1 << outer {
            for d in 0..1 << inner {
                let prefix = |bits: u32, width: u32| {
                    (0..width).take_while(|i| bits & (1 << i) != 0).count() as u32
                };
                let toggle = if wrong {
                    d & 1
                } else {
                    (prefix(c, outer) & 1) * ((prefix(d, inner) & 1) ^ u32::from(continuation))
                };
                rows.push(row(&[t, c, d], &[t ^ toggle, c, d]));
            }
        }
    }
    standalone(&source,&format!(r#"{{"a":{{"abi":"0.9","args":[],"bounds":[{outer},{inner}],"in":[{outer},{inner}],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}}}"#),"a",vec![table_claim(&[("t",1),("c",outer),("d",inner)],rows)]).0
}
#[test]
fn nested_while_flags_preserve_disabled_outer_predicates_and_continuations() {
    for (outer, inner) in [(1, 1), (3, 2)] {
        for continuation in [false, true] {
            let verdict = check_certificate(&nested(outer, inner, continuation, false));
            assert!(
                codes(&verdict).is_empty(),
                "{outer}/{inner}, continuation={continuation}: {verdict:?}"
            );
            let claim = &verdict.regions[0].claims[0].1;
            assert_eq!(claim.outcome, "proof", "{verdict:?}");
            assert_eq!(
                claim.elements.as_deref(),
                Some((1u32 << (1 + outer + inner)).to_string().as_str())
            );
        }
    }
}
#[test]
fn nested_while_cannot_certify_the_reviewed_wrong_inner_only_relation() {
    let verdict = check_certificate(&nested(1, 1, false, true));
    assert!(codes(&verdict).contains(&"E_CERT_REPLAY"), "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "reject");
}

#[test]
fn qif_executes_both_arms_unconditional_lifetimes_and_predicates_only_gates() {
    for (body, mode) in [
        ("qif c {\nd = qalloc any 1\n} {\nqfree d\n}", 0),
        ("qif c {\nd = qalloc any 1\nx t\n} {\nqfree d\n}", 1),
        (
            "qpred e\nqif c {\nd = qalloc any 1\nx t\n} {\nx t\nqfree d\n}\nqunpred e",
            2,
        ),
    ] {
        let mut rows = vec![];
        for t in 0..2 {
            for c in 0..2 {
                for e in 0..2 {
                    let toggle = match mode {
                        0 => 0,
                        1 => c,
                        _ => e,
                    };
                    rows.push(row(&[t, c, e], &[t ^ toggle, c, e]));
                }
            }
        }
        let source =
            format!("action @a (t:wire mut) (c:wire in, e:wire in) () () ()\n^entry():\n{body}");
        let (inputs, _) = standalone(
            &source,
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1,1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
            "a",
            vec![table_claim(&[("t", 1), ("c", 1), ("e", 1)], rows)],
        );
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).is_empty(), "mode={mode}: {verdict:?}");
        assert_eq!(
            verdict.regions[0].claims[0].1.outcome, "proof",
            "{verdict:?}"
        );
    }
}

fn sampled_identity(rows: Vec<Value>) -> Inputs {
    let mut claim = table_claim(&[("t", 1)], rows);
    set(&mut claim, "kind", Value::Str("sampled".into()));
    set(&mut claim, "label", Value::Str("evidence".into()));
    set(
        &mut claim,
        "sampler",
        json(r#"{"algorithm":"xoshiro256**","seed":["1","0","0","0"],"count":1}"#),
    );
    standalone("perm_action @a (t:wire mut) () () () ()\n^entry():",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,"a",vec![claim]).0
}
#[test]
fn sampled_tables_cover_the_declared_domain_while_execution_covers_only_draws() {
    let mut inputs = sampled_identity(vec![row(&[0], &[0]), row(&[1], &[1])]);
    inputs.limits.max_elements = 1; // Bounds draws, not declared-table completeness.
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "evidence", "{verdict:?}");
    assert_eq!(claim.draws.as_deref(), Some("1"));
    assert_eq!(claim.distinct.as_deref(), Some("1"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/2"));
    // A legal but false output at the undrawn point is not checked by this
    // sample; the verdict must continue to describe only the one drawn point.
    let verdict = check_certificate(&sampled_identity(vec![row(&[0], &[0]), row(&[1], &[0])]));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "evidence");
}
#[test]
fn sampled_tables_reject_missing_duplicate_reordered_and_malformed_undrawn_rows() {
    for rows in [
        vec![row(&[0], &[0])],
        vec![row(&[0], &[0]), row(&[0], &[0])],
        vec![row(&[1], &[1]), row(&[0], &[0])],
        vec![row(&[0], &[0]), row(&[1], &[2])],
    ] {
        let verdict = check_certificate(&sampled_identity(rows));
        assert!(codes(&verdict).contains(&"E_CERT_TABLE"), "{verdict:?}");
    }
}

#[test]
fn every_sampler_field_and_seed_position_rejects_bad_types_and_removals() {
    let base = sampled_identity(vec![row(&[0], &[0]), row(&[1], &[1])]);
    let mut rejected = 0;
    for remove in [false, true] {
        for field in 0..8 {
            let mut cert = json(std::str::from_utf8(&base.cert_bytes).unwrap());
            let Value::Obj(top) = &mut cert else { panic!() };
            let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
                panic!()
            };
            let Value::Obj(record) = regions.values_mut().next().unwrap() else {
                panic!()
            };
            let Value::Arr(claims) = record.get_mut("claims").unwrap() else {
                panic!()
            };
            let Value::Obj(claim) = &mut claims[0] else {
                panic!()
            };
            if field == 0 {
                if remove {
                    claim.remove("sampler");
                } else {
                    claim.insert("sampler".into(), Value::Null);
                }
            } else {
                let Value::Obj(sampler) = claim.get_mut("sampler").unwrap() else {
                    panic!()
                };
                if field <= 3 {
                    let key = ["algorithm", "seed", "count"][field - 1];
                    if remove {
                        sampler.remove(key);
                    } else {
                        sampler.insert(key.into(), Value::Null);
                    }
                } else {
                    let Value::Arr(seed) = sampler.get_mut("seed").unwrap() else {
                        panic!()
                    };
                    if remove {
                        seed.remove(field - 4);
                    } else {
                        seed[field - 4] = Value::Null;
                    }
                }
            }
            let mut inputs = base.clone();
            inputs.cert_bytes = bennett_cert_checker::json::jcs(&cert).unwrap();
            let verdict = check_certificate(&inputs);
            let code = if remove && field == 0 {
                "E_CERT_SCHEMA"
            } else {
                "E_CERT_SAMPLER"
            };
            assert!(
                codes(&verdict).contains(&code),
                "remove={remove}, field={field}: {verdict:?}"
            );
            rejected += 1;
        }
    }
    assert_eq!(rejected, 16);
}

#[test]
fn while_selectors_cannot_be_modified_even_if_the_claim_matches_a_noninjective_map() {
    for (body, kind) in [
        ("x c", 0),
        ("swap c t", 1),
        ("qxor t c", 2),
        ("action_call @flip (c) () () ()", 0),
    ] {
        let mut source=format!("perm_action @a (t:wire mut, c:wire mut) () () () ()\nregs {{  }} bounds {{1}}\n^entry():\nqloop 1 while c {{\n{body}\n}}");
        let mut signatures = json(
            r#"{"a":{"abi":"0.9","args":[],"bounds":[1],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#,
        );
        if body.starts_with("action_call") {
            source.push_str("\nperm_action @flip (x:wire mut) () () () ()\n^entry():\nx x");
            set(
                &mut signatures,
                "flip",
                json(
                    r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}"#,
                ),
            );
        }
        let mut rows = vec![];
        for t in 0..2 {
            for c in 0..2 {
                let after = match kind {
                    0 => vec![t, 0],
                    1 => vec![t | c, t & c],
                    _ => vec![t, c & (1 - t)],
                };
                rows.push(row(&[t, c], &after));
            }
        }
        let (inputs, _) = standalone(
            &source,
            &encode(&signatures),
            "a",
            vec![table_claim(&[("t", 1), ("c", 1)], rows)],
        );
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_OBJECT"),
            "{body}: {verdict:?}"
        );
    }
}

#[test]
fn selector_guards_allow_structural_qif_flips_and_end_with_the_loop() {
    let source="perm_action @a (t:wire mut, c:wire mut) () () () ()\nregs {  } bounds {1}\n^entry():\nqloop 1 while c {\nqif c {\nx t\n} {\nx t\n}\n}\nx c";
    let rows = (0..2)
        .flat_map(|t| (0..2).map(move |c| row(&[t, c], &[t ^ c, c ^ 1])))
        .collect();
    let (inputs, _) = standalone(
        source,
        r#"{"a":{"abi":"0.9","args":[],"bounds":[1],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#,
        "a",
        vec![table_claim(&[("t", 1), ("c", 1)], rows)],
    );
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
}

#[test]
fn selector_guard_follows_callee_reads_without_treating_a_mutable_formal_as_a_write() {
    for indexed in [false, true] {
        for read_only_gate in [false, true] {
            let gate = if read_only_gate { "\ncx c t" } else { "" };
            let width = if indexed { 2 } else { 1 };
            let actual = if indexed { "t[0], c[0]" } else { "t, c" };
            let source=format!("perm_action @a (t:wire[{width}] mut, c:wire mut) () () () ()\nregs {{  }} bounds {{1}}\n^entry():\nqloop 1 while c {{\naction_call @reader ({actual}) () () ()\n}}\nx c\nperm_action @reader (t:wire mut, c:wire mut) () () () ()\n^entry():{gate}");
            let mut rows = vec![];
            for t in 0..1 << width {
                for c in 0..2 {
                    rows.push(row(
                        &[t, c],
                        &[t ^ if read_only_gate { c } else { 0 }, c ^ 1],
                    ));
                }
            }
            let signatures = format!(
                r#"{{"a":{{"abi":"0.9","args":[],"bounds":[1],"in":[],"kind":"perm_action","mem":[],"mut":[{width},1],"tier":"q"}},"reader":{{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}}}"#
            );
            let (inputs, _) = standalone(
                &source,
                &signatures,
                "a",
                vec![table_claim(&[("t", width), ("c", 1)], rows)],
            );
            let verdict = check_certificate(&inputs);
            assert!(codes(&verdict).is_empty(), "{verdict:?}");
            assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
        }
    }
}
#[test]
fn action_call_width_mismatch_cannot_certify_truncation_even_when_disabled() {
    let source="perm_action @a (v:wire[2] mut) (c:wire in) () () ()\nregs {  } bounds {1}\n^entry():\nqloop 1 while c {\naction_call @flip (v) () () ()\n}\nperm_action @flip (x:wire mut) () () () ()\n^entry():\nx x";
    let rows = (0..4)
        .flat_map(|v| (0..2).map(move |c| row(&[v, c], &[(v & 1) ^ c, c])))
        .collect();
    let (inputs, _) = standalone(
        source,
        r#"{"a":{"abi":"0.9","args":[],"bounds":[1],"in":[1],"kind":"perm_action","mem":[],"mut":[2],"tier":"q"},"flip":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a",
        vec![table_claim(&[("v", 2), ("c", 1)], rows)],
    );
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
}
#[test]
fn action_call_aliases_cannot_certify_a_noninjective_relation() {
    for actual in ["c, c", "c, c[0]"] {
        let source=format!("perm_action @a (c:wire mut) () () () ()\n^entry():\naction_call @two ({actual}) () () ()\nperm_action @two (x:wire mut, y:wire mut) () () () ()\n^entry():\ncx x y");
        let rows = vec![row(&[0], &[0]), row(&[1], &[0])];
        let (inputs, _) = standalone(
            &source,
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"two":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#,
            "a",
            vec![table_claim(&[("c", 1)], rows)],
        );
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
    }
}

#[test]
fn action_memory_actuals_preserve_origins_and_require_exact_shapes_and_disjointness() {
    use bennett_cert_checker::{
        domain::Tuple,
        eval::{eval_action_tuple, EvalError},
        object::Object,
    };
    for (a, w, duplicate) in [(0, 1, false), (1, 1, false), (0, 2, false), (0, 1, true)] {
        let actual = if duplicate { "r, r" } else { "r" };
        let formal = if duplicate {
            format!("p:region[{a},{w}] mut, q:region[{a},{w}] mut")
        } else {
            format!("p:region[{a},{w}] mut")
        };
        let source=format!("perm_action @a (t:wire mut) () (r:region[0,1] mut) () ()\n^entry():\naction_call @reader (t) () ({actual}) ()\nperm_action @reader (t:wire mut) () ({formal}) () ()\n^entry():");
        let mem = json(&format!(r#"{{"A":{a},"W":{w},"role":"mut"}}"#));
        let mut reader = json(
            r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}"#,
        );
        set(
            &mut reader,
            "mem",
            Value::Arr(vec![mem; if duplicate { 2 } else { 1 }]),
        );
        let mut signatures = json(
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[{"A":0,"W":1,"role":"mut"}],"mut":[1],"tier":"q"}}"#,
        );
        set(&mut signatures, "reader", reader);
        let (inputs, _) = standalone(&source, &encode(&signatures), "a", vec![]);
        let text=rebind_object_symbols(&format!("{}\nsection layout\n{{\"A_tab\":0,\"H\":4,\"S_cells\":0,\"globals\":{{}},\"heap_base\":1,\"heap_limit_max\":0,\"interface_regions\":{{}},\"null_size\":1,\"ptr_width\":32}}\n",inputs.object_text));
        let object = Object::parse(&text).unwrap();
        let symbol = object.symbol("a").unwrap();
        let iface = object.interface("a").unwrap();
        for t in 0..2u32 {
            for cell in 0..2u32 {
                let before = Tuple {
                    bundles: vec![t.into()],
                    borrowed: vec![],
                    regions: vec![vec![cell.into()]],
                    args: vec![],
                };
                let result = eval_action_tuple(&object, symbol, &iface, &before, &[], 0, false, 0);
                if (a, w, duplicate) == (0, 1, false) {
                    assert_eq!(result.unwrap(), before);
                } else {
                    assert!(matches!(result,Err(EvalError::Reject(e))if e.code=="E_CERT_OBJECT"));
                }
            }
        }
    }
}

#[test]
fn qif_lifetime_adjoints_reverse_the_complete_expansion_and_all_blocks() {
    for case in 0..3 {
        let body=match case {
            0=>"^entry():\nqif c {\nd = qalloc any 1\n} {\nqfree d\n}",
            1=>"^entry():\nqif c {\nd = qalloc any 1\nx t\n} {\nqfree d\n}",
            _=>"^entry():\nd = qalloc any 2\ncx t u\n^middle():\nqif c {\nx t\n} {\n}\n^done():\nqfree d",
        };
        let params = if case == 2 {
            "t:wire mut, u:wire mut"
        } else {
            "t:wire mut"
        };
        let actual = if case == 2 { "t, u" } else { "t" };
        let mut source=format!("perm_action @inverse ({params}) (c:wire in) () () ()\n^entry():\nadj action_call @a ({actual}) (c) () ()\nperm_action @a ({params}) (c:wire in) () () ()\n{body}");
        source.push('\n');
        let widths = if case == 2 { "[1,1]" } else { "[1]" };
        let signature = format!(
            r#"{{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":{widths},"tier":"q"}}"#
        );
        let signatures = format!(r#"{{"a":{signature},"inverse":{signature}}}"#);
        let mut rows = vec![];
        for t in 0..2 {
            for u in 0..if case == 2 { 2 } else { 1 } {
                for c in 0..2 {
                    if case == 2 {
                        rows.push(row(&[t, u, c], &[t ^ c, u ^ t ^ c, c]));
                    } else {
                        rows.push(row(&[t, c], &[t ^ if case == 1 { c } else { 0 }, c]));
                    }
                }
            }
        }
        let fields = if case == 2 {
            vec![("t", 1), ("u", 1), ("c", 1)]
        } else {
            vec![("t", 1), ("c", 1)]
        };
        let (inputs, _) = standalone(
            &source,
            &signatures,
            "inverse",
            vec![table_claim(&fields, rows)],
        );
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).is_empty(), "{verdict:?}");
        assert_eq!(
            verdict.regions[0].claims[0].1.outcome, "proof",
            "case={case}: {verdict:?}"
        );
    }
}

#[test]
fn malformed_lifetimes_cannot_hide_dirty_allocations_or_turn_missing_values_into_zero() {
    for body in [
        "d = qalloc any 1\nx d\nd = qalloc any 1\nqfree d",
        "qfree ghost",
        "d = qcopy ghost\nquncopy ghost d",
        "d = qalloc any 0\nqfree d",
        "d = qalloc any 1\nx d\nd = qcopy t\nquncopy t d",
    ] {
        let source=format!("perm_action @a (t:wire mut) () () () ()\n^entry():\n{body}");
        let (inputs,_)=standalone(&source,r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,"a",vec![table_claim(&[("t",1)],vec![row(&[0],&[0]),row(&[1],&[1])])]);
        let verdict=check_certificate(&inputs);assert!(codes(&verdict).contains(&"E_CERT_OBJECT"),"{body}: {verdict:?}");
    }
}
#[test]
fn indexed_copy_sources_and_cleanup_preserve_every_bit_under_both_predicate_values() {
    let source="perm_action @a (t:wire[2] mut) (c:wire in) () () ()\n^entry():\nqpred c\nd = qcopy t[0]\nquncopy t[0] d\nqunpred c";
    let rows=(0..4).flat_map(|t|(0..2).map(move|c|row(&[t,c],&[t,c]))).collect();
    let (inputs,_)=standalone(source,r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[2],"tier":"q"}}"#,"a",vec![table_claim(&[("t",2),("c",1)],rows)]);
    let verdict=check_certificate(&inputs);assert!(codes(&verdict).is_empty(),"{verdict:?}");assert_eq!(verdict.regions[0].claims[0].1.outcome,"proof","{verdict:?}");
}
