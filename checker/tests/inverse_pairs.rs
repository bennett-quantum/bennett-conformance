use crate::common;
use bennett_cert_checker::cert::{check_certificate, Inputs};
use bennett_cert_checker::json::{jcs, Value};
use common::*;
fn fixture() -> Inputs {
    let functions="perm_action @pair (t:wire[2] mut) () () (tk:token, a:i2) ()\n^entry():\nqinplace @g @ginv tk t () () (a)\noracle @g (x:wire[2] in) (t:wire[2]) (f:wire[4]) () (a:i2)\nregs { r:reg[2] }\n^entry():\nr = lift.add x a\nlift.retval r\noracle @ginv (x:wire[2] in) (t:wire[2]) (f:wire[4]) () (a:i2)\nregs { r:reg[2] }\n^entry():\nr = lift.sub x a\nlift.retval r";
    let signatures = r#"{"pair":{"abi":"0.9","args":["token","i2"],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[2],"tier":"q"},"g":{"abi":"0.9","args":["i2"],"fault":4,"in":[2],"kind":"oracle","mem":[],"tgt":2,"tier":"q"},"ginv":{"abi":"0.9","args":["i2"],"fault":4,"in":[2],"kind":"oracle","mem":[],"tgt":2,"tier":"q"}}"#;
    let c = json(
        r#"{"domain":{"args":[{"hi":3,"lo":0,"name":"a","width":2}],"borrowed":[],"bundles":[{"hi":3,"lo":0,"name":"x","width":2}],"regions":[]},"expected":"inverse_pair","kind":"exhaustive"}"#,
    );
    let (mut inp, h) = standalone(functions, signatures, "pair", vec![c]);
    let object = bennett_cert_checker::object::Object::parse(&inp.object_text).unwrap();
    let g = object.symbol("g").unwrap().closure_hash.as_ref().unwrap();
    let gi = object
        .symbol("ginv")
        .unwrap()
        .closure_hash
        .as_ref()
        .unwrap();
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
        "subject",
        json(&format!(
            r#"{{"closure_hash":"{g}","inherited_controls":0,"variant":"forward"}}"#
        )),
    );
    let mut verification = region.get("verification").unwrap().clone();
    set(
        &mut verification,
        "inverse_pairs",
        json(&format!(
            r#"[{{"budget":[2,0,0,0],"claim":0,"g":"{g}","ginv":"{gi}"}}]"#
        )),
    );
    region.insert("verification".into(), verification);
    inp.cert_bytes = jcs(&cert).unwrap();
    inp
}
#[test]
fn inverse_pair_quantifies_arguments_and_checks_both_compositions() {
    let v = check_certificate(&fixture());
    assert!(codes(&v).is_empty(), "{v:?}");
    let o = &v.regions[0].claims[0].1;
    assert_eq!(o.outcome, "proof", "{v:?}");
    assert_eq!(o.elements.as_deref(), Some("16"));
}
#[test]
fn malformed_inverse_pair_budget_must_not_silently_become_zero() {
    let mut inp = fixture();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let Value::Obj(region) = regions.values_mut().next().unwrap() else {
        unreachable!()
    };
    let Value::Obj(ver) = region.get_mut("verification").unwrap() else {
        unreachable!()
    };
    let Value::Arr(pairs) = ver.get_mut("inverse_pairs").unwrap() else {
        unreachable!()
    };
    set(&mut pairs[0], "budget", json(r#"[null,0,0,0]"#));
    inp.cert_bytes = jcs(&cert).unwrap();
    let v = check_certificate(&inp);
    assert!(codes(&v).contains(&"E_CERT_INVOCATION"), "{v:?}");
}

#[test]
fn inverse_pair_proofs_do_not_satisfy_an_image_relative_label_request() {
    let mut inputs = fixture();
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let Value::Obj(region) = regions.values_mut().next().unwrap() else {
        panic!()
    };
    let Value::Arr(claims) = region.get_mut("claims").unwrap() else {
        panic!()
    };
    set(
        &mut claims[0],
        "label",
        Value::Str("proof_relative_to_image".into()),
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
    let verdict = check_certificate(&inputs);
    assert!(
        codes(&verdict).contains(&"E_CERT_COVERAGE_LABEL"),
        "{verdict:?}"
    );
}

#[test]
fn every_qinplace_source_site_requires_an_exact_owner_pair_entry() {
    for missing in [false, true] {
        let mut inputs = fixture();
        let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
        let Value::Obj(top) = &mut cert else { panic!() };
        let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
            panic!()
        };
        let Value::Obj(region) = regions.values_mut().next().unwrap() else {
            panic!()
        };
        let Value::Obj(ver) = region.get_mut("verification").unwrap() else {
            panic!()
        };
        let Value::Arr(pairs) = ver.get_mut("inverse_pairs").unwrap() else {
            panic!()
        };
        if missing {
            pairs.clear();
            region.insert("claims".into(), json("[]"));
        } else {
            let inverse = pairs[0].get("ginv").unwrap().clone();
            set(&mut pairs[0], "g", inverse);
        }
        inputs.cert_bytes = jcs(&cert).unwrap();
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_INVERSE_PAIR"),
            "missing={missing}: {verdict:?}"
        );
    }
}
#[test]
fn an_exhaustive_oracle_or_table_claim_cannot_be_referenced_as_an_inverse_pair_proof() {
    let mut inputs = fixture();
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let Value::Obj(region) = regions.values_mut().next().unwrap() else {
        panic!()
    };
    let Value::Arr(claims) = region.get_mut("claims").unwrap() else {
        panic!()
    };
    set(&mut claims[0], "expected", Value::Str("oracle".into()));
    inputs.cert_bytes = jcs(&cert).unwrap();
    let verdict = check_certificate(&inputs);
    assert!(
        codes(&verdict).contains(&"E_CERT_VERIFICATION_INDEX"),
        "{verdict:?}"
    );
}
