//! Selected §12.1/§12.2 borrow frames. Schema admission grants no proof:
//! normative entry domains, occurrence identities and tensor factors remain
//! obligations until they can be derived independently.
use super::*;
use crate::cert::{check_keys, Subject};

#[allow(clippy::too_many_arguments)]
pub(super) fn check_claim(
    claim: &Value,
    path: &str,
    subject: &Subject,
    symbol: &Symbol,
    iface: &Interface,
    docs: &LoadedDocs,
    root_invocation: &Option<Invocation>,
    cert_schedule_hash: Option<&str>,
) -> Result<ClaimOutcome, ClaimOutcome> {
    let mut errors = vec![];
    let frame = claim.get("borrow_frame").unwrap_or(&Value::Null);
    check_keys(
        frame,
        &["lifetime", "first", "last", "context_key"],
        &["lifetime", "first", "last"],
        &format!("{path}.borrow_frame"),
        &mut errors,
    );
    if let Some(lifetime) = frame.get("lifetime") {
        crate::domain::biguint_of_json(lifetime, "borrow_frame.lifetime").map_err(|e| match e {
            ResolveError::Reject(r) => ClaimOutcome::rejected(vec![r.at(path)]),
            ResolveError::HostLimit(h) => host_limit_outcome(h),
        })?;
    }
    let scheduled = subject.schedule_hash.is_some();
    if scheduled && subject.schedule_hash.as_deref() != cert_schedule_hash {
        errors.push(rej(
            "E_CERT_BINDING",
            "§12.3",
            path,
            "borrow-frame subject schedule does not match certificate binding",
        ));
    }
    if scheduled {
        match frame.get("context_key").and_then(Value::as_arr) {
            None => errors.push(schema_err(
                format!("{path}.borrow_frame.context_key"),
                "schedule-bound borrow frame requires a resolved invocation key",
            )),
            Some(key) if subject.closure_hash == docs.root_closure && !key.is_empty() => errors
                .push(rej(
                    "E_CERT_BINDING",
                    "§12.1",
                    path,
                    "root borrow-frame context_key must be []",
                )),
            Some(_) => (),
        }
    }
    match claim.get("invocation") {
        Some(v) => {
            let inv = parse_invocation_for_kind(
                v,
                &iface.args,
                &docs.object,
                &symbol.kind,
                &format!("{path}.invocation"),
            )
            .map_err(ClaimOutcome::rejected)?;
            if scheduled && subject.closure_hash == docs.root_closure {
                if let Some(root) = root_invocation {
                    if inv.args != root.args
                        || inv.budget != root.budget
                        || inv.token_aliases != root.token_aliases
                    {
                        errors.push(rej(
                            "E_CERT_BINDING",
                            "§12.3",
                            path,
                            "borrow-frame invocation conflicts with root binding",
                        ));
                    }
                }
            }
        }
        None if !scheduled => errors.push(rej(
            "E_CERT_INVOCATION",
            "§12.3",
            path,
            "standalone borrow-frame claim requires explicit invocation",
        )),
        None => (),
    }
    if !errors.is_empty() {
        return Err(ClaimOutcome::rejected(errors));
    }
    // Do not run an ordinary interface-domain proof on a selected frame.
    // In particular, workspace and entry_products describe frame-entry
    // states, which resolve_domain intentionally cannot certify.
    Ok(ClaimOutcome::unchecked(
        "selected borrow-frame occurrence resolution, normative entry domain (workspace/entry_products), sampler and tensor-factor replay have not been independently derived (§12.1/§12.2)",
    ))
}
