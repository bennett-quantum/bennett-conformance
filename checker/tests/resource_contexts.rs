use crate::common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::{jcs, Value},
};
use common::*;
fn repaired() -> Inputs {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/controlled-action-k87");
    let mut inputs = controlled();
    inputs.cert_bytes = std::fs::read(d.join("certificate.json")).unwrap();
    inputs.object_text = std::fs::read_to_string(d.join("controlled.bisa")).unwrap();
    inputs
}
fn keyed() -> Inputs {
    let mut inputs = repaired();
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    for (hash, r) in regions {
        let key = if hash == &root {
            json("[]")
        } else {
            json(&format!(
                r#"[{{"symbol":"{root}","point":"entry:0","iteration":[],"instance":[],"call_index":0}}]"#
            ))
        };
        let mut entry = json("{}");
        set(&mut entry, "context_key", key);
        set(&mut entry, "resources", r.get("resources").unwrap().clone());
        set(
            &mut entry,
            "gates_used",
            r.get("gates_used").unwrap().clone(),
        );
        set(r, "resource_contexts", Value::Arr(vec![entry]));
    }
    inputs.cert_bytes = jcs(&cert).unwrap();
    inputs
}
#[test]
fn k87_singleton_contexts_and_canonical_vector_displays_verify() {
    for inputs in [repaired(), keyed()] {
        let verdict = check_certificate(&inputs);
        assert_eq!(verdict.overall, "verified", "{verdict:?}");
    }
}
#[test]
fn pre_k87_numeric_and_unqualified_resource_reports_reject() {
    let mut original = controlled();
    let upstream =
        pins::isa_spec_dir().join("conformance/fixtures/controlled-action-09/certificate.json");
    original.cert_bytes = std::fs::read(upstream).unwrap();
    let verdict = check_certificate(&original);
    assert!(codes(&verdict).contains(&"E_CERT_SCHEMA"), "{verdict:?}");
}

fn repeated(thirds: bool) -> Inputs {
    let source="perm_action @root (t:wire mut) (c:wire in) () () ()\n^entry():\naction_call @flip (t) () () ()\nqpred c\naction_call @flip (t) () () ()\nqunpred c\nperm_action @flip (t:wire mut) () () () ()\n^entry():\nx t";
    let signatures = r#"{"root":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"flip":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let claim =
        json(r#"{"bound":0,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#);
    let (mut inputs, h) = standalone(source, signatures, "root", vec![claim]);
    let template = controlled();
    let template_cert = json(std::str::from_utf8(&template.cert_bytes).unwrap());
    let child = template_cert
        .get("regions")
        .unwrap()
        .as_obj()
        .unwrap()
        .iter()
        .find(|(_, r)| {
            r.get("name").is_none()
                && r.get("signature")
                    .unwrap()
                    .get("in")
                    .unwrap()
                    .as_arr()
                    .unwrap()
                    .is_empty()
        })
        .unwrap();
    let child_hash = child.0.clone();
    let mut child_record = child.1.clone();
    let mut claims = vec![];
    for c in 0..2 {
        let mut claim = child_record.get("claims").unwrap().as_arr().unwrap()[0].clone();
        let mut subject = claim.get("subject").unwrap().clone();
        set(&mut subject, "inherited_controls", json(&c.to_string()));
        set(&mut claim, "subject", subject);
        claims.push(claim);
    }
    set(&mut child_record, "claims", Value::Arr(claims));
    let mut verification = child_record.get("verification").unwrap().clone();
    set(&mut verification, "unitary", json("[0,1]"));
    set(&mut child_record, "verification", verification);
    set(
        &mut child_record,
        "error",
        json(
            r#"{"forward:0":{"local":{"operator_norm":0},"inclusive":{"operator_norm":0}},"forward:1":{"local":{"operator_norm":0},"inclusive":{"operator_norm":0}}}"#,
        ),
    );
    let frame = child_record.get("frame").unwrap().clone();
    let mut frames = vec![];
    let mut reports = vec![];
    for (point, controlled) in [(0, false), (2, true)] {
        let key = json(&format!(
            r#"[{{"symbol":"{h}","point":"entry:{point}","iteration":[],"instance":[],"call_index":0}}]"#
        ));
        let time = if thirds {
            "0.3333333333333333333333333333333334"
        } else {
            "1"
        };
        let occupancy = if thirds {
            if controlled {
                "0.6666666666666666666666666666666667"
            } else {
                time
            }
        } else {
            if controlled {
                "2"
            } else {
                "1"
            }
        };
        let peak = if controlled { "2" } else { "1" };
        let resources = json(&format!(
            r#"{{"time":"{time}","peak_qubits[fast]":"{peak}","qubit_second[fast]":"{occupancy}"}}"#
        ));
        let mut report = json("{}");
        set(&mut report, "context_key", key.clone());
        set(&mut report, "resources", resources);
        set(&mut report, "gates_used", json(r#"{"x":{"count":1}}"#));
        reports.push(report);
        let mut descriptor = json("{}");
        set(&mut descriptor, "context_key", key);
        set(&mut descriptor, "frame", frame.clone());
        frames.push(descriptor);
    }
    set(
        &mut child_record,
        "resources",
        reports[0].get("resources").unwrap().clone(),
    );
    set(&mut child_record, "resource_contexts", Value::Arr(reports));
    let Value::Obj(fields) = &mut child_record else {
        panic!()
    };
    fields.remove("frame");
    fields.insert("frame_contexts".into(), Value::Arr(frames));
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    regions.insert(child_hash.clone(), child_record);
    let root = regions.get_mut(&h).unwrap();
    let (time, occupancy) = if thirds {
        (
            "0.6666666666666666666666666666666667",
            "1.333333333333333333333333333333334",
        )
    } else {
        ("2", "4")
    };
    set(
        root,
        "resources",
        json(&format!(
            r#"{{"time":"{time}","peak_qubits[fast]":"2","qubit_second[fast]":"{occupancy}"}}"#
        )),
    );
    set(root, "gates_used", json(r#"{"x":{"count":2}}"#));
    set(
        root,
        "error",
        json(r#"{"forward:0":{"local":{"operator_norm":0},"inclusive":{"operator_norm":0}}}"#),
    );
    set(
        top.get_mut("composition").unwrap(),
        "multiplicities",
        json(&format!(
            r#"{{"{h}:forward:0":1,"{child_hash}:forward:0":1,"{child_hash}:forward:1":1}}"#
        )),
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
    let mut target = json(std::str::from_utf8(template.target_bytes.as_ref().unwrap()).unwrap());
    let mut gate = target.get("gates").unwrap().as_arr().unwrap()[0].clone();
    set(&mut gate, "controls", json(r#"{"min":0,"max":1}"#));
    if thirds {
        set(
            &mut gate,
            "cost",
            json(r#"{"time":{"op":"/","args":[1,3]}}"#),
        );
    }
    set(&mut target, "gates", Value::Arr(vec![gate]));
    let mut schedule =
        json(std::str::from_utf8(template.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", json("[]"));
    bind_schedule(&mut inputs, schedule, target);
    inputs
}
fn mutate_child(inputs: &mut Inputs, f: impl FnOnce(&mut Value)) {
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    f(regions
        .iter_mut()
        .find(|(hash, _)| hash.as_str() != root)
        .unwrap()
        .1);
    inputs.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn repeated_default_covered_contexts_use_exact_local_timing_and_unrounded_root_values() {
    for thirds in [false, true] {
        let verdict = check_certificate(&repeated(thirds));
        assert_eq!(verdict.overall, "verified", "thirds={thirds}: {verdict:?}");
    }
}
#[test]
fn context_coverage_projection_frames_and_every_context_cost_reject_mismatches() {
    for case in 0..11 {
        let mut inputs = repeated(false);
        mutate_child(&mut inputs, |r| {
            let mut reports = r
                .get("resource_contexts")
                .unwrap()
                .as_arr()
                .unwrap()
                .to_vec();
            match case {
                0 => {
                    let Value::Obj(m) = r else { panic!() };
                    m.remove("resource_contexts");
                    return;
                }
                1 => {
                    reports.pop();
                }
                2 => {
                    reports.reverse();
                }
                3 => {
                    reports[1] = reports[0].clone();
                }
                4 => {
                    let mut key = reports[1].get("context_key").unwrap().clone();
                    let Value::Arr(edges) = &mut key else {
                        panic!()
                    };
                    set(&mut edges[0], "point", Value::Str("entry:9".into()));
                    set(&mut reports[1], "context_key", key);
                }
                5 => set(
                    &mut reports[1],
                    "resources",
                    json(r#"{"time":"2","peak_qubits[fast]":"2","qubit_second[fast]":"2"}"#),
                ),
                6 => set(&mut reports[1], "gates_used", json(r#"{"x":{"count":2}}"#)),
                7 => {
                    set(r, "resources", reports[1].get("resources").unwrap().clone());
                    return;
                }
                8 => {
                    set(
                        r,
                        "estimate",
                        json(r#"{"precision_digits":34,"exact_resources":{}}"#),
                    );
                    return;
                }
                9 => {
                    let frames = r.get("frame_contexts").unwrap().as_arr().unwrap();
                    set(r, "frame_contexts", Value::Arr(vec![frames[0].clone()]));
                    return;
                }
                _ => {
                    let mut frames = r.get("frame_contexts").unwrap().as_arr().unwrap().to_vec();
                    frames.reverse();
                    set(r, "frame_contexts", Value::Arr(frames));
                    return;
                }
            }
            set(r, "resource_contexts", Value::Arr(reports));
        });
        let verdict = check_certificate(&inputs);
        let expected = match case {
            2 | 3 => "E_CERT_SCHEMA",
            6 => "E_CERT_GATE_COUNT",
            9 | 10 => "E_CERT_FRAME",
            _ => "E_CERT_RESOURCE",
        };
        assert!(
            codes(&verdict).contains(&expected),
            "case={case}: expected {expected}: {verdict:?}"
        );
    }
}
#[test]
fn canonical_resource_strings_cannot_use_numbers_exponents_signs_or_redundant_zeroes() {
    for text in ["1e0", "1.0", "01", "+1", "-0", "0.00", ""] {
        let mut inputs = keyed();
        mutate_child(&mut inputs, |r| {
            let mut res = r.get("resources").unwrap().clone();
            set(&mut res, "time", Value::Str(text.into()));
            set(r, "resources", res);
        });
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_SCHEMA"),
            "{text}: {verdict:?}"
        );
    }
}

#[derive(Clone, Debug)]
enum Part {
    Key(String),
    Index(usize),
}
fn positions(v: &Value, p: &mut Vec<Part>, out: &mut Vec<Vec<Part>>) {
    match v {
        Value::Obj(m) => {
            for (k, v) in m {
                p.push(Part::Key(k.clone()));
                out.push(p.clone());
                positions(v, p, out);
                p.pop();
            }
        }
        Value::Arr(a) => {
            for (i, v) in a.iter().enumerate() {
                p.push(Part::Index(i));
                out.push(p.clone());
                positions(v, p, out);
                p.pop();
            }
        }
        _ => (),
    }
}
fn at<'a>(v: &'a mut Value, p: &[Part]) -> &'a mut Value {
    if p.is_empty() {
        return v;
    }
    match (&p[0], v) {
        (Part::Key(k), Value::Obj(m)) => at(m.get_mut(k).unwrap(), &p[1..]),
        (Part::Index(i), Value::Arr(a)) => at(&mut a[*i], &p[1..]),
        _ => panic!(),
    }
}
#[test]
fn resource_contexts_exhaust_every_field_type_and_required_removal() {
    let base = keyed();
    let cert = json(std::str::from_utf8(&base.cert_bytes).unwrap());
    let mut paths = vec![];
    for (hash, r) in cert.get("regions").unwrap().as_obj().unwrap() {
        let mut p = vec![
            Part::Key("regions".into()),
            Part::Key(hash.clone()),
            Part::Key("resource_contexts".into()),
        ];
        paths.push(p.clone());
        positions(r.get("resource_contexts").unwrap(), &mut p, &mut paths);
    }
    let mut removals = 0;
    for p in &paths {
        let mut inputs = base.clone();
        let mut changed = cert.clone();
        *at(&mut changed, p) = Value::Null;
        inputs.cert_bytes = jcs(&changed).unwrap();
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_SCHEMA"),
            "type {p:?}: {verdict:?}"
        );
        if let Some(Part::Key(key)) = p.last() {
            let mut changed = cert.clone();
            let Value::Obj(m) = at(&mut changed, &p[..p.len() - 1]) else {
                panic!()
            };
            m.remove(key);
            inputs.cert_bytes = jcs(&changed).unwrap();
            let verdict = check_certificate(&inputs);
            if key == "resource_contexts" {
                assert_eq!(
                    verdict.overall, "verified",
                    "optional singleton array {p:?}: {verdict:?}"
                );
                continue;
            }
            let expected = match key.as_str() {
                "time" | "peak_qubits[fast]" | "qubit_second[fast]" => "E_CERT_RESOURCE",
                "x" => "E_CERT_RESOURCE",
                _ => "E_CERT_SCHEMA",
            };
            assert!(
                codes(&verdict).contains(&expected),
                "remove {p:?} expected {expected}: {verdict:?}"
            );
            removals += 1;
        }
    }
    assert_eq!((paths.len(), removals), (26, 21));
}
#[test]
fn uniform_frames_and_each_selected_descriptor_resolve_c_at_every_reached_invocation() {
    for uniform in [false, true] {
        let mut inputs = repeated(false);
        mutate_child(&mut inputs, |r| {
            let mut frames = r.get("frame_contexts").unwrap().as_arr().unwrap().to_vec();
            for e in &mut frames {
                let Value::Obj(m) = e else { panic!() };
                let Value::Obj(f) = m.get_mut("frame").unwrap() else {
                    panic!()
                };
                let Value::Arr(slots) = f.get_mut("slots").unwrap() else {
                    panic!()
                };
                set(
                    &mut slots[0],
                    "width",
                    json(r#"{"op":"+","args":[{"var":"c"},1]}"#),
                );
            }
            if uniform {
                let Value::Obj(m) = r else { panic!() };
                m.remove("frame_contexts");
                m.insert("frame".into(), frames[0].get("frame").unwrap().clone());
            } else {
                set(r, "frame_contexts", Value::Arr(frames));
            }
        });
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_FRAME"),
            "uniform={uniform}: {verdict:?}"
        );
    }
}

#[test]
fn a_recompute_cleanup_cannot_be_certified_using_the_two_gate_reverse_trace() {
    use bennett_cert_checker::object::Object;
    let mut inputs = repeated(false);
    let old_object = Object::parse(&inputs.object_text).unwrap();
    let old = old_object
        .symbol("root")
        .unwrap()
        .closure_hash
        .as_ref()
        .unwrap()
        .clone();
    inputs.object_text = rebind_object_symbols(&inputs.object_text.replace(
        "qpred c\naction_call @flip (t) () () ()\nqunpred c",
        "adj action_call @flip (t) () () ()",
    ));
    let object = Object::parse(&inputs.object_text).unwrap();
    let h = object
        .symbol("root")
        .unwrap()
        .closure_hash
        .as_ref()
        .unwrap()
        .clone();
    let child = object
        .symbol("flip")
        .unwrap()
        .closure_hash
        .as_ref()
        .unwrap()
        .clone();
    let cert = std::str::from_utf8(&inputs.cert_bytes)
        .unwrap()
        .replace(&old, &h);
    let mut cert = json(&cert);
    let Value::Obj(top) = &mut cert else { panic!() };
    set(
        top.get_mut("bindings").unwrap(),
        "object_hash",
        Value::Str(object.object_hash.clone()),
    );
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let record = regions.get_mut(&child).unwrap();
    let mut claim = record.get("claims").unwrap().as_arr().unwrap()[0].clone();
    let mut subject = claim.get("subject").unwrap().clone();
    set(&mut subject, "inherited_controls", json("0"));
    set(&mut claim, "subject", subject);
    set(record, "claims", Value::Arr(vec![claim]));
    let mut verification = record.get("verification").unwrap().clone();
    set(&mut verification, "unitary", json("[0]"));
    set(record, "verification", verification);
    set(
        record,
        "error",
        json(r#"{"forward:0":{"local":{"operator_norm":0},"inclusive":{"operator_norm":0}}}"#),
    );
    for field in ["frame_contexts", "resource_contexts"] {
        let mut entries = record.get(field).unwrap().as_arr().unwrap().to_vec();
        let mut key = entries[1]
            .get("context_key")
            .unwrap()
            .as_arr()
            .unwrap()
            .to_vec();
        set(&mut key[0], "point", Value::Str("entry:1".into()));
        set(&mut entries[1], "context_key", Value::Arr(key));
        if field == "resource_contexts" {
            let resources = entries[0].get("resources").unwrap().clone();
            set(&mut entries[1], "resources", resources);
        }
        set(record, field, Value::Arr(entries));
    }
    set(
        top.get_mut("composition").unwrap(),
        "multiplicities",
        json(&format!(r#"{{"{h}:forward:0":1,"{child}:forward:0":2}}"#)),
    );
    inputs.cert_bytes = jcs(&cert).unwrap();
    let mut schedule = json(std::str::from_utf8(inputs.schedule_bytes.as_ref().unwrap()).unwrap());
    let target = json(std::str::from_utf8(inputs.target_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inputs, schedule.clone(), target.clone());
    let baseline = check_certificate(&inputs);
    assert_eq!(baseline.overall, "verified", "{baseline:?}");
    set(
        &mut schedule,
        "entries",
        json(&format!(
            r#"[{{"path":[{{"symbol":"{h}","point":"entry:1","iteration":[],"instance":[]}}],"step":"*","phase":"*","generated":0,"kind":"cleanup","cleanup":{{"realisation":"recompute"}}}}]"#
        )),
    );
    bind_schedule(&mut inputs, schedule, target);
    let verdict = check_certificate(&inputs);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert!(
        encode(&verdict.to_value()).contains("recompute"),
        "{verdict:?}"
    );
}
