use crate::common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::{jcs, Value},
    sha256_hex,
};
use common::*;
fn fixture() -> Inputs {
    let rows=(0..4).map(|v|json(&format!(r#"{{"before":{{"bundles":["{v}"],"borrowed":[],"regions":[]}},"after":{{"bundles":["{}"],"borrowed":[],"regions":[]}}}}"#,v^3))).collect();
    let mut claim = json(
        r#"{"kind":"exhaustive","label":"proof","invocation":{"args":[],"budget":["0","0","0","0"]},"domain":{"bundles":[{"name":"t","width":2,"lo":0,"hi":3}],"borrowed":[],"regions":[]},"expected":{}}"#,
    );
    let mut expected = json("{}");
    set(&mut expected, "table", Value::Arr(rows));
    set(&mut claim, "expected", expected);
    let (mut inputs, h) = standalone(
        "perm_action @split (t:wire[2] mut) () () () ()\n^entry():\nx t[0]\nx t[1]",
        r#"{"split":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[2],"tier":"q"}}"#,
        "split",
        vec![claim],
    );
    let base = controlled();
    let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    set(
        &mut target,
        "classes",
        json(
            r#"{"A":{"capacity":2,"dirty_rate":1,"qubit_second":1},"B":{"capacity":2,"dirty_rate":1,"qubit_second":1}}"#,
        ),
    );
    let mut gate = target.get("gates").unwrap().as_arr().unwrap()[0].clone();
    set(&mut gate, "classes", json(r#"["*"]"#));
    set(&mut gate, "controls", json(r#"{"min":0,"max":0}"#));
    set(&mut target, "gates", Value::Arr(vec![gate]));
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    set(
        top.get_mut("bindings").unwrap(),
        "target_hash",
        Value::Str(sha256_hex(&jcs(&target).unwrap())),
    );
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    set(
        regions.get_mut(&h).unwrap(),
        "frame",
        json(
            r#"{"slots":[{"id":"mut:0","role":"mut","width":2,"fragments":[{"offset":0,"width":1,"class":"A"},{"offset":1,"width":1,"class":"B"}]}],"scratch":[],"history":[],"borrow":[],"ancillae":[]}"#,
        ),
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
    inputs.target_bytes = Some(jcs(&target).unwrap());
    inputs
}
fn mutate(inputs: &mut Inputs, f: impl FnOnce(&mut Value)) {
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let Value::Obj(r) = regions.values_mut().next().unwrap() else {
        panic!()
    };
    let Value::Obj(frame) = r.get_mut("frame").unwrap() else {
        panic!()
    };
    let Value::Arr(slots) = frame.get_mut("slots").unwrap() else {
        panic!()
    };
    f(&mut slots[0]);
    inputs.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn normative_two_class_fragmented_slot_has_one_source_identity_and_replays_all_four_inputs() {
    let verdict = check_certificate(&fixture());
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(
        verdict.regions[0].claims[0].1.outcome, "proof",
        "{verdict:?}"
    );
    assert_eq!(
        verdict.overall, "unchecked",
        "class/placement binding is not yet proved: {verdict:?}"
    );
    assert!(
        verdict.regions[0]
            .unchecked_items
            .iter()
            .any(|x| x.contains("fragmented")),
        "{verdict:?}"
    );
}
#[test]
fn fragment_gaps_overlaps_order_classes_merge_and_exclusive_forms_reject() {
    for text in [
        r#"[{"offset":1,"width":1,"class":"A"},{"offset":2,"width":1,"class":"B"}]"#,
        r#"[{"offset":0,"width":1,"class":"A"},{"offset":0,"width":1,"class":"B"}]"#,
        r#"[{"offset":1,"width":1,"class":"B"},{"offset":0,"width":1,"class":"A"}]"#,
        r#"[{"offset":0,"width":1,"class":"A"},{"offset":1,"width":2,"class":"B"}]"#,
        r#"[{"offset":0,"width":1,"class":"A"},{"offset":1,"width":1,"class":"A"}]"#,
        r#"[{"offset":0,"width":1,"class":"A"},{"offset":1,"width":1,"class":"C"}]"#,
        r#"[{"offset":0,"width":2,"class":"A"}]"#,
        r#"[{"offset":0,"width":0,"class":"A"},{"offset":0,"width":2,"class":"B"}]"#,
        "[]",
    ] {
        let mut inputs = fixture();
        mutate(&mut inputs, |slot| set(slot, "fragments", json(text)));
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_FRAME"),
            "{text}: {verdict:?}"
        );
    }
    let mut inputs = fixture();
    mutate(&mut inputs, |slot| {
        set(slot, "class", Value::Str("A".into()))
    });
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).contains(&"E_CERT_FRAME"), "{verdict:?}");
}
#[test]
fn every_fragment_field_type_position_and_required_removal_rejects() {
    let mut mutations = 0;
    let mut removals = 0;
    let mut inputs = fixture();
    mutate(&mut inputs, |slot| set(slot, "fragments", Value::Null));
    assert!(codes(&check_certificate(&inputs)).contains(&"E_CERT_FRAME"));
    mutations += 1;
    for i in 0..2 {
        let mut inputs = fixture();
        mutate(&mut inputs, |slot| {
            let Value::Obj(m) = slot else { panic!() };
            let Value::Arr(a) = m.get_mut("fragments").unwrap() else {
                panic!()
            };
            a[i] = Value::Null;
        });
        assert!(codes(&check_certificate(&inputs)).contains(&"E_CERT_FRAME"));
        mutations += 1;
        for key in ["offset", "width", "class"] {
            for remove in [false, true] {
                let mut inputs = fixture();
                mutate(&mut inputs, |slot| {
                    let Value::Obj(m) = slot else { panic!() };
                    let Value::Arr(a) = m.get_mut("fragments").unwrap() else {
                        panic!()
                    };
                    let Value::Obj(f) = &mut a[i] else { panic!() };
                    if remove {
                        f.remove(key);
                    } else {
                        f.insert(key.into(), Value::Null);
                    }
                });
                let verdict = check_certificate(&inputs);
                assert!(
                    codes(&verdict).contains(&"E_CERT_FRAME"),
                    "i={i} key={key} remove={remove}: {verdict:?}"
                );
                if remove {
                    removals += 1;
                } else {
                    mutations += 1;
                }
            }
        }
    }
    let mut inputs = fixture();
    mutate(&mut inputs, |slot| {
        let Value::Obj(m) = slot else { panic!() };
        m.remove("fragments");
    });
    assert!(codes(&check_certificate(&inputs)).contains(&"E_CERT_FRAME"));
    removals += 1;
    assert_eq!((mutations, removals), (9, 7));
}
