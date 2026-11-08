//! Certificate schema/binding layer: strict fields, hash bindings, version
//! agreement. Claim replay outcomes are asserted in later test files.

use bennett_cert_checker::cert::{check_certificate, Inputs, Limits};
use bennett_cert_checker::json::{jcs, parse, Value};
use std::path::PathBuf;

#[path = "common/pins.rs"]
mod pins;
fn isa_dir() -> PathBuf {
    pins::isa_spec_dir()
}

struct Fixture {
    cert: Vec<u8>,
    object: String,
    target: Vec<u8>,
    schedule: Vec<u8>,
}

fn controlled() -> Fixture {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/controlled-action-k87");
    Fixture {
        cert: std::fs::read(d.join("certificate.json")).unwrap(),
        object: std::fs::read_to_string(d.join("controlled.bisa")).unwrap(),
        target: std::fs::read(d.join("target.json")).unwrap(),
        schedule: std::fs::read(d.join("schedule.json")).unwrap(),
    }
}

fn inputs(f: &Fixture) -> Inputs {
    Inputs {
        cert_bytes: f.cert.clone(),
        object_text: f.object.clone(),
        target_bytes: Some(f.target.clone()),
        schedule_bytes: Some(f.schedule.clone()),
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    }
}

fn mutate_cert(f: &Fixture, f2: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut v = parse(std::str::from_utf8(&f.cert).unwrap()).unwrap();
    f2(&mut v);
    jcs(&v).unwrap()
}

fn codes(v: &bennett_cert_checker::cert::Verdict) -> Vec<String> {
    let mut out: Vec<String> = v.errors.iter().map(|e| e.code.to_string()).collect();
    for r in &v.regions {
        out.extend(r.errors.iter().map(|e| e.code.to_string()));
        for (_, o) in &r.claims {
            out.extend(o.errors.iter().map(|e| e.code.to_string()));
        }
    }
    out
}

#[test]
fn controlled_certificate_passes_schema_and_bindings() {
    let f = controlled();
    let v = check_certificate(&inputs(&f));
    assert_eq!(
        codes(&v),
        Vec::<String>::new(),
        "verdict: {}",
        String::from_utf8(v.to_json_bytes()).unwrap()
    );
    assert_eq!(
        v.hashes.get("object_hash").map(String::as_str),
        Some("45bbb506541e3a8bb28a18e1bb0aef16ba01ee1e7cac74d5a190deb3ad36ef5a")
    );
    assert_eq!(v.regions.len(), 2);
    let total_claims: usize = v.regions.iter().map(|r| r.claims.len()).sum();
    assert_eq!(total_claims, 3);
}

#[test]
fn verdict_json_is_deterministic() {
    let f = controlled();
    let a = check_certificate(&inputs(&f)).to_json_bytes();
    let b = check_certificate(&inputs(&f)).to_json_bytes();
    assert_eq!(a, b);
    // And re-canonicalising the verdict changes nothing (it is JCS).
    let v = parse(std::str::from_utf8(&a).unwrap()).unwrap();
    assert_eq!(jcs(&v).unwrap(), a);
}

#[test]
fn cert_unknown_field_rejected_case_f_cert_schema() {
    // Case F-cert-schema: certificate {"unknown":0} (object f_cert_unknown_field).
    let object = std::fs::read_to_string(
        isa_dir().join("conformance/cases/objects/f_cert_unknown_field.bisa"),
    )
    .unwrap();
    let v = check_certificate(&Inputs {
        cert_bytes: br#"{"unknown":0}"#.to_vec(),
        object_text: object,
        target_bytes: None,
        schedule_bytes: None,
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    });
    assert_eq!(v.overall, "rejected");
    assert!(
        codes(&v).iter().any(|c| c == "E_CERT_SCHEMA"),
        "{:?}",
        codes(&v)
    );
}

#[test]
fn non_jcs_certificate_bytes_rejected() {
    let f = controlled();
    let mut inp = inputs(&f);
    inp.cert_bytes = b"{ \"bindings\": {} }".to_vec();
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    assert!(codes(&v)
        .iter()
        .any(|c| c == "E_CERT_JCS" || c == "E_CERT_JSON"));
}

#[test]
fn binding_hash_mismatches_reject_with_binding_code() {
    let f = controlled();
    // Flip one hex digit of bindings.object_hash.
    let cert = mutate_cert(&f, |v| {
        let b = v.get("bindings").unwrap().clone();
        let mut m = b.as_obj().unwrap().clone();
        let h = m.get("object_hash").unwrap().as_str().unwrap().to_string();
        let flipped = format!("0{}", &h[1..]);
        let flipped = if flipped == h {
            format!("1{}", &h[1..])
        } else {
            flipped
        };
        m.insert("object_hash".into(), Value::Str(flipped));
        if let Value::Obj(top) = v {
            top.insert("bindings".into(), Value::Obj(m));
        }
    });
    let mut inp = inputs(&f);
    inp.cert_bytes = cert;
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    assert!(
        v.errors
            .iter()
            .any(|e| e.code == "E_CERT_BINDING" && e.path == "bindings.object_hash"),
        "{:?}",
        v.errors
    );
}

#[test]
fn version_disagreement_rejected() {
    let f = controlled();
    let cert = mutate_cert(&f, |v| {
        if let Value::Obj(top) = v {
            let mut vers = top.get("versions").unwrap().as_obj().unwrap().clone();
            vers.insert("isa".into(), Value::Str("0.8".into()));
            top.insert("versions".into(), Value::Obj(vers));
        }
    });
    let mut inp = inputs(&f);
    inp.cert_bytes = cert;
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    assert!(codes(&v).iter().any(|c| c == "E_CERT_VERSION"));
}

#[test]
fn supplied_schedule_mismatch_rejected() {
    let f = controlled();
    let mut inp = inputs(&f);
    // Target document supplied where the schedule belongs: hash differs.
    inp.schedule_bytes = Some(f.target.clone());
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    assert!(
        v.errors.iter().any(|e| e.path == "bindings.schedule_hash"),
        "{:?}",
        v.errors
    );
}

#[test]
fn unknown_claim_kind_rejected_per_claim() {
    let f = controlled();
    let cert = mutate_cert(&f, |v| {
        if let Value::Obj(top) = v {
            let mut regions = top.get("regions").unwrap().as_obj().unwrap().clone();
            let key = top
                .get("bindings")
                .unwrap()
                .get("closure_hash")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let mut region = regions.get(&key).unwrap().as_obj().unwrap().clone();
            let mut claims = region.get("claims").unwrap().as_arr().unwrap().to_vec();
            if let Value::Obj(c0) = &mut claims[0] {
                c0.insert("kind".into(), Value::Str("minimality".into()));
            }
            region.insert("claims".into(), Value::Arr(claims));
            regions.insert(key, Value::Obj(region));
            top.insert("regions".into(), Value::Obj(regions));
        }
    });
    let mut inp = inputs(&f);
    inp.cert_bytes = cert;
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    let root = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("controlled"))
        .unwrap();
    assert_eq!(root.claims[0].1.outcome, "reject");
}

#[test]
fn frame_overlapping_live_ranges_rejected() {
    // Case F-frame-live-ranges, as a frame mutation of the controlled cert:
    // one shared scratch frame with overlapping uses.
    let f = controlled();
    let cert = mutate_cert(&f, |v| {
        if let Value::Obj(top) = v {
            let mut regions = top.get("regions").unwrap().as_obj().unwrap().clone();
            let key = top
                .get("bindings")
                .unwrap()
                .get("closure_hash")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let mut region = regions.get(&key).unwrap().as_obj().unwrap().clone();
            let frame = parse(r#"{"ancillae":[],"borrow":[],"history":[],"live_ranges":[{"end":2,"frame":"s","start":0},{"end":3,"frame":"s","start":1}],"scratch":[{"class":"fast","id":"s","state":"clean","width":1}],"slots":[{"class":"fast","id":"mut:0","role":"mut","width":1},{"class":"fast","id":"in:0","role":"in","width":1}]}"#).unwrap();
            region.insert("frame".into(), frame);
            regions.insert(key, Value::Obj(region));
            top.insert("regions".into(), Value::Obj(regions));
        }
    });
    let mut inp = inputs(&f);
    inp.cert_bytes = cert;
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "rejected");
    assert!(
        codes(&v).iter().any(|c| c == "E_CERT_FRAME"),
        "{:?}",
        codes(&v)
    );
}

#[test]
fn frame_witness_live_ranges_are_disjoint() {
    // Section 9.4 FRAME witness arithmetic: uses meeting at 2 do not overlap.
    let f = controlled();
    let cert = mutate_cert(&f, |v| {
        if let Value::Obj(top) = v {
            let mut regions = top.get("regions").unwrap().as_obj().unwrap().clone();
            let key = top
                .get("bindings")
                .unwrap()
                .get("closure_hash")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let mut region = regions.get(&key).unwrap().as_obj().unwrap().clone();
            let frame = parse(r#"{"ancillae":[],"borrow":[],"history":[],"live_ranges":[{"end":2,"frame":"s","start":0},{"end":4,"frame":"s","start":2}],"scratch":[{"class":"fast","id":"s","state":"clean","width":1}],"slots":[{"class":"fast","id":"mut:0","role":"mut","width":1},{"class":"fast","id":"in:0","role":"in","width":1}]}"#).unwrap();
            region.insert("frame".into(), frame);
            regions.insert(key, Value::Obj(region));
            top.insert("regions".into(), Value::Obj(regions));
        }
    });
    let mut inp = inputs(&f);
    inp.cert_bytes = cert;
    let v = check_certificate(&inp);
    assert!(
        !codes(&v).iter().any(|c| c == "E_CERT_FRAME"),
        "{:?}",
        codes(&v)
    );
}

#[test]
fn legacy_compat_fixture_is_not_misdiagnosed_with_the_09_schema() {
    let d = isa_dir().join("conformance/fixtures/controlled-action-09");
    let object = std::fs::read_to_string(d.join("compat-0.8.bisa")).unwrap();
    let parsed = bennett_cert_checker::object::Object::parse(&object).unwrap();
    let v = check_certificate(&Inputs {
        cert_bytes: parsed.certificate_raw.unwrap().into_bytes(),
        object_text: object,
        target_bytes: Some(std::fs::read(d.join("target.json")).unwrap()),
        schedule_bytes: None,
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    });
    assert_eq!(v.overall, "unchecked", "{v:?}");
    assert!(v.errors.is_empty());
    assert!(v.unchecked_items.iter().any(|s| s.contains("legacy")));
}
