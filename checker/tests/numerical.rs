mod common;
use bennett_cert_checker::cert::check_certificate;
use common::*;
#[test]
fn controlled_fixture_numerical_claims_check_every_default_context() {
    let v = check_certificate(&controlled());
    assert!(codes(&v).is_empty(), "{v:?}");
    let mut checked = 0;
    for region in &v.regions {
        for (kind, claim) in &region.claims {
            if kind == "numerical" {
                assert_eq!(claim.outcome, "accept", "{region:?}");
                assert_eq!(claim.computed.as_deref(), Some("0"));
                assert_eq!(claim.contexts, Some(1));
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 2);
}

use bennett_cert_checker::json::{jcs, n, Value};
fn add_channel(inp: &mut bennett_cert_checker::cert::Inputs, zero_pivot: bool, missing: bool) {
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let root = regions
        .values_mut()
        .find(|r| r.get("claims").unwrap().as_arr().unwrap().len() == 2)
        .unwrap();
    let mut channel = root.get("claims").unwrap().as_arr().unwrap()[1].clone();
    set(&mut channel, "kind", Value::Str("channel".into()));
    set(&mut channel, "metric", Value::Str("diamond".into()));
    set(&mut channel, "bound", json("0.625"));
    let entries = (0..16)
        .flat_map(|i| {
            (0..16).map(move |j| {
                json(if i == j {
                    r#"["0.125","0.125","0","0"]"#
                } else {
                    r#"["0","0","0","0"]"#
                })
            })
        })
        .collect();
    let mut dual = json(r#"{"mu":0.625,"Y":[]}"#);
    set(&mut dual, "Y", Value::Arr(entries));
    if zero_pivot {
        set(&mut dual, "mu", json("0.5"));
    }
    set(
        &mut channel,
        "dual",
        Value::Arr(if missing { vec![] } else { vec![dual] }),
    );
    let Value::Obj(root) = root else {
        unreachable!()
    };
    let Value::Arr(claims) = root.get_mut("claims").unwrap() else {
        unreachable!()
    };
    claims.push(channel);
    let ver = root.get_mut("verification").unwrap();
    set(ver, "channel", Value::Arr(vec![n(2)]));
    inp.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn channel_strict_dual_witness_is_checked_at_the_resolved_context() {
    let mut inp = controlled();
    add_channel(&mut inp, false, false);
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    let root = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("controlled"))
        .unwrap();
    assert_eq!(root.claims[2].1.outcome, "accept", "{root:?}");
    assert_eq!(root.claims[2].1.contexts, Some(1));
}
#[test]
fn channel_rejects_zero_pivot_and_missing_context_witness() {
    for (zero, missing) in [(true, false), (false, true)] {
        let mut inp = controlled();
        add_channel(&mut inp, zero, missing);
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_CHANNEL"), "{v:?}");
    }
}

#[test]
fn scheduled_decisions_are_not_inferred_from_unrelated_exact_target_entries() {
    for variant in [
        "unsupported-controls",
        "unimplemented-scheme",
        "tied-decisions",
    ] {
        let mut inp = controlled();
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        if variant == "unsupported-controls" {
            let Value::Obj(t) = &mut target else {
                unreachable!()
            };
            let Value::Arr(gates) = t.get_mut("gates").unwrap() else {
                unreachable!()
            };
            set(&mut gates[0], "controls", json(r#"{"max":2,"min":2}"#));
        } else {
            let Value::Obj(s) = &mut schedule else {
                unreachable!()
            };
            let Value::Arr(entries) = s.get_mut("entries").unwrap() else {
                unreachable!()
            };
            if variant == "unimplemented-scheme" {
                set(
                    &mut entries[0],
                    "gate",
                    json(r#"{"epsilon_share":0,"realisation":"fake-exact-scheme"}"#),
                );
            } else {
                entries.push(entries[0].clone());
            }
        }
        bind_schedule(&mut inp, schedule, target);
        let v = check_certificate(&inp);
        for r in &v.regions {
            for (_, c) in &r.claims {
                assert!(
                    !matches!(c.outcome.as_str(), "proof" | "accept"),
                    "{variant}: {v:?}"
                );
            }
        }
        if variant == "tied-decisions" {
            assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"), "{v:?}");
        }
    }
}
#[test]
fn default_only_schedule_still_enumerates_two_distinct_call_contexts() {
    let funcs="perm_action @controlled (t:wire mut) (c:wire in) () () ()\n^entry():\nctl c action_call @flip (t) () () ()\nctl c action_call @flip (t) () () ()\nperm_action @flip (t:wire mut) () () () ()\n^entry():\nx t";
    let sigs = r#"{"controlled":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"flip":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let numerical =
        json(r#"{"bound":0,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#);
    let (mut inp, _) = standalone(funcs, sigs, "controlled", vec![numerical.clone()]);
    let (child, ch) = standalone(funcs, sigs, "flip", vec![numerical]);
    let mut child_cert = json(std::str::from_utf8(&child.cert_bytes).unwrap());
    let Value::Obj(ctop) = &mut child_cert else {
        unreachable!()
    };
    let Value::Obj(cr) = ctop.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let mut child_region = cr.remove(&ch).unwrap();
    let Value::Obj(r) = &mut child_region else {
        unreachable!()
    };
    let Value::Arr(cs) = r.get_mut("claims").unwrap() else {
        unreachable!()
    };
    let subject = cs[0].get("subject").unwrap().clone();
    let mut subject = subject;
    set(&mut subject, "inherited_controls", n(1));
    set(&mut cs[0], "subject", subject);
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    regions.insert(ch, child_region);
    inp.cert_bytes = jcs(&cert).unwrap();
    let base = controlled();
    let target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", Value::Arr(vec![]));
    bind_schedule(&mut inp, schedule, target);
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    let child = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("flip"))
        .unwrap();
    assert_eq!(child.claims[0].1.outcome, "accept", "{v:?}");
    assert_eq!(child.claims[0].1.contexts, Some(2));
}

#[test]
fn adjoint_calls_match_the_inverse_primitive_in_the_target() {
    let source="action @root (t:wire mut) () () () ()\n^entry():\nadj action_call @phase (t) () () ()\naction @phase (t:wire mut) () () () ()\n^entry():\nt t";
    let sigs = r#"{"root":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"},"phase":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#;
    for op in ["tdg", "t"] {
        let (mut inp, _) = standalone(
            source,
            sigs,
            "root",
            vec![json(
                r#"{"bound":0.00000001,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#,
            )],
        );
        let base = controlled();
        let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
        set(
            &mut target,
            "gates",
            json(&format!(
                r#"[{{"op":"{op}","classes":["fast"],"controls":{{"min":0,"max":0}},"epsilon":0,"cost":{{"time":1}}}}]"#
            )),
        );
        let mut schedule =
            json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
        set(&mut schedule, "entries", Value::Arr(vec![]));
        bind_schedule(&mut inp, schedule, target);
        let v = check_certificate(&inp);
        let outcome = &v.regions[0].claims[0].1;
        if op=="tdg" {assert!(codes(&v).is_empty(),"{v:?}");}
        else {assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"),"{v:?}");}
        assert_eq!(
            outcome.outcome,
            if op == "tdg" { "accept" } else { "reject" },
            "{op}: {v:?}"
        );
    }
}

fn rotation_inputs(op: &str, angle: &str) -> bennett_cert_checker::cert::Inputs {
    let arg_group = if angle == "theta" {
        "(theta:f64)"
    } else {
        "()"
    };
    let source =
        format!("action @rotation (t:wire mut) () () {arg_group} ()\n^entry():\n{op} {angle} t");
    let sigs = if angle == "theta" {
        r#"{"rotation":{"abi":"0.9","args":["f64"],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#
    } else {
        r#"{"rotation":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#
    };
    let (mut inp, h) = standalone(
        &source,
        sigs,
        "rotation",
        vec![json(
            r#"{"bound":0.000000000000001,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#,
        )],
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    if angle == "theta" {
        set(
            top.get_mut("bindings").unwrap(),
            "invocation",
            json(r#"{"args":["0x3ff0000000000000"],"budget":["0","0","0","0"]}"#),
        );
    }
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let region = regions.get_mut(&h).unwrap();
    set(
        region,
        "resources",
        json(r#"{"peak_qubits":1,"qubit_second":1,"time":1}"#),
    );
    set(
        region,
        "gates_used",
        json(&format!(r#"{{"{op}":{{"count":1}}}}"#)),
    );
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
    let base = controlled();
    let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    set(
        &mut target,
        "gates",
        json(&format!(
            r#"[{{"op":"{op}","params":["theta"],"classes":["fast"],"controls":{{"min":0,"max":0}},"epsilon":0,"cost":{{"time":1}}}}]"#
        )),
    );
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", Value::Arr(vec![]));
    bind_schedule(&mut inp, schedule, target);
    inp
}
#[test]
fn exact_native_rotations_verify_the_requested_rigorous_bound() {
    for op in ["rx", "ry", "rz"] {
        for angle in ["-0.0", "0.0", "1.0", "-1.0", "10.0"] {
            let verdict = check_certificate(&rotation_inputs(op, angle));
            assert_eq!(verdict.overall, "verified", "{op} {angle}: {verdict:?}");
            let claim = &verdict.regions[0].claims[0].1;
            assert_eq!(claim.outcome, "accept");
            assert_eq!(claim.contexts, Some(1));
            assert!(claim.computed.is_some());
        }
    }
}
#[test]
fn malformed_rotation_literals_are_rejected_and_huge_angles_report_host_limits() {
    for angle in ["1", "1.00", "NaN", "inf", "1e999"] {
        let verdict = check_certificate(&rotation_inputs("rx", angle));
        assert!(
            codes(&verdict).contains(&"E_CERT_OBJECT"),
            "{angle}: {verdict:?}"
        );
    }
    let verdict = check_certificate(&rotation_inputs("rx", "1e+100"));
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(verdict.regions[0].claims[0]
        .1
        .unchecked_reason
        .as_ref()
        .unwrap()
        .contains("host limit"));
}

#[test]
fn rotation_classical_arguments_preserve_bits_and_reject_nonfinite_angles() {
    for bits in [
        "0x0000000000000000",
        "0x8000000000000000",
        "0x3ff0000000000000",
        "0xbff0000000000000",
        "0x7ff8000000000001",
        "0x7ff0000000000000",
    ] {
        let mut inp = rotation_inputs("rz", "theta");
        let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
        let Value::Obj(m) = &mut cert else {
            unreachable!()
        };
        set(
            m.get_mut("bindings").unwrap(),
            "invocation",
            json(&format!(
                r#"{{"args":["{bits}"],"budget":["0","0","0","0"]}}"#
            )),
        );
        inp.cert_bytes = jcs(&cert).unwrap();
        let schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        let target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        bind_schedule(&mut inp, schedule, target);
        let verdict = check_certificate(&inp);
        if bits.starts_with("0x7ff") {
            assert!(
                codes(&verdict).contains(&"E_CERT_OBJECT"),
                "{bits}: {verdict:?}"
            );
        } else {
            assert_eq!(verdict.overall, "verified", "{bits}: {verdict:?}");
        }
    }
}

#[test]
fn native_epsilon_expressions_are_evaluated_before_ordered_matching() {
    for (expression, share, expected) in [
        (
            r#"{"op":"-","args":[{"var":"k"},{"var":"k"}]}"#,
            "0",
            "verified",
        ),
        ("-1", "0", "rejected"),
        ("0.1", "0", "verified"),
        (r#"{"op":"*","args":[0,{"var":"eps"}]}"#, "0.1", "rejected"),
    ] {
        let mut inp = rotation_inputs("rx", "1.0");
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let first = json(&format!(
            r#"{{"op":"rx","params":["theta"],"classes":["fast"],"controls":{{"min":0,"max":0}},"epsilon":{expression},"cost":{{"time":1}}}}"#
        ));
        let second = target.get("gates").unwrap().as_arr().unwrap()[0].clone();
        set(&mut target, "gates", Value::Arr(vec![first, second]));
        let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        set(
            &mut schedule,
            "entries",
            json(&format!(
                r#"[{{"path":[],"step":"*","phase":"*","generated":"*","kind":"gate","gate":{{"realisation":"native","epsilon_share":{share}}}}}]"#
            )),
        );
        bind_schedule(&mut inp, schedule, target);
        let verdict = check_certificate(&inp);
        assert_eq!(verdict.overall, expected, "{expression}: {verdict:?}");
        if expected == "rejected" {
            assert!(codes(&verdict).contains(&"E_CERT_TARGET"), "{verdict:?}");
        }
    }
}

#[test]
fn missing_native_and_named_scheme_matches_are_infeasible(){
    for case in ["missing-name","wrong-range","no-realisation","unrelated-default","matching-default","native-with-only-scheme","matching-name"] {
        let mut inp=controlled();let mut target=json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());let mut schedule=json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        if case!="missing-name" && case!="no-realisation" {
            let op=if case=="unrelated-default"{"h"}else{"x"};let max=if case=="wrong-range"{0}else{1};
            set(&mut target,"schemes",json(&format!(r#"[{{"name":"controlled-x","realises":{{"op":"{op}","controls":{{"min":0,"max":{max}}}}},"ancillae":{{"count":0,"state":"clean","class":"fast"}},"template":[{{"gate":"cx","wires":["c[0]","t"]}}]}}]"#)));
        }
        if matches!(case,"no-realisation"|"unrelated-default") {set(&mut target,"gates",Value::Arr(vec![]));}
        if matches!(case,"matching-default"|"native-with-only-scheme"|"matching-name") {set(&mut target,"gates",json(r#"[{"op":"cx","classes":["fast"],"controls":{"min":1,"max":1},"epsilon":0,"cost":{"time":1}}]"#));}
        if matches!(case,"unrelated-default"|"matching-default") {set(&mut schedule,"entries",Value::Arr(vec![]));}
        else if matches!(case,"missing-name"|"wrong-range"|"matching-name") {
            let Value::Obj(top)=&mut schedule else {unreachable!()};let Value::Arr(entries)=top.get_mut("entries").unwrap() else {unreachable!()};
            let Value::Obj(e)=&mut entries[0] else {unreachable!()};set(e.get_mut("gate").unwrap(),"realisation",Value::Str(if case=="missing-name"{"missing"}else{"controlled-x"}.into()));
        }
        bind_schedule(&mut inp,schedule,target);let v=check_certificate(&inp);
        if matches!(case,"matching-default"|"matching-name") {assert_eq!(v.overall,"unchecked","{case}: {v:?}");assert!(codes(&v).is_empty(),"{case}: {v:?}");}
        else {assert_eq!(v.overall,"rejected","{case}: {v:?}");assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"),"{case}: {v:?}");}
    }
}

#[test]
fn native_control_range_metadata_integers_are_compared_without_host_truncation(){
    for (min,max,expected) in [("1","1","verified"),("0","18446744073709551616","verified"),("18446744073709551616","18446744073709551616","rejected")] {
        let mut inp=controlled();let mut target=json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let Value::Obj(top)=&mut target else {unreachable!()};let Value::Arr(gates)=top.get_mut("gates").unwrap() else {unreachable!()};set(&mut gates[0],"controls",json(&format!(r#"{{"min":"{min}","max":"{max}"}}"#)));
        let schedule=json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());bind_schedule(&mut inp,schedule,target);let v=check_certificate(&inp);
        assert_eq!(v.overall,expected,"{min}..{max}: {v:?}");if expected=="rejected" {assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"),"{v:?}");}
    }
}
