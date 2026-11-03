//! K90 descriptor selection and all-context validation, from ISA §§9.4/12.3.
use crate::cert::{check_keys, rej, u64_of, ClaimOutcome, Inputs, LoadedDocs, RegionReport};
use crate::json::{jcs, Value};
use crate::object::Symbol;
use crate::rat::Rat;
use crate::realisation::Context;
use std::collections::HashMap;

fn error(path: &str, message: &str) -> crate::fault::Reject {
    rej("E_CERT_FRAME", "§9.4/§12.3/K90", path, message)
}
fn keys(
    v: &Value,
    allowed: &[&str],
    required: &[&str],
    path: &str,
    errors: &mut Vec<crate::fault::Reject>,
) -> bool {
    let start = errors.len();
    let valid = check_keys(v, allowed, required, path, errors);
    for e in &mut errors[start..] {
        e.code = "E_CERT_FRAME";
    }
    valid
}
pub(crate) fn key_schema(key: &Value, path: &str, errors: &mut Vec<crate::fault::Reject>) {
    let Some(chain) = key.as_arr() else {
        errors.push(error(
            path,
            "context_key must be a concrete invocation chain",
        ));
        return;
    };
    for (i, edge) in chain.iter().enumerate() {
        let path = format!("{path}[{i}]");
        if !keys(
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
            &path,
            errors,
        ) {
            continue;
        }
        let point_ok = edge
            .get("point")
            .and_then(Value::as_str)
            .and_then(|p| p.rsplit_once(':'))
            .is_some_and(|(b, n)| {
                !b.is_empty() && b != "*" && crate::rat::is_canonical_uint_str(n)
            });
        let arrays_ok = ["iteration", "instance"].iter().all(|k| {
            edge.get(k)
                .and_then(Value::as_arr)
                .is_some_and(|a| a.iter().all(|v| u64_of(v).is_some()))
        });
        let phase_ok = match (edge.get("step"), edge.get("phase")) {
            (None, None) => true,
            (Some(n), Some(p)) => {
                u64_of(n).is_some_and(|n| n > 0) && matches!(p.as_str(), Some("fwd" | "rev"))
            }
            _ => false,
        };
        if !edge
            .get("symbol")
            .and_then(Value::as_str)
            .is_some_and(crate::is_hash_hex)
            || !point_ok
            || !arrays_ok
            || edge.get("call_index").and_then(u64_of).is_none()
            || !phase_ok
        {
            errors.push(error(&path,"context edge requires a hash, source point, concrete index arrays and call occurrence; oracle step/phase occur together"));
        }
    }
}

pub(crate) fn selected<'a>(
    record: &'a Value,
    context: &Context,
) -> Result<&'a Value, ClaimOutcome> {
    if let Some(frame) = record.get("frame") {
        return Ok(frame);
    }
    let key = Value::Arr(context.key.clone());
    record
        .get("frame_contexts")
        .and_then(Value::as_arr)
        .and_then(|a| a.iter().find(|e| e.get("context_key") == Some(&key)))
        .and_then(|e| e.get("frame"))
        .ok_or_else(|| {
            ClaimOutcome::rejected(vec![error(
                "frame_contexts",
                "invocation has no matching frame descriptor",
            )])
        })
}

pub(crate) fn check(
    region: &Value,
    symbol: Option<&Symbol>,
    docs: &LoadedDocs,
    inputs: &Inputs,
    base: &str,
    report: &mut RegionReport,
) {
    let frame = region.get("frame");
    let contexts = region.get("frame_contexts");
    match (frame, contexts) {
        (None, None) => {
            report.errors.push(rej(
                "E_CERT_SCHEMA",
                "§12.3",
                base,
                "exactly one of frame and frame_contexts is required",
            ));
            return;
        }
        (Some(_), Some(_)) => {
            report.errors.push(error(
                base,
                "frame and frame_contexts are exclusive alternatives",
            ));
            return;
        }
        _ => (),
    }
    let start_errors = report.errors.len();
    if let Some(frame) = frame {
        crate::numeric::check_frame(frame, symbol, docs, base, report);
    }
    if let Some(value) = contexts {
        let Some(entries) = value.as_arr() else {
            report.errors.push(error(
                &format!("{base}.frame_contexts"),
                "expected an array of context descriptors",
            ));
            return;
        };
        if entries.is_empty() {
            report
                .errors
                .push(error(base, "frame_contexts cannot be empty"));
            return;
        }
        let mut previous = None;
        for (i, entry) in entries.iter().enumerate() {
            let path = format!("{base}.frame_contexts[{i}]");
            if !keys(
                entry,
                &["context_key", "frame"],
                &["context_key", "frame"],
                &path,
                &mut report.errors,
            ) {
                continue;
            }
            let key = entry.get("context_key").unwrap();
            key_schema(key, &format!("{path}.context_key"), &mut report.errors);
            let encoded = jcs(key).expect("parsed JSON");
            if previous.as_ref().is_some_and(|p| p >= &encoded) {
                report.errors.push(error(
                    &path,
                    "context records must have distinct keys in increasing bytewise JCS order",
                ));
            }
            previous = Some(encoded);
            crate::numeric::check_frame(entry.get("frame").unwrap(), symbol, docs, &path, report);
        }
        if docs.schedule.is_none() {
            report.errors.push(error(
                base,
                "a standalone reference record requires frame for its bound invocation",
            ));
        }
    }
    if report.errors.len() != start_errors || docs.schedule.is_none() {
        return;
    }
    let Some(symbol) = symbol else {
        return;
    };
    let reached = match crate::realisation::contexts(docs, inputs) {
        Ok(all) => all
            .into_iter()
            .filter(|c| c.symbol == symbol.name)
            .collect::<Vec<_>>(),
        Err(out) => {
            report.errors.extend(out.errors);
            report.unchecked_items.push(format!(
                "frame invocation coverage/uniformity remains unchecked: {}",
                out.unchecked_reason
                    .unwrap_or_else(|| "source expansion failed".into())
            ));
            return;
        }
    };
    if let Some(entries) = contexts.and_then(Value::as_arr) {
        if entries.len() != reached.len()
            || entries
                .iter()
                .zip(&reached)
                .any(|(e, c)| e.get("context_key") != Some(&Value::Arr(c.key.clone())))
        {
            report.errors.push(error(base,"frame_contexts must cover exactly the reached invocation keys in canonical order, including default-covered and predicated-off calls"));
            return;
        }
    }
    let Ok(iface) = docs.object.interface(&symbol.name) else {
        return;
    };
    for context in &reached {
        let descriptor = selected(region, context).expect("validated context coverage");
        for (slot, bundle) in descriptor
            .get("slots")
            .and_then(Value::as_arr)
            .unwrap_or(&[])
            .iter()
            .zip(&iface.bundles)
        {
            let vars = HashMap::from([("c", Rat::from_integer(context.controls.into()))]);
            match crate::expressions::evaluate(slot.get("width").unwrap(), &vars, &Value::Null) {
                Ok(width) if width == Rat::from_integer(bundle.width.into()) => (),
                Ok(_) => report.errors.push(error(
                    base,
                    "frame slot width does not equal the interface in every reached invocation",
                )),
                Err(crate::expressions::Error::Invalid(message)) => {
                    report.errors.push(error(base, &message))
                }
                Err(crate::expressions::Error::HostLimit(limit)) => report.unchecked_items.push(
                    format!("host limit: frame width evaluation: {}", limit.quantity),
                ),
            }
        }
    }
    if !reached.is_empty() && report.errors.len() == start_errors {
        report
            .unchecked_items
            .retain(|s| !s.starts_with("symbolic frame slot widths have not been evaluated"));
        report.establishes.push(format!("frame descriptors selected and slot widths checked for all {} reached invocation(s); dynamic storage/lifetime obligations remain separate (§12.3/K90)",reached.len()));
    }
}
