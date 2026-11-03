//! Forward F_B call/return semantics of §3.7, with canonical §3.6 SCC
//! placement and fixed-binding instances for the transitive lproc closure.
//! Unread historical cells may be omitted extensionally; sp still grows.
use super::*;
mod bindings;
use bindings::{operand_list, Bindings};
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
    regions: HashMap<String, String>,
    in_lproc: bool,
}
impl Scope {
    fn swap(&mut self, env: &mut Lifted<'_, '_>) {
        std::mem::swap(&mut self.vals, &mut env.vals);
        std::mem::swap(&mut self.widths, &mut env.widths);
        std::mem::swap(&mut self.locations, &mut env.locations);
        std::mem::swap(&mut self.tokens, &mut env.tokens);
        std::mem::swap(&mut self.regions, &mut env.region_names);
        std::mem::swap(&mut self.in_lproc, &mut env.in_lproc);
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
    root: &Function,
    binding: &Bindings,
    args: &[Classical],
    ptr: u64,
) -> Result<(Scope, Vec<String>), EvalError> {
    if function.kind != "lproc" || symbol.kind != "lproc" {
        return Err(bad("lift.call must name a linked lproc"));
    }
    let sig = &symbol.signature;
    let mut errors = Vec::new();
    let fields = [
        "abi", "args", "kind", "mem", "params", "result", "tier", "tokens",
    ];
    if !crate::cert::check_keys(sig, &fields, &fields, "lproc.signature", &mut errors)
        || sig.get("abi").and_then(Value::as_str) != Some("0.9")
        || sig.get("kind").and_then(Value::as_str) != Some("lproc")
        || sig.get("tier").and_then(Value::as_str) != Some("q")
    {
        return Err(bad(
            "lproc signature is not a closed ISA 0.9 Q-tier lproc signature",
        ));
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
        if param.suffix.is_some() || crate::cert::u64_of(value) != Some(width(&param.ty)?) {
            return Err(bad("lproc parameter widths disagree"));
        }
        declare(&mut scope, &mut names, &param.name, &param.ty)?;
    }
    let result = &function.groups[1][0];
    if result.suffix.is_some()
        || sig.get("result").and_then(crate::cert::u64_of) != Some(width(&result.ty)?)
    {
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
    bindings::fixed_scope(&mut scope, function, symbol, root, binding, args, ptr)?;
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
    bytes: u128,
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
    let root_binding = Bindings::root(root);
    let mut pending = vec![(root, root_binding.clone())];
    let mut discovered = HashSet::from([(root.name.clone(), vec![])]);
    let mut plan_work = 0u128;
    let mut retained = 0u128;
    let mut row_count = 0usize;
    let mut instances = Vec::new();
    while let Some((function, binding)) = pending.pop() {
        row_count = row_count.saturating_add(
            function
                .blocks
                .iter()
                .map(|b| b.instructions.len())
                .sum::<usize>(),
        );
        if row_count > 100_000 {
            return unchecked("host limit: procedure index space exceeds 100000 rows");
        }
        retained += function
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .map(|r| r.len() as u128 * 2 + 128)
            .sum::<u128>()
            + binding
                .args
                .keys()
                .chain(binding.regions.keys())
                .map(|s| s.len() as u128 + 160)
                .sum::<u128>();
        if retained > ACTIVE_BYTES {
            return unchecked("host limit: retained procedure plan exceeds 256 MiB");
        }
        if function.name != root.name {
            instances.push((function, binding.clone()));
        }
        for row in function
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .map(|r| tokenise(r))
        {
            plan_work += 1;
            if plan_work > 1_048_576 {
                return unchecked("host limit: static procedure binding work exceeds 1048576");
            }
            let (op, operands, _) = parts(&row);
            if op != "lift.call" {
                continue;
            }
            let target = operands
                .first()
                .and_then(|s| s.strip_prefix('@'))
                .ok_or_else(|| bad("lift.call requires @callee"))?;
            let callee = env
                .object
                .function(target)
                .ok_or_else(|| EvalError::Unchecked("called lproc is not linked".into()))?;
            if callee.kind != "lproc" {
                return Err(bad("lift.call targets a non-lproc"));
            }
            if env.object.closure_order(target).is_none() {
                return unchecked("lproc closure order depends on an unresolved import");
            }
            plan_work += callee.groups[2..5].iter().map(Vec::len).sum::<usize>() as u128;
            if plan_work > 1_048_576 {
                return unchecked("host limit: static procedure binding work exceeds 1048576");
            }
            let resolved = binding.resolve(root, callee, operands)?;
            if discovered.insert((callee.name.clone(), resolved.key.clone())) {
                if discovered.len() > 4097 {
                    return unchecked("host limit: procedure plan exceeds 4096 binding instances");
                }
                pending.push((callee, resolved));
            }
        }
    }
    instances.sort_by(|(f, a), (g, b)| {
        env.object
            .closure_order(&f.name)
            .unwrap()
            .cmp(env.object.closure_order(&g.name).unwrap())
            .then_with(|| a.key.cmp(&b.key))
    });
    let (mut all, root_labels) = rows(root, 0)?;
    if all != root_rows {
        return Err(bad(
            "oracle row flattening disagrees with its canonical index space",
        ));
    }
    struct Definition<'a> {
        function: &'a Function,
        binding: Bindings,
        names: Vec<String>,
        first: usize,
        end: usize,
        labels: HashMap<String, usize>,
        result_width: u64,
        cells: usize,
        frame_bytes: u128,
    }
    let mut definitions = vec![Definition {
        function: root,
        binding: root_binding,
        names: vec![],
        first: 0,
        end: all.len(),
        labels: root_labels,
        result_width: 0,
        cells: 0,
        frame_bytes: 0,
    }];
    let mut banks = vec![Scope::default()];
    let mut targets = HashMap::new();
    let mut plan_bytes = 0u128;
    for (function, binding) in instances {
        let symbol = env
            .object
            .symbol(&function.name)
            .ok_or_else(|| bad("lproc symbol is missing"))?;
        let (scope, names) = procedure_scope(
            function,
            symbol,
            root,
            &binding,
            args,
            env.object.header.ptr_width,
        )?;
        plan_bytes += 512
            + scope
                .widths
                .iter()
                .map(|(name, w)| u128::from((w + 7) / 8) + 192 + name.len() as u128)
                .sum::<u128>()
            + scope
                .tokens
                .iter()
                .map(|(name, t)| {
                    name.len() as u128
                        + 256
                        + t.iter()
                            .map(|n| u128::from(n.bits().div_ceil(8)))
                            .sum::<u128>()
                })
                .sum::<u128>()
            + scope.regions.len() as u128 * 192;
        if plan_bytes > ACTIVE_BYTES {
            return unchecked("host limit: procedure instance storage exceeds 256 MiB");
        }
        let first = all.len();
        let (proc_rows, labels) = rows(function, first)?;
        if first.saturating_add(proc_rows.len()) > 100_000 {
            return unchecked("host limit: procedure index space exceeds 100000 rows");
        }
        all.extend(proc_rows);
        let result_width = width(&function.groups[1][0].ty)?;
        let cells = names.len().checked_add(3).ok_or_else(|| {
            EvalError::Unchecked("host limit: procedure frame size overflow".into())
        })?;
        let frame_bytes = 128
            + names
                .iter()
                .map(|n| u128::from((scope.widths[n] + 7) / 8) + 64)
                .sum::<u128>();
        targets.insert(
            (function.name.clone(), binding.key.clone()),
            definitions.len(),
        );
        definitions.push(Definition {
            function,
            binding,
            names,
            first,
            end: all.len(),
            labels,
            result_width,
            cells,
            frame_bytes,
        });
        banks.push(scope);
    }
    if all.len() > 100_000 {
        return unchecked("host limit: procedure index space exceeds 100000 rows");
    }
    if BigUint::from(all.len()).bits() > env.object.header.ptr_width {
        return Err(bad("index space does not fit the pointer width"));
    }
    let mut frames = Vec::<Frame>::new();
    let mut active_bytes = 0u128;
    let mut sp = 0u64;
    let mut current = 0usize;
    let mut pc = 0usize;
    let mut steps = BigUint::zero();
    loop {
        if steps >= budget[0] {
            return Ok(halted(FAULT_BUDGET_STEPS));
        }
        let definition = &definitions[current];
        let (start, end, labels) = (definition.first, definition.end, &definition.labels);
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
            if !matches!(operands.len(), 2 | 5) {
                return Err(bad("lift.call has the wrong number of operand lists"));
            }
            let name = operands[0]
                .strip_prefix('@')
                .ok_or_else(|| bad("lift.call requires @callee"))?;
            let function = env
                .object
                .function(name)
                .ok_or_else(|| bad("call target is not linked"))?;
            let binding = definition.binding.resolve(root, function, operands)?;
            let callee = *targets
                .get(&(name.to_string(), binding.key))
                .ok_or_else(|| bad("call target is absent from the fixed-binding closure"))?;
            let definition = &definitions[callee];
            let params = &definition.function.groups[0];
            let result_width = definition.result_width;
            let names = &definition.names;
            let cells = definition.cells;
            let sources = operand_list(&operands[1])?;
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
            if active_bytes + definition.frame_bytes > ACTIVE_BYTES {
                return unchecked("host limit: active procedure frames exceed 256 MiB");
            }
            let captured = sources
                .iter()
                .map(|s| env.read(s))
                .collect::<Result<Vec<_>, _>>()?;
            let caller = current;
            if current != callee {
                banks[current].swap(env);
                banks[callee].swap(env);
                current = callee;
            }
            let mut saved = Vec::with_capacity(names.len());
            for name in names {
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
                bytes: definition.frame_bytes,
            });
            active_bytes += definition.frame_bytes;
            sp = next_sp;
            pc = definition.first;
            continue;
        }
        if op == "lift.ret" {
            if current == 0 || destination.is_some() || operands.len() != 1 {
                return Err(bad("lift.ret is only valid in a called lproc"));
            }
            if env.width_of(&operands[0])? != definition.result_width {
                return Err(bad("lift.ret value has wrong width"));
            }
            let value = env.read(&operands[0])?;
            let frame = frames
                .pop()
                .ok_or_else(|| bad("lift.ret has no activation record"))?;
            active_bytes -= frame.bytes;
            for (name, old) in definition.names.iter().zip(frame.saved) {
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
