//! Schedule-bound native action realisations, independently derived from
//! ISA §§4.4, 4.7, 8.3, 11 and 12.1. The traversal includes default-covered
//! call contexts. Unsupported expansions terminate with an explicit reason.
use crate::cert::{
    host_limit_outcome, parse_invocation, rej, u64_of, ClaimOutcome, Inputs, Invocation,
    LoadedDocs, Subject,
};
use crate::eval::{list_items, tokenise};
use crate::fault::HostLimit;
use crate::json::{jcs, n, s, Value};
use crate::matrix::{ComplexInterval as C, Matrix};
use crate::rat::{rat_of_json, Interval, Rat};
use num_traits::{One, Zero};
use std::collections::{BTreeMap, HashMap, HashSet};

type Result<T> = std::result::Result<T, ClaimOutcome>;
fn pending<T>(reason: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::unchecked(reason))
}
fn invalid<T>(path: &str, reason: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::rejected(vec![rej(
        "E_SCHEDULE_INFEASIBLE",
        "§11",
        path,
        reason.into(),
    )]))
}
#[derive(Debug, Clone)]
pub struct Context {
    pub symbol: String,
    pub controls: u64,
    pub invocation: Invocation,
    pub key: Vec<Value>,
    pub adjoint: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct Gate {
    pub(crate) op: String,
    pub(crate) own_controls: usize,
    pub(crate) controls: Vec<usize>,
    pub(crate) targets: Vec<usize>,
    pub(crate) path: Vec<Value>,
    angle_bits: Option<u64>,
    adjoint: bool,
}
fn resolve_angle(text:&str,context:&Context,docs:&LoadedDocs)->Result<u64> {
    let bad=|message:String|ClaimOutcome::rejected(vec![rej("E_CERT_OBJECT","§3.1/§16.1",format!("object.@{}",context.symbol),message)]);
    let f=docs.object.function(&context.symbol).expect("expanded function exists");
    let bits=if let Some((i,param))=f.groups[3].iter().enumerate().find(|(_,p)|p.name==text) {
        if param.ty!="f64" {return Err(bad(format!("gate angle argument {text} has type {}, expected f64",param.ty)));}
        let encoded=context.invocation.args.get(i).and_then(Value::as_str).unwrap_or("");
        if encoded.len()!=18 || !encoded.starts_with("0x") {return Err(bad("angle argument is not a bound binary64 bit pattern".into()));}
        u64::from_str_radix(&encoded[2..],16).map_err(|_|bad("invalid binary64 angle argument".into()))?
    }else {
        let value=text.parse::<f64>().map_err(|_|bad(format!("gate angle {text:?} is not a finite canonical FLOAT or bound f64 argument")))?;
        if !value.is_finite(){return Err(bad("gate angle is NaN or infinite".into()));}
        let mut spelling=if value==0.0 && value.is_sign_negative(){"-0.0".to_string()}else{crate::json::ecmascript_number(value)};
        if !spelling.contains(['.','e']) {spelling.push_str(".0");}
        if text!=spelling{return Err(bad(format!("noncanonical gate angle {text:?}; expected {spelling:?}")));}
        value.to_bits()
    };
    crate::transcendental::binary64_rational(bits).map_err(bad)?;
    Ok(bits)
}
impl Gate {
    /// §5.3 replaces a primitive by its adjoint before target matching and
    /// leaf accounting. A native T entry does not implement T-dagger.
    pub(crate) fn realised_opcode(&self) -> &str {
        if !self.adjoint {
            return &self.op;
        }
        match self.op.as_str() {
            "s" => "sdg",
            "sdg" => "s",
            "t" => "tdg",
            "tdg" => "t",
            _ => &self.op,
        }
    }
}
fn element(hash: &str, block: &str, ordinal: usize) -> Value {
    Value::Obj(BTreeMap::from([
        ("symbol".into(), s(hash)),
        ("point".into(), s(&format!("{block}:{ordinal}"))),
        ("iteration".into(), Value::Arr(vec![])),
        ("instance".into(), Value::Arr(vec![])),
        ("call_index".into(), n(0)),
    ]))
}
fn prefixes<'a>(
    mut tokens: &'a [String],
    controls: &mut Vec<usize>,
    wires: &HashMap<String, Vec<usize>>,
    adjoint: &mut bool,
) -> Result<&'a [String]> {
    loop {
        match tokens.first().map(String::as_str) {
            Some("ctl") => {
                let wire = tokens
                    .get(1)
                    .ok_or_else(|| ClaimOutcome::unchecked("ctl without wire"))?;
                controls.push(one_wire(wire, wires)?);
                tokens = &tokens[2..];
            }
            Some("adj") => {
                *adjoint = !*adjoint;
                tokens = &tokens[1..];
            }
            _ => return Ok(tokens),
        }
    }
}
fn bundle(name: &str, wires: &HashMap<String, Vec<usize>>) -> Result<Vec<usize>> {
    if let Some((name, index)) = name.strip_suffix(']').and_then(|s| s.split_once('[')) {
        let index = index.parse::<usize>().map_err(|_| {
            ClaimOutcome::unchecked("nonliteral wire subscript in native matrix builder")
        })?;
        return wires
            .get(name)
            .and_then(|b| b.get(index))
            .map(|i| vec![*i])
            .ok_or_else(|| ClaimOutcome::unchecked("wire subscript out of range"));
    }
    wires
        .get(name)
        .cloned()
        .ok_or_else(|| ClaimOutcome::unchecked(format!("unresolved wire {name:?}")))
}
fn one_wire(name: &str, wires: &HashMap<String, Vec<usize>>) -> Result<usize> {
    let b = bundle(name, wires)?;
    if b.len() != 1 {
        return pending(format!("expected a single wire, got {name:?}"));
    }
    Ok(b[0])
}
fn child_invocation(tokens: &[String], parent: &Context, docs: &LoadedDocs) -> Result<Invocation> {
    let f = docs
        .object
        .function(&parent.symbol)
        .ok_or_else(|| ClaimOutcome::unchecked("parent action has no body"))?;
    let arg_values: HashMap<&str, &Value> = f.groups[3]
        .iter()
        .zip(&parent.invocation.args)
        .map(|(p, v)| (p.name.as_str(), v))
        .collect();
    let args = list_items(tokens.get(5).map(String::as_str).unwrap_or("()"));
    let mut resolved = Vec::new();
    for name in args {
        resolved.push(
            arg_values
                .get(name.as_str())
                .map(|v| (*v).clone())
                .ok_or_else(|| {
                    ClaimOutcome::unchecked(format!(
                        "unresolved classical action-call argument {name:?}"
                    ))
                })?,
        );
    }
    let callee = tokens
        .get(1)
        .map(|s| s.trim_start_matches('@'))
        .unwrap_or("");
    let iface = docs
        .object
        .interface(callee)
        .map_err(ClaimOutcome::unchecked)?;
    let value = Value::Obj(BTreeMap::from([
        ("args".into(), Value::Arr(resolved)),
        (
            "budget".into(),
            Value::Arr(
                parent
                    .invocation
                    .budget
                    .iter()
                    .map(|b| s(&b.to_string()))
                    .collect(),
            ),
        ),
    ]));
    parse_invocation(&value, &iface.args, &docs.object, "resolved invocation")
        .map_err(ClaimOutcome::rejected)
}
fn collect(
    context: &Context,
    docs: &LoadedDocs,
    wires: HashMap<String, Vec<usize>>,
    inherited: Vec<usize>,
    contexts: &mut Vec<Context>,
    gates: &mut Vec<Gate>,
    max: u64,
    depth: usize,
) -> Result<()> {
    if depth > 64 || contexts.len() as u64 >= max || gates.len() as u64 >= max {
        return Err(host_limit_outcome(HostLimit {
            quantity: "native action invocation/gate expansion limit".into(),
        }));
    }
    contexts.push(context.clone());
    let f = docs
        .object
        .function(&context.symbol)
        .ok_or_else(|| ClaimOutcome::unchecked("action context has no body"))?;
    if !matches!(f.kind.as_str(), "action" | "perm_action") {
        return pending(format!(
            "§11.1 expansion of {} @{} is not implemented by the native matrix builder",
            f.kind, f.name
        ));
    }
    if f.blocks.len() != 1 {
        return pending(format!(
            "multi-block action @{} requires structured expansion",
            f.name
        ));
    }
    let hash = docs
        .object
        .symbol(&context.symbol)
        .and_then(|s| s.closure_hash.as_deref())
        .ok_or_else(|| ClaimOutcome::unchecked("context symbol has no closure identity"))?;
    let mut predicates = inherited;
    let mut body_gates = Vec::new();
    for (ordinal, line) in f.blocks[0].instructions.iter().enumerate() {
        if gates.len() + body_gates.len() > max as usize {
            return Err(host_limit_outcome(HostLimit {
                quantity: "native gate expansion limit".into(),
            }));
        }
        let tokens = tokenise(line);
        let mut controls = predicates.clone();
        let mut adjoint = false;
        let tokens = prefixes(&tokens, &mut controls, &wires, &mut adjoint)?;
        let op = tokens.first().map(String::as_str).unwrap_or("");
        let mut path = context.key.clone();
        path.push(element(hash, &f.blocks[0].name, ordinal));
        match op {
            "qpred" => {
                predicates.push(one_wire(
                    tokens.get(1).map(String::as_str).unwrap_or(""),
                    &wires,
                )?);
            }
            "qunpred" => {
                if predicates.len() <= context.controls as usize {
                    return invalid("object", "qunpred cannot pop an inherited predicate");
                }
                let bit = one_wire(tokens.get(1).map(String::as_str).unwrap_or(""), &wires)?;
                if predicates.pop() != Some(bit) {
                    return invalid("object", "unbalanced predicate stack");
                }
            }
            "action_call" => {
                let name = tokens
                    .get(1)
                    .map(|s| s.trim_start_matches('@'))
                    .unwrap_or("")
                    .to_string();
                let iface = docs
                    .object
                    .interface(&name)
                    .map_err(ClaimOutcome::unchecked)?;
                if !iface.regions.is_empty() || !iface.borrowed_widths.is_empty() {
                    return pending("nested regions/borrow lifetimes need expansion before native matrix construction");
                }
                let actuals: Vec<String> = [2, 3]
                    .iter()
                    .flat_map(|i| list_items(tokens.get(*i).map(String::as_str).unwrap_or("()")))
                    .collect();
                if actuals.len() != iface.bundles.len() {
                    return invalid("object", "action-call operand count differs from signature");
                }
                let mut child_wires = HashMap::new();
                for (b, a) in iface.bundles.iter().zip(actuals) {
                    let actual = bundle(&a, &wires)?;
                    if actual.len() != b.width as usize {
                        return invalid(
                            "object",
                            "action-call bundle width differs from signature",
                        );
                    }
                    child_wires.insert(b.name.clone(), actual);
                }
                let child = Context {
                    symbol: name,
                    controls: controls.len() as u64,
                    invocation: child_invocation(tokens, context, docs)?,
                    key: path,
                    adjoint: adjoint ^ context.adjoint,
                };
                let mut nested_gates = Vec::new();
                collect(
                    &child,
                    docs,
                    child_wires,
                    controls,
                    contexts,
                    &mut nested_gates,
                    max,
                    depth + 1,
                )?;
                // The enclosing adjoint is applied to this complete body's
                // stream below. Child context identity already includes it;
                // undo that part here to avoid applying it twice to leaves.
                if context.adjoint {
                    nested_gates.reverse();
                    for g in &mut nested_gates {
                        g.adjoint = !g.adjoint;
                    }
                }
                body_gates.extend(nested_gates);
            }
            "x" | "cx" | "ccx" | "mcx" | "swap" | "h" | "s" | "sdg" | "t" | "tdg" | "cz" | "rx" | "ry" | "rz" => {
                let arity = match op {
                    "cx" | "mcx" | "swap" | "cz" | "rx" | "ry" | "rz" => 3,
                    "ccx" => 4,
                    _ => 2,
                };
                if tokens.len() != arity {
                    return invalid(
                        "object",
                        format!("gate {op} has the wrong number of operands"),
                    );
                }
                let operands = &tokens[1..];
                let angle_bits=if matches!(op,"rx"|"ry"|"rz"){Some(resolve_angle(&operands[0],context,docs)?)}else{None};
                let inherited_count = controls.len();
                let targets = match op {
                    "cx" => {
                        controls.push(one_wire(
                            operands.first().map(String::as_str).unwrap_or(""),
                            &wires,
                        )?);
                        vec![one_wire(
                            operands.get(1).map(String::as_str).unwrap_or(""),
                            &wires,
                        )?]
                    }
                    "ccx" => {
                        for operand in operands.iter().take(2) {
                            controls.push(one_wire(operand, &wires)?);
                        }
                        vec![one_wire(
                            operands.get(2).map(String::as_str).unwrap_or(""),
                            &wires,
                        )?]
                    }
                    "mcx" => {
                        for operand in
                            list_items(operands.first().map(String::as_str).unwrap_or("()"))
                        {
                            controls.push(one_wire(&operand, &wires)?);
                        }
                        vec![one_wire(
                            operands.get(1).map(String::as_str).unwrap_or(""),
                            &wires,
                        )?]
                    }
                    "swap" | "cz" => operands
                        .iter()
                        .map(|t| one_wire(t, &wires))
                        .collect::<Result<Vec<_>>>()?,
                    "rx"|"ry"|"rz"=>vec![one_wire(&operands[1],&wires)?],
                    _ => vec![one_wire(
                        operands.first().map(String::as_str).unwrap_or(""),
                        &wires,
                    )?],
                };
                let mut all = controls.clone();
                all.extend(&targets);
                let unique: HashSet<usize> = all.iter().copied().collect();
                if all.len() != unique.len() {
                    return invalid("object", "gate controls and targets are not disjoint");
                }
                body_gates.push(Gate {
                    op: op.into(),
                    own_controls: controls.len() - inherited_count,
                    controls,
                    targets,
                    path,
                    angle_bits,
                    adjoint,
                });
            }
            _ => {
                return pending(format!(
                "native matrix expansion of @{hash} instruction {op:?} has not been implemented"
            ))
            }
        }
    }
    if predicates.len() != context.controls as usize {
        return invalid("object", "predicate stack is not restored at action exit");
    }
    if context.adjoint {
        body_gates.reverse();
        for gate in &mut body_gates {
            gate.adjoint = !gate.adjoint;
        }
    }
    gates.extend(body_gates);
    Ok(())
}
fn root_wires(context: &Context, docs: &LoadedDocs) -> Result<HashMap<String, Vec<usize>>> {
    let iface = docs
        .object
        .interface(&context.symbol)
        .map_err(ClaimOutcome::unchecked)?;
    if !iface.regions.is_empty() || !iface.borrowed_widths.is_empty() {
        return pending(
            "native matrix builder needs region/borrow lifetime expansion for this subject",
        );
    }
    let mut next = context.controls as usize;
    let mut wires = HashMap::new();
    for b in iface.bundles {
        let end = next.checked_add(b.width as usize).ok_or_else(|| {
            host_limit_outcome(HostLimit {
                quantity: "native wire allocation".into(),
            })
        })?;
        wires.insert(b.name, (next..end).collect());
        next = end;
    }
    Ok(wires)
}
pub(crate) fn trace(docs: &LoadedDocs, inputs: &Inputs) -> Result<(Vec<Context>, Vec<Gate>)> {
    let schedule = docs
        .schedule
        .as_ref()
        .ok_or_else(|| ClaimOutcome::unchecked("schedule document is missing"))?;
    if docs.expansion_unknown {
        return pending("bound expansion library is not implemented");
    }
    let root = docs
        .object
        .symbol_by_closure(&docs.root_closure)
        .ok_or_else(|| ClaimOutcome::unchecked("root closure is not linked"))?;
    let invocation = docs
        .root_invocation
        .clone()
        .ok_or_else(|| ClaimOutcome::unchecked("root invocation is unavailable"))?;
    let controls = schedule
        .get("bindings")
        .and_then(|b| b.get("inherited_controls"))
        .and_then(u64_of)
        .unwrap_or(0);
    let context = Context {
        symbol: root.name.clone(),
        controls,
        invocation,
        key: vec![],
        adjoint: false,
    };
    // Bound before materialising any wire vector.
    check_width(&context, docs, inputs)?;
    let mut contexts = Vec::new();
    let mut gates = Vec::new();
    collect(
        &context,
        docs,
        root_wires(&context, docs)?,
        (0..controls as usize).collect(),
        &mut contexts,
        &mut gates,
        inputs.limits.max_elements,
        0,
    )?;
    contexts
        .sort_by_key(|c| jcs(&Value::Arr(c.key.clone())).expect("concrete context key is JSON"));
    Ok((contexts, gates))
}
pub fn contexts(docs: &LoadedDocs, inputs: &Inputs) -> Result<Vec<Context>> {
    trace(docs, inputs).map(|(contexts, _)| contexts)
}
fn check_width(c: &Context, docs: &LoadedDocs, inputs: &Inputs) -> Result<usize> {
    let iface = docs
        .object
        .interface(&c.symbol)
        .map_err(ClaimOutcome::unchecked)?;
    let width = iface
        .bundles
        .iter()
        .try_fold(c.controls, |sum, b| sum.checked_add(b.width))
        .ok_or_else(|| {
            host_limit_outcome(HostLimit {
                quantity: "unitary wire count overflow".into(),
            })
        })?;
    // Dense cubic rational matrix multiplication is bounded separately from
    // legal ISA widths. No host cap is a domain rejection.
    if width > u64::from(inputs.limits.max_wires.min(6)) {
        return Err(host_limit_outcome(HostLimit {
            quantity: format!(
                "dense unitary requires {width} wires (host cap {})",
                inputs.limits.max_wires.min(6)
            ),
        }));
    }
    Ok(width as usize)
}
fn selector(entry: &Value, path: &[Value]) -> bool {
    let Some(p) = entry.get("path").and_then(Value::as_arr) else {
        return false;
    };
    if p.len() > path.len()
        || entry.get("step").and_then(Value::as_str) != Some("*")
        || entry.get("phase").and_then(Value::as_str) != Some("*")
    {
        return false;
    }
    if entry.get("generated").and_then(Value::as_str) != Some("*")
        && entry.get("generated").and_then(u64_of) != Some(0)
    {
        return false;
    }
    p.iter().zip(path).all(|(want, got)| {
        if want.get("symbol") != got.get("symbol") || want.get("point") != got.get("point") {
            return false;
        }
        for key in ["iteration", "instance"] {
            if let Some(v) = want.get(key) {
                if v.as_str() != Some("*") && got.get(key) != Some(v) {
                    return false;
                }
            }
        }
        want.get("call_index")
            .map(|v| v.as_str() == Some("*") || u64_of(v) == Some(0))
            .unwrap_or(true)
    })
}
pub(crate) fn decision_kind<'a>(
    docs: &'a LoadedDocs,
    gate: &Gate,
    kind: &str,
) -> Result<Option<&'a Value>> {
    let entries = docs
        .schedule
        .as_ref()
        .and_then(|s| s.get("entries"))
        .and_then(Value::as_arr)
        .unwrap_or(&[]);
    let mut best: Option<((usize, usize, u8, u8, u8), &Value)> = None;
    let mut tied = false;
    for entry in entries {
        if entry.get("kind").and_then(Value::as_str) != Some(kind) || !selector(entry, &gate.path) {
            continue;
        }
        let p = entry.get("path").and_then(Value::as_arr).unwrap();
        let specificity = (
            p.len(),
            p.iter()
                .filter(|e| e.get("iteration").and_then(Value::as_str) != Some("*"))
                .count(),
            0,
            0,
            u8::from(entry.get("generated").and_then(Value::as_str) != Some("*")),
        );
        if best.as_ref().map(|(s, _)| specificity > *s).unwrap_or(true) {
            best = Some((specificity, entry));
            tied = false;
        } else if best
            .as_ref()
            .map(|(s, _)| specificity == *s)
            .unwrap_or(false)
        {
            tied = true;
        }
    }
    if tied {
        return invalid(
            "schedule.entries",
            format!("two matching {kind} decisions have equal maximal specificity"),
        );
    }
    Ok(best.and_then(|(_, e)| e.get(kind)))
}
pub(crate) fn native<'a>(docs: &'a LoadedDocs, gate: &Gate) -> Result<&'a Value> {
    let target = docs
        .target
        .as_ref()
        .ok_or_else(|| ClaimOutcome::unchecked("target document is missing"))?;
    if target.get("topology").is_some() {
        return pending("native topology/placement validation has not been implemented");
    }
    let classes = target
        .get("classes")
        .and_then(Value::as_obj)
        .ok_or_else(|| ClaimOutcome::unchecked("target classes unavailable"))?;
    if classes.len() != 1 {
        return pending("native matrix builder needs resolved multi-class allocation");
    }
    let class = classes.keys().next().unwrap();
    if let Some(g) = decision_kind(docs, gate, "gate")? {
        if g.get("realisation").and_then(Value::as_str) != Some("native") {
            return pending("scheduled scheme template expansion has not been implemented");
        }
    }
    let gates = target.get("gates").and_then(Value::as_arr).unwrap_or(&[]);
    for entry in gates {
        if entry.get("op").and_then(Value::as_str) != Some(gate.realised_opcode()) {
            continue;
        }
        let min = entry
            .get("controls")
            .and_then(|v| v.get("min"))
            .and_then(u64_of);
        let max = entry.get("controls").and_then(|v| v.get("max"));
        let count = gate.controls.len() as u64;
        if min.map(|n| n <= count) != Some(true)
            || !max
                .map(|v| {
                    v.as_str() == Some("unbounded")
                        || u64_of(v).map(|n| count <= n).unwrap_or(false)
                })
                .unwrap_or(false)
        {
            continue;
        }
        let pattern = entry.get("classes").and_then(Value::as_arr).unwrap_or(&[]);
        if pattern.len() != 1 || !matches!(pattern[0].as_str(),Some(c) if c==class || c=="*") {
            return pending(
                "native class-pattern matching needs a resolved operand-class expansion",
            );
        }
        match entry.get("epsilon").and_then(|v|rat_of_json(v).ok()) {
            Some(e) if e.is_zero()=>return Ok(entry),
            _=>return pending("nonzero/expressed native epsilon needs evaluated shares and a specified realised operator before numerical replay"),
        }
    }
    pending(format!(
        "gate {} with {} total controls has no checked exact native match",
        gate.realised_opcode(),
        gate.controls.len()
    ))
}
fn gate_matrix(g: &Gate, wires: usize, p: u32) -> Result<Matrix> {
    let dim = 1usize << wires;
    let mut m = Matrix::identity(dim);
    let one = C::real(Rat::one());
    let on = |i: usize| g.controls.iter().all(|b| i & (1usize << b) != 0);
    let target = g.targets[0];
    match g.op.as_str() {
        "x" | "cx" | "ccx" | "mcx" | "swap" => {
            m = Matrix::zeros(dim);
            for i in 0..dim {
                let j = if !on(i) {
                    i
                } else if g.op == "swap" {
                    let a = 1usize << g.targets[0];
                    let b = 1usize << g.targets[1];
                    if (i & a != 0) != (i & b != 0) {
                        i ^ a ^ b
                    } else {
                        i
                    }
                } else {
                    i ^ (1usize << target)
                };
                m.set(j, i, one.clone());
            }
        }
        "h" => {
            let half = Interval::exact(Rat::new(1.into(), 2.into()))
                .sqrt(p)
                .unwrap();
            for i in 0..dim {
                if on(i) && i & (1usize << target) == 0 {
                    let j = i | (1usize << target);
                    let h = C {
                        re: half.clone(),
                        im: Interval::exact(Rat::zero()),
                    };
                    m.set(i, i, h.clone());
                    m.set(j, i, h.clone());
                    m.set(i, j, h.clone());
                    m.set(
                        j,
                        j,
                        C {
                            re: h.re.neg(),
                            im: h.im,
                        },
                    );
                }
            }
        }
        "s" | "sdg" | "t" | "tdg" => {
            let negative = matches!(g.op.as_str(), "sdg" | "tdg") ^ g.adjoint;
            let (re, im) = if matches!(g.op.as_str(), "s" | "sdg") {
                (Interval::exact(Rat::zero()), Interval::exact(Rat::one()))
            } else {
                let half = Interval::exact(Rat::new(1.into(), 2.into()))
                    .sqrt(p)
                    .unwrap();
                (half.clone(), half)
            };
            let phase = C {
                re,
                im: if negative { im.neg() } else { im },
            };
            for i in 0..dim {
                if on(i) && i & (1usize << target) != 0 {
                    m.set(i, i, phase.clone());
                }
            }
        }
        "cz" => {
            for i in 0..dim {
                if on(i) && g.targets.iter().all(|b| i & (1usize << b) != 0) {
                    m.set(i, i, C::real(-Rat::one()));
                }
            }
        }
        "rx"|"ry"|"rz"=>{
            let bits=g.angle_bits.expect("rotation angle resolved") ^ if g.adjoint {1u64<<63}else{0};
            let angle=crate::transcendental::binary64_rational(bits).expect("finite angle checked")/Rat::from_integer(2.into());
            let (sin,cos)=crate::transcendental::sin_cos(&angle,p).map_err(host_limit_outcome)?;
            let real=|re:Interval|C{re,im:Interval::exact(Rat::zero())};
            for i in 0..dim {if on(i) && i&(1usize<<target)==0 {
                let j=i|(1usize<<target);
                if g.op=="rz" {
                    m.set(i,i,C{re:cos.clone(),im:sin.neg()});m.set(j,j,C{re:cos.clone(),im:sin.clone()});
                }else {
                    m.set(i,i,real(cos.clone()));m.set(j,j,real(cos.clone()));
                    if g.op=="rx" {let z=C{re:Interval::exact(Rat::zero()),im:sin.neg()};m.set(i,j,z.clone());m.set(j,i,z);}
                    else {m.set(i,j,real(sin.neg()));m.set(j,i,real(sin.clone()));}
                }
            }}
        }
        _ => unreachable!("gate op validated during collection"),
    }
    Ok(m)
}
pub fn subject_matrices(
    subject: &Subject,
    docs: &LoadedDocs,
    inputs: &Inputs,
    p: u32,
) -> Result<Vec<(Context, Matrix, Matrix)>> {
    let contexts = contexts(docs, inputs)?;
    let variant = docs
        .schedule
        .as_ref()
        .and_then(|s| s.get("bindings"))
        .and_then(|b| b.get("variant"))
        .and_then(Value::as_str);
    if variant != Some(subject.variant.as_str()) {
        return invalid(
            "subject.variant",
            "subject variant has no invocation in this root realisation",
        );
    }
    let mut out = Vec::new();
    for context in contexts {
        let hash = docs
            .object
            .symbol(&context.symbol)
            .and_then(|s| s.closure_hash.as_deref());
        if hash != Some(subject.closure_hash.as_str())
            || context.controls != subject.inherited_controls
        {
            continue;
        }
        let wires = check_width(&context, docs, inputs)?;
        let mut gates = Vec::new();
        collect(
            &context,
            docs,
            root_wires(&context, docs)?,
            (0..context.controls as usize).collect(),
            &mut vec![],
            &mut gates,
            inputs.limits.max_elements,
            0,
        )?;
        let mut reference = Matrix::identity(1 << wires);
        let mut realised = reference.clone();
        if docs.object.symbol(&context.symbol).map(|s| s.kind.as_str()) == Some("perm_action")
            && gates
                .iter()
                .any(|g| !matches!(g.op.as_str(), "x" | "cx" | "ccx" | "mcx" | "swap"))
        {
            return invalid(
                "object",
                "permutation action transitively uses a non-permutation gate (§5.3)",
            );
        }
        for gate in &gates {
            native(docs, gate)?;
            // Build both matrices from their operation streams. Native exact
            // entries have the §4.4 matrix; schemes must supply expanded leaves.
            let v = gate_matrix(gate, wires, p)?;
            reference = v.mul(&reference, p);
            let u = gate_matrix(gate, wires, p)?;
            realised = u.mul(&realised, p);
        }
        let iface = docs
            .object
            .interface(&context.symbol)
            .map_err(ClaimOutcome::unchecked)?;
        let mut offset = context.controls as usize;
        let mut protected = (1usize << offset) - 1;
        for bundle in &iface.bundles {
            if bundle.group == "in" {
                protected |= ((1usize << bundle.width) - 1) << offset;
            }
            offset += bundle.width as usize;
        }
        for matrix in [&reference, &realised] {
            for row in 0..matrix.dim {
                for col in 0..matrix.dim {
                    if (row ^ col) & protected == 0 {
                        continue;
                    }
                    let z = matrix.get(row, col);
                    if !z.re.contains_zero() || !z.im.contains_zero() {
                        return Err(ClaimOutcome::rejected(vec![rej(
                            "E_CERT_REPLAY",
                            "§5.3",
                            "subject",
                            "unitary changes an input bundle or inherited control",
                        )]));
                    }
                    if !z.re.lo.is_zero()
                        || !z.re.hi.is_zero()
                        || !z.im.lo.is_zero()
                        || !z.im.hi.is_zero()
                    {
                        return pending("interval construction cannot prove preservation of an input bundle or inherited control");
                    }
                }
            }
        }
        out.push((context, realised, reference));
    }
    if out.is_empty() {
        return invalid(
            "subject",
            "subject has no invocation context in the bound realisation",
        );
    }
    Ok(out)
}

/// For a permutation replay, certify that every selected native leaf is
/// exactly the reference gate, with the actual inherited control count.
/// This checks schedule decisions instead of inferring exactness merely
/// from the presence of an unrelated epsilon-zero entry in the target.
pub fn exact_native_subject(subject: &Subject, docs: &LoadedDocs, inputs: &Inputs) -> Result<()> {
    let contexts = contexts(docs, inputs)?;
    let mut found = false;
    for context in contexts {
        let hash = docs
            .object
            .symbol(&context.symbol)
            .and_then(|s| s.closure_hash.as_deref());
        if hash != Some(subject.closure_hash.as_str())
            || context.controls != subject.inherited_controls
        {
            continue;
        }
        found = true;
        let mut gates = vec![];
        collect(
            &context,
            docs,
            root_wires(&context, docs)?,
            (0..context.controls as usize).collect(),
            &mut vec![],
            &mut gates,
            inputs.limits.max_elements,
            0,
        )?;
        for gate in &gates {
            native(docs, gate)?;
        }
    }
    if !found {
        return invalid("subject", "subject has no invocation in this schedule");
    }
    Ok(())
}

#[cfg(test)]
mod rotation_columns {
    use super::*;
    fn bracket(token:&str)->(Interval,Interval) {
        let r=|x:&str|crate::rat::parse_decimal(x).unwrap();
        let zero=Interval::exact(Rat::zero());
        let c=Interval{lo:r("0.8775825618903727161162815826038296519916"),hi:r("0.8775825618903727161162815826038296519917")};
        let s=Interval{lo:r("0.4794255386042030002732879352155713880818"),hi:r("0.4794255386042030002732879352155713880819")};
        match token {"0"=>(zero.clone(),zero),"1"=>(Interval::exact(Rat::one()),zero),"c"=>(c,zero),"s"=>(s,zero),"-s"=>(s.neg(),zero),"-is"=>(zero,s.neg()),"c-is"=>(c,s.neg()),"c+is"=>(c,s),_=>unreachable!()}
    }
    #[test]
    fn all_controlled_rotation_columns_match_independent_decimal_brackets_and_adjoint() {
        // These full 4x4 columns pin §4.4's phases/signs and §12.1's control-
        // first order. The constants bracket sin(1/2), cos(1/2) independently.
        for (op,block) in [("rx",[["c","-is"],["-is","c"]]),("ry",[["c","-s"],["s","c"]]),("rz",[["c-is","0"],["0","c+is"]])] {
            let mut gate=Gate{op:op.into(),own_controls:0,controls:vec![0],targets:vec![1],path:vec![],angle_bits:Some(1.0f64.to_bits()),adjoint:false};
            let forward=gate_matrix(&gate,2,128).unwrap();
            for row in 0..4 {for col in 0..4 {
                let token=if col%2==0 {if row==col {"1"}else{"0"}}else if row%2==0 {"0"}else{block[row/2][col/2]};
                let (re,im)=bracket(token);let z=forward.get(row,col);
                assert!(z.re.lo<=re.lo && z.re.hi>=re.hi,"{op} [{row},{col}] real {z:?}");
                assert!(z.im.lo<=im.lo && z.im.hi>=im.hi,"{op} [{row},{col}] imaginary {z:?}");
            }}
            gate.adjoint=true;assert_eq!(gate_matrix(&gate,2,128).unwrap(),forward.adjoint(),"{op} adjoint");
        }
    }
}
