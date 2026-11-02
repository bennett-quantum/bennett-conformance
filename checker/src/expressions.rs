//! Exact ISA §8.1 target expression evaluation. Table bounds and logarithm
//! domains are checked; no floating point logarithm or fallback is used.
use crate::fault::HostLimit;
use crate::json::Value;
use crate::rat::{rat_of_json, Rat};
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::HashMap;
#[derive(Debug)]
pub enum Error {
    Invalid(String),
    HostLimit(HostLimit),
}
type Result<T> = std::result::Result<T, Error>;
fn bad<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(message.into()))
}
fn epsilon_index(vars: &HashMap<&str, Rat>, base: u32) -> Result<u32> {
    let eps = vars
        .get("eps")
        .ok_or_else(|| Error::Invalid("epsilon index references unbound eps".into()))?;
    if eps <= &Rat::zero() || eps > &Rat::one() {
        return bad("epsilon index requires 0 < eps <= 1");
    }
    let bits = eps.denom().bits();
    if bits > 1_000_000 {
        return Err(Error::HostLimit(HostLimit {
            quantity: "epsilon logarithm exceeds one million host bits".into(),
        }));
    }
    let mut low = 0u32;
    let mut high = bits as u32;
    while low < high {
        let mid = low + (high - low) / 2;
        if eps.numer() * num_bigint::BigInt::from(base).pow(mid) >= *eps.denom() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Ok(low)
}
pub fn uses_epsilon(v: &Value) -> bool {
    v.get("var").and_then(Value::as_str) == Some("eps")
        || v.get("op").and_then(Value::as_str) == Some("log2inv_eps")
        || v.get("index").and_then(Value::as_str) == Some("eps_decade")
        || match v {
            Value::Obj(m) => m.values().any(uses_epsilon),
            Value::Arr(a) => a.iter().any(uses_epsilon),
            _ => false,
        }
}
pub fn evaluate(v: &Value, vars: &HashMap<&str, Rat>, tables: &Value) -> Result<Rat> {
    if let Value::Num(text) | Value::Str(text) = v {
        if let Some(limit) = crate::rat::decimal_host_limit(text) {
            return Err(Error::HostLimit(limit));
        }
    }
    if let Ok(r) = rat_of_json(v) {
        return Ok(r);
    }
    if let Some(name) = v.get("var").and_then(Value::as_str) {
        return vars.get(name).cloned().ok_or_else(|| {
            Error::Invalid(format!("expression references unbound variable {name}"))
        });
    }
    if let Some(name) = v.get("table").and_then(Value::as_str) {
        let table = tables
            .get(name)
            .and_then(Value::as_arr)
            .ok_or_else(|| Error::Invalid(format!("undeclared table {name}")))?;
        let index = match v.get("index").and_then(Value::as_str) {
            Some("eps_decade") => Rat::from_integer(epsilon_index(vars, 10)?.into()),
            Some("k") => vars
                .get("k")
                .cloned()
                .ok_or_else(|| Error::Invalid("table index references unbound k".into()))?,
            _ => return bad("table index must be k or eps_decade"),
        };
        if index.is_negative()
            || !index.is_integer()
            || index >= Rat::from_integer(table.len().into())
        {
            return bad(format!("table {name} index {index} is outside its array"));
        }
        return rat_of_json(
            &table[index
                .to_integer()
                .to_usize()
                .expect("index bounded by host table length")],
        )
        .map_err(Error::Invalid);
    }
    let op = v.get("op").and_then(Value::as_str).unwrap_or("");
    let args = v
        .get("args")
        .and_then(Value::as_arr)
        .ok_or_else(|| Error::Invalid("expression args must be an array".into()))?;
    let arity = match op {
        "+" | "-" | "*" | "/" | "min" | "max" => 2,
        "ceil" => 1,
        "log2inv_eps" => 0,
        _ => return bad(format!("unknown target expression operation {op:?}")),
    };
    if args.len() != arity {
        return bad("target expression has incorrect arity");
    }
    if op == "log2inv_eps" {
        return Ok(Rat::from_integer(epsilon_index(vars, 2)?.into()));
    }
    let a = evaluate(&args[0], vars, tables)?;
    if op == "ceil" {
        return Ok(Rat::from(a.ceil().to_integer()));
    }
    let b = evaluate(&args[1], vars, tables)?;
    Ok(match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => {
            if b.is_zero() {
                return bad("division by zero in target expression");
            }
            a / b
        }
        "min" => a.min(b),
        "max" => a.max(b),
        _ => unreachable!("arity and opcode checked"),
    })
}
