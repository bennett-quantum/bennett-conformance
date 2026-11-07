//! Whole-object static source validation, independent of replay paths (§§3.3,4.8,16).
use super::{sec_err, Function, Object, Param};
use crate::eval::{canonical_list, tokenise};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;
use std::collections::{BTreeMap, BTreeSet};
mod calls;
mod classical;
mod operations;
mod predicates;
mod scalars;
mod syntax;
mod views;
mod wires;
use operations::{Code, Row};
use scalars::{Scalar, Type};
use wires::Action;

pub(super) fn validate(object: &Object) -> Result<Vec<String>, String> {
    let mut obligations = vec![];
    require(
        (1..=128).contains(&object.header.ptr_width),
        "§3.3",
        "ptr_width must be 1..128",
    )?;
    if let Some(layout) = &object.layout {
        require(
            layout.get("ptr_width").and_then(crate::cert::u64_of) == Some(object.header.ptr_width),
            "§7.2",
            "layout pointer width mismatch",
        )?;
    }
    let mut names = BTreeSet::new();
    for function in &object.functions {
        require(
            names.insert(&function.name),
            "§9.1",
            "repeated function name",
        )?;
        let symbol = object
            .symbol(&function.name)
            .ok_or_else(|| sec_err("§9.1", "function has no symbol"))?;
        require(
            symbol.kind == function.kind,
            "§9.1",
            "symbol and source function kinds differ",
        )?;
        super::signatures::validate(symbol, function, &object.header.versions["isa"])?;
        if matches!(function.kind.as_str(), "cc" | "r") {
            for (group, key) in [(0, "params"), (1, "results")] {
                let declared = symbol
                    .signature
                    .get(key)
                    .and_then(crate::json::Value::as_arr)
                    .ok_or_else(|| sec_err("§9.1", "missing classical signature group"))?;
                require(
                    declared.len() == function.groups[group].len(),
                    "§9.1",
                    "classical signature group count differs",
                )?;
                for (p, t) in function.groups[group].iter().zip(declared) {
                    require(
                        t.as_str() == Some(&p.ty),
                        "§9.1",
                        "classical signature type differs from source",
                    )?;
                }
            }
        }
        syntax::validate(function)?;
        let rows = function
            .bytes
            .lines()
            .skip(1)
            .filter(|line| !line.starts_with("regs ") && !line.starts_with('^'))
            .filter(|line| !matches!(*line, "}" | "} {"))
            .map(|line| Row::parse(line.strip_suffix(" {").unwrap_or(line), &function.kind))
            .collect::<Result<Vec<_>, _>>()?;
        match function.kind.as_str() {
            "action" | "perm_action" => Action::validate(object, function)?,
            _ => {
                Scalar::validate(object, function, &rows)?;
                if matches!(function.kind.as_str(), "cc" | "r")
                    && classical::repeated_definitions(function, &rows)
                {
                    obligations.push(format!("@{}: multiple definitions retain their §3.4 type constraints, but source admission against the pinned SSA rule is unresolved; proof is deferred",function.name));
                }
            }
        }
    }
    Ok(obligations)
}

fn require(ok: bool, section: &str, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(sec_err(section, message))
    }
}
fn uint(text: &str) -> Result<BigUint, String> {
    require(
        crate::rat::is_canonical_uint_str(text),
        "§16.1",
        "expected canonical unsigned INT",
    )?;
    text.parse().map_err(|_| sec_err("§16.1", "invalid INT"))
}
fn integer(text: &str) -> Result<BigInt, String> {
    uint(text.strip_prefix('-').unwrap_or(text))?;
    text.parse().map_err(|_| sec_err("§16.1", "invalid INT"))
}
fn positive(text: &str) -> Result<BigUint, String> {
    let value = uint(text)?;
    require(!value.is_zero(), "§3.3", "width/count must be positive")?;
    Ok(value)
}
fn list(text: &str) -> Result<Vec<String>, String> {
    canonical_list(text).ok_or_else(|| sec_err("§16.2", "expected a canonical operand list"))
}
