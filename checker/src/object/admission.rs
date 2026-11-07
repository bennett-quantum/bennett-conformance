//! Whole-object static source validation, independent of replay paths (§§3.3,4.8,16).
use super::{sec_err, Function, Object, Param};
use crate::eval::{canonical_list, tokenise};
use num_bigint::{BigInt, BigUint};
use num_traits::{ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet};
mod calls;
mod operations;
mod predicates;
mod scalars;
mod syntax;
mod wires;
use operations::{Code, Row};
use scalars::{Scalar, Type};
use wires::Action;

pub(super) fn validate(object: &Object) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for function in &object.functions {
        require(
            names.insert(&function.name),
            "§9.1",
            "repeated function name",
        )?;
        let symbol = object
            .symbol(&function.name)
            .ok_or_else(|| error("§9.1", "function has no symbol"))?;
        require(
            symbol.kind == function.kind,
            "§9.1",
            "symbol and source function kinds differ",
        )?;
        super::signatures::validate(symbol, function, &object.header.versions["isa"])?;
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
            _ => Scalar::validate(object, function, &rows)?,
        }
    }
    Ok(())
}

fn error(section: &str, message: impl Into<String>) -> String {
    sec_err(section, message)
}
fn require(ok: bool, section: &str, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(error(section, message))
    }
}
fn uint(text: &str) -> Result<BigUint, String> {
    require(
        crate::rat::is_canonical_uint_str(text),
        "§16.1",
        "expected canonical unsigned INT",
    )?;
    text.parse().map_err(|_| error("§16.1", "invalid INT"))
}
fn integer(text: &str) -> Result<BigInt, String> {
    uint(text.strip_prefix('-').unwrap_or(text))?;
    text.parse().map_err(|_| error("§16.1", "invalid INT"))
}
fn positive(text: &str) -> Result<BigUint, String> {
    let value = uint(text)?;
    require(!value.is_zero(), "§3.3", "width/count must be positive")?;
    Ok(value)
}
fn list(text: &str) -> Result<Vec<String>, String> {
    canonical_list(text).ok_or_else(|| error("§16.2", "expected a canonical operand list"))
}
