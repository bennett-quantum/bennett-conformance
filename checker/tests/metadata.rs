mod common;
use bennett_cert_checker::cert::check_certificate;
use bennett_cert_checker::json::{jcs, Value};
use bennett_cert_checker::object::Object;
use common::*;

#[test]
fn object_metadata_sections_require_jcs_bytes() {
    let inp = controlled();
    let changed = inp.object_text.replacen("\"0\":", "\"0\" :", 1);
    assert!(
        Object::parse(&changed).is_err(),
        "non-JCS object symbol metadata was accepted"
    );
}
#[test]
fn unsafe_json_metadata_numbers_reject_after_all_hashes_are_rebound() {
    for (part, code) in [
        ("certificate", "E_CERT_SCHEMA"),
        ("target", "E_CERT_TARGET"),
        ("schedule", "E_CERT_SCHEDULE_BINDING"),
    ] {
        let mut inp = controlled();
        let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        let number = Value::Num("9007199254740992".into());
        match part {
            "certificate" => {
                let Value::Obj(top) = &mut cert else {
                    unreachable!()
                };
                let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
                    unreachable!()
                };
                let Value::Obj(region) = regions.values_mut().next().unwrap() else {
                    unreachable!()
                };
                let Value::Arr(claims) = region.get_mut("claims").unwrap() else {
                    unreachable!()
                };
                set(&mut claims[0], "bound", number);
            }
            "target" => {
                let Value::Obj(top) = &mut target else {
                    unreachable!()
                };
                let Value::Obj(classes) = top.get_mut("classes").unwrap() else {
                    unreachable!()
                };
                set(classes.get_mut("fast").unwrap(), "dirty_rate", number);
            }
            _ => {
                let Value::Obj(top) = &mut schedule else {
                    unreachable!()
                };
                set(top.get_mut("bindings").unwrap(), "epsilon", number);
            }
        }
        inp.cert_bytes = jcs(&cert).unwrap();
        bind_schedule(&mut inp, schedule, target);
        let verdict = check_certificate(&inp);
        assert_eq!(verdict.overall, "rejected", "{part}: {verdict:?}");
        assert!(codes(&verdict).contains(&code), "{part}: {verdict:?}");
    }
}
#[test]
fn object_metadata_safe_integer_limit_does_not_apply_to_decimal_strings() {
    let inp = controlled();
    for encoded in ["9007199254740992", r#""9007199254740992""#] {
        let object=format!("{}\nsection coverage\n{{\"entries\":[{{\"classification\":\"lowered\",\"count\":{encoded},\"point\":[],\"reason\":\"\",\"source\":\"example\"}}]}}\n",inp.object_text);
        let result = Object::parse(&object);
        assert_eq!(
            result.is_ok(),
            encoded.starts_with('"'),
            "{encoded}: {result:?}"
        );
    }
}

#[test]
fn canonical_integer_strings_keep_signature_widths_regions_and_bounds_exact() {
    let (oracle,_) = standalone(
        "oracle @o (x:wire[4] in) (t:wire[4]) (f:wire[4]) (db:region[0,8] in) ()\n^entry():\nlift.retval x",
        r#"{"o":{"abi":"0.9","args":[],"fault":"4","in":["4"],"kind":"oracle","mem":[{"A":"0","W":"8","role":"in"}],"tgt":"4","tier":"q"}}"#,
        "o",vec![],
    );
    let object = Object::parse(&oracle.object_text).unwrap();
    let iface = object.interface("o").unwrap();
    assert_eq!(
        iface.bundles.iter().map(|b| b.width).collect::<Vec<_>>(),
        vec![4, 4, 4]
    );
    assert_eq!((iface.regions[0].a, iface.regions[0].w), (0, 8));
    let (action,_) = standalone(
        "action @a (t:wire mut) (c:wire in) () () ()\nregs {  } bounds {2}\n^entry():\nqloop 2 {\ncx c t\n}",
        r#"{"a":{"abi":"0.9","args":[],"bounds":["2"],"in":["1"],"kind":"action","mem":[],"mut":["1"],"tier":"q"}}"#,
        "a",vec![],
    );
    let object = Object::parse(&action.object_text).unwrap();
    let iface = object.interface("a").unwrap();
    assert_eq!(iface.bounds, vec![2]);
    assert_eq!(iface.bundles.len(), 2);
    let verdict = check_certificate(&action);
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
}

#[test]
fn metadata_unsigned_integer_decoding_never_truncates_or_accepts_noncanonical_strings() {
    use bennett_cert_checker::cert::u64_of;
    for (text, want) in [
        ("0", Some(0)),
        ("9007199254740992", Some(1 << 53)),
        ("18446744073709551615", Some(u64::MAX)),
        ("18446744073709551616", None),
        ("340282366920938463463374607431768211456", None),
        ("-1", None),
        ("01", None),
        ("+1", None),
    ] {
        assert_eq!(u64_of(&Value::Str(text.into())), want, "{text}");
    }
}

#[test]
fn a_legal_wide_string_signature_reaches_a_domain_host_limit_without_allocation() {
    let claim = json(
        r#"{"kind":"exhaustive","domain":{"bundles":[],"borrowed":[],"regions":[]},"invocation":{"args":[],"budget":["0","0","0","0"]},"expected":{"table":[{"before":{"bundles":["0"],"borrowed":[],"regions":[]},"after":{"bundles":["0"],"borrowed":[],"regions":[]}}]}}"#,
    );
    let (inputs, _) = standalone(
        "action @a (t:wire[9007199254740992] mut) () () () ()\n^entry():\nx t[0]",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":["9007199254740992"],"tier":"q"}}"#,
        "a",
        vec![claim],
    );
    let verdict = check_certificate(&inputs);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "unchecked", "{claim:?}");
    assert!(claim
        .unchecked_reason
        .as_deref()
        .unwrap()
        .contains("host limit"));
    assert!(claim.elements.is_none());
}

fn beyond_host_width(
    signature_width: &str,
    source_width: &str,
) -> bennett_cert_checker::cert::Inputs {
    let (mut inp, old) = standalone(
        "action @a (t:wire mut) () () () ()\n^entry():\nx t[0]",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a",
        vec![],
    );
    let mut symbols = json(
        inp.object_text
            .split("section symbols\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    );
    let Value::Obj(table) = &mut symbols else {
        unreachable!()
    };
    let entry = table.get_mut("0").unwrap();
    let mut signature = entry.get("signature").unwrap().clone();
    set(
        &mut signature,
        "mut",
        Value::Arr(vec![Value::Str(signature_width.into())]),
    );
    set(entry, "signature", signature.clone());
    set(entry, "interface_hash", Value::Str("0".repeat(64)));
    let header = inp.object_text.lines().next().unwrap().to_string();
    let make = |symbols: &Value| {
        format!("{header}\n\nsection symbols\n{}\n\nsection functions\naction @a (t:wire[{source_width}] mut) () () () ()\n^entry():\nx t[0]\n",encode(symbols))
    };
    let initial = Object::parse(&make(&symbols)).unwrap();
    let Value::Obj(table) = &mut symbols else {
        unreachable!()
    };
    set(
        table.get_mut("0").unwrap(),
        "interface_hash",
        Value::Str(initial.symbol("a").unwrap().interface_hash.clone()),
    );
    inp.object_text = make(&symbols);
    let object = Object::parse(&inp.object_text).unwrap();
    assert!(object.stated_hashes_ok());
    let root = object.symbol("a").unwrap();
    let hash = root.closure_hash.as_ref().unwrap();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let bindings = top.get_mut("bindings").unwrap();
    for key in ["closure_hash", "interface_hash", "reference_program_hash"] {
        set(bindings, key, Value::Str(hash.clone()));
    }
    set(
        bindings,
        "object_hash",
        Value::Str(object.object_hash.clone()),
    );
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let mut region = regions.remove(&old).unwrap();
    set(&mut region, "signature", signature);
    let mut frame = region.get("frame").unwrap().clone();
    let Value::Obj(f) = &mut frame else {
        unreachable!()
    };
    let Value::Arr(slots) = f.get_mut("slots").unwrap() else {
        unreachable!()
    };
    set(&mut slots[0], "width", Value::Str(signature_width.into()));
    set(&mut region, "frame", frame);
    regions.insert(hash.clone(), region);
    inp.cert_bytes = jcs(&cert).unwrap();
    inp
}

#[test]
fn sizes_beyond_u64_are_host_limits_only_when_the_source_agrees() {
    for width in [
        "18446744073709551616",
        "340282366920938463463374607431768211456",
    ] {
        let verdict = check_certificate(&beyond_host_width(width, width));
        assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
        assert!(codes(&verdict).is_empty());
        assert!(
            verdict
                .unchecked_items
                .iter()
                .any(|s| s.contains("host limit") && s.contains("u64 interface")),
            "{verdict:?}"
        );
        assert!(verdict.regions.is_empty());
        let wrong = check_certificate(&beyond_host_width(width, "1"));
        assert!(codes(&wrong).contains(&"E_CERT_OBJECT"), "{wrong:?}");
    }
}

#[test]
fn source_interfaces_require_positive_widths_fault_width_four_and_matching_kinds() {
    let (action, _) = standalone(
        "action @a (t:wire mut) () () () ()\n^entry():\nx t",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a",
        vec![],
    );
    let (oracle, _) = standalone(
        "oracle @o (x:wire[4] in) (t:wire[4]) (f:wire[4]) () ()\n^entry():\nlift.retval x",
        r#"{"o":{"abi":"0.9","args":[],"fault":4,"in":[4],"kind":"oracle","mem":[],"tgt":4,"tier":"q"}}"#,
        "o",
        vec![],
    );
    let cases = [
        (
            action
                .object_text
                .replacen("action @a", "perm_action @a", 1),
            "a",
        ),
        (
            action
                .object_text
                .replacen(r#""mut":[1]"#, r#""mut":[0]"#, 1)
                .replacen("t:wire mut", "t:wire[0] mut", 1),
            "a",
        ),
        (
            oracle
                .object_text
                .replacen(r#""fault":4"#, r#""fault":3"#, 1)
                .replacen("f:wire[4]", "f:wire[3]", 1),
            "o",
        ),
    ];
    for (source, name) in cases {
        let object = Object::parse(&source).unwrap();
        assert!(object.interface(name).is_err(), "{source}");
    }
}
