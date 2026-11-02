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

fn counted_fixture(body: &str, start: usize) -> (bennett_cert_checker::cert::Inputs, String) {
    use bennett_cert_checker::json::jcs;
    let claim = json(&format!(
        r#"{{"invocation":{{"args":"all","budget":"all"}},"kind":"structural","rule":"translation_lemma","subclaims":[],"witness":{{"B":["b[0]"],"X":["x"],"V":["entry:{start}"],"W":["entry:{}"],"frame":["entry:{start}","entry:{}"]}}}}"#,
        start + 1,
        start + 3
    ));
    let source=format!("perm_action @borrow_demo (t:wire) (x:wire in) () () ()\nregs {{  }} bounds {{1}}\n^entry():\n{body}");
    let (mut inp, h) = standalone(
        &source,
        r#"{"borrow_demo":{"abi":"0.9","args":[],"bounds":[1],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "borrow_demo",
        vec![claim],
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    set(regions.get_mut(&h).unwrap(), "bounds", json("[1]"));
    inp.cert_bytes = jcs(&cert).unwrap();
    (inp, h)
}
#[test]
fn normative_borrow_demo_translation_frame_inside_counted_loop() {
    let (inp, h) = counted_fixture(
        "b = qborrow any 1\nqloop 1 {\ncx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t\n}\nqreturn b",
        2,
    );
    assert_eq!(
        h, "6f4782e9bde4267acd1afddd650093722a056d9d50ee465b79cb86a4606b2a94",
        "published §16.3 example 3 identity"
    );
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    assert_eq!(v.regions[0].claims[0].1.outcome, "accept", "{v:?}");
}
#[test]
fn translation_maximality_stays_in_its_containing_static_loop_sequence() {
    for (body,start,expected) in [
        ("b = qborrow any 1\ncx x b[0]\nqloop 1 {\ncx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t\n}\ncx x b[0]\nqreturn b",3,"accept"),
        ("b = qborrow any 1\nqloop 1 {\ncx x b[0]\ncx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t\n}\nqreturn b",3,"reject"),
    ] {
        let (inp,_)=counted_fixture(body,start);let v=check_certificate(&inp);assert_eq!(v.regions[0].claims[0].1.outcome,expected,"{v:?}");
        if expected=="reject" {assert!(codes(&v).contains(&"E_CERT_STRUCTURAL"),"{v:?}");}else{assert!(codes(&v).is_empty(),"{v:?}");}
    }
}

#[test]
fn translation_loop_frame_rejects_non_x_gates() {
    let (inp,_)=counted_fixture("b = qborrow any 1\nqloop 1 {\ncx x b[0]\nswap b[0] t\ncx x b[0]\nswap b[0] t\n}\nqreturn b",2);
    let v = check_certificate(&inp);
    assert!(codes(&v).contains(&"E_CERT_STRUCTURAL"), "{v:?}");
}

#[test]
fn normative_borrow_demo_and_printed_adjoint_replay_all_8_and_16_basis_inputs() {
    use bennett_cert_checker::domain::Tuple;
    use bennett_cert_checker::eval::eval_action_tuple;
    use bennett_cert_checker::object::Object;
    use num_bigint::BigUint;
    let mut checked = 0;
    for pattern in [
        "cx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t",
        "cx b[0] t\ncx x b[0]\ncx b[0] t\ncx x b[0]",
    ] {
        let (inp, _) = counted_fixture(
            &format!("b = qborrow any 1\nqloop 1 {{\n{pattern}\n}}\nqreturn b"),
            2,
        );
        let object = Object::parse(&inp.object_text).unwrap();
        let symbol = object.symbol("borrow_demo").unwrap();
        let iface = object.interface("borrow_demo").unwrap();
        for controls in 0..=1usize {
            for control in 0..(1u32 << controls) {
                for t in 0..2u32 {
                    for x in 0..2u32 {
                        for b in 0..2u32 {
                            let mut bundles = if controls == 1 {
                                vec![BigUint::from(control)]
                            } else {
                                vec![]
                            };
                            bundles.extend([BigUint::from(t), BigUint::from(x)]);
                            let before = Tuple {
                                bundles,
                                borrowed: vec![b.into()],
                                regions: vec![],
                                args: vec![],
                            };
                            let mut expected = before.clone();
                            expected.bundles[controls] =
                                BigUint::from(t ^ (x & u32::from(controls == 0 || control == 1)));
                            let actual = eval_action_tuple(
                                &object,
                                symbol,
                                &iface,
                                &before,
                                &[],
                                controls,
                                false,
                                0,
                            )
                            .unwrap();
                            assert_eq!(
                                actual, expected,
                                "control count {controls}, control {control}, t={t}, x={x}, b={b}"
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        checked, 48,
        "both printed orientations, every input and borrowed value"
    );
}
