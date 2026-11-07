use crate::common;
use bennett_cert_checker::eval::{eval_oracle_bounded, Classical, EvalError};
use bennett_cert_checker::object::Object;
use common::*;
use num_bigint::BigUint;

#[test]
fn an_oversized_lifted_cast_never_silently_masks_its_result_to_zero() {
    for width in ["1000000", "18446744073709551616"] {
        let source=format!("oracle @o (x:wire in) (t:wire) (f:wire[4]) () ()\nregs {{ wide:reg[{width}], r:reg[1] }}\n^entry():\nwide = lift.zext.i{width} x\nr = lift.trunc.i1 wide\nlift.retval r");
        let (inputs, _) = standalone(
            &source,
            r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#,
            "o",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let result = eval_oracle_bounded(
            &object,
            object.symbol("o").unwrap(),
            &[1u32.into()],
            &[],
            &[],
            &[3u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
            0,
            10,
        );
        assert!(
            matches!(result,Err(EvalError::Unchecked(reason))if reason.contains("host limit")),
            "oversized width must not become a zero mask"
        );
    }
}

#[test]
fn reference_storage_limits_precede_quantum_workspace_materialisation() {
    use bennett_cert_checker::domain::Tuple;
    use bennett_cert_checker::eval::eval_action_tuple;
    for body in [
        "a = qalloc fast 600000\nb = qalloc fast 600000\nqfree b\nqfree a",
        "qregion db 18 8",
    ] {
        let source = format!("action @a (t:wire mut) () () () ()\n^entry():\n{body}");
        let (inputs, _) = standalone(
            &source,
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
            "a",
            vec![],
        );
        let object = Object::parse(&inputs.object_text).unwrap();
        let iface = object.interface("a").unwrap();
        let before = Tuple {
            bundles: vec![0u32.into()],
            borrowed: vec![],
            regions: vec![],
            args: vec![],
        };
        let result = eval_action_tuple(
            &object,
            object.symbol("a").unwrap(),
            &iface,
            &before,
            &[],
            0,
            false,
            0,
        );
        assert!(
            matches!(result,Err(EvalError::Unchecked(reason))if reason.contains("host limit")),
            "{body}: storage must be bounded before allocation"
        );
    }
}

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
    inp.limits.max_reference_steps = 4;
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

#[test]
fn action_to_oracle_calls_use_the_configured_reference_row_limit() {
    use bennett_cert_checker::cert::check_certificate;
    let source="perm_action @outer (t:wire mut) (x:wire in) () (tk:token) ()\n^entry():\nf = qalloc fast 4\noracle_call @spin tk (x) t f () ()\noracle_call @spin tk (x) t f () ()\nqfree f\noracle @spin (x:wire in) (t:wire) (f:wire[4]) () ()\n^entry():\nqjump ^entry";
    let sig = r#"{"outer":{"abi":"0.9","args":["token"],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"spin":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#;
    let claim = json(
        r#"{"kind":"exhaustive","label":"proof","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[["100","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":[0]},"expected":{"table":[{"before":{"bundles":["0","0"],"borrowed":[],"regions":[]},"after":{"bundles":["0","0"],"borrowed":[],"regions":[]}}]}}"#,
    );
    let (mut inp, _) = standalone(source, sig, "outer", vec![claim]);
    inp.limits.max_reference_steps = 4;
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "unchecked", "{v:?}");
    assert!(codes(&v).is_empty(), "{v:?}");
    assert!(
        v.regions[0].claims[0]
            .1
            .unchecked_reason
            .as_ref()
            .unwrap()
            .contains("host limit"),
        "{v:?}"
    );
}

#[test]
fn reference_work_limit_counts_empty_loop_visits() {
    use bennett_cert_checker::cert::check_certificate;
    use bennett_cert_checker::json::{jcs, Value};
    let source="perm_action @loops (t:wire mut) () () () ()\nregs {  } bounds {1000}\n^entry():\nqloop 1000 {\n}";
    let sig = r#"{"loops":{"abi":"0.9","args":[],"bounds":[1000],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let claim = json(
        r#"{"kind":"exhaustive","label":"proof","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[],"budget":["0","0","0","0"]},"expected":{"table":[{"before":{"bundles":["0"],"borrowed":[],"regions":[]},"after":{"bundles":["0"],"borrowed":[],"regions":[]}}]}}"#,
    );
    let (mut inp, h) = standalone(source, sig, "loops", vec![claim]);
    inp.limits.max_reference_steps = 4;
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    set(regions.get_mut(&h).unwrap(), "bounds", json("[1000]"));
    inp.cert_bytes = jcs(&cert).unwrap();
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "unchecked", "{v:?}");
    assert!(codes(&v).is_empty(), "{v:?}");
    assert!(
        v.regions[0].claims[0]
            .1
            .unchecked_reason
            .as_ref()
            .unwrap()
            .contains("host limit"),
        "{v:?}"
    );
}

#[test]
fn lifted_permutation_calls_keep_the_shared_reference_work_counter() {
    let source="oracle @outer (x:wire in) (t:wire) (f:wire[4]) () (tk:token)\nregs { r:reg[1] }\n^entry():\nr = lift.const.i1 0\nlift.perm_call @middle (r) (x) () (tk)\nlift.retval r\nperm_action @middle (t:wire mut) (x:wire in) () (tk:token) ()\n^entry():\nf = qalloc fast 4\noracle_call @spin tk (x) t f () ()\noracle_call @spin tk (x) t f () ()\nqfree f\noracle @spin (x:wire in) (t:wire) (f:wire[4]) () ()\n^entry():\nqjump ^entry";
    let sig = r#"{"outer":{"abi":"0.9","args":["token"],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"},"middle":{"abi":"0.9","args":["token"],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"spin":{"abi":"0.9","args":[],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#;
    let (inp, _) = standalone(source, sig, "outer", vec![]);
    let object = Object::parse(&inp.object_text).unwrap();
    let budget = [100u32.into(), 0u32.into(), 0u32.into(), 0u32.into()];
    let result = eval_oracle_bounded(
        &object,
        object.symbol("outer").unwrap(),
        &[0u32.into()],
        &[],
        &[Classical::Token(budget.clone())],
        &budget,
        0,
        4,
    );
    assert!(
        matches!(result,Err(EvalError::Unchecked(s)) if s.contains("host limit") && s.contains("reference replay"))
    );
}
