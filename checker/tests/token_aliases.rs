mod common;
use bennett_cert_checker::{
    cert::{check_certificate, parse_invocation, Inputs, Limits},
    json::jcs,
    object::Object,
    sha256_hex,
};
use common::*;
fn object() -> Object {
    Object::parse(&controlled().object_text).unwrap()
}
#[test]
fn equal_tuple_action_tokens_admit_every_canonical_partition_without_tuple_interning() {
    let object = object();
    let types = vec!["token".to_string(); 3];
    for aliases in ["[0,0,0]", "[0,0,1]", "[0,1,0]", "[0,1,1]", "[0,1,2]"] {
        let invocation = json(&format!(
            r#"{{"args":[["1","0","0","0"],["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":{aliases}}}"#
        ));
        let parsed = parse_invocation(&invocation, &types, &object, "test");
        assert!(parsed.is_ok(), "{aliases}: {parsed:?}");
    }
}
#[test]
fn token_partitions_reject_bad_labels_arity_missing_values_and_inconsistent_equalities() {
    let object = object();
    let types = vec!["token".to_string(); 2];
    for aliases in [
        "null", "[1,1]", "[0,2]", "[0,-1]", "[]", "[0]", "[0,1,2]", "[0,true]",
    ] {
        let invocation = json(&format!(
            r#"{{"args":[["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":{aliases}}}"#
        ));
        let errors = parse_invocation(&invocation, &types, &object, "test").unwrap_err();
        assert!(
            errors.iter().any(|e| e.code == "E_CERT_INVOCATION"),
            "{aliases}: {errors:?}"
        );
    }
    for invocation in [
        r#"{"args":[["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"]}"#,
        r#"{"args":[["1","0","0","0"],["2","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":[0,0]}"#,
    ] {
        let errors = parse_invocation(&json(invocation), &types, &object, "test").unwrap_err();
        assert!(
            errors.iter().any(|e| e.code == "E_CERT_INVOCATION"),
            "{errors:?}"
        );
    }
}
#[test]
fn published_k91_reference_certificates_bind_identity_partitions_and_replay_all_sixty_four_tuples()
{
    let root = pins::isa_spec_dir().join("conformance/fixtures/accepted-k91-09");
    let manifest = json(&std::fs::read_to_string(root.join("k91.json")).unwrap());
    let mut count = 0;
    for name in ["pair-same", "pair-distinct", "wrong-partition-certificate"] {
        let case = manifest
            .get("fixtures")
            .unwrap()
            .as_arr()
            .unwrap()
            .iter()
            .find(|c| c.get("name").unwrap().as_str() == Some(name))
            .unwrap();
        let bytes =
            std::fs::read(root.join(case.get("object").unwrap().as_str().unwrap())).unwrap();
        assert_eq!(
            sha256_hex(&bytes),
            case.get("sha256").unwrap().as_str().unwrap()
        );
        let object_text = String::from_utf8(bytes).unwrap();
        let object = Object::parse(&object_text).unwrap();
        assert!(object.stated_hashes_ok());
        let cert = json(object.certificate_raw.as_deref().unwrap());
        let inputs = Inputs {
            cert_bytes: jcs(&cert).unwrap(),
            object_text,
            target_bytes: None,
            schedule_bytes: None,
            libraries: vec![],
            oracle_bin: None,
            limits: Limits::default(),
        };
        let verdict = check_certificate(&inputs);
        if name == "wrong-partition-certificate" {
            assert!(codes(&verdict).contains(&"E_CERT_BINDING"), "{verdict:?}");
        } else {
            assert!(codes(&verdict).is_empty(), "{name}: {verdict:?}");
            let outcome = &verdict.regions[0].claims[0].1;
            assert_eq!(outcome.outcome, "proof", "{name}: {verdict:?}");
            assert_eq!(outcome.elements.as_deref(), Some("64"));
        }
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn oracle_partitions_prepend_primary_identity_and_ignore_non_token_arguments() {
    use bennett_cert_checker::cert::parse_invocation_for_kind;
    let object = object();
    let types = vec!["token".into(), "i2".into(), "token".into()];
    let mut hashes = std::collections::HashSet::new();
    for labels in ["[0,0,0]", "[0,0,1]", "[0,1,0]", "[0,1,1]", "[0,1,2]"] {
        let invocation = json(&format!(
            r#"{{"args":[["1","0","0","0"],"2",["1","0","0","0"]],"budget":["1","0","0","0"],"token_aliases":{labels}}}"#
        ));
        let parsed =
            parse_invocation_for_kind(&invocation, &types, &object, "oracle", "test").unwrap();
        assert_eq!(parsed.token_aliases.len(), 3);
        hashes.insert(parsed.params_hash);
    }
    assert_eq!(hashes.len(), 1, "params_hash excludes the partition");
    let incompatible = json(
        r#"{"args":[["1","0","0","0"],"2",["1","0","0","0"]],"budget":["2","0","0","0"],"token_aliases":[0,0,0]}"#,
    );
    assert!(parse_invocation_for_kind(&incompatible, &types, &object, "oracle", "test").is_err());
    for (kind, expected) in [
        ("oracle", vec![0]),
        ("action", vec![]),
        ("perm_action", vec![]),
        ("cc", vec![]),
        ("r", vec![]),
    ] {
        let parsed = parse_invocation_for_kind(
            &json(r#"{"args":[],"budget":["0","0","0","0"]}"#),
            &[],
            &object,
            kind,
            "test",
        )
        .unwrap();
        assert_eq!(parsed.token_aliases, expected);
    }
}
#[test]
fn native_action_contexts_forward_identity_by_source_bindings_and_recanonicalize_each_child() {
    use bennett_cert_checker::cert::LoadedDocs;
    let source="perm_action @parent (t:wire mut) () () (a:token, b:token, c:token) ()\n^entry():\naction_call @child (t) () () (c, b, c, a)\nperm_action @child (v:wire mut) () () (x:token, y:token, z:token, w:token) ()\n^entry():\nx v";
    let sig = r#"{"parent":{"abi":"0.9","args":["token","token","token"],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"child":{"abi":"0.9","args":["token","token","token","token"],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let (inputs, h) = standalone(source, sig, "parent", vec![]);
    let mut params = std::collections::HashSet::new();
    for (labels, expected) in [("[0,1,0]", vec![0, 1, 0, 0]), ("[0,1,2]", vec![0, 1, 0, 2])] {
        let object = Object::parse(&inputs.object_text).unwrap();
        let inv = json(&format!(
            r#"{{"args":[["1","0","0","0"],["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":{labels}}}"#
        ));
        let parsed = parse_invocation(
            &inv,
            &object.interface("parent").unwrap().args,
            &object,
            "test",
        )
        .unwrap();
        let docs = LoadedDocs {
            root_closure: h.clone(),
            root_invocation: Some(parsed),
            object,
            target: None,
            schedule: Some(json(r#"{"bindings":{"inherited_controls":0}}"#)),
            schedule_hash: None,
            expansion: None,
            expansion_unknown: false,
        };
        let contexts = bennett_cert_checker::realisation::contexts(&docs, &inputs).unwrap();
        assert_eq!(contexts.len(), 2);
        let child = contexts.iter().find(|c| c.symbol == "child").unwrap();
        assert_eq!(child.invocation.token_aliases, expected);
        params.insert(child.invocation.params_hash.clone());
    }
    assert_eq!(
        params.len(),
        1,
        "same tuples, distinct forwarded identities"
    );
}
#[test]
fn every_published_partition_field_position_and_required_removal_has_its_expected_code() {
    let path = pins::isa_spec_dir().join("conformance/fixtures/accepted-k91-09/pair-same.bisa");
    let object_text = std::fs::read_to_string(path).unwrap();
    let object = Object::parse(&object_text).unwrap();
    let cert = json(object.certificate_raw.as_deref().unwrap());
    let schedule = object.schedule.clone().unwrap();
    let mut mutations = 0;
    let mut removals = 0;
    for scope in ["root", "claim", "schedule"] {
        for index in [None, Some(0), Some(1)] {
            let mut changed = cert.clone();
            let mut sched = schedule.clone();
            let value = match scope {
                "root" => {
                    let bennett_cert_checker::json::Value::Obj(top) = &mut changed else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(b) =
                        top.get_mut("bindings").unwrap()
                    else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(inv) =
                        b.get_mut("invocation").unwrap()
                    else {
                        panic!()
                    };
                    inv.get_mut("token_aliases").unwrap()
                }
                "claim" => {
                    let bennett_cert_checker::json::Value::Obj(top) = &mut changed else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(regions) =
                        top.get_mut("regions").unwrap()
                    else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(r) =
                        regions.values_mut().next().unwrap()
                    else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Arr(claims) =
                        r.get_mut("claims").unwrap()
                    else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(c) = &mut claims[0] else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(inv) =
                        c.get_mut("invocation").unwrap()
                    else {
                        panic!()
                    };
                    inv.get_mut("token_aliases").unwrap()
                }
                _ => {
                    let bennett_cert_checker::json::Value::Obj(top) = &mut sched else {
                        panic!()
                    };
                    let bennett_cert_checker::json::Value::Obj(b) =
                        top.get_mut("bindings").unwrap()
                    else {
                        panic!()
                    };
                    b.get_mut("token_aliases").unwrap()
                }
            };
            if let Some(i) = index {
                let bennett_cert_checker::json::Value::Arr(a) = value else {
                    panic!()
                };
                a[i] = bennett_cert_checker::json::Value::Null;
            } else {
                *value = bennett_cert_checker::json::Value::Null;
            }
            let sh = sha256_hex(&jcs(&sched).unwrap());
            let bennett_cert_checker::json::Value::Obj(top) = &mut changed else {
                panic!()
            };
            set(
                top.get_mut("bindings").unwrap(),
                "schedule_hash",
                bennett_cert_checker::json::Value::Str(sh),
            );
            let inputs = Inputs {
                cert_bytes: jcs(&changed).unwrap(),
                object_text: object_text.clone(),
                target_bytes: None,
                schedule_bytes: Some(jcs(&sched).unwrap()),
                libraries: vec![],
                oracle_bin: None,
                limits: Limits::default(),
            };
            let verdict = check_certificate(&inputs);
            let expected = if scope == "schedule" {
                "E_CERT_SCHEDULE_BINDING"
            } else {
                "E_CERT_INVOCATION"
            };
            assert!(
                codes(&verdict).contains(&expected),
                "{scope}/{index:?}: {verdict:?}"
            );
            mutations += 1;
        }
    }
    for scope in ["root", "claim", "schedule"] {
        let mut changed = cert.clone();
        let mut sched = schedule.clone();
        if scope == "schedule" {
            let bennett_cert_checker::json::Value::Obj(top) = &mut sched else {
                panic!()
            };
            let bennett_cert_checker::json::Value::Obj(b) = top.get_mut("bindings").unwrap() else {
                panic!()
            };
            b.remove("token_aliases");
        } else {
            let bennett_cert_checker::json::Value::Obj(top) = &mut changed else {
                panic!()
            };
            let value = if scope == "root" {
                let bennett_cert_checker::json::Value::Obj(b) = top.get_mut("bindings").unwrap()
                else {
                    panic!()
                };
                b.get_mut("invocation").unwrap()
            } else {
                let bennett_cert_checker::json::Value::Obj(regions) =
                    top.get_mut("regions").unwrap()
                else {
                    panic!()
                };
                let bennett_cert_checker::json::Value::Obj(r) =
                    regions.values_mut().next().unwrap()
                else {
                    panic!()
                };
                let bennett_cert_checker::json::Value::Arr(claims) = r.get_mut("claims").unwrap()
                else {
                    panic!()
                };
                let bennett_cert_checker::json::Value::Obj(c) = &mut claims[0] else {
                    panic!()
                };
                c.get_mut("invocation").unwrap()
            };
            let bennett_cert_checker::json::Value::Obj(inv) = value else {
                panic!()
            };
            inv.remove("token_aliases");
        }
        let sh = sha256_hex(&jcs(&sched).unwrap());
        let bennett_cert_checker::json::Value::Obj(top) = &mut changed else {
            panic!()
        };
        set(
            top.get_mut("bindings").unwrap(),
            "schedule_hash",
            bennett_cert_checker::json::Value::Str(sh),
        );
        let inputs = Inputs {
            cert_bytes: jcs(&changed).unwrap(),
            object_text: object_text.clone(),
            target_bytes: None,
            schedule_bytes: Some(jcs(&sched).unwrap()),
            libraries: vec![],
            oracle_bin: None,
            limits: Limits::default(),
        };
        let verdict = check_certificate(&inputs);
        let expected = if scope == "schedule" {
            "E_CERT_SCHEDULE_BINDING"
        } else {
            "E_CERT_INVOCATION"
        };
        assert!(
            codes(&verdict).contains(&expected),
            "remove {scope}: {verdict:?}"
        );
        removals += 1;
    }
    assert_eq!((mutations, removals), (9, 3));
}

#[test]
fn parameter_uniform_structural_proofs_quantify_all_consistent_token_partitions() {
    let source="perm_action @borrow_demo (t:wire mut) (x:wire in) () (a:token, b:token) ()\n^entry():\nb = qborrow fast 1\ncx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t\nqreturn b";
    // Use distinct local borrow and immutable argument names.
    let source = source.replacen("a:token, b:token", "a:token, permit:token", 1);
    let sig = r#"{"borrow_demo":{"abi":"0.9","args":["token","token"],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    for labels in [Some("all"), Some("concrete"), None] {
        let mut invocation = json(r#"{"args":"all","budget":"all"}"#);
        if let Some(labels) = labels {
            set(
                &mut invocation,
                "token_aliases",
                json(if labels == "all" { "\"all\"" } else { "[0,1]" }),
            );
        }
        let mut claim = json(
            r#"{"kind":"structural","rule":"translation_lemma","subclaims":[],"witness":{"B":["b[0]"],"X":["x"],"V":["entry:1"],"W":["entry:2"],"frame":["entry:1","entry:4"]}}"#,
        );
        set(&mut claim, "invocation", invocation);
        let (inputs, _) = standalone(&source, sig, "borrow_demo", vec![claim]);
        let verdict = check_certificate(&inputs);
        if labels == Some("all") {
            assert!(codes(&verdict).is_empty(), "{verdict:?}");
            assert_eq!(
                verdict.regions[0].claims[0].1.outcome, "accept",
                "{verdict:?}"
            );
        } else {
            assert!(
                codes(&verdict).contains(&"E_CERT_INVOCATION"),
                "{verdict:?}"
            );
        }
    }
}
#[test]
fn inverse_pair_entries_bind_primary_and_argument_identity_with_tuple_consistency() {
    let source="perm_action @pair (t:wire mut) () () (tk:token, a:token) ()\n^entry():\nqinplace @g @ginv tk t () () (a)\noracle @g (x:wire in) (t:wire) (f:wire[4]) () (a:token)\n^entry():\nlift.retval x\noracle @ginv (x:wire in) (t:wire) (f:wire[4]) () (a:token)\n^entry():\nlift.retval x";
    let sig = r#"{"pair":{"abi":"0.9","args":["token","token"],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"g":{"abi":"0.9","args":["token"],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"},"ginv":{"abi":"0.9","args":["token"],"fault":4,"in":[1],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#;
    for (budget, labels, valid) in [
        (1, Some("[0,0]"), true),
        (1, Some("[0,1]"), true),
        (2, Some("[0,0]"), false),
        (1, None, false),
    ] {
        let claim = json(
            r#"{"kind":"exhaustive","expected":"inverse_pair","domain":{"bundles":[{"name":"x","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]}}"#,
        );
        let (mut inputs, h) = standalone(source, sig, "pair", vec![claim]);
        let object = Object::parse(&inputs.object_text).unwrap();
        let g = object.symbol("g").unwrap().closure_hash.as_ref().unwrap();
        let gi = object
            .symbol("ginv")
            .unwrap()
            .closure_hash
            .as_ref()
            .unwrap();
        let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
        let bennett_cert_checker::json::Value::Obj(top) = &mut cert else {
            panic!()
        };
        let bennett_cert_checker::json::Value::Obj(regions) = top.get_mut("regions").unwrap()
        else {
            panic!()
        };
        let bennett_cert_checker::json::Value::Obj(r) = regions.get_mut(&h).unwrap() else {
            panic!()
        };
        let bennett_cert_checker::json::Value::Arr(claims) = r.get_mut("claims").unwrap() else {
            panic!()
        };
        set(
            &mut claims[0],
            "subject",
            json(&format!(
                r#"{{"closure_hash":"{g}","variant":"forward","inherited_controls":0}}"#
            )),
        );
        let mut entry = json(&format!(
            r#"{{"g":"{g}","ginv":"{gi}","claim":0,"budget":[{budget},0,0,0],"arg_tokens":{{"a":[1,0,0,0]}}}}"#
        ));
        if let Some(labels) = labels {
            set(&mut entry, "token_aliases", json(labels));
        }
        set(
            r.get_mut("verification").unwrap(),
            "inverse_pairs",
            bennett_cert_checker::json::Value::Arr(vec![entry]),
        );
        inputs.cert_bytes = jcs(&cert).unwrap();
        let verdict = check_certificate(&inputs);
        if valid {
            assert!(codes(&verdict).is_empty(), "{verdict:?}");
            let outcome = &verdict.regions[0].claims[0].1;
            assert_eq!(outcome.outcome, "proof");
            assert!(outcome.token_aliases.is_some());
        } else {
            assert!(
                codes(&verdict).contains(&"E_CERT_INVOCATION"),
                "{verdict:?}"
            );
        }
    }
}
