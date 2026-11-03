//! Static source point ordinals and dynamic qloop iterations are separate
//! schedule coordinates. All test expected counts and timing are explicit.
mod common;
use bennett_cert_checker::cert::{check_certificate, Inputs};
use bennett_cert_checker::json::{jcs, n, Value};
use common::*;

fn fixture(body: &str, bounds: &str, count: u64, time: u64) -> (Inputs, String) {
    let source=format!("perm_action @looped (t:wire mut) () () () ()\nregs bounds {{ {bounds} }}\n^entry():\n{body}");
    let sig = format!(
        r#"{{"looped":{{"abi":"0.9","args":[],"bounds":[{bounds}],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}}}"#
    );
    let (mut inp, h) = standalone(
        &source,
        &sig,
        "looped",
        vec![json(
            r#"{"bound":0,"kind":"numerical","metric":"operator_norm","precision_bits":64}"#,
        )],
    );
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let region = regions.get_mut(&h).unwrap();
    set(region, "bounds", json(&format!("[{bounds}]")));
    set(
        region,
        "resources",
        json(&format!(
            r#"{{"peak_qubits[fast]":"1","qubit_second[fast]":"{time}","time":"{time}"}}"#
        )),
    );
    set(
        region,
        "gates_used",
        if count == 0 {
            json("{}")
        } else {
            json(&format!(r#"{{"x":{{"count":{count}}}}}"#))
        },
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
        json(
            r#"[{"op":"x","classes":["fast"],"controls":{"min":0,"max":0},"epsilon":0,"cost":{"time":1}}]"#,
        ),
    );
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", Value::Arr(vec![]));
    bind_schedule(&mut inp, schedule, target);
    (inp, h)
}
#[test]
fn nested_counted_loops_and_sequential_blocks_have_exact_counts() {
    for (body, bounds, count) in [
        ("qloop 3 {\nx t\n}", "3", 3),
        ("qloop 2 {\nx t\nqloop 3 {\nx t\n}\n}", "2,3", 8),
        ("x t\n^second():\nx t", "", 2),
    ] {
        let (inp, _) = fixture(body, bounds, count, count);
        let verdict = check_certificate(&inp);
        assert_eq!(verdict.overall, "verified", "{body}: {verdict:?}");
    }
}
#[test]
fn loop_iteration_schedule_selectors_use_static_point_and_dynamic_iteration() {
    let (mut inp, h) = fixture("qloop 2 {\nx t\n}", "2", 2, 4);
    let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    set(
        &mut schedule,
        "entries",
        json(&format!(
            r#"[{{"path":[{{"symbol":"{h}","point":"entry:1","iteration":[1]}}],"step":"*","phase":"*","generated":0,"kind":"start","start":{{"time":3}}}}]"#
        )),
    );
    let target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inp, schedule, target);
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}
#[test]
fn zero_trip_loop_has_no_operation_and_cannot_witness_a_selector_tie() {
    let (mut inp, h) = fixture("qloop 0 {\nx t\n}", "0", 0, 0);
    let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    let entry = json(&format!(
        r#"{{"path":[{{"symbol":"{h}","point":"entry:1","iteration":"*"}}],"step":"*","phase":"*","generated":0,"kind":"gate","gate":{{"realisation":"native"}}}}"#
    ));
    set(
        &mut schedule,
        "entries",
        Value::Arr(vec![entry.clone(), entry]),
    );
    let target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
    bind_schedule(&mut inp, schedule, target);
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}

#[test]
fn coherent_qif_emits_both_arms_and_selector_flips_with_distinct_generated_ordinals() {
    let source="action @branch (t:wire mut) (c:wire in) () () ()\n^entry():\nqif c {\nh t\n} {\ns t\n}\nx t";
    let sig = r#"{"branch":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#;
    let (mut inp, h) = standalone(
        source,
        sig,
        "branch",
        vec![json(
            r#"{"bound":0.0000000001,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#,
        )],
    );
    let object = bennett_cert_checker::object::Object::parse(&inp.object_text).unwrap();
    assert_eq!(
        object.instruction_count("branch", "entry"),
        Some(4),
        "closing braces do not have instruction ordinals"
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
        json(r#"{"peak_qubits[fast]":"2","qubit_second[fast]":"14","time":"7"}"#),
    );
    set(
        region,
        "gates_used",
        json(r#"{"h":{"count":1},"s":{"count":1},"x":{"count":3}}"#),
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
        json(
            r#"[{"op":"x","classes":["fast"],"controls":{"min":0,"max":1},"epsilon":0,"cost":{"time":1}},{"op":"h","classes":["fast"],"controls":{"min":0,"max":1},"epsilon":0,"cost":{"time":1}},{"op":"s","classes":["fast"],"controls":{"min":0,"max":1},"epsilon":0,"cost":{"time":1}}]"#,
        ),
    );
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    // The second flip starts at 5; the following source X is entry:3 and
    // follows at 6. Confusing generated=0 with generated=1 breaks this report.
    set(
        &mut schedule,
        "entries",
        json(&format!(
            r#"[{{"path":[{{"symbol":"{h}","point":"entry:0","iteration":[]}}],"step":"*","phase":"*","generated":1,"kind":"start","start":{{"time":5}}}}]"#
        )),
    );
    bind_schedule(&mut inp, schedule, target);
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");

    // Pin both reconstructions to independently specified full complex
    // columns: X*S when c=0; X*H when c=1. This also fixes global phase.
    use bennett_cert_checker::cert::{parse_invocation, LoadedDocs, Subject, REF_09_HASH};
    use bennett_cert_checker::rat::{parse_decimal, Rat};
    let cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let invocation = parse_invocation(
        cert.get("bindings").unwrap().get("invocation").unwrap(),
        &[],
        &object,
        "test",
    )
    .unwrap();
    let schedule_hash = bennett_cert_checker::sha256_hex(inp.schedule_bytes.as_ref().unwrap());
    let docs = LoadedDocs {
        root_closure: h.clone(),
        root_invocation: Some(invocation),
        object,
        target: Some(json(
            std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap(),
        )),
        schedule: Some(json(
            std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap(),
        )),
        schedule_hash: Some(schedule_hash.clone()),
        expansion: Some(("ref".into(), "0.9".into(), REF_09_HASH.into())),
        expansion_unknown: false,
    };
    let subject = Subject {
        closure_hash: h,
        variant: "forward".into(),
        inherited_controls: 0,
        schedule_hash: Some(schedule_hash),
    };
    let matrices =
        bennett_cert_checker::realisation::subject_matrices(&subject, &docs, &inp, 128).unwrap();
    let rlo = parse_decimal("0.7071067811865475244008443621048490392848").unwrap();
    let rhi = parse_decimal("0.7071067811865475244008443621048490392849").unwrap();
    for matrix in [&matrices[0].1, &matrices[0].2] {
        for row in 0..4 {
            for col in 0..4 {
                let (lo, hi) = match (row, col) {
                    (1, 0) => (Rat::from_integer(1.into()), Rat::from_integer(1.into())),
                    (2, 2) | (3, 2) | (3, 3) => (rlo.clone(), rhi.clone()),
                    (2, 3) => (-rhi.clone(), -rlo.clone()),
                    _ => (Rat::from_integer(0.into()), Rat::from_integer(0.into())),
                };
                let im = Rat::from_integer(if (row, col) == (0, 1) {
                    1.into()
                } else {
                    0.into()
                });
                let z = matrix.get(row, col);
                assert!(z.re.lo <= lo && z.re.hi >= hi, "real[{row},{col}]: {z:?}");
                assert!(
                    z.im.lo <= im && z.im.hi >= im,
                    "imaginary[{row},{col}]: {z:?}"
                );
            }
        }
    }
}
