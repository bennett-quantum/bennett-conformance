//! ISA §§9.1, 9.2 and 13 coverage metadata. Shape/point binding is distinct
//! from proving that dialect source was lowered, trapped or shimmed correctly.
use crate::cert::{check_keys, rej, u64_of};
use crate::fault::Reject;
use crate::json::{jcs, Value};
use crate::object::Object;
use crate::rat::is_canonical_uint_str;
fn bad(errors: &mut Vec<Reject>, path: impl AsRef<str>, message: &str) {
    errors.push(rej("E_CERT_COVERAGE", "§13", path, message));
}
fn keys(
    v: &Value,
    allowed: &[&str],
    required: &[&str],
    path: &str,
    errors: &mut Vec<Reject>,
) -> bool {
    let mut local = vec![];
    let ok = check_keys(v, allowed, required, path, &mut local);
    errors.extend(local.into_iter().map(|mut e| {
        e.code = "E_CERT_COVERAGE";
        e.section = "§13";
        e
    }));
    ok
}
fn uint(v: &Value) -> bool {
    u64_of(v).is_some() || v.as_str().map(is_canonical_uint_str) == Some(true)
}
fn point(v: &Value, object: &Object, path: &str, errors: &mut Vec<Reject>) {
    let Some(parts) = v.as_arr().filter(|a| a.len() == 3) else {
        bad(
            errors,
            path,
            "program-point id must be [interface_hash, block, ordinal]",
        );
        return;
    };
    let symbol = parts[0]
        .as_str()
        .filter(|h| crate::is_hash_hex(h))
        .and_then(|h| object.symbols.iter().find(|s| s.interface_hash == h));
    let Some(symbol) = symbol else {
        bad(
            errors,
            format!("{path}[0]"),
            "program-point symbol must match a defined symbol's interface hash",
        );
        return;
    };
    let block = parts[1]
        .as_str()
        .and_then(|name| object.instruction_count(&symbol.name, name));
    let Some(count) = block else {
        bad(
            errors,
            format!("{path}[1]"),
            "block does not exist in the identified function",
        );
        return;
    };
    let index = u64_of(&parts[2]).or_else(|| {
        parts[2]
            .as_str()
            .filter(|s| is_canonical_uint_str(s))
            .and_then(|s| s.parse::<u64>().ok())
    });
    if index.map(|i| i < count as u64) != Some(true) {
        bad(
            errors,
            format!("{path}[2]"),
            "ordinal must name an instruction in the identified block",
        );
    }
}
pub fn validate(object: &Object, errors: &mut Vec<Reject>) {
    let Some(coverage) = &object.coverage else {
        return;
    };
    if object
        .coverage_raw
        .as_ref()
        .map(|raw| jcs(coverage).ok().as_deref() == Some(raw.as_bytes()))
        != Some(true)
    {
        bad(
            errors,
            "coverage",
            "coverage metadata must use RFC 8785 canonical bytes",
        );
    }
    if !keys(coverage, &["entries"], &["entries"], "coverage", errors) {
        return;
    }
    let Some(entries) = coverage.get("entries").and_then(Value::as_arr) else {
        bad(errors, "coverage.entries", "expected an array");
        return;
    };
    for (i, entry) in entries.iter().enumerate() {
        let path = format!("coverage.entries[{i}]");
        if !keys(
            entry,
            &[
                "point",
                "source",
                "classification",
                "reason",
                "fault_code",
                "shim_hash",
                "count",
            ],
            &["point", "source", "classification", "reason", "count"],
            &path,
            errors,
        ) {
            continue;
        }
        point(
            entry.get("point").unwrap(),
            object,
            &format!("{path}.point"),
            errors,
        );
        if entry
            .get("source")
            .and_then(Value::as_str)
            .map(|s| !s.is_empty())
            != Some(true)
        {
            bad(
                errors,
                format!("{path}.source"),
                "expected a dialect stable-id string",
            );
        }
        if entry.get("reason").and_then(Value::as_str).is_none() {
            bad(errors, format!("{path}.reason"), "expected a string");
        }
        if !entry.get("count").map(uint).unwrap_or(false) {
            bad(
                errors,
                format!("{path}.count"),
                "expected an exact nonnegative metadata integer",
            );
        }
        let kind = entry.get("classification").and_then(Value::as_str);
        if !matches!(kind, Some("lowered" | "trapped" | "shimmed")) {
            bad(
                errors,
                format!("{path}.classification"),
                "expected lowered, trapped or shimmed",
            );
        }
        if entry.get("fault_code").is_some() != (kind == Some("trapped")) {
            bad(
                errors,
                format!("{path}.fault_code"),
                "fault_code must be present iff classification is trapped",
            );
        }
        if entry.get("shim_hash").is_some() != (kind == Some("shimmed")) {
            bad(
                errors,
                format!("{path}.shim_hash"),
                "shim_hash must be present iff classification is shimmed",
            );
        }
        if entry.get("fault_code").map(|v| !uint(v)) == Some(true) {
            bad(
                errors,
                format!("{path}.fault_code"),
                "expected an exact nonnegative metadata integer",
            );
        }
        if entry
            .get("shim_hash")
            .map(|v| v.as_str().map(crate::is_hash_hex) != Some(true))
            == Some(true)
        {
            bad(
                errors,
                format!("{path}.shim_hash"),
                "expected a 64-lowercase-hex hash",
            );
        }
    }
}
