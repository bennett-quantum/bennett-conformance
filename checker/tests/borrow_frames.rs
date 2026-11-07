//! ISA-12.1 / ISA-12.2: selected borrow frames remain explicit obligations.
use crate::common;
use bennett_cert_checker::{cert::check_certificate, json::Value};
use common::*;

fn inputs(mut claim: Value) -> bennett_cert_checker::cert::Inputs {
    // K89 witness: clean c, dirty b, identity on b when c=0.
    if claim.get("invocation").is_none() {
        set(
            &mut claim,
            "invocation",
            json(r#"{"args":[],"budget":["0","0","0","0"]}"#),
        );
    }
    standalone(
        "perm_action @a (t:wire mut) () () () ()\nregs {  } bounds {}\n^entry():\nc = qalloc any 1\nb = qborrow any 1\ncx c b\nqreturn b\nqfree c",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a", vec![claim],
    ).0
}

fn claim() -> Value {
    json(
        r#"{"kind":"exhaustive","expected":"borrow_frame",
    "borrow_frame":{"lifetime":0,"first":"entry:2","last":"entry:2"},
    "domain":{"bundles":[{"name":"t","width":1,"lo":0,"hi":1}],
    "borrowed":[{"name":"b","width":1,"lo":0,"hi":1}],"regions":[],
    "workspace":[{"allocation":"entry:0","offset":0,"width":1,"lo":0,"hi":0}]}}"#,
    )
}

#[test]
fn caq_selected_borrow_frame_products_are_unchecked_until_independently_derived() {
    for union in [false, true] {
        let mut c = claim();
        if union {
            let product = c.get("domain").unwrap().clone();
            let mut domain = json("{}");
            set(&mut domain, "entry_products", Value::Arr(vec![product]));
            set(&mut c, "domain", domain);
        }
        let verdict = check_certificate(&inputs(c));
        assert!(codes(&verdict).is_empty(), "{verdict:?}");
        assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
        let outcome = &verdict.regions[0].claims[0].1;
        assert_eq!(outcome.outcome, "unchecked", "{verdict:?}");
        assert!(outcome
            .unchecked_reason
            .as_deref()
            .unwrap()
            .contains("borrow-frame"));
        assert!(outcome.elements.is_none());
        assert!(outcome.covered_fraction.is_none());
    }
}

#[test]
fn caq_malformed_borrow_frame_bindings_reject_before_unsupported_replay() {
    let base = claim();
    for field in ["lifetime", "first", "last"] {
        let mut c = base.clone();
        let Value::Obj(frame) = c.as_obj().unwrap().get("borrow_frame").unwrap().clone() else {
            panic!()
        };
        let mut frame = frame;
        frame.remove(field);
        set(&mut c, "borrow_frame", Value::Obj(frame));
        let verdict = check_certificate(&inputs(c));
        assert_eq!(verdict.overall, "rejected", "{field}: {verdict:?}");
        assert!(codes(&verdict).contains(&"E_CERT_SCHEMA"), "{verdict:?}");
    }
    let mut c = base;
    set(&mut c, "invocation", json(r#"{"args":[],"budget":[]}"#));
    let verdict = check_certificate(&inputs(c));
    assert_eq!(verdict.overall, "rejected", "{verdict:?}");
    assert!(
        codes(&verdict).contains(&"E_CERT_INVOCATION"),
        "{verdict:?}"
    );
}
