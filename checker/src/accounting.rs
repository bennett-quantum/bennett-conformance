//! Recompute certificate reports from independently expanded native leaf
//! events (§8.2) and invocation contexts (§12.3). No producer report supplies
//! an input to event counts, timing, wire occupancy or multiplicities.
use crate::cert::{inner_key, key_of, rej, u64_of, ClaimOutcome, Inputs, LoadedDocs, Verdict};
use crate::json::Value;
use crate::rat::{rat_of_json, Rat};
use crate::realisation::{Context, Gate};
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, HashMap, HashSet};
type Result<T> = std::result::Result<T, ClaimOutcome>;
fn bad<T>(code: &'static str, path: impl AsRef<str>, message: impl AsRef<str>) -> Result<T> {
    Err(ClaimOutcome::rejected(vec![rej(
        code,
        "§8.2/§12.3",
        path,
        message,
    )]))
}
fn pending<T>(message: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::unchecked(message))
}
fn decimal(v: Option<&Value>, path: &str) -> Result<Rat> {
    v.and_then(|v| rat_of_json(v).ok()).ok_or_else(|| {
        ClaimOutcome::rejected(vec![rej(
            "E_CERT_SCHEMA",
            "§12.3",
            path,
            "expected an exact decimal",
        )])
    })
}
fn context_hash<'a>(c: &Context, docs: &'a LoadedDocs) -> &'a str {
    docs.object
        .symbol(&c.symbol)
        .and_then(|s| s.closure_hash.as_deref())
        .expect("expanded context is linked")
}
fn expr(v: &Value, vars: &HashMap<&str, Rat>, tables: &Value, path: &str) -> Result<Rat> {
    crate::expressions::evaluate(v, vars, tables).map_err(|error| match error {
        crate::expressions::Error::Invalid(message) => {
            ClaimOutcome::rejected(vec![rej("E_CERT_TARGET", "§8.1", path, message)])
        }
        crate::expressions::Error::HostLimit(limit) => crate::cert::host_limit_outcome(limit),
    })
}
struct Event<'a> {
    gate: &'a Gate,
    start: Rat,
    end: Rat,
    before: Rat,
    cost: BTreeMap<String, Rat>,
}
fn events<'a>(gates: &'a [Gate], docs: &LoadedDocs) -> Result<Vec<Event<'a>>> {
    let mut cursor = Rat::zero();
    let mut wire_ends: HashMap<usize, Rat> = HashMap::new();
    let mut out = vec![];
    for gate in gates {
        let entry = crate::realisation::native(docs, gate)?;
        let decision = crate::realisation::decision_kind(docs, gate, "gate")?;
        let eps = decision
            .and_then(|d| d.get("epsilon_share"))
            .map(|v| decimal(Some(v), "schedule.gate.epsilon_share"))
            .transpose()?
            .unwrap_or_else(Rat::zero);
        let own = gate.own_controls;
        let vars = HashMap::from([
            ("k", Rat::from_integer(own.into())),
            ("c", Rat::from_integer(gate.controls.len().into())),
            ("eps", eps),
        ]);
        let costs = entry.get("cost").and_then(Value::as_obj).ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_TARGET",
                "§8.3",
                "target.gates.cost",
                "native gate lacks its cost object",
            )])
        })?;
        let mut cost = BTreeMap::new();
        for (name, value) in costs {
            let r = expr(
                value,
                &vars,
                docs.target
                    .as_ref()
                    .and_then(|t| t.get("tables"))
                    .unwrap_or(&Value::Null),
                "target.gates.cost",
            )?;
            if r.is_negative() {
                return bad(
                    "E_CERT_TARGET",
                    "target.gates.cost",
                    "gate cost is negative",
                );
            }
            cost.insert(name.clone(), r);
        }
        let duration = cost.get("time").cloned().ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_TARGET",
                "§8.2",
                "target.gates.cost.time",
                "native gate has no duration",
            )])
        })?;
        let selected = crate::realisation::decision_kind(docs, gate, "start")?;
        let start = match selected {
            Some(s) => decimal(s.get("time"), "schedule.start.time")?,
            None => cursor.clone(),
        };
        if start.is_negative() {
            return bad(
                "E_SCHEDULE_INFEASIBLE",
                "schedule.start.time",
                "negative gate start time",
            );
        }
        for wire in gate.controls.iter().chain(&gate.targets) {
            if wire_ends.get(wire).map(|end| end > &start).unwrap_or(false) {
                return bad(
                    "E_SCHEDULE_INFEASIBLE",
                    "schedule.start.time",
                    "gate starts before a dependency on one of its wires has completed",
                );
            }
        }
        if start < cursor {
            return pending("overlapping disjoint native events require per-context lifetime accounting before resource reports can be discharged");
        }
        let end = &start + duration;
        let before = cursor.clone();
        cursor = end.clone();
        for wire in gate.controls.iter().chain(&gate.targets) {
            wire_ends.insert(*wire, end.clone());
        }
        out.push(Event {
            gate,
            start,
            end,
            before,
            cost,
        });
    }
    Ok(out)
}
fn resources(
    c: &Context,
    events: &[Event<'_>],
    docs: &LoadedDocs,
) -> Result<(BTreeMap<String, Rat>, BTreeMap<String, u64>)> {
    let iface = docs
        .object
        .interface(&c.symbol)
        .map_err(ClaimOutcome::unchecked)?;
    let wires = iface
        .bundles
        .iter()
        .try_fold(c.controls, |n, b| n.checked_add(b.width))
        .ok_or_else(|| {
            crate::cert::host_limit_outcome(crate::fault::HostLimit {
                quantity: "resource wire count".into(),
            })
        })?;
    let target = docs
        .target
        .as_ref()
        .ok_or_else(|| ClaimOutcome::unchecked("target missing for accounting"))?;
    let classes = target
        .get("classes")
        .and_then(Value::as_obj)
        .expect("native matcher checked target classes");
    let (class, description) = classes.iter().next().unwrap();
    if let Some(capacity) = description
        .get("capacity")
        .filter(|v| v.as_str() != Some("unbounded"))
    {
        let capacity = decimal(Some(capacity), "target.classes.capacity")?;
        if capacity < Rat::from_integer(wires.into()) {
            return bad("E_SCHEDULE_INFEASIBLE","target.classes.capacity",format!("invocation requires {wires} simultaneously live {class} wires; capacity is {capacity}"));
        }
    }
    let selected: Vec<_> = events
        .iter()
        .filter(|e| e.gate.path.starts_with(&c.key))
        .collect();
    // An action with no emitted events retains its interface for zero elapsed
    // time; peak occupancy still includes the live interface at entry.
    let duration = selected
        .first()
        .map(|first| &selected.last().unwrap().end - &first.before)
        .unwrap_or_else(Rat::zero);
    let declarations = target
        .get("resources")
        .and_then(Value::as_obj)
        .ok_or_else(|| ClaimOutcome::unchecked("target resource declarations unavailable"))?;
    let mut values = BTreeMap::new();
    for (name, definition) in declarations {
        let kind = definition.get("kind").and_then(Value::as_str).unwrap_or("");
        let value = match kind {
            "count" => selected
                .iter()
                .filter_map(|e| e.cost.get(name))
                .cloned()
                .sum(),
            "time" => duration.clone(),
            "occupancy" => {
                let rate = definition
                    .get("rate")
                    .and_then(|r| r.get(class))
                    .map(|v| decimal(Some(v), "target.resources.rate"))
                    .transpose()?
                    .unwrap_or_else(Rat::one);
                if rate.is_negative() {
                    return bad(
                        "E_CERT_TARGET",
                        "target.resources.rate",
                        "occupancy rate cannot be negative",
                    );
                }
                Rat::from_integer(wires.into()) * &duration * rate
            }
            "peak" => Rat::from_integer(wires.into()),
            _ => {
                return bad(
                    "E_CERT_TARGET",
                    "target.resources.kind",
                    "unknown resource aggregation kind",
                )
            }
        };
        values.insert(name.clone(), value);
    }
    for event in &selected {
        for name in event.cost.keys() {
            if !declarations.contains_key(name) {
                return bad(
                    "E_CERT_TARGET",
                    "target.gates.cost",
                    format!("cost names undeclared resource {name}"),
                );
            }
        }
    }
    let caps = target.get("capabilities");
    if let Some(max) = caps.and_then(|c| c.get("max_parallel")).and_then(u64_of) {
        if max == 0 && selected.iter().any(|e| e.end > e.start) {
            return bad(
                "E_SCHEDULE_INFEASIBLE",
                "target.capabilities.max_parallel",
                "nonzero-duration gate exceeds zero parallel capacity",
            );
        }
    }
    if let Some(max) = caps
        .and_then(|c| c.get("max_predicate_depth"))
        .filter(|v| v.as_str() != Some("unbounded"))
    {
        let max = decimal(Some(max), "target.capabilities.max_predicate_depth")?;
        if selected
            .iter()
            .any(|e| Rat::from_integer((e.gate.controls.len() - e.gate.own_controls).into()) > max)
        {
            return bad(
                "E_SCHEDULE_INFEASIBLE",
                "target.capabilities.max_predicate_depth",
                "effective predicate depth exceeds target capability",
            );
        }
    }
    let mut counts = BTreeMap::new();
    for event in selected {
        *counts
            .entry(event.gate.realised_opcode().to_string())
            .or_insert(0) += 1;
    }
    Ok((values, counts))
}
fn escape(name: &str) -> String {
    let mut out = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
fn check_estimate(
    record: &Value,
    values: &BTreeMap<String, Rat>,
    docs: &LoadedDocs,
    base: &str,
) -> Result<()> {
    let estimate = record.get("estimate").expect("estimate branch");
    let mut schema = vec![];
    if !crate::cert::check_keys(
        estimate,
        &["precision_digits", "exact_resources"],
        &["precision_digits", "exact_resources"],
        &format!("{base}.estimate"),
        &mut schema,
    ) {
        for e in &mut schema {
            e.code = "E_CERT_ESTIMATE";
        }
        return Err(ClaimOutcome::rejected(schema));
    }
    if estimate.get("precision_digits").and_then(u64_of) != Some(34) {
        return bad(
            "E_CERT_ESTIMATE",
            format!("{base}.estimate.precision_digits"),
            "launch estimates use exactly 34 display digits",
        );
    }
    let target = docs.target.as_ref().unwrap();
    let classes = target.get("classes").and_then(Value::as_obj).unwrap();
    let class = classes.keys().next().unwrap();
    let declarations = target.get("resources").and_then(Value::as_obj).unwrap();
    let mut rows = BTreeMap::new();
    for (name, value) in values {
        let kind = declarations[name]
            .get("kind")
            .and_then(Value::as_str)
            .unwrap();
        let key = if matches!(kind, "occupancy" | "peak") {
            format!("{}[{}]", escape(name), escape(class))
        } else {
            escape(name)
        };
        rows.insert(key, value.clone());
    }
    let exact = estimate
        .get("exact_resources")
        .and_then(Value::as_obj)
        .ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_ESTIMATE",
                "§10.1",
                base,
                "exact_resources must be an object",
            )])
        })?;
    let display = record.get("resources").and_then(Value::as_obj).unwrap();
    if exact.len() != rows.len() || display.len() != rows.len() {
        return bad(
            "E_CERT_ESTIMATE",
            base,
            "estimate exact/display maps must contain every encoded resource row exactly once",
        );
    }
    for (name, computed) in rows {
        let Some(fraction) = exact.get(&name) else {
            return bad(
                "E_CERT_ESTIMATE",
                base,
                format!("estimate lacks exact row {name}"),
            );
        };
        let mut errors = vec![];
        if !crate::cert::check_keys(
            fraction,
            &["numerator", "denominator"],
            &["numerator", "denominator"],
            base,
            &mut errors,
        ) {
            for e in &mut errors {
                e.code = "E_CERT_ESTIMATE";
            }
            return Err(ClaimOutcome::rejected(errors));
        }
        let parse = |key: &str| {
            fraction
                .get(key)
                .and_then(Value::as_str)
                .and_then(crate::rat::parse_biguint_str)
        };
        let (Some(n), Some(d)) = (parse("numerator"), parse("denominator")) else {
            return bad(
                "E_CERT_ESTIMATE",
                base,
                "fraction components must be canonical nonnegative decimal strings",
            );
        };
        if d.is_zero() || crate::rat::gcd(n.clone(), d.clone()) != num_bigint::BigUint::one() {
            return bad("E_CERT_ESTIMATE",base,"estimate fractions must be reduced, with positive denominator and zero encoded as 0/1");
        }
        let got = Rat::new(n.into(), d.into());
        if got != computed {
            return bad(
                "E_CERT_ESTIMATE",
                base,
                format!("row {name}: exact {got} differs from independently recomputed {computed}"),
            );
        }
        let expected = crate::rat::upward_decimal_34(&computed).expect("resources are nonnegative");
        if display.get(&name).and_then(Value::as_str) != Some(expected.as_str()) {
            return bad(
                "E_CERT_ESTIMATE",
                format!("{base}.resources.{name}"),
                format!("expected canonical upward 34-digit display {expected}"),
            );
        }
    }
    Ok(())
}
fn check_resources(
    c: &Context,
    record: &Value,
    events: &[Event<'_>],
    docs: &LoadedDocs,
) -> Result<()> {
    let hash = context_hash(c, docs);
    let base = format!("regions.{hash}");
    let (values, counts) = resources(c, events, docs)?;
    if record.get("estimate").is_some() {
        check_estimate(record, &values, docs, &base)?;
    } else {
        let supplied = record
            .get("resources")
            .and_then(Value::as_obj)
            .expect("resource schema checked");
        if supplied.len() != values.len() {
            return bad(
                "E_CERT_RESOURCE",
                format!("{base}.resources"),
                "resource report must contain every declared target resource exactly once",
            );
        }
        for (name, computed) in &values {
            let got = decimal(supplied.get(name), &format!("{base}.resources.{name}"))?;
            if &got != computed {
                return bad(
                    "E_CERT_RESOURCE",
                    format!("{base}.resources.{name}"),
                    format!("declared {got}, independently recomputed {computed}"),
                );
            }
        }
    }
    let supplied = record
        .get("gates_used")
        .and_then(Value::as_obj)
        .expect("gate report schema checked");
    if supplied.len() != counts.len() {
        return bad(
            "E_CERT_GATE_COUNT",
            format!("{base}.gates_used"),
            "gate report keys differ from the realised native leaves",
        );
    }
    for (op, count) in counts {
        if supplied
            .get(&op)
            .and_then(|g| g.get("count"))
            .and_then(u64_of)
            != Some(count)
        {
            return bad(
                "E_CERT_GATE_COUNT",
                format!("{base}.gates_used.{op}.count"),
                format!("realised leaf count is {count}"),
            );
        }
    }
    let frame = record.get("frame").expect("frame schema checked");
    let slots = frame.get("slots").and_then(Value::as_arr).unwrap_or(&[]);
    let iface = docs
        .object
        .interface(&c.symbol)
        .map_err(ClaimOutcome::unchecked)?;
    for (slot, bundle) in slots.iter().zip(iface.bundles) {
        let width = expr(
            slot.get("width").unwrap(),
            &HashMap::from([("c", Rat::from_integer(c.controls.into()))]),
            &Value::Null,
            "frame.slots.width",
        )?;
        if width != Rat::from_integer(bundle.width.into()) {
            return bad(
                "E_CERT_FRAME",
                format!("{base}.frame.slots"),
                "evaluated source slot width differs from its interface",
            );
        }
    }
    Ok(())
}
fn check_errors(
    cert: &Value,
    contexts: &[Context],
    docs: &LoadedDocs,
    verdict: &Verdict,
) -> Result<()> {
    let records = cert
        .get("regions")
        .and_then(Value::as_obj)
        .expect("certificate regions checked");
    let variant = docs
        .schedule
        .as_ref()
        .and_then(|s| s.get("bindings"))
        .and_then(|b| b.get("variant"))
        .and_then(Value::as_str)
        .expect("schedule variant checked");
    let mut local: BTreeMap<String, BTreeMap<String, Rat>> = BTreeMap::new();
    let mut inclusive = local.clone();
    for c in contexts {
        let hash = context_hash(c, docs);
        let key = key_of(hash, variant, c.controls);
        let inner = inner_key(variant, c.controls);
        let region = records.get(hash).ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_BINDING",
                "§12.3",
                "regions",
                "invocation has no region record",
            )])
        })?;
        let map = region
            .get("error")
            .and_then(Value::as_obj)
            .expect("error schema checked");
        if map.len() != 1 || !map.contains_key(&inner) {
            return pending("error report includes unresolved variants or omits the realised variant/control key");
        }
        for (part, out) in [("local", &mut local), ("inclusive", &mut inclusive)] {
            let values = map[&inner]
                .get(part)
                .and_then(Value::as_obj)
                .expect("error map schema checked");
            if values.is_empty() {
                return pending("realised error report has no verified metric");
            }
            let mut checked = BTreeMap::new();
            for (metric, value) in values {
                let bound = decimal(Some(value), "region.error")?;
                if bound.is_negative() {
                    return bad(
                        "E_CERT_COMPOSITION",
                        "region.error",
                        "error upper bounds cannot be negative",
                    );
                }
                checked.insert(metric.clone(), bound);
            }
            out.insert(key.clone(), checked);
        }
        if c.controls > 0 {
            let report = verdict
                .regions
                .iter()
                .find(|r| r.closure_hash == hash)
                .expect("region report exists");
            let indices = region
                .get("verification")
                .and_then(|v| v.get("unitary"))
                .and_then(Value::as_arr)
                .unwrap_or(&[]);
            let claims = region.get("claims").and_then(Value::as_arr).unwrap();
            let valid = indices.iter().filter_map(u64_of).any(|i| {
                claims
                    .get(i as usize)
                    .and_then(|c| c.get("subject"))
                    .map(|s| {
                        s.get("variant").and_then(Value::as_str) == Some(variant)
                            && s.get("inherited_controls").and_then(u64_of) == Some(c.controls)
                    })
                    .unwrap_or(false)
                    && report
                        .claims
                        .get(i as usize)
                        .map(|(_, o)| o.outcome == "accept")
                        .unwrap_or(false)
            });
            if !valid {
                return pending("controlled unitary has no completed numerical claim in verification.unitary for its realised key (§12.3)");
            }
        }
    }
    for c in contexts {
        let key = key_of(context_hash(c, docs), variant, c.controls);
        for (metric, declared) in &inclusive[&key] {
            let Some(mut computed) = local[&key].get(metric).cloned() else {
                return bad(
                    "E_CERT_COMPOSITION",
                    "region.error",
                    "inclusive metric has no local bound in that metric",
                );
            };
            for child in contexts
                .iter()
                .filter(|child| child.key.len() == c.key.len() + 1 && child.key.starts_with(&c.key))
            {
                let child_key = key_of(context_hash(child, docs), variant, child.controls);
                let value = if let Some(v) = inclusive[&child_key].get(metric) {
                    v.clone()
                } else if metric == "diamond" {
                    inclusive[&child_key]
                        .get("operator_norm")
                        .map(|v| v * Rat::from_integer(2.into()))
                        .ok_or_else(|| {
                            ClaimOutcome::unchecked("callee has no composable error metric")
                        })?
                } else {
                    return bad(
                        "E_CERT_COMPOSITION",
                        "region.error",
                        "operator_norm conclusion requires operator_norm bounds throughout",
                    );
                };
                computed += value;
            }
            if &computed != declared {
                return bad(
                    "E_CERT_COMPOSITION",
                    format!("regions.{}.error", context_hash(c, docs)),
                    format!("inclusive {metric} is {declared}; local plus callees is {computed}"),
                );
            }
        }
    }
    let total: Rat = contexts
        .iter()
        .map(|c| &local[&key_of(context_hash(c, docs), variant, c.controls)])
        .map(|m| {
            m.get("diamond")
                .cloned()
                .or_else(|| {
                    m.get("operator_norm")
                        .map(|r| r * Rat::from_integer(2.into()))
                })
                .expect("a supported error metric is present")
        })
        .sum();
    let declared = decimal(
        cert.get("composition")
            .and_then(|c| c.get("epsilon_total"))
            .and_then(|e| e.get("diamond")),
        "composition.epsilon_total.diamond",
    )?;
    if declared != total {
        return bad(
            "E_CERT_COMPOSITION",
            "composition.epsilon_total.diamond",
            format!(
                "declared total {declared}; multiplicity sum of verified local bounds is {total}"
            ),
        );
    }
    let budget = decimal(
        docs.schedule
            .as_ref()
            .and_then(|s| s.get("bindings"))
            .and_then(|b| b.get("epsilon")),
        "schedule.bindings.epsilon",
    )?;
    if total > budget {
        return bad(
            "E_CERT_COMPOSITION",
            "composition.epsilon_total",
            format!("composed diamond error {total} exceeds root epsilon {budget}"),
        );
    }
    Ok(())
}
fn inner(cert: &Value, docs: &LoadedDocs, inputs: &Inputs, verdict: &Verdict) -> Result<()> {
    let (contexts, gates) = crate::realisation::trace(docs, inputs)?;
    let records = cert
        .get("regions")
        .and_then(Value::as_obj)
        .expect("region schema checked");
    let variant = docs
        .schedule
        .as_ref()
        .and_then(|s| s.get("bindings"))
        .and_then(|b| b.get("variant"))
        .and_then(Value::as_str)
        .unwrap();
    let mut multiplicities: BTreeMap<String, u64> = BTreeMap::new();
    let mut hashes = HashSet::new();
    for c in &contexts {
        *multiplicities
            .entry(key_of(context_hash(c, docs), variant, c.controls))
            .or_default() += 1;
        hashes.insert(context_hash(c, docs));
    }
    let supplied = cert
        .get("composition")
        .and_then(|c| c.get("multiplicities"))
        .and_then(Value::as_obj)
        .expect("composition schema checked");
    // Empty scheduled reports are incomplete, rather than a claim that the
    // independently counted tree is empty. They discharge no composition.
    if supplied.is_empty() {
        return pending("scheduled certificate has no multiplicity report to compare with the realised invocation tree");
    }
    if supplied.len() != multiplicities.len()
        || multiplicities
            .iter()
            .any(|(k, n)| supplied.get(k).and_then(u64_of) != Some(*n))
    {
        return bad(
            "E_CERT_MULTIPLICITY",
            "composition.multiplicities",
            format!(
                "declared counts differ from independently enumerated contexts: {multiplicities:?}"
            ),
        );
    }
    if hashes.len() != contexts.len() {
        return pending("per-region resource accounting for repeated contexts requires an explicit aggregation contract; each context is still numerically checked");
    }
    if records.len() != hashes.len() || records.keys().any(|h| !hashes.contains(h.as_str())) {
        return pending("certificate includes region records outside the realised invocation tree");
    }
    for record in records.values() {
        if ["scratch", "history", "borrow", "ancillae"]
            .iter()
            .any(|k| {
                record
                    .get("frame")
                    .and_then(|f| f.get(k))
                    .and_then(Value::as_arr)
                    .map(|a| !a.is_empty())
                    == Some(true)
            })
        {
            return pending(
                "nontrivial frame descriptors need allocation/lifetime expansion before accounting",
            );
        }
    }
    let events = events(&gates, docs)?;
    for c in &contexts {
        check_resources(c, &records[context_hash(c, docs)], &events, docs)?;
    }
    check_errors(cert, &contexts, docs, verdict)
}
pub(crate) fn check(cert: &Value, docs: &LoadedDocs, inputs: &Inputs, verdict: &mut Verdict) {
    if docs.schedule.is_none()
        || !verdict.errors.is_empty()
        || verdict
            .regions
            .iter()
            .any(|r| !r.errors.is_empty() || r.claims.iter().any(|(_, o)| o.outcome == "reject"))
    {
        return;
    }
    match inner(cert, docs, inputs, verdict) {
        Ok(()) => {
            verdict.unchecked_items.retain(|s|s!="composition multiplicities and inclusive error recurrence have not been recomputed from the realised invocation tree (§12.3)");
            for r in &mut verdict.regions {
                r.unchecked_items.retain(|s| {
                    !s.starts_with("resources producer report has not been recomputed")
                        && !s.starts_with("gates_used producer report has not been recomputed")
                        && !s.starts_with("local/inclusive error entries have not been tied")
                        && !s.starts_with("estimate exact_resources (§10.1) not recomputed")
                        && !s.starts_with("symbolic frame slot widths have not been evaluated")
                });
            }
            verdict.establishes.push("native leaf timing, interface-wire resources, gate counts, invocation multiplicities and local/inclusive error composition were independently recomputed (§8.2/§12.3)".into());
        }
        Err(o) if o.outcome == "reject" => verdict.errors.extend(o.errors),
        Err(o) => verdict.unchecked_items.push(
            o.unchecked_reason
                .unwrap_or_else(|| "accounting incomplete".into()),
        ),
    }
}
