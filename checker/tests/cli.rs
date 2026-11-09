use std::path::PathBuf;
use std::process::Command;
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/controlled-action-k87")
}
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bennett-check"))
}
#[test]
fn cli_verdict_is_deterministic_and_verifies_the_published_positive() {
    let f = fixture();
    let run = || {
        bin()
            .arg(f.join("certificate.json"))
            .arg("--object")
            .arg(f.join("controlled.bisa"))
            .arg("--target")
            .arg(f.join("target.json"))
            .arg("--schedule")
            .arg(f.join("schedule.json"))
            .output()
            .unwrap()
    };
    let a = run();
    let b = run();
    assert_eq!(a.status.code(), Some(0));
    assert_eq!(a.stdout, b.stdout);
    assert!(a.stderr.is_empty());
    let text = std::str::from_utf8(&a.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    let verdict = bennett_cert_checker::json::parse(text).unwrap();
    assert_eq!(
        verdict.get("overall").and_then(|v| v.as_str()),
        Some("verified")
    );
    assert!(verdict
        .get("unchecked_items")
        .unwrap()
        .as_arr()
        .unwrap()
        .is_empty());
}
#[test]
fn cli_missing_option_value_is_usage_error() {
    let f = fixture();
    for flag in [
        "--object",
        "--target",
        "--schedule",
        "--oracle-bin",
        "--library",
        "--max-elements",
        "--max-wires",
    ] {
        let out = bin()
            .arg(f.join("certificate.json"))
            .arg("--object")
            .arg(f.join("controlled.bisa"))
            .arg(flag)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(3),
            "{flag}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(out.stdout.is_empty(), "{flag}");
    }
}
#[test]
fn cli_embedded_and_detached_certificate_agree() {
    let f = fixture();
    let mut detached = bin();
    detached
        .arg(f.join("certificate.json"))
        .arg("--object")
        .arg(f.join("controlled.bisa"));
    let mut embedded = bin();
    embedded.arg("--from-object").arg(f.join("controlled.bisa"));
    for cmd in [&mut detached, &mut embedded] {
        cmd.arg("--target").arg(f.join("target.json"));
    }
    let a = detached.output().unwrap();
    let b = embedded.output().unwrap();
    assert_eq!(a.status.code(), b.status.code());
    assert_eq!(a.stdout, b.stdout);
}

#[test]
fn unimplemented_optional_comparison_still_reports_unchecked() {
    let f = fixture();
    let out = bin()
        .arg(f.join("certificate.json"))
        .arg("--object")
        .arg(f.join("controlled.bisa"))
        .arg("--target")
        .arg(f.join("target.json"))
        .arg("--oracle-bin")
        .arg("unused-oracle-bin")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v = bennett_cert_checker::json::parse(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    assert_eq!(v.get("overall").and_then(|v| v.as_str()), Some("unchecked"));
}

#[test]
fn cli_help_exits_successfully() {
    for flag in ["--help", "-h"] {
        let out = bin().arg(flag).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{flag}");
        assert!(String::from_utf8(out.stdout)
            .unwrap()
            .contains("certificate checker"));
    }
}

#[test]
fn unknown_verdict_states_fail_loud() {
    let verdict = bennett_cert_checker::cert::Verdict {
        overall: "unknown".into(),
        ..Default::default()
    };
    assert!(std::panic::catch_unwind(|| verdict.exit_code()).is_err());
}

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "checker-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove CLI test inputs");
    }
}

#[test]
fn cli_rejected_io_utf8_and_embedded_errors_have_distinct_exit_contracts() {
    let scratch = Scratch::new();
    let f = fixture();
    let bad_cert = scratch.write("bad.json", b"{}");
    let out = bin()
        .arg(bad_cert)
        .arg("--object")
        .arg(f.join("controlled.bisa"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v = bennett_cert_checker::json::parse(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    assert_eq!(v.get("overall").unwrap().as_str(), Some("rejected"));
    assert!(out.stderr.is_empty());
    for (path, message) in [
        (scratch.0.join("absent.bisa"), "cannot read"),
        (scratch.write("binary.bisa", &[0xff]), "not UTF-8"),
        (
            scratch.write("no-cert.bisa", b"bennett-object\n"),
            "no certificate section",
        ),
    ] {
        let out = bin().arg("--from-object").arg(path).output().unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8(out.stderr).unwrap().contains(message));
    }
}

#[test]
fn cli_fixed_image_slice_reports_exact_evidence_and_checks_producer_label() {
    use crate::{common::*, fixed_images};
    use bennett_cert_checker::{json::Value, object::Object};

    let scratch = Scratch::new();
    for label in ["evidence", "proof", "proof_relative_to_image"] {
        let mut inputs = fixed_images::fixture(1, 2, "09");
        fixed_images::claim_mut(&mut inputs, |claim| {
            set(claim, "label", Value::Str(label.into()));
            let Value::Obj(fields) = claim else { panic!() };
            let domain = fields.get_mut("domain").unwrap();
            let Value::Obj(fields) = domain else { panic!() };
            fields.remove("fixed_regions");
            fields.insert(
                "regions".into(),
                json(r#"[{"name":"r","cells":[{"index":0,"lo":1,"hi":1},{"index":1,"lo":2,"hi":2}]}]"#),
            );
        });
        let hash = Object::parse(&inputs.object_text)
            .unwrap()
            .symbol("a")
            .unwrap()
            .closure_hash
            .clone()
            .unwrap();
        let cert = scratch.write("image.json", &inputs.cert_bytes);
        let obj = scratch.write("image.bisa", inputs.object_text.as_bytes());
        let out = bin().arg(cert).arg("--object").arg(obj).output().unwrap();
        assert!(out.stderr.is_empty());
        let text = std::str::from_utf8(&out.stdout).unwrap();
        assert_eq!(text.lines().count(), 1);
        let report = json(text);
        let claim = &report
            .get("regions")
            .unwrap()
            .get(&hash)
            .unwrap()
            .get("claims")
            .unwrap()
            .as_arr()
            .unwrap()[0];
        // t has four values, c two; the two 2-bit cells are fixed at [1,2].
        // Thus n=8, unrestricted |D|=4*2*4*4=128, n/|D|=1/16.
        assert_eq!(claim.get("elements").unwrap().as_str(), Some("8"));
        assert_eq!(
            claim.get("covered_fraction").unwrap().as_str(),
            Some("1/16")
        );
        if label == "evidence" {
            assert_eq!(claim.get("outcome").unwrap().as_str(), Some("evidence"));
            assert_eq!(out.status.code(), Some(0));
            assert_eq!(report.get("overall").unwrap().as_str(), Some("verified"));
        } else {
            assert_eq!(out.status.code(), Some(1));
            assert_eq!(claim.get("outcome").unwrap().as_str(), Some("reject"));
            assert!(claim
                .get("errors")
                .unwrap()
                .as_arr()
                .unwrap()
                .iter()
                .any(|error| error.get("code").unwrap().as_str() == Some("E_CERT_COVERAGE_LABEL")));
        }
    }
}
