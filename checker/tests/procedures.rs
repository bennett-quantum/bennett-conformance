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
fn unimplemented_fixed_binding_instances_supply_no_reference_result() {
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
    assert!(matches!(result,Err(EvalError::Unchecked(s))if s.contains("fixed-binding")));
}
