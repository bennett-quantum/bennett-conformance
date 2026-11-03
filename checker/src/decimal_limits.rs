//! Preflight only schema positions interpreted as exact decimal rationals.
//! Names, argument strings, tuple integers and other opaque strings are not
//! decimals. A host cap cannot turn a legal decimal into a schema rejection.
use crate::fault::HostLimit;
use crate::json::Value;

type Found = Option<HostLimit>;
fn scalar(v: &Value, path: &str) -> Found {
    let text = match v {
        Value::Str(s) | Value::Num(s) => s,
        _ => return None,
    };
    crate::rat::decimal_host_limit(text).map(|h| HostLimit {
        quantity: format!("{path}: {}", h.quantity),
    })
}
fn tree(v: &Value, path: &str) -> Found {
    match v {
        Value::Obj(m) => m.iter().find_map(|(k, v)| tree(v, &format!("{path}.{k}"))),
        Value::Arr(a) => a
            .iter()
            .enumerate()
            .find_map(|(i, v)| tree(v, &format!("{path}[{i}]"))),
        _ => scalar(v, path),
    }
}
fn field(v: &Value, key: &str, path: &str) -> Found {
    v.get(key).and_then(|v| scalar(v, &format!("{path}.{key}")))
}
fn expression(v: &Value, path: &str) -> Found {
    if let Some(args) = v.get("args").and_then(Value::as_arr) {
        args.iter()
            .enumerate()
            .find_map(|(i, v)| expression(v, &format!("{path}.args[{i}]")))
    } else {
        scalar(v, path)
    }
}
fn costs(v: Option<&Value>, path: &str) -> Found {
    v.and_then(Value::as_obj).and_then(|m| {
        m.iter()
            .find_map(|(k, v)| expression(v, &format!("{path}.{k}")))
    })
}
fn certificate(v: &Value) -> Found {
    if let Some(regions) = v.get("regions").and_then(Value::as_obj) {
        for (hash, r) in regions {
            let path = format!("certificate.regions.{hash}");
            for field in ["error"] {
                if let Some(v) = r.get(field) {
                    if let Some(h) = tree(v, &format!("{path}.{field}")) {
                        return Some(h);
                    }
                }
            }
            let resource_reports = std::iter::once(r).chain(
                r.get("resource_contexts")
                    .and_then(Value::as_arr)
                    .into_iter()
                    .flatten(),
            );
            for (i, record) in resource_reports.enumerate() {
                if let Some(resources) = record.get("resources").and_then(Value::as_obj) {
                    for (name, value) in resources {
                        if value.as_str().is_some_and(crate::numeric::resource_display) {
                            if let Some(h) = scalar(
                                value,
                                &format!("{path}.resource_report[{i}].resources.{name}"),
                            ) {
                                return Some(h);
                            }
                        }
                    }
                }
                if let Some(gates) = record.get("gates_used").and_then(Value::as_obj) {
                    for (name, entry) in gates {
                        if let Some(h) = field(
                            entry,
                            "epsilon",
                            &format!("{path}.resource_report[{i}].gates_used.{name}"),
                        ) {
                            return Some(h);
                        }
                    }
                }
            }
            if let Some(gates) = r.get("gates_used").and_then(Value::as_obj) {
                for (name, g) in gates {
                    if let Some(h) = field(g, "epsilon", &format!("{path}.gates_used.{name}")) {
                        return Some(h);
                    }
                }
            }
            let frames = r.get("frame").into_iter().chain(
                r.get("frame_contexts")
                    .and_then(Value::as_arr)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.get("frame")),
            );
            for frame in frames {
                for group in ["slots", "scratch", "history", "borrow", "ancillae"] {
                    if let Some(records) = frame.get(group).and_then(Value::as_arr) {
                        for (i, r) in records.iter().enumerate() {
                            if let Some(width) = r.get("width") {
                                if let Some(h) =
                                    expression(width, &format!("{path}.frame.{group}[{i}].width"))
                                {
                                    return Some(h);
                                }
                            }
                        }
                    }
                }
            }
            if let Some(claims) = r.get("claims").and_then(Value::as_arr) {
                for (i, c) in claims.iter().enumerate() {
                    let p = format!("{path}.claims[{i}]");
                    if matches!(
                        c.get("kind").and_then(Value::as_str),
                        Some("numerical" | "channel")
                    ) {
                        if let Some(h) = field(c, "bound", &p) {
                            return Some(h);
                        }
                        if let Some(dual) = c.get("dual") {
                            if let Some(h) = tree(dual, &format!("{p}.dual")) {
                                return Some(h);
                            }
                        }
                    }
                }
            }
        }
    }
    v.get("composition")
        .and_then(|v| v.get("epsilon_total"))
        .and_then(|v| tree(v, "certificate.composition.epsilon_total"))
}
fn target(v: &Value) -> Found {
    for field in ["classes", "tables", "capabilities"] {
        if let Some(v) = v.get(field) {
            if let Some(h) = tree(v, &format!("target.{field}")) {
                return Some(h);
            }
        }
    }
    if let Some(resources) = v.get("resources").and_then(Value::as_obj) {
        for (name, r) in resources {
            if let Some(rate) = r.get("rate") {
                if let Some(h) = tree(rate, &format!("target.resources.{name}.rate")) {
                    return Some(h);
                }
            }
        }
    }
    if let Some(gates) = v.get("gates").and_then(Value::as_arr) {
        for (i, g) in gates.iter().enumerate() {
            let path = format!("target.gates[{i}]");
            if let Some(e) = g.get("epsilon") {
                if let Some(h) = expression(e, &format!("{path}.epsilon")) {
                    return Some(h);
                }
            }
            if let Some(h) = costs(g.get("cost"), &format!("{path}.cost")) {
                return Some(h);
            }
        }
    }
    if let Some(qram) = v.get("qram").and_then(Value::as_obj) {
        for (name, q) in qram {
            if let Some(h) = costs(q.get("cost"), &format!("target.qram.{name}.cost")) {
                return Some(h);
            }
        }
    }
    if let Some(schemes) = v.get("schemes").and_then(Value::as_arr) {
        for (i, s) in schemes.iter().enumerate() {
            let path = format!("target.schemes[{i}]");
            if let Some(count) = s.get("ancillae").and_then(|a| a.get("count")) {
                if let Some(h) = expression(count, &format!("{path}.ancillae.count")) {
                    return Some(h);
                }
            }
            if let Some(template) = s.get("template").and_then(Value::as_arr) {
                for (j, t) in template.iter().enumerate() {
                    let p = format!("{path}.template[{j}]");
                    if let Some(params) = t.get("params").and_then(Value::as_arr) {
                        for (k, e) in params.iter().enumerate() {
                            if let Some(h) = expression(e, &format!("{p}.params[{k}]")) {
                                return Some(h);
                            }
                        }
                    }
                    if let Some(repeat) = t.get("repeat") {
                        for key in ["from", "to"] {
                            if let Some(e) = repeat.get(key) {
                                if let Some(h) = expression(e, &format!("{p}.repeat.{key}")) {
                                    return Some(h);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(weighted) = v.get("objective").and_then(|v| v.get("weighted")) {
        if let Some(h) = tree(weighted, "target.objective.weighted") {
            return Some(h);
        }
    }
    if let Some(imports) = v.get("imports").and_then(Value::as_obj) {
        for (name, import) in imports {
            if let Some(map) = import.get("map").and_then(Value::as_obj) {
                for (quantity, pair) in map {
                    if let Some(factor) = pair.as_arr().and_then(|a| a.get(1)) {
                        if let Some(h) =
                            scalar(factor, &format!("target.imports.{name}.map.{quantity}[1]"))
                        {
                            return Some(h);
                        }
                    }
                }
            }
        }
    }
    None
}
fn schedule(v: &Value) -> Found {
    if let Some(bindings) = v.get("bindings") {
        if let Some(h) = field(bindings, "epsilon", "schedule.bindings") {
            return Some(h);
        }
    }
    if let Some(entries) = v.get("entries").and_then(Value::as_arr) {
        for (i, e) in entries.iter().enumerate() {
            if let Some(gate) = e.get("gate") {
                if let Some(h) = field(
                    gate,
                    "epsilon_share",
                    &format!("schedule.entries[{i}].gate"),
                ) {
                    return Some(h);
                }
            }
            if let Some(start) = e.get("start") {
                if let Some(h) = field(start, "time", &format!("schedule.entries[{i}].start")) {
                    return Some(h);
                }
            }
        }
    }
    None
}
pub fn document(v: &Value, kind: &str) -> Found {
    match kind {
        "target" => target(v),
        "schedule" => schedule(v),
        "certificate" => certificate(v),
        _ => None,
    }
}
