mod common;
use bennett_cert_checker::cert::check_certificate;
use common::*;

fn nested(depth: usize) -> bennett_cert_checker::cert::Inputs {
    let mut source = String::from("action @a (t:wire mut) (c:wire in) () () ()\n^entry():\n");
    source.push_str(&"qif c {\n".repeat(depth));
    source.push_str("x t\n");
    source.push_str(&"}\n".repeat(depth));
    standalone(&source,r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,"a",vec![]).0
}
#[test]
fn source_nesting_above_the_host_depth_cap_is_unchecked_before_recursive_analysis() {
    for depth in [65, 1000] {
        let verdict = check_certificate(&nested(depth));
        assert_eq!(verdict.overall, "unchecked", "depth={depth}: {verdict:?}");
        assert!(codes(&verdict).is_empty(), "{verdict:?}");
        assert!(
            verdict
                .unchecked_items
                .iter()
                .any(|r| r.contains("host limit") && r.contains("structured source nesting")),
            "{verdict:?}"
        );
        assert!(
            verdict.regions.is_empty(),
            "deep source must not reach recursive claim analysis"
        );
    }
}
#[test]
fn the_source_nesting_cap_is_not_an_isa_legality_limit() {
    let verdict = check_certificate(&nested(64));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert!(!verdict
        .unchecked_items
        .iter()
        .any(|r| r.contains("structured source nesting")));
}
