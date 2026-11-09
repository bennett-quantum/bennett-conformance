//! Conditional theorem applicability: ISA §12.3 image-preparation/1.
//! Descriptor validity and independently checked entry/exit premises are separate.
use crate::cert::{Inputs, LoadedDocs, Verdict};
use crate::domain::FixedRegion;
use crate::json::{jcs, s, Value};
use std::collections::BTreeMap;
mod launch;
mod logical;
mod restoration;
mod schema;
use schema::{array, get, hash, index, keys};

type Result<T> = std::result::Result<T, Error>;
enum Error {
    Rejected(Vec<crate::fault::Reject>),
    Unavailable(String),
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Rejected(vec![crate::cert::rej(
        "E_CERT_PREPARATION",
        "§12.3",
        "bindings.preparation",
        message.into(),
    )])
}
fn images(
    v: &Value,
    iface: &crate::object::Interface,
    canonical: bool,
) -> Result<Vec<FixedRegion>> {
    let values = array(v)?;
    let domain = Value::Obj(BTreeMap::from([("fixed_regions".into(), v.clone())]));
    let resolved = crate::domain::resolve_fixed_images(&domain, iface).map_err(|e| match e {
        crate::fault::ResolveError::Reject(e) => invalid(e.message),
        crate::fault::ResolveError::HostLimit(e) => Error::Unavailable(e.quantity),
    })?;
    if canonical {
        if values.is_empty() {
            return Err(invalid("attachment images must be nonempty"));
        }
        let names: Vec<_> = resolved
            .iter()
            .map(|r| iface.regions[r.position].name.as_str())
            .collect();
        if !values
            .iter()
            .zip(names)
            .all(|(v, name)| v.get("name").and_then(Value::as_str) == Some(name))
        {
            return Err(invalid(
                "attachment images must be in subject interface mem-slot order",
            ));
        }
    }
    Ok(resolved)
}
fn claim_images(claim: &Value, iface: &crate::object::Interface) -> Result<Vec<FixedRegion>> {
    let domain = get(claim, "domain")?;
    let fixed = get(domain, "fixed_regions")?;
    images(fixed, iface, false)
}
fn same_images(a: &[FixedRegion], b: &[FixedRegion]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.position == b.position
                && a.a == b.a
                && a.w == b.w
                && a.sha256 == b.sha256
                && a.cells == b.cells
        })
}
fn concrete_invocation(
    entry: &Value,
    claim: &Value,
    symbol: &crate::object::Symbol,
    docs: &LoadedDocs,
) -> Result<()> {
    let iface = docs
        .object
        .interface(&symbol.name)
        .map_err(Error::Unavailable)?;
    let supplied = crate::cert::parse_invocation_for_kind(
        get(entry, "invocation")?,
        &iface.args,
        &docs.object,
        &symbol.kind,
        "bindings.preparation.invocation",
    )
    .map_err(|mut errors| {
        for e in &mut errors {
            e.code = "E_CERT_PREPARATION";
        }
        Error::Rejected(errors)
    })?;
    let root = get(entry, "context_key")?
        .as_arr()
        .is_some_and(|a| a.is_empty());
    if root && symbol.closure_hash.as_deref() != Some(docs.root_closure.as_str()) {
        return Err(invalid(
            "empty context key denotes the bound device root, not another closure",
        ));
    }
    let bound = if root
        && claim
            .get("subject")
            .and_then(|s| s.get("schedule_hash"))
            .is_some()
    {
        docs.root_invocation
            .clone()
            .ok_or_else(|| Error::Unavailable("root invocation unavailable".into()))?
    } else if root {
        crate::cert::parse_invocation_for_kind(
            get(claim, "invocation")?,
            &iface.args,
            &docs.object,
            &symbol.kind,
            "claim.invocation",
        )
        .map_err(Error::Rejected)?
    } else {
        return Err(Error::Unavailable("nested preparation invocation/context resolution needs the complete linked construction".into()));
    };
    if supplied.args != bound.args
        || supplied.budget != bound.budget
        || supplied.token_aliases != bound.token_aliases
    {
        return Err(invalid(
            "attachment invocation differs from independently bound args/budget/token_aliases",
        ));
    }
    Ok(())
}
fn attachment(
    entry: &Value,
    cert: &Value,
    docs: &LoadedDocs,
    libraries: &[Value],
    inputs: &Inputs,
    reports: &[crate::cert::RegionReport],
) -> Result<String> {
    let fields = [
        "closure_hash",
        "claim",
        "context_key",
        "invocation",
        "images",
        "witness",
        "restoration",
    ];
    keys(entry, &fields, &fields)?;
    let closure = hash(get(entry, "closure_hash")?)?;
    let symbol = docs
        .object
        .symbol_by_closure(closure)
        .ok_or_else(|| invalid("attachment closure is absent from the selected device object"))?;
    let record = get(get(cert, "regions")?, closure)?;
    let claims = array(get(record, "claims")?)?;
    let ci = index(get(entry, "claim")?)?;
    let claim = claims
        .get(ci)
        .ok_or_else(|| invalid("attachment claim index is out of range"))?;
    if get(get(claim, "subject")?, "closure_hash")?.as_str() != Some(closure) {
        return Err(invalid("attachment claim subject names another closure"));
    }
    schema::context(get(entry, "context_key")?, &docs.object)?;
    let iface = docs
        .object
        .interface(&symbol.name)
        .map_err(Error::Unavailable)?;
    let bound = images(get(entry, "images")?, &iface, true)?;
    if !same_images(&bound, &claim_images(claim, &iface)?) {
        return Err(invalid(
            "attachment image bindings differ from the claim after mem-slot resolution",
        ));
    }
    let names: Vec<_> = bound
        .iter()
        .map(|i| iface.regions[i.position].name.clone())
        .collect();
    schema::witness(get(entry, "witness")?, &docs.object, &names)?;
    schema::restoration(get(entry, "restoration")?, ci)?;
    restoration::check(entry, claim, claims, &bound, &iface, docs, reports)?;
    if get(entry, "context_key")?
        .as_arr()
        .is_some_and(|v| v.is_empty())
    {
        concrete_invocation(entry, claim, symbol, docs)?;
    }
    match get(get(entry, "witness")?, "kind")?.as_str() {
        Some("launch_copy") => launch::check(entry, cert, docs, libraries, &bound),
        Some("logical") => logical::check(entry, claim, docs, inputs, &bound),
        _ => unreachable!("witness kind was validated"),
    }
}
pub(crate) fn check(
    cert: &Value,
    docs: &LoadedDocs,
    inputs: &Inputs,
    libraries: &[Value],
    verdict: &mut Verdict,
) {
    let Some(descriptor) = cert.get("bindings").and_then(|b| b.get("preparation")) else {
        return;
    };
    let result = (|| -> Result<()> {
        let version = get(descriptor, "version")?
            .as_str()
            .ok_or_else(|| invalid("preparation version must be a string"))?;
        if version != "bennett.image-preparation/1" {
            return Err(Error::Unavailable(
                "unsupported preparation version; applicability unestablished".into(),
            ));
        }
        keys(descriptor, &["version", "entries"], &["version", "entries"])?;
        let mut previous = None;
        for entry in array(get(descriptor, "entries")?)? {
            let key = (
                hash(get(entry, "closure_hash")?)?.to_string(),
                schema::uint(get(entry, "claim")?)?,
                jcs(get(entry, "context_key")?).expect("JSON context key"),
            );
            if previous.as_ref().is_some_and(|p| p >= &key) {
                return Err(invalid("attachments must be sorted by closure, numeric claim and JCS context, with no duplicate triple"));
            }
            previous = Some(key);
            let outcome = attachment(entry, cert, docs, libraries, inputs, &verdict.regions);
            let (status, reason) = match &outcome {
                Ok(reason) => ("established", reason.clone()),
                Err(Error::Unavailable(reason)) => ("unestablished", reason.clone()),
                Err(Error::Rejected(_)) => (
                    "rejected",
                    "malformed or refuted preparation binding".into(),
                ),
            };
            verdict.preparation.push(Value::Obj(BTreeMap::from([
                ("closure_hash".into(), get(entry, "closure_hash")?.clone()),
                ("claim".into(), get(entry, "claim")?.clone()),
                ("context_key".into(), get(entry, "context_key")?.clone()),
                ("applicability".into(), s(status)),
                ("reason".into(), s(&reason)),
            ])));
            match outcome {
                Err(Error::Rejected(errors)) => verdict.errors.extend(errors),
                Err(Error::Unavailable(reason)) => verdict.unchecked_items.push(reason),
                Ok(_) => {}
            }
        }
        Ok(())
    })();
    match result {
        Err(Error::Rejected(errors)) => verdict.errors.extend(errors),
        Err(Error::Unavailable(reason)) => verdict.unchecked_items.push(reason),
        Ok(()) => {}
    }
}
