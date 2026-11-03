mod common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::Value,
};
use common::*;

fn table_claim(widths: &[(&str, u32)], rows: Vec<Value>) -> Value {
    let bundles = widths
        .iter()
        .map(|(name, width)| {
            json(&format!(
                r#"{{"name":"{name}","width":{width},"lo":0,"hi":{}}}"#,
                (1u32 << width) - 1
            ))
        })
        .collect();
    let mut domain = json(r#"{"bundles":[],"borrowed":[],"regions":[]}"#);
    set(&mut domain, "bundles", Value::Arr(bundles));
    let mut claim = json(
        r#"{"kind":"exhaustive","label":"proof","invocation":{"args":[],"budget":["0","0","0","0"]},"domain":{},"expected":{"table":[]}}"#,
    );
    set(&mut claim, "domain", domain);
    set(&mut claim, "expected", {
        let mut e = json("{}");
        set(&mut e, "table", Value::Arr(rows));
        e
    });
    claim
}
fn row(before: &[u32], after: &[u32]) -> Value {
    let tuple = |values: &[u32]| {
        let mut t = json(r#"{"bundles":[],"borrowed":[],"regions":[]}"#);
        set(
            &mut t,
            "bundles",
            Value::Arr(values.iter().map(|v| Value::Str(v.to_string())).collect()),
        );
        t
    };
    let mut r = json("{}");
    set(&mut r, "before", tuple(before));
    set(&mut r, "after", tuple(after));
    r
}
fn nested(outer: u32, inner: u32, continuation: bool, wrong: bool) -> Inputs {
    let suffix = if continuation { "\nx t" } else { "" };
    let source=format!("perm_action @a (t:wire mut) (c:wire[{outer}] in, d:wire[{inner}] in) () () ()\nregs {{  }} bounds {{{outer}, {inner}}}\n^entry():\nqloop {outer} while c {{\nqloop {inner} while d {{\nx t\n}}{suffix}\n}}");
    let mut rows = vec![];
    for t in 0..2 {
        for c in 0..1 << outer {
            for d in 0..1 << inner {
                let prefix = |bits: u32, width: u32| {
                    (0..width).take_while(|i| bits & (1 << i) != 0).count() as u32
                };
                let toggle = if wrong {
                    d & 1
                } else {
                    (prefix(c, outer) & 1) * ((prefix(d, inner) & 1) ^ u32::from(continuation))
                };
                rows.push(row(&[t, c, d], &[t ^ toggle, c, d]));
            }
        }
    }
    standalone(&source,&format!(r#"{{"a":{{"abi":"0.9","args":[],"bounds":[{outer},{inner}],"in":[{outer},{inner}],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}}}"#),"a",vec![table_claim(&[("t",1),("c",outer),("d",inner)],rows)]).0
}
#[test]
fn nested_while_flags_preserve_disabled_outer_predicates_and_continuations() {
    for (outer, inner) in [(1, 1), (3, 2)] {
        for continuation in [false, true] {
            let verdict = check_certificate(&nested(outer, inner, continuation, false));
            assert!(
                codes(&verdict).is_empty(),
                "{outer}/{inner}, continuation={continuation}: {verdict:?}"
            );
            let claim = &verdict.regions[0].claims[0].1;
            assert_eq!(claim.outcome, "proof", "{verdict:?}");
            assert_eq!(
                claim.elements.as_deref(),
                Some((1u32 << (1 + outer + inner)).to_string().as_str())
            );
        }
    }
}
#[test]
fn nested_while_cannot_certify_the_reviewed_wrong_inner_only_relation() {
    let verdict = check_certificate(&nested(1, 1, false, true));
    assert!(codes(&verdict).contains(&"E_CERT_REPLAY"), "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "reject");
}

#[test]
fn qif_executes_both_arms_unconditional_lifetimes_and_predicates_only_gates() {
    for (body, mode) in [
        ("qif c {\nd = qalloc any 1\n} {\nqfree d\n}", 0),
        ("qif c {\nd = qalloc any 1\nx t\n} {\nqfree d\n}", 1),
        (
            "qpred e\nqif c {\nd = qalloc any 1\nx t\n} {\nx t\nqfree d\n}\nqunpred e",
            2,
        ),
    ] {
        let mut rows = vec![];
        for t in 0..2 {
            for c in 0..2 {
                for e in 0..2 {
                    let toggle = match mode {
                        0 => 0,
                        1 => c,
                        _ => e,
                    };
                    rows.push(row(&[t, c, e], &[t ^ toggle, c, e]));
                }
            }
        }
        let source =
            format!("action @a (t:wire mut) (c:wire in, e:wire in) () () ()\n^entry():\n{body}");
        let (inputs, _) = standalone(
            &source,
            r#"{"a":{"abi":"0.9","args":[],"bounds":[],"in":[1,1],"kind":"action","mem":[],"mut":[1],"tier":"q"}}"#,
            "a",
            vec![table_claim(&[("t", 1), ("c", 1), ("e", 1)], rows)],
        );
        let verdict = check_certificate(&inputs);
        assert!(codes(&verdict).is_empty(), "mode={mode}: {verdict:?}");
        assert_eq!(
            verdict.regions[0].claims[0].1.outcome, "proof",
            "{verdict:?}"
        );
    }
}
