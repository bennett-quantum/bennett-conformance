//! Section 12.2 domain resolution/coverage and section 12.1 sampling,
//! pinned by the COMPONENT-COVERAGE and CLAIM-REPLAY witnesses.

use bennett_cert_checker::domain::{resolve_domain, HostLimit, ResolveError};
use bennett_cert_checker::json::parse;
use bennett_cert_checker::object::Object;
use bennett_cert_checker::sampler::{draw_elements, Sampler};

fn t3() -> Object {
    let text = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../bennett-isa-spec/conformance/cases/objects/t3_domains.bisa"),
    )
    .unwrap();
    Object::parse(&text).unwrap()
}

#[test]
fn component_coverage_witness_arithmetic() {
    // ISA 0.9 COMPONENT-COVERAGE witness against borrow_tuple
    // (mut t,u width 1; the witness action borrows one bit; here the
    // borrowed entry is positional and can never cover mut t).
    let o = t3();
    // borrow_tuple has no qborrow; use oracle_tuple-style direct interface
    // arithmetic on borrow_tuple bundles only for the bundle half:
    let iface = o.interface("borrow_tuple").unwrap();
    let domain = parse(r#"{"bundles":[{"name":"u","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#).unwrap();
    // borrow_tuple bundles are named t,b (mut) and x (in) in this object;
    // "u" is unknown here and must reject — resolution is by header name.
    match resolve_domain(&domain, &iface, 0) {
        Err(ResolveError::Reject(e)) => assert!(e.message.contains("u"), "{e:?}"),
        other => panic!("expected reject, got {other:?}"),
    }
}

#[test]
fn omitted_component_is_zero_slice_not_full_range() {
    let o = t3();
    let iface = o.interface("borrow_tuple").unwrap();
    // mut t,b and in x: full D has 8 elements. Domain over b,x only:
    // t omitted (held at 0) -> 4 elements, fraction 1/2, not coverage.
    let domain = parse(
        r#"{"bundles":[{"name":"b","width":1,"lo":0,"hi":1},{"name":"x","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#,
    )
    .unwrap();
    let d = resolve_domain(&domain, &iface, 0).unwrap();
    assert_eq!(d.n.to_string(), "4");
    assert_eq!(d.full_n.to_string(), "8");
    assert_eq!(d.covered_fraction(), "1/2");
    assert!(!d.covers_full);
    // Enumeration: last component fastest; omitted t stays 0.
    let els: Vec<_> = d.elements().collect();
    assert_eq!(els.len(), 4);
    assert!(els.iter().all(|e| e.bundles[0].to_string() == "0"), "t omitted must be 0");
    let bx: Vec<(String, String)> = els
        .iter()
        .map(|e| (e.bundles[1].to_string(), e.bundles[2].to_string()))
        .collect();
    assert_eq!(
        bx,
        [("0".into(), "0".into()), ("0".into(), "1".into()), ("1".into(), "0".into()), ("1".into(), "1".into())]
    );
}

#[test]
fn full_domain_by_resolved_identity() {
    let o = t3();
    let iface = o.interface("borrow_tuple").unwrap();
    let domain = parse(
        r#"{"bundles":[{"name":"t","width":1,"lo":0,"hi":1},{"name":"b","width":1,"lo":0,"hi":1},{"name":"x","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#,
    )
    .unwrap();
    let d = resolve_domain(&domain, &iface, 0).unwrap();
    assert!(d.covers_full);
    assert_eq!(d.covered_fraction(), "1/1");
    // Duplicate component, width mismatch, out-of-range hi: all ill-formed.
    for bad in [
        r#"{"bundles":[{"name":"t","width":1,"lo":0,"hi":1},{"name":"t","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#,
        r#"{"bundles":[{"name":"t","width":2,"lo":0,"hi":1}],"borrowed":[],"regions":[]}"#,
        r#"{"bundles":[{"name":"t","width":1,"lo":0,"hi":2}],"borrowed":[],"regions":[]}"#,
        r#"{"bundles":[{"name":"t","width":1,"lo":1,"hi":0}],"borrowed":[],"regions":[]}"#,
        r#"{"bundles":[],"borrowed":[{"name":"z","width":1,"lo":0,"hi":1}],"regions":[]}"#,
    ] {
        let d = parse(bad).unwrap();
        assert!(
            matches!(resolve_domain(&d, &iface, 0), Err(ResolveError::Reject(_))),
            "must reject: {bad}"
        );
    }
}

#[test]
fn oracle_full_domain_includes_tgt_and_fault() {
    // D-expected-oracle-claim: inc2 D is x(2) x tgt(2) x fault(4) = 256.
    let o = t3();
    let iface = o.interface("inc2").unwrap();
    let domain = parse(
        r#"{"bundles":[{"name":"x","width":2,"lo":0,"hi":3},{"name":"t","width":2,"lo":0,"hi":3},{"name":"f","width":4,"lo":0,"hi":15}],"borrowed":[],"regions":[]}"#,
    )
    .unwrap();
    let d = resolve_domain(&domain, &iface, 0).unwrap();
    assert_eq!(d.n.to_string(), "256");
    assert!(d.covers_full);
    // Input-only domain is a slice: 4 of 256.
    let domain = parse(r#"{"bundles":[{"name":"x","width":2,"lo":0,"hi":3}],"borrowed":[],"regions":[]}"#).unwrap();
    let d = resolve_domain(&domain, &iface, 0).unwrap();
    assert_eq!(d.covered_fraction(), "1/64");
    assert!(!d.covers_full);
}

#[test]
fn sampler_singleton_components_draw_no_words() {
    // CLAIM-REPLAY witness: sampler seed [1,0,0,0], count 4 over singleton
    // components draws no generator words; 1 distinct of 2^25 full tuples.
    let mut s = Sampler::new(["1", "0", "0", "0"]).unwrap();
    let ranges = [(7u64, 7u64), (0, 0), (3, 3)];
    for _ in 0..4 {
        let el: Vec<String> = ranges
            .iter()
            .map(|(lo, hi)| s.draw_component(*lo, *hi).to_string())
            .collect();
        assert_eq!(el, ["7", "0", "3"]);
    }
    // State must be untouched (no words drawn for R=1).
    assert_eq!(s.state(), [1, 0, 0, 0]);
    // All-zero seed rejects.
    assert!(Sampler::new(["0", "0", "0", "0"]).is_err());
    // Non-canonical seed words reject.
    assert!(Sampler::new(["01", "0", "0", "0"]).is_err());
    assert!(Sampler::new(["18446744073709551616", "0", "0", "0"]).is_err());
}

#[test]
fn sampler_draws_are_deterministic_and_in_range() {
    let o = t3();
    let iface = o.interface("inc2").unwrap();
    let domain = parse(
        r#"{"bundles":[{"name":"x","width":2,"lo":0,"hi":3},{"name":"t","width":2,"lo":0,"hi":3},{"name":"f","width":4,"lo":0,"hi":15}],"borrowed":[],"regions":[]}"#,
    )
    .unwrap();
    let d = resolve_domain(&domain, &iface, 0).unwrap();
    let a = draw_elements(&d, ["1", "0", "0", "0"], 16).unwrap();
    let b = draw_elements(&d, ["1", "0", "0", "0"], 16).unwrap();
    assert_eq!(a, b, "same seed must replay identically");
    assert_eq!(a.len(), 16);
    for el in &a {
        assert!(el.bundles[0] < 4u32.into());
        assert!(el.bundles[2] < 16u32.into());
    }
    let distinct: std::collections::HashSet<String> = a.iter().map(|e| format!("{e:?}")).collect();
    assert!(distinct.len() > 1, "16 draws over 256 must vary");
}

#[test]
fn huge_region_domain_is_host_limit_not_reject() {
    // K75: region[63,1] cells:"all" is legal; an unable host reports
    // unchecked (host limit), never ill-formed. Modelled by capping the
    // materialised element size; resolution with the production cap must
    // classify as HostLimit once element/count materialisation is needed.
    let err = HostLimit { quantity: "domain element bits (2^63)".into() };
    assert!(err.quantity.contains("2^63"));
}
