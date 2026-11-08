use crate::common;
use bennett_cert_checker::cert::{check_certificate, Inputs};
use bennett_cert_checker::json::{jcs, Value};
use common::*;

fn fixture(named: bool, mutate: impl FnOnce(&mut Value, &mut Value)) -> Inputs {
    let mut inputs = controlled();
    let mut target = json(std::str::from_utf8(inputs.target_bytes.as_ref().unwrap()).unwrap());
    let mut schedule = json(std::str::from_utf8(inputs.schedule_bytes.as_ref().unwrap()).unwrap());
    set(
        &mut target,
        "gates",
        json(
            r#"[{"op":"cx","classes":["fast"],"controls":{"min":1,"max":1},"epsilon":0,"cost":{"time":{"var":"k"}}}]"#,
        ),
    );
    set(
        &mut target,
        "schemes",
        json(
            r#"[{"name":"controlled-x","realises":{"op":"x","controls":{"min":1,"max":1}},"ancillae":{"count":{"op":"-","args":[{"var":"k"},{"var":"k"}]},"state":"clean","class":"fast"},"template":[{"gate":"cx","wires":["c[0]","t"]}]}]"#,
        ),
    );
    if named {
        let Value::Obj(top) = &mut schedule else {
            unreachable!()
        };
        let Value::Arr(entries) = top.get_mut("entries").unwrap() else {
            unreachable!()
        };
        let Value::Obj(entry) = &mut entries[0] else {
            unreachable!()
        };
        set(
            entry.get_mut("gate").unwrap(),
            "realisation",
            Value::Str("controlled-x".into()),
        );
    } else {
        set(&mut schedule, "entries", Value::Arr(vec![]));
    }
    mutate(&mut target, &mut schedule);
    bind_schedule(&mut inputs, schedule, target);
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    for region in regions.values_mut() {
        set(region, "gates_used", json(r#"{"cx":{"count":1}}"#));
    }
    inputs.cert_bytes = jcs(&cert).unwrap();
    inputs
}
#[test]
fn exact_single_leaf_schemes_verify_default_and_named_realisation() {
    for named in [false, true] {
        let verdict = check_certificate(&fixture(named, |_, _| {}));
        assert_eq!(verdict.overall, "verified", "named={named}: {verdict:?}");
        assert!(codes(&verdict).is_empty());
    }
}
#[test]
fn scheme_leaf_cost_uses_its_own_controls_and_preserves_source_predicate_depth() {
    let inputs = fixture(true, |target, _| {
        let Value::Obj(t) = target else {
            unreachable!()
        };
        set(
            t.get_mut("capabilities").unwrap(),
            "max_predicate_depth",
            json("0"),
        );
    });
    let verdict = check_certificate(&inputs);
    assert!(
        codes(&verdict).contains(&"E_SCHEDULE_INFEASIBLE"),
        "{verdict:?}"
    );
}
#[test]
fn malformed_or_incorrect_exact_scheme_leaves_are_rejected() {
    for (wires, gate, code) in [
        (r#"["c[1]","t"]"#, "cx", "E_CERT_TARGET"),
        (r#"["c[-1]","t"]"#, "cx", "E_CERT_TARGET"),
        (r#"["t","c[0]"]"#, "cx", "E_CERT_TARGET"),
        (r#"["c[0]","c[0]"]"#, "cx", "E_CERT_TARGET"),
        (r#"["a[0]","t"]"#, "cx", "E_CERT_TARGET"),
        (r#"["c[0]","t"]"#, "h", "E_CERT_TARGET"),
        (r#"["t"]"#, "s", "E_CERT_TARGET"),
    ] {
        let inputs = fixture(true, |target, _| {
            let Value::Obj(t) = target else {
                unreachable!()
            };
            let Value::Arr(schemes) = t.get_mut("schemes").unwrap() else {
                unreachable!()
            };
            set(
                &mut schemes[0],
                "template",
                json(&format!(r#"[{{"gate":"{gate}","wires":{wires}}}]"#)),
            );
        });
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&code),
            "{gate} {wires}: {verdict:?}"
        );
    }
}
#[test]
fn schemes_cannot_nest_even_when_the_second_scheme_matches() {
    let inputs = fixture(true, |target, _| {
        set(target, "gates", Value::Arr(vec![]));
        let Value::Obj(t) = target else {
            unreachable!()
        };
        let Value::Arr(schemes) = t.get_mut("schemes").unwrap() else {
            unreachable!()
        };
        schemes.push(json(r#"{"name":"second","realises":{"op":"cx","controls":{"min":1,"max":1}},"ancillae":{"count":0,"state":"clean","class":"fast"},"template":[{"gate":"cx","wires":["c[0]","t"]}]}"#));
    });
    let verdict = check_certificate(&inputs);
    assert!(
        codes(&verdict).contains(&"E_SCHEDULE_INFEASIBLE"),
        "{verdict:?}"
    );
}

fn field_mutants(value: &Value) -> Vec<(String, Value)> {
    let invalid = |v: &Value| {
        if *v == Value::Null {
            Value::Bool(true)
        } else {
            Value::Null
        }
    };
    let mut out = Vec::new();
    match value {
        Value::Obj(map) => {
            for (key, child) in map {
                let mut changed = value.clone();
                set(&mut changed, key, invalid(child));
                out.push((key.clone(), changed));
                for (path, mutant) in field_mutants(child) {
                    let mut changed = value.clone();
                    set(&mut changed, key, mutant);
                    out.push((format!("{key}.{path}"), changed));
                }
            }
        }
        Value::Arr(array) => {
            for (i, child) in array.iter().enumerate() {
                let mut changed = array.clone();
                changed[i] = invalid(child);
                out.push((i.to_string(), Value::Arr(changed)));
                for (path, mutant) in field_mutants(child) {
                    let mut changed = array.clone();
                    changed[i] = mutant;
                    out.push((format!("{i}.{path}"), Value::Arr(changed)));
                }
            }
        }
        _ => {}
    }
    out
}

#[test]
fn scheme_target_every_field_and_position_type_mutation_rejects_with_target_code() {
    let base = fixture(true, |_, _| {});
    let target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let mutants = field_mutants(&target);
    assert_eq!(mutants.len(), 67, "scheme target field inventory changed");
    let mut failures = Vec::new();
    for (path, mutated) in &mutants {
        let mut inputs = base.clone();
        bind_schedule(&mut inputs, schedule.clone(), mutated.clone());
        let verdict = check_certificate(&inputs);
        if verdict.overall != "rejected" || !codes(&verdict).contains(&"E_CERT_TARGET") {
            failures.push(format!("{path}: {} {:?}", verdict.overall, codes(&verdict)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    eprintln!(
        "scheme target sweep: {}/{} fields/positions rejected with E_CERT_TARGET",
        mutants.len(),
        mutants.len()
    );
}

#[test]
fn scheme_ancilla_count_domain_errors_reject_but_unexpanded_lifetimes_stay_unchecked() {
    for (count, expected) in [("-1", "rejected"), ("0.5", "rejected"), ("1", "unchecked")] {
        let inputs = fixture(true, |target, _| {
            let Value::Obj(t) = target else {
                unreachable!()
            };
            let Value::Arr(schemes) = t.get_mut("schemes").unwrap() else {
                unreachable!()
            };
            let Value::Obj(scheme) = &mut schemes[0] else {
                unreachable!()
            };
            set(scheme.get_mut("ancillae").unwrap(), "count", json(count));
        });
        let verdict = check_certificate(&inputs);
        assert_eq!(verdict.overall, expected, "count={count}: {verdict:?}");
        if expected == "rejected" {
            assert!(codes(&verdict).contains(&"E_CERT_TARGET"));
        } else {
            assert!(codes(&verdict).is_empty());
        }
    }
}
