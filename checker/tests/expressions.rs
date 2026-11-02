use bennett_cert_checker::expressions::evaluate;
use bennett_cert_checker::json::parse;
use bennett_cert_checker::rat::{parse_decimal, Rat};
use std::collections::HashMap;
fn value(s: &str) -> bennett_cert_checker::json::Value {
    parse(s).unwrap()
}
#[test]
fn normative_eps_decade_witness_uses_exact_rational_comparisons() {
    let table = value(r#"{"t":[0,1,2,3,4]}"#);
    for (eps, want) in [
        ("1", 0),
        ("0.1", 1),
        ("0.1001", 1),
        ("0.0999", 2),
        ("0.001", 3),
        ("0.000999", 4),
    ] {
        let vars = HashMap::from([("eps", parse_decimal(eps).unwrap())]);
        assert_eq!(
            evaluate(
                &value(r#"{"table":"t","index":"eps_decade"}"#),
                &vars,
                &table
            )
            .unwrap(),
            Rat::from_integer(want.into())
        );
    }
}
#[test]
fn epsilon_domain_and_table_bounds_have_no_fallback() {
    let v = value(r#"{"table":"t","index":"eps_decade"}"#);
    for eps in ["0", "-0.1", "1.0001"] {
        assert!(evaluate(
            &v,
            &HashMap::from([("eps", parse_decimal(eps).unwrap())]),
            &value(r#"{"t":[0,1,2]}"#)
        )
        .is_err());
    }
    assert!(evaluate(&v, &HashMap::new(), &value(r#"{"t":[0]}"#)).is_err());
    let vars = HashMap::from([("eps", parse_decimal("0.0999").unwrap())]);
    assert!(evaluate(&v, &vars, &value(r#"{"t":[0,1]}"#)).is_err());
    assert_eq!(
        evaluate(&v, &vars, &value(r#"{"t":[0,1,2]}"#)).unwrap(),
        Rat::from_integer(2.into())
    );
    for k in ["-1", "0.5", "3", "10000000000000000000000000000000"] {
        assert!(evaluate(
            &value(r#"{"table":"t","index":"k"}"#),
            &HashMap::from([("k", parse_decimal(k).unwrap())]),
            &value(r#"{"t":[0,1,2]}"#)
        )
        .is_err());
    }
}
#[test]
fn logarithm_and_arithmetic_operators_are_exact() {
    for (eps, want) in [
        ("1", 0),
        ("0.5", 1),
        ("0.5001", 1),
        ("0.4999", 2),
        ("0.125", 3),
        ("0.0009765625", 10),
    ] {
        assert_eq!(
            evaluate(
                &value(r#"{"op":"log2inv_eps","args":[]}"#),
                &HashMap::from([("eps", parse_decimal(eps).unwrap())]),
                &value("{}")
            )
            .unwrap(),
            Rat::from_integer(want.into())
        );
    }
    for (expression, want) in [
        (r#"{"op":"ceil","args":[-1.2]}"#, "-1"),
        (r#"{"op":"/","args":[1,3]}"#, "1/3"),
        (r#"{"op":"min","args":[1,2]}"#, "1"),
        (r#"{"op":"max","args":[1,2]}"#, "2"),
        (r#"{"op":"-","args":[1,2]}"#, "-1"),
        (r#"{"op":"+","args":[1,2]}"#, "3"),
        (r#"{"op":"*","args":[1,2]}"#, "2"),
    ] {
        let actual = evaluate(&value(expression), &HashMap::new(), &value("{}")).unwrap();
        let expected = if want == "1/3" {
            Rat::new(1.into(), 3.into())
        } else {
            parse_decimal(want).unwrap()
        };
        assert_eq!(actual, expected, "{expression}");
    }
    for expression in [
        r#"{"op":"/","args":[1,0]}"#,
        r#"{"var":"theta"}"#,
        r#"{"op":"ceil","args":[]}"#,
    ] {
        assert!(evaluate(&value(expression), &HashMap::new(), &value("{}")).is_err());
    }
}
