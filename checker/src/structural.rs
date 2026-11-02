//! Syntactic certificate rules from ISA §§5.6 and 12.1. In particular the
//! translation lemma proves independence from arbitrary borrowed values;
//! no chosen initial borrow value or basis replay can stand in for it.
use crate::cert::{
    check_keys, claim_subject, parse_invocation, rej, schema_err, u64_of, ClaimOutcome, LoadedDocs,
};
use crate::eval::{list_items, parse_blocks, tokenise, Stmt};
use crate::json::Value;
use crate::object::Symbol;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
type Result<T> = std::result::Result<T, ClaimOutcome>;
fn bad<T>(path: &str, msg: impl AsRef<str>) -> Result<T> {
    Err(ClaimOutcome::rejected(vec![rej(
        "E_CERT_STRUCTURAL",
        "§5.6",
        path,
        msg,
    )]))
}
fn strings(v: Option<&Value>, path: &str) -> Result<Vec<String>> {
    v.and_then(Value::as_arr)
        .and_then(|a| a.iter().map(|v| v.as_str().map(str::to_string)).collect())
        .ok_or_else(|| {
            ClaimOutcome::rejected(vec![schema_err(path, "expected an array of strings")])
        })
}
#[derive(Debug, Clone, PartialEq)]
struct Flip {
    controls: HashSet<String>,
    target: String,
}
#[derive(Debug,Clone)]
pub(crate) struct BorrowFrameEvidence {
    universal:bool,
    closed:bool,
    touches:BTreeSet<(usize,String)>,
    protected:BTreeSet<(usize,String)>,
}

pub(crate) fn discharge_borrow_frames(region:&Value,docs:&LoadedDocs,report:&mut crate::cert::RegionReport){
    if docs.schedule.is_some() || !report.errors.is_empty() || report.claims.iter().any(|(_,c)|c.outcome=="reject") {return;}
    if !report.unchecked_items.iter().any(|s|s==crate::cert::BORROW_PROOF_PENDING){return;}
    let Some(indices)=region.get("verification").and_then(|v|v.get("borrow_frames")).and_then(Value::as_arr) else{return;};
    let Some(universe)=report.claims.iter().filter_map(|(_,c)|c.borrow_frame.as_ref()).find(|e|e.universal && e.closed) else{return;};
    let mut protected=BTreeSet::new();
    for index in indices {
        let evidence=index.as_i128().and_then(|i|usize::try_from(i).ok()).and_then(|i|report.claims.get(i)).and_then(|(_,c)|c.borrow_frame.as_ref());
        if let Some(e)=evidence {if e.universal && e.closed && e.touches==universe.touches {protected.extend(e.protected.iter().cloned());}}
    }
    if universe.touches.is_subset(&protected) {
        report.unchecked_items.retain(|s|s!=crate::cert::BORROW_PROOF_PENDING);
        report.establishes.push("indexed universal translation proofs cover every borrowed-wire touch in this reference region, on every counted-loop visit; their composition restores the borrowed lifetimes independently of arbitrary borrowed states and external references (§5.6)".into());
    }
}
fn wire(v: &str, widths: &HashMap<String, u64>) -> Result<String> {
    let (name, index) = if let Some((name, i)) = v.strip_suffix(']').and_then(|v| v.split_once('['))
    {
        (
            name,
            i.parse::<u64>().map_err(|_| {
                ClaimOutcome::unchecked(
                    "symbolic wire index requires invocation-dependent structural expansion",
                )
            })?,
        )
    } else {
        (v, 0)
    };
    let width = widths.get(name).copied().ok_or_else(|| {
        ClaimOutcome::rejected(vec![rej(
            "E_CERT_STRUCTURAL",
            "§5.6",
            "witness",
            format!("wire {v:?} is not in the checked lifetime"),
        )])
    })?;
    if index >= width || (!v.ends_with(']') && width != 1) {
        return bad("witness", format!("{v:?} does not denote one live wire"));
    }
    Ok(format!("{name}[{index}]"))
}
fn set(v: Vec<String>, widths: &HashMap<String, u64>, path: &str) -> Result<HashSet<String>> {
    let resolved: Vec<String> = v.iter().map(|v| wire(v, widths)).collect::<Result<_>>()?;
    let unique: HashSet<String> = resolved.iter().cloned().collect();
    if unique.len() != resolved.len() {
        return bad(path, "wire occurs twice after resolution");
    }
    Ok(unique)
}
fn point(v: &str) -> Option<(&str, usize)> {
    let (b, i) = v.rsplit_once(':')?;
    Some((b, i.parse().ok()?))
}

#[derive(Default)]
struct Sequences {
    owner: HashMap<usize, usize>,
    loops: HashSet<usize>,
    bounds: Vec<u64>,
}
fn sequence_layout(
    stmts: &[Stmt],
    sequence: usize,
    ordinal: &mut usize,
    next: &mut usize,
    layout: &mut Sequences,
    depth: usize,
) -> Result<()> {
    if depth > 64 {
        return Err(crate::cert::host_limit_outcome(crate::fault::HostLimit {
            quantity: "structural source nesting exceeds 64 host levels".into(),
        }));
    }
    for stmt in stmts {
        let at = *ordinal;
        *ordinal += 1;
        layout.owner.insert(at, sequence);
        match stmt {
            Stmt::Line(tokens) if depth > 0 => {
                let mut i = 0;
                while tokens.get(i).map(String::as_str) == Some("ctl") {
                    i += 2;
                }
                if !matches!(
                    tokens.get(i).map(String::as_str),
                    Some("x" | "cx" | "ccx" | "mcx")
                ) {
                    return Err(ClaimOutcome::unchecked("structural loop lifetime/predicate/call changes require visit-specific expansion; the current static proof handles X-family loop bodies"));
                }
            }
            Stmt::Block { head, body, rest } => {
                if head.first().map(String::as_str) != Some("qloop")
                    || head.len() != 2
                    || rest.is_some()
                {
                    return Err(ClaimOutcome::unchecked("structural qif/qloop-while control and lifetime expansion has not been implemented"));
                }
                let count = head[1].parse::<u64>().map_err(|_| {
                    crate::cert::host_limit_outcome(crate::fault::HostLimit {
                        quantity: "structural loop count exceeds host integer representation"
                            .into(),
                    })
                })?;
                layout.bounds.push(count);
                layout.loops.insert(at);
                let child = *next;
                *next += 1;
                sequence_layout(body, child, ordinal, next, layout, depth + 1)?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn check(
    claim: &Value,
    path: &str,
    region_hash: &str,
    symbol: Option<&Symbol>,
    docs: &LoadedDocs,
    prior: &[(String, ClaimOutcome)],
) -> ClaimOutcome {
    match inner(claim, path, region_hash, symbol, docs, prior) {
        Ok(o) | Err(o) => o,
    }
}
fn inner(
    claim: &Value,
    path: &str,
    region_hash: &str,
    symbol: Option<&Symbol>,
    docs: &LoadedDocs,
    prior: &[(String, ClaimOutcome)],
) -> Result<ClaimOutcome> {
    let mut errs = vec![];
    check_keys(
        claim,
        &[
            "kind",
            "rule",
            "subclaims",
            "witness",
            "invocation",
            "subject",
        ],
        &["kind", "rule", "subclaims", "witness", "invocation"],
        path,
        &mut errs,
    );
    let rule = claim.get("rule").and_then(Value::as_str).unwrap_or("");
    if !matches!(
        rule,
        "translation_lemma" | "inverse_pair_library" | "adjoint_table"
    ) {
        errs.push(schema_err(
            format!("{path}.rule"),
            "unknown structural rule",
        ));
    }
    let Some(symbol) = symbol else {
        return Ok(ClaimOutcome::unchecked(
            "structural claim's region is not linked",
        ));
    };
    let iface = docs
        .object
        .interface(&symbol.name)
        .map_err(ClaimOutcome::unchecked)?;
    let invocation = claim.get("invocation");
    let universal = invocation
        .map(|i| {
            i.get("args").and_then(Value::as_str) == Some("all")
                && i.get("budget").and_then(Value::as_str) == Some("all")
        })
        .unwrap_or(false);
    if let Some(inv) = invocation {
        if universal {
            check_keys(
                inv,
                &["args", "budget"],
                &["args", "budget"],
                &format!("{path}.invocation"),
                &mut errs,
            );
        } else if let Err(e) = parse_invocation(
            inv,
            &iface.args,
            &docs.object,
            &format!("{path}.invocation"),
        ) {
            errs.extend(e);
        }
    }
    if claim.get("subject").is_some() {
        match claim_subject(claim,path,region_hash,false) {
            Ok(s) if s.closure_hash!=region_hash=>errs.push(rej("E_CERT_BINDING","§12.3",format!("{path}.subject.closure_hash"),"structural subject names a different region")),
            Ok(s) if s.schedule_hash.is_some() || s.inherited_controls!=0=>return Ok(ClaimOutcome::unchecked("structural proof under inherited/scheduled controls needs resolved invocation contexts")),
            Ok(_)=>{},Err(o)=>errs.extend(o.errors),
        }
    }
    let indices = claim.get("subclaims").and_then(Value::as_arr);
    let mut subclaims_pending = false;
    if let Some(indices) = indices {
        for value in indices {
            match u64_of(value)
                .and_then(|i| usize::try_from(i).ok())
                .and_then(|i| prior.get(i))
            {
                Some((_, o)) if matches!(o.outcome.as_str(), "proof" | "accept") => {}
                Some((_, o)) if o.outcome == "reject" => errs.push(rej(
                    "E_CERT_STRUCTURAL",
                    "§12.1",
                    format!("{path}.subclaims"),
                    "a required subclaim was rejected",
                )),
                Some(_) => subclaims_pending = true,
                None => errs.push(rej(
                    "E_CERT_VERIFICATION_INDEX",
                    "§12.1",
                    format!("{path}.subclaims"),
                    "subclaim index is invalid or out of range",
                )),
            }
        }
    } else {
        errs.push(schema_err(
            format!("{path}.subclaims"),
            "expected an array of claim indices",
        ));
    }
    let witness = claim.get("witness").unwrap_or(&Value::Null);
    match rule {
        "translation_lemma" => {
            check_keys(
                witness,
                &["V", "W", "X", "B", "frame"],
                &["V", "W", "X", "B", "frame"],
                &format!("{path}.witness"),
                &mut errs,
            );
        }
        "inverse_pair_library" => {
            check_keys(
                witness,
                &["library_closure_hash", "claim_index"],
                &["library_closure_hash", "claim_index"],
                &format!("{path}.witness"),
                &mut errs,
            );
            if witness
                .get("library_closure_hash")
                .and_then(Value::as_str)
                .map(crate::is_hash_hex)
                != Some(true)
                || witness.get("claim_index").and_then(u64_of).is_none()
            {
                errs.push(schema_err(
                    format!("{path}.witness"),
                    "expected library closure hash and nonnegative claim index",
                ));
            }
        }
        "adjoint_table" => {
            check_keys(witness, &[], &[], &format!("{path}.witness"), &mut errs);
        }
        _ => {}
    }
    if !errs.is_empty() {
        return Err(ClaimOutcome::rejected(errs));
    }
    if subclaims_pending {
        return Ok(ClaimOutcome::unchecked("structural rule requires proof from every subclaim; evidence/unchecked outcomes discharge none"));
    }
    if rule == "inverse_pair_library" {
        return Ok(ClaimOutcome::unchecked("linked library claim needs independently checked source/certificate provenance and quantified argument coverage; producer assertions are not trusted"));
    }
    if rule == "adjoint_table" {
        return Ok(ClaimOutcome::unchecked("adjoint_table needs a checked correspondence from the forward subclaim to this body's reversed primitive operations"));
    }
    translation(witness, path, symbol, docs, universal)
}
fn translation(
    witness: &Value,
    path: &str,
    symbol: &Symbol,
    docs: &LoadedDocs,
    universal: bool,
) -> Result<ClaimOutcome> {
    let v = strings(witness.get("V"), path)?;
    let w = strings(witness.get("W"), path)?;
    let frame = strings(witness.get("frame"), path)?;
    if frame.len() != 2 || v.is_empty() || w.is_empty() {
        return bad(
            path,
            "translation frame requires two endpoints and nonempty V and W sequences",
        );
    }
    let (start_block, start) = point(&frame[0]).ok_or_else(|| {
        ClaimOutcome::rejected(vec![schema_err(path, "invalid first frame point")])
    })?;
    let (end_block, end) = point(&frame[1]).ok_or_else(|| {
        ClaimOutcome::rejected(vec![schema_err(path, "invalid last frame point")])
    })?;
    if start_block != end_block || start > end {
        return bad(
            path,
            "frame endpoints must be ordered in one static instruction sequence",
        );
    }
    let f = docs
        .object
        .function(&symbol.name)
        .ok_or_else(|| ClaimOutcome::unchecked("structural subject has no body"))?;
    let block = f
        .blocks
        .iter()
        .find(|b| b.name == start_block)
        .ok_or_else(|| {
            ClaimOutcome::rejected(vec![schema_err(path, "frame block does not exist")])
        })?;
    if f.blocks.len() != 1 {
        return Ok(ClaimOutcome::unchecked(
            "multi-block structural lifetime expansion has not been implemented",
        ));
    }
    if end >= block.instructions.len() {
        return bad(path, "frame endpoint is beyond the block");
    }
    // This necessary side condition is decidable even when other loop
    // lifetime mechanics are not yet supported.
    for line in &block.instructions[start..=end] {
        let tokens = tokenise(line);
        let mut t = tokens.as_slice();
        while t.first().map(String::as_str) == Some("ctl") {
            t = t.get(2..).unwrap_or(&[]);
        }
        if !matches!(
            t.first().map(String::as_str),
            Some("x" | "cx" | "ccx" | "mcx")
        ) {
            return bad(
                path,
                "translation frame contains a non-X-family instruction",
            );
        }
    }
    let parsed = parse_blocks(f);
    let stmts = &parsed
        .iter()
        .find(|(name, _)| name == start_block)
        .expect("parsed source block exists")
        .1;
    let mut sequences = Sequences::default();
    let mut ordinal = 0;
    let mut next = 1;
    sequence_layout(stmts, 0, &mut ordinal, &mut next, &mut sequences, 0)?;
    if sequences.bounds != f.bounds {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_OBJECT",
            "§5.3",
            format!("object.@{}.bounds", f.name),
            "declared bounds differ from static qloop counts in text order",
        )]));
    }
    let owner = sequences.owner.get(&start);
    if owner.is_none()
        || owner != sequences.owner.get(&end)
        || sequences.loops.iter().any(|i| (start..=end).contains(i))
    {
        return bad(
            path,
            "frame must lie within one containing static instruction sequence",
        );
    }
    let owner = *owner.unwrap();
    let mut widths: HashMap<String, u64> = docs
        .object
        .interface(&symbol.name)
        .map_err(ClaimOutcome::unchecked)?
        .bundles
        .into_iter()
        .map(|b| (b.name, b.width))
        .collect();
    let mut predicates = vec![];
    let mut active_borrow = HashSet::new();
    let mut active_bundles=HashSet::new();
    let mut valid_returns=true;
    let mut touches=BTreeSet::new();
    let mut frame_borrow = HashSet::new();
    let mut gates: BTreeMap<usize, Flip> = BTreeMap::new();
    for (i, line) in block.instructions.iter().enumerate() {
        // Counted X-only bodies preserve names, borrowed lifetimes and the
        // predicate stack across every visit. Their header emits no flip.
        if sequences.loops.contains(&i) {
            continue;
        }
        let tokens = tokenise(line);
        let mut t = tokens.as_slice();
        let mut controls = predicates.clone();
        while t.first().map(String::as_str) == Some("ctl") {
            controls.push(wire(t.get(1).map(String::as_str).unwrap_or(""), &widths)?);
            t = &t[2..];
        }
        let op = t.first().map(String::as_str).unwrap_or("");
        if (start..=end).contains(&i) && !matches!(op, "x" | "cx" | "ccx" | "mcx") {
            return bad(path,format!("frame contains {op:?}; the lemma admits a contiguous sequence of X-family flips"));
        }
        if t.len() >= 3 && t[1] == "=" && t[2] == "qborrow" {
            let width = t
                .get(4)
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| {
                    ClaimOutcome::unchecked("symbolic qborrow width requires resolved expansion")
                })?;
            if width > 1_000_000 {
                return Ok(crate::cert::host_limit_outcome(crate::fault::HostLimit {
                    quantity: "structural borrow wire count".into(),
                }));
            }
            if widths.insert(t[0].clone(), width).is_some() {
                return Ok(ClaimOutcome::unchecked(
                    "reused borrow identifiers need lifetime-specific structural resolution",
                ));
            }
            active_bundles.insert(t[0].clone());
            for bit in 0..width {
                active_borrow.insert(format!("{}[{bit}]", t[0]));
            }
        } else if op == "qreturn" {
            let name = t.get(1).map(String::as_str).unwrap_or("");
            valid_returns &= active_bundles.remove(name);
            active_borrow.retain(|w| !w.starts_with(&format!("{name}[")));
        } else if op == "qpred" {
            predicates.push(wire(t.get(1).map(String::as_str).unwrap_or(""), &widths)?);
        } else if op == "qunpred" {
            if predicates.pop() != Some(wire(t.get(1).map(String::as_str).unwrap_or(""), &widths)?)
            {
                return bad(path, "unbalanced structural predicate stack");
            }
        } else {
            if !matches!(op, "x" | "cx" | "ccx" | "mcx") {
                if i >= start && i <= end {
                    return bad(
                        path,
                        format!("frame contains {op:?}; the lemma admits only X-family flips"),
                    );
                }
                return Ok(ClaimOutcome::unchecked(format!("entry/lifetime analysis of {op:?} outside the structural frame has not been implemented")));
            }
            let target = match op {
                "x" => t.get(1),
                "cx" => {
                    controls.push(wire(t.get(1).map(String::as_str).unwrap_or(""), &widths)?);
                    t.get(2)
                }
                "ccx" => {
                    for operand in t.iter().skip(1).take(2) {
                        controls.push(wire(operand, &widths)?);
                    }
                    t.get(3)
                }
                "mcx" => {
                    for operand in list_items(t.get(1).map(String::as_str).unwrap_or("()")) {
                        controls.push(wire(&operand, &widths)?);
                    }
                    t.get(2)
                }
                _ => unreachable!(),
            };
            let target = wire(target.map(String::as_str).unwrap_or(""), &widths)?;
            let controls: HashSet<_> = controls.into_iter().collect();
            if controls.contains(&target) {
                return bad(path, "flip target also occurs as a control");
            }
            for w in controls.iter().chain(std::iter::once(&target)) {
                if active_borrow.contains(w) {touches.insert((i,w.clone()));}
            }
            gates.insert(i, Flip { controls, target });
        }
        if i == start {
            frame_borrow = active_borrow.clone();
        }
    }
    let x = set(strings(witness.get("X"), path)?, &widths, path)?;
    let b = set(strings(witness.get("B"), path)?, &widths, path)?;
    if b.is_empty() || !b.is_subset(&frame_borrow) || !x.is_disjoint(&b) {
        return bad(
            path,
            "B must name live borrowed wires, and X must be disjoint from B",
        );
    }
    let selected: Vec<_> = gates
        .range(start..=end)
        .map(|(i, g)| (*i, g.clone()))
        .collect();
    if selected.len() != 2 * (v.len() + w.len())
        || selected.first().map(|(i, _)| *i) != Some(start)
        || selected.last().map(|(i, _)| *i) != Some(end)
    {
        return bad(path, "frame is not exactly V;W;reverse(V);W");
    }
    let v_gates = &selected[..v.len()];
    let w_gates = &selected[v.len()..v.len() + w.len()];
    for (points, actual) in [(&v[..], v_gates), (&w[..], w_gates)] {
        for (p, (i, _)) in points.iter().zip(actual) {
            if point(p) != Some((start_block, *i)) {
                return bad(
                    path,
                    "V/W point lists must identify the complete leading V and W in order",
                );
            }
        }
    }
    let reverse_v = &selected[v.len() + w.len()..2 * v.len() + w.len()];
    let second_w = &selected[2 * v.len() + w.len()..];
    if !v_gates
        .iter()
        .rev()
        .map(|(_, g)| g)
        .eq(reverse_v.iter().map(|(_, g)| g))
        || !w_gates
            .iter()
            .map(|(_, g)| g)
            .eq(second_w.iter().map(|(_, g)| g))
    {
        return bad(
            path,
            "V inverse or second W differs in gates, targets or effective control sets",
        );
    }
    for (_, gate) in v_gates {
        if !b.contains(&gate.target) || !gate.controls.is_subset(&x) {
            return bad(path, "V must target only B and control only on X");
        }
    }
    let all_w_controls: HashSet<_> = w_gates
        .iter()
        .flat_map(|(_, g)| g.controls.iter().cloned())
        .collect();
    for (_, gate) in w_gates {
        if x.contains(&gate.target)
            || b.contains(&gate.target)
            || all_w_controls.contains(&gate.target)
        {
            return bad(path, "W target lies in X, B or the union of all W controls");
        }
        if gate.controls.intersection(&b).count() > 1 {
            return bad(path, "W gate has more than one borrowed control");
        }
    }
    for (i, gate) in &gates {
        if sequences.owner.get(i) == Some(&owner)
            && (*i < start || *i > end)
            && (b.contains(&gate.target) || !gate.controls.is_disjoint(&b))
        {
            return bad(path, "borrow frame is not maximal for B");
        }
    }
    let protected=touches.iter().filter(|(i,w)|(start..=end).contains(i) && b.contains(w)).cloned().collect();
    let evidence=BorrowFrameEvidence{universal,closed:valid_returns && active_bundles.is_empty() && predicates.is_empty(),touches,protected};
    Ok(ClaimOutcome{outcome:"accept".into(),establishes:format!("reference borrow frame {}..{} satisfies every translation-lemma side condition, restores B and acts independently of arbitrary borrowed states and external references{}{} (§5.6)",frame[0],frame[1],if owner==0 {""}else{" on every visit of its containing counted-loop body"},if universal {" for all well-typed classical arguments and budgets"} else {" at the explicit invocation"}),borrow_frame:Some(evidence),..Default::default()})
}
