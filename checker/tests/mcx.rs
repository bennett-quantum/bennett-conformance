mod common;
use bennett_cert_checker::{
    cert::check_certificate,
    domain::Tuple,
    eval::{eval_action_tuple, EvalError},
    object::Object,
};
use common::*;
fn object(gate: &str, coherent: bool) -> Object {
    let h = if coherent { "h t\nh t\n" } else { "" };
    let (inputs,_)=standalone(&format!("action @a (t:wire[1] mut) (s:wire in, c:wire[2] in, r:wire in) () () ()\n^entry():\n{h}ctl r {gate}"),
        r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1,2,1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,"a",vec![]);
    Object::parse(&inputs.object_text).unwrap()
}
#[test]
fn mixed_bundle_scalar_and_empty_lists_replay_every_controlled_basis_column() {
    let mut cases = 0;
    for coherent in [false, true] {
        for controls in ["(s, c)", "(c, s)", "(c[-0], c[1], s)", "()"] {
            let object = object(&format!("mcx {controls} t"), coherent);
            let symbol = object.symbol("a").unwrap();
            let iface = object.interface("a").unwrap();
            for inherited in [None, Some(0u32), Some(1)] {
                for t in 0..2u32 {
                    for s in 0..2u32 {
                        for c in 0..4u32 {
                            for r in 0..2u32 {
                                let mut bundles = vec![];
                                if let Some(control) = inherited {
                                    bundles.push(control.into());
                                }
                                bundles.extend([t.into(), s.into(), c.into(), r.into()]);
                                let before = Tuple {
                                    bundles,
                                    borrowed: vec![],
                                    regions: vec![],
                                    args: vec![],
                                };
                                let mut expected = before.clone();
                                let on = r == 1
                                    && inherited != Some(0)
                                    && (controls == "()" || s == 1 && c == 3);
                                expected.bundles[usize::from(inherited.is_some())] =
                                    (t ^ u32::from(on)).into();
                                let after = eval_action_tuple(
                                    &object,
                                    symbol,
                                    &iface,
                                    &before,
                                    &[],
                                    usize::from(inherited.is_some()),
                                    false,
                                    0,
                                )
                                .unwrap();
                                assert_eq!(after,expected,"{controls}, coherent={coherent}, inherited={inherited:?}, t={t}, s={s}, c={c}, r={r}");
                                assert_eq!(
                                    eval_action_tuple(
                                        &object,
                                        symbol,
                                        &iface,
                                        &after,
                                        &[],
                                        usize::from(inherited.is_some()),
                                        true,
                                        0
                                    )
                                    .unwrap(),
                                    before
                                );
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 768);
}
#[test]
fn repeated_overlapping_nonscalar_and_invalid_indexed_operands_reject_even_when_disabled() {
    let gates = [
        "mcx (c, c) t",
        "mcx (c, c[0]) t",
        "mcx (c[0], c[0]) t",
        "mcx (s, c, s) t",
        "mcx (t) t",
        "mcx (s) c",
        "mcx (r, c) t",
        "mcx (s) c[2]",
        "mcx (c[+0]) t",
        "mcx (c[00]) t",
        "mcx c t",
        "mcx (c,s) t",
    ];
    let mut cases = 0;
    for coherent in [false, true] {
        for gate in gates {
            // Inject malformed operands after parsing a canonical object so
            // this also tests the evaluator's own validation. The text-entry
            // rejection of noncanonical integers is covered in lexical.rs.
            let mut object = object("mcx (s) t", coherent);
            let function = &mut object.functions[0];
            function.bytes = function
                .bytes
                .replace("ctl r mcx (s) t", &format!("ctl r {gate}"));
            for line in &mut function.blocks[0].instructions {
                if line == "ctl r mcx (s) t" {
                    *line = format!("ctl r {gate}");
                }
            }
            let symbol = object.symbol("a").unwrap();
            let iface = object.interface("a").unwrap();
            for r in 0..2u32 {
                let before = Tuple {
                    bundles: vec![0u32.into(), 0u32.into(), 0u32.into(), r.into()],
                    borrowed: vec![],
                    regions: vec![],
                    args: vec![],
                };
                let result = eval_action_tuple(&object, symbol, &iface, &before, &[], 0, false, 0);
                assert!(
                    matches!(result,Err(EvalError::Reject(e))if e.code=="E_CERT_OBJECT"),
                    "{gate}, coherent={coherent}, r={r}"
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 48);
}

#[test]
fn native_matching_and_costs_use_three_explicit_and_four_total_controls() {
    use bennett_cert_checker::json::Value;
    let source = object("mcx (s, c) t", false)
        .function("a")
        .unwrap()
        .bytes
        .clone();
    for time in [3, 4] {
        let (mut inputs, _) = standalone(
            &source,
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1,2,1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
            "a",
            vec![json(
                r#"{"kind":"numerical","metric":"operator_norm","bound":0,"precision_bits":128}"#,
            )],
        );
        let base = controlled();
        let mut target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
        set(
            &mut target,
            "gates",
            json(
                r#"[{"op":"mcx","classes":["fast"],"controls":{"min":4,"max":4},"epsilon":0,"cost":{"time":{"var":"k"}}}]"#,
            ),
        );
        set(
            &mut target,
            "classes",
            json(r#"{"fast":{"capacity":5,"dirty_rate":1,"qubit_second":1}}"#),
        );
        let mut schedule =
            json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
        set(&mut schedule, "entries", Value::Arr(vec![]));
        bind_schedule(&mut inputs, schedule, target);
        single_root_zero_error_reports(
            &mut inputs,
            json(&format!(
                r#"{{"time":"{time}","peak_qubits[fast]":"5","qubit_second[fast]":"15"}}"#
            )),
            json(r#"{"mcx":{"count":1}}"#),
        );
        let verdict = check_certificate(&inputs);
        if time == 3 {
            assert!(codes(&verdict).is_empty(), "{verdict:?}");
            assert_eq!(verdict.regions[0].claims[0].1.outcome, "accept");
            assert_eq!(verdict.overall, "verified", "{verdict:?}");
        } else {
            assert!(codes(&verdict).contains(&"E_CERT_RESOURCE"), "{verdict:?}");
        }
    }
}
