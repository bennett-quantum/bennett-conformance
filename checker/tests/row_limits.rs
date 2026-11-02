mod common;
use bennett_cert_checker::eval::{eval_oracle_bounded, Classical, EvalError};
use bennett_cert_checker::object::Object;
use common::*;
use num_bigint::BigUint;

fn spin() -> Object {
    let (inp, _) = standalone(
        "oracle @spin (x:wire in) (t:wire) (f:wire[4]) () ()\n^entry():\nqjump ^entry",
        r#"{"spin":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#,
        "spin",
        vec![],
    );
    Object::parse(&inp.object_text).unwrap()
}
#[test]
fn huge_legal_oracle_budget_stops_at_the_host_row_limit() {
    let object = spin();
    let budget = [
        BigUint::from(1u32) << 200,
        0u32.into(),
        0u32.into(),
        0u32.into(),
    ];
    let result = eval_oracle_bounded(
        &object,
        object.symbol("spin").unwrap(),
        &[0u32.into()],
        &[],
        &[],
        &budget,
        0,
        4,
    );
    assert!(
        matches!(result,Err(EvalError::Unchecked(s)) if s.contains("host limit") && s.contains("row"))
    );
}
#[test]
fn isa_budget_exhaustion_at_the_host_boundary_remains_a_fault() {
    let object = spin();
    let budget = [4u32.into(), 0u32.into(), 0u32.into(), 0u32.into()];
    let result = eval_oracle_bounded(
        &object,
        object.symbol("spin").unwrap(),
        &[0u32.into()],
        &[],
        &[],
        &budget,
        0,
        4,
    )
    .unwrap();
    assert_eq!(result.code, BigUint::from(1u32));
}
#[test]
fn nested_oracle_calls_share_the_host_work_counter() {
    let source="oracle @outer (x:wire in) (t:wire) (f:wire[4]) () (tk:token)\nregs { tmp:reg[1], flag:reg[4] }\n^entry():\nlift.oracle_call @spin tk (x) tmp flag () ()\nlift.retval tmp\noracle @spin (x:wire in) (t:wire) (f:wire[4]) () ()\n^entry():\nqjump ^entry";
    let sig = r#"{"outer":{"abi":"0.9","args":["token"],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"},"spin":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#;
    let (inp, _) = standalone(source, sig, "outer", vec![]);
    let object = Object::parse(&inp.object_text).unwrap();
    let budget = [100u32.into(), 0u32.into(), 0u32.into(), 0u32.into()];
    let token = Classical::Token(budget.clone());
    let result = eval_oracle_bounded(
        &object,
        object.symbol("outer").unwrap(),
        &[0u32.into()],
        &[],
        &[token],
        &budget,
        0,
        4,
    );
    assert!(
        matches!(result,Err(EvalError::Unchecked(s)) if s.contains("host limit") && s.contains("row"))
    );
}

#[test]
fn host_row_limit_discharges_no_proof_request_or_evidence_count() {
    use bennett_cert_checker::cert::check_certificate;
    use bennett_cert_checker::json::{jcs, Value};
    let claim = json(
        r#"{"kind":"exhaustive","label":"proof","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[],"budget":["1606938044258990275541962092341162602522202993782792835301376","0","0","0"]},"expected":{"program":"0000000000000000000000000000000000000000000000000000000000000000"}}"#,
    );
    let (mut inp, h) = standalone(
        "oracle @spin (x:wire in) (t:wire) (f:wire[4]) () ()\n^entry():\nqjump ^entry",
        r#"{"spin":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#,
        "spin",
        vec![claim],
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let Value::Obj(region) = regions.get_mut(&h).unwrap() else {
        unreachable!()
    };
    let Value::Arr(claims) = region.get_mut("claims").unwrap() else {
        unreachable!()
    };
    set(
        &mut claims[0],
        "expected",
        json(&format!(r#"{{"program":"{h}"}}"#)),
    );
    inp.cert_bytes = jcs(&cert).unwrap();
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "unchecked");
    assert!(claim
        .unchecked_reason
        .as_ref()
        .unwrap()
        .contains("host limit"));
    assert!(
        claim.elements.is_none() && claim.draws.is_none() && claim.covered_fraction.is_none(),
        "{claim:?}"
    );
}
