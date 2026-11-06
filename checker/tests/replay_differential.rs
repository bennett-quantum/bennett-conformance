mod common;
use bennett_cert_checker::cert::{check_certificate, Inputs, Limits};
use bennett_cert_checker::eval::{eval_oracle, EvalError};
use bennett_cert_checker::json::Value;
use bennett_cert_checker::object::Object;
use common::*;
use num_bigint::BigUint;
use std::path::Path;

// Only the independently supplied objects and tables are consumed here. No
// implementation from checker-replay is linked, copied or inspected.
fn witness(name: &str) -> Inputs {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/replay-differential")
        .join(name);
    Inputs {
        cert_bytes: std::fs::read(dir.join("certificate.json")).unwrap(),
        object_text: std::fs::read_to_string(dir.join("object.bisa")).unwrap(),
        target_bytes: None,
        schedule_bytes: None,
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    }
}
#[test]
fn negative_literal_witness_replays_all_sixteen_rows() {
    let verdict = check_certificate(&witness("const-negative-spelling"));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "evidence", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("16"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/256"));
}

#[test]
fn heap_store_load_witness_replays_all_sixteen_rows() {
    let verdict = check_certificate(&witness("oracle-store-load-heap"));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "evidence", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("16"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/256"));
}

#[test]
fn recursive_lproc_witness_replays_all_sixteen_rows() {
    let verdict = check_certificate(&witness("lproc-recursive-sum"));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "evidence", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("16"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/256"));
}
#[test]
fn arithmetic_shift_witness_replays_all_256_rows() {
    let verdict = check_certificate(&witness("ashr-shift-ge-n"));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "evidence", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("256"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/256"));
}
#[test]
fn mcx_bundle_controls_expand_every_bit_under_k93() {
    let inputs = witness("mcx-bundle-control");
    let object = Object::parse(&inputs.object_text).unwrap();
    assert_eq!(
        object.object_hash,
        "db3d3f92a79077074901449d83532f1a0d82ef1621bd833adb7ad14d1aedd467"
    );
    assert_eq!(
        object.symbol("a").unwrap().closure_hash.as_deref(),
        Some("e38d5da64f9a0ae89a66243bcbe5a321cb807a5bbabf83952b63d3465dfab346")
    );
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "proof", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("16"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/1"));
}
#[test]
fn explicit_mcx_bundle_bits_replay_the_complete_sixteen_row_table() {
    let inp = witness("mcx-bundle-control");
    let cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let region = cert
        .get("regions")
        .unwrap()
        .as_obj()
        .unwrap()
        .values()
        .next()
        .unwrap();
    let mut claim = region.get("claims").unwrap().as_arr().unwrap()[0].clone();
    // standalone rebinds the claim subject to the changed object.
    if let Value::Obj(m) = &mut claim {
        m.remove("subject");
    }
    let sig = region.get("signature").unwrap();
    let functions = Object::parse(&inp.object_text)
        .unwrap()
        .function("a")
        .unwrap()
        .bytes
        .replace("mcx (c) p", "mcx (c[0], c[1]) p");
    let (inp, _) = standalone(
        &functions,
        &format!("{{\"a\":{}}}", encode(sig)),
        "a",
        vec![claim],
    );
    let verdict = check_certificate(&inp);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "proof", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("16"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/1"));
}

fn literal_outcome(literal: &str) -> Result<bennett_cert_checker::eval::OracleOutcome, EvalError> {
    let source = "oracle @o () (t:wire[4]) (f:wire[4]) () ()\nregs { c:reg[4] }\n^entry():\nc = lift.const.i4 0\nlift.retval c";
    let (inp, _) = standalone(
        source,
        r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":4,"tier":"q"}}"#,
        "o",
        vec![],
    );
    let text = inp
        .object_text
        .replace("lift.const.i4 0", &format!("lift.const.i4 {literal}"));
    let object = Object::parse(&text).map_err(|e| {
        EvalError::Reject(bennett_cert_checker::fault::Reject::new(
            "E_CERT_OBJECT",
            "§16.1",
            "object",
            e,
        ))
    })?;
    eval_oracle(
        &object,
        object.symbol("o").unwrap(),
        &[],
        &[],
        &[],
        &[2u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
        0,
    )
}
#[test]
fn every_legal_four_bit_constant_spelling_has_its_specified_bits() {
    for literal in -8i32..16 {
        let outcome = literal_outcome(&literal.to_string()).unwrap();
        assert_eq!(outcome.code, BigUint::from(0u32));
        assert_eq!(
            outcome.payload,
            BigUint::from(literal.rem_euclid(16) as u32),
            "{literal}"
        );
    }
}
#[test]
fn invalid_constant_literals_never_default_to_zero_or_wrap() {
    for literal in ["-9", "16", "032", "+3", "no_such_value"] {
        let outcome = literal_outcome(literal);
        assert!(
            matches!(outcome, Err(EvalError::Reject(_))),
            "{literal}: expected malformed literal rejection"
        );
    }
}

#[test]
fn lifted_constants_are_not_limited_to_the_classical_128_bit_widths() {
    for width in [129, 256, 1024, 1_000_001] {
        let source=format!("oracle @o () (t:wire) (f:wire[4]) () ()\nregs {{ c:reg[{width}], r:reg[1] }}\n^entry():\nc = lift.const.i{width} -1\nr = lift.trunc.i1 c\nlift.retval r");
        let (inputs, _) = standalone(
            &source,
            r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#,
            "o",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let result = eval_oracle(
            &object,
            object.symbol("o").unwrap(),
            &[],
            &[],
            &[],
            &[3u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
            0,
        );
        if width > 1_000_000 {
            assert!(
                matches!(result,Err(EvalError::Unchecked(reason))if reason.contains("host limit"))
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result.payload, BigUint::from(1u32));
            assert_eq!(result.code, BigUint::from(0u32));
        }
    }
}

#[test]
fn lifted_parallel_moves_capture_all_sources_before_any_write() {
    let source="oracle @o (x:wire[4] in) (t:wire[4]) (f:wire[4]) () ()\nregs { a:reg[4], b:reg[4], r:reg[4] }\n^entry():\na = lift.const.i4 1\nb = lift.const.i4 2\n(a, b) = lift.parmov (b, a)\nr = lift.sub b a\nlift.retval r";
    let (inputs, _) = standalone(
        source,
        r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[4],"kind":"oracle","mem":[],"tgt":4,"tier":"q"}}"#,
        "o",
        vec![],
    );
    let object = Object::parse(&inputs.object_text).unwrap();
    let result = eval_oracle(
        &object,
        object.symbol("o").unwrap(),
        &[0u32.into()],
        &[],
        &[],
        &[5u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
        0,
    )
    .unwrap();
    assert_eq!(result.payload, BigUint::from(15u32));
    assert_eq!(result.code, BigUint::from(0u32));
}

#[test]
fn invalid_lifted_destinations_widths_and_labels_reject_instead_of_defaulting() {
    for body in [
        "x = lift.const.i4 0\nlift.retval x",
        "r = lift.label ^missing\nlift.retval r",
        "qjump ^missing",
        "p = lift.const.i1 1\nqbranch p ^missing ^entry",
        "p = lift.add x x\nlift.retval x",
        "p = lift.mov x\nlift.retval x",
        "(r, r) = lift.parmov (x, x)\nlift.retval r",
    ] {
        let source=format!("oracle @o (x:wire[4] in) (t:wire[4]) (f:wire[4]) () ()\nregs {{ r:reg[4], p:reg[1] }}\n^entry():\n{body}");
        let (inputs, _) = standalone(
            &source,
            r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[4],"kind":"oracle","mem":[],"tgt":4,"tier":"q"}}"#,
            "o",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let result = eval_oracle(
            &object,
            object.symbol("o").unwrap(),
            &[3u32.into()],
            &[],
            &[],
            &[10u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
            0,
        );
        assert!(
            matches!(result,Err(EvalError::Reject(ref error))if error.code=="E_CERT_OBJECT"),
            "{body}"
        );
    }
}

#[test]
fn permutation_adjoints_reverse_block_order_on_all_twelve_controlled_basis_inputs() {
    use bennett_cert_checker::domain::Tuple;
    use bennett_cert_checker::eval::eval_action_tuple;
    let (inputs, _) = standalone(
        "perm_action @a (t:wire mut, u:wire mut) () () () ()\n^first():\nx t\n^second():\ncx t u",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#,
        "a",
        vec![],
    );
    let object = Object::parse(&inputs.object_text).unwrap();
    let symbol = object.symbol("a").unwrap();
    let iface = object.interface("a").unwrap();
    let mut cases = 0;
    for control in [None, Some(0u32), Some(1)] {
        for t in 0u32..2 {
            for u in 0u32..2 {
                let bundles = control
                    .into_iter()
                    .chain([t, u])
                    .map(BigUint::from)
                    .collect();
                let before = Tuple {
                    bundles,
                    borrowed: vec![],
                    regions: vec![],
                    args: vec![],
                };
                let inherited = usize::from(control.is_some());
                let forward =
                    eval_action_tuple(&object, symbol, &iface, &before, &[], inherited, false, 0)
                        .unwrap();
                let restored =
                    eval_action_tuple(&object, symbol, &iface, &forward, &[], inherited, true, 0)
                        .unwrap();
                assert_eq!(restored, before, "control={control:?},t={t},u={u}");
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 12);
}

#[test]
fn lifted_fault_causes_preserve_all_declared_bits_and_standard_names() {
    let large: BigUint = BigUint::from(1u32) << 100usize;
    for (body, width, want) in [
        (format!("fault {large}"), 128, large.clone()),
        ("fault 0".into(), 4, BigUint::from(4u32)),
        (
            format!("p = lift.const.i1 1\nlift.faultif p {large}\nlift.retval p"),
            128,
            large,
        ),
        (
            "p = lift.const.i1 1\nlift.faultif p FAULT_ENTROPY_EXHAUSTED\nlift.retval p".into(),
            4,
            BigUint::from(10u32),
        ),
    ] {
        let source = format!(
            "oracle @o () (t:wire) (f:wire[{width}]) () ()\nregs {{ p:reg[1] }}\n^entry():\n{body}"
        );
        let (inputs, _) = standalone(
            &source,
            &format!(
                r#"{{"o":{{"abi":"0.9","args":[],"fault":{width},"in":[],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}}}"#
            ),
            "o",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let result = eval_oracle(
            &object,
            object.symbol("o").unwrap(),
            &[],
            &[],
            &[],
            &[10u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
            0,
        )
        .unwrap();
        assert_eq!(result.code, want, "{body}");
        assert_eq!(result.payload, BigUint::from(0u32));
    }
}

#[test]
fn ill_formed_fault_literals_reject_even_when_the_predicate_is_false() {
    for body in [
        "fault unknown_cause",
        "fault 16",
        "p = lift.const.i1 0\nlift.faultif p 16\nlift.retval p",
    ] {
        let source = format!(
            "oracle @o () (t:wire) (f:wire[4]) () ()\nregs {{ p:reg[1] }}\n^entry():\n{body}"
        );
        let (inputs, _) = standalone(
            &source,
            r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#,
            "o",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let result = eval_oracle(
            &object,
            object.symbol("o").unwrap(),
            &[],
            &[],
            &[],
            &[10u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
            0,
        );
        assert!(
            matches!(result,Err(EvalError::Reject(ref r))if r.code=="E_CERT_OBJECT"),
            "{body}"
        );
    }
}

#[test]
fn definitively_absent_closures_and_wrong_oracle_subject_reject() {
    for name in [
        "unchecked-subject-closure-absent",
        "unchecked-program-absent",
        "unchecked-oracle-on-action",
    ] {
        let verdict = check_certificate(&witness(name));
        assert!(
            codes(&verdict).contains(&"E_CERT_BINDING"),
            "{name}: {verdict:?}"
        );
    }
}

fn edit_root_table(inputs: &mut Inputs, edit: impl FnOnce(&mut Vec<Value>)) {
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let Value::Obj(region) = regions.get_mut(&root).unwrap() else {
        unreachable!()
    };
    let Value::Arr(claims) = region.get_mut("claims").unwrap() else {
        unreachable!()
    };
    let Value::Obj(claim) = &mut claims[0] else {
        unreachable!()
    };
    let Value::Obj(expected) = claim.get_mut("expected").unwrap() else {
        unreachable!()
    };
    let Value::Arr(table) = expected.get_mut("table").unwrap() else {
        unreachable!()
    };
    edit(table);
    inputs.cert_bytes = bennett_cert_checker::json::jcs(&cert).unwrap();
}

#[test]
fn table_schema_and_domain_correspondence_reject_without_a_target() {
    // All 12 mutations are target-independent: rows, tuple shape/encoding,
    // widths, input enumeration, and domain cardinality. Output *values* are
    // deliberately not compared with a reference without realised semantics.
    for mutation in 0..12 {
        let mut inputs = controlled();
        inputs.target_bytes = None;
        edit_root_table(&mut inputs, |rows| match mutation {
            0 => {
                rows.pop();
            }
            1 => rows.push(rows[0].clone()),
            2 => rows.swap(0, 1),
            3 => rows[1] = rows[0].clone(),
            4 => set(&mut rows[0], "extra", Value::Null),
            n => {
                let Value::Obj(row) = &mut rows[0] else {
                    unreachable!()
                };
                let Value::Obj(after) = row.get_mut("after").unwrap() else {
                    unreachable!()
                };
                if n == 5 {
                    after.insert("extra".into(), Value::Null);
                    return;
                }
                let Value::Arr(bundles) = after.get_mut("bundles").unwrap() else {
                    unreachable!()
                };
                match n {
                    6 => bundles[0] = json("0"),
                    7 => bundles[0] = Value::Str("00".into()),
                    8 => {
                        bundles.pop();
                    }
                    9 => bundles.push(Value::Str("0".into())),
                    10 => bundles[0] = Value::Str("2".into()),
                    11 => bundles[0] = Value::Str("-1".into()),
                    _ => unreachable!(),
                }
            }
        });
        let verdict = check_certificate(&inputs);
        let expected = if mutation == 5 {
            "E_CERT_SCHEMA"
        } else {
            "E_CERT_TABLE"
        };
        assert!(
            codes(&verdict).contains(&expected),
            "mutation {mutation}: {verdict:?}"
        );
    }
}

#[test]
fn a_well_formed_table_value_mismatch_needs_the_realised_target() {
    let mut inputs = controlled();
    inputs.target_bytes = None;
    edit_root_table(&mut inputs, |rows| {
        let Value::Obj(row) = &mut rows[0] else {
            unreachable!()
        };
        let Value::Obj(after) = row.get_mut("after").unwrap() else {
            unreachable!()
        };
        let Value::Arr(bundles) = after.get_mut("bundles").unwrap() else {
            unreachable!()
        };
        bundles[0] = Value::Str("1".into());
    });
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let root = verdict
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("controlled"))
        .unwrap();
    assert_eq!(root.claims[0].1.outcome, "unchecked");
}

#[test]
fn coherent_hzh_witness_replays_both_basis_states_with_unit_phase() {
    let verdict = check_certificate(&witness("action-hzh"));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "proof", "{claim:?}");
    assert_eq!(claim.elements.as_deref(), Some("2"));
    assert_eq!(claim.covered_fraction.as_deref(), Some("1/1"));
}

fn coherent_fixture(body: &str, extra: &str, flip: bool) -> (Inputs, String) {
    let domain = json(
        r#"{"bundles":[{"name":"t","width":1,"lo":0,"hi":1},{"name":"c","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#,
    );
    let mut rows = Vec::new();
    for t in 0..2 {
        for c in 0..2 {
            let before = json(&format!(
                r#"{{"bundles":["{t}","{c}"],"borrowed":[],"regions":[]}}"#
            ));
            let after = json(&format!(
                r#"{{"bundles":["{}","{c}"],"borrowed":[],"regions":[]}}"#,
                t ^ u8::from(flip && c == 1)
            ));
            let mut row = json("{}");
            set(&mut row, "before", before);
            set(&mut row, "after", after);
            rows.push(row);
        }
    }
    let mut claim = json(
        r#"{"kind":"exhaustive","label":"proof","invocation":{"args":[],"budget":["0","0","0","0"]}}"#,
    );
    set(&mut claim, "domain", domain);
    let mut expected = json("{}");
    set(&mut expected, "table", Value::Arr(rows));
    set(&mut claim, "expected", expected);
    let sig = r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"action","mem":[],"mut":[1],"tier":"q"},"inner":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#;
    let source = format!("action @a (t:wire mut) (c:wire in) () () ()\n^entry():\n{body}\naction @inner (t:wire mut) () () () ()\n^entry():\n{extra}");
    standalone(&source, sig, "a", vec![claim])
}

#[test]
fn exact_phase_cancellation_controls_and_adjoint_calls_preserve_all_four_inputs() {
    for (body, inner) in [
        ("ctl c h t\nctl c t t\nctl c tdg t\nctl c h t", "x t"),
        ("ctl c action_call @inner (t) () () ()\nctl c h t", "h t"),
        (
            "ctl c action_call @inner (t) () () ()\nctl c adj action_call @inner (t) () () ()",
            "h t\nt t\ns t",
        ),
        ("h t\nt t\nt t\nt t\nt t\nt t\nt t\nt t\nt t\nh t", "x t"),
        ("cz c t\ncz c t\nh t\nh t", "x t"),
        ("qpred c\nctl c h t\nctl c h t\nqunpred c", "x t"),
        ("qif c {\nh t\nh t\n} {\nt t\ntdg t\n}", "x t"),
    ] {
        let (inputs, root) = coherent_fixture(body, inner, false);
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).is_empty(), "{body}: {verdict:?}");
        let claim = &verdict
            .regions
            .iter()
            .find(|r| r.closure_hash == root)
            .unwrap()
            .claims[0]
            .1;
        assert_eq!(claim.outcome, "proof", "{body}: {claim:?}");
        assert_eq!(claim.elements.as_deref(), Some("4"));
    }
}

#[test]
fn coherent_replay_rejects_final_superposition_and_non_unit_basis_phase() {
    for body in ["h t", "s t\ns t", "x t\ns t\ns t\nx t\ns t\ns t"] {
        let (inputs, _) = coherent_fixture(body, "x t", false);
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_REPLAY"),
            "{body}: {verdict:?}"
        );
    }
}

#[test]
fn coherent_replay_obeys_the_shared_work_limit() {
    let (mut inputs, root) = coherent_fixture("h t\nh t\nh t\nh t", "x t", false);
    inputs.limits.max_reference_steps = 3;
    let verdict = check_certificate(&inputs);
    let claim = &verdict
        .regions
        .iter()
        .find(|r| r.closure_hash == root)
        .unwrap()
        .claims[0]
        .1;
    assert_eq!(claim.outcome, "unchecked", "{claim:?}");
    assert!(claim
        .unchecked_reason
        .as_deref()
        .unwrap()
        .contains("host limit"));
    assert!(claim.elements.is_none());
}

#[test]
fn a_nested_perm_action_cannot_hide_nonpermutation_primitives() {
    let source="action @a (t:wire mut) () () () ()\n^entry():\naction_call @bad (t) () () ()\nperm_action @bad (t:wire mut) () () () ()\n^entry():\nh t\nh t";
    let signatures = r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"},"bad":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let claim = json(
        r#"{"kind":"exhaustive","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[],"budget":["0","0","0","0"]},"expected":{"table":[{"before":{"bundles":["0"],"borrowed":[],"regions":[]},"after":{"bundles":["0"],"borrowed":[],"regions":[]}}]}}"#,
    );
    let (inputs, _) = standalone(source, signatures, "a", vec![claim]);
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
}

#[test]
fn coherent_reference_wire_cap_discharges_no_obligation() {
    let source = "action @a (t:wire[11] mut) () () () ()\n^entry():\nh t[0]\nh t[0]";
    let signatures = r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[11],"tier":"q"}}"#;
    let claim = json(
        r#"{"kind":"exhaustive","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[],"budget":["0","0","0","0"]},"expected":{"table":[{"before":{"bundles":["0"],"borrowed":[],"regions":[]},"after":{"bundles":["0"],"borrowed":[],"regions":[]}}]}}"#,
    );
    let (inputs, _) = standalone(source, signatures, "a", vec![claim]);
    let verdict = check_certificate(&inputs);
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "unchecked", "{claim:?}");
    assert!(claim
        .unchecked_reason
        .as_deref()
        .unwrap()
        .contains("host limit"));
    assert!(claim.elements.is_none());
}
