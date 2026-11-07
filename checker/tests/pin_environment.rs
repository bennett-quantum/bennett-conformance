//! Pin transport must work in an exported checkout with no local gate cache.
use crate::common::pins;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn selected_path_is_the_supplied_pinned_checkout() {
    let selected = pins::isa_spec_dir();
    assert_eq!(
        selected,
        PathBuf::from(std::env::var_os("BENNETT_ISA_SPEC").unwrap())
    );
}

struct PinFixture(PathBuf);
impl PinFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("checker-pin-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("metadata")).unwrap();
        std::fs::write(root.join(".git"), "gitdir: metadata\n").unwrap();
        Self(root)
    }
}
impl Drop for PinFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove synthetic pin metadata");
    }
}

#[test]
fn missing_variable_wrong_revision_and_symbolic_head_fail_loud_in_subprocesses() {
    // Validate the parent pin before testing failures in isolated child processes.
    let supplied = pins::isa_spec_dir();
    assert!(supplied.is_dir());
    let fixture = PinFixture::new();
    for (head, expected) in [
        (None, "BENNETT_ISA_SPEC is required"),
        (
            Some("0000000000000000000000000000000000000000\n"),
            "actual HEAD",
        ),
        (Some("ref: refs/heads/main\n"), "actual HEAD"),
    ] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child.args([
            "--exact",
            "pin_environment::selected_path_is_the_supplied_pinned_checkout",
            "--nocapture",
        ]);
        match head {
            None => {
                child.env_remove("BENNETT_ISA_SPEC");
            }
            Some(head) => {
                std::fs::write(fixture.0.join("metadata/HEAD"), head).unwrap();
                child.env("BENNETT_ISA_SPEC", &fixture.0);
            }
        }
        let result = child.output().unwrap();
        let diagnostic = String::from_utf8(result.stderr).unwrap();
        assert_eq!(result.status.code(), Some(101), "{diagnostic}");
        assert!(diagnostic.contains("BENNETT_ISA_SPEC"), "{diagnostic}");
        assert!(diagnostic.contains(expected), "{diagnostic}");
    }
}
