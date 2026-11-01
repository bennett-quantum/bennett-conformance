use std::path::PathBuf;
use std::process::Command;
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../bennett-isa-spec/conformance/fixtures/controlled-action-09")
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
