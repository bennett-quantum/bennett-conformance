//! Closed quantum interface schemas and source agreement before host limits.
use super::{sec_err, Function, Param, Symbol};
use crate::json::Value;
use crate::rat::is_canonical_uint_str;
use std::collections::HashSet;

fn integer(value: &Value, positive: bool) -> Result<String, String> {
    let digits = match value {
        Value::Str(s) | Value::Num(s) => s.as_str(),
        _ => {
            return Err(sec_err(
                "§9.1",
                "signature size must be a canonical nonnegative integer",
            ))
        }
    };
    if !is_canonical_uint_str(digits) || (positive && digits == "0") {
        return Err(sec_err(
            "§9.1",
            "signature size has invalid spelling or is zero for a nonempty value",
        ));
    }
    Ok(digits.into())
}
fn source_integer(text: &str, positive: bool) -> Result<String, String> {
    integer(
        &Value::Str(if text == "-0" { "0" } else { text }.into()),
        positive,
    )
}
fn widths(params: &[Param], value: &Value, role: Option<&str>) -> Result<(), String> {
    let array = value
        .as_arr()
        .ok_or_else(|| sec_err("§9.1", "signature width group must be an array"))?;
    if params.len() != array.len() {
        return Err(sec_err(
            "§9.1",
            "signature width group differs in count from the header",
        ));
    }
    for (param, width) in params.iter().zip(array) {
        let source = if param.ty == "wire" {
            "1"
        } else {
            param
                .ty
                .strip_prefix("wire[")
                .and_then(|s| s.strip_suffix(']'))
                .ok_or_else(|| sec_err("§16.2", "quantum interface bundle must have wire type"))?
        };
        if source_integer(source, true)? != integer(width, true)? {
            return Err(sec_err(
                "§9.1",
                "signature bundle width differs from its source declaration",
            ));
        }
        if param.suffix.as_deref().is_some_and(|r| Some(r) != role) {
            return Err(sec_err(
                "§5.1",
                "wire role suffix must agree with its group; target/fault carry no suffix",
            ));
        }
    }
    Ok(())
}
fn classical(ty: &str) -> bool {
    matches!(ty, "ptr" | "f64" | "token")
        || ty.strip_prefix('i').is_some_and(|w| {
            is_canonical_uint_str(w) && w.parse::<u32>().is_ok_and(|n| (1..=128).contains(&n))
        })
}
pub(super) fn validate(symbol: &Symbol, function: &Function, isa: &str) -> Result<(), String> {
    if !matches!(symbol.kind.as_str(), "oracle" | "action" | "perm_action") {
        return Ok(());
    }
    let oracle = symbol.kind == "oracle";
    let required: &[&str] = if oracle {
        &["tier", "kind", "in", "tgt", "fault", "mem", "args", "abi"]
    } else {
        &["tier", "kind", "mut", "in", "mem", "args", "bounds", "abi"]
    };
    let sig = symbol
        .signature
        .as_obj()
        .ok_or_else(|| sec_err("§9.1", "quantum signature must be an object"))?;
    if sig.len() != required.len() || required.iter().any(|k| !sig.contains_key(*k)) {
        return Err(sec_err(
            "§9.1",
            "quantum signature has a missing or unknown field",
        ));
    }
    if sig["tier"].as_str() != Some("q")
        || sig["kind"].as_str() != Some(symbol.kind.as_str())
        || sig["abi"].as_str() != Some(isa)
    {
        return Err(sec_err(
            "§9.1",
            "signature tier/kind/ABI disagrees with the quantum source object",
        ));
    }
    let mut names = HashSet::new();
    for param in function.groups.iter().flatten() {
        let mut bytes = param.name.bytes();
        if !bytes
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
            || !bytes.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.'))
            || !names.insert(&param.name)
        {
            return Err(sec_err(
                "§3.3/§16.2",
                "interface parameter names must be valid and pairwise distinct",
            ));
        }
    }
    let (mem_group, arg_group) = if oracle {
        widths(&function.groups[0], &sig["in"], Some("in"))?;
        widths(
            &function.groups[1],
            &Value::Arr(vec![sig["tgt"].clone()]),
            None,
        )?;
        widths(
            &function.groups[2],
            &Value::Arr(vec![sig["fault"].clone()]),
            None,
        )?;
        (3, 4)
    } else {
        widths(&function.groups[0], &sig["mut"], Some("mut"))?;
        widths(&function.groups[1], &sig["in"], Some("in"))?;
        if !function.groups[4].is_empty() {
            return Err(sec_err("§16.2", "action fifth header group must be empty"));
        }
        let bounds = sig["bounds"]
            .as_arr()
            .ok_or_else(|| sec_err("§9.1", "action bounds must be an array"))?;
        let signature_bounds = bounds
            .iter()
            .map(|v| integer(v, false))
            .collect::<Result<Vec<_>, _>>()?;
        let source = function
            .bytes
            .lines()
            .find_map(|line| {
                line.split_once("bounds {")
                    .map(|(_, rest)| rest.trim_end_matches('}').trim())
            })
            .unwrap_or("");
        let source_bounds = if source.is_empty() {
            vec![]
        } else {
            source
                .split(", ")
                .map(|s| source_integer(s, false))
                .collect::<Result<Vec<_>, _>>()?
        };
        if source_bounds != signature_bounds {
            return Err(sec_err(
                "§9.1/§5.3",
                "signature bounds disagree with the source bounds declaration",
            ));
        }
        (2, 3)
    };
    let args = sig["args"]
        .as_arr()
        .ok_or_else(|| sec_err("§9.1", "args must be an array of classical types"))?;
    if args.len() != function.groups[arg_group].len() {
        return Err(sec_err(
            "§9.1",
            "classical argument count differs from the header",
        ));
    }
    for (ty, param) in args.iter().zip(&function.groups[arg_group]) {
        if ty.as_str() != Some(param.ty.as_str()) || !classical(&param.ty) || param.suffix.is_some()
        {
            return Err(sec_err(
                "§9.1/§16.2",
                "classical argument type/role differs from the source header",
            ));
        }
    }
    let mem = sig["mem"]
        .as_arr()
        .ok_or_else(|| sec_err("§9.1", "mem must be an array of typed region records"))?;
    if mem.len() != function.groups[mem_group].len() {
        return Err(sec_err("§9.1", "region count differs from the header"));
    }
    for (record, param) in mem.iter().zip(&function.groups[mem_group]) {
        let fields = record
            .as_obj()
            .ok_or_else(|| sec_err("§9.1", "mem entry must be an object"))?;
        if fields.len() != 3 || ["A", "W", "role"].iter().any(|k| !fields.contains_key(*k)) {
            return Err(sec_err("§9.1", "mem entry requires exactly A, W and role"));
        }
        let role = fields["role"].as_str();
        if !matches!(role, Some("in" | "mut"))
            || (oracle && role != Some("in"))
            || param.suffix.as_deref() != role
        {
            return Err(sec_err(
                "§5.1",
                "region role must be declared and match its signature; oracle memory is in-role",
            ));
        }
        let (a, w) = param
            .ty
            .strip_prefix("region[")
            .and_then(|s| s.strip_suffix(']'))
            .and_then(|s| s.split_once(','))
            .ok_or_else(|| sec_err("§16.2", "memory parameter must have region[A,W] type"))?;
        if source_integer(a, false)? != integer(&fields["A"], false)?
            || source_integer(w, true)? != integer(&fields["W"], true)?
        {
            return Err(sec_err(
                "§9.1",
                "region shape differs from its source declaration",
            ));
        }
    }
    Ok(())
}
