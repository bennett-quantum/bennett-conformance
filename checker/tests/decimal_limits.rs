use crate::common;
use bennett_cert_checker::cert::check_certificate;
use bennett_cert_checker::json::{jcs, Value};
use bennett_cert_checker::rat::parse_decimal;
use common::*;
#[test]
fn decimal_scale_never_wraps_at_u32_boundaries() {
    for text in [
        "1e4294967296",
        "1e-4294967296",
        "1e-9223372036854775808",
        "1e999999999999999999999999999999999999",
    ] {
        assert!(
            parse_decimal(text).is_err(),
            "scale silently truncated for {text}"
        );
    }
    assert_eq!(
        parse_decimal("0e99999999999999999999999999999").unwrap(),
        parse_decimal("0").unwrap()
    );
    for malformed in [
        "1e",
        "1e+",
        "1e--99999999999999999999",
        "01e99999999",
        ".1e99999999",
    ] {
        assert!(
            !parse_decimal(malformed).unwrap_err().contains("host limit"),
            "{malformed}"
        );
    }
}
#[test]
fn excessive_report_decimals_are_unchecked_host_limits() {
    for subject in ["certificate", "target", "schedule"] {
        let mut inp = controlled();
        match subject {
            "certificate" => {
                let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
                let Value::Obj(top) = &mut cert else {
                    unreachable!()
                };
                let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
                    unreachable!()
                };
                let Value::Obj(region) = regions.values_mut().next().unwrap() else {
                    unreachable!()
                };
                set(
                    region.get_mut("resources").unwrap(),
                    "time",
                    Value::Str(format!("1{}", "0".repeat(100_001))),
                );
                inp.cert_bytes = jcs(&cert).unwrap();
            }
            other => {
                let mut target =
                    json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
                let mut schedule =
                    json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
                if other == "target" {
                    let Value::Obj(top) = &mut target else {
                        unreachable!()
                    };
                    let Value::Arr(gates) = top.get_mut("gates").unwrap() else {
                        unreachable!()
                    };
                    let Value::Obj(gate) = &mut gates[0] else {
                        unreachable!()
                    };
                    set(
                        gate.get_mut("cost").unwrap(),
                        "time",
                        Value::Str("1e-4294967296".into()),
                    );
                } else {
                    let Value::Obj(top) = &mut schedule else {
                        unreachable!()
                    };
                    set(
                        top.get_mut("bindings").unwrap(),
                        "epsilon",
                        Value::Str("1e-4294967296".into()),
                    );
                }
                bind_schedule(&mut inp, schedule, target);
            }
        }
        let v = check_certificate(&inp);
        assert_eq!(v.overall, "unchecked", "{subject}: {v:?}");
        assert!(codes(&v).is_empty(), "{v:?}");
        assert!(
            v.unchecked_items
                .iter()
                .any(|s| s.contains("host limit") && s.contains(subject)),
            "{v:?}"
        );
    }
}

#[test]
fn numeric_looking_producer_and_scheduler_names_are_opaque_strings() {
    let mut inp = controlled();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    set(
        top.get_mut("producer").unwrap(),
        "name",
        Value::Str("1e99999999999".into()),
    );
    inp.cert_bytes = jcs(&cert).unwrap();
    let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(top) = &mut schedule else {
        unreachable!()
    };
    let Value::Obj(bindings) = top.get_mut("bindings").unwrap() else {
        unreachable!()
    };
    set(
        bindings.get_mut("scheduler").unwrap(),
        "name",
        Value::Str("1e99999999999".into()),
    );
    let target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inp, schedule, target);
    let v = check_certificate(&inp);
    assert_eq!(v.overall, "verified", "{v:?}");
}
