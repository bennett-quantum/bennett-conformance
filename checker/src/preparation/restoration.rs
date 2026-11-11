//! A restoration index is a checked request, not evidence by itself (§12.3).
use super::*;
pub(super) fn check(
    entry: &Value,
    claim: &Value,
    claims: &[Value],
    images: &[FixedRegion],
    iface: &crate::object::Interface,
    docs: &LoadedDocs,
    reports: &[crate::cert::RegionReport],
) -> Result<()> {
    let restoration = get(entry, "restoration")?;
    if get(restoration, "kind")?.as_str() != Some("claim") {
        return Ok(());
    }
    let i = index(get(restoration, "claim")?)?;
    let previous = claims
        .get(i)
        .ok_or_else(|| invalid("restoration claim index is absent"))?;
    if get(previous, "kind")?.as_str() != Some("exhaustive") {
        return Err(invalid(
            "sampled, approximate or structural substitutes do not prove exact restoration",
        ));
    }
    if get(previous, "subject")? != get(claim, "subject")? {
        return Err(invalid(
            "restoration claim names another subject/variant/control/schedule",
        ));
    }
    let schedule = get(previous, "subject")?
        .get("schedule_hash")
        .and_then(Value::as_str);
    if schedule.is_none() || schedule != docs.schedule_hash.as_deref() {
        return Err(invalid(
            "a reference-only theorem does not establish restoration of the realised map",
        ));
    }
    if previous.get("invocation") != claim.get("invocation") {
        return Err(invalid("restoration claim invocation differs"));
    }
    if !same_images(images, &claim_images(previous, iface)?) {
        return Err(invalid("restoration claim image bindings differ"));
    }
    let closure = hash(get(entry, "closure_hash")?)?;
    let outcome = reports
        .iter()
        .find(|r| r.closure_hash == closure)
        .and_then(|r| r.claims.get(i))
        .ok_or_else(|| invalid("restoration claim has no checked report"))?;
    if outcome.0 != "exhaustive"
        || !matches!(
            outcome.1.outcome.as_str(),
            "proof" | "proof_relative_to_image"
        )
    {
        return Err(Error::Unavailable("restoration claim has not established the required exact complete conditional relation; applicability unestablished".into()));
    }
    Ok(())
}
