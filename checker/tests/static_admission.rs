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
    input_with(source, signature, &[])
}
fn input_with(source: &str, signature: &str, companions: &[(&str, &str)]) -> Inputs {
    let base = saved("canonical");
    let text = format!("{}\n{source}\n", base.object_text.trim_end());
    let mut text = add_symbol(&text, "host", signature);
    for (name, sig) in companions {
        text = add_symbol(&text, name, sig);
    }
    if source.contains("lift.call") {
        text.push_str("\nsection layout\n");
        text.push_str(r#"{"A_tab":0,"H":6,"S_cells":4,"globals":{},"heap_base":32,"heap_limit_max":0,"interface_regions":{},"null_size":16,"ptr_width":32}"#);
        text.push('\n');
    }
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
        "s = qreg.slice q 0 1\nbr ^next(s)\n^next(received:qreg[1]):\naction_call @a (received) () () ()",
        "s = qreg.slice q 0 1\naction_call @a (s) () () ()\naction_call @a (s) () () ()",
    ] {
        // @a has one mut and one in slot. Supply a separate register, so a
        // malformed view cannot be rejected merely for a call-count mismatch.
        let body = body
            .replace("action_call @a (s) ()", "action_call @a (s) (other)")
            .replace("action_call @a (received) ()", "action_call @a (other) (q)");
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

const PROCEDURE: &str = r#"{"abi":"0.9","args":[],"kind":"lproc","mem":[],"params":[],"result":1,"tier":"q","tokens":[]}"#;
fn procedure_source(fault_width: u32, cause: &str) -> Inputs {
    let source=format!("oracle @host () (t:wire) (f:wire[{fault_width}]) () ()\nregs {{ r:reg[1] }}\n^entry():\nr = lift.call @worker ()\nlift.retval r\nlproc @worker () (r:reg[1]) () () ()\n^entry():\nfault {cause}");
    let sig = format!(
        r#"{{"abi":"0.9","args":[],"fault":{fault_width},"in":[],"kind":"oracle","mem":[],"tgt":1,"tier":"q"}}"#
    );
    input_with(&source, &sig, &[("worker", PROCEDURE)])
}
#[test]
fn every_lproc_signature_matches_its_unused_source_header() {
    for (source, sig) in [
        (
            "lproc @host () (r:reg[2]) () () ()\n^entry():\nlift.ret r",
            r#"{"abi":"0.9","args":[],"kind":"lproc","mem":[],"params":[],"result":3,"tier":"q","tokens":[]}"#,
        ),
        (
            "lproc @host () (r:reg[1]) () () ()\n^entry():\nlift.ret r",
            r#"{"abi":"0.9","args":[],"kind":"lproc","mem":[],"params":[],"result":1,"tier":"q","tokens":["token"]}"#,
        ),
    ] {
        rejected(source, sig);
    }
}
#[test]
fn transitive_lproc_fault_literals_fit_every_containing_oracle() {
    let input = procedure_source(4, "16");
    assert!(!Oracle::new()
        .run("check", &input.object_text)
        .status
        .success());
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "rejected", "{verdict:?}");
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
}
#[test]
fn lproc_fault_literals_preserve_wide_containing_oracle_widths() {
    let input = procedure_source(128, "1267650600228229401496703205376"); // 2^100.
    let result = Oracle::new().run("check", &input.object_text);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
    assert_eq!(verdict.regions[0].claims[0].1.outcome, "proof");
}
#[test]
fn indirect_label_source_validation_is_explicitly_unchecked_until_index_spaces_are_instantiated() {
    let source="oracle @host () (t:wire[2]) (f:wire[4]) () ()\nregs { r:reg[2], pc:reg[1] }\n^entry():\npc = lift.label ^done\nqjumpr pc\n^done():\nr = lift.const.i2 0\nlift.retval r";
    let sig =
        r#"{"abi":"0.9","args":[],"fault":4,"in":[],"kind":"oracle","mem":[],"tgt":2,"tier":"q"}"#;
    let input = input(source, sig);
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(
        verdict
            .unchecked_items
            .iter()
            .any(|r| r.contains("§3.6") && r.contains("index")),
        "{verdict:?}"
    );
    assert!(verdict.regions.is_empty());
}

#[test]
fn imported_source_admission_is_deferred_until_all_linked_bodies_are_validated() {
    let source = "fn cc @host () () () () ()\n^entry():\ncall @future\nret";
    let base = saved("canonical");
    let initial = input(source, EMPTY);
    let imports = format!(r#"{{"future":{{"signature":{EMPTY}}}}}"#);
    let text = initial.object_text.replace(
        "\n\nsection functions\n",
        &format!("\n\nsection imports\n{imports}\n\nsection functions\n"),
    );
    let text = Oracle::new().bound_candidate(&text);
    let input = candidate_inputs(&base, text);
    Object::parse(&input.object_text).unwrap();
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "unchecked", "{verdict:?}");
    assert!(
        verdict
            .unchecked_items
            .iter()
            .any(|r| r.contains("imported") && r.contains("§9.2")),
        "{verdict:?}"
    );
    assert!(verdict.regions.is_empty());
}

#[test]
fn unclaimed_classical_signature_abi_tier_and_fields_are_exact() {
    for signature in [
        r#"{"abi":"0.8","kind":"cc","params":[],"results":[],"tier":"cc"}"#,
        r#"{"abi":"0.9","kind":"cc","params":[],"results":[],"tier":"r"}"#,
        r#"{"abi":"0.9","extra":0,"kind":"cc","params":[],"results":[],"tier":"cc"}"#,
    ] {
        rejected("fn cc @host () () () () ()\n^entry():\nret", signature);
    }
}

#[test]
fn explicit_fault_ends_the_run_without_returning_a_pending_view() {
    accepted(
        "fn cc @host () () () () ()\n^entry():\nq = qreg.new any 1\ns = qreg.slice q 0 1\nfault 0",
        EMPTY,
    );
}
#[test]
fn trap_does_not_hide_an_unconsumed_slice_view() {
    rejected(
        "fn cc @host () () () () ()\n^entry():\nq = qreg.new any 1\ns = qreg.slice q 0 1\ntrap",
        EMPTY,
    );
}

#[test]
fn register_controls_and_all_region_slots_follow_static_disjointness() {
    for (ty, target, input_name) in [
        ("qreg[2]", "t", "c"),
        ("qreg[*]", "t", "c"),
        ("qreg[1]", "t", "c"),
        ("qreg[1]", "q", "c"),
        ("qreg[1]", "t", "q"),
    ] {
        let source=format!("fn cc @host (q:{ty}, t:qreg[1], c:qreg[1]) () () () ()\n^entry():\nctl q action_call @a ({target}) ({input_name}) () ()\nret");
        let sig = format!(
            r#"{{"abi":"0.9","kind":"cc","params":["{ty}","qreg[1]","qreg[1]"],"results":[],"tier":"cc"}}"#
        );
        let input = input(&source, &sig);
        let oracle = Oracle::new().run("check", &input.object_text);
        let parsed = Object::parse(&input.object_text);
        assert_eq!(
            parsed.is_ok(),
            oracle.status.success(),
            "{ty}/{target}/{input_name}: {parsed:?} {}",
            String::from_utf8_lossy(&oracle.stderr)
        );
        let verdict = check_certificate(&input);
        assert_eq!(
            verdict.overall,
            if oracle.status.success() {
                "verified"
            } else {
                "rejected"
            },
            "{verdict:?}"
        );
    }
}
#[test]
fn one_register_cannot_fill_a_wire_and_a_memory_slot() {
    let source="fn cc @host (q:qreg[1]) () () () ()\n^entry():\naction_call @reader (q) () (q) ()\nret\nperm_action @reader (t:wire mut) () (m:region[0,1] in) () ()\n^entry():\nx t";
    let sig = r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[{"A":0,"W":1,"role":"in"}],"mut":[1],"tier":"q"}"#;
    let input = input_with(source, REGISTER, &[("reader", sig)]);
    assert!(!Oracle::new()
        .run("check", &input.object_text)
        .status
        .success());
    let verdict = check_certificate(&input);
    assert_eq!(verdict.overall, "rejected", "{verdict:?}");
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"));
}

#[test]
fn controlled_prefixes_require_region_calls() {
    rejected(
        "action @host (t:wire mut) (c:wire in) () () ()\n^entry():\nctl c x t",
        r#"{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"action","mem":[],"mut":[1],"tier":"q"}"#,
    );
}

#[test]
fn operand_lists_consume_only_the_published_name_production() {
    for body in ["mcx (c[0]) t", "ctl c action_call @a (t[0]) (c) () ()"] {
        rejected(
            &format!("action @host (t:wire mut) (c:wire in) () () ()\n^entry():\n{body}"),
            r#"{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"action","mem":[],"mut":[1],"tier":"q"}"#,
        );
    }
}
