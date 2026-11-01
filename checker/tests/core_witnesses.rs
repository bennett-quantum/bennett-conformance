//! Slice-1 witness tests: SHA-256, JSON/JCS (RFC 8785), exact rationals,
//! pinned by ISA 0.9 spec values (sections 8.3, 9.4, 11.1, 12.3).

use bennett_cert_checker::json::{jcs, parse, Value};
use bennett_cert_checker::rat::{parse_decimal, upward_decimal_34, Rat};
use bennett_cert_checker::sha256_hex;
use num_traits::Zero;
use std::path::PathBuf;

fn isa_spec_dir() -> PathBuf {
    if let Ok(d) = std::env::var("BENNETT_ISA_SPEC_DIR") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bennett-isa-spec")
}

#[test]
fn sha256_standard_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        sha256_hex(b"[]"),
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
}

#[test]
fn jcs_number_rendering() {
    let cases = [
        ("0", "0"),
        ("-0", "0"),
        ("1", "1"),
        ("-1", "-1"),
        ("1.5", "1.5"),
        ("0.1", "0.1"),
        ("1e21", "1e+21"),
        ("1E21", "1e+21"),
        ("1e20", "100000000000000000000"),
        ("0.000001", "0.000001"),
        ("1e-7", "1e-7"),
        ("333333333.33333329", "333333333.3333333"),
        ("1.0", "1"),
        ("123456789012345680000", "123456789012345680000"),
    ];
    for (input, expect) in cases {
        let v = parse(input).unwrap_or_else(|e| panic!("parse {input}: {e}"));
        assert_eq!(jcs(&v).unwrap(), expect.as_bytes(), "input {input}");
    }
}

#[test]
fn jcs_object_key_order_and_escapes() {
    let v = parse(r#"{"b":1,"a":"x\"y\n","aa":[true,null]}"#).unwrap();
    assert_eq!(
        jcs(&v).unwrap(),
        br#"{"a":"x\"y\n","aa":[true,null],"b":1}"#
    );
    // duplicate keys are ill-formed input, never canonicalisable silently
    assert!(parse(r#"{"a":1,"a":2}"#).is_err());
    // trailing garbage rejected
    assert!(parse("{} x").is_err());
    assert!(parse("").is_err());
}

#[test]
fn claim_binding_params_hash_witness() {
    // ISA 0.9 section 12.3 CLAIM-BINDING witness.
    let args = parse(r#"["18446744073709551617","0x8000000000000000",["2","0","0","0"]]"#).unwrap();
    assert_eq!(
        sha256_hex(&jcs(&args).unwrap()),
        "9f628a694fa7259bf899bfb677aabce1c084e268a738a40f7f643eb6d8c9c093"
    );
    // +0 vs -0 f64 differ; token id never appears (tuple only).
    let args2 =
        parse(r#"["18446744073709551617","0x0000000000000000",["2","0","0","0"]]"#).unwrap();
    assert_ne!(
        sha256_hex(&jcs(&args2).unwrap()),
        sha256_hex(&jcs(&args).unwrap())
    );
}

#[test]
fn topology_and_frame_witness_hashes() {
    // Canonical bytes as printed in ISA 0.9 sections 8.3 and 9.4 (code fence content, no LF).
    let topo = parse(r#"{"capabilities":{"classical_step_time":1,"max_parallel":"unbounded","max_predicate_depth":"unbounded","mbu":false,"measurement":{"feedforward_latency":0,"mid_circuit":false}},"classes":{"fast":{"capacity":4,"dirty_rate":1,"qubit_second":1}},"gates":[{"classes":["fast"],"controls":{"max":1,"min":1},"cost":{"time":1},"epsilon":0,"op":"cx"}],"imports":{},"objective":{"weighted":{"time":1}},"ptr_widths_supported":[32],"qram":null,"resources":{"peak_qubits":{"kind":"peak","unit":"qubit"},"qubit_second":{"kind":"occupancy","unit":"qubit_second"},"time":{"kind":"time","unit":"second"}},"schemes":[],"tables":{},"topology":{"edges":[[0,1],[0,2],[1,3],[2,3]],"vertices":[{"class":"fast"},{"class":"fast"},{"class":"fast"},{"class":"fast"}]}}"#).unwrap();
    assert_eq!(
        sha256_hex(&jcs(&topo).unwrap()),
        "960cee71e88da141cc6c2b13cab3f28fce1d50c006e67446aadb592e2bfd0882"
    );
    let frame = parse(r#"{"ancillae":[{"class":"fast","id":"a","scheme":"controlled-x","state":"clean","width":{"var":"c"}}],"borrow":[{"class":"fast","id":"b","width":1}],"history":[{"class":"fast","id":"h","width":3}],"live_ranges":[{"end":2,"frame":"s","start":0},{"end":4,"frame":"s","start":2}],"scratch":[{"class":"fast","id":"s","state":"clean","width":{"args":[2,{"var":"c"}],"op":"+"}}],"slots":[{"class":"fast","id":"mut:0","role":"mut","width":2},{"class":"fast","id":"in:0","role":"in","width":1}]}"#).unwrap();
    assert_eq!(
        sha256_hex(&jcs(&frame).unwrap()),
        "f314eeef24a5a65a9c72f91e062aa3c83556358eaca166e689571700a72083ff"
    );
}

#[test]
fn controlled_fixture_documents_are_jcs_and_hash_linked() {
    let dir = isa_spec_dir().join("conformance/fixtures/controlled-action-09");
    let cert_b = std::fs::read(dir.join("certificate.json")).expect("controlled fixture present");
    let sched_b = std::fs::read(dir.join("schedule.json")).unwrap();
    let tgt_b = std::fs::read(dir.join("target.json")).unwrap();
    for (name, b) in [
        ("certificate", &cert_b),
        ("schedule", &sched_b),
        ("target", &tgt_b),
    ] {
        let v = parse(std::str::from_utf8(b).unwrap()).unwrap();
        assert_eq!(&jcs(&v).unwrap(), b, "{name} must already be JCS bytes");
    }
    let cert = parse(std::str::from_utf8(&cert_b).unwrap()).unwrap();
    let bindings = cert.get("bindings").unwrap();
    assert_eq!(
        bindings.get("schedule_hash").unwrap().as_str().unwrap(),
        sha256_hex(&sched_b)
    );
    assert_eq!(
        bindings.get("target_hash").unwrap().as_str().unwrap(),
        sha256_hex(&tgt_b)
    );
}

#[test]
fn ref_library_envelope_hash() {
    // ISA 0.9 section 11.1: SHA-256 of the four-line envelope.
    let lib =
        std::fs::read(isa_spec_dir().join("spec/ref-0.9.json")).expect("ref-0.9.json present");
    let mut env = b"isa=0.9\nexpansion=ref\nversion=0.9\n".to_vec();
    env.extend_from_slice(&lib);
    env.push(b'\n');
    assert_eq!(
        sha256_hex(&env),
        "5de4e4995350313e50217880ba1b95de39cd043283b726337d4894efb005d197"
    );
}

#[test]
fn decimal_rationals_exact() {
    assert_eq!(parse_decimal("0").unwrap(), Rat::zero());
    assert_eq!(
        parse_decimal("0.1").unwrap() * parse_decimal("3").unwrap(),
        parse_decimal("0.3").unwrap()
    );
    assert!(
        parse_decimal("1/3").is_err(),
        "fractions are not decimal text"
    );
    assert!(parse_decimal("01").is_err(), "non-canonical leading zero");
    assert!(parse_decimal("").is_err());
    // numbers equal regardless of spelling (ISA section 11: 0 = \"0.0\" = \"0.000\")
    assert_eq!(parse_decimal("0.000").unwrap(), Rat::zero());
}

#[test]
fn upward_decimal_34_witness() {
    // ISA 0.9 section 10.1 decimal_checks.
    let cases = [
        ("0", "1", "0"),
        ("2", "1", "2"),
        ("1", "8", "0.125"),
        ("1", "3", "0.3333333333333333333333333333333334"),
        ("2", "3", "0.6666666666666666666666666666666667"),
        ("1", "30", "0.03333333333333333333333333333333334"),
        ("10", "3", "3.333333333333333333333333333333334"),
    ];
    for (n, d, expect) in cases {
        let q = Rat::new(n.parse().unwrap(), d.parse().unwrap());
        assert_eq!(upward_decimal_34(&q).unwrap(), expect, "{n}/{d}");
    }
}

#[test]
fn value_number_lexeme_preserved() {
    let v = parse("18446744073709551617").unwrap();
    match v {
        Value::Num(lex) => assert_eq!(lex, "18446744073709551617"),
        other => panic!("expected number, got {other:?}"),
    }
}
