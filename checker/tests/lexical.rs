//! clause: ISA-16.1, ISA-16.2
use crate::common;
use bennett_cert_checker::object::Object;
use common::*;

fn source() -> String {
    standalone(
        "perm_action @a (t:wire mut) (c:wire in) () () ()\nregs {  } bounds {2}\n^entry():\nqloop 2 {\ncx c t\n}",
        r#"{"a":{"abi":"0.9","args":[],"bounds":[2],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
        "a", vec![],
    ).0.object_text
}

#[test]
fn eq2_noncanonical_body_tokens_cannot_enter_the_checker() {
    let valid = source();
    for (from, to) in [
        ("qloop 2", "qloop 02"),
        ("qloop 2", "qloop +2"),
        ("cx c t", "cx  c t"),
        ("cx c t", "cx c t "),
        ("cx c t", " cx c t"),
        ("cx c t", "cx\tc t"),
        ("cx c t", "cx c t # comment"),
        ("regs {  }", "regs { }"),
        ("wire mut", "wire[01] mut"),
        ("section functions", "section functions "),
    ] {
        let bad = valid.replacen(from, to, 1);
        assert_ne!(bad, valid);
        assert!(Object::parse(&bad).is_err(), "accepted {to:?}");
    }
}

#[test]
fn lexical_exceptions_preserve_empty_regs_empty_data_and_quoted_spaces() {
    let valid = source();
    Object::parse(&valid).unwrap();
    Object::parse(&format!("{valid}\nsection data\nempty \n")).unwrap();
    let quoted = valid.replace("cx c t", "future_op \"a  b\"");
    Object::parse(&quoted).unwrap();
}
