//! Verify the entire realised entry prefix and the conditional map, not a producer-selected subtrace.
use super::*;
use crate::coherent::Amp;
use crate::construction::{state::State, Call, Kind, Trace};
use crate::domain::Tuple;
use num_bigint::BigUint;
use num_traits::{One, Zero};

pub(super) fn construction_error(out: crate::cert::ClaimOutcome) -> Error {
    if out.errors.is_empty() {
        Error::Unavailable(
            out.unchecked_reason
                .unwrap_or_else(|| "linked construction unavailable".into()),
        )
    } else {
        Error::Rejected(out.errors)
    }
}
fn resolve<'a>(
    trace: &'a Trace,
    address: &Value,
) -> Result<(usize, &'a crate::construction::Event)> {
    let matches: Vec<_> = trace
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| &e.address == address)
        .collect();
    if matches.len() != 1 {
        return Err(invalid("preparation occurrence does not resolve to exactly one public leaf and allocation lifetime"));
    }
    Ok(matches[0])
}
fn bounded_work(elements: usize, events: usize, inputs: &Inputs) -> Result<()> {
    if (elements as u128) * (events as u128) > inputs.limits.max_reference_steps as u128 {
        return Err(Error::Unavailable("host limit: complete prefix/conditional matrix-column work exceeds reference work limit".into()));
    }
    Ok(())
}
fn coordinates(
    entry: &Value,
    trace: &Trace,
    call: &Call,
    entry_at: usize,
    images: &[FixedRegion],
) -> Result<()> {
    let regions = array(get(get(entry, "witness")?, "regions")?)?;
    for (region, image) in regions.iter().zip(images) {
        let (at, event) = resolve(trace, get(region, "allocation")?)?;
        let Kind::Allocate(bits) = &event.kind else {
            return Err(invalid("allocation occurrence does not acquire storage"));
        };
        if at >= entry_at || call.regions.get(image.position) != Some(bits) {
            return Err(invalid(
                "allocation lifetime is not the subject mem slot at invocation entry",
            ));
        }
        let mut previous = at;
        for occurrence in array(get(region, "prepare")?)? {
            let (at, _) = resolve(trace, occurrence)?;
            if at <= previous || at >= entry_at {
                return Err(invalid("preparation occurrences must precede entry in actual execution order after acquisition"));
            }
            previous = at;
        }
    }
    Ok(())
}
fn prefix(
    trace: &Trace,
    call: &Call,
    at: usize,
    images: &[FixedRegion],
    docs: &LoadedDocs,
    inputs: &Inputs,
) -> Result<()> {
    if trace.input_width >= usize::BITS as usize {
        return Err(Error::Unavailable(
            "host limit: root input matrix-column count exceeds usize".into(),
        ));
    }
    let dim = 1usize << trace.input_width;
    bounded_work(dim, at, inputs)?;
    for basis in 0..dim {
        let mut state = State::basis(basis.into());
        for event in &trace.events[..at] {
            state.apply(event, docs).map_err(construction_error)?;
        }
        for image in images {
            if !state.image(&call.regions[image.position], &image.cells, image.w) {
                return Err(invalid("realised prefix does not prepare the declared exact image factor on every root input column"));
            }
        }
    }
    Ok(())
}
fn basis(tuple: &Tuple, call: &Call, iface: &crate::object::Interface) -> BigUint {
    let mut value = BigUint::zero();
    let c = call.context.controls as usize;
    for (i, v) in tuple.bundles[..c].iter().enumerate() {
        if v.bit(0) {
            value |= BigUint::one() << i;
        }
    }
    for (v, bits) in tuple.bundles[c..].iter().zip(&call.bundles) {
        for (i, bit) in bits.iter().enumerate() {
            if v.bit(i as u64) {
                value |= BigUint::one() << *bit;
            }
        }
    }
    for ((cells, bits), region) in tuple.regions.iter().zip(&call.regions).zip(&iface.regions) {
        for (i, bit) in bits.iter().enumerate() {
            if cells[i / region.w as usize].bit((i % region.w as usize) as u64) {
                value |= BigUint::one() << *bit;
            }
        }
    }
    value
}
fn conditional_map(
    claim: &Value,
    call: &Call,
    images: &[FixedRegion],
    docs: &LoadedDocs,
    inputs: &Inputs,
) -> Result<()> {
    let trace = Trace::subject(docs, inputs, call.context.clone()).map_err(construction_error)?;
    let Kind::Enter(local) = &trace.events[0].kind else {
        unreachable!("subject trace starts with entry")
    };
    let symbol = docs
        .object
        .symbol(&call.context.symbol)
        .ok_or_else(|| invalid("subject absent"))?;
    let iface = docs
        .object
        .interface(&symbol.name)
        .map_err(Error::Unavailable)?;
    let domain =
        crate::domain::resolve_domain(get(claim, "domain")?, &iface, call.context.controls)
            .map_err(|e| match e {
                crate::fault::ResolveError::Reject(e) => invalid(e.message),
                crate::fault::ResolveError::HostLimit(e) => Error::Unavailable(e.quantity),
            })?;
    if !domain.covers_relative_to_image {
        return Err(Error::Unavailable("executable conditional relation needs every non-image component, never a sampled or narrowed domain".into()));
    }
    let count = domain.n.to_string().parse::<usize>().map_err(|_| {
        Error::Unavailable("host limit: conditional matrix-column count exceeds usize".into())
    })?;
    bounded_work(count, trace.events.len(), inputs)?;
    for before in domain.elements() {
        let mut state = State::basis(basis(&before, local, &iface));
        for event in &trace.events {
            state.apply(event, docs).map_err(construction_error)?;
            if let Kind::Leave(exiting) = &event.kind {
                if exiting.context.key == local.context.key {
                    for image in images {
                        if !state.image(&exiting.regions[image.position], &image.cells, image.w) {
                            return Err(invalid(
                                "invocation exit does not restore the exact image factor",
                            ));
                        }
                    }
                }
            }
        }
        for image in images {
            if !state.image(&local.regions[image.position], &image.cells, image.w) {
                return Err(invalid(
                    "realised conditional map fails exact image restoration/factorisation",
                ));
            }
        }
        let expected = crate::eval::eval_subject_tuple(
            &docs.object,
            symbol,
            &iface,
            &before,
            &call.context.invocation,
            call.context.controls as usize,
        )
        .map_err(|e| match e {
            crate::eval::EvalError::Reject(e) => Error::Rejected(vec![e]),
            crate::eval::EvalError::Unchecked(reason) => Error::Unavailable(reason),
        })?;
        let expected = basis(&expected, local, &iface);
        if state.amplitudes.len() != 1 || state.amplitudes.get(&expected) != Some(&Amp::one()) {
            return Err(invalid("realised conditional permutation map differs from the independently checked reference full tuple or exact +1 basis phase"));
        }
    }
    Ok(())
}
pub(super) fn check(
    entry: &Value,
    claim: &Value,
    docs: &LoadedDocs,
    inputs: &Inputs,
    images: &[FixedRegion],
) -> Result<String> {
    let trace = Trace::build(docs, inputs).map_err(construction_error)?;
    let matches: Vec<_> = trace
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, event)| match &event.kind {
            Kind::Enter(call)
                if Value::Arr(call.context.key.clone()) == *get(entry, "context_key").ok()? =>
            {
                Some((i, call))
            }
            _ => None,
        })
        .collect();
    if matches.len() != 1 {
        return Err(invalid(
            "attachment context does not resolve to exactly one linked invocation",
        ));
    }
    let (at, call) = matches[0];
    let symbol = docs
        .object
        .symbol(&call.context.symbol)
        .ok_or_else(|| invalid("reached subject absent"))?;
    if symbol.closure_hash.as_deref() != get(entry, "closure_hash")?.as_str() {
        return Err(invalid("context reaches another closure"));
    }
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
    .map_err(Error::Rejected)?;
    if supplied.args != call.context.invocation.args
        || supplied.budget != call.context.invocation.budget
        || supplied.token_aliases != call.context.invocation.token_aliases
    {
        return Err(invalid("nested attachment invocation differs from independently forwarded arguments/budget/token identities"));
    }
    let claimed = crate::cert::parse_invocation_for_kind(
        get(claim, "invocation")?,
        &iface.args,
        &docs.object,
        &symbol.kind,
        "claim.invocation",
    )
    .map_err(Error::Rejected)?;
    if claimed.args != supplied.args
        || claimed.budget != supplied.budget
        || claimed.token_aliases != supplied.token_aliases
    {
        return Err(invalid("claim invocation differs from attachment and independently reached arguments/budget/token identities"));
    }
    let subject = get(claim, "subject")?;
    if get(subject, "variant")?.as_str() != Some("forward")
        || schema::uint(get(subject, "inherited_controls")?)?
            != BigUint::from(call.context.controls)
    {
        return Err(invalid(
            "attachment subject variant or inherited controls differ from reached invocation",
        ));
    }
    coordinates(entry, &trace, call, at, images)?;
    prefix(&trace, call, at, images, docs, inputs)?;
    conditional_map(claim, call, images, docs, inputs)?;
    Ok("complete realised prefix proves the exact image factor on every root input column; the actual conditional map independently matches the full reference permutation with exact +1 phase and restores the image on every non-image basis column, hence also under arbitrary external references".into())
}
