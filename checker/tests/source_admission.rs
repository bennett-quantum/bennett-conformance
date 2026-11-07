//! Whole-object admission: independently bound complete-byte regression inputs.
use crate::{
    common::*,
    lexical_differential::{candidate_inputs, Oracle},
};
use bennett_cert_checker::{
    cert::{check_certificate, Inputs, Limits},
    object::Object,
};
use std::path::Path;

fn saved(name: &str) -> Inputs {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/source-admission");
    Inputs {
        cert_bytes: std::fs::read(root.join(format!("{name}.json"))).unwrap(),
        object_text: std::fs::read_to_string(root.join(format!("{name}.bisa"))).unwrap(),
        target_bytes: None,
        schedule_bytes: None,
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    }
}

fn rejected(name: &str) {
    let input = saved(name);
    assert!(
        !Oracle::new()
            .run("check", &input.object_text)
            .status
            .success(),
        "{name}"
    );
    let result = check_certificate(&input);
    assert_eq!(result.overall, "rejected", "{name}: {result:?}");
    assert!(
        codes(&result).contains(&"E_CERT_OBJECT"),
        "{name}: {result:?}"
    );
    assert!(
        !result
            .regions
            .iter()
            .flat_map(|r| &r.claims)
            .any(|(_, c)| c.outcome == "proof"),
        "{result:?}"
    );
}

macro_rules! rejected_source {
    ($test:ident,$fixture:literal) => {
        #[test]
        fn $test() {
            rejected($fixture);
        }
    };
}
rejected_source!(missing_closing_brace, "missing-close");
rejected_source!(extra_closing_brace, "extra-close");
rejected_source!(trailing_function_header, "header-tail");
rejected_source!(trailing_block_header, "block-tail");
rejected_source!(zero_trip_overlap, "zero-overlap");
rejected_source!(zero_trip_unknown_wire, "zero-undefined");
rejected_source!(zero_trip_wrong_arity, "zero-wrong-arity");
rejected_source!(zero_trip_unknown_opcode, "zero-unknown-opcode");
rejected_source!(unclaimed_unknown_opcode, "unused-unknown-opcode");
rejected_source!(unclaimed_empty_bounds, "unused-empty-bounds");
#[test]
fn canonical_and_zero_trip_controls_remain_verified() {
    for name in ["canonical", "zero-valid"] {
        let input = saved(name);
        assert!(Oracle::new()
            .run("check", &input.object_text)
            .status
            .success());
        let verdict = check_certificate(&input);
        assert_eq!(verdict.overall, "verified", "{verdict:?}");
        assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
    }
}

fn review_source(instruction: &str) -> String {
    let base = saved("canonical");
    let text = base
        .object_text
        .trim_end()
        .replace("c:wire in", "c:wire[1] in")
        + "\naction @unused (w:wire mut) () () () ()\n^entry():\n"
        + instruction
        + "\n";
    add_symbol(
        &text,
        "unused",
        r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"action","mem":[],"mut":[1],"tier":"q"}"#,
    )
}
fn add_symbol(text: &str, name: &str, signature: &str) -> String {
    let (before, rest) = text.split_once("section symbols\n").unwrap();
    let (symbols, after) = rest.split_once("\n\nsection functions\n").unwrap();
    let mut symbols = json(symbols);
    let sig = json(signature);
    let key = symbols.as_obj().unwrap().len().to_string();
    let mut entry = json("{}");
    set(
        &mut entry,
        "name",
        bennett_cert_checker::json::Value::Str(name.into()),
    );
    set(&mut entry, "kind", sig.get("kind").unwrap().clone());
    set(&mut entry, "signature", sig);
    set(
        &mut entry,
        "interface_hash",
        bennett_cert_checker::json::Value::Str("0".repeat(64)),
    );
    set(&mut symbols, &key, entry);
    format!(
        "{before}section symbols\n{}\n\nsection functions\n{after}",
        encode(&symbols)
    )
}
#[test]
fn alphabetic_angle_atoms_do_not_escape_in_unused_functions() {
    let base = saved("canonical");
    let oracle = Oracle::new();
    for angle in ["NaN", "inf", "identifier"] {
        let text = review_source(&format!("rx {angle} w"));
        assert!(!oracle.run("check", &text).status.success());
        let result = check_certificate(&candidate_inputs(&base, text));
        assert_eq!(result.overall, "rejected", "{angle}: {result:?}");
        assert!(codes(&result).contains(&"E_CERT_OBJECT"), "{result:?}");
    }
    let text = oracle.bound_candidate(&review_source("rx 1.0 w"));
    let result = check_certificate(&candidate_inputs(&base, text));
    assert_eq!(result.overall, "verified", "{result:?}");
}
#[test]
fn indexed_and_scalar_controlled_calls_remain_verified() {
    let base = saved("canonical");
    let oracle = Oracle::new();
    for control in ["c", "c[0]"] {
        let text = base
            .object_text
            .trim_end()
            .replace("c:wire in", "c:wire[1] in")
            .replace(
                "cx c t",
                &format!("ctl {control} action_call @flip (t) () () ()"),
            )
            + "\nperm_action @flip (x:wire mut) () () () ()\n^entry():\nx x\n";
        let text = add_symbol(
            &text,
            "flip",
            r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}"#,
        );
        let text = oracle.bound_candidate(&text);
        assert!(oracle.run("check", &text).status.success());
        let input = candidate_inputs(&base, text);
        assert!(
            Object::parse(&input.object_text).is_ok(),
            "indexed ctl rejected"
        );
        let verdict = check_certificate(&input);
        assert_eq!(verdict.overall, "verified", "{control}: {verdict:?}");
        assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
    }
}
