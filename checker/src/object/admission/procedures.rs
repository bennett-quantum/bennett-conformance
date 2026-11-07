//! §3.3 fault literal widths belong to every enclosing oracle's static closure.
use super::*;
// ISA §4.9 uses uint64_t classical causes; §5.4 reserves four oracle fault bits.
const CLASSICAL_FAULT_WIDTH: u32 = 64;
const MIN_ORACLE_FAULT_WIDTH: u32 = 4;

pub(super) fn fault_width(object: &Object, function: &Function) -> Result<Option<BigUint>, String> {
    match function.kind.as_str() {
        "oracle" => Ok(Some(oracle_width(function, object)?)),
        "lproc" => {
            let mut widths = vec![];
            for oracle in object.functions.iter().filter(|f| f.kind == "oracle") {
                if reaches(object, oracle, &function.name)? {
                    widths.push(oracle_width(oracle, object)?);
                }
            }
            Ok(widths.into_iter().min())
        }
        _ => Ok(Some(CLASSICAL_FAULT_WIDTH.into())),
    }
}
fn oracle_width(function: &Function, object: &Object) -> Result<BigUint, String> {
    let fault = function.groups[2]
        .first()
        .ok_or_else(|| sec_err("§3.3", "oracle has no fault register"))?;
    let width = Type::parse(&fault.ty)?.bits(object.header.ptr_width)?;
    require(
        width >= MIN_ORACLE_FAULT_WIDTH.into(),
        "§5.4",
        "oracle fault register requires at least four bits",
    )?;
    Ok(width)
}
fn reaches(object: &Object, root: &Function, target: &str) -> Result<bool, String> {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(function) = pending.pop() {
        if function.name == target {
            return Ok(true);
        }
        if !seen.insert(&function.name) {
            continue;
        }
        for line in function.blocks.iter().flat_map(|b| &b.instructions) {
            let row = Row::parse(line, &function.kind)?;
            if row.code == Code::ProcedureCall {
                let name = row.args[0]
                    .strip_prefix('@')
                    .ok_or_else(|| sec_err("§16.2", "expected procedure symbol"))?;
                if let Some(callee) = object.function(name) {
                    pending.push(callee);
                } else {
                    require(
                        object.imports.contains_key(name),
                        "§3.7",
                        "unknown procedure callee",
                    )?;
                }
            }
        }
    }
    Ok(false)
}

pub(super) fn signature_fields(symbol: &super::super::Symbol, isa: &str) -> Result<(), String> {
    let s = symbol
        .signature
        .as_obj()
        .ok_or_else(|| sec_err("§9.1", "function signature must be an object"))?;
    let (tier, required, allowed): (&str, &[&str], &[&str]) = match symbol.kind.as_str() {
        "cc" => (
            "cc",
            &["abi", "kind", "tier", "params", "results"],
            &["abi", "kind", "tier", "params", "results"],
        ),
        "r" => (
            "r",
            &["abi", "kind", "tier", "params", "results"],
            &["abi", "kind", "tier", "params", "results"],
        ),
        "lproc" if isa == "0.8" => (
            "q",
            &["abi", "kind", "tier", "params", "result"],
            &[
                "abi", "kind", "tier", "params", "result", "tokens", "args", "mem",
            ],
        ),
        "lproc" => (
            "q",
            &[
                "abi", "kind", "tier", "params", "result", "tokens", "args", "mem",
            ],
            &[
                "abi", "kind", "tier", "params", "result", "tokens", "args", "mem",
            ],
        ),
        _ => return Err(sec_err("§9.1", "unexpected signature kind")),
    };
    require(
        required.iter().all(|k| s.contains_key(*k))
            && s.keys().all(|k| allowed.contains(&k.as_str())),
        "§9.1",
        "signature has missing or unknown fields",
    )?;
    require(
        s["abi"].as_str() == Some(isa)
            && s["kind"].as_str() == Some(&symbol.kind)
            && s["tier"].as_str() == Some(tier),
        "§9.1",
        "signature ABI, kind or tier differs from source",
    )?;
    Ok(())
}
