//! Resolve published test inputs exclusively through this repository's gate pin.
use std::path::PathBuf;

pub fn isa_spec_dir() -> PathBuf {
    let manifest = include_str!("../../../gate.toml");
    let section = manifest
        .split("[siblings.\"bennett-isa-spec\"]")
        .nth(1)
        .expect("gate.toml must pin bennett-isa-spec");
    let sha = section
        .lines()
        .take_while(|l| !l.starts_with('['))
        .find_map(|l| {
            l.trim()
                .strip_prefix("sha = \"")
                .and_then(|s| s.strip_suffix('"'))
        })
        .expect("ISA gate pin must have a SHA");
    assert_eq!(sha.len(), 40, "ISA gate pin must be a full SHA");
    let path = PathBuf::from(std::env::var_os("BENNETT_ISA_SPEC").unwrap_or_else(|| {
        panic!(
            "BENNETT_ISA_SPEC is required: set it to the detached gate checkout for ISA pin {sha}"
        )
    }));
    let gitfile = std::fs::read_to_string(path.join(".git")).unwrap_or_else(|e| {
        panic!(
            "BENNETT_ISA_SPEC={}: ISA pin {sha}: cannot read .git: {e}",
            path.display()
        )
    });
    let gitdir = gitfile
        .trim()
        .strip_prefix("gitdir: ")
        .expect("BENNETT_ISA_SPEC must identify a detached gate worktree via gitdir");
    let actual = std::fs::read_to_string(path.join(gitdir).join("HEAD")).unwrap_or_else(|e| {
        panic!(
            "BENNETT_ISA_SPEC={}: ISA pin {sha}: cannot read detached HEAD: {e}",
            path.display()
        )
    });
    assert_eq!(
        actual.trim(),
        sha,
        "BENNETT_ISA_SPEC={}: expected detached ISA pin {sha}; actual HEAD {}",
        path.display(),
        actual.trim()
    );
    path
}
