//! Forward F_B call/return semantics of §3.7. With one empty-key lproc,
//! canonical rows are the oracle rows followed by that procedure's rows.
//! Unread historical cells may be omitted extensionally; sp still grows.
use super::*;
const ACTIVE_BYTES: u128 = 256 * 1024 * 1024;
fn bad(message: &str) -> EvalError {
    EvalError::Reject(Reject::new("E_CERT_OBJECT", "§3.6/§3.7", "object", message))
}
fn parts(tokens: &[String]) -> (&str, &[String], Option<&str>) {
    if tokens.len() >= 3 && tokens[1] == "=" {
        (tokens[2].as_str(), &tokens[3..], Some(tokens[0].as_str()))
    } else {
        (
            tokens.first().map(String::as_str).unwrap_or(""),
            &tokens[1.min(tokens.len())..],
            None,
        )
    }
}
pub(super) fn has_call(rows: &[Vec<String>]) -> bool {
    rows.iter().any(|r| parts(r).0 == "lift.call")
}
fn rows(
    function: &Function,
    offset: usize,
) -> Result<(Vec<Vec<String>>, HashMap<String, usize>), EvalError> {
    let mut rows = Vec::new();
    let mut labels = HashMap::new();
    for block in &function.blocks {
        if !block.params.is_empty() {
            return Err(bad("lifted blocks do not carry classical block parameters"));
        }
        labels.insert(block.name.clone(), offset + rows.len());
        for line in &block.instructions {
            let tokens = tokenise(line);
            if tokens.last().map(String::as_str) == Some("{") {
                return Err(bad("structured action forms are not lifted rows"));
            }
            rows.push(tokens);
        }
        if !matches!(
            rows.last().map(|r| parts(r).0),
            Some("qjump" | "qbranch" | "qjumpr" | "lift.ret" | "lift.retval" | "fault")
        ) {
            return Err(bad("lifted blocks must end in a terminator"));
        }
    }
    Ok((rows, labels))
}
#[derive(Default)]
struct Scope {
    vals: HashMap<String, BigUint>,
    widths: HashMap<String, u64>,
    locations: HashSet<String>,
    tokens: HashMap<String, [BigUint; 4]>,
}
impl Scope {
    fn swap(&mut self, env: &mut Lifted<'_, '_>) {
        std::mem::swap(&mut self.vals, &mut env.vals);
        std::mem::swap(&mut self.widths, &mut env.widths);
        std::mem::swap(&mut self.locations, &mut env.locations);
        std::mem::swap(&mut self.tokens, &mut env.tokens);
    }
}
fn width(ty: &str) -> Result<u64, EvalError> {
    let digits = ty
        .strip_prefix("reg[")
        .and_then(|s| s.strip_suffix(']'))
        .filter(|s| crate::rat::is_canonical_uint_str(s) && *s != "0")
        .ok_or_else(|| bad("procedure locations must have positive reg[N] types"))?;
    let n = digits.parse::<u64>().map_err(|_| {
        EvalError::Unchecked("host limit: procedure location width exceeds u64".into())
    })?;
    reference_width(n)?;
    Ok(n)
}
fn declare(
    scope: &mut Scope,
    names: &mut Vec<String>,
    name: &str,
    ty: &str,
) -> Result<(), EvalError> {
    let w = width(ty)?;
    if !scope.locations.insert(name.into()) {
        return Err(bad("procedure location names are not distinct"));
    }
    scope.widths.insert(name.into(), w);
    scope.vals.insert(name.into(), BigUint::zero());
    names.push(name.into());
    Ok(())
}
fn procedure_scope(
    function: &Function,
    symbol: &Symbol,
) -> Result<(Scope, Vec<String>), EvalError> {
    if function.kind != "lproc" || symbol.kind != "lproc" {
        return Err(bad("lift.call must name a linked lproc"));
    }
    let sig = &symbol.signature;
    for (key, group) in [("tokens", 2), ("args", 3), ("mem", 4)] {
        let values = sig
            .get(key)
            .and_then(Value::as_arr)
            .ok_or_else(|| bad("lproc tokens/args/mem arrays are mandatory"))?;
        if values.len() != function.groups[group].len() {
            return Err(bad("lproc binding groups disagree with the signature"));
        }
        if !values.is_empty() {
            return unchecked(
                "lproc fixed-binding instances need canonical key instantiation before replay",
            );
        }
    }
    let params = sig
        .get("params")
        .and_then(Value::as_arr)
        .ok_or_else(|| bad("lproc params must be a width array"))?;
    if params.len() != function.groups[0].len() || function.groups[1].len() != 1 {
        return Err(bad("lproc location parameter/result counts disagree"));
    }
    let mut scope = Scope::default();
    let mut names = Vec::new();
    for (param, value) in function.groups[0].iter().zip(params) {
        if crate::cert::u64_of(value) != Some(width(&param.ty)?) {
            return Err(bad("lproc parameter widths disagree"));
        }
        declare(&mut scope, &mut names, &param.name, &param.ty)?;
    }
    let result = &function.groups[1][0];
    if sig.get("result").and_then(crate::cert::u64_of) != Some(width(&result.ty)?) {
        return Err(bad("lproc result width disagrees"));
    }
    declare(&mut scope, &mut names, &result.name, &result.ty)?;
    for line in function.bytes.lines() {
        if let Some(rest) = line.strip_prefix("regs ") {
            let inner = rest
                .trim()
                .trim_start_matches('{')
                .split(" bounds")
                .next()
                .unwrap()
                .trim_end()
                .trim_end_matches('}')
                .trim();
            if !inner.is_empty() {
                for decl in inner.split(", ") {
                    let (name, ty) = decl
                        .split_once(':')
                        .ok_or_else(|| bad("malformed procedure location declaration"))?;
                    declare(&mut scope, &mut names, name.trim(), ty.trim())?;
                }
            }
        }
    }
    if scope.widths.values().map(|n| u128::from(*n)).sum::<u128>() >= 1_000_000 {
        return unchecked("host limit: procedure location storage exceeds 999999 bits");
    }
    Ok((scope, names))
}
struct Frame {
    saved: Vec<BigUint>,
    destination: String,
    return_pc: usize,
    caller: usize,
}
fn halted(code: u64) -> OracleOutcome {
    OracleOutcome {
        payload: BigUint::zero(),
        code: code.into(),
    }
}

pub(super) fn run(
    env: &mut Lifted<'_, '_>,
    root_rows: &[Vec<String>],
    mem: &mut HashMap<String, Vec<BigUint>>,
    args: &[Classical],
    budget: &[BigUint; 4],
) -> Result<OracleOutcome, EvalError> {
    if env.object.layout.is_none() {
        return Err(bad("lift.call is forbidden in a memoryless object"));
    }
    let root = env
        .object
        .function(&env.oracle_name)
        .ok_or_else(|| bad("root oracle is not linked"))?;
    let mut name = None;
    let mut pending = vec![root];
    let mut seen = HashSet::new();
    while let Some(function) = pending.pop() {
        if !seen.insert(function.name.clone()) {
            continue;
        }
        for row in function
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .map(|r| tokenise(r))
        {
            let (op, operands, _) = parts(&row);
            if op != "lift.call" {
                continue;
            }
            let target = operands
                .first()
                .and_then(|s| s.strip_prefix('@'))
                .ok_or_else(|| bad("lift.call requires @callee"))?;
            if name.as_deref().map(|n| n != target) == Some(true) {
                return unchecked(
                    "multiple-lproc canonical SCC/instance placement has not been implemented",
                );
            }
            name = Some(target.to_string());
            let function = env
                .object
                .function(target)
                .ok_or_else(|| EvalError::Unchecked("called lproc is not linked".into()))?;
            if function.kind != "lproc" {
                return Err(bad("lift.call targets a non-lproc"));
            }
            pending.push(function);
        }
    }
    let name = name.ok_or_else(|| bad("procedure replay was requested without a call"))?;
    let function = env.object.function(&name).unwrap();
    let symbol = env
        .object
        .symbol(&name)
        .ok_or_else(|| bad("lproc symbol is missing"))?;
    let (proc_scope, names) = procedure_scope(function, symbol)?;
    let (mut all, root_labels) = rows(root, 0)?;
    if all != root_rows {
        return Err(bad(
            "oracle row flattening disagrees with its canonical index space",
        ));
    }
    let first = all.len();
    let (proc_rows, proc_labels) = rows(function, first)?;
    all.extend(proc_rows);
    if all.len() > 100_000 {
        return unchecked("host limit: procedure index space exceeds 100000 rows");
    }
    if BigUint::from(all.len()).bits() > env.object.header.ptr_width {
        return Err(bad("index space does not fit the pointer width"));
    }
    let params = &function.groups[0];
    let result_width = width(&function.groups[1][0].ty)?;
    let cells = names
        .len()
        .checked_add(3)
        .ok_or_else(|| EvalError::Unchecked("host limit: procedure frame size overflow".into()))?;
    let frame_bytes: u128 = 128
        + names
            .iter()
            .map(|n| u128::from((proc_scope.widths[n] + 7) / 8) + 64)
            .sum::<u128>();
    let mut banks = [Scope::default(), proc_scope];
    let mut frames = Vec::<Frame>::new();
    let mut sp = 0u64;
    let mut current = 0usize;
    let mut pc = 0usize;
    let mut steps = BigUint::zero();
    loop {
        if steps >= budget[0] {
            return Ok(halted(FAULT_BUDGET_STEPS));
        }
        let (start, end, labels) = if current == 0 {
            (0, first, &root_labels)
        } else {
            (first, all.len(), &proc_labels)
        };
        if pc < start || pc >= end {
            return Err(bad(
                "procedure control flow leaves its function's row range",
            ));
        }
        reference_step(env.work)?;
        steps += BigUint::one();
        let row = &all[pc];
        let (op, operands, destination) = parts(row);
        if op == "lift.call" {
            let destination = destination
                .filter(|n| env.locations.contains(*n))
                .ok_or_else(|| bad("lift.call destination must be a location"))?;
            if !matches!(operands.len(), 2 | 5) || operands[0] != format!("@{name}") {
                return Err(bad("lift.call has the wrong callee or operand lists"));
            }
            if operands.len() == 5 && operands[2..].iter().any(|s| s != "()") {
                return Err(bad("empty-key procedure call has nonempty fixed bindings"));
            }
            let sources = list_items(&operands[1]);
            if sources.len() != params.len() || env.width_of(destination)? != result_width {
                return Err(bad("lift.call signature mismatch"));
            }
            for (source, param) in sources.iter().zip(params) {
                if env.width_of(source)? != width(&param.ty)? {
                    return Err(bad("lift.call source width mismatch"));
                }
            }
            let next_sp = sp.checked_add(cells as u64).ok_or_else(|| {
                EvalError::Unchecked("host limit: procedure stack position exceeds u64".into())
            })?;
            if BigUint::from(next_sp) > budget[2] || BigUint::from(frames.len() + 1) > budget[3] {
                return Ok(halted(FAULT_BUDGET_DEPTH));
            }
            if frame_bytes * (frames.len() as u128 + 1) > ACTIVE_BYTES {
                return unchecked("host limit: active procedure frames exceed 256 MiB");
            }
            let captured = sources
                .iter()
                .map(|s| env.read(s))
                .collect::<Result<Vec<_>, _>>()?;
            let caller = current;
            if current != 1 {
                banks[current].swap(env);
                banks[1].swap(env);
                current = 1;
            }
            let mut saved = Vec::with_capacity(names.len());
            for name in &names {
                saved.push(std::mem::replace(
                    env.vals.get_mut(name).unwrap(),
                    BigUint::zero(),
                ));
            }
            for (param, value) in params.iter().zip(captured) {
                env.vals.insert(param.name.clone(), value);
            }
            frames.push(Frame {
                saved,
                destination: destination.into(),
                return_pc: pc + 1,
                caller,
            });
            sp = next_sp;
            pc = first;
            continue;
        }
        if op == "lift.ret" {
            if current != 1 || destination.is_some() || operands.len() != 1 {
                return Err(bad("lift.ret is only valid in a called lproc"));
            }
            if env.width_of(&operands[0])? != result_width {
                return Err(bad("lift.ret value has wrong width"));
            }
            let value = env.read(&operands[0])?;
            let frame = frames
                .pop()
                .ok_or_else(|| bad("lift.ret has no activation record"))?;
            for (name, old) in names.iter().zip(frame.saved) {
                env.vals.insert(name.clone(), old);
            }
            if current != frame.caller {
                banks[current].swap(env);
                banks[frame.caller].swap(env);
                current = frame.caller;
            }
            env.vals.insert(frame.destination, value);
            pc = frame.return_pc;
            // sp is intentionally not reduced: prior records remain history.
            continue;
        }
        if op == "lift.retval" && current != 0 {
            return Err(bad("lift.retval is forbidden inside an lproc"));
        }
        if op == "qjumpr" {
            if destination.is_some() || operands.len() != 1 {
                return Err(bad("qjumpr arity mismatch"));
            }
            let address = env.read(&operands[0])?;
            let index = address.to_string().parse::<usize>().ok();
            if index.map(|i| i < start || i >= end).unwrap_or(true) {
                return Ok(halted(FAULT_UB_TRAP));
            }
            pc = index.unwrap();
            continue;
        }
        match env.exec_row(row, &all, labels, mem, args)? {
            RowFlow::Next => pc += 1,
            RowFlow::Jump(target) => pc = target,
            RowFlow::Halt(outcome) => return Ok(outcome),
        }
    }
}
