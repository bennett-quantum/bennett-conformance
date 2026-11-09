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
