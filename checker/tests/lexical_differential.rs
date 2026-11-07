//! Generated ISA §16.1/§16.2 admission differential against the pinned binary.
use crate::common::*;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::{jcs, Value},
    object::Object,
};
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub(super) struct Oracle {
    dir: PathBuf,
    binary: PathBuf,
}
impl Oracle {
    pub(super) fn new() -> Self {
        let pin = pins::isa_spec_dir();
        let binary = pin.join("reference-interpreter/target/release/bennett-isa-ref");
        assert!(
            binary.is_file(),
            "build the pinned release interpreter: {}",
            binary.display()
        );
        if let Some(configured) = std::env::var_os("BENNETT_ISA_BINARY") {
            assert_eq!(
                std::fs::canonicalize(configured).unwrap(),
                std::fs::canonicalize(&binary).unwrap(),
                "BENNETT_ISA_BINARY must be the gate-pinned interpreter"
            );
        }
        let dir = std::env::temp_dir().join(format!(
            "checker-lexical-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        Self { dir, binary }
    }
    pub(super) fn run(&self, command: &str, text: &str) -> Output {
        let path = self.dir.join("candidate.bisa");
        std::fs::write(&path, text).unwrap();
        let out = Command::new("timeout")
            .arg("5")
            .arg(&self.binary)
            .arg(command)
            .arg(path)
            .output()
            .unwrap();
        assert!(
            matches!(out.status.code(), Some(0 | 1)),
            "oracle failed: {out:?}"
        );
        out
    }
    pub(super) fn bound_candidate(&self, text: &str) -> String {
        // The oracle's hash command ignores stale stated hashes. This keeps
        // accepted whitespace/value changes from being rejected only for §9.2.
        let hashed = self.run("hash", text);
        if !hashed.status.success() {
            return text.into();
        }
        let hashes = json(std::str::from_utf8(&hashed.stdout).unwrap());
        let (prefix, rest) = text.split_once("section symbols\n").unwrap();
        let (symbols, suffix) = rest.split_once("\n\nsection functions\n").unwrap();
        let mut symbols = json(symbols);
        let Value::Obj(entries) = &mut symbols else {
            panic!()
        };
        for entry in entries.values_mut() {
            let name = entry.get("name").unwrap().as_str().unwrap();
            let hash = hashes
                .get("symbols")
                .unwrap()
                .get(name)
                .unwrap()
                .get("interface_hash")
                .unwrap()
                .clone();
            set(entry, "interface_hash", hash);
        }
        format!(
            "{prefix}section symbols\n{}\n\nsection functions\n{suffix}",
            encode(&symbols)
        )
    }
}
impl Drop for Oracle {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("remove lexical oracle inputs");
    }
}

fn identity_claim() -> Value {
    let mut rows = vec![];
    for t in 0..2 {
        for c in 0..2 {
            let tuple = json(&format!(
                r#"{{"bundles":["{t}","{c}"],"borrowed":[],"regions":[]}}"#
            ));
            let mut row = json("{}");
            set(&mut row, "before", tuple.clone());
            set(&mut row, "after", tuple);
            rows.push(row);
        }
    }
    let mut claim = json(
        r#"{"kind":"exhaustive","label":"proof","invocation":{"args":[],"budget":["0","0","0","0"]},"domain":{"bundles":[{"name":"t","width":1,"lo":0,"hi":1},{"name":"c","width":1,"lo":0,"hi":1}],"borrowed":[],"regions":[]},"expected":{}}"#,
    );
    set(&mut claim, "expected", {
        let mut e = json("{}");
        set(&mut e, "table", Value::Arr(rows));
        e
    });
    claim
}

fn seed(extra: bool) -> Inputs {
    let root = "perm_action @a (t:wire[1] mut) (c:wire[1] in) () () ()\nregs {  } bounds {2}\n^entry():\nqloop 2 {\ncx c[0] t[0]\n}";
    let mut functions =
        root.to_string() + "\nperm_action @flip (x:wire mut) () () () ()\n^entry():\nx x";
    let mut sigs = json(
        r#"{"a":{"abi":"0.9","args":[],"bounds":[2],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#,
    );
    set(
        &mut sigs,
        "flip",
        json(
            r#"{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}"#,
        ),
    );
    if extra {
        functions.push_str("\naction @types (w:wire[2] mut) () () (n:i4, p:ptr, f:f64, tk:token) ()\nregs {  } bounds {2}\n^entry():\nd = qalloc any 2\nb = qborrow any 2\nqregion local 1 2\nqloop 2 {\nx d[1]\n}\nqreturn b\nqfree d\nqif w[0] {\nx w[1]\n} {\nx w[1]\n}\nrx 1.0 w[0]\noracle @o (x:wire[4] in) (t:wire[4]) (f:wire[4]) (db:region[0,8] in) ()\nregs { r:reg[4], s:reg[4], zero:reg[4] }\n^entry():\nr = lift.const.i4 -1\ns = lift.const.i4 1\nzero = lift.const.i4 0\nlift.retval r\nfn cc @host (q:qreg[2], dyn:qreg[*], n:i4, p:ptr, f:f64, tk:token) () () () ()\n^entry():\nbr ^next(n)\n^next(value:i4):\nret");
        functions.push_str("\naction @dead (w:wire[2] mut) (c:wire in) () () ()\nregs {  } bounds {0}\n^entry():\nqloop 0 {\ncx c w[0]\n}\nfn cc @counts () () () () ()\n^entry():\nq = qreg.new any 2\ns = qreg.slice q 0 1\nn = const.i64 2\nfault 0");
        set(
            &mut sigs,
            "dead",
            json(
                r#"{"abi":"0.9","args":[],"bounds":[0],"in":[1],"kind":"action","mem":[],"mut":[2],"tier":"q"}"#,
            ),
        );
        set(
            &mut sigs,
            "counts",
            json(r#"{"abi":"0.9","kind":"cc","params":[],"results":[],"tier":"cc"}"#),
        );
        set(
            &mut sigs,
            "types",
            json(
                r#"{"abi":"0.9","args":["i4","ptr","f64","token"],"bounds":[2],"in":[],"kind":"action","mem":[],"mut":[2],"tier":"q"}"#,
            ),
        );
        set(
            &mut sigs,
            "o",
            json(
                r#"{"abi":"0.9","args":[],"fault":4,"in":[4],"kind":"oracle","mem":[{"A":0,"W":8,"role":"in"}],"tgt":4,"tier":"q"}"#,
            ),
        );
        set(
            &mut sigs,
            "host",
            json(
                r#"{"abi":"0.9","kind":"cc","params":["qreg[2]","qreg[*]","i4","ptr","f64","token"],"results":[],"tier":"cc"}"#,
            ),
        );
    }
    let inputs = standalone(&functions, &encode(&sigs), "a", vec![identity_claim()]).0;
    if extra {
        let text = format!(
            "{}\nsection layout\n{}\n",
            inputs.object_text,
            encode(&json(
                r#"{"A_tab":2,"H":6,"S_cells":0,"globals":{},"heap_base":16,"heap_limit_max":32,"interface_regions":{"o":{"db":{"base":4}}},"null_size":4,"ptr_width":32}"#
            ))
        );
        candidate_inputs(&inputs, text)
    } else {
        inputs
    }
}

pub(super) fn candidate_inputs(base: &Inputs, text: String) -> Inputs {
    let mut input = base.clone();
    input.object_text = text;
    let Ok(_) = Object::parse(&input.object_text) else {
        return input;
    };
    input.object_text = rebind_object_symbols(&input.object_text);
    let object = Object::parse(&input.object_text).unwrap();
    let root = object.symbol("a").unwrap();
    let hash = root.closure_hash.as_ref().unwrap();
    let mut cert = json(std::str::from_utf8(&input.cert_bytes).unwrap());
    let old = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    set(
        &mut cert,
        "bindings",
        json(&format!(
            r#"{{"closure_hash":"{hash}","interface_hash":"{}","object_hash":"{}","reference_program_hash":"{}"}}"#,
            root.interface_hash, object.object_hash, root.interface_hash
        )),
    );
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    let mut region = regions.remove(&old).unwrap();
    let Value::Obj(r) = &mut region else { panic!() };
    let Value::Arr(claims) = r.get_mut("claims").unwrap() else {
        panic!()
    };
    set(
        &mut claims[0],
        "subject",
        json(&format!(
            r#"{{"closure_hash":"{hash}","inherited_controls":0,"variant":"forward"}}"#
        )),
    );
    regions.insert(hash.clone(), region);
    input.cert_bytes = jcs(&cert).unwrap();
    input
}

fn witness_verdict(from: &str, to: &str) -> bennett_cert_checker::cert::Verdict {
    let oracle = Oracle::new();
    let base = seed(false);
    assert!(oracle.run("check", &base.object_text).status.success());
    assert_eq!(check_certificate(&base).overall, "verified");
    let text = oracle.bound_candidate(&base.object_text.replacen(from, to, 1));
    assert!(!oracle.run("check", &text).status.success(), "{to}");
    check_certificate(&candidate_inputs(&base, text))
}

#[test]
fn noncanonical_integer_types_cannot_receive_identity_proofs() {
    let verdict = witness_verdict("regs {  }", "regs { unused:i04 }");
    assert_eq!(verdict.overall, "rejected", "{verdict:?}");
    assert!(
        verdict
            .errors
            .iter()
            .any(|e| e.code == "E_CERT_OBJECT" && e.section == "§16.1"),
        "{verdict:?}"
    );
}

#[test]
fn noncanonical_bounds_cannot_receive_identity_proofs() {
    let verdict = witness_verdict("bounds {2}", "bounds { 2 }");
    assert_eq!(verdict.overall, "rejected", "{verdict:?}");
    assert!(
        verdict
            .errors
            .iter()
            .any(|e| e.code == "E_CERT_OBJECT" && e.section == "§16.1"),
        "{verdict:?}"
    );
}

#[test]
fn generated_lexical_admission_matches_the_pinned_interpreter() {
    let oracle = Oracle::new();
    let base = seed(true);
    let checked = oracle.run("check", &base.object_text);
    assert!(
        checked.status.success(),
        "canonical seed: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(check_certificate(&base).overall, "verified");
    let (numeric, punctuation) = sites(&base.object_text);
    let numeric_sites = numeric.len();
    let punctuation_sites = punctuation.len();
    let mut failures = vec![];
    let mut count = 0;
    let mut accepted = 0;
    let mut covered = std::collections::BTreeSet::new();
    for (range, role) in numeric {
        covered.insert(role);
        let atom = &base.object_text[range.clone()];
        for replacement in numeric_spellings(atom) {
            compare_candidate(
                &oracle,
                &base,
                range.clone(),
                &replacement,
                &mut failures,
                &mut count,
                &mut accepted,
            );
        }
    }
    for range in opcode_sites(&base.object_text) {
        for replacement in [
            "NaN",
            "inf",
            "identifier",
            "frobnicate",
            "x",
            "cx",
            "ctl",
            "lift.const.i4",
        ] {
            compare_candidate(
                &oracle,
                &base,
                range.clone(),
                replacement,
                &mut failures,
                &mut count,
                &mut accepted,
            );
        }
    }
    for (range, replacement) in structural_candidates(&base.object_text) {
        compare_candidate(
            &oracle,
            &base,
            range,
            &replacement,
            &mut failures,
            &mut count,
            &mut accepted,
        );
    }
    let dead = base.object_text.rfind("cx c w[0]").unwrap();
    for row in [
        "cx missing w[0]",
        "cx c",
        "cx c c",
        "cx c w[2]",
        "x w",
        "rx NaN w[0]",
        "x c",
        "qfree w",
        "d = qalloc any 0",
        "cx c w[0] ignored",
        "d = qalloc any 1\nqfree d\nx d",
        "qpred c\nx c\nqunpred c",
        "qxor c w",
        "mcx (c, c) w[0]",
        "frobnicate",
        "qloop NaN {\nx w[0]\n}",
    ] {
        compare_candidate(
            &oracle,
            &base,
            dead..dead + "cx c w[0]".len(),
            row,
            &mut failures,
            &mut count,
            &mut accepted,
        );
    }
    assert_eq!(
        covered,
        std::collections::BTreeSet::from([
            NumericRole::Version,
            NumericRole::Width,
            NumericRole::Bound,
            NumericRole::Literal,
            NumericRole::Index,
            NumericRole::Count,
            NumericRole::Region,
            NumericRole::Angle,
        ])
    );
    for range in punctuation {
        for side in [false, true] {
            for gap in ["", " ", "  ", "\t", "\n"] {
                let replacement = if side {
                    format!("{}{gap}", &base.object_text[range.clone()])
                } else {
                    format!("{gap}{}", &base.object_text[range.clone()])
                };
                let mut changed_range = range.clone();
                if side {
                    while base.object_text.as_bytes().get(changed_range.end) == Some(&b' ') {
                        changed_range.end += 1;
                    }
                } else {
                    while changed_range.start > 0
                        && base.object_text.as_bytes()[changed_range.start - 1] == b' '
                    {
                        changed_range.start -= 1;
                    }
                }
                compare_candidate(
                    &oracle,
                    &base,
                    changed_range,
                    &replacement,
                    &mut failures,
                    &mut count,
                    &mut accepted,
                );
            }
        }
    }
    eprintln!("lexical differential: {numeric_sites} numeric sites; {punctuation_sites} punctuation sites; {count} generated candidates; {accepted} accepted by oracle; {} disagreements", failures.len());
    assert!(
        failures.is_empty(),
        "{} disagreements:\n{}",
        failures.len(),
        failures
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NumericRole {
    Version,
    Width,
    Bound,
    Literal,
    Index,
    Count,
    Region,
    Angle,
}

fn sites(
    text: &str,
) -> (
    Vec<(std::ops::Range<usize>, NumericRole)>,
    Vec<std::ops::Range<usize>>,
) {
    let mut numbers = vec![];
    let mut punctuation = vec![];
    let mut offset = 0;
    let mut section = "";
    for line in text.split_inclusive('\n') {
        if let Some(name) = line.strip_prefix("section ") {
            section = name.trim_end();
        }
        let source = section == "functions" || line.starts_with("bennett-object ");
        if source {
            let bytes = line.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if b"()[]{},:=@^.*".contains(&bytes[i]) {
                    punctuation.push(offset + i..offset + i + 1);
                }
                if !bytes[i].is_ascii_digit() {
                    i += 1;
                    continue;
                }
                if i > 0 && bytes[i - 1] == b'f' {
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    continue;
                }
                let mut end = i + 1;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                let start = if i > 0 && bytes[i - 1] == b'-' {
                    i - 1
                } else {
                    i
                };
                let role = if line.starts_with("bennett-object ") {
                    if line[..i].contains("ptr_width=") {
                        NumericRole::Width
                    } else {
                        NumericRole::Version
                    }
                } else if i > 0 && bytes[i - 1] == b'i' {
                    NumericRole::Width
                } else if line.starts_with("rx ") {
                    // FLOAT is one atom, not two INTs.
                    while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
                        end += 1;
                    }
                    // The fractional dot is also a punctuation site.
                    for j in i..end {
                        if bytes[j] == b'.' {
                            punctuation.push(offset + j..offset + j + 1);
                        }
                    }
                    NumericRole::Angle
                } else if line.starts_with("regs ") && line[..i].contains("bounds") {
                    NumericRole::Bound
                } else if line.contains("qloop ")
                    || line.contains("qalloc ")
                    || line.contains("qborrow ")
                {
                    NumericRole::Count
                } else if line.starts_with("qregion ") {
                    NumericRole::Region
                } else if i > 0 && matches!(bytes[i - 1], b'[' | b',') {
                    if line.starts_with("regs ") || line.contains(" @") {
                        NumericRole::Width
                    } else {
                        NumericRole::Index
                    }
                } else {
                    NumericRole::Literal
                };
                numbers.push((offset + start..offset + end, role));
                i = end;
            }
        }
        offset += line.len();
    }
    (numbers, punctuation)
}

fn numeric_spellings(atom: &str) -> Vec<String> {
    let digits = atom.strip_prefix('-').unwrap_or(atom);
    let leading = if atom.starts_with('-') {
        format!("-0{digits}")
    } else {
        format!("0{digits}")
    };
    vec![
        leading,
        "NaN".into(),
        "inf".into(),
        "identifier".into(),
        format!("+{atom}"),
        format!("-{digits}"),
        format!("0x{digits}"),
        format!("{digits}_0"),
        format!("{atom}.0"),
        format!(" {atom}"),
        format!("{atom} "),
        format!("{}\t{}", &atom[..1], &atom[1..]),
        digits.split('.').next().unwrap().into(),
    ]
}

#[allow(clippy::too_many_arguments)]
fn compare_candidate(
    oracle: &Oracle,
    base: &Inputs,
    range: std::ops::Range<usize>,
    replacement: &str,
    failures: &mut Vec<String>,
    count: &mut usize,
    accepted: &mut usize,
) {
    let mut text = base.object_text.clone();
    text.replace_range(range.clone(), replacement);
    if text == base.object_text {
        return;
    }
    *count += 1;
    let text = oracle.bound_candidate(&text);
    let reference = oracle.run("check", &text);
    let parsed = Object::parse(&text);
    let admits = reference.status.success();
    *accepted += usize::from(admits);
    if parsed.is_ok() != admits {
        failures.push(format!(
            "at {range:?} replacement {replacement:?}: checker={:?}, reference={}",
            parsed.as_ref().map(|_| "accept"),
            String::from_utf8_lossy(&reference.stderr)
        ));
    }
    if !admits {
        let verdict = check_certificate(&candidate_inputs(base, text));
        if verdict.overall == "verified"
            || verdict
                .regions
                .iter()
                .flat_map(|r| &r.claims)
                .any(|(_, c)| c.outcome == "proof")
        {
            failures.push(format!(
                "oracle-rejected mutation granted proof: at {range:?} {replacement:?}: {verdict:?}"
            ));
        }
    }
}

// Every instruction site, including prefixes and rows in unclaimed functions.
fn opcode_sites(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = vec![];
    let mut offset = 0;
    let mut functions = false;
    for line in text.split_inclusive('\n') {
        if line.starts_with("section ") {
            functions = line == "section functions\n";
        }
        if functions
            && ![
                "section ",
                "regs ",
                "^",
                "}",
                "fn ",
                "action ",
                "perm_action ",
                "oracle ",
                "lproc ",
            ]
            .iter()
            .any(|p| line.starts_with(p))
            && !line.trim().is_empty()
        {
            let mut start = line.find(" = ").map_or(0, |i| i + 3);
            loop {
                let tail = &line[start..];
                if tail.starts_with("adj ") {
                    out.push(offset + start..offset + start + 3);
                    start += 4;
                } else if tail.starts_with("ctl ") {
                    out.push(offset + start..offset + start + 3);
                    start += 4 + tail[4..].find(' ').unwrap() + 1;
                } else {
                    break;
                }
            }
            let end = start
                + line[start..]
                    .find([' ', '\n'])
                    .unwrap_or(line.len() - start);
            out.push(offset + start..offset + end);
        }
        offset += line.len();
    }
    out
}
fn structural_candidates(text: &str) -> Vec<(std::ops::Range<usize>, String)> {
    let mut out = vec![];
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line == "}\n" {
            out.push((offset..offset + line.len(), String::new()));
            out.push((offset..offset + line.len(), "}\n}\n".into()));
        }
        if ["action ", "perm_action ", "oracle ", "lproc ", "fn ", "^"]
            .iter()
            .any(|p| line.starts_with(p))
        {
            out.push((
                offset + line.len() - 1..offset + line.len() - 1,
                " trailing".into(),
            ));
        }
        if line.starts_with("regs ") && line.contains("bounds {") {
            let start = offset + line.find("bounds {").unwrap() + 8;
            out.push((start..offset + line.len() - 2, String::new()));
        }
        offset += line.len();
    }
    // Truncations at each instruction boundary exercise nonempty bodies and EOF.
    for range in opcode_sites(text) {
        out.push((range.start..text.len(), String::new()));
    }
    out
}
