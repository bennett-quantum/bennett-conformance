//! Target/schedule/frame/composition checks and the numerical, channel,
//! and structural claim verifiers (ISA sections 8, 9.4, 11, 12.1, 12.3).

use crate::cert::{
    int_of, rej, schema_err, u64_of, ClaimOutcome, Inputs, Invocation, LoadedDocs, RegionReport,
    REF_09_HASH,
};
use crate::fault::Reject;
use crate::json::Value;
use crate::object::{Object, Symbol};
use crate::rat::rat_of_json;
use crate::is_hash_hex;
use std::collections::HashSet;

fn keys_strict(v: &Value, allowed: &[&str], required: &[&str], path: &str, code: &'static str, section: &'static str, errs: &mut Vec<Reject>) -> bool {
    let Some(obj) = v.as_obj() else {
        errs.push(rej(code, section, path.to_string(), format!("expected object, got {}", v.kind())));
        return false;
    };
    let mut ok = true;
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            errs.push(rej(code, section, format!("{path}.{k}"), format!("unknown field {k:?}")));
            ok = false;
        }
    }
    for r in required {
        if !obj.contains_key(*r) {
            errs.push(rej(code, section, format!("{path}.{r}"), format!("missing required field {r:?}")));
            ok = false;
        }
    }
    ok
}

// ---------- target (section 8), bindings-level shape ----------

pub fn validate_target_shape(t: &Value, errs: &mut Vec<Reject>) {
    let path = "target";
    if !keys_strict(
        t,
        &["ptr_widths_supported", "classes", "topology", "gates", "schemes", "qram", "capabilities", "objective", "tables", "imports", "resources"],
        &["ptr_widths_supported", "classes", "gates", "schemes", "qram", "capabilities", "objective", "tables", "imports", "resources"],
        path,
        "E_CERT_TARGET",
        "§8",
        errs,
    ) {
        return;
    }
    // Mandatory resources with mandatory kinds (§8.2).
    if let Some(res) = t.get("resources").and_then(Value::as_obj) {
        for (name, kind) in [("qubit_second", "occupancy"), ("time", "time"), ("peak_qubits", "peak")] {
            match res.get(name).and_then(|r| r.get("kind")).and_then(Value::as_str) {
                Some(k) if k == kind => {}
                _ => errs.push(rej("E_CERT_TARGET", "§8.2", format!("target.resources.{name}"), format!("mandatory resource missing or kind is not {kind}"))),
            }
        }
    } else {
        errs.push(rej("E_CERT_TARGET", "§8.2", "target.resources", "resources must be an object"));
    }
    if let Some(caps) = t.get("capabilities") {
        if caps.get("mbu") != Some(&Value::Bool(false)) {
            errs.push(rej("E_CERT_TARGET", "§17", "target.capabilities.mbu", "capabilities.mbu is reserved and must be false in 0.9 (item 6)"));
        }
    }
    // Topology well-formedness (§8.3).
    if let Some(topo) = t.get("topology") {
        let classes: HashSet<&str> = t
            .get("classes")
            .and_then(Value::as_obj)
            .map(|c| c.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let verts = topo.get("vertices").and_then(Value::as_arr).map(|a| a.to_vec()).unwrap_or_default();
        for (i, v) in verts.iter().enumerate() {
            match v.get("class").and_then(Value::as_str) {
                Some(c) if classes.contains(c) => {}
                _ => errs.push(rej("E_CERT_TARGET", "§8.3", format!("target.topology.vertices[{i}]"), "vertex class is not a declared class")),
            }
        }
        let mut prev: Option<(i128, i128)> = None;
        if let Some(edges) = topo.get("edges").and_then(Value::as_arr) {
            for (i, e) in edges.iter().enumerate() {
                let pair = e.as_arr().filter(|a| a.len() == 2).and_then(|a| Some((int_of(&a[0])?, int_of(&a[1])?)));
                match pair {
                    Some((u, v)) if 0 <= u && u < v && (v as usize) < verts.len() => {
                        if prev.map(|p| (u, v) <= p).unwrap_or(false) {
                            errs.push(rej("E_CERT_TARGET", "§8.3", format!("target.topology.edges[{i}]"), "edges must be sorted lexicographically without duplicates"));
                        }
                        prev = Some((u, v));
                    }
                    _ => errs.push(rej("E_CERT_TARGET", "§8.3", format!("target.topology.edges[{i}]"), "edge must satisfy 0 <= u < v < vertices.length")),
                }
            }
        }
    }
}

pub fn target_classes(t: &Value) -> Vec<String> {
    let mut v: Vec<String> = t
        .get("classes")
        .and_then(Value::as_obj)
        .map(|c| c.keys().cloned().collect())
        .unwrap_or_default();
    v.sort();
    v
}

// ---------- schedule bindings (section 11) ----------

/// Returns (effective expansion (name, version, hash), expansion_unknown).
pub fn check_schedule_bindings(
    sched: &Value,
    object: &Object,
    root: &Symbol,
    cert_bindings: &Value,
    _schedule_hash: Option<&str>,
    errs: &mut Vec<Reject>,
) -> (Option<(String, String, String)>, bool) {
    let mut local = Vec::new();
    if !keys_strict(sched, &["bindings", "interface_vertices", "entries"], &["bindings", "entries"], "schedule", "E_CERT_SCHEDULE_BINDING", "§11", &mut local) {
        errs.extend(local);
        return (None, false);
    }
    let sb = sched.get("bindings").cloned().unwrap_or(Value::Null);
    keys_strict(
        &sb,
        &["closure_hash", "target_hash", "region", "variant", "inherited_controls", "budget", "params_hash", "epsilon", "expansion", "scheduler"],
        &["closure_hash", "target_hash", "region", "variant", "inherited_controls", "budget", "params_hash", "epsilon", "scheduler"],
        "schedule.bindings",
        "E_CERT_SCHEDULE_BINDING",
        "§11",
        &mut local,
    );
    let root_closure = root.closure_hash.clone().unwrap_or_default();
    if sb.get("closure_hash").and_then(Value::as_str) != Some(root_closure.as_str()) {
        local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.closure_hash", "does not equal the certificate root closure_hash"));
    }
    if let (Some(want), Some(got)) = (
        cert_bindings.get("target_hash").and_then(Value::as_str),
        sb.get("target_hash").and_then(Value::as_str),
    ) {
        if want != got {
            local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.target_hash", "does not equal the certificate target_hash"));
        }
    }
    if sb.get("region").and_then(Value::as_str) != Some(root.name.as_str()) {
        local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.region", "does not name the root region of the object"));
    }
    match sb.get("variant").and_then(Value::as_str) {
        Some(v) if !v.is_empty() => {}
        _ => local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.variant", "variant must be a nonempty string")),
    }
    if sb.get("inherited_controls").and_then(u64_of).is_none() {
        local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.inherited_controls", "expected nonnegative integer"));
    }
    match sb.get("params_hash").and_then(Value::as_str) {
        Some(h) if is_hash_hex(h) => {}
        _ => local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.params_hash", "expected 64-lowercase-hex")),
    }
    if sb.get("epsilon").map(|e| rat_of_json(e).is_ok()) != Some(true) {
        local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.epsilon", "epsilon must be a decimal"));
    }
    // Expansion library: explicit binding, else the §17 item 7 default at 0.9.
    let isa09 = object.header.versions.get("isa").map(String::as_str) == Some("0.9");
    let (expansion, unknown) = match sb.get("expansion") {
        None => {
            if isa09 {
                (Some(("ref".to_string(), "0.9".to_string(), REF_09_HASH.to_string())), false)
            } else {
                (None, true)
            }
        }
        Some(e) => {
            let name = e.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let version = e.get("version").and_then(Value::as_str).unwrap_or("").to_string();
            let hash = e.get("hash").and_then(Value::as_str).unwrap_or("").to_string();
            if !is_hash_hex(&hash) || name.is_empty() || version.is_empty() {
                local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.bindings.expansion", "expansion must be {name, version, hash} with a 64-hex hash"));
                (None, false)
            } else if name == "ref" && version == "0.9" && hash != REF_09_HASH {
                local.push(rej("E_CERT_SCHEDULE_BINDING", "§11.1", "schedule.bindings.expansion.hash", "ref 0.9 must bind the published full-library envelope hash"));
                (None, false)
            } else {
                let known = name == "ref" && version == "0.9";
                (Some((name, version, hash)), !known)
            }
        }
    };
    // Entries: selector shapes + path existence against the object.
    if let Some(Value::Arr(entries)) = sched.get("entries") {
        for (i, entry) in entries.iter().enumerate() {
            validate_schedule_entry(entry, object, &format!("schedule.entries[{i}]"), &mut local);
        }
    } else {
        local.push(rej("E_CERT_SCHEDULE_BINDING", "§11", "schedule.entries", "entries must be an array"));
    }
    errs.extend(local);
    (expansion, unknown)
}

pub fn validate_schedule_entry(entry: &Value, object: &Object, path: &str, errs: &mut Vec<Reject>) {
    let kind = entry.get("kind").and_then(Value::as_str).unwrap_or("");
    if !matches!(kind, "cleanup" | "gate" | "alloc" | "start") {
        errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.kind"), format!("unknown entry kind {kind:?}")));
        return;
    }
    let allowed: &[&str] = &["path", "step", "phase", "generated", "kind", "cleanup", "gate", "alloc", "start"];
    keys_strict(entry, allowed, &["path", "step", "phase", "generated", "kind", kind], path, "E_CERT_SCHEDULE_BINDING", "§11", errs);
    for other in ["cleanup", "gate", "alloc", "start"] {
        if other != kind && entry.get(other).is_some() {
            errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.{other}"), "only the entry's own kind object may be present"));
        }
    }
    validate_selector(entry.get("step"), true, &format!("{path}.step"), errs);
    match entry.get("phase").and_then(Value::as_str) {
        Some("fwd" | "rev" | "*") => {}
        _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.phase"), "phase must be fwd, rev or *")),
    }
    validate_selector(entry.get("generated"), true, &format!("{path}.generated"), errs);
    if let Some(Value::Arr(p)) = entry.get("path") {
        for (j, el) in p.iter().enumerate() {
            validate_path_element(el, object, &format!("{path}.path[{j}]"), errs);
        }
    } else {
        errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.path"), "path must be an array (possibly empty)"));
    }
    // Kind objects.
    match kind {
        "cleanup" => {
            if let Some(c) = entry.get("cleanup") {
                keys_strict(c, &["realisation"], &["realisation"], &format!("{path}.cleanup"), "E_CERT_SCHEDULE_BINDING", "§11", errs);
                match c.get("realisation").and_then(Value::as_str) {
                    Some("reverse" | "recompute") => {}
                    _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.cleanup.realisation"), "must be reverse or recompute (measurement-based cleanup is deferred, §17 item 6)")),
                }
            }
        }
        "gate" => {
            if let Some(g) = entry.get("gate") {
                keys_strict(g, &["realisation", "epsilon_share"], &["realisation"], &format!("{path}.gate"), "E_CERT_SCHEDULE_BINDING", "§11", errs);
                if g.get("realisation").and_then(Value::as_str).map(|x| x.is_empty()) != Some(false) {
                    errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.gate.realisation"), "must be native or a scheme name"));
                }
                if let Some(e) = g.get("epsilon_share") {
                    if rat_of_json(e).is_err() {
                        errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.gate.epsilon_share"), "must be a decimal"));
                    }
                }
            }
        }
        "alloc" => {
            if let Some(a) = entry.get("alloc") {
                keys_strict(a, &["class", "vertices"], &["class"], &format!("{path}.alloc"), "E_CERT_SCHEDULE_BINDING", "§11", errs);
                match a.get("class").and_then(Value::as_str) {
                    Some(c) if !c.is_empty() && c != "any" => {}
                    _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.alloc.class"), "must be a concrete class; \"any\" is not allowed in a schedule")),
                }
            }
        }
        "start" => {
            if let Some(x) = entry.get("start") {
                keys_strict(x, &["time"], &["time"], &format!("{path}.start"), "E_CERT_SCHEDULE_BINDING", "§11", errs);
                if x.get("time").map(|t| rat_of_json(t).is_ok()) != Some(true) {
                    errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.start.time"), "must be a decimal"));
                }
            }
        }
        _ => unreachable!(),
    }
}

fn validate_selector(v: Option<&Value>, _int_ok: bool, path: &str, errs: &mut Vec<Reject>) {
    match v {
        Some(Value::Str(x)) if x == "*" => {}
        Some(x) if u64_of(x).is_some() => {}
        _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", path.to_string(), "selector must be a nonnegative integer or \"*\"")),
    }
}

pub fn validate_path_element(el: &Value, object: &Object, path: &str, errs: &mut Vec<Reject>) {
    keys_strict(el, &["symbol", "point", "iteration", "instance", "call_index"], &["symbol", "point", "iteration"], path, "E_CERT_SCHEDULE_BINDING", "§11", errs);
    let sym = match el.get("symbol").and_then(Value::as_str) {
        Some(h) if is_hash_hex(h) => match object.symbol_by_closure(h).or_else(|| object.symbols.iter().find(|x| x.interface_hash == h)) {
            Some(s) => Some(s.clone()),
            None => {
                errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.symbol"), "path symbol hash names no symbol of the object"));
                None
            }
        },
        _ => {
            errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.symbol"), "expected 64-lowercase-hex symbol hash"));
            None
        }
    };
    match el.get("point").and_then(Value::as_str) {
        Some(p @ ("$construct" | "$step" | "$commit" | "$release")) => {
            if let Some(s) = &sym {
                if s.kind != "oracle" {
                    errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.point"), format!("pseudo-point {p} is only legal in an oracle")));
                }
            }
        }
        Some(p) => match p.rsplit_once(':') {
            Some((block, ord)) if ord.parse::<u64>().is_ok() && !block.is_empty() => {
                if let Some(s) = &sym {
                    match object.instruction_count(&s.name, block) {
                        Some(count) if (ord.parse::<usize>().unwrap()) < count => {}
                        Some(_) => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.point"), format!("ordinal out of range for block {block:?} of @{}", s.name))),
                        None => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.point"), format!("@{} has no block {block:?}", s.name))),
                    }
                }
            }
            _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.point"), format!("point must be \"<block>:<ordinal>\" or a reserved pseudo-point, got {p:?}"))),
        },
        None => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.point"), "expected point string")),
    }
    for field in ["iteration", "instance"] {
        match el.get(field) {
            None if field == "instance" => {}
            Some(Value::Str(x)) if x == "*" => {}
            Some(Value::Arr(a)) if a.iter().all(|x| u64_of(x).is_some()) => {}
            _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.{field}"), "must be \"*\" or an array of nonnegative integers")),
        }
    }
    if let Some(ci) = el.get("call_index") {
        match ci {
            Value::Str(x) if x == "*" => {}
            x if u64_of(x).is_some() => {}
            _ => errs.push(rej("E_CERT_SCHEDULE_BINDING", "§11", format!("{path}.call_index"), "must be a nonnegative integer or \"*\"")),
        }
    }
}

// ---------- frame descriptor (section 9.4) ----------

pub fn check_frame(frame: &Value, symbol: Option<&Symbol>, docs: &LoadedDocs, base: &str, report: &mut RegionReport) {
    let path = format!("{base}.frame");
    let mut errs = Vec::new();
    if !keys_strict(
        frame,
        &["slots", "scratch", "history", "borrow", "ancillae", "live_ranges"],
        &["slots", "scratch", "history", "borrow", "ancillae"],
        &path,
        "E_CERT_FRAME",
        "§9.4",
        &mut errs,
    ) {
        report.errors.extend(errs);
        return;
    }
    let classes: Option<HashSet<String>> = docs.target.as_ref().map(|t| target_classes(t).into_iter().collect());
    let mut identities: HashSet<String> = HashSet::new();
    let mut slot_ids: Vec<String> = Vec::new();
    let check_records = |key: &str, fields: &[&str], errs: &mut Vec<Reject>, identities: &mut HashSet<String>| -> Vec<Value> {
        let arr = frame.get(key).and_then(Value::as_arr).map(|a| a.to_vec()).unwrap_or_default();
        for (i, rec) in arr.iter().enumerate() {
            keys_strict(rec, fields, fields, &format!("{path}.{key}[{i}]"), "E_CERT_FRAME", "§9.4", errs);
            if let Some(id) = rec.get("id").and_then(Value::as_str) {
                if key != "slots" && !identities.insert(id.to_string()) {
                    errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.{key}[{i}].id"), "frame identities must be distinct across the descriptor"));
                }
                if key != "slots" && id.is_empty() {
                    errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.{key}[{i}].id"), "frame identity must be nonempty"));
                }
            }
            if let (Some(cs), Some(cls)) = (&classes, rec.get("class").and_then(Value::as_str)) {
                if !cs.contains(cls) {
                    errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.{key}[{i}].class"), "class is not a resolved target class"));
                }
            }
        }
        arr
    };
    let slots = check_records("slots", &["id", "role", "class", "width"], &mut errs, &mut identities);
    for srec in &slots {
        if let Some(id) = srec.get("id").and_then(Value::as_str) {
            slot_ids.push(id.to_string());
        }
        match srec.get("role").and_then(Value::as_str) {
            Some("in" | "tgt" | "fault" | "mut" | "mem_in" | "mem_mut") => {}
            _ => errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.slots"), "slot role must be in/tgt/fault/mut/mem_in/mem_mut")),
        }
    }
    check_records("scratch", &["id", "class", "state", "width"], &mut errs, &mut identities);
    check_records("history", &["id", "class", "width"], &mut errs, &mut identities);
    check_records("borrow", &["id", "class", "width"], &mut errs, &mut identities);
    check_records("ancillae", &["id", "scheme", "class", "state", "width"], &mut errs, &mut identities);
    for key in ["scratch", "ancillae"] {
        if let Some(arr) = frame.get(key).and_then(Value::as_arr) {
            for (i, rec) in arr.iter().enumerate() {
                match rec.get("state").and_then(Value::as_str) {
                    Some("clean" | "dirty") => {}
                    _ => errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.{key}[{i}].state"), "state must be clean or dirty")),
                }
            }
        }
    }
    // Slots must be the source interface in operand order with matching widths.
    if let Some(sym) = symbol {
        if let Ok(iface) = docs.object.interface(&sym.name) {
            let mut want: Vec<(String, String, u64)> = Vec::new();
            for b in &iface.bundles {
                want.push((format!("{}:{}", b.group, b.slot), b.group.clone(), b.width));
            }
            for r in &iface.regions {
                let cells = if r.a >= 64 { u64::MAX } else { 1u64 << r.a };
                want.push((format!("mem:{}", want.iter().filter(|(_, g, _)| g == "mem_in" || g == "mem_mut").count()), if r.role == "in" { "mem_in".to_string() } else { "mem_mut".to_string() }, cells.saturating_mul(r.w)));
            }
            if slots.len() != want.len() {
                errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.slots"), format!("frame lists {} slots; the interface has {}", slots.len(), want.len())));
            } else {
                for (rec, (wid, wrole, _ww)) in slots.iter().zip(want.iter()) {
                    if rec.get("id").and_then(Value::as_str) != Some(wid.as_str())
                        || rec.get("role").and_then(Value::as_str) != Some(wrole.as_str())
                    {
                        errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.slots"), "slots must list the source interface in operand order (in/tgt/fault then regions, or mut/in then regions)"));
                        break;
                    }
                    // Widths are section 8.1 expressions in c only; exact
                    // equality with the signature is checked when the
                    // expression is a literal number.
                    if let Some(w) = rec.get("width").and_then(u64_of) {
                        if w != *_ww && *_ww != u64::MAX {
                            errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.slots"), format!("slot {wid} literal width {w} != signature width {_ww}")));
                        }
                    }
                }
            }
        }
    }
    // live_ranges: shared-frame intervals of one frame must be disjoint.
    if let Some(Value::Arr(ranges)) = frame.get("live_ranges") {
        let mut by_frame: Vec<(String, i128, i128)> = Vec::new();
        for (i, r) in ranges.iter().enumerate() {
            keys_strict(r, &["frame", "start", "end"], &["frame", "start", "end"], &format!("{path}.live_ranges[{i}]"), "E_CERT_FRAME", "§9.4", &mut errs);
            let name = r.get("frame").and_then(Value::as_str).unwrap_or("");
            let start = r.get("start").and_then(int_of).unwrap_or(-1);
            let end = r.get("end").and_then(int_of).unwrap_or(-1);
            if !identities.contains(name) {
                errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.live_ranges[{i}].frame"), "names no non-slot frame identity"));
            }
            if !(0 <= start && start < end) {
                errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.live_ranges[{i}]"), "live range must be a half-open interval with start < end"));
            }
            by_frame.push((name.to_string(), start, end));
        }
        by_frame.sort();
        for w in by_frame.windows(2) {
            if w[0].0 == w[1].0 && w[1].1 < w[0].2 {
                errs.push(rej("E_CERT_FRAME", "§9.4", format!("{path}.live_ranges"), format!("shared frame {:?} has overlapping live ranges", w[0].0)));
            }
        }
    }
    report.errors.extend(errs);
    let _ = slot_ids;
}

// ---------- region reports / cleanup / error maps (shapes) ----------

pub fn check_region_reports(region: &Value, docs: &LoadedDocs, base: &str, report: &mut RegionReport) {
    let mut errs = Vec::new();
    if let Some(res) = region.get("resources") {
        match res.as_obj() {
            Some(m) => {
                for (k, v) in m {
                    if rat_of_json(v).is_err() {
                        errs.push(schema_err(format!("{base}.resources.{k}"), "resource values are decimals"));
                    }
                }
            }
            None => errs.push(schema_err(format!("{base}.resources"), "expected object")),
        }
    }
    if let Some(g) = region.get("gates_used") {
        match g.as_obj() {
            Some(m) => {
                for (gate, entry) in m {
                    let p = format!("{base}.gates_used.{gate}");
                    if keys_strict(entry, &["count", "scheme", "epsilon"], &["count"], &p, "E_CERT_SCHEMA", "§12.3", &mut errs) {
                        if entry.get("count").and_then(u64_of).is_none() {
                            errs.push(schema_err(format!("{p}.count"), "expected nonnegative integer"));
                        }
                        if let Some(e) = entry.get("epsilon") {
                            if rat_of_json(e).is_err() {
                                errs.push(schema_err(format!("{p}.epsilon"), "expected decimal"));
                            }
                        }
                    }
                }
            }
            None => errs.push(schema_err(format!("{base}.gates_used"), "expected object")),
        }
    }
    let _ = docs;
    report.errors.extend(errs);
    // Resource/gate-count recomputation needs the scheduled event list
    // (section 8.2 over the section 11.1 expansion). Until the checker
    // re-derives that expansion these producer reports are shape-checked
    // only; the verdict's establishes lines say so.
}

pub fn check_cleanup_entries(region: &Value, docs: &LoadedDocs, base: &str, report: &mut RegionReport) {
    let Some(cleanup) = region.get("cleanup").and_then(Value::as_obj).cloned() else {
        if region.get("cleanup").is_some() {
            report.errors.push(schema_err(format!("{base}.cleanup"), "expected object"));
        }
        return;
    };
    for (key, entries) in &cleanup {
        let p = format!("{base}.cleanup.{key}");
        match key.rsplit_once(':') {
            Some((variant, controls)) if !variant.is_empty() && controls.parse::<u64>().is_ok() => {}
            _ => {
                report.errors.push(schema_err(p.clone(), "cleanup keys are \"<variant>:<inherited_controls>\""));
                continue;
            }
        }
        let Some(list) = entries.as_arr() else {
            report.errors.push(schema_err(p.clone(), "expected array of {path, realisation}"));
            continue;
        };
        for (i, e) in list.iter().enumerate() {
            let ep = format!("{p}[{i}]");
            let mut errs = Vec::new();
            if keys_strict(e, &["path", "realisation"], &["path", "realisation"], &ep, "E_CERT_SCHEMA", "§12.3", &mut errs) {
                match e.get("realisation").and_then(Value::as_str) {
                    Some("reverse" | "recompute") => {}
                    _ => errs.push(rej("E_CERT_SCHEMA", "§12.3", format!("{ep}.realisation"), "must be reverse or recompute (§17 item 6 defers measurement-based cleanup)")),
                }
                if let Some(Value::Arr(path)) = e.get("path") {
                    for (j, el) in path.iter().enumerate() {
                        validate_path_element(el, &docs.object, &format!("{ep}.path[{j}]"), &mut errs);
                    }
                } else {
                    errs.push(schema_err(format!("{ep}.path"), "expected schedule path array"));
                }
            }
            report.errors.extend(errs);
        }
    }
}

pub fn check_error_map(region: &Value, base: &str, errs: &mut Vec<Reject>) {
    let Some(map) = region.get("error").and_then(Value::as_obj).cloned() else {
        if region.get("error").is_some() {
            errs.push(schema_err(format!("{base}.error"), "expected object"));
        }
        return;
    };
    for (key, entry) in &map {
        let p = format!("{base}.error.{key}");
        match key.rsplit_once(':') {
            Some((variant, controls)) if !variant.is_empty() && controls.parse::<u64>().is_ok() => {}
            _ => {
                errs.push(schema_err(p.clone(), "error keys are \"<variant>:<inherited_controls>\""));
                continue;
            }
        }
        if keys_strict(entry, &["local", "inclusive"], &["local", "inclusive"], &p, "E_CERT_SCHEMA", "§12.3", errs) {
            for part in ["local", "inclusive"] {
                if let Some(m) = entry.get(part) {
                    keys_strict(m, &["operator_norm", "diamond"], &[], &format!("{p}.{part}"), "E_CERT_SCHEMA", "§12.3", errs);
                    if let Some(mm) = m.as_obj() {
                        for (metric, v) in mm {
                            if rat_of_json(v).is_err() {
                                errs.push(schema_err(format!("{p}.{part}.{metric}"), "expected decimal"));
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------- claim verifiers (implemented in the numeric slice) ----------

pub fn check_numerical_claim(
    _claim: &Value,
    _path: &str,
    _region_hash: &str,
    _docs: &LoadedDocs,
    _cert_schedule_hash: Option<&str>,
    _inputs: &Inputs,
) -> ClaimOutcome {
    ClaimOutcome::unchecked("numerical checker not yet implemented in this slice")
}

pub fn check_channel_claim(
    _claim: &Value,
    _path: &str,
    _region_hash: &str,
    _docs: &LoadedDocs,
    _cert_schedule_hash: Option<&str>,
    _inputs: &Inputs,
) -> ClaimOutcome {
    ClaimOutcome::unchecked("channel checker not yet implemented in this slice")
}

#[allow(clippy::too_many_arguments)]
pub fn check_structural_claim(
    _claim: &Value,
    _path: &str,
    _region_hash: &str,
    _region_symbol: Option<&Symbol>,
    _region: &Value,
    _docs: &LoadedDocs,
    _root_invocation: &Option<Invocation>,
    _libraries: &[Value],
    _all_claims: &[Value],
    _prior: &[(String, ClaimOutcome)],
    _inputs: &Inputs,
) -> ClaimOutcome {
    ClaimOutcome::unchecked("structural checker not yet implemented in this slice")
}

pub fn check_composition(
    _comp: &Value,
    _cert: &Value,
    _docs: &LoadedDocs,
    _regions: &[RegionReport],
    _errs: &mut Vec<Reject>,
) {
    // Shape + arithmetic implemented in the numeric slice.
}
