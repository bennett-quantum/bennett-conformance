mod common;
use bennett_cert_checker::cert::{check_certificate, Inputs};
use bennett_cert_checker::json::{n, Value};
use common::*;

fn x_model(negative: bool) -> Value {
    let one = if negative { "-1" } else { "1" };
    json(&format!(
        r#"{{"version":"bennett.native-model/1","kind":"unitary","arity":1,"basis":["t"],"parameter_bindings":{{"k":"k","c":"c"}},"control_models":{{"kind":"controlled_U","controls":"c","disabled":"identity"}},"operand_map":[0],"data":{{"matrix":[[[["0","1"],["0","1"]],[["{one}","1"],["0","1"]]],[[["{one}","1"],["0","1"]],[["0","1"],["0","1"]]]]}}}}"#
    ))
}
fn fixture(model: Value, epsilon: u32, bound: u32) -> Inputs {
    let (mut inputs, h) = standalone(
        "action @a (t:wire mut) () () () ()\n^entry():\nx t",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a",
        vec![json(&format!(
            r#"{{"kind":"numerical","metric":"operator_norm","bound":{bound},"precision_bits":128}}"#
        ))],
    );
    let base = controlled();
    let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let mut entry = json(&format!(
        r#"{{"op":"x","classes":["fast"],"controls":{{"min":0,"max":0}},"epsilon":{epsilon},"cost":{{"time":1}}}}"#
    ));
    set(&mut entry, "realisation_model", model);
    set(&mut target, "gates", Value::Arr(vec![entry]));
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let mut bindings = schedule.get("bindings").unwrap().clone();
    set(&mut bindings, "epsilon", n(epsilon));
    set(&mut schedule, "bindings", bindings);
    // Resolve the root source point and its requested epsilon share.
    set(
        &mut schedule,
        "entries",
        json(&format!(
            r#"[{{"kind":"gate","path":[{{"symbol":"{h}","point":"entry:0","iteration":[]}}],"generated":"*","phase":"*","step":"*","gate":{{"epsilon_share":{epsilon},"realisation":"native"}}}}]"#
        )),
    );
    bind_schedule(&mut inputs, schedule, target);
    if epsilon == 0 {
        single_root_zero_error_reports(
            &mut inputs,
            json(r#"{"time":1,"peak_qubits":1,"qubit_second":1}"#),
            json(r#"{"x":{"count":1}}"#),
        );
    }
    inputs
}
#[test]
fn an_exact_rational_model_is_reconstructed_before_a_numerical_claim_passes() {
    let verdict = check_certificate(&fixture(x_model(false), 0, 0));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(
        verdict.regions[0].claims[0].1.outcome, "accept",
        "{verdict:?}"
    );
    assert_eq!(
        verdict.regions[0].claims[0].1.computed.as_deref(),
        Some("0")
    );
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}
#[test]
fn a_supplied_model_cannot_contradict_a_zero_epsilon_opcode() {
    let verdict = check_certificate(&fixture(x_model(true), 0, 3));
    assert!(codes(&verdict).contains(&"E_CERT_TARGET"), "{verdict:?}");
}
#[test]
fn positive_epsilon_checks_unitarity_accuracy_and_phase_separately() {
    let verdict = check_certificate(&fixture(x_model(true), 2, 3));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    let claim = &verdict.regions[0].claims[0].1;
    assert_eq!(claim.outcome, "accept", "{verdict:?}");
    assert_ne!(claim.computed.as_deref(), Some("0"));
    let too_small = check_certificate(&fixture(x_model(true), 1, 3));
    assert!(
        codes(&too_small).contains(&"E_CERT_TARGET"),
        "{too_small:?}"
    );
    let mut nonunitary = x_model(false);
    set(
        &mut nonunitary,
        "data",
        json(
            r#"{"matrix":[[[["0","1"],["0","1"]],[["1","2"],["0","1"]]],[[["1","2"],["0","1"]],[["0","1"],["0","1"]]]]}"#,
        ),
    );
    let invalid = check_certificate(&fixture(nonunitary, 2, 3));
    assert!(codes(&invalid).contains(&"E_CERT_TARGET"), "{invalid:?}");
}
#[test]
fn unknown_model_versions_are_unchecked_and_never_replaced_by_the_ideal() {
    let mut model = x_model(false);
    set(
        &mut model,
        "version",
        Value::Str("bennett.native-model/99".into()),
    );
    let verdict = check_certificate(&fixture(model, 0, 0));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(
        verdict.regions[0].claims[0].1.outcome, "unchecked",
        "{verdict:?}"
    );
}

#[test]
fn rational_and_complex_unitarity_are_exact_including_nondyadic_entries() {
    for (matrix, bound, exact_display) in [
        (
            r#"[[[["3","5"],["0","1"]],[["-4","5"],["0","1"]]],[[["4","5"],["0","1"]],[["3","5"],["0","1"]]]]"#,
            3,
            None,
        ),
        (
            r#"[[[["0","1"],["0","1"]],[["0","1"],["1","1"]]],[[["0","1"],["1","1"]],[["0","1"],["0","1"]]]]"#,
            2,
            Some("2"),
        ),
    ] {
        let mut model = x_model(false);
        set(
            &mut model,
            "data",
            json(&format!(r#"{{"matrix":{matrix}}}"#)),
        );
        let verdict = check_certificate(&fixture(model.clone(), 2, bound));
        assert!(codes(&verdict).is_empty(), "{verdict:?}");
        let claim = &verdict.regions[0].claims[0].1;
        assert_eq!(claim.outcome, "accept", "{verdict:?}");
        if let Some(display) = exact_display {
            assert_eq!(claim.computed.as_deref(), Some(display));
        } else {
            // Exact model accuracy is 2, but the mandated outward Frobenius
            // calculation encloses its nondyadic intermediates above 2.
            assert_ne!(claim.computed.as_deref(), Some("2"));
            let boundary = check_certificate(&fixture(model, 2, 2));
            assert!(
                codes(&boundary).contains(&"E_CERT_NUMERICAL"),
                "{boundary:?}"
            );
        }
    }
}

fn mutants(value: &Value, remove: bool) -> Vec<(String, Value)> {
    let mut out = vec![];
    match value {
        Value::Obj(fields) => {
            for (key, child) in fields {
                let mut changed = value.clone();
                let Value::Obj(ref mut fields) = changed else {
                    panic!()
                };
                if remove {
                    fields.remove(key);
                } else {
                    fields.insert(key.clone(), Value::Null);
                }
                out.push((key.clone(), changed));
                for (sub, replacement) in mutants(child, remove) {
                    let mut changed = value.clone();
                    set(&mut changed, key, replacement);
                    out.push((format!("{key}.{sub}"), changed));
                }
            }
        }
        Value::Arr(items) => {
            for (index, child) in items.iter().enumerate() {
                let mut changed = items.clone();
                if remove {
                    changed.remove(index);
                } else {
                    changed[index] = Value::Null;
                }
                out.push((index.to_string(), Value::Arr(changed)));
                for (sub, replacement) in mutants(child, remove) {
                    let mut changed = items.clone();
                    changed[index] = replacement;
                    out.push((format!("{index}.{sub}"), Value::Arr(changed)));
                }
            }
        }
        _ => {}
    }
    out
}
#[test]
fn every_native_model_field_and_array_position_rejects_bad_types() {
    let model = x_model(false);
    let all = mutants(&model, false);
    assert_eq!(all.len(), 46);
    for (path, model) in all {
        let verdict = check_certificate(&fixture(model, 0, 0));
        assert!(
            codes(&verdict).contains(&"E_CERT_TARGET"),
            "{path}: {verdict:?}"
        );
    }
}
#[test]
fn every_required_native_model_field_and_position_rejects_removal() {
    let model = x_model(false);
    let all = mutants(&model, true);
    let mut rejected = 0;
    for (path, model) in all {
        // Unused parameter declarations are optional members of their map.
        if matches!(
            path.as_str(),
            "parameter_bindings.k" | "parameter_bindings.c"
        ) {
            continue;
        }
        let verdict = check_certificate(&fixture(model, 0, 0));
        assert!(
            codes(&verdict).contains(&"E_CERT_TARGET"),
            "{path}: {verdict:?}"
        );
        rejected += 1;
    }
    assert_eq!(rejected, 44);
}
#[test]
fn unused_parameter_declarations_can_be_removed_without_changing_the_model() {
    for bindings in [r#"{"k":"k"}"#, r#"{"c":"c"}"#, "{}"] {
        let mut model = x_model(false);
        set(&mut model, "parameter_bindings", json(bindings));
        let verdict = check_certificate(&fixture(model, 0, 0));
        assert_eq!(
            verdict.regions[0].claims[0].1.outcome, "accept",
            "{verdict:?}"
        );
    }
}
#[test]
fn unchecked_representation_and_operand_embeddings_never_idealise_models() {
    let mut model = x_model(false);
    let mut data = model.get("data").unwrap().clone();
    set(&mut data, "kraus", Value::Arr(vec![]));
    set(&mut model, "data", data);
    let verdict = check_certificate(&fixture(model, 0, 0));
    assert!(codes(&verdict).is_empty(), "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "unchecked");
    let mut model = x_model(false);
    set(&mut model, "operand_map", json("[1]"));
    let verdict = check_certificate(&fixture(model, 0, 0));
    assert!(codes(&verdict).contains(&"E_CERT_TARGET"), "{verdict:?}");
}

#[test]
fn realised_truth_tables_cannot_reuse_the_ideal_when_a_model_changes_phase() {
    for (negative, version, expected) in [
        (false, "bennett.native-model/1", "proof"),
        (true, "bennett.native-model/1", "unchecked"),
        (false, "future/99", "unchecked"),
    ] {
        let mut model = x_model(negative);
        set(&mut model, "version", Value::Str(version.into()));
        let mut inputs = fixture(model, 2, 3);
        let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
        let Value::Obj(top) = &mut cert else { panic!() };
        let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
            panic!()
        };
        let record = regions.values_mut().next().unwrap();
        let subject = record.get("claims").unwrap().as_arr().unwrap()[0]
            .get("subject")
            .unwrap()
            .clone();
        let mut claim = json(
            r#"{"kind":"exhaustive","label":"proof","domain":{"bundles":[{"name":"t","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]},"expected":{"table":[{"before":{"bundles":["0"],"borrowed":[],"regions":[]},"after":{"bundles":["1"],"borrowed":[],"regions":[]}},{"before":{"bundles":["1"],"borrowed":[],"regions":[]},"after":{"bundles":["0"],"borrowed":[],"regions":[]}}]}}"#,
        );
        set(&mut claim, "subject", subject);
        set(record, "claims", Value::Arr(vec![claim]));
        let mut verification = record.get("verification").unwrap().clone();
        set(&mut verification, "unitary", Value::Arr(vec![]));
        set(record, "verification", verification);
        inputs.cert_bytes = bennett_cert_checker::json::jcs(&cert).unwrap();
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).is_empty(), "{verdict:?}");
        assert_eq!(
            verdict.regions[0].claims[0].1.outcome, expected,
            "{verdict:?}"
        );
    }
}
