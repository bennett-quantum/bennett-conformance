//! ISA §§3.3–3.4: whole-object checks include unclaimed classical functions.
use crate::{
    common::*,
    lexical_differential::{candidate_inputs, Oracle},
    source_admission::{add_symbol, saved},
};
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    object::Object,
};

fn input(source: &str, signature: &str) -> Inputs {
    let base = saved("canonical");
    let text = format!("{}\n{source}\n", base.object_text.trim_end());
    let text = add_symbol(&text, "host", signature);
    let text = Oracle::new().bound_candidate(&text);
    candidate_inputs(&base, text)
}
fn accepted(source: &str, signature: &str) {
    let input = input(source, signature);
    let reference = Oracle::new().run("check", &input.object_text);
    assert!(
        reference.status.success(),
        "{}",
        String::from_utf8_lossy(&reference.stderr)
    );
    assert!(
        Object::parse(&input.object_text).is_ok(),
        "{}",
        Object::parse(&input.object_text).unwrap_err()
    );
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
}
fn rejected(source: &str, signature: &str) {
    let input = input(source, signature);
    assert!(
        !Oracle::new()
            .run("check", &input.object_text)
            .status
            .success(),
        "{source}"
    );
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "rejected", "{source}: {verdict:?}");
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
    assert!(verdict.regions.is_empty(), "{verdict:?}");
}
const RESULT: &str = r#"{"abi":"0.9","kind":"cc","params":[],"results":["i4"],"tier":"cc"}"#;
const EMPTY: &str = r#"{"abi":"0.9","kind":"cc","params":[],"results":[],"tier":"cc"}"#;
const REGISTER: &str = r#"{"abi":"0.9","kind":"cc","params":["qreg[1]"],"results":[],"tier":"cc"}"#;

#[test]
fn forward_references_follow_definitions_across_textually_later_blocks() {
    accepted("fn cc @host () (r:i4) () () ()\n^entry():\nbr ^make()\n^use():\nr = add late late\nret r\n^make():\nlate = const.i4 1\nbr ^use()",RESULT);
}
#[test]
fn multiple_definitions_of_one_name_share_one_type() {
    let source = "fn cc @host () (r:i4) () () ()\n^entry():\nr = const.i4 1\nr = add r r\nret r";
    let input = input(source, RESULT);
    // §3.4 explicitly allows these definitions; the pinned check still rejects
    // them as reused SSA. Admission retains the source; certification defers.
    assert!(!Oracle::new()
        .run("check", &input.object_text)
        .status
        .success());
    Object::parse(&input.object_text).unwrap();
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(
        verdict
            .unchecked_items
            .iter()
            .any(|r| r.contains("§3.4") && r.contains("multiple definitions")),
        "{verdict:?}"
    );
    assert!(verdict.regions.is_empty());
}

#[test]
fn incompatible_definitions_and_undefined_values_reject_before_proof() {
    for body in [
        "r = const.i4 1\nr = const.i8 2\nret r",
        "r = add missing missing\nret r",
    ] {
        rejected(
            &format!("fn cc @host () (r:i4) () () ()\n^entry():\n{body}"),
            RESULT,
        );
    }
}
#[test]
fn slice_views_reject_parent_and_non_call_uses_on_any_path() {
    for body in [
        "s = qreg.slice q 0 1\nn = qreg.width q\naction_call @a (s) () () ()",
        "s = qreg.slice q 0 1\nn = qreg.width s",
        "s = qreg.slice q 0 1",
        "s = qreg.slice q 0 1\nbr ^next(s)\n^next(other:qreg[1]):\naction_call @a (other) () () ()",
        "s = qreg.slice q 0 1\naction_call @a (s) () () ()\naction_call @a (s) () () ()",
    ] {
        // @a has one mut and one in slot. Supply a separate register, so a
        // malformed view cannot be rejected merely for a call-count mismatch.
        let body = body
            .replace("action_call @a (s) ()", "action_call @a (s) (other)")
            .replace("action_call @a (other) ()", "action_call @a (other) (q)");
        let source = format!(
            "fn cc @host (q:qreg[1]) () () () ()\n^entry():\nother = qreg.new any 1\n{body}\nret"
        );
        rejected(&source, REGISTER);
    }
}
#[test]
fn slice_views_follow_branch_successors_and_release_the_parent_after_one_call() {
    let sig =
        r#"{"abi":"0.9","kind":"cc","params":["qreg[1]","qreg[1]","i1"],"results":[],"tier":"cc"}"#;
    let prefix="fn cc @host (q:qreg[1], c:qreg[1], p:i1) () () () ()\n^entry():\ns = qreg.slice q 0 1\nbr p ^left() ^right()\n^left():\naction_call @a (s) (c) () ()\nbr ^done()\n^right():\n";
    rejected(&(prefix.to_owned() + "br ^done()\n^done():\nret"), sig);
    accepted(
        &(prefix.to_owned()
            + "action_call @a (s) (c) () ()\nbr ^done()\n^done():\nn = qreg.width q\nret"),
        sig,
    );
}
#[test]
fn dynamic_register_transfers_keep_runtime_width_checks() {
    let sig = r#"{"abi":"0.9","kind":"cc","params":["qreg[*]","qreg[1]"],"results":["qreg[1]"],"tier":"cc"}"#;
    accepted("fn cc @host (q:qreg[*], c:qreg[1]) (r:qreg[1]) () () ()\n^entry():\nbr ^next(q)\n^next(s:qreg[1]):\naction_call @a (s) (c) () ()\nret q",sig);
}
#[test]
fn evident_slice_range_failure_is_a_runtime_precondition() {
    accepted("fn cc @host () () () () ()\n^entry():\nq = qreg.new any 1\nc = qreg.new any 1\ns = qreg.slice q 9 1\naction_call @a (s) (c) () ()\nret",EMPTY);
}
#[test]
fn region_calls_still_require_distinct_register_names() {
    rejected(
        "fn cc @host (q:qreg[1]) () () () ()\n^entry():\naction_call @a (q) (q) () ()\nret",
        REGISTER,
    );
}

#[test]
fn register_slot_widths_agree_with_the_pinned_static_and_runtime_boundary() {
    for ty in ["qreg[*]", "qreg[1]", "qreg[2]"] {
        let source=format!("fn cc @host (q:{ty}, c:qreg[1]) () () () ()\n^entry():\naction_call @a (q) (c) () ()\nret");
        let sig = format!(
            r#"{{"abi":"0.9","kind":"cc","params":["{ty}","qreg[1]"],"results":[],"tier":"cc"}}"#
        );
        let input = input(&source, &sig);
        let reference = Oracle::new().run("check", &input.object_text);
        let parsed = Object::parse(&input.object_text);
        assert_eq!(
            parsed.is_ok(),
            reference.status.success(),
            "{ty}: {parsed:?} {}",
            String::from_utf8_lossy(&reference.stderr)
        );
        let verdict = check_certificate(&input);
        assert_eq!(
            verdict.overall,
            if reference.status.success() {
                "verified"
            } else {
                "rejected"
            },
            "{ty}: {verdict:?}"
        );
    }
}
