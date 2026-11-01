mod common;
use bennett_cert_checker::cert::check_certificate;
use bennett_cert_checker::json::Value;
use common::*;
fn fixture(
    body: &str,
    x: &str,
    b: &str,
    v: &str,
    w: &str,
    frame: &str,
) -> bennett_cert_checker::cert::Inputs {
    let mut claim = json(
        r#"{"invocation":{"args":"all","budget":"all"},"kind":"structural","rule":"translation_lemma","subclaims":[],"witness":{}}"#,
    );
    set(
        &mut claim,
        "witness",
        json(&format!(
            r#"{{"B":{b},"X":{x},"V":{v},"W":{w},"frame":{frame}}}"#
        )),
    );
    standalone(&format!("perm_action @borrow_demo (t:wire mut) (x:wire in) () () ()\n^entry():\nb = qborrow fast 1\n{body}\nqreturn b"),r#"{"borrow_demo":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,"borrow_demo",vec![claim]).0
}
#[test]
fn translation_lemma_proves_the_dirty_ancilla_pattern_without_enumeration() {
    let inp = fixture(
        "cx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t",
        r#"["x"]"#,
        r#"["b[0]"]"#,
        r#"["entry:1"]"#,
        r#"["entry:2"]"#,
        r#"["entry:1","entry:4"]"#,
    );
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    assert_eq!(v.regions[0].claims[0].1.outcome, "accept", "{v:?}");
}
#[test]
fn translation_lemma_rejects_wrong_shape_wrong_target_and_swap() {
    let patterns = [
        (
            "cx x b[0]\ncx b[0] t\ncx x b[0]",
            r#"["entry:1","entry:3"]"#,
        ),
        (
            "cx b[0] x\ncx b[0] t\ncx b[0] x\ncx b[0] t",
            r#"["entry:1","entry:4"]"#,
        ),
        (
            "cx x b[0]\nswap b[0] t\ncx x b[0]\nswap b[0] t",
            r#"["entry:1","entry:4"]"#,
        ),
    ];
    for (body, frame) in patterns {
        let inp = fixture(
            body,
            r#"["x"]"#,
            r#"["b[0]"]"#,
            r#"["entry:1"]"#,
            r#"["entry:2"]"#,
            frame,
        );
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_STRUCTURAL"), "{v:?}");
    }
}
#[test]
fn structural_unknown_rule_and_unbound_invocation_are_rejected() {
    for c in [
        r#"{"invocation":{"args":"all","budget":"all"},"kind":"structural","rule":"magic","subclaims":[],"witness":{}}"#,
        r#"{"kind":"structural","rule":"translation_lemma","subclaims":[],"witness":{"B":[],"V":[],"W":[],"X":[],"frame":[]}}"#,
    ] {
        let (inp, _) = standalone(
            "perm_action @identity (t:wire mut) () () () ()\n^entry():\nx t\nx t",
            r#"{"identity":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
            "identity",
            vec![json(c)],
        );
        let v = check_certificate(&inp);
        assert_eq!(v.overall, "rejected", "{v:?}");
    }
    let _ = Value::Null;
}
