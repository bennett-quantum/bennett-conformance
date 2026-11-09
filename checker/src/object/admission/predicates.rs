//! Check the source validator against an independent literal stack machine.
use super::*;

fn source(body: &str) -> String {
    let seed = include_str!("../../../tests/fixtures/source-admission/canonical.bisa");
    seed.replace("cx c t", body)
}
#[test]
fn repeated_stack_effects_match_literal_push_pop_execution() {
    for length in 1..=5 {
        for encoding in 0usize..4usize.pow(length) {
            let mut code = encoding;
            let mut stack = vec![];
            let mut valid = true;
            let mut lines = vec![];
            for _ in 0..length {
                let op = code % 4;
                code /= 4;
                let wire = if op % 2 == 0 { "c" } else { "t" };
                if op < 2 {
                    stack.push(wire);
                    lines.push(format!("qpred {wire}"));
                } else {
                    valid &= stack.pop() == Some(wire);
                    lines.push(format!("qunpred {wire}"));
                }
            }
            valid &= stack.is_empty();
            assert_eq!(
                Object::parse(&source(&lines.join("\n"))).is_ok(),
                valid,
                "{length}/{encoding}"
            );
        }
    }
}
#[test]
fn huge_balanced_loop_counts_do_not_unroll_source_predicates() {
    // Balanced loop bodies need a bounded number of visits, regardless of the source count.
    let count = "1000000000000000000000000000000000000000000000000000000000000";
    let text = source("qpred c\nqpred c\nqunpred c\nqunpred c")
        .replace("qloop 2 {", &format!("qloop {count} {{"))
        .replace("bounds {2}", &format!("bounds {{{count}}}"))
        .replace(r#""bounds":[2]"#, &format!(r#""bounds":["{count}"]"#));
    let object = Object::parse(&text).unwrap();
    assert_eq!(object.function("a").unwrap().bounds[0].to_string(), count);
}

#[test]
fn zero_trip_loop_must_preserve_the_entire_predicate_stack() {
    // ISA §4.6 requires the body to preserve its entry stack (see §3.5),
    // independently of the trip count. Equal depth alone is insufficient.
    for (entry, body, exit) in [
        ("", "qpred c", ""),
        ("qpred c\n", "qunpred c", "\nqunpred c"),
        ("qpred c\n", "qunpred c\nqpred t", "\nqunpred c"),
    ] {
        let text = source(body)
            .trim_end()
            .replace("qloop 2 {", &format!("{entry}qloop 0 {{"))
            .replace("bounds {2}", "bounds {0}")
            .replace(r#""bounds":[2]"#, r#""bounds":[0]"#)
            + exit
            + "\n";
        let error = Object::parse(&text).expect_err("unbalanced zero-trip body admitted");
        assert!(
            error.contains("loop body altered predicate stack"),
            "{error}"
        );
    }
}
