use crate::{common::*, fixed_images};
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::{jcs, Value},
    object::Object,
};

fn attach(inputs: &mut Inputs, preparation: Value) {
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(fields) = &mut cert else {
        panic!()
    };
    set(
        fields.get_mut("bindings").unwrap(),
        "preparation",
        preparation,
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
}
fn attachment(inputs: &Inputs) -> Value {
    let cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let hash = Object::parse(&inputs.object_text)
        .unwrap()
        .symbol("a")
        .unwrap()
        .closure_hash
        .clone()
        .unwrap();
    let claim = &cert
        .get("regions")
        .unwrap()
        .get(&hash)
        .unwrap()
        .get("claims")
        .unwrap()
        .as_arr()
        .unwrap()[0];
    let mut entry = json(&format!(
        r#"{{"closure_hash":"{hash}","claim":0,"context_key":[],"invocation":{{"args":[],"budget":["0","0","0","0"]}},"images":[],"witness":{{"kind":"launch_copy","manifest_hash":"{}","copy_indices":[0]}},"restoration":{{"kind":"matrix_units"}}}}"#,
        "0".repeat(64)
    ));
    set(
        &mut entry,
        "images",
        claim
            .get("domain")
            .unwrap()
            .get("fixed_regions")
            .unwrap()
            .clone(),
    );
    entry
}
fn descriptor(entries: Vec<Value>) -> Value {
    let mut d = json(r#"{"version":"bennett.image-preparation/1","entries":[]}"#);
    set(&mut d, "entries", Value::Arr(entries));
    d
}
#[test]
fn absent_and_empty_preparation_have_identical_theorem_scope() {
    let inputs = fixed_images::fixture(1, 2, "09");
    let before = check_certificate(&inputs);
    let mut with = inputs.clone();
    attach(&mut with, descriptor(vec![]));
    let after = check_certificate(&with);
    assert!(codes(&after).is_empty(), "{after:?}");
    assert_eq!(
        after.regions[0].claims[0].1.outcome,
        "proof_relative_to_image"
    );
    assert_eq!(
        after.regions[0].claims[0].1.covered_fraction.as_deref(),
        Some("1/16")
    );
    assert_eq!(before.overall, after.overall);
}
#[test]
fn unknown_preparation_version_is_unestablished_never_an_unrestricted_claim() {
    let mut inputs = fixed_images::fixture(1, 2, "09");
    attach(
        &mut inputs,
        json(r#"{"version":"bennett.image-preparation/999","future":true}"#),
    );
    let report = check_certificate(&inputs);
    assert!(codes(&report).is_empty(), "{report:?}");
    assert_eq!(
        report.regions[0].claims[0].1.outcome,
        "proof_relative_to_image"
    );
    assert_eq!(report.overall, "unchecked");
    assert!(encode(&report.to_value()).contains("unsupported preparation version"));
}
#[test]
fn preparation_validates_claim_images_and_concrete_invocation_before_mechanics() {
    for (key, value) in [
        ("claim", json("1")),
        ("closure_hash", Value::Str("f".repeat(64))),
        ("context_key", json("[{}]")),
        (
            "invocation",
            json(r#"{"args":[],"budget":["1","0","0","0"]}"#),
        ),
        ("images", json("[]")),
        ("extra", json("0")),
    ] {
        let mut inputs = fixed_images::fixture(1, 2, "09");
        let mut entry = attachment(&inputs);
        set(&mut entry, key, value);
        attach(&mut inputs, descriptor(vec![entry]));
        let report = check_certificate(&inputs);
        assert!(
            codes(&report).contains(&"E_CERT_PREPARATION"),
            "{key}: {report:?}"
        );
        assert_eq!(
            report.regions[0].claims[0].1.outcome,
            "proof_relative_to_image"
        );
    }
}
#[test]
fn launch_template_or_missing_manifest_cannot_attest_runtime_copy() {
    let mut inputs = fixed_images::fixture(1, 2, "09");
    let entry = attachment(&inputs);
    attach(&mut inputs, descriptor(vec![entry]));
    let report = check_certificate(&inputs);
    assert!(codes(&report).is_empty(), "{report:?}");
    assert_eq!(report.overall, "unchecked");
    assert!(encode(&report.to_value()).contains("launch manifest was not supplied"));
}
#[test]
fn attachment_keys_are_sorted_unique_and_never_rewrite_replay_order() {
    let mut inputs = fixed_images::fixture(1, 2, "09");
    let entry = attachment(&inputs);
    attach(&mut inputs, descriptor(vec![entry.clone(), entry]));
    let report = check_certificate(&inputs);
    assert!(codes(&report).contains(&"E_CERT_PREPARATION"), "{report:?}");
}

#[test]
fn logical_occurrences_require_full_terminal_coordinates_and_linked_symbols() {
    for missing in [
        "context_key",
        "symbol",
        "point",
        "iteration",
        "instance",
        "generated",
    ] {
        let mut inputs = fixed_images::fixture(1, 2, "09");
        let hash = Object::parse(&inputs.object_text)
            .unwrap()
            .symbol("a")
            .unwrap()
            .interface_hash
            .clone();
        let mut occurrence = json(&format!(
            r#"{{"context_key":[],"symbol":"{hash}","point":"entry:0","iteration":[],"instance":[],"generated":0}}"#
        ));
        let Value::Obj(fields) = &mut occurrence else {
            panic!()
        };
        fields.remove(missing);
        let mut region = json(r#"{"name":"r","allocation":{},"prepare":[]}"#);
        set(&mut region, "allocation", occurrence);
        let mut witness = json(r#"{"kind":"logical","regions":[]}"#);
        set(&mut witness, "regions", Value::Arr(vec![region]));
        let mut entry = attachment(&inputs);
        set(&mut entry, "witness", witness);
        attach(&mut inputs, descriptor(vec![entry]));
        let report = check_certificate(&inputs);
        assert!(
            codes(&report).contains(&"E_CERT_PREPARATION"),
            "{missing}: {report:?}"
        );
    }
}

fn logical_fixture() -> Inputs {
    let functions="action @root (t:wire mut) () () () ()\n^entry():\nqregion r 0 8 @img\naction_call @a (t) () (r) ()\naction @a (t:wire mut) () (m:region[0,8] in) () ()\n^entry():\nx t\nx t\n\nsection data\nimg 01\n\nsection devices\n{\"models\":{},\"services\":{}}";
    let signatures = r#"{"root":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"},"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[{"A":0,"W":8,"role":"in"}],"mut":[1],"tier":"q"},"img":{"abi":"0.9","kind":"data","size":1,"tier":"-"}}"#;
    let claim = json(&format!(
        r#"{{"kind":"exhaustive","label":"proof_relative_to_image","invocation":{{"args":[],"budget":["0","0","0","0"]}},"domain":{{"fixed_regions":[{{"name":"m","A":0,"W":8,"image":"01","sha256":"{}"}}],"bundles":[{{"name":"t","width":1,"lo":0,"hi":1}}],"borrowed":[],"regions":[]}},"expected":{{"table":[{{"before":{{"bundles":["0"],"borrowed":[],"regions":[["1"]]}},"after":{{"bundles":["0"],"borrowed":[],"regions":[["1"]]}}}},{{"before":{{"bundles":["1"],"borrowed":[],"regions":[["1"]]}},"after":{{"bundles":["1"],"borrowed":[],"regions":[["1"]]}}}}]}}}}"#,
        bennett_cert_checker::sha256_hex(&[1])
    ));
    let (mut inputs, callee) = standalone(functions, signatures, "a", vec![claim]);
    let (root_inputs, root) = standalone(functions, signatures, "root", vec![]);
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root_cert = json(std::str::from_utf8(&root_inputs.cert_bytes).unwrap());
    set(
        &mut cert,
        "bindings",
        root_cert.get("bindings").unwrap().clone(),
    );
    let Value::Obj(fields) = &mut cert else {
        panic!()
    };
    set(
        fields.get_mut("regions").unwrap(),
        &root,
        root_cert
            .get("regions")
            .unwrap()
            .get(&root)
            .unwrap()
            .clone(),
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
    let base = controlled();
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", json("[]"));
    let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    set(
        &mut target,
        "gates",
        json(
            r#"[{"op":"x","classes":["fast"],"controls":{"min":0,"max":0},"epsilon":0,"cost":{"time":1}}]"#,
        ),
    );
    bind_schedule(&mut inputs, schedule, target);
    let object = Object::parse(&inputs.object_text).unwrap();
    let root_symbol = &object.symbol("root").unwrap().interface_hash;
    let key = json(&format!(
        r#"[{{"symbol":"{root_symbol}","point":"entry:1","iteration":[],"instance":[],"call_index":0}}]"#
    ));
    let allocation = json(&format!(
        r#"{{"context_key":[],"symbol":"{root_symbol}","point":"$construct","iteration":[],"instance":[],"generated":0}}"#
    ));
    let mut acquiring = allocation.clone();
    set(&mut acquiring, "allocation", allocation.clone());
    let mut prepare = acquiring.clone();
    set(&mut prepare, "generated", json("1"));
    let mut region = json(r#"{"name":"m","allocation":{},"prepare":[]}"#);
    set(&mut region, "allocation", acquiring);
    set(&mut region, "prepare", Value::Arr(vec![prepare]));
    let mut witness = json(r#"{"kind":"logical","regions":[]}"#);
    set(&mut witness, "regions", Value::Arr(vec![region]));
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let Value::Obj(record) = regions.get_mut(&callee).unwrap() else {
        panic!()
    };
    let Value::Obj(frame) = record.get_mut("frame").unwrap() else {
        panic!()
    };
    let Value::Arr(slots) = frame.get_mut("slots").unwrap() else {
        panic!()
    };
    slots.push(json(
        r#"{"class":"fast","id":"mem:0","role":"mem_in","width":8}"#,
    ));
    let Value::Arr(claims) = record.get_mut("claims").unwrap() else {
        panic!()
    };
    let Value::Obj(claim) = &mut claims[0] else {
        panic!()
    };
    let Value::Obj(subject) = claim.get_mut("subject").unwrap() else {
        panic!()
    };
    subject.remove("schedule_hash");
    let images = claim
        .get("domain")
        .unwrap()
        .get("fixed_regions")
        .unwrap()
        .clone();
    inputs.cert_bytes = jcs(&cert).unwrap();
    let mut entry = json(&format!(
        r#"{{"closure_hash":"{callee}","claim":0,"context_key":[],"invocation":{{"args":[],"budget":["0","0","0","0"]}},"images":[],"witness":{{}},"restoration":{{"kind":"matrix_units"}}}}"#
    ));
    set(&mut entry, "context_key", key);
    set(&mut entry, "images", images);
    set(&mut entry, "witness", witness);
    attach(&mut inputs, descriptor(vec![entry]));
    inputs
}
#[test]
fn logical_preparation_reconstructs_the_real_prefix_and_checks_exact_restoration() {
    let inputs = logical_fixture();
    let report = check_certificate(&inputs);
    assert!(codes(&report).is_empty(), "{report:?}");
    let value = report.to_value();
    let preparation = &value.get("preparation").unwrap().as_arr().unwrap()[0];
    assert_eq!(
        preparation.get("applicability").unwrap().as_str(),
        Some("established"),
        "{report:?}"
    );
}
#[test]
fn stale_public_leaf_address_rejects_instead_of_becoming_an_unchecked_premise() {
    let mut inputs = logical_fixture();
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(bindings) = top.get_mut("bindings").unwrap() else {
        panic!()
    };
    let Value::Obj(prep) = bindings.get_mut("preparation").unwrap() else {
        panic!()
    };
    let Value::Arr(entries) = prep.get_mut("entries").unwrap() else {
        panic!()
    };
    let Value::Obj(entry) = &mut entries[0] else {
        panic!()
    };
    let Value::Obj(witness) = entry.get_mut("witness").unwrap() else {
        panic!()
    };
    let Value::Arr(regions) = witness.get_mut("regions").unwrap() else {
        panic!()
    };
    let Value::Obj(region) = &mut regions[0] else {
        panic!()
    };
    let Value::Arr(prepare) = region.get_mut("prepare").unwrap() else {
        panic!()
    };
    set(&mut prepare[0], "generated", json("9"));
    inputs.cert_bytes = jcs(&cert).unwrap();
    let report = check_certificate(&inputs);
    assert!(codes(&report).contains(&"E_CERT_PREPARATION"), "{report:?}");
}

fn prefix_model(mut inputs: Inputs, matrix: &str, epsilon: u32) -> Inputs {
    let mut target = json(std::str::from_utf8(inputs.target_bytes.as_ref().unwrap()).unwrap());
    let mut model = json(
        r#"{"version":"bennett.native-model/1","kind":"unitary","arity":1,"basis":["t"],"parameter_bindings":{"k":"k","c":"c"},"control_models":{"kind":"controlled_U","controls":"c","disabled":"identity"},"operand_map":[0],"data":{}}"#,
    );
    set(
        &mut model,
        "data",
        json(&format!(r#"{{"matrix":{matrix}}}"#)),
    );
    let mut gate = json(&format!(
        r#"{{"op":"x","classes":["fast"],"controls":{{"min":0,"max":0}},"epsilon":{epsilon},"cost":{{"time":1}}}}"#
    ));
    set(&mut gate, "realisation_model", model);
    set(&mut target, "gates", Value::Arr(vec![gate]));
    let mut schedule = json(std::str::from_utf8(inputs.schedule_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(fields) = &mut schedule else {
        panic!()
    };
    set(
        fields.get_mut("bindings").unwrap(),
        "epsilon",
        json(&epsilon.to_string()),
    );
    set(
        &mut schedule,
        "entries",
        json(&format!(
            r#"[{{"kind":"gate","path":[],"generated":"*","phase":"*","step":"*","gate":{{"epsilon_share":{epsilon},"realisation":"native"}}}}]"#
        )),
    );
    bind_schedule(&mut inputs, schedule, target);
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    for record in regions.values_mut() {
        let Value::Obj(record) = record else { panic!() };
        let Value::Arr(claims) = record.get_mut("claims").unwrap() else {
            panic!()
        };
        for claim in claims {
            let Value::Obj(fields) = claim else { panic!() };
            let Value::Obj(subject) = fields.get_mut("subject").unwrap() else {
                panic!()
            };
            subject.remove("schedule_hash");
        }
    }
    inputs.cert_bytes = jcs(&cert).unwrap();
    inputs
}
#[test]
fn realised_prefix_leakage_refutes_applicability_without_erasing_the_conditional_theorem() {
    // Published §15: R|0>=(3/5)|0>+(4/5)|1>; leakage outside declared |1> is 9/25.
    let inputs = prefix_model(
        logical_fixture(),
        r#"[[[["3","5"],["0","1"]],[["4","5"],["0","1"]]],[[["4","5"],["0","1"]],[["-3","5"],["0","1"]]]]"#,
        1,
    );
    let report = check_certificate(&inputs);
    assert!(codes(&report).contains(&"E_CERT_PREPARATION"), "{report:?}");
    assert!(
        report.regions.iter().any(|r| r
            .claims
            .iter()
            .any(|(_, c)| c.outcome == "proof_relative_to_image")),
        "{report:?}"
    );
}
#[test]
fn positive_advertised_epsilon_does_not_preclude_exact_image_preparation() {
    // -X has operator-norm distance 2 from X, but still prepares the exact |1> image.
    // Two callee -X gates compose to +I, including phase.
    let inputs = prefix_model(
        logical_fixture(),
        r#"[[[["0","1"],["0","1"]],[["-1","1"],["0","1"]]],[[["-1","1"],["0","1"]],[["0","1"],["0","1"]]]]"#,
        2,
    );
    let report = check_certificate(&inputs);
    assert!(codes(&report).is_empty(), "{report:?}");
    let value = report.to_value();
    assert_eq!(
        value.get("preparation").unwrap().as_arr().unwrap()[0]
            .get("applicability")
            .unwrap()
            .as_str(),
        Some("established"),
        "{report:?}"
    );
}
