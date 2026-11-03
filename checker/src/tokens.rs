//! K91 equality partitions. Tuple equality never creates token identity.
use crate::{
    cert::{rej, u64_of},
    fault::Reject,
    json::Value,
};
use num_bigint::BigUint;

pub(crate) fn partition(
    value: Option<&Value>,
    tuples: &[[BigUint; 4]],
    has_token_args: bool,
    path: &str,
) -> Result<Vec<u64>, Vec<Reject>> {
    let bad = |message: &str| vec![rej("E_CERT_INVOCATION", "§12.3/K91", path, message)];
    let Some(value) = value else {
        // Published token-free positives predate the explicit field. There
        // is precisely one partition here; no equality choice is inferred.
        if !has_token_args && tuples.len() <= 1 {
            return Ok((0..tuples.len() as u64).collect());
        }
        return Err(bad(
            "token arguments require an explicit canonical token_aliases partition",
        ));
    };
    let labels = value.as_arr().ok_or_else(|| {
        bad("token_aliases must be a canonical array of nonnegative integer labels")
    })?;
    if labels.len() != tuples.len() {
        return Err(bad(
            "token_aliases arity differs from primary-oracle-token plus token-argument order",
        ));
    }
    let mut classes: Vec<&[BigUint; 4]> = vec![];
    let mut out = Vec::with_capacity(labels.len());
    for (value, tuple) in labels.iter().zip(tuples) {
        let label = u64_of(value)
            .ok_or_else(|| bad("token equality labels must be nonnegative integers"))?;
        if label > classes.len() as u64 {
            return Err(bad("token equality labels start at zero and introduce consecutive classes in first-occurrence order"));
        }
        if label == classes.len() as u64 {
            classes.push(tuple);
        } else if classes[label as usize] != tuple {
            return Err(bad(
                "equal token identity labels require exactly equal budget tuples",
            ));
        }
        out.push(label);
    }
    Ok(out)
}
