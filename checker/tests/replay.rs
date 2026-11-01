mod common;
use bennett_cert_checker::cert::{check_certificate, ClaimOutcome, Inputs};
use bennett_cert_checker::json::{jcs, Value};
use common::*;

const SIGS: &str = r#"{"cnot":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"},"identity":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#;
const FUNCS: &str = "perm_action @cnot (t:wire mut, u:wire mut) () () () ()\n^entry():\nb = qborrow fast 1\ncx t u\nqreturn b\nperm_action @identity (u:wire mut, t:wire mut) () () () ()\n^entry():\nb = qborrow fast 1\nqreturn b";
fn claim() -> Value {
    json(
        r#"{"domain":{"borrowed":[{"hi":1,"lo":0,"name":"t","width":1}],"bundles":[{"hi":1,"lo":0,"name":"u","width":1}],"regions":[]},"expected":{"program":"0000000000000000000000000000000000000000000000000000000000000000"},"invocation":{"args":[],"budget":["0","0","0","0"]},"kind":"exhaustive"}"#,
    )
}
fn component_fixture(mut claim: Value) -> (Inputs, String) {
    let (mut inp, h) = standalone(FUNCS, SIGS, "cnot", vec![]);
    let object = bennett_cert_checker::object::Object::parse(&inp.object_text).unwrap();
    set(
        &mut claim,
        "expected",
        json(&format!(
            r#"{{"program":"{}"}}"#,
            object
                .symbol("identity")
                .unwrap()
                .closure_hash
                .as_ref()
                .unwrap()
        )),
    );
    set(
        &mut claim,
        "subject",
        json(&format!(
            r#"{{"closure_hash":"{h}","inherited_controls":0,"variant":"forward"}}"#
        )),
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(regions) = cert.as_obj().unwrap().get("regions").unwrap().clone() else {
        unreachable!()
    };
    let mut region = regions[&h].clone();
    set(&mut region, "claims", Value::Arr(vec![claim]));
    let mut regions = Value::Obj(regions);
    set(&mut regions, &h, region);
    set(&mut cert, "regions", regions);
    inp.cert_bytes = jcs(&cert).unwrap();
    (inp, h)
}
fn outcome(inp: &Inputs, h: &str) -> ClaimOutcome {
    let v = check_certificate(inp);
    assert!(v.errors.is_empty(), "{v:?}");
    v.regions
        .into_iter()
        .find(|r| r.closure_hash == h)
        .unwrap()
        .claims
        .remove(0)
        .1
}

#[test]
fn component_coverage_witness_replays_four_tuples_and_preserves_borrow() {
    let (inp, h) = component_fixture(claim());
    let o = outcome(&inp, &h);
    assert_eq!(o.outcome, "evidence", "{o:?}");
    assert_eq!(o.elements.as_deref(), Some("4"));
    assert_eq!(o.covered_fraction.as_deref(), Some("1/2"));
}
#[test]
fn component_coverage_full_range_exposes_first_mismatch_at_four() {
    let mut c = claim();
    let mut d = c.get("domain").unwrap().clone();
    set(
        &mut d,
        "bundles",
        json(r#"[{"hi":1,"lo":0,"name":"t","width":1},{"hi":1,"lo":0,"name":"u","width":1}]"#),
    );
    set(&mut c, "domain", d);
    let (inp, h) = component_fixture(c);
    let o = outcome(&inp, &h);
    assert_eq!(o.outcome, "reject", "{o:?}");
    assert_eq!(o.errors[0].code, "E_CERT_REPLAY");
    assert!(o.errors[0].message.contains("element 4"), "{o:?}");
}
#[test]
fn component_coverage_renaming_borrow_label_changes_nothing() {
    for label in ["t", "b", "arbitrary"] {
        let mut c = claim();
        let mut d = c.get("domain").unwrap().clone();
        set(
            &mut d,
            "borrowed",
            json(&format!(
                r#"[{{"hi":1,"lo":0,"name":"{label}","width":1}}]"#
            )),
        );
        set(&mut c, "domain", d);
        let (inp, h) = component_fixture(c);
        assert_eq!(outcome(&inp, &h).outcome, "evidence");
    }
}
#[test]
fn proof_request_cannot_promote_slice_or_sampled_success() {
    for sampled in [false, true] {
        let mut c = claim();
        set(&mut c, "label", Value::Str("proof".into()));
        if sampled {
            set(&mut c, "kind", Value::Str("sampled".into()));
            set(
                &mut c,
                "sampler",
                json(r#"{"algorithm":"xoshiro256**","count":8,"seed":["1","0","0","0"]}"#),
            );
        }
        let (inp, h) = component_fixture(c);
        let o = outcome(&inp, &h);
        assert_eq!(o.outcome, "reject", "{o:?}");
        assert_eq!(o.errors[0].code, "E_CERT_COVERAGE_LABEL");
    }
}
#[test]
fn replay_limit_reports_no_count_or_coverage() {
    let (mut inp, h) = component_fixture(claim());
    inp.limits.max_elements = 3;
    let o = outcome(&inp, &h);
    assert_eq!(o.outcome, "unchecked");
    assert!(o.unchecked_reason.unwrap().contains("host limit"));
    assert!(o.elements.is_none());
    assert!(o.covered_fraction.is_none());
}
#[test]
fn controlled_fixture_replays_entire_truth_table() {
    let inp = controlled();
    let v = check_certificate(&inp);
    let root = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("controlled"))
        .unwrap();
    let o = &root.claims[0].1;
    assert_eq!(o.outcome, "proof", "{o:?}");
    assert_eq!(o.elements.as_deref(), Some("4"));
    assert_eq!(o.covered_fraction.as_deref(), Some("1/1"));
}

#[test]
fn matching_table_does_not_license_mutation_of_input_bundle() {
    let c = json(
        r#"{"domain":{"borrowed":[],"bundles":[{"hi":1,"lo":0,"name":"t","width":1},{"hi":1,"lo":0,"name":"c","width":1}],"regions":[]},"expected":{"table":[{"before":{"bundles":["0","0"],"borrowed":[],"regions":[]},"after":{"bundles":["0","1"],"borrowed":[],"regions":[]}},{"before":{"bundles":["0","1"],"borrowed":[],"regions":[]},"after":{"bundles":["0","0"],"borrowed":[],"regions":[]}},{"before":{"bundles":["1","0"],"borrowed":[],"regions":[]},"after":{"bundles":["1","1"],"borrowed":[],"regions":[]}},{"before":{"bundles":["1","1"],"borrowed":[],"regions":[]},"after":{"bundles":["1","0"],"borrowed":[],"regions":[]}}]},"invocation":{"args":[],"budget":["0","0","0","0"]},"kind":"exhaustive"}"#,
    );
    let (inp, h) = standalone(
        "perm_action @bad (t:wire mut) (c:wire in) () () ()\n^entry():\nx c",
        r#"{"bad":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "bad",
        vec![c],
    );
    let o = outcome(&inp, &h);
    assert_eq!(o.outcome, "reject", "{o:?}");
    assert_eq!(o.errors[0].code, "E_CERT_REPLAY");
    assert!(o.errors[0].message.contains("input"));
}
#[test]
fn scheduled_root_explicit_invocation_cannot_conflict_with_binding() {
    let mut inp = controlled();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(m) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = m.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let region = regions
        .values_mut()
        .find(|r| r.get("claims").unwrap().as_arr().unwrap().len() == 2)
        .unwrap();
    let Value::Obj(r) = region else {
        unreachable!()
    };
    let Value::Arr(claims) = r.get_mut("claims").unwrap() else {
        unreachable!()
    };
    set(
        &mut claims[0],
        "invocation",
        json(r#"{"args":[],"budget":["1","0","0","0"]}"#),
    );
    inp.cert_bytes = jcs(&cert).unwrap();
    let v = check_certificate(&inp);
    assert!(codes(&v).contains(&"E_CERT_BINDING"), "{v:?}");
}

#[test]
fn nested_action_cannot_hide_a_callees_input_mutation() {
    let (inp,_)=standalone("perm_action @outer (t:wire mut) (c:wire in) () () ()\n^entry():\naction_call @inner (t) (c) () ()\nperm_action @inner (u:wire mut) (p:wire in) () () ()\n^entry():\nx p",r#"{"outer":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"inner":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,"outer",vec![]);
    let object = bennett_cert_checker::object::Object::parse(&inp.object_text).unwrap();
    let iface = object.interface("outer").unwrap();
    let before = bennett_cert_checker::domain::Tuple {
        bundles: vec![0u32.into(), 0u32.into()],
        borrowed: vec![],
        regions: vec![],
        args: vec![],
    };
    let invocation = bennett_cert_checker::cert::parse_invocation(
        &json(r#"{"args":[],"budget":["0","0","0","0"]}"#),
        &[],
        &object,
        "test",
    )
    .unwrap();
    let after = bennett_cert_checker::eval::eval_subject_tuple(
        &object,
        object.symbol("outer").unwrap(),
        &iface,
        &before,
        &invocation,
        0,
    );
    assert!(
        matches!(after, Err(bennett_cert_checker::eval::EvalError::Reject(_))),
        "{after:?}"
    );
}

#[test]
fn arbitrary_precision_sample_count_is_host_limited_not_illegal() {
    let mut c = claim();
    set(&mut c, "kind", Value::Str("sampled".into()));
    set(
        &mut c,
        "sampler",
        json(
            r#"{"algorithm":"xoshiro256**","count":"18446744073709551616","seed":["1","0","0","0"]}"#,
        ),
    );
    let (inp, h) = component_fixture(c);
    let o = outcome(&inp, &h);
    assert_eq!(o.outcome, "unchecked", "{o:?}");
    assert!(o.unchecked_reason.unwrap().contains("host limit"));
    assert!(o.elements.is_none());
}
