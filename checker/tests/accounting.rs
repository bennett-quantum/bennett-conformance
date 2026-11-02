mod common;
use bennett_cert_checker::cert::{check_certificate, Inputs};
use bennett_cert_checker::json::{jcs, n, Value};
use common::*;
fn change_root(inp: &mut Inputs, f: impl FnOnce(&mut Value)) {
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let hash = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    f(regions.get_mut(&hash).unwrap());
    inp.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn published_positive_certificate_checks_resources_and_composition() {
    let verdict = check_certificate(&controlled());
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
    assert!(verdict.unchecked_items.is_empty());
    assert!(verdict.regions.iter().all(|r| r.unchecked_items.is_empty()));
}
#[test]
fn every_published_resource_and_gate_count_value_is_checked() {
    let mut mutations = 0;
    for field in ["time", "peak_qubits", "qubit_second"] {
        for delta in [-1, 1] {
            let mut inp = controlled();
            change_root(&mut inp, |r| {
                let mut resources = r.get("resources").unwrap().clone();
                let old = resources.get(field).unwrap().as_i128().unwrap();
                set(&mut resources, field, n(old + delta));
                set(r, "resources", resources);
            });
            let v = check_certificate(&inp);
            assert!(
                codes(&v).contains(&"E_CERT_RESOURCE"),
                "{field}/{delta}: {v:?}"
            );
            mutations += 1;
        }
    }
    for count in [0, 2] {
        let mut inp = controlled();
        change_root(&mut inp, |r| {
            set(
                r,
                "gates_used",
                json(&format!(r#"{{"x":{{"count":{count}}}}}"#)),
            )
        });
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_GATE_COUNT"), "{v:?}");
        mutations += 1;
    }
    assert_eq!(mutations, 8);
}
#[test]
fn every_published_error_and_multiplicity_is_checked() {
    for part in ["local", "inclusive"] {
        let mut inp = controlled();
        change_root(&mut inp, |r| {
            let mut errors = r.get("error").unwrap().clone();
            let mut entry = errors.get("forward:0").unwrap().clone();
            set(&mut entry, part, json(r#"{"operator_norm":1}"#));
            set(&mut errors, "forward:0", entry);
            set(r, "error", errors);
        });
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_COMPOSITION"), "{part}: {v:?}");
    }
    let base = controlled();
    let cert = json(std::str::from_utf8(&base.cert_bytes).unwrap());
    let keys: Vec<String> = cert
        .get("composition")
        .unwrap()
        .get("multiplicities")
        .unwrap()
        .as_obj()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys.len(), 2);
    for key in keys {
        let mut inp = base.clone();
        let mut cert = cert.clone();
        let Value::Obj(top) = &mut cert else {
            unreachable!()
        };
        let comp = top.get_mut("composition").unwrap();
        let mut counts = comp.get("multiplicities").unwrap().clone();
        set(&mut counts, &key, n(2));
        set(comp, "multiplicities", counts);
        inp.cert_bytes = jcs(&cert).unwrap();
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_MULTIPLICITY"), "{v:?}");
    }
}
#[test]
fn total_error_is_recomputed_instead_of_trusting_the_declared_zero() {
    let mut inp = controlled();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    set(
        top.get_mut("composition").unwrap(),
        "epsilon_total",
        json(r#"{"diamond":1}"#),
    );
    inp.cert_bytes = jcs(&cert).unwrap();
    let v = check_certificate(&inp);
    assert!(codes(&v).contains(&"E_CERT_COMPOSITION"), "{v:?}");
}
#[test]
fn capacity_and_negative_start_reject_with_the_schedule_code() {
    for bad_capacity in [true, false] {
        let mut inp = controlled();
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        if bad_capacity {
            let Value::Obj(t) = &mut target else {
                unreachable!()
            };
            let classes = t.get_mut("classes").unwrap();
            let mut fast = classes.get("fast").unwrap().clone();
            set(&mut fast, "capacity", n(1));
            set(classes, "fast", fast);
        } else {
            let Value::Obj(s) = &mut schedule else {
                unreachable!()
            };
            let Value::Arr(entries) = s.get_mut("entries").unwrap() else {
                unreachable!()
            };
            set(&mut entries[1], "start", json(r#"{"time":-1}"#));
        }
        bind_schedule(&mut inp, schedule, target);
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"), "{v:?}");
    }
}

#[test]
fn native_cost_table_lookup_checks_own_controls_and_array_bounds() {
    for values in ["[1,999]", "[]"] {
        let mut inp = controlled();
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        set(
            &mut target,
            "tables",
            json(&format!(r#"{{"ticks":{values}}}"#)),
        );
        let Value::Obj(t) = &mut target else {
            unreachable!()
        };
        let Value::Arr(gates) = t.get_mut("gates").unwrap() else {
            unreachable!()
        };
        set(
            &mut gates[0],
            "cost",
            json(r#"{"time":{"table":"ticks","index":"k"}}"#),
        );
        let schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        bind_schedule(&mut inp, schedule, target);
        let verdict = check_certificate(&inp);
        if values == "[]" {
            assert!(codes(&verdict).contains(&"E_CERT_TARGET"), "{verdict:?}");
        } else {
            assert_eq!(verdict.overall, "verified", "{verdict:?}");
        }
    }
}

fn estimate_fixture() -> Inputs {
    let mut inp = controlled();
    let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(t) = &mut target else {
        unreachable!()
    };
    let Value::Arr(gates) = t.get_mut("gates").unwrap() else {
        unreachable!()
    };
    set(
        &mut gates[0],
        "cost",
        json(r#"{"time":{"args":[1,3],"op":"/"}}"#),
    );
    let schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inp, schedule, target);
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    for region in regions.values_mut() {
        set(
            region,
            "estimate",
            json(
                r#"{"exact_resources":{"peak_qubits[fast]":{"denominator":"1","numerator":"2"},"qubit_second[fast]":{"denominator":"3","numerator":"2"},"time":{"denominator":"3","numerator":"1"}},"precision_digits":34}"#,
            ),
        );
        set(
            region,
            "resources",
            json(
                r#"{"peak_qubits[fast]":"2","qubit_second[fast]":"0.6666666666666666666666666666666667","time":"0.3333333333333333333333333333333334"}"#,
            ),
        );
    }
    inp.cert_bytes = jcs(&cert).unwrap();
    inp
}
#[test]
fn estimates_check_exact_rationals_and_upward_34_digit_display() {
    let verdict = check_certificate(&estimate_fixture());
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}
#[test]
fn estimate_rejects_unreduced_wrong_fraction_or_display() {
    for mutation in [
        "unreduced",
        "wrong-fraction",
        "rounded-down",
        "missing-row",
        "wrong-precision",
    ] {
        let mut inp = estimate_fixture();
        change_root(&mut inp, |region| {
            if mutation == "rounded-down" {
                let mut r = region.get("resources").unwrap().clone();
                set(
                    &mut r,
                    "time",
                    json(r#""0.3333333333333333333333333333333333""#),
                );
                set(region, "resources", r);
            } else {
                let mut e = region.get("estimate").unwrap().clone();
                if mutation == "wrong-precision" {
                    set(&mut e, "precision_digits", n(33));
                } else {
                    let mut exact = e.get("exact_resources").unwrap().clone();
                    match mutation {
                        "unreduced" => set(
                            &mut exact,
                            "time",
                            json(r#"{"denominator":"6","numerator":"2"}"#),
                        ),
                        "wrong-fraction" => set(
                            &mut exact,
                            "time",
                            json(r#"{"denominator":"4","numerator":"1"}"#),
                        ),
                        "missing-row" => {
                            let Value::Obj(m) = &mut exact else {
                                unreachable!()
                            };
                            m.remove("time");
                        }
                        _ => unreachable!(),
                    }
                    set(&mut e, "exact_resources", exact);
                }
                set(region, "estimate", e);
            }
        });
        let verdict = check_certificate(&inp);
        assert!(
            codes(&verdict).contains(&"E_CERT_ESTIMATE"),
            "{mutation}: {verdict:?}"
        );
    }
}

#[test]
fn predicate_depth_limit_is_checked_against_the_effective_control_context() {
    let mut inp = controlled();
    let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(t) = &mut target else {
        unreachable!()
    };
    set(
        t.get_mut("capabilities").unwrap(),
        "max_predicate_depth",
        n(0),
    );
    let schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inp, schedule, target);
    let verdict = check_certificate(&inp);
    assert!(
        codes(&verdict).contains(&"E_SCHEDULE_INFEASIBLE"),
        "{verdict:?}"
    );
}

#[test]
fn gate_cost_distinguishes_own_controls_from_inherited_controls() {
    let claim =
        json(r#"{"bound":0,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#);
    let (mut inp, h) = standalone(
        "perm_action @a (t:wire mut) (c:wire in, p:wire in) () () ()\n^entry():\nctl p mcx (c) t",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1,1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a",
        vec![claim],
    );
    let base = controlled();
    let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(t) = &mut target else {
        unreachable!()
    };
    set(
        t.get_mut("classes").unwrap(),
        "fast",
        json(r#"{"capacity":3,"dirty_rate":1,"qubit_second":1}"#),
    );
    set(
        &mut target,
        "gates",
        json(
            r#"[{"classes":["fast"],"controls":{"max":2,"min":2},"cost":{"time":{"args":[{"var":"k"},{"var":"c"}],"op":"+"}},"epsilon":0,"op":"mcx"}]"#,
        ),
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let region = regions.get_mut(&h).unwrap();
    set(
        region,
        "resources",
        json(r#"{"peak_qubits":3,"qubit_second":9,"time":3}"#),
    );
    set(region, "gates_used", json(r#"{"mcx":{"count":1}}"#));
    set(
        region,
        "error",
        json(r#"{"forward:0":{"inclusive":{"operator_norm":0},"local":{"operator_norm":0}}}"#),
    );
    let mut counts = json("{}");
    set(&mut counts, &format!("{h}:forward:0"), n(1));
    set(
        top.get_mut("composition").unwrap(),
        "multiplicities",
        counts,
    );
    inp.cert_bytes = jcs(&cert).unwrap();
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", Value::Arr(vec![]));
    bind_schedule(&mut inp, schedule, target);
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}

#[test]
fn symbolic_source_slot_widths_are_evaluated_at_each_resolved_control_count() {
    let mut inp = controlled();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    for region in regions.values_mut() {
        let child = region.get("claims").unwrap().as_arr().unwrap().len() == 1;
        let Value::Obj(r) = region else {
            unreachable!()
        };
        let Value::Obj(frame) = r.get_mut("frame").unwrap() else {
            unreachable!()
        };
        let Value::Arr(slots) = frame.get_mut("slots").unwrap() else {
            unreachable!()
        };
        for slot in slots {
            set(
                slot,
                "width",
                json(if child {
                    r#"{"var":"c"}"#
                } else {
                    r#"{"args":[1,{"var":"c"}],"op":"+"}"#
                }),
            );
        }
    }
    inp.cert_bytes = jcs(&cert).unwrap();
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}
