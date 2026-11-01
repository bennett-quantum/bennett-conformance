//! Checker rejection and host-limit records. Codes are defined by this
//! checker (the ISA spec publishes no checker error-code registry; see
//! docs/checker.md). Every record names the governing spec section.

use crate::json::{s, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Reject {
    pub code: &'static str,
    pub section: &'static str,
    /// Dotted path into the checked document, e.g. "regions.<hash>.claims[0].domain".
    pub path: String,
    pub message: String,
}

impl Reject {
    pub fn new(code: &'static str, section: &'static str, path: impl Into<String>, message: impl Into<String>) -> Reject {
        Reject { code, section, path: path.into(), message: message.into() }
    }
    pub fn at(&self, prefix: &str) -> Reject {
        let path = if self.path.is_empty() {
            prefix.to_string()
        } else if prefix.is_empty() {
            self.path.clone()
        } else {
            format!("{prefix}.{}", self.path)
        };
        Reject { path, ..self.clone() }
    }
    pub fn to_value(&self) -> Value {
        let mut m = BTreeMap::new();
        m.insert("code".into(), s(self.code));
        m.insert("section".into(), s(self.section));
        m.insert("path".into(), s(&self.path));
        m.insert("message".into(), s(&self.message));
        Value::Obj(m)
    }
}

impl std::fmt::Display for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}) at {}: {}", self.code, self.section, self.path, self.message)
    }
}

/// A legal claim the host cannot materialise/enumerate (ISA section 12.2
/// host limits, K75). Distinct from rejection; discharges no obligation.
#[derive(Debug, Clone, PartialEq)]
pub struct HostLimit {
    pub quantity: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    Reject(Reject),
    HostLimit(HostLimit),
}

impl From<Reject> for ResolveError {
    fn from(r: Reject) -> Self {
        ResolveError::Reject(r)
    }
}
impl From<HostLimit> for ResolveError {
    fn from(h: HostLimit) -> Self {
        ResolveError::HostLimit(h)
    }
}
