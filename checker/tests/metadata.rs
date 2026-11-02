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
