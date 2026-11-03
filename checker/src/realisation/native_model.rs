//! Phase-bearing native models, from ISA §8.3 K86. Rational matrix models
//! are checked exactly before conversion to outward interval arithmetic.
use super::*;
use num_bigint::{BigInt, BigUint};

fn bad<T>(message: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::rejected(vec![rej(
        "E_CERT_TARGET",
        "§8.3 K86",
        "target.gates.realisation_model",
        message.into(),
    )]))
}
fn keys(value: &Value, fields: &[&str], required: &[&str]) -> Result<()> {
    let mut errors = vec![];
    if !crate::cert::check_keys(
        value,
        fields,
        required,
        "target.gates.realisation_model",
        &mut errors,
    ) {
        for e in &mut errors {
            e.code = "E_CERT_TARGET";
            e.section = "§8.3 K86";
        }
        return Err(ClaimOutcome::rejected(errors));
    }
    Ok(())
}
fn numerator(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    crate::rat::is_canonical_uint_str(digits) && text != "-0"
}
fn fraction(value: &Value) -> Result<Rat> {
    let a = value.as_arr().filter(|a| a.len() == 2).ok_or_else(|| {
        ClaimOutcome::rejected(vec![rej(
            "E_CERT_TARGET",
            "§8.3 K86",
            "realisation_model",
            "rational constant must be a numerator/denominator pair",
        )])
    })?;
    let Some(n) = a[0].as_str().filter(|s| numerator(s)) else {
        return bad("noncanonical rational numerator");
    };
    let Some(d) = a[1]
        .as_str()
        .filter(|s| crate::rat::is_canonical_uint_str(s) && *s != "0")
    else {
        return bad("noncanonical or nonpositive rational denominator");
    };
    if n.len() > 4096 || d.len() > 4096 {
        return pending("host limit: native rational constants exceed 4096 decimal digits");
    }
    let n = n.parse::<BigInt>().unwrap();
    let d = d.parse::<BigUint>().unwrap();
    if crate::rat::gcd(n.magnitude().clone(), d.clone()) != BigUint::one() {
        return bad("native rational constants must be reduced");
    }
    Ok(Rat::new(n, d.into()))
}

/// Schema checks apply even to unused entries. Unknown versions are retained
/// for unsupported-model reporting, rather than interpreted as version 1.
pub(crate) fn validate(model: &Value, path: &str, errors: &mut Vec<crate::fault::Reject>) {
    let result = (|| -> Result<()> {
        let Some(version) = model.get("version").and_then(Value::as_str) else {
            return bad("native model requires a string version");
        };
        if version != "bennett.native-model/1" {
            return Ok(());
        }
        let fields = [
            "version",
            "kind",
            "arity",
            "basis",
            "parameter_bindings",
            "control_models",
            "data",
            "operand_map",
        ];
        keys(model, &fields, &fields)?;
        if model.get("kind").and_then(Value::as_str) != Some("unitary") {
            return bad("native model kind must be unitary");
        }
        let Some(arity) = model.get("arity").and_then(u64_of) else {
            return bad("native arity must be a supported nonnegative integer");
        };
        let Some(basis) = model.get("basis").and_then(Value::as_arr) else {
            return bad("native basis must be an ordered string array");
        };
        if arity != basis.len() as u64
            || basis.iter().any(|b| b.as_str().is_none())
            || basis
                .iter()
                .filter_map(Value::as_str)
                .collect::<HashSet<_>>()
                .len()
                != basis.len()
        {
            return bad("native basis must contain arity distinct labels");
        }
        let Some(bindings) = model.get("parameter_bindings").and_then(Value::as_obj) else {
            return bad("native parameter_bindings must be a map");
        };
        if bindings
            .values()
            .any(|v| !matches!(v.as_str(), Some("k" | "c" | "eps" | "theta")))
        {
            return bad("native parameter bindings must name resolved k, c, eps or theta");
        }
        let Some(control) = model.get("control_models").and_then(Value::as_obj) else {
            return bad("native control_models must be a record");
        };
        if control.get("kind").and_then(Value::as_str).is_none() {
            return bad("native control model requires a string kind");
        }
        if control.get("kind").and_then(Value::as_str) == Some("controlled_U") {
            let value = model.get("control_models").unwrap();
            keys(
                value,
                &["kind", "controls", "disabled"],
                &["kind", "controls", "disabled"],
            )?;
            if value.get("controls").and_then(Value::as_str) != Some("c")
                || value.get("disabled").and_then(Value::as_str) != Some("identity")
            {
                return bad("controlled_U must bind c and identity on disabled branches");
            }
        }
        let Some(map) = model.get("operand_map").and_then(Value::as_arr) else {
            return bad("native operand_map must be an integer array");
        };
        if (map.len() as u64) < arity {
            return bad("native operand_map omits logical operands");
        }
        if map.iter().any(|v| u64_of(v).is_none()) {
            return bad("native operand_map contains a noninteger position");
        }
        if map.iter().filter_map(u64_of).collect::<HashSet<_>>().len() != map.len() {
            return bad("native operand_map repeats an allocation-basis position");
        }
        let Some(data) = model.get("data").and_then(Value::as_obj) else {
            return bad("native model data must be a record");
        };
        if data.is_empty() {
            return bad("native model data does not identify a phase-bearing unitary");
        }
        if let Some(matrix) = data.get("matrix") {
            let Some(rows) = matrix.as_arr() else {
                return bad("native matrix must be a row-major array of rows");
            };
            if !rows.len().is_power_of_two() || rows.len().trailing_zeros() as u64 != arity {
                return bad("native matrix dimensions do not match its declared arity");
            }
            for row in rows {
                let Some(row) = row.as_arr() else {
                    return bad("native matrix row must be an array");
                };
                if row.len() != rows.len() {
                    return bad("native matrix must be square");
                }
                for entry in row {
                    let Some(pair) = entry.as_arr().filter(|a| a.len() == 2) else {
                        return bad("native matrix entry must be [real,imaginary]");
                    };
                    let mut pending_expr = pair.iter().collect::<Vec<_>>();
                    while let Some(expr) = pending_expr.pop() {
                        if expr.as_arr().is_some() {
                            // A large exact constant is a host limit at use,
                            // never a target-schema rejection.
                            match fraction(expr) {
                                Ok(_) => {}
                                Err(e) if e.outcome == "unchecked" => {}
                                Err(e) => return Err(e),
                            }
                        } else if let Some(var) = expr.get("var") {
                            keys(expr, &["var"], &["var"])?;
                            if !var.as_str().is_some_and(|s| bindings.contains_key(s)) {
                                return bad("native expression variable is not declared");
                            }
                        } else {
                            keys(expr, &["op", "args"], &["op", "args"])?;
                            let arity = match expr.get("op").and_then(Value::as_str) {
                                Some("i" | "pi") => 0,
                                Some("sqrt" | "exp" | "sin" | "cos") => 1,
                                Some("+" | "-" | "*" | "/") => 2,
                                _ => return bad("unknown native exact expression operation"),
                            };
                            let Some(args) = expr.get("args").and_then(Value::as_arr) else {
                                return bad("native expression args must be an array");
                            };
                            if args.len() != arity {
                                return bad("native exact expression arity mismatch");
                            }
                            pending_expr.extend(args);
                        }
                    }
                }
            }
        }
        Ok(())
    })();
    if let Err(mut outcome) = result {
        for error in &mut outcome.errors {
            error.path = path.to_string();
        }
        errors.extend(outcome.errors);
    }
}

#[derive(Clone, PartialEq)]
struct Z {
    re: Rat,
    im: Rat,
}
impl Z {
    fn zero() -> Self {
        Self {
            re: Rat::zero(),
            im: Rat::zero(),
        }
    }
    fn real(re: Rat) -> Self {
        Self {
            re,
            im: Rat::zero(),
        }
    }
    fn conj(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: -&self.im,
        }
    }
    fn sub(&self, b: &Self) -> Self {
        Self {
            re: &self.re - &b.re,
            im: &self.im - &b.im,
        }
    }
    fn mul(&self, b: &Self) -> Self {
        Self {
            re: &self.re * &b.re - &self.im * &b.im,
            im: &self.re * &b.im + &self.im * &b.re,
        }
    }
    fn scale(&self, r: &Rat) -> Self {
        Self {
            re: &self.re * r,
            im: &self.im * r,
        }
    }
    fn interval(&self) -> C {
        C {
            re: Interval::exact(self.re.clone()),
            im: Interval::exact(self.im.clone()),
        }
    }
}
fn gram(matrix: &[Z], n: usize) -> Vec<Z> {
    let mut result = vec![Z::zero(); n * n];
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let value = matrix[k * n + i].conj().mul(&matrix[k * n + j]);
                result[i * n + j].re += value.re;
                result[i * n + j].im += value.im;
            }
        }
    }
    result
}
/// Exact Hermitian Schur elimination, including zero pivots. A zero diagonal
/// of a PSD matrix has a zero row/column; no strict-positive shortcut applies.
fn psd(mut matrix: Vec<Z>, n: usize) -> bool {
    for k in 0..n {
        let d = matrix[k * n + k].clone();
        if !d.im.is_zero() || d.re.is_negative() {
            return false;
        }
        if d.re.is_zero() {
            if (k + 1..n).any(|i| matrix[i * n + k] != Z::zero()) {
                return false;
            }
            continue;
        }
        for i in k + 1..n {
            for j in k + 1..n {
                let adjustment = matrix[i * n + k]
                    .mul(&matrix[j * n + k].conj())
                    .scale(&d.re.recip());
                matrix[i * n + j] = matrix[i * n + j].sub(&adjustment);
            }
        }
    }
    true
}
fn exact_expression(value: &Value, vars: &HashMap<String, Rat>, depth: usize) -> Result<Rat> {
    if depth > 64 {
        return pending("host limit: native exact expression nesting exceeds 64");
    }
    if value.as_arr().is_some() {
        return fraction(value);
    }
    if let Some(var) = value.get("var").and_then(Value::as_str) {
        let value = vars.get(var).cloned().ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_TARGET",
                "§8.3 K86",
                "realisation_model",
                "native variable has no resolved binding",
            )])
        })?;
        return bounded(value);
    }
    let op = value.get("op").and_then(Value::as_str).unwrap_or("");
    let args = value.get("args").and_then(Value::as_arr).unwrap_or(&[]);
    if !matches!(op, "+" | "-" | "*" | "/") {
        return pending(format!(
            "native exact operation {op} needs symbolic/interval model reconstruction"
        ));
    }
    let a = exact_expression(&args[0], vars, depth + 1)?;
    let b = exact_expression(&args[1], vars, depth + 1)?;
    bounded(match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" if !b.is_zero() => a / b,
        _ => return bad("native exact expression divides by zero"),
    })
}
fn bounded(value: Rat) -> Result<Rat> {
    if value.numer().bits() > 65_536 || value.denom().bits() > 65_536 {
        return pending(
            "host limit: native rational expression exceeds 65536 numerator/denominator bits",
        );
    }
    Ok(value)
}

pub(super) fn matrix(docs: &LoadedDocs, gate: &Gate, wires: usize, p: u32) -> Result<Matrix> {
    let entry = native(docs, gate)?;
    let Some(model) = entry.get("realisation_model") else {
        return gate_matrix(gate, wires, p);
    };
    if model.get("version").and_then(Value::as_str) != Some("bennett.native-model/1") {
        return pending("unsupported native realisation model version");
    }
    let (resolved, achieved, _) = native_numbers(docs, gate, entry)?;
    let mut vars = HashMap::new();
    for (name, binding) in model
        .get("parameter_bindings")
        .and_then(Value::as_obj)
        .unwrap()
    {
        let binding = binding.as_str().unwrap();
        let value = if binding == "theta" {
            gate.angle_bits
                .and_then(|b| crate::transcendental::binary64_rational(b).ok())
        } else if binding == "eps" && achieved.is_zero() {
            None
        } else {
            resolved.get(binding).cloned()
        };
        let Some(value) = value else {
            return bad(format!(
                "native binding {name} cannot resolve {binding} in this context"
            ));
        };
        vars.insert(name.clone(), value);
    }
    if model
        .get("control_models")
        .and_then(|c| c.get("kind"))
        .and_then(Value::as_str)
        != Some("controlled_U")
    {
        return pending("explicit native control-model reconstruction is not implemented");
    }
    // First supported representation: the complete one-wire context. Larger
    // interfaces and controlled embedding remain unchecked, never idealised.
    if wires != 1 || !gate.controls.is_empty() || gate.targets != [0] {
        return pending("native controlled/multi-wire operand-map embedding is not implemented");
    }
    if model.get("arity").and_then(u64_of) != Some(1)
        || model.get("basis").and_then(Value::as_arr).unwrap() != [s("t")]
        || model
            .get("operand_map")
            .and_then(Value::as_arr)
            .unwrap()
            .iter()
            .map(u64_of)
            .collect::<Vec<_>>()
            != [Some(0)]
    {
        return bad(
            "native arity, basis or operand_map disagrees with the resolved one-wire interface",
        );
    }
    let data = model.get("data").and_then(Value::as_obj).unwrap();
    if data.len() != 1 || !data.contains_key("matrix") {
        return pending("native Kraus/instrument/enclosure equivalence needs an independent representation proof");
    }
    let rows = data["matrix"].as_arr().unwrap();
    if rows.len() != 2 || rows.iter().any(|r| r.as_arr().unwrap().len() != 2) {
        return bad("native matrix dimensions do not match its basis");
    }
    let mut exact = Vec::new();
    for cell in rows.iter().flat_map(|r| r.as_arr().unwrap()) {
        let pair = cell.as_arr().unwrap();
        exact.push(Z {
            re: exact_expression(&pair[0], &vars, 0)?,
            im: exact_expression(&pair[1], &vars, 0)?,
        });
    }
    let unit = gram(&exact, 2);
    for i in 0..2 {
        for j in 0..2 {
            if unit[i * 2 + j] != Z::real(if i == j { Rat::one() } else { Rat::zero() }) {
                return bad("phase-bearing native matrix is not unitary");
            }
        }
    }
    let actual = Matrix {
        dim: 2,
        data: exact.iter().map(Z::interval).collect(),
    };
    let ideal = gate_matrix(gate, wires, p)?;
    let exact_ideal = ideal
        .data
        .iter()
        .map(|z| {
            if z.re.lo == z.re.hi && z.im.lo == z.im.hi {
                Some(Z {
                    re: z.re.lo.clone(),
                    im: z.im.lo.clone(),
                })
            } else {
                None
            }
        })
        .collect::<Option<Vec<_>>>();
    if let Some(ideal) = exact_ideal {
        let delta = exact
            .iter()
            .zip(ideal)
            .map(|(u, v)| u.sub(&v))
            .collect::<Vec<_>>();
        let mut bound = gram(&delta, 2);
        for i in 0..2 {
            for j in 0..2 {
                bound[i * 2 + j] = Z::real(if i == j {
                    &achieved * &achieved
                } else {
                    Rat::zero()
                })
                .sub(&bound[i * 2 + j]);
            }
        }
        if !psd(bound, 2) {
            return bad(
                "native model violates its advertised operator-norm accuracy (including phase)",
            );
        }
    } else if actual.sub(&ideal, p).frobenius_upper(p) > achieved {
        return pending(
            "native interval model accuracy bound is inconclusive at the requested precision",
        );
    }
    Ok(actual)
}
