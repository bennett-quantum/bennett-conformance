//! Independent certificate checker for Bennett ISA certificates
//! (bennett-isa-spec section 12, with section 13 coverage and the
//! section 11 schedule bindings a certificate carries).
//!
//! Written from the published spec only. It shares no code with the
//! reference interpreter, the VM, or the plugin; sibling binaries may be
//! run as black boxes (see `--oracle-bin`) but their output is compared,
//! never trusted. See `docs/checker.md` for the verdict contract.

pub mod cert;
pub mod domain;
pub mod fault;
pub mod eval;
pub mod json;
pub mod numeric;
pub mod object;
pub mod rat;
pub mod sampler;
pub mod sha256;

pub use sha256::{is_hash_hex, sha256, sha256_hex};
