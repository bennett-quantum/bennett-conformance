//! Whole-object static source validation, independent of replay paths (§§3.3,4.8,16).
use super::{sec_err, Function, Object, Param};
use crate::eval::{canonical_list, tokenise};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;
use std::collections::{BTreeMap, BTreeSet};
mod calls;
mod classical;
mod operations;
#[cfg(test)]
mod predicates;
mod procedures;
mod scalars;
mod syntax;
mod views;
mod wires;
use operations::{Code, Row};
use scalars::{Scalar, Type};
use wires::Action;
// ISA §§3.3/4.8: iN and pointer widths share this source type bound.
const MAX_INLINE_WIDTH: u32 = 128;
// ISA §3.3: register handles and dynamic size/index values use i64.
const REGISTER_INTEGER_WIDTH: u32 = 64;
// ISA §3.1: f64 pseudo-locations carry the binary64 bits.
const FLOAT_WIDTH: u32 = 64;

pub(super) fn validate(object: &Object) -> Result<Vec<String>, String> {
    let mut obligations = vec![];
    require(
        (1..=u64::from(MAX_INLINE_WIDTH)).contains(&object.header.ptr_width),
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
        if matches!(function.kind.as_str(), "cc" | "r" | "lproc") {
            procedures::signature_fields(symbol, &object.header.versions["isa"])?;
            let declared = calls::metadata(&symbol.signature)?;
            let header = function
                .groups
                .iter()
                .map(|g| g.iter().map(|p| Type::parse(&p.ty)).collect())
                .collect::<Result<Vec<Vec<Type>>, _>>()?;
            require(
                declared.kind == function.kind && declared.groups == header,
                "§9.1",
                "signature types differ from source header",
            )?;
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
        if function.kind == "oracle" {
            procedures::fault_width(object, function)?;
        }
        if function.kind == "lproc" || rows.iter().any(|r| r.code == Code::ProcedureCall) {
            require(
                object.layout.is_some(),
                "§7.2",
                "memory or lprocs require layout",
            )?;
        }
        if rows
            .iter()
            .any(|r| matches!(r.code, Code::Label | Code::JumpRegister))
        {
            obligations.push(format!("@{}: §3.6 source index-space widths require complete procedure binding instances; static admission remains unchecked",function.name));
        }
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
    if !object.imports.is_empty() {
        obligations.push("§3.3/§9.2: complete source admission of imported closures requires linked-body validation; imported source remains unchecked".into());
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
