//! Named schema, relation, table, program and enumeration phases for §12 replay.
use super::*;

pub(super) fn validate_shape(claim: &Value, kind: &str, path: &str) -> Result<(), ClaimOutcome> {
    let mut errs = Vec::new();
    let allowed: &[&str] = if kind == "sampled" {
        &[
            "kind",
            "subject",
            "domain",
            "expected",
            "sampler",
            "label",
            "invocation",
        ]
    } else {
        &[
            "kind",
            "subject",
            "domain",
            "expected",
            "label",
            "invocation",
        ]
    };
    let mut allowed = allowed.to_vec();
    if claim.get("expected").and_then(Value::as_str) == Some("borrow_frame") {
        allowed.push("borrow_frame");
    }
    let required: &[&str] = if kind == "sampled" {
        &["kind", "subject", "domain", "expected", "sampler"]
    } else {
        &["kind", "subject", "domain", "expected"]
    };
    if let Some(obj) = claim.as_obj() {
        for k in obj.keys() {
            if !allowed.contains(&k.as_str()) {
                errs.push(schema_err(
                    format!("{path}.{k}"),
                    format!("unknown field {k:?} in {kind} claim"),
                ));
            }
        }
        for r in required {
            if !obj.contains_key(*r) {
                errs.push(schema_err(
                    format!("{path}.{r}"),
                    format!("missing required field {r:?}"),
                ));
            }
        }
    } else {
        errs.push(schema_err(path.to_string(), "claim is not an object"));
    }
    if !errs.is_empty() {
        return Err(ClaimOutcome::rejected(errs));
    }
    Ok(())
}

#[derive(PartialEq)]
pub(super) enum Expected {
    Oracle,
    InversePair,
    BorrowFrame,
    Table(Vec<Value>),
    Program(String),
}

pub(super) fn parse_expected(claim: &Value, path: &str) -> Result<Expected, ClaimOutcome> {
    Ok(match claim.get("expected") {
        Some(Value::Str(x)) if x == "oracle" => Expected::Oracle,
        Some(Value::Str(x)) if x == "inverse_pair" => Expected::InversePair,
        Some(Value::Str(x)) if x == "borrow_frame" => Expected::BorrowFrame,
        Some(Value::Obj(m)) if m.len() == 1 && m.contains_key("table") => {
            match m.get("table").and_then(Value::as_arr) {
                Some(rows) => Expected::Table(rows.to_vec()),
                None => return Err(ClaimOutcome::rejected(vec![rej("E_CERT_TABLE", "§12.1", format!("{path}.expected.table"), "expected array of before/after rows")])),
            }
        }
        Some(Value::Obj(m)) if m.len() == 1 && m.contains_key("program") => {
            match m.get("program").and_then(Value::as_str).filter(|h| crate::is_hash_hex(h)) {
                Some(h) => Expected::Program(h.to_string()),
                None => {
                    return Err(ClaimOutcome::rejected(vec![schema_err(
                        format!("{path}.expected.program"),
                        "expected closure hash",
                    )]))
                }
            }
        }
        _ => {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_BINDING",
                "§12.1",
                format!("{path}.expected"),
                "expected must be \"borrow_frame\", \"oracle\", {table}, {program}, or (exhaustive) \"inverse_pair\"; there is no default relation",
            )]))
        }
    })
}

pub(super) fn table_rows(
    expected: &Expected,
    domain: &ResolvedDomain,
    path: &str,
) -> Result<Option<Vec<(Tuple, Tuple)>>, ClaimOutcome> {
    Ok(match &expected {
        Expected::Table(rows) => {
            if BigUint::from(rows.len()) != domain.n {
                return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_TABLE",
                    "§12.1",
                    format!("{path}.expected.table"),
                    format!(
                        "table has {} rows; the declared domain has {} elements",
                        rows.len(),
                        domain.n
                    ),
                )]));
            }
            // Both before/after tuples are retained. Validate the entire
            // declared table even when only a drawn subset will execute.
            if let Err(limit) = domain.check_retained_elements(&(BigUint::from(rows.len()) * 2u32))
            {
                return Err(host_limit_outcome(limit));
            }
            let mut parsed = Vec::new();
            for (i, (row, element)) in rows.iter().zip(domain.elements()).enumerate() {
                let mut errors = vec![];
                if !crate::cert::check_keys(
                    row,
                    &["before", "after"],
                    &["before", "after"],
                    &format!("{path}.expected.table[{i}]"),
                    &mut errors,
                ) {
                    for error in &mut errors {
                        error.code = "E_CERT_TABLE";
                    }
                    return Err(ClaimOutcome::rejected(errors));
                }
                let before = parse_tuple(
                    row.get("before").unwrap_or(&Value::Null),
                    &domain,
                    &format!("{path}.expected.table[{i}].before"),
                )
                .map_err(|e| ClaimOutcome::rejected(vec![e]))?;
                if before != element {
                    return Err(ClaimOutcome::rejected(vec![rej(
                        "E_CERT_TABLE",
                        "§12.1",
                        format!("{path}.expected.table[{i}].before"),
                        format!(
                            "row before {} is not declared-domain element {i} {}",
                            fmt_tuple(&before),
                            fmt_tuple(&element)
                        ),
                    )]));
                }
                let after = parse_tuple(
                    row.get("after").unwrap_or(&Value::Null),
                    &domain,
                    &format!("{path}.expected.table[{i}].after"),
                )
                .map_err(|e| ClaimOutcome::rejected(vec![e]))?;
                parsed.push((before, after));
            }
            Some(parsed)
        }
        _ => None,
    })
}

pub(super) fn bind_program(
    expected: &Expected,
    iface: &Interface,
    docs: &LoadedDocs,
    path: &str,
) -> Result<Option<(Symbol, Interface)>, ClaimOutcome> {
    Ok(match &expected {
        Expected::Program(h) => {
            let sym = match docs.object.symbol_by_closure(h) {
                Some(s) => s.clone(),
                None if docs.object.imports.is_empty() => return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING", "§12.1", format!("{path}.expected.program"),
                    "expected program closure is absent from the complete supplied object (no imports)",
                )])),
                None => {
                    return Err(ClaimOutcome::unchecked(
                        "expected program is not in the supplied object",
                    ))
                }
            };
            let piface = docs
                .object
                .interface(&sym.name)
                .map_err(|e| ClaimOutcome::unchecked(format!("expected program interface: {e}")))?;
            let shape = |i: &Interface| {
                (
                    i.bundles
                        .iter()
                        .map(|b| (b.group.clone(), b.width))
                        .collect::<Vec<_>>(),
                    i.regions
                        .iter()
                        .map(|r| (r.role.clone(), r.a, r.w))
                        .collect::<Vec<_>>(),
                    i.args.clone(),
                    i.borrowed_widths.clone(),
                )
            };
            if shape(&iface) != shape(&piface) {
                return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING",
                    "§12.1",
                    format!("{path}.expected.program"),
                    "expected program signature is not compatible with the subject (roles, widths, region shapes, argument types, borrowed widths)",
                )]));
            }
            Some((sym, piface))
        }
        _ => None,
    })
}

pub(super) fn select_elements(
    claim: &Value,
    kind: &str,
    domain: &ResolvedDomain,
    inputs: &Inputs,
    path: &str,
) -> Result<(Vec<Tuple>, Option<u64>, Option<usize>), ClaimOutcome> {
    Ok(if kind == "exhaustive" {
        if domain.n > BigUint::from(inputs.limits.max_elements) {
            return Err(host_limit_outcome(crate::fault::HostLimit {
                quantity: format!(
                    "domain element count {} (cap {})",
                    domain.n, inputs.limits.max_elements
                ),
            }));
        }
        if let Err(h) = domain.check_retained_elements(&domain.n) {
            return Err(host_limit_outcome(h));
        }
        (domain.elements().collect(), None, None)
    } else {
        let sampler = claim.get("sampler").cloned().unwrap_or(Value::Null);
        let mut errs = Vec::new();
        crate::cert::check_keys(
            &sampler,
            &["algorithm", "seed", "count"],
            &["algorithm", "seed", "count"],
            &format!("{path}.sampler"),
            &mut errs,
        );
        if sampler.get("algorithm").and_then(Value::as_str) != Some("xoshiro256**") {
            errs.push(rej(
                "E_CERT_SAMPLER",
                "§12.1",
                format!("{path}.sampler.algorithm"),
                "algorithm must be \"xoshiro256**\"",
            ));
        }
        let seed: Vec<String> = sampler
            .get("seed")
            .and_then(Value::as_arr)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if seed.len() != 4
            || sampler.get("seed").and_then(Value::as_arr).map(|a| a.len()) != Some(4)
        {
            errs.push(rej(
                "E_CERT_SAMPLER",
                "§12.1",
                format!("{path}.sampler.seed"),
                "seed must be 4 decimal strings",
            ));
        }
        let count = sampler
            .get("count")
            .and_then(|v| {
                v.as_str()
                    .and_then(crate::rat::parse_biguint_str)
                    .or_else(|| u64_of(v).map(BigUint::from))
            })
            .unwrap_or_else(|| {
                errs.push(rej(
                    "E_CERT_SAMPLER",
                    "§12.1",
                    format!("{path}.sampler.count"),
                    "count must be a nonnegative integer",
                ));
                BigUint::zero()
            });
        if !errs.is_empty() {
            return Err(ClaimOutcome::rejected(errs));
        }
        if let Err(e) = crate::sampler::Sampler::new([&seed[0], &seed[1], &seed[2], &seed[3]]) {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_SAMPLER",
                "§12.1",
                format!("{path}.sampler.seed"),
                e,
            )]));
        }
        if count > BigUint::from(inputs.limits.max_elements) {
            return Err(host_limit_outcome(crate::fault::HostLimit {
                quantity: format!("sampler count {count} (cap {})", inputs.limits.max_elements),
            }));
        }
        if let Err(h) = domain.check_retained_elements(&count) {
            return Err(host_limit_outcome(h));
        }
        let count = count
            .to_string()
            .parse::<u64>()
            .expect("count was bounded by the u64 host limit");
        let draws = match crate::sampler::draw_elements(
            &domain,
            [&seed[0], &seed[1], &seed[2], &seed[3]],
            count,
        ) {
            Ok(d) => d,
            Err(ResolveError::Reject(r)) => return Err(ClaimOutcome::rejected(vec![r.at(path)])),
            Err(ResolveError::HostLimit(h)) => return Err(host_limit_outcome(h)),
        };
        let distinct: std::collections::HashSet<String> = draws.iter().map(fmt_tuple).collect();
        (draws, Some(count), Some(distinct.len()))
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_outcome(
    kind: &str,
    domain: &ResolvedDomain,
    invocation: &Invocation,
    subject: &crate::cert::Subject,
    scheduled: bool,
    requested_label: Option<String>,
    draws_count: Option<u64>,
    distinct_count: Option<usize>,
    path: &str,
) -> Result<ClaimOutcome, ClaimOutcome> {
    let (computed, elements_s, fraction) = if kind == "exhaustive" {
        (
            if domain.covers_full {
                "proof"
            } else if domain.covers_relative_to_image {
                "proof_relative_to_image"
            } else {
                "evidence"
            },
            domain.n.to_string(),
            domain.covered_fraction(),
        )
    } else {
        (
            "evidence",
            draws_count.unwrap_or(0).to_string(),
            fraction_string(&BigUint::from(distinct_count.unwrap_or(0)), &domain.full_n),
        )
    };
    let mut out = ClaimOutcome {
        outcome: computed.into(),
        establishes: String::new(),
        elements: Some(elements_s.clone()),
        draws: draws_count.map(|d| d.to_string()),
        distinct: distinct_count.map(|d| d.to_string()),
        covered_fraction: Some(fraction.clone()),
        token_aliases: Some(invocation.token_aliases.clone()),
        image_bindings: if domain.fixed_regions.is_empty() {
            None
        } else {
            let regions = domain
                .fixed_regions
                .iter()
                .map(|r| {
                    Value::Obj(std::collections::BTreeMap::from([
                        (
                            "identity".into(),
                            Value::Arr(vec![Value::Str("mem".into()), crate::json::n(r.position)]),
                        ),
                        ("A".into(), crate::json::n(r.a)),
                        ("W".into(), crate::json::n(r.w)),
                        ("sha256".into(), Value::Str(r.sha256.clone())),
                    ]))
                })
                .collect();
            Some(Value::Obj(std::collections::BTreeMap::from([
                ("fixed_regions".into(), Value::Arr(regions)),
                ("context_key".into(), Value::Arr(vec![])),
                (
                    "invocation".into(),
                    Value::Obj(std::collections::BTreeMap::from([
                        ("args".into(), Value::Arr(invocation.args.clone())),
                        (
                            "budget".into(),
                            Value::Arr(
                                invocation
                                    .budget
                                    .iter()
                                    .map(|n| Value::Str(n.to_string()))
                                    .collect(),
                            ),
                        ),
                        (
                            "token_aliases".into(),
                            Value::Arr(
                                invocation
                                    .token_aliases
                                    .iter()
                                    .map(crate::json::n)
                                    .collect(),
                            ),
                        ),
                    ])),
                ),
            ])))
        },
        ..Default::default()
    };
    out.establishes = if computed == "proof" {
        format!(
            "every element of D ({elements_s} tuples) was replayed and full output tuples agree with the expected relation under the subject {} variant {:?} with {} inherited control(s); by §12.2 this is proof (and the §14.3 T3 premise for permutation subjects)",
            if scheduled { "realised (schedule-bound, exact) " } else { "" },
            subject.variant,
            subject.inherited_controls
        )
    } else if computed == "proof_relative_to_image" {
        format!("all {elements_s} tuples of every remaining unrestricted component were replayed with the independently bound fixed image(s); this is a conditional proof_relative_to_image theorem with exact +1 phase, full outputs, input-image restoration and clean workspace, covering {fraction} of unrestricted D; executable preparation and entry factorisation are separate obligations, not established by this theorem")
    } else if kind == "exhaustive" {
        format!("all {elements_s} domain elements were replayed and agree; this is evidence over a strict slice of D (covered fraction {fraction}), never proof")
    } else {
        format!(
            "{} sampled draws ({} distinct) were replayed on both sides and agree; sampled success is always evidence (distinct covered fraction {fraction}), even at full coverage",
            draws_count.unwrap_or(0),
            distinct_count.unwrap_or(0)
        )
    };
    if requested_label.as_deref().is_some_and(|label| {
        matches!(label, "proof" | "proof_relative_to_image") && label != computed
    }) {
        let mut rejected = ClaimOutcome::rejected(vec![Reject::new(
            "E_CERT_COVERAGE_LABEL",
            "§12.2",
            format!("{path}.label"),
            format!("producer requested {}; the checker-computed outcome is {computed} (fraction {fraction})",requested_label.unwrap()),
        )]);
        rejected.elements = out.elements;
        rejected.draws = out.draws;
        rejected.distinct = out.distinct;
        rejected.covered_fraction = out.covered_fraction;
        rejected.establishes = out.establishes;
        rejected.image_bindings = out.image_bindings;
        rejected.token_aliases = out.token_aliases;
        return Err(rejected);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn schema_phase_rejects_missing_and_unknown_fields() {
        let valid =
            parse(r#"{"kind":"exhaustive","subject":{},"domain":{},"expected":"oracle"}"#).unwrap();
        assert!(validate_shape(&valid, "exhaustive", "claim").is_ok());
        for invalid in [
            r#"{"kind":"exhaustive","subject":{},"domain":{}}"#,
            r#"{"kind":"exhaustive","subject":{},"domain":{},"expected":"oracle","extra":0}"#,
        ] {
            let outcome =
                validate_shape(&parse(invalid).unwrap(), "exhaustive", "claim").unwrap_err();
            assert_eq!(outcome.outcome, "reject");
            assert!(outcome.errors.iter().all(|e| e.code == "E_CERT_SCHEMA"));
        }
    }

    #[test]
    fn relation_phase_never_invents_an_expected_relation() {
        for invalid in [
            r#"{}"#,
            r#"{"expected":null}"#,
            r#"{"expected":{"table":0}}"#,
            r#"{"expected":{"program":"not-a-hash"}}"#,
        ] {
            assert!(parse_expected(&parse(invalid).unwrap(), "claim").is_err());
        }
        let claim = parse(r#"{"expected":"borrow_frame"}"#).unwrap();
        assert!(matches!(
            parse_expected(&claim, "claim"),
            Ok(Expected::BorrowFrame)
        ));
    }
}
