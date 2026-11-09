//! Closed image attachment productions from ISA §12.3.
use super::*;
use num_bigint::BigUint;
use num_traits::Zero;

pub(super) fn keys(v: &Value, fields: &[&str], required: &[&str]) -> Result<()> {
    let mut errors = vec![];
    if !crate::cert::check_keys(v, fields, required, "bindings.preparation", &mut errors) {
        for e in &mut errors {
            e.code = "E_CERT_PREPARATION";
        }
        return Err(Error::Rejected(errors));
    }
    Ok(())
}
pub(super) fn uint(v: &Value) -> Result<BigUint> {
    crate::domain::biguint_of_json(v, "preparation coordinate").map_err(|e| match e {
        crate::fault::ResolveError::Reject(e) => Error::Rejected(vec![crate::cert::rej(
            "E_CERT_PREPARATION",
            "§12.3",
            "bindings.preparation",
            e.message,
        )]),
        crate::fault::ResolveError::HostLimit(e) => Error::Unavailable(e.quantity),
    })
}
pub(super) fn index(v: &Value) -> Result<usize> {
    uint(v)?
        .to_string()
        .parse()
        .map_err(|_| Error::Unavailable("host limit: preparation index exceeds usize".into()))
}
pub(super) fn array(v: &Value) -> Result<&[Value]> {
    v.as_arr().ok_or_else(|| invalid("expected an array"))
}
pub(super) fn hash(v: &Value) -> Result<&str> {
    v.as_str()
        .filter(|s| crate::is_hash_hex(s))
        .ok_or_else(|| invalid("expected a canonical SHA-256 hash"))
}
pub(super) fn get<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key).ok_or_else(|| invalid(format!("missing {key}")))
}
fn integers(v: &Value) -> Result<()> {
    for v in array(v)? {
        uint(v)?;
    }
    Ok(())
}
fn coordinates(v: &Value, object: &crate::object::Object) -> Result<()> {
    let symbol = hash(get(v, "symbol")?)?;
    let sym = object
        .symbols
        .iter()
        .find(|s| s.interface_hash == symbol)
        .ok_or_else(|| {
            invalid("occurrence symbol is not a defining interface hash in the linked object")
        })?;
    let point = get(v, "point")?
        .as_str()
        .ok_or_else(|| invalid("point must be a source point"))?;
    let support = match point {
        "$construct" | "$release" => {
            matches!(sym.kind.as_str(), "oracle" | "action" | "perm_action")
        }
        "$commit" | "$step" => sym.kind == "oracle",
        _ => false,
    };
    if !support {
        let (block, ord) = point
            .rsplit_once(':')
            .ok_or_else(|| invalid("unknown source/pseudo point"))?;
        let ordinal = ord
            .parse::<usize>()
            .map_err(|_| invalid("source ordinal must be an integer"))?;
        if ordinal.to_string() != ord
            || object
                .instruction_count(&sym.name, block)
                .is_none_or(|n| ordinal >= n)
        {
            return Err(invalid("source point is stale or out of range"));
        }
    }
    integers(get(v, "iteration")?)?;
    integers(get(v, "instance")?)?;
    match (v.get("step"), v.get("phase")) {
        (Some(step), Some(phase))
            if sym.kind == "oracle" && !matches!(point, "$construct" | "$commit" | "$release") =>
        {
            if uint(step)?.is_zero() || !matches!(phase.as_str(), Some("fwd" | "rev")) {
                return Err(invalid("invalid oracle step/phase"));
            }
        }
        (None, None)
            if sym.kind != "oracle" || matches!(point, "$construct" | "$commit" | "$release") => {}
        _ => {
            return Err(invalid(
                "step and phase must occur together exactly on oracle step leaves",
            ))
        }
    }
    Ok(())
}
pub(super) fn context(v: &Value, object: &crate::object::Object) -> Result<()> {
    for edge in array(v)? {
        keys(
            edge,
            &[
                "symbol",
                "point",
                "iteration",
                "instance",
                "call_index",
                "step",
                "phase",
            ],
            &["symbol", "point", "iteration", "instance", "call_index"],
        )?;
        coordinates(edge, object)?;
        uint(get(edge, "call_index")?)?;
    }
    Ok(())
}
pub(super) fn occurrence(
    v: &Value,
    object: &crate::object::Object,
    allocation: bool,
) -> Result<()> {
    let required = [
        "context_key",
        "symbol",
        "point",
        "iteration",
        "instance",
        "generated",
    ];
    let fields = if allocation {
        &required[..]
    } else {
        &[
            "context_key",
            "symbol",
            "point",
            "iteration",
            "instance",
            "generated",
            "allocation",
        ][..]
    };
    let mut fields = fields.to_vec();
    fields.extend(["step", "phase"]);
    keys(v, &fields, &required)?;
    context(get(v, "context_key")?, object)?;
    coordinates(v, object)?;
    uint(get(v, "generated")?)?;
    if let Some(a) = v.get("allocation") {
        occurrence(a, object, true)?;
    }
    Ok(())
}
pub(super) fn witness(
    v: &Value,
    object: &crate::object::Object,
    image_names: &[String],
) -> Result<()> {
    match get(v, "kind")?.as_str() {
        Some("logical") => {
            keys(v, &["kind", "regions"], &["kind", "regions"])?;
            let regions = array(get(v, "regions")?)?;
            if regions.len() != image_names.len() {
                return Err(invalid(
                    "logical regions must cover each attachment image exactly once",
                ));
            }
            for (r, name) in regions.iter().zip(image_names) {
                keys(
                    r,
                    &["name", "allocation", "prepare"],
                    &["name", "allocation", "prepare"],
                )?;
                if get(r, "name")?.as_str() != Some(name) {
                    return Err(invalid(
                        "logical regions must follow subject mem-slot order",
                    ));
                }
                occurrence(get(r, "allocation")?, object, false)?;
                let mut seen = std::collections::HashSet::new();
                for p in array(get(r, "prepare")?)? {
                    occurrence(p, object, false)?;
                    if !seen.insert(jcs(p).expect("JSON occurrence")) {
                        return Err(invalid("duplicate complete preparation occurrence"));
                    }
                }
            }
        }
        Some("launch_copy") => {
            keys(
                v,
                &["kind", "manifest_hash", "copy_indices"],
                &["kind", "manifest_hash", "copy_indices"],
            )?;
            hash(get(v, "manifest_hash")?)?;
            let indices = array(get(v, "copy_indices")?)?;
            if indices.len() != image_names.len() {
                return Err(invalid("copy_indices must select one copy per image"));
            }
            let mut seen = std::collections::HashSet::new();
            for i in indices {
                if !seen.insert(uint(i)?) {
                    return Err(invalid("duplicate copy index"));
                }
            }
        }
        _ => return Err(invalid("unknown preparation witness kind")),
    }
    Ok(())
}
pub(super) fn restoration(v: &Value, claim_index: usize) -> Result<()> {
    match get(v, "kind")?.as_str() {
        Some("matrix_units") => keys(v, &["kind"], &["kind"]),
        Some("claim") => {
            keys(v, &["kind", "claim"], &["kind", "claim"])?;
            if index(get(v, "claim")?)? >= claim_index {
                return Err(invalid("restoration claim must precede the attached claim; circular dependencies are forbidden"));
            }
            Ok(())
        }
        _ => Err(invalid("unknown restoration witness kind")),
    }
}
