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
        "action @a (t:wire mut) (c:wire in) () () ()\nregs { } bounds { 2 }\n^entry():\nqloop 2 {\ncx c t\n}",
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
