//! K92 owner inventory. Reached invocation applicability is a separate check.
use crate::{
    cert::{rej, LoadedDocs, RegionReport},
    json::Value,
    object::Symbol,
};

pub(crate) fn inventory(
    region: &Value,
    symbol: Option<&Symbol>,
    docs: &LoadedDocs,
    base: &str,
    report: &mut RegionReport,
) {
    let Some(function) = symbol.and_then(|s| docs.object.function(&s.name)) else {
        return;
    };
    let pairs = region
        .get("verification")
        .and_then(|v| v.get("inverse_pairs"))
        .and_then(Value::as_arr)
        .unwrap_or(&[]);
    let mut sites = 0;
    let mut matched_sites = 0;
    let initial_errors = report.errors.len();
    for tokens in crate::eval::find_opcodes(function) {
        let mut row = tokens.as_slice();
        while row.first().map(String::as_str) == Some("ctl") && row.len() >= 2 {
            row = &row[2..];
        }
        if row.first().map(String::as_str) == Some("adj") {
            row = &row[1..];
        }
        if row.first().map(String::as_str) != Some("qinplace") {
            continue;
        }
        sites += 1;
        let mut closures = vec![];
        for i in [1, 2] {
            let name = row.get(i).and_then(|s| s.strip_prefix('@')).unwrap_or("");
            match docs.object.symbol(name) {
                Some(s) if s.kind=="oracle" =>match &s.closure_hash {
                    Some(hash)=>closures.push(hash.clone()),
                    None=>report.unchecked_items.push(format!("qinplace @{name} closure is unresolved; exact owner pair inventory remains unchecked")),
                },
                None if docs.object.imports.contains_key(name)=>report.unchecked_items.push(format!("qinplace @{name} needs its linked imported owner/proof record")),
                _=>report.errors.push(rej("E_CERT_OBJECT","§4.5/K92",base,"qinplace must name two resolved oracle subjects")),
            }
        }
        if closures.len() != 2 {
            continue;
        }
        let matching = pairs.iter().any(|pair| {
            let g = pair.get("g").and_then(Value::as_str);
            let h = pair.get("ginv").and_then(Value::as_str);
            (g == Some(closures[0].as_str()) && h == Some(closures[1].as_str()))
                || (g == Some(closures[1].as_str()) && h == Some(closures[0].as_str()))
        });
        if !matching {
            report.errors.push(rej("E_CERT_INVERSE_PAIR","§4.5/§12.3/K92",format!("{base}.verification.inverse_pairs"),format!("qinplace in owner @{} requires an entry for exact pair ({},{}) in either orientation",function.name,closures[0],closures[1])));
        } else {
            matched_sites += 1;
        }
    }
    if sites > 0 && matched_sites == sites && report.errors.len() == initial_errors {
        report.establishes.push(format!("all {sites} source qinplace site(s) have exact pair entries in their owner region record; invocation budget/argument/token applicability remains separate (§12.3/K92)"));
    }
}
