use crate::common;
use bennett_cert_checker::eval::{eval_oracle_bounded, Classical, EvalError};
use bennett_cert_checker::object::Object;
use num_bigint::BigUint;
const SOURCE: &str = include_str!("fixtures/replay-differential/lproc-recursive-sum/object.bisa");
fn run(
    object: &Object,
    x: u32,
    b: u32,
    stack: u32,
    depth: u32,
    host: u64,
) -> Result<(BigUint, BigUint), EvalError> {
    let out = eval_oracle_bounded(
        object,
        object.symbol("o").unwrap(),
        &[x.into()],
        &[],
        &[],
        &[b.into(), 0u32.into(), stack.into(), depth.into()],
        0,
        host,
    )?;
    Ok((out.payload, out.code))
}
fn body(text: &str) -> Object {
    let (functions, layout) = SOURCE.split_once("\nsection layout\n").unwrap();
    let procedure = functions.find("lproc @f").unwrap();
    let start = procedure + functions[procedure..].find("^entry():").unwrap();
    Object::parse(&format!(
        "{}{}\n\nsection layout\n{}",
        &functions[..start],
        text,
        layout
    ))
    .unwrap()
}
#[test]
fn recursive_rows_charge_exactly_eight_per_positive_input_plus_six() {
    let object = Object::parse(SOURCE).unwrap();
    let mut cases = 0;
    for x in 0..16 {
        let required = 8 * x + 6;
        for b in 0..=required + 1 {
            let expected = if b < required {
                (0u32.into(), 1u32.into())
            } else {
                (x.into(), 0u32.into())
            };
            assert_eq!(
                run(&object, x, b, 200, 50, 10000).unwrap(),
                expected,
                "x={x}, B={b}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 1088);
}
#[test]
fn stack_and_depth_limits_are_checked_before_call_effects() {
    let object = Object::parse(SOURCE).unwrap();
    let mut cases = 0;
    for x in 0..16 {
        let needed = x + 1;
        for depth in 0..=needed {
            let expected = if depth < needed {
                (0u32.into(), 3u32.into())
            } else {
                (x.into(), 0u32.into())
            };
            assert_eq!(
                run(&object, x, 1000, 200, depth, 10000).unwrap(),
                expected,
                "x={x}, depth={depth}"
            );
            cases += 1;
        }
        for stack in 0..=9 * needed {
            let expected = if stack < 9 * needed {
                (0u32.into(), 3u32.into())
            } else {
                (x.into(), 0u32.into())
            };
            assert_eq!(
                run(&object, x, 1000, stack, 50, 10000).unwrap(),
                expected,
                "x={x}, stack={stack}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 1392);
}
#[test]
fn returns_restore_locations_and_never_reclaim_the_stack_counter() {
    let repeated = Object::parse(&SOURCE.replacen(
        "r = lift.call @f (x0)",
        "r = lift.call @f (x0)\nr = lift.call @f (x0)",
        1,
    ))
    .unwrap();
    assert_eq!(
        run(&repeated, 0, 1000, 17, 1, 10000).unwrap(),
        (0u32.into(), 3u32.into())
    );
    assert_eq!(
        run(&repeated, 0, 1000, 18, 1, 10000).unwrap(),
        (0u32.into(), 0u32.into())
    );
    let restored =
        Object::parse(&SOURCE.replace("one = lift.add r one\nlift.ret one", "lift.ret n")).unwrap();
    for x in 0..16 {
        assert_eq!(
            run(&restored, x, 1000, 200, 50, 10000).unwrap(),
            (x.into(), 0u32.into())
        );
    }
}
#[test]
fn procedure_labels_are_global_indices_and_indirect_jumps_are_function_local() {
    let labels = body("^entry():\nz = lift.label ^d\nlift.ret z\n^d():\nlift.ret n");
    assert_eq!(
        run(&labels, 3, 100, 200, 50, 1000).unwrap(),
        (4u32.into(), 0u32.into())
    );
    let escaped = body(
        "^entry():\nz = lift.label ^entry\none = lift.const.i4 2\nz = lift.sub z one\nqjumpr z",
    );
    assert_eq!(
        run(&escaped, 3, 100, 200, 50, 1000).unwrap(),
        (0u32.into(), 4u32.into())
    );
    let middle=body("^entry():\nz = lift.label ^middle\none = lift.const.i4 1\nz = lift.add z one\nqjumpr z\n^middle():\nn = lift.const.i4 14\nlift.ret n");
    assert_eq!(
        run(&middle, 3, 100, 200, 50, 1000).unwrap(),
        (3u32.into(), 0u32.into())
    );
}
#[test]
fn procedure_execution_keeps_isa_and_host_work_limits_separate() {
    let object = Object::parse(SOURCE).unwrap();
    assert!(
        matches!(run(&object,15,1000,200,50,125),Err(EvalError::Unchecked(s))if s.contains("host limit"))
    );
    assert_eq!(
        run(&object, 15, 125, 200, 50, 125).unwrap(),
        (0u32.into(), 1u32.into())
    );
    assert_eq!(
        run(&object, 15, 126, 200, 50, 126).unwrap(),
        (15u32.into(), 0u32.into())
    );
}
#[test]
fn recursive_classical_fixed_binding_is_forwarded_without_stack_cells() {
    let source = SOURCE
        .replace(r#""args":[]"#, r#""args":["i4"]"#)
        .replace("(f:wire[4]) () ()", "(f:wire[4]) () (a:i4)")
        .replace("(r:reg[4]) () () ()", "(r:reg[4]) () (bias:i4) ()")
        .replace("lift.call @f (x0)", "lift.call @f (x0) () (a) ()")
        .replace("lift.call @f (m)", "lift.call @f (m) () (bias) ()");
    let object = Object::parse(&source).unwrap();
    let result = eval_oracle_bounded(
        &object,
        object.symbol("o").unwrap(),
        &[3u32.into()],
        &[],
        &[Classical::Bits(2u32.into())],
        &[1000u32.into(), 0u32.into(), 200u32.into(), 50u32.into()],
        0,
        10000,
    );
    let out = result.unwrap();
    assert_eq!((out.payload, out.code), (3u32.into(), 0u32.into()));
}

const MUTUAL: &str = include_str!("fixtures/replay-differential/lproc-mutual-09.bisa");
fn no_input(object: &Object, b: u32) -> Result<(BigUint, BigUint), EvalError> {
    let result = eval_oracle_bounded(
        object,
        object.symbol("main").unwrap(),
        &[],
        &[],
        &[],
        &[b.into(), 0u32.into(), 32u32.into(), 4u32.into()],
        0,
        1000,
    )?;
    Ok((result.payload, result.code))
}
#[test]
fn mutual_scc_members_follow_name_indices_even_when_member_hashes_disagree() {
    // Published C-mutual-label-index, with 0.9 version/signature arrays.
    // main occupies 0..2, f 2..4, g 4..10. Exactly six rows execute.
    let object = Object::parse(MUTUAL).unwrap();
    assert!(object.stated_hashes_ok());
    assert!(object.symbol("f").unwrap().closure_hash > object.symbol("g").unwrap().closure_hash);
    assert_eq!(no_input(&object, 5).unwrap(), (0u32.into(), 1u32.into()));
    assert_eq!(no_input(&object, 6).unwrap(), (4u32.into(), 0u32.into()));
}
#[test]
fn independent_procedures_follow_closure_keys_rather_than_names_or_text_order() {
    let (prefix, _) = MUTUAL.split_once("section functions\n").unwrap();
    let (_, layout) = MUTUAL.split_once("\nsection layout\n").unwrap();
    let object = Object::parse(&format!(
        "{prefix}section functions\n\
oracle @main () (t:wire[16]) (fault:wire[4]) () ()\n\
regs {{ r:reg[16] }}\n\
^entry():\n\
r = lift.call @f ()\n\
r = lift.call @g ()\n\
lift.retval r\n\
lproc @f () (r:reg[16]) () () ()\n\
^entry():\n\
r = lift.label ^entry\n\
lift.ret r\n\
lproc @g () (r:reg[16]) () () ()\n\
^second():\n\
r = lift.label ^second\n\
lift.ret r\n\nsection layout\n{layout}"
    ))
    .unwrap();
    assert!(object.symbol("g").unwrap().closure_hash < object.symbol("f").unwrap().closure_hash);
    // g is first after the three root rows; f follows at 5.
    assert_eq!(no_input(&object, 7).unwrap(), (3u32.into(), 0u32.into()));
}

#[test]
fn normative_through_forwards_token_classical_and_region_parameters() {
    let object = Object::parse(include_str!(
        "fixtures/replay-differential/lproc-through-09.bisa"
    ))
    .unwrap();
    assert!(object.stated_hashes_ok());
    let mut cases = 0;
    for x in 0..4u32 {
        for a in 0..4u32 {
            for d in 0..256u32 {
                for (b, nested, stack, depth, expected) in [
                    (8u32, 6u32, 8u32, 1u32, (((x + a + d % 4) % 4) ^ 1, 0u32)),
                    (7, 6, 8, 1, (0, 1)),
                    (5, 5, 8, 1, (0, 9)),
                    (8, 6, 7, 1, (0, 3)),
                    (8, 6, 8, 0, (0, 3)),
                ] {
                    let out = eval_oracle_bounded(
                        &object,
                        object.symbol("through").unwrap(),
                        &[x.into()],
                        &[vec![d.into()]],
                        &[
                            Classical::Token([
                                nested.into(),
                                0u32.into(),
                                0u32.into(),
                                0u32.into(),
                            ]),
                            Classical::Bits(a.into()),
                        ],
                        &[b.into(), 0u32.into(), stack.into(), depth.into()],
                        0,
                        100,
                    )
                    .unwrap();
                    assert_eq!(
                        (out.payload, out.code),
                        (expected.0.into(), expected.1.into()),
                        "x={x}, a={a}, d={d}, B={b}, nested={nested}, stack={stack}, depth={depth}"
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 20480);
}

#[test]
fn equal_entry_values_keep_distinct_instances_in_origin_key_order() {
    // Two root argument positions remain different keys even for equal bits.
    // Root 0..3; f[0] 3..5; f[1] 5..7. Classical parameters add no cells.
    let source = SOURCE
        .replace(r#""args":[],"fault""#, r#""args":["i4","i4"],"fault""#)
        .replace(
            r#""args":[],"kind":"lproc""#,
            r#""args":["i4"],"kind":"lproc""#,
        )
        .replace(r#""params":[4]"#, r#""params":[]"#)
        .replace("(f:wire[4]) () ()", "(f:wire[4]) () (a:i4, b:i4)");
    let (prefix, _) = source.split_once("lproc @f").unwrap();
    let (prefix, _) = prefix.split_once("^entry():").unwrap();
    let (_, layout) = source.split_once("\nsection layout\n").unwrap();
    for (calls, expected) in [
        ("(a) ()\nr = lift.call @f () () (b) ()", 5u32),
        ("(b) ()\nr = lift.call @f () () (a) ()", 3u32),
    ] {
        let object = Object::parse(&format!("{prefix}^entry():\nr = lift.call @f () () {calls}\nlift.retval r\nlproc @f () (r:reg[4]) () (bias:i4) ()\n^entry():\nr = lift.label ^entry\nlift.ret r\n\nsection layout\n{layout}")).unwrap();
        for value in 0..16u32 {
            for stack in [7u32, 8] {
                let out = eval_oracle_bounded(
                    &object,
                    object.symbol("o").unwrap(),
                    &[0u32.into()],
                    &[],
                    &[Classical::Bits(value.into()), Classical::Bits(value.into())],
                    &[7u32.into(), 0u32.into(), stack.into(), 1u32.into()],
                    0,
                    100,
                )
                .unwrap();
                let expected = if stack < 8 { (0, 3u32) } else { (expected, 0) };
                assert_eq!(
                    (out.payload, out.code),
                    (expected.0.into(), expected.1.into())
                );
            }
        }
    }
}

fn program(signatures: &str, functions: &str) -> Object {
    let mut symbols = common::json("{}");
    for (i, (name, sig)) in common::json(signatures)
        .as_obj()
        .unwrap()
        .iter()
        .enumerate()
    {
        let mut symbol = common::json("{}");
        common::set(&mut symbol, "name", common::json(&format!("\"{name}\"")));
        common::set(&mut symbol, "kind", sig.get("kind").unwrap().clone());
        common::set(&mut symbol, "signature", sig.clone());
        common::set(
            &mut symbol,
            "interface_hash",
            common::json(&format!("\"{}\"", "0".repeat(64))),
        );
        common::set(&mut symbols, &i.to_string(), symbol);
    }
    let make = |symbols: &bennett_cert_checker::json::Value| {
        format!(
        "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=32\n\nsection symbols\n{}\n\nsection functions\n{}\n\nsection layout\n{{\"A_tab\":0,\"H\":8,\"S_cells\":8,\"globals\":{{}},\"heap_base\":1,\"heap_limit_max\":0,\"interface_regions\":{{}},\"null_size\":1,\"ptr_width\":32}}\n",
        common::encode(symbols), functions.trim_end())
    };
    let object = Object::parse(&make(&symbols)).unwrap();
    for (i, symbol) in object.symbols.iter().enumerate() {
        let bennett_cert_checker::json::Value::Obj(ref mut fields) = symbols else {
            unreachable!()
        };
        common::set(
            fields.get_mut(&i.to_string()).unwrap(),
            "interface_hash",
            common::json(&format!("\"{}\"", symbol.interface_hash)),
        );
    }
    let object = Object::parse(&make(&symbols)).unwrap();
    assert!(object.stated_hashes_ok());
    object
}

#[test]
fn unexecuted_recursive_forwarding_still_instantiates_its_origin_key() {
    let object = program(
        r#"{"o":{"abi":"0.9","args":["i4","i4"],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":4,"tier":"q"},"f":{"abi":"0.9","args":["i4","i4"],"kind":"lproc","mem":[],"params":[],"result":4,"tier":"q","tokens":[]}}"#,
        "oracle @o () (t:wire[4]) (fault:wire[4]) () (a:i4, b:i4)\nregs { r:reg[4] }\n^entry():\nr = lift.call @f () () (b, a) ()\nlift.retval r\nlproc @f () (r:reg[4]) () (a:i4, b:i4) ()\nregs { p:reg[1] }\n^entry():\np = lift.const.i1 1\nqbranch p ^done ^recurse\n^done():\nr = lift.label ^entry\nlift.ret r\n^recurse():\nr = lift.call @f () () (b, a) ()\nlift.ret r",
    );
    let out = eval_oracle_bounded(
        &object,
        object.symbol("o").unwrap(),
        &[],
        &[],
        &[Classical::Bits(0u32.into()), Classical::Bits(0u32.into())],
        &[6u32.into(), 0u32.into(), 5u32.into(), 1u32.into()],
        0,
        100,
    )
    .unwrap();
    // f[0,1] spans 2..8, f[1,0] spans 8..14, although recursion is disabled.
    assert_eq!((out.payload, out.code), (8u32.into(), 0u32.into()));
}

#[test]
fn forwarding_preserves_i128_pointer_and_f64_raw_bits() {
    let object = program(
        r#"{"o":{"abi":"0.9","args":["i128","ptr","f64"],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":128,"tier":"q"},"f":{"abi":"0.9","args":["i128","ptr","f64"],"kind":"lproc","mem":[],"params":[],"result":128,"tier":"q","tokens":[]}}"#,
        "oracle @o () (t:wire[128]) (fault:wire[4]) () (v:i128, p:ptr, bits:f64)\nregs { r:reg[128] }\n^entry():\nr = lift.call @f () () (v, p, bits) ()\nlift.retval r\nlproc @f () (r:reg[128]) () (v:i128, p:ptr, bits:f64) ()\nregs { pw:reg[128], fw:reg[128] }\n^entry():\npw = lift.zext.i128 p\nfw = lift.zext.i128 bits\nr = lift.xor v pw\nr = lift.xor r fw\nlift.ret r",
    );
    let v = (BigUint::from(1u32) << 127usize) + BigUint::from(3u32);
    for raw in [0x8000_0000_0000_0000u64, 0x7ff8_0000_0000_0001, u64::MAX] {
        let p = BigUint::from(0xffff_fffeu32);
        let out = eval_oracle_bounded(
            &object,
            object.symbol("o").unwrap(),
            &[],
            &[],
            &[
                Classical::Bits(v.clone()),
                Classical::Bits(p.clone()),
                Classical::Bits(raw.into()),
            ],
            &[7u32.into(), 0u32.into(), 6u32.into(), 1u32.into()],
            0,
            100,
        )
        .unwrap();
        assert_eq!(
            (out.payload, out.code),
            ((&v ^ &p ^ BigUint::from(raw)), 0u32.into())
        );
    }
}

#[test]
fn fixed_parameter_names_resolve_to_origins_and_cannot_address_root_regions() {
    let source = include_str!("fixtures/replay-differential/lproc-through-09.bisa");
    let aliased = source
        .replace(
            "(tk:token token) (a:i2 classical) (db:region[0,8] in)",
            "(permit:token token) (bias:i2 classical) (view:region[0,8] in)",
        )
        .replace(
            "lift.oracle_call @plus_db tk (x) r fr (db) (a)",
            "lift.oracle_call @plus_db permit (x) r fr (view) (bias)",
        )
        .replace(
            "lift.perm_call @flip_arg (r) () () (tk, a)",
            "lift.perm_call @flip_arg (r) () () (permit, bias)",
        );
    let object = Object::parse(&aliased).unwrap();
    let args = [
        Classical::Token([6u32.into(), 0u32.into(), 0u32.into(), 0u32.into()]),
        Classical::Bits(2u32.into()),
    ];
    let run = |object: &Object| {
        eval_oracle_bounded(
            object,
            object.symbol("through").unwrap(),
            &[3u32.into()],
            &[vec![255u32.into()]],
            &args,
            &[8u32.into(), 0u32.into(), 8u32.into(), 1u32.into()],
            0,
            100,
        )
    };
    let out = run(&object).unwrap();
    assert_eq!((out.payload, out.code), (1u32.into(), 0u32.into()));
    let illegal = aliased
        .replace("regs { fr:reg[4],", "regs { base:reg[32], fr:reg[4],")
        .replace(
            "lift.oracle_call @plus_db permit",
            "base = lift.addr @view\nlift.oracle_call @plus_db permit",
        );
    assert!(Object::parse(&illegal)
        .unwrap_err()
        .contains("unknown global or mapped oracle region"));
}

fn through(object: &Object) -> Result<(BigUint, BigUint), EvalError> {
    let out = eval_oracle_bounded(
        object,
        object.symbol("through").unwrap(),
        &[3u32.into()],
        &[vec![255u32.into()]],
        &[
            Classical::Token([6u32.into(), 0u32.into(), 0u32.into(), 0u32.into()]),
            Classical::Bits(2u32.into()),
        ],
        &[8u32.into(), 0u32.into(), 8u32.into(), 1u32.into()],
        0,
        100,
    )?;
    Ok((out.payload, out.code))
}

#[test]
fn every_lproc_signature_field_and_position_rejects_bad_types_and_removals() {
    use bennett_cert_checker::json::Value;
    fn paths(value: &Value, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        match value {
            Value::Obj(fields) => {
                for (key, value) in fields {
                    prefix.push(key.clone());
                    out.push(prefix.clone());
                    paths(value, prefix, out);
                    prefix.pop();
                }
            }
            Value::Arr(items) => {
                for (index, value) in items.iter().enumerate() {
                    prefix.push(index.to_string());
                    out.push(prefix.clone());
                    paths(value, prefix, out);
                    prefix.pop();
                }
            }
            _ => {}
        }
    }
    fn change(value: &mut Value, path: &[String], remove: bool) {
        let key = &path[0];
        if path.len() > 1 {
            let next = match value {
                Value::Obj(fields) => fields.get_mut(key).unwrap(),
                Value::Arr(items) => &mut items[key.parse::<usize>().unwrap()],
                _ => panic!(),
            };
            change(next, &path[1..], remove);
        } else {
            match value {
                Value::Obj(fields) => {
                    if remove {
                        fields.remove(key);
                    } else {
                        fields.insert(key.clone(), Value::Null);
                    }
                }
                Value::Arr(items) => {
                    let index = key.parse::<usize>().unwrap();
                    if remove {
                        items.remove(index);
                    } else {
                        items[index] = Value::Null;
                    }
                }
                _ => panic!(),
            }
        }
    }
    let source = include_str!("fixtures/replay-differential/lproc-through-09.bisa");
    let (prefix, rest) = source.split_once("section symbols\n").unwrap();
    let (symbols, suffix) = rest.split_once("\n\nsection functions\n").unwrap();
    let original = common::json(symbols);
    let signature = original.get("1").unwrap().get("signature").unwrap();
    let mut all = vec![];
    paths(signature, &mut vec![], &mut all);
    assert_eq!(all.len(), 15);
    let mut rejected = 0;
    for remove in [false, true] {
        for path in &all {
            let mut symbols = original.clone();
            let Value::Obj(fields) = &mut symbols else {
                panic!()
            };
            let Value::Obj(worker) = fields.get_mut("1").unwrap() else {
                panic!()
            };
            change(worker.get_mut("signature").unwrap(), path, remove);
            let make = |symbols: &Value| {
                format!(
                    "{prefix}section symbols\n{}\n\nsection functions\n{suffix}",
                    common::encode(symbols)
                )
            };
            let object = match Object::parse(&make(&symbols)) {
                Ok(object) => object,
                Err(error) => {
                    assert!(error.starts_with('§'), "path={path:?}: {error}");
                    let mut input = common::controlled();
                    input.object_text = make(&symbols);
                    assert!(
                        common::codes(&bennett_cert_checker::cert::check_certificate(&input))
                            .contains(&"E_CERT_OBJECT")
                    );
                    rejected += 1;
                    continue;
                }
            };
            for (i, symbol) in object.symbols.iter().enumerate() {
                let Value::Obj(fields) = &mut symbols else {
                    panic!()
                };
                common::set(
                    fields.get_mut(&i.to_string()).unwrap(),
                    "interface_hash",
                    Value::Str(symbol.interface_hash.clone()),
                );
            }
            let object = Object::parse(&make(&symbols)).unwrap();
            assert!(object.stated_hashes_ok());
            assert!(
                matches!(through(&object),Err(EvalError::Reject(r))if r.code=="E_CERT_OBJECT"),
                "path={path:?}, remove={remove}"
            );
            rejected += 1;
        }
    }
    assert_eq!(rejected, 30);
}

#[test]
fn malformed_fixed_arguments_and_role_suffixes_reject_with_object_errors() {
    let source = include_str!("fixtures/replay-differential/lproc-through-09.bisa");
    let variants = [
        ("(x) (tk) (a) (db)", "(x)"),
        ("(x) (tk) (a) (db)", "(x) () (a) (db)"),
        ("(x) (tk) (a) (db)", "(x) (x) (a) (db)"),
        ("(x) (tk) (a) (db)", "(x) (a) (a) (db)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (r) (db)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (x) (db)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (1) (db)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (tk) (db)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (a) (t)"),
        ("(x) (tk) (a) (db)", "(x) (tk) (a) ()"),
        ("(tk:token token)", "(tk:token classical)"),
        ("(a:i2 classical)", "(a:i2 token)"),
        (
            "(tk:token token) (a:i2 classical) (db:region[0,8] in)",
            "(tk:token token) (a:i2 classical) (db:region[0,8] mut)",
        ),
        ("regs { fr:reg[4],", "regs { a:reg[2], fr:reg[4],"),
        ("lift.ret r", "a = lift.const.i2 0\nlift.ret r"),
        ("lift.ret r", "r = lift.trunc.i2 tk\nlift.ret r"),
        ("lift.ret r", "r = lift.mov db\nlift.ret r"),
    ];
    for (before, after) in variants {
        assert!(source.contains(before));
        let object = match Object::parse(&source.replacen(before, after, 1)) {
            Ok(object) => object,
            Err(error) => {
                assert!(error.starts_with('§'), "{before} => {after}: {error}");
                continue;
            }
        };
        assert!(
            matches!(through(&object),Err(EvalError::Reject(r))if r.code=="E_CERT_OBJECT"),
            "{before} => {after}"
        );
    }
    assert_eq!(variants.len(), 17);
}

#[test]
fn root_arguments_cannot_alias_writable_locations_and_outputs_are_not_pseudo_locations() {
    let source = include_str!("fixtures/replay-differential/lproc-through-09.bisa");
    for changed in [
        source
            .replacen("regs { r:reg[2] }", "regs { r:reg[2], a:reg[2] }", 1)
            .replacen("r = lift.call", "a = lift.const.i2 0\nr = lift.call", 1),
        source.replacen("r = lift.call", "r = lift.mov t\nr = lift.call", 1),
    ] {
        let object = match Object::parse(&changed) {
            Ok(object) => object,
            Err(error) => {
                assert!(error.starts_with('§'), "{error}");
                continue;
            }
        };
        assert!(matches!(through(&object),Err(EvalError::Reject(r))if r.code=="E_CERT_OBJECT"));
    }
}

#[test]
fn finite_binding_closure_beyond_host_cap_is_unchecked_even_for_zero_row_budget() {
    // Adjacent swaps generate all 7! origin keys, despite equal entry values.
    let params = (0..7)
        .map(|i| format!("a{i}:i1"))
        .collect::<Vec<_>>()
        .join(", ");
    let names = (0..7).map(|i| format!("a{i}")).collect::<Vec<_>>();
    let mut calls = Vec::new();
    for i in 0..6 {
        let mut swapped = names.clone();
        swapped.swap(i, i + 1);
        calls.push(format!(
            "r = lift.call @f () () ({}) ()",
            swapped.join(", ")
        ));
    }
    let types = r#"["i1","i1","i1","i1","i1","i1","i1"]"#;
    let signatures = format!(
        r#"{{"o":{{"abi":"0.9","args":{types},"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":32,"tier":"q"}},"f":{{"abi":"0.9","args":{types},"kind":"lproc","mem":[],"params":[],"result":32,"tier":"q","tokens":[]}}}}"#
    );
    let object=program(&signatures,&format!("oracle @o () (t:wire[32]) (fault:wire[4]) () ({params})\nregs {{ r:reg[32] }}\n^entry():\nr = lift.call @f () () ({}) ()\nlift.retval r\nlproc @f () (r:reg[32]) () ({params}) ()\n^entry():\n{}\nlift.ret r",names.join(", "),calls.join("\n")));
    let result = eval_oracle_bounded(
        &object,
        object.symbol("o").unwrap(),
        &[],
        &[],
        &vec![Classical::Bits(0u32.into()); 7],
        &[0u32.into(), 0u32.into(), 0u32.into(), 0u32.into()],
        0,
        1,
    );
    assert!(matches!(result,Err(EvalError::Unchecked(s)) if s.contains("4096 binding instances")));
}
