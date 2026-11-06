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
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../.tmp/gate/cache")
        .join(format!("bennett-isa-spec@{}", &sha[..12]))
        .join("bennett-isa-spec");
    let gitfile = std::fs::read_to_string(path.join(".git"))
        .unwrap_or_else(|e| panic!("ISA pin {sha}: missing cache {}: {e}", path.display()));
    let gitdir = gitfile
        .trim()
        .strip_prefix("gitdir: ")
        .expect("gate cache must identify its detached worktree");
    let actual = std::fs::read_to_string(path.join(gitdir).join("HEAD"))
        .unwrap_or_else(|e| panic!("ISA pin {sha}: cannot read cached HEAD: {e}"));
    assert_eq!(
        actual.trim(),
        sha,
        "ISA pin {sha}: cached checkout actual HEAD {}",
        actual.trim()
    );
    path
}
