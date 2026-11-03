//! Independent replay evaluators, written from ISA sections 4.2, 4.4-4.7,
//! 5.2, 5.3 and 14.1: a lifted-oracle reference evaluator (F_B subset) and
//! a basis-state action evaluator for permutation realisations. Bodies
//! outside the supported subset yield `unchecked` with the exact row or
//! instruction named; nothing is silently approximated.

use crate::cert::{ClaimOutcome, Inputs, Invocation, LoadedDocs};
use crate::domain::Tuple;
use crate::fault::Reject;
use crate::object::{Function, Object, Symbol};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::{HashMap, HashSet, VecDeque};
mod procedures;

#[derive(Debug)]
pub enum EvalError {
    /// The producer's claim about this object is false (unclean workspace,
    /// borrowed changed, non-permutation gate in a perm_action, faulting
    /// oracle inside qinplace, ...).
    Reject(Reject),
    /// This checker cannot replay the body (unsupported row/instruction).
    Unchecked(String),
}

pub fn unchecked<T>(msg: impl Into<String>) -> Result<T, EvalError> {
    Err(EvalError::Unchecked(msg.into()))
}

fn classical_width(ty: &str, ptr: u64) -> Result<u64, EvalError> {
    match ty {
        "ptr" => Ok(ptr),
        "f64" => Ok(64),
        _ => ty
            .strip_prefix('i')
            .filter(|s| crate::rat::is_canonical_uint_str(s))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|w| (1..=128).contains(w))
            .ok_or_else(|| {
                EvalError::Reject(Reject::new(
                    "E_CERT_OBJECT",
                    "§3.7",
                    "object",
                    "classical parameter must be i1..i128, ptr or f64",
                ))
            }),
    }
}

fn mask(width: u64) -> BigUint {
    (BigUint::one() << width) - BigUint::one()
}

fn reference_width(width: u64) -> Result<(), EvalError> {
    if width >= 1_000_000 {
        return unchecked("host limit: reference bit-vector width exceeds cap 999999 (§12.2); this is not an ISA width limit");
    }
    Ok(())
}

fn masked(v: &BigUint, width: u64) -> BigUint {
    v & mask(width)
}

// ---------- tokeniser / statements ----------

pub fn tokenise(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_str {
            cur.push(ch);
            if ch == '\\' {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                cur.push(ch);
            }
            '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ' ' if depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Split a parenthesised operand list "(a, b)" / "()" / "(a)".
pub fn list_items(tok: &str) -> Vec<String> {
    let inner = tok
        .strip_prefix('(')
        .and_then(|t| t.strip_suffix(')'))
        .unwrap_or(tok);
    let inner = inner.trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner.split(", ").map(|x| x.trim().to_string()).collect()
}
pub(crate) fn canonical_list(text: &str) -> Option<Vec<String>> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return None;
    }
    let entries = list_items(text);
    if format!("({})", entries.join(", ")) != text
        || entries.iter().any(|s| s.is_empty() || s.contains(','))
    {
        return None;
    }
    Some(entries)
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Line(Vec<String>),
    Block {
        head: Vec<String>,
        body: Vec<Stmt>,
        rest: Option<Vec<Stmt>>,
    },
}

/// Check recursive-AST host depth with a flat scan before constructing any
/// nested Stmt vectors. This does not classify ISA source as ill-formed.
pub(crate) fn source_nesting_limit(object: &Object) -> Option<crate::fault::HostLimit> {
    for function in &object.functions {
        let mut depth = 0usize;
        for line in function.bytes.lines().skip(1) {
            if line == "}" {
                depth = depth.saturating_sub(1);
            } else if line != "} {"
                && !line.starts_with("regs ")
                && tokenise(line).last().map(String::as_str) == Some("{")
            {
                depth += 1;
                if depth > 64 {
                    return Some(crate::fault::HostLimit {
                        quantity: format!(
                            "structured source nesting in @{} exceeds the host depth cap 64",
                            function.name
                        ),
                    });
                }
            }
        }
    }
    None
}

/// Parse block bodies from raw function lines (starting after the header).
/// Returns (block name -> statements) preserving order.
pub fn parse_blocks(func: &Function) -> Vec<(String, Vec<Stmt>)> {
    let lines: Vec<&str> = func.bytes.split('\n').collect();
    let mut out: Vec<(String, Vec<Stmt>)> = Vec::new();
    let mut i = 1usize;
    while i < lines.len() {
        let line = lines[i];
        if let Some(rest) = line.strip_prefix('^') {
            let name = rest.split('(').next().unwrap_or("").to_string();
            i += 1;
            let (stmts, ni) = parse_seq(&lines, i);
            i = ni;
            out.push((name, stmts));
        } else {
            i += 1;
        }
    }
    out
}

fn parse_seq(lines: &[&str], mut i: usize) -> (Vec<Stmt>, usize) {
    let mut out = Vec::new();
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with('^') || line == "}" || line == "} {" {
            break;
        }
        if line.starts_with("regs ") || line.starts_with("regs{") {
            i += 1;
            continue;
        }
        let toks = tokenise(line);
        if toks.last().map(String::as_str) == Some("{") {
            let head = toks[..toks.len() - 1].to_vec();
            let (body, ni) = parse_seq(lines, i + 1);
            i = ni;
            // lines[i] is "}" or "} {"
            if i < lines.len() && lines[i] == "} {" {
                let (rest, ni2) = parse_seq(lines, i + 1);
                i = ni2;
                if i < lines.len() && lines[i] == "}" {
                    i += 1;
                }
                out.push(Stmt::Block {
                    head,
                    body,
                    rest: Some(rest),
                });
            } else {
                if i < lines.len() && lines[i] == "}" {
                    i += 1;
                }
                out.push(Stmt::Block {
                    head,
                    body,
                    rest: None,
                });
            }
        } else {
            out.push(Stmt::Line(toks));
            i += 1;
        }
    }
    (out, i)
}

fn flatten(stmts: &[Stmt], out: &mut Vec<Vec<String>>) {
    for s in stmts {
        match s {
            Stmt::Line(t) => out.push(t.clone()),
            Stmt::Block { body, rest, .. } => {
                flatten(body, out);
                if let Some(r) = rest {
                    flatten(r, out);
                }
            }
        }
    }
}

/// All instruction token-lines of a function mentioning any of `opcodes`.
pub fn find_opcodes(func: &Function) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for (_, stmts) in parse_blocks(func) {
        flatten(&stmts, &mut out);
    }
    out
}

// ---------- classical argument values ----------

#[derive(Debug, Clone)]
pub enum Classical {
    Bits(BigUint),
    Token([BigUint; 4]),
}

/// Decode invocation args (certificate JSON values) by signature types.
pub fn decode_invocation_args(
    args: &[crate::json::Value],
    types: &[String],
) -> Result<Vec<Classical>, EvalError> {
    let mut out = Vec::new();
    for (a, ty) in args.iter().zip(types.iter()) {
        if ty == "token" {
            let t = a
                .as_arr()
                .ok_or_else(|| EvalError::Unchecked("token arg is not a tuple".into()))?;
            let mut tuple: [BigUint; 4] = Default::default();
            for (i, c) in t.iter().enumerate() {
                tuple[i] = c.as_str().and_then(|x| x.parse().ok()).ok_or_else(|| {
                    EvalError::Unchecked("token tuple component undecodable".into())
                })?;
            }
            out.push(Classical::Token(tuple));
        } else if ty == "f64" {
            let x = a.as_str().unwrap_or("");
            let bits = u64::from_str_radix(x.strip_prefix("0x").unwrap_or(""), 16)
                .map(BigUint::from)
                .map_err(|_| EvalError::Unchecked("f64 arg undecodable".into()))?;
            out.push(Classical::Bits(bits));
        } else {
            let v: BigUint = a
                .as_str()
                .and_then(|x| x.parse().ok())
                .ok_or_else(|| EvalError::Unchecked("classical arg undecodable".into()))?;
            out.push(Classical::Bits(v));
        }
    }
    Ok(out)
}

// ---------- lifted oracle reference evaluator (F_B, subset) ----------

const FAULT_BUDGET_STEPS: u64 = 1;
const FAULT_BUDGET_HEAP: u64 = 2;
const FAULT_BUDGET_DEPTH: u64 = 3;
const FAULT_UB_TRAP: u64 = 4;
const FAULT_UB_DIV0: u64 = 5;
const FAULT_UB_SDIV_OVERFLOW: u64 = 6;
const FAULT_UB_MEMFAULT: u64 = 7;

pub struct OracleOutcome {
    pub payload: BigUint,
    pub code: BigUint,
}

fn reference_step(work: &mut u64) -> Result<(), EvalError> {
    if *work == 0 {
        return unchecked("unchecked (host limit): reference replay work (rows, instructions and loop visits) exceeds the shared host limit (§12.2)");
    }
    *work -= 1;
    Ok(())
}

struct Lifted<'a, 'w> {
    object: &'a Object,
    vals: HashMap<String, BigUint>,
    widths: HashMap<String, u64>,
    tokens: HashMap<String, [BigUint; 4]>,
    result_val: BigUint,
    tgt_width: u64,
    depth: usize,
    work: &'w mut u64,
    oracle_name: String,
    heap_limit: BigUint,
    memory: Option<crate::lifted_memory::Memory>,
    locations: HashSet<String>,
    fault_width: u64,
    region_names: HashMap<String, String>,
    in_lproc: bool,
}

/// F_B of an oracle on (ins, mem cells in interface-region order, args)
/// under `budget`. Memory rows use the independent sparse byte model;
/// Call/return replay instantiates fixed bindings and canonical SCC placement.
pub fn eval_oracle(
    object: &Object,
    oracle: &Symbol,
    ins: &[BigUint],
    mem: &[Vec<BigUint>],
    args: &[Classical],
    budget: &[BigUint; 4],
    depth: usize,
) -> Result<OracleOutcome, EvalError> {
    eval_oracle_bounded(
        object,
        oracle,
        ins,
        mem,
        args,
        budget,
        depth,
        crate::cert::Limits::default().max_reference_steps,
    )
}

/// Bound host work independently of the ISA budget. Nested oracle invocations
/// use their own ISA tuples but share this invocation's host row counter.
pub fn eval_oracle_bounded(
    object: &Object,
    oracle: &Symbol,
    ins: &[BigUint],
    mem: &[Vec<BigUint>],
    args: &[Classical],
    budget: &[BigUint; 4],
    depth: usize,
    row_limit: u64,
) -> Result<OracleOutcome, EvalError> {
    let mut work = row_limit;
    eval_oracle_work(object, oracle, ins, mem, args, budget, depth, &mut work)
}
fn eval_oracle_work(
    object: &Object,
    oracle: &Symbol,
    ins: &[BigUint],
    mem: &[Vec<BigUint>],
    args: &[Classical],
    budget: &[BigUint; 4],
    depth: usize,
    work: &mut u64,
) -> Result<OracleOutcome, EvalError> {
    if depth > 64 {
        return unchecked("unchecked (host limit): oracle call depth exceeds 64 (§12.2)");
    }
    let func = object.function(&oracle.name).ok_or_else(|| {
        EvalError::Unchecked(format!("oracle @{} has no body in the object", oracle.name))
    })?;
    let iface = object
        .interface(&oracle.name)
        .map_err(EvalError::Unchecked)?;
    let bad_interface = || {
        EvalError::Reject(Reject::new(
            "E_CERT_OBJECT",
            "§3.7",
            "object",
            "oracle arguments, types or names disagree with its declared interface",
        ))
    };
    if ins.len() != func.groups[0].len()
        || mem.len() != iface.regions.len()
        || args.len() != iface.args.len()
        || func.groups[4].len() != iface.args.len()
    {
        return Err(bad_interface());
    }
    let mut names = HashSet::new();
    for param in func.groups.iter().flatten() {
        if !names.insert(param.name.clone()) {
            return Err(bad_interface());
        }
    }
    for ((param, ty), value) in func.groups[4].iter().zip(&iface.args).zip(args) {
        if param.ty != *ty {
            return Err(bad_interface());
        }
        match value {
            Classical::Token(_) if ty == "token" => {}
            Classical::Bits(v) if ty != "token" => {
                let width = classical_width(ty, object.header.ptr_width)?;
                if v.bits() > width {
                    return Err(bad_interface());
                }
            }
            _ => return Err(bad_interface()),
        }
    }
    for width in iface
        .bundles
        .iter()
        .map(|b| b.width)
        .chain(iface.regions.iter().map(|r| r.w))
    {
        reference_width(width)?;
    }
    // Flatten rows in block text order; block first-row indices.
    let blocks = parse_blocks(func);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut block_first: HashMap<String, usize> = HashMap::new();
    for (name, stmts) in &blocks {
        block_first.insert(name.clone(), rows.len());
        let mut flat = Vec::new();
        flatten(stmts, &mut flat);
        rows.extend(flat);
    }
    let mut env = Lifted {
        object,
        vals: HashMap::new(),
        widths: HashMap::new(),
        tokens: HashMap::new(),
        result_val: BigUint::zero(),
        tgt_width: iface
            .bundles
            .iter()
            .find(|b| b.group == "tgt")
            .map(|b| b.width)
            .unwrap_or(0),
        depth,
        work,
        oracle_name: oracle.name.clone(),
        heap_limit: budget[1].clone(),
        memory: None,
        locations: HashSet::new(),
        fault_width: iface
            .bundles
            .iter()
            .find(|b| b.group == "fault")
            .expect("oracle fault interface")
            .width,
        region_names: func.groups[3]
            .iter()
            .map(|p| (p.name.clone(), p.name.clone()))
            .collect(),
        in_lproc: false,
    };
    // Declared locations from the regs line.
    for line in func.bytes.split('\n') {
        if let Some(rest) = line.strip_prefix("regs ") {
            let inner = rest.trim().trim_start_matches('{').trim_end();
            let inner = inner
                .split(" bounds")
                .next()
                .unwrap_or(inner)
                .trim_end_matches('}')
                .trim();
            if !inner.is_empty() {
                for decl in inner.split(", ") {
                    if let Some((name, ty)) = decl.trim().split_once(':') {
                        if let Some(digits) = ty
                            .trim()
                            .strip_prefix("reg[")
                            .and_then(|t| t.strip_suffix(']'))
                        {
                            if !crate::rat::is_canonical_uint_str(digits) || digits == "0" {
                                return Err(EvalError::Reject(Reject::new(
                                    "E_CERT_OBJECT",
                                    "§3.3",
                                    "object",
                                    "location width must be a positive canonical integer",
                                )));
                            }
                            let w=digits.parse::<u64>().map_err(|_|EvalError::Unchecked("host limit: location width exceeds the u64 reference host representation".into()))?;
                            reference_width(w)?;
                            if !names.insert(name.trim().to_string()) {
                                return Err(bad_interface());
                            }
                            env.widths.insert(name.trim().to_string(), w);
                            env.vals.insert(name.trim().to_string(), BigUint::zero());
                            env.locations.insert(name.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    // Pseudo-locations: in bundles (by header name) and classical args.
    for spec in func.groups[0].iter().zip(ins.iter()) {
        let width = crate::object::wire_width_of(&spec.0.ty).unwrap_or(0);
        env.widths.insert(spec.0.name.clone(), width);
        env.vals.insert(spec.0.name.clone(), spec.1.clone());
    }
    for (spec, value) in func.groups[4].iter().zip(args.iter()) {
        match value {
            Classical::Bits(v) => {
                let width = if spec.ty == "ptr" {
                    object.header.ptr_width
                } else if spec.ty == "f64" {
                    64
                } else {
                    spec.ty
                        .strip_prefix('i')
                        .and_then(|n| n.parse().ok())
                        .unwrap_or(0)
                };
                reference_width(width)?;
                env.widths.insert(spec.name.clone(), width);
                env.vals.insert(spec.name.clone(), v.clone());
            }
            Classical::Token(t) => {
                env.tokens.insert(spec.name.clone(), t.clone());
            }
        }
    }
    let budget_b = &budget[0];
    let mut steps: BigUint = BigUint::zero();
    let mut pc: usize = *block_first.get("entry").unwrap_or(&0);
    // Caller mem cells keyed by caller-facing region param name.
    let mut mem_by_name: HashMap<String, Vec<BigUint>> = HashMap::new();
    for (spec, cells) in func.groups[3].iter().zip(mem.iter()) {
        mem_by_name.insert(spec.name.clone(), cells.clone());
    }
    if procedures::has_call(&rows) {
        return procedures::run(&mut env, &rows, &mut mem_by_name, args, budget);
    }
    loop {
        if steps >= *budget_b {
            return Ok(OracleOutcome {
                payload: BigUint::zero(),
                code: BigUint::from(FAULT_BUDGET_STEPS),
            });
        }
        if pc >= rows.len() {
            return unchecked(format!(
                "oracle @{} control flow leaves its rows (pc {pc})",
                oracle.name
            ));
        }
        reference_step(env.work)?;
        let toks = rows[pc].clone();
        steps += BigUint::one();
        match env.exec_row(&toks, &rows, &block_first, &mut mem_by_name, args)? {
            RowFlow::Next => pc += 1,
            RowFlow::Jump(target) => pc = target,
            RowFlow::Halt(outcome) => return Ok(outcome),
        }
    }
}

enum RowFlow {
    Next,
    Jump(usize),
    Halt(OracleOutcome),
}

impl Lifted<'_, '_> {
    fn fault_cause(&self, text: &str) -> Result<BigUint, EvalError> {
        let bad = || {
            EvalError::Reject(Reject::new("E_CERT_OBJECT","§4.2/§5.4","object","fault cause must be a nonnegative canonical literal or standard cause, within the fault width"))
        };
        let standard = match text {
            "FAULT_NONE" => Some(0u32),
            "FAULT_BUDGET_STEPS" => Some(1),
            "FAULT_BUDGET_HEAP" => Some(2),
            "FAULT_BUDGET_DEPTH" => Some(3),
            "FAULT_UB_TRAP" => Some(4),
            "FAULT_UB_DIV0" => Some(5),
            "FAULT_UB_SDIV_OVERFLOW" => Some(6),
            "FAULT_UB_MEMFAULT" => Some(7),
            "FAULT_EFFECT_UNMODELLED" => Some(8),
            "FAULT_CALLEE" => Some(9),
            "FAULT_ENTROPY_EXHAUSTED" => Some(10),
            _ => None,
        };
        let code = match standard {
            Some(n) => BigUint::from(n),
            None => {
                let digits = text.strip_prefix('-').unwrap_or(text);
                if !crate::rat::is_canonical_uint_str(digits) {
                    return Err(bad());
                }
                let n = digits.parse::<BigUint>().map_err(|_| bad())?;
                if text.starts_with('-') && !n.is_zero() {
                    return Err(bad());
                }
                n
            }
        };
        if code.bits() > self.fault_width {
            return Err(bad());
        }
        Ok(if code.is_zero() {
            BigUint::from(FAULT_UB_TRAP)
        } else {
            code
        })
    }
    fn memory(
        &mut self,
        mem: &HashMap<String, Vec<BigUint>>,
    ) -> Result<&mut crate::lifted_memory::Memory, EvalError> {
        if self.memory.is_none() {
            self.memory = Some(crate::lifted_memory::Memory::new(
                self.object,
                &self.oracle_name,
                mem,
                &self.heap_limit,
            )?);
        }
        Ok(self.memory.as_mut().unwrap())
    }
    fn region_name(&self, name: &str) -> Result<&str, EvalError> {
        self.region_names
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| {
                EvalError::Reject(Reject::new(
                    "E_CERT_OBJECT",
                    "§3.7",
                    "object",
                    "memory operand is not a mapped interface region or a fixed region parameter",
                ))
            })
    }
    fn pointer(&self, name: &str) -> Result<BigUint, EvalError> {
        if self.width_of(name)? != self.object.header.ptr_width {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                "memory address is not a pointer-width location",
            )));
        }
        self.read(name)
    }
    fn memory_length(&self, token: &str) -> Result<BigUint, EvalError> {
        if self.vals.contains_key(token) {
            return self.read(token);
        }
        let digits = token.strip_prefix('-').unwrap_or(token);
        let value = digits.parse::<BigUint>().ok().filter(|v| {
            crate::rat::is_canonical_uint_str(digits)
                && (!token.starts_with('-') || v.is_zero())
                && v.bits() <= self.object.header.ptr_width
        });
        value.ok_or_else(|| {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                "memory length literal is not canonical or exceeds pointer width",
            ))
        })
    }
    fn width_of(&self, name: &str) -> Result<u64, EvalError> {
        self.widths.get(name).copied().ok_or_else(|| {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.7",
                "object",
                format!(
                    "operand {name:?} is not a declared bit-vector location or pseudo-location"
                ),
            ))
        })
    }
    fn read(&self, tok: &str) -> Result<BigUint, EvalError> {
        if let Some(v) = self.vals.get(tok) {
            return Ok(v.clone());
        }
        if let Ok(lit) = tok.parse::<BigUint>() {
            return Ok(lit);
        }
        Err(EvalError::Reject(Reject::new(
            "E_CERT_OBJECT",
            "§3.7",
            "object",
            format!("operand {tok:?} is not a declared bit-vector location or pseudo-location"),
        )))
    }
    fn write(&mut self, name: &str, v: BigUint, width: u64) {
        self.widths.insert(name.to_string(), width);
        self.vals.insert(name.to_string(), masked(&v, width));
    }
    fn signed(&self, v: &BigUint, width: u64) -> num_bigint::BigInt {
        use num_bigint::BigInt;
        let bits = masked(v, width);
        if width > 0 && (&bits >> (width - 1)) == BigUint::one() {
            BigInt::from(bits) - (BigInt::one() << width)
        } else {
            BigInt::from(bits)
        }
    }

    fn exec_row(
        &mut self,
        toks: &[String],
        _rows: &[Vec<String>],
        block_first: &HashMap<String, usize>,
        mem_by_name: &mut HashMap<String, Vec<BigUint>>,
        _args: &[Classical],
    ) -> Result<RowFlow, EvalError> {
        // Assignment form: lhs = op args   (lhs a name or (a, b, ..))
        let (lhs, op, operands): (Option<Vec<String>>, &str, &[String]) =
            if toks.len() >= 3 && toks[1] == "=" {
                let lhs = if toks[0].starts_with('(') {
                    list_items(&toks[0])
                } else {
                    vec![toks[0].clone()]
                };
                (Some(lhs), toks[2].as_str(), &toks[3..])
            } else {
                (
                    None,
                    toks.first().map(String::as_str).unwrap_or(""),
                    &toks[1.min(toks.len())..],
                )
            };
        let fault = |code: u64| {
            RowFlow::Halt(OracleOutcome {
                payload: BigUint::zero(),
                code: BigUint::from(code),
            })
        };
        let unsupported = |what: &str| -> Result<RowFlow, EvalError> {
            unchecked(format!(
                "lifted row {what} is outside this checker's F_B subset"
            ))
        };
        let bad = || {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3/§3.7",
                "object",
                "lifted result/operand locations, widths or labels do not match the opcode schema",
            ))
        };
        if let Some(names) = &lhs {
            if names.is_empty() || names.iter().any(|n| !self.locations.contains(n)) {
                return Err(bad());
            }
        }
        let memory_shape = match op {
            "lift.addr" | "lift.alloc" => Some((true, 1)),
            "lift.free" => Some((false, 1)),
            "lift.boundscheck" => Some((false, 2)),
            "lift.label" | "lift.mov" => Some((true, 1)),
            "qjump" | "qjumpr" => Some((false, 1)),
            "qbranch" => Some((false, 3)),
            "lift.select" => Some((true, 3)),
            "fault" => Some((false, 1)),
            "lift.faultif" => Some((false, 2)),
            op if [
                "lift.add",
                "lift.sub",
                "lift.mul",
                "lift.and",
                "lift.or",
                "lift.xor",
                "lift.shl",
                "lift.lshr",
                "lift.ashr",
                "lift.udiv",
                "lift.urem",
                "lift.sdiv",
                "lift.srem",
            ]
            .contains(&op)
                || op.starts_with("lift.icmp.") =>
            {
                Some((true, 2))
            }
            op if op.starts_with("lift.load.") => Some((true, 1)),
            op if op.starts_with("lift.store.") => Some((false, 2)),
            _ => None,
        };
        if let Some((assigned, count)) = memory_shape {
            if operands.len() != count
                || lhs.as_ref().map(Vec::len) != if assigned { Some(1) } else { None }
            {
                return Err(EvalError::Reject(Reject::new(
                    "E_CERT_OBJECT",
                    "§4.2",
                    "object",
                    "lifted row has the wrong assignment/operand arity",
                )));
            }
        }
        match op.split('.').take(2).collect::<Vec<_>>().join(".").as_str() {
            _ if op.starts_with("lift.const") => {
                let bad = || {
                    EvalError::Reject(Reject::new(
                    "E_CERT_REPLAY", "§3.3", "",
                    "lift.const.iN requires one canonical INT in [-2^(N-1), 2^N), a matching destination and positive N",
                ))
                };
                let width = op
                    .strip_prefix("lift.const.i")
                    .filter(|s| crate::rat::is_canonical_uint_str(s) && *s != "0")
                    .ok_or_else(bad)?;
                let Some(n) = width.parse::<u64>().ok().filter(|n| *n < 1_000_000) else {
                    return unchecked("host limit: lifted constant width exceeds the reference bit-vector cap 999999 (§12.2); lifted widths have no ISA upper bound (§4.2)");
                };
                let dest = lhs.as_ref().filter(|v| v.len() == 1).ok_or_else(bad)?;
                if operands.len() != 1 || self.width_of(&dest[0])? != n {
                    return Err(bad());
                }
                let literal = &operands[0];
                if !crate::rat::is_canonical_uint_str(literal.strip_prefix('-').unwrap_or(literal))
                {
                    return Err(bad());
                }
                let v = literal.parse::<num_bigint::BigInt>().map_err(|_| bad())?;
                let lower = -(num_bigint::BigInt::one() << (n - 1));
                let upper = num_bigint::BigInt::one() << n;
                if v < lower || v >= upper {
                    return Err(bad());
                }
                self.write(&dest[0], bigint_to_masked(v, n), n);
                Ok(RowFlow::Next)
            }
            _ if op == "lift.label" => {
                let target = operands
                    .first()
                    .and_then(|x| x.strip_prefix('^'))
                    .unwrap_or("");
                let idx = *block_first.get(target).ok_or_else(bad)?;
                let name = lhs.unwrap()[0].clone();
                let w = self.width_of(&name)?;
                self.write(&name, BigUint::from(idx), w);
                Ok(RowFlow::Next)
            }
            _ if op == "lift.mov" => {
                let v = self.read(&operands[0])?;
                let name = lhs.unwrap()[0].clone();
                let w = self.width_of(&operands[0])?;
                if self.width_of(&name)? != w {
                    return Err(bad());
                }
                self.write(&name, v, w);
                Ok(RowFlow::Next)
            }
            _ if op == "lift.addr" => {
                let destination = &lhs
                    .as_ref()
                    .ok_or_else(|| EvalError::Unchecked("lift.addr has no destination".into()))?[0];
                if self.width_of(destination)? != self.object.header.ptr_width {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§3.3",
                        "object",
                        "lift.addr destination has wrong width",
                    )));
                }
                let name = operands[0].strip_prefix('@').ok_or_else(|| {
                    EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§4.2",
                        "object",
                        "lift.addr requires an @name operand",
                    ))
                })?;
                if self.in_lproc
                    && self
                        .object
                        .layout
                        .as_ref()
                        .and_then(|l| l.get("globals"))
                        .and_then(|g| g.get(name))
                        .is_none()
                {
                    return Err(EvalError::Reject(Reject::new("E_CERT_OBJECT", "§4.2", "object",
                        "lift.addr in an lproc may name globals only; region bases must be passed as locations")));
                }
                let base = self.memory(mem_by_name)?.address(name).ok_or_else(|| {
                    EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§4.2",
                        "object",
                        "lift.addr does not name a global or mapped region of this oracle",
                    ))
                })?;
                self.write(destination, base.into(), self.object.header.ptr_width);
                Ok(RowFlow::Next)
            }
            _ if op == "lift.alloc" => {
                let size = self.memory_length(&operands[0])?;
                let destination = &lhs
                    .as_ref()
                    .ok_or_else(|| EvalError::Unchecked("lift.alloc has no destination".into()))?
                    [0];
                if self.width_of(destination)? != self.object.header.ptr_width {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§3.3",
                        "object",
                        "lift.alloc destination has wrong width",
                    )));
                }
                match self.memory(mem_by_name)?.allocate(&size)? {
                    Some(base) => {
                        self.write(destination, base.into(), self.object.header.ptr_width);
                        Ok(RowFlow::Next)
                    }
                    None => Ok(fault(FAULT_BUDGET_HEAP)),
                }
            }
            _ if op == "lift.free" => {
                let address = self.pointer(&operands[0])?;
                if self.memory(mem_by_name)?.free(&address) {
                    Ok(RowFlow::Next)
                } else {
                    Ok(fault(FAULT_UB_MEMFAULT))
                }
            }
            _ if op == "lift.boundscheck" => {
                let address = self.pointer(&operands[0])?;
                let size = self.memory_length(&operands[1])?;
                if self.memory(mem_by_name)?.inrange(&address, &size) {
                    Ok(RowFlow::Next)
                } else {
                    Ok(fault(FAULT_UB_MEMFAULT))
                }
            }
            _ if op.starts_with("lift.load.") || op.starts_with("lift.store.") => {
                let width = op
                    .rsplit_once(".i")
                    .filter(|(_, n)| crate::rat::is_canonical_uint_str(n))
                    .and_then(|(_, n)| n.parse::<u64>().ok())
                    .filter(|n| *n >= 8 && *n <= 128 && *n % 8 == 0)
                    .ok_or_else(|| {
                        EvalError::Reject(Reject::new(
                            "E_CERT_OBJECT",
                            "§4.2",
                            "object",
                            "load/store width must be a multiple of eight from 8 through 128",
                        ))
                    })?;
                let address = self.pointer(&operands[0])?;
                let bytes = (width / 8) as usize;
                let register = if op.starts_with("lift.load.") {
                    &lhs.as_ref().ok_or_else(|| {
                        EvalError::Unchecked("lift.load has no destination".into())
                    })?[0]
                } else {
                    &operands[1]
                };
                if self.width_of(register)? != width {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§3.3",
                        "object",
                        "load/store value location has wrong width",
                    )));
                }
                if !self
                    .memory(mem_by_name)?
                    .inrange(&address, &BigUint::from(bytes))
                {
                    return Ok(fault(FAULT_UB_MEMFAULT));
                }
                if op.starts_with("lift.load.") {
                    let value = self.memory(mem_by_name)?.load(&address, bytes, mem_by_name);
                    self.write(register, value, width);
                } else {
                    let value = self.read(register)?;
                    if !self
                        .memory(mem_by_name)?
                        .store(&address, &value, bytes, mem_by_name)?
                    {
                        return Ok(fault(FAULT_UB_MEMFAULT));
                    }
                }
                Ok(RowFlow::Next)
            }
            _ if op == "lift.parmov" => {
                if operands.len() != 1 || !toks[0].starts_with('(') || !operands[0].starts_with('(')
                {
                    return Err(bad());
                }
                let names = lhs.unwrap();
                if names.iter().collect::<HashSet<_>>().len() != names.len() {
                    return Err(bad());
                }
                let sources = list_items(&operands[0]);
                if names.len() != sources.len() {
                    return Err(bad());
                }
                let srcs: Vec<BigUint> = sources
                    .iter()
                    .map(|o| self.read(o))
                    .collect::<Result<_, _>>()?;
                let widths: Vec<u64> = sources
                    .iter()
                    .map(|o| self.width_of(o))
                    .collect::<Result<_, _>>()?;
                for (name, width) in names.iter().zip(&widths) {
                    if self.width_of(name)? != *width {
                        return Err(bad());
                    }
                }
                for ((n, v), w) in names.into_iter().zip(srcs).zip(widths) {
                    self.write(&n, v, w);
                }
                Ok(RowFlow::Next)
            }
            _ if op == "lift.select" => {
                let w = self.width_of(&operands[1])?;
                if self.width_of(&operands[0])? != 1
                    || self.width_of(&operands[2])? != w
                    || self.width_of(&lhs.as_ref().unwrap()[0])? != w
                {
                    return Err(bad());
                }
                let p = self.read(&operands[0])?;
                let v = if p == BigUint::one() {
                    self.read(&operands[1])?
                } else {
                    self.read(&operands[2])?
                };
                let name = lhs.unwrap()[0].clone();
                self.write(&name, v, w);
                Ok(RowFlow::Next)
            }
            _ if op.starts_with("lift.zext")
                || op.starts_with("lift.sext")
                || op.starts_with("lift.trunc") =>
            {
                let bad = || {
                    EvalError::Reject(Reject::new("E_CERT_OBJECT","§3.3","object","cast requires one declared source/destination with matching positive result width and the specified widening/narrowing relation"))
                };
                let digits = ["lift.zext.i", "lift.sext.i", "lift.trunc.i"]
                    .iter()
                    .find_map(|p| op.strip_prefix(p))
                    .filter(|s| crate::rat::is_canonical_uint_str(s) && *s != "0")
                    .ok_or_else(bad)?;
                let m = digits.parse::<u64>().map_err(|_| {
                    EvalError::Unchecked(
                        "host limit: cast width exceeds the u64 reference host representation"
                            .into(),
                    )
                })?;
                reference_width(m)?;
                if operands.len() != 1 || lhs.as_ref().map(Vec::len) != Some(1) {
                    return Err(bad());
                }
                let destination = &lhs.as_ref().unwrap()[0];
                let w = self.width_of(&operands[0]).map_err(|_| bad())?;
                if self.width_of(destination).map_err(|_| bad())? != m
                    || if op.starts_with("lift.trunc.") {
                        m >= w
                    } else {
                        m <= w
                    }
                {
                    return Err(bad());
                }
                let v = self.read(&operands[0])?;
                let out = if op.contains("sext") {
                    let w = self.width_of(&operands[0])?;
                    let s = self.signed(&v, w);
                    bigint_to_masked(s, m)
                } else {
                    masked(&v, m)
                };
                self.write(&lhs.unwrap()[0], out, m);
                Ok(RowFlow::Next)
            }
            _ if op.starts_with("lift.icmp.") => {
                let pred = op.rsplit('.').next().unwrap_or("");
                let a = self.read(&operands[0])?;
                let b = self.read(&operands[1])?;
                let w = self.width_of(&operands[0])?;
                if self.width_of(&operands[1])? != w
                    || self.width_of(&lhs.as_ref().unwrap()[0])? != 1
                {
                    return Err(bad());
                }
                let (sa, sb) = (self.signed(&a, w), self.signed(&b, w));
                let r = match pred {
                    "eq" => a == b,
                    "ne" => a != b,
                    "ult" => a < b,
                    "ule" => a <= b,
                    "slt" => sa < sb,
                    "sle" => sa <= sb,
                    _ => return unsupported(op),
                };
                self.write(
                    &lhs.unwrap()[0],
                    if r { BigUint::one() } else { BigUint::zero() },
                    1,
                );
                Ok(RowFlow::Next)
            }
            _ if matches!(op, "lift.udiv" | "lift.urem" | "lift.sdiv" | "lift.srem") => {
                let a = self.read(&operands[0])?;
                let b = self.read(&operands[1])?;
                let w = self.width_of(&operands[0])?;
                let signed = op.contains("sdiv") || op.contains("srem");
                if self.width_of(&operands[1])? != w
                    || self.width_of(&lhs.as_ref().unwrap()[0])? != w
                {
                    return Err(bad());
                }
                if b.is_zero() {
                    return Ok(fault(FAULT_UB_DIV0));
                }
                if signed {
                    let (sa, sb) = (self.signed(&a, w), self.signed(&b, w));
                    let min = -(num_bigint::BigInt::one() << (w - 1));
                    if sa == min && sb == -num_bigint::BigInt::one() {
                        return Ok(fault(FAULT_UB_SDIV_OVERFLOW));
                    }
                    let r = if op.contains("sdiv") {
                        &sa / &sb
                    } else {
                        &sa % &sb
                    };
                    self.write(&lhs.unwrap()[0], bigint_to_masked(r, w), w);
                } else {
                    let r = if op.contains("udiv") {
                        &a / &b
                    } else {
                        &a % &b
                    };
                    self.write(&lhs.unwrap()[0], r, w);
                }
                Ok(RowFlow::Next)
            }
            _ if op == "lift.retval" => {
                let v = self.read(&operands[0])?;
                self.result_val = masked(&(&self.result_val ^ &v), self.tgt_width);
                Ok(RowFlow::Halt(OracleOutcome {
                    payload: self.result_val.clone(),
                    code: BigUint::zero(),
                }))
            }
            _ if op == "fault" => Ok(RowFlow::Halt(OracleOutcome {
                payload: BigUint::zero(),
                code: self.fault_cause(&operands[0])?,
            })),
            _ if op == "lift.faultif" => {
                if self.width_of(&operands[0])? != 1 {
                    return Err(bad());
                }
                let cause = self.fault_cause(&operands[1])?;
                let p = self.read(&operands[0])?;
                if p == BigUint::one() {
                    Ok(RowFlow::Halt(OracleOutcome {
                        payload: BigUint::zero(),
                        code: cause,
                    }))
                } else {
                    Ok(RowFlow::Next)
                }
            }
            _ if op == "qjump" => {
                let target = operands
                    .first()
                    .map(|x| x.trim_start_matches('^'))
                    .unwrap_or("");
                match block_first.get(target) {
                    Some(i) => Ok(RowFlow::Jump(*i)),
                    None => Err(bad()),
                }
            }
            _ if op == "qbranch" => {
                let p = self.read(&operands[0])?;
                let target = if p == BigUint::one() {
                    &operands[1]
                } else {
                    &operands[2]
                };
                // Targets may carry block-arg lists in CC form; lifted form is ^name.
                let name = target
                    .trim_start_matches('^')
                    .split('(')
                    .next()
                    .unwrap_or("");
                match block_first.get(name) {
                    Some(i) => Ok(RowFlow::Jump(*i)),
                    None => Err(bad()),
                }
            }
            _ if op == "qjumpr" => {
                let a = self.read(&operands[0])?;
                let idx = a.to_string().parse::<usize>().unwrap_or(usize::MAX);
                if idx >= _rows.len() {
                    Ok(fault(FAULT_UB_TRAP))
                } else {
                    Ok(RowFlow::Jump(idx))
                }
            }
            _ if op == "lift.oracle_call" || op == "lift.oracle_uncall" => {
                // @o tk (ins..) tgt fr (mem..) (args..)
                let callee_name = operands[0].trim_start_matches('@').to_string();
                let tk_name = operands[1].clone();
                let ins_toks = list_items(&operands[2]);
                let tgt_loc = operands[3].clone();
                let fr_loc = operands[4].clone();
                let mem_toks = operands.get(5).map(|x| list_items(x)).unwrap_or_default();
                let arg_toks = operands.get(6).map(|x| list_items(x)).unwrap_or_default();
                let budget = self.tokens.get(&tk_name).cloned().ok_or_else(|| {
                    EvalError::Unchecked(format!(
                        "token operand {tk_name:?} has no bound tuple in this invocation"
                    ))
                })?;
                let callee = self.object.symbol(&callee_name).cloned().ok_or_else(|| {
                    EvalError::Unchecked(format!(
                        "callee @{callee_name} is not defined in the object"
                    ))
                })?;
                let ins_vals: Vec<BigUint> = ins_toks
                    .iter()
                    .map(|t| self.read(t))
                    .collect::<Result<_, _>>()?;
                let mem_vals: Vec<Vec<BigUint>> = mem_toks
                    .iter()
                    .map(|n| {
                        self.region_name(n).and_then(|root| {
                            mem_by_name.get(root).cloned().ok_or_else(|| {
                                EvalError::Reject(Reject::new(
                                    "E_CERT_OBJECT",
                                    "§3.7",
                                    "object",
                                    "mapped region contents are absent",
                                ))
                            })
                        })
                    })
                    .collect::<Result<_, _>>()?;
                let mut callee_args = Vec::new();
                for t in &arg_toks {
                    if let Some(tuple) = self.tokens.get(t) {
                        callee_args.push(Classical::Token(tuple.clone()));
                    } else {
                        callee_args.push(Classical::Bits(self.read(t)?));
                    }
                }
                let outcome = eval_oracle_work(
                    self.object,
                    &callee,
                    &ins_vals,
                    &mem_vals,
                    &callee_args,
                    &budget,
                    self.depth + 1,
                    self.work,
                )?;
                let tgt_cur = self.read(&tgt_loc).unwrap_or_default();
                let tgt_w = self.width_of(&tgt_loc)?;
                self.write(&tgt_loc, &tgt_cur ^ &outcome.payload, tgt_w);
                let fr_cur = self.read(&fr_loc).unwrap_or_default();
                let fr_w = self.width_of(&fr_loc)?;
                self.write(&fr_loc, &fr_cur ^ &outcome.code, fr_w);
                Ok(RowFlow::Next)
            }
            _ if op == "lift.perm_call" => {
                // @a (mut..) (ins..) (mem..) (args..) — locations hold values.
                let callee_name = operands[0].trim_start_matches('@').to_string();
                let mut_toks = list_items(&operands[1]);
                let in_toks = list_items(&operands[2]);
                let mem_toks = operands.get(3).map(|x| list_items(x)).unwrap_or_default();
                let arg_toks = operands.get(4).map(|x| list_items(x)).unwrap_or_default();
                let callee = self.object.symbol(&callee_name).cloned().ok_or_else(|| {
                    EvalError::Unchecked(format!("perm callee @{callee_name} is not defined"))
                })?;
                let iface = self
                    .object
                    .interface(&callee_name)
                    .map_err(EvalError::Unchecked)?;
                let mut bundles: Vec<BigUint> = Vec::new();
                for t in mut_toks.iter().chain(in_toks.iter()) {
                    bundles.push(self.read(t)?);
                }
                let regs: Vec<Vec<BigUint>> = mem_toks
                    .iter()
                    .map(|n| {
                        self.region_name(n).and_then(|root| {
                            mem_by_name.get(root).cloned().ok_or_else(|| {
                                EvalError::Reject(Reject::new(
                                    "E_CERT_OBJECT",
                                    "§3.7",
                                    "object",
                                    "mapped region contents are absent",
                                ))
                            })
                        })
                    })
                    .collect::<Result<_, _>>()?;
                let mut callee_args = Vec::new();
                for t in &arg_toks {
                    if let Some(tuple) = self.tokens.get(t) {
                        callee_args.push(Classical::Token(tuple.clone()));
                    } else {
                        callee_args.push(Classical::Bits(self.read(t)?));
                    }
                }
                let before = Tuple {
                    bundles,
                    borrowed: vec![],
                    regions: regs,
                    args: vec![],
                };
                let after = eval_action_tuple_work(
                    self.object,
                    &callee,
                    &iface,
                    &before,
                    &callee_args,
                    0,
                    false,
                    self.depth + 1,
                    self.work,
                )?;
                for (t, v) in mut_toks.iter().zip(after.bundles.iter()) {
                    let w = self.width_of(t)?;
                    self.write(t, v.clone(), w);
                }
                for (n, cells) in mem_toks.iter().zip(after.regions.into_iter()) {
                    let root = self.region_name(n)?.to_string();
                    mem_by_name.insert(root, cells);
                }
                Ok(RowFlow::Next)
            }
            _ if op.starts_with("lift.") => {
                // Binary operators: lift.add, lift.sub, ... (section 4.2).
                let base = op.strip_prefix("lift.").unwrap_or("");
                if !matches!(
                    base,
                    "add" | "sub" | "mul" | "and" | "or" | "xor" | "shl" | "lshr" | "ashr"
                ) {
                    return unsupported(op);
                }
                let a = self.read(&operands[0])?;
                let b = self.read(&operands[1])?;
                let w = self.width_of(&operands[0])?;
                if self.width_of(&operands[1])? != w
                    || self.width_of(&lhs.as_ref().unwrap()[0])? != w
                {
                    return Err(bad());
                }
                let r = binop(base, &a, &b, w, |v| self.signed(v, w));
                self.write(&lhs.unwrap()[0], r, w);
                Ok(RowFlow::Next)
            }
            other => unsupported(other),
        }
    }
}

fn binop(
    op: &str,
    a: &BigUint,
    b: &BigUint,
    w: u64,
    signed: impl Fn(&BigUint) -> num_bigint::BigInt,
) -> BigUint {
    let r = match op {
        "add" => a + b,
        "sub" => masked(&(a + &mask(w) + BigUint::one() - b), w),
        "mul" => a * b,
        "and" => a & b,
        "or" => a | b,
        "xor" => a ^ b,
        "shl" => {
            if *b >= BigUint::from(w) {
                BigUint::zero()
            } else {
                a << b.to_string().parse::<usize>().unwrap_or(0)
            }
        }
        "lshr" => {
            if *b >= BigUint::from(w) {
                BigUint::zero()
            } else {
                a >> b.to_string().parse::<usize>().unwrap_or(0)
            }
        }
        "ashr" => {
            if *b >= BigUint::from(w) {
                BigUint::zero()
            } else {
                let shift = b.to_string().parse::<usize>().unwrap_or(0);
                let s = signed(a) >> shift;
                bigint_to_masked(s, w)
            }
        }
        _ => unreachable!(),
    };
    masked(&r, w)
}

fn bigint_to_masked(v: num_bigint::BigInt, w: u64) -> BigUint {
    let modulus = num_bigint::BigInt::one() << w;
    let r = ((v % &modulus) + &modulus) % &modulus;
    BigUint::try_from(r).unwrap_or_default()
}

/// The section 5.2 oracle contract applied to one full before-tuple, with
/// `inherited_controls` leading control bundles in the tuple.
pub fn eval_oracle_tuple(
    object: &Object,
    oracle: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    args: &[Classical],
    budget: &[BigUint; 4],
    inherited_controls: usize,
) -> Result<Tuple, EvalError> {
    let mut work = crate::cert::Limits::default().max_reference_steps;
    eval_oracle_tuple_work(
        object,
        oracle,
        iface,
        before,
        args,
        budget,
        inherited_controls,
        &mut work,
    )
}
fn eval_oracle_tuple_work(
    object: &Object,
    oracle: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    args: &[Classical],
    budget: &[BigUint; 4],
    inherited_controls: usize,
    work: &mut u64,
) -> Result<Tuple, EvalError> {
    let controls_on = before.bundles[..inherited_controls]
        .iter()
        .all(|b| *b == BigUint::one());
    if !controls_on {
        return Ok(before.clone());
    }
    let mut pos = inherited_controls;
    let mut ins = Vec::new();
    for b in iface.bundles.iter() {
        if b.group == "in" {
            ins.push(before.bundles[pos].clone());
        }
        pos += 1;
    }
    let outcome = eval_oracle_work(object, oracle, &ins, &before.regions, args, budget, 0, work)?;
    let mut after = before.clone();
    let mut pos = inherited_controls;
    for b in &iface.bundles {
        match b.group.as_str() {
            "tgt" => {
                after.bundles[pos] = masked(&(&before.bundles[pos] ^ &outcome.payload), b.width)
            }
            "fault" => {
                after.bundles[pos] = masked(&(&before.bundles[pos] ^ &outcome.code), b.width)
            }
            _ => {}
        }
        pos += 1;
    }
    Ok(after)
}

// ---------- basis-state action evaluator (permutation realisations) ----------

fn bits_of(v: &BigUint, width: u64) -> Vec<bool> {
    (0..width)
        .map(|i| (v >> i) & BigUint::one() == BigUint::one())
        .collect()
}
fn value_of(bits: &[bool]) -> BigUint {
    let mut v = BigUint::zero();
    for (i, b) in bits.iter().enumerate() {
        if *b {
            v |= BigUint::one() << i;
        }
    }
    v
}

fn wire_ref(tok: &str) -> (String, Option<usize>) {
    if let Some(open) = tok.find('[') {
        let name = tok[..open].to_string();
        let idx = tok[open + 1..].trim_end_matches(']').parse().ok();
        (name, idx)
    } else {
        (tok.to_string(), None)
    }
}
pub(crate) fn literal_wire_index(text: &str) -> Option<usize> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if !crate::rat::is_canonical_uint_str(digits) || (text.starts_with('-') && digits != "0") {
        return None;
    }
    digits.parse().ok()
}

struct ActionEnv<'a, 'w> {
    object: &'a Object,
    wires: HashMap<String, Vec<bool>>,
    regions: HashMap<String, Vec<BigUint>>,
    region_width: HashMap<String, u64>,
    local_region_image: HashMap<String, Vec<BigUint>>,
    args: HashMap<String, Classical>,
    predicates: Vec<(String, usize)>,
    loop_serial: u64,
    selector_guards: Vec<String>,
    borrowed_queue: VecDeque<Vec<bool>>,
    borrowed_live: Vec<(String, Vec<bool>)>,
    live_alloc: HashSet<String>,
    depth: usize,
    kind: String,
    work: &'w mut u64,
}

impl ActionEnv<'_, '_> {
    fn selector_write(&self, token: &str) -> Result<(), EvalError> {
        let (name, _) = wire_ref(token);
        if self.selector_guards.contains(&name) {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3/§4.6",
                "object",
                "qloop-while body writes or consumes its selector",
            )));
        }
        Ok(())
    }
    fn check_selector_writes(
        &self,
        op: &str,
        lhs: Option<&str>,
        operands: &[String],
    ) -> Result<(), EvalError> {
        if self.selector_guards.is_empty() {
            return Ok(());
        }
        let check = |index: usize| -> Result<(), EvalError> {
            match operands.get(index) {
                Some(s) => self.selector_write(s),
                None => Ok(()),
            }
        };
        match op {
            "qalloc" | "qborrow" | "qcopy" | "qload" => {
                if let Some(lhs) = lhs {
                    self.selector_write(lhs)
                } else {
                    Ok(())
                }
            }
            "x" | "h" | "s" | "sdg" | "t" | "tdg" | "qfree" | "qreturn" => check(0),
            "cx" | "mcx" | "qxor" | "quncopy" | "rx" | "ry" | "rz" => check(1),
            "ccx" | "qunload" | "qexchm" => check(2),
            "swap" | "qexch" | "cz" => {
                check(0)?;
                check(1)
            }
            "qinplace" => check(3),
            "oracle_call" | "oracle_uncall" => {
                check(3)?;
                check(4)
            }
            // Propagate the guard into linked action bodies; a mutable formal
            // can still be used only as a reader by that particular body.
            "action_call" => Ok(()),
            _ => Ok(()),
        }
    }
    fn storage_room(&self, wires: usize, cells: usize, width: u64) -> Result<(), EvalError> {
        reference_width(width)?;
        let live_wires: u128 = self.wires.values().map(|v| v.len() as u128).sum();
        let live_regions: u128 = self
            .regions
            .iter()
            .map(|(name, v)| v.len() as u128 * u128::from(self.region_width[name]))
            .sum();
        let total = live_wires + live_regions + wires as u128 + cells as u128 * u128::from(width);
        if total >= 1_000_000 {
            return unchecked(
                "host limit: reference live wire/region storage exceeds cap 999999 bits (§12.2)",
            );
        }
        Ok(())
    }
    fn single_wire(&self, token: &str) -> Result<(String, usize), EvalError> {
        let (name, mut index) = wire_ref(token);
        if let Some((_, digits)) = token.strip_suffix(']').and_then(|s| s.split_once('[')) {
            index = literal_wire_index(digits);
        }
        if token.contains('[') && (index.is_none() || !token.ends_with(']')) {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "",
                format!("invalid wire operand {token:?}"),
            )));
        }
        let wires = self.wires.get(&name).ok_or_else(|| {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                format!("wire {name:?} is not live"),
            ))
        })?;
        let index = match index {
            Some(index) => index,
            None if wires.len() == 1 => 0,
            None => {
                return Err(EvalError::Reject(Reject::new(
                    "E_CERT_OBJECT",
                    "§4.4 K93",
                    "object",
                    format!("{token:?} must denote exactly one wire"),
                )))
            }
        };
        if index >= wires.len() {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                "wire index is outside its live bundle",
            )));
        }
        Ok((name, index))
    }
    fn bit(&self, bundle: &str, idx: usize) -> Result<bool, EvalError> {
        self.wires
            .get(bundle)
            .and_then(|w| w.get(idx).copied())
            .ok_or_else(|| {
                EvalError::Unchecked(format!("wire {bundle}[{idx}] is not live in this replay"))
            })
    }
    fn controls_on(&self) -> Result<bool, EvalError> {
        for (b, i) in &self.predicates {
            if !self.bit(b, *i)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn bundle_value(&self, name: &str) -> Result<BigUint, EvalError> {
        self.wires.get(name).map(|w| value_of(w)).ok_or_else(|| {
            EvalError::Unchecked(format!("bundle {name:?} is not live in this replay"))
        })
    }
    fn actual_width(&self, token: &str) -> Result<usize, EvalError> {
        if token.contains('[') {
            self.single_wire(token)?;
            return Ok(1);
        }
        self.wires.get(token).map(Vec::len).ok_or_else(|| {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                "action actual is not a live wire bundle",
            ))
        })
    }
    fn actual_value(&self, token: &str) -> Result<BigUint, EvalError> {
        if token.contains('[') {
            let (name, index) = self.single_wire(token)?;
            return Ok(BigUint::from(self.bit(&name, index)?));
        }
        self.bundle_value(token)
    }
    fn deliver_actual(&mut self, token: &str, value: &BigUint) -> Result<(), EvalError> {
        let width = self.actual_width(token)?;
        if value.bits() > width as u64 {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3",
                "object",
                "callee output exceeds the actual wire width",
            )));
        }
        let (name, _) = wire_ref(token);
        if self.selector_guards.contains(&name) && self.actual_value(token)? != *value {
            return self.selector_write(token);
        }
        if token.contains('[') {
            let (name, index) = self.single_wire(token)?;
            self.wires.get_mut(&name).unwrap()[index] = !value.is_zero();
        } else {
            self.wires
                .insert(token.into(), bits_of(value, width as u64));
        }
        Ok(())
    }
    fn flip_if(&mut self, bundle: &str, idx: usize, cond: bool) -> Result<(), EvalError> {
        if cond {
            let w = self
                .wires
                .get_mut(bundle)
                .ok_or_else(|| EvalError::Unchecked(format!("wire {bundle} is not live")))?;
            if idx >= w.len() {
                return unchecked(format!("wire {bundle}[{idx}] out of range"));
            }
            w[idx] = !w[idx];
        }
        Ok(())
    }

    fn exec_stmts(&mut self, stmts: &[Stmt]) -> Result<(), EvalError> {
        for s in stmts {
            reference_step(self.work)?;
            match s {
                Stmt::Line(t) => self.exec_line(t)?,
                Stmt::Block { head, body, rest } => self.exec_block(head, body, rest.as_deref())?,
            }
        }
        Ok(())
    }

    fn exec_block(
        &mut self,
        head: &[String],
        body: &[Stmt],
        rest: Option<&[Stmt]>,
    ) -> Result<(), EvalError> {
        match head.first().map(String::as_str) {
            Some("qloop") => {
                if head.get(2).map(String::as_str) == Some("while") {
                    let n: usize = head[1]
                        .parse()
                        .map_err(|_| EvalError::Unchecked("qloop bound is not a literal".into()))?;
                    let cond_name = head.get(3).cloned().unwrap_or_default();
                    let cond = self.wires.get(&cond_name).cloned().ok_or_else(|| {
                        EvalError::Reject(Reject::new(
                            "E_CERT_OBJECT",
                            "§4.6",
                            "object",
                            "qloop-while selector is not a live wire bundle",
                        ))
                    })?;
                    if cond.len() != n {
                        return Err(EvalError::Reject(Reject::new(
                            "E_CERT_OBJECT",
                            "§4.6",
                            "object",
                            "qloop-while selector width must equal its bound",
                        )));
                    }
                    let mut flag = n > 0 && cond.first().copied().unwrap_or(false);
                    let flag_bundle = format!("$loopflag.{}", self.loop_serial);
                    self.loop_serial = self.loop_serial.checked_add(1).ok_or_else(|| {
                        EvalError::Unchecked(
                            "host limit: private loop identity counter exceeds u64".into(),
                        )
                    })?;
                    self.wires.insert(flag_bundle.clone(), vec![flag]);
                    self.selector_guards.push(cond_name);
                    for i in 0..n {
                        reference_step(self.work)?;
                        self.wires.insert(flag_bundle.clone(), vec![flag]);
                        self.predicates.push((flag_bundle.clone(), 0));
                        let r = self.exec_stmts(body);
                        self.predicates.pop();
                        r?;
                        if i + 1 < n {
                            flag = flag && cond.get(i + 1).copied().unwrap_or(false);
                        }
                    }
                    self.wires.remove(&flag_bundle);
                    self.selector_guards.pop();
                    Ok(())
                } else {
                    let n: usize = head.get(1).and_then(|x| x.parse().ok()).ok_or_else(|| {
                        EvalError::Unchecked("qloop bound is not a literal".into())
                    })?;
                    if n > 1_000_000 {
                        return unchecked(format!("qloop bound {n} exceeds replay cap"));
                    }
                    for _ in 0..n {
                        reference_step(self.work)?;
                        self.exec_stmts(body)?;
                    }
                    Ok(())
                }
            }
            Some("qif") => {
                let selector = head.get(1).ok_or_else(|| {
                    EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§4.6",
                        "object",
                        "qif requires one selector",
                    ))
                })?;
                self.single_wire(selector)?;
                // Execute the published expansion. Both arms' allocations,
                // images and releases happen regardless of the selector.
                let push = vec!["qpred".into(), selector.clone()];
                let pop = vec!["qunpred".into(), selector.clone()];
                let flip = vec![selector.clone()];
                self.exec_line(&push)?;
                self.exec_stmts(body)?;
                self.exec_line(&pop)?;
                // This structural pair restores the selector. It is not a
                // source-body write to an enclosing while selector.
                self.exec_perm_gate("x", &flip)?;
                self.exec_line(&push)?;
                if let Some(else_body) = rest {
                    self.exec_stmts(else_body)?;
                }
                self.exec_line(&pop)?;
                self.exec_perm_gate("x", &flip)
            }
            other => unchecked(format!(
                "structured form {other:?} is outside this checker's action subset"
            )),
        }
    }

    fn exec_line(&mut self, toks: &[String]) -> Result<(), EvalError> {
        if toks.is_empty() {
            return Ok(());
        }
        // Prefix modifiers: ctl <wire>, adj
        if toks[0] == "ctl" {
            let (bundle, idx) = self.single_wire(&toks[1])?;
            self.predicates.push((bundle, idx));
            let r = self.exec_line(&toks[2..]);
            self.predicates.pop();
            return r;
        }
        if toks[0] == "adj" {
            return self.exec_call(&toks[1..], true);
        }
        let (lhs, op, operands): (Option<String>, &str, &[String]) =
            if toks.len() >= 3 && toks[1] == "=" {
                (Some(toks[0].clone()), toks[2].as_str(), &toks[3..])
            } else {
                (None, toks[0].as_str(), &toks[1..])
            };
        self.check_selector_writes(op, lhs.as_deref(), operands)?;
        match op {
            "x" | "cx" | "ccx" | "mcx" | "swap" => self.exec_perm_gate(op, operands),
            "h" | "s" | "sdg" | "t" | "tdg" | "cz" | "rz" | "rx" | "ry" => {
                if self.kind == "perm_action" {
                    Err(EvalError::Reject(Reject::new(
                        "E_CERT_REPLAY",
                        "§5.3",
                        "",
                        format!("permutation action body uses non-permutation gate {op}"),
                    )))
                } else {
                    unchecked(format!("basis replay of non-permutation gate {op} needs statevector evaluation (use numerical/channel claims, §12.1)"))
                }
            }
            "qalloc" => {
                let name =
                    lhs.ok_or_else(|| EvalError::Unchecked("qalloc without result".into()))?;
                let n: usize = operands
                    .get(1)
                    .and_then(|x| x.parse().ok())
                    .ok_or_else(|| EvalError::Unchecked("qalloc count is not a literal".into()))?;
                self.storage_room(n, 0, 0)?;
                self.wires.insert(name.clone(), vec![false; n]);
                self.live_alloc.insert(name);
                Ok(())
            }
            "qfree" => {
                let name = &operands[0];
                let bits = self.wires.remove(name).unwrap_or_default();
                self.live_alloc.remove(name);
                if bits.iter().any(|b| *b) {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_REPLAY",
                        "§12.1",
                        "",
                        format!("qfree of non-clean bundle {name:?} on this basis element; workspace must finish zero"),
                    )));
                }
                Ok(())
            }
            "qborrow" => {
                let name =
                    lhs.ok_or_else(|| EvalError::Unchecked("qborrow without result".into()))?;
                let bits = self.borrowed_queue.pop_front().ok_or_else(|| {
                    EvalError::Unchecked(
                        "body borrows more bundles than the checked borrow requirement supplies"
                            .into(),
                    )
                })?;
                self.borrowed_live.push((name.clone(), bits.clone()));
                self.wires.insert(name, bits);
                Ok(())
            }
            "qreturn" => {
                let name = &operands[0];
                let bits = self.wires.remove(name).unwrap_or_default();
                match self.borrowed_live.iter().position(|(n, _)| n == name) {
                    Some(pos) => {
                        let (_, orig) = self.borrowed_live.remove(pos);
                        if orig != bits {
                            return Err(EvalError::Reject(Reject::new(
                                "E_CERT_REPLAY",
                                "§5.6",
                                "",
                                format!("borrowed bundle {name:?} changed on this basis element"),
                            )));
                        }
                        Ok(())
                    }
                    None => unchecked(format!("qreturn of unborrowed bundle {name:?}")),
                }
            }
            "qcopy" => {
                let dst = lhs.ok_or_else(|| EvalError::Unchecked("qcopy without result".into()))?;
                let src = self.wires.get(&operands[0]).cloned().unwrap_or_default();
                self.storage_room(src.len(), 0, 0)?;
                let on = self.controls_on()?;
                self.wires
                    .insert(dst.clone(), if on { src } else { vec![false; src.len()] });
                self.live_alloc.insert(dst);
                Ok(())
            }
            "quncopy" => {
                let (src_name, dst_name) = (&operands[0], &operands[1]);
                let src = self.wires.get(src_name).cloned().unwrap_or_default();
                let on = self.controls_on()?;
                if on {
                    let dst = self
                        .wires
                        .get_mut(dst_name)
                        .ok_or_else(|| EvalError::Unchecked("quncopy target not live".into()))?;
                    for (i, s) in src.iter().enumerate() {
                        if *s {
                            dst[i] = !dst[i];
                        }
                    }
                }
                let bits = self.wires.remove(dst_name).unwrap_or_default();
                self.live_alloc.remove(dst_name);
                if bits.iter().any(|b| *b) {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_REPLAY",
                        "§12.1",
                        "",
                        format!(
                            "quncopy leaves bundle {dst_name:?} non-clean on this basis element"
                        ),
                    )));
                }
                Ok(())
            }
            "qxor" | "qexch" => {
                let a = self.wires.get(&operands[0]).cloned().unwrap_or_default();
                let on = self.controls_on()?;
                let b = self
                    .wires
                    .get_mut(&operands[1])
                    .ok_or_else(|| EvalError::Unchecked(format!("{op} target not live")))?;
                if a.len() != b.len() {
                    return unchecked(format!("{op} bundle width mismatch"));
                }
                if on {
                    if op == "qxor" {
                        for (i, s) in a.iter().enumerate() {
                            if *s {
                                b[i] = !b[i];
                            }
                        }
                    } else {
                        let tmp = b.clone();
                        *b = a;
                        self.wires.insert(operands[0].clone(), tmp);
                    }
                }
                Ok(())
            }
            "qregion" => {
                // qregion name A W [@image]
                let name = operands[0].clone();
                let a: u32 = operands[1]
                    .parse()
                    .map_err(|_| EvalError::Unchecked("qregion A not literal".into()))?;
                let w: u64 = operands[2]
                    .parse()
                    .map_err(|_| EvalError::Unchecked("qregion W not literal".into()))?;
                reference_width(w)?;
                if a > 20 {
                    return unchecked(format!("local region[2^{a}] exceeds replay cap"));
                }
                let cells = 1usize << a;
                self.storage_room(0, cells, w)?;
                let mut values = vec![BigUint::zero(); cells];
                if let Some(img) = operands.get(3) {
                    let data_name = img.trim_start_matches('@');
                    let hex = self.object.data.get(data_name).cloned().unwrap_or_default();
                    let bytes: Vec<u8> = (0..hex.len() / 2)
                        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0))
                        .collect();
                    for (i, v) in values.iter_mut().enumerate() {
                        let mut cell = BigUint::zero();
                        for bit in 0..w {
                            let byte = bytes
                                .get((i as u64 * w + bit) as usize / 8)
                                .copied()
                                .unwrap_or(0);
                            if (byte >> (((i as u64 * w + bit) % 8) as u32)) & 1 == 1 {
                                cell |= BigUint::one() << bit;
                            }
                        }
                        *v = cell;
                    }
                }
                self.local_region_image.insert(name.clone(), values.clone());
                self.region_width.insert(name.clone(), w);
                self.regions.insert(name, values);
                Ok(())
            }
            "qload" => {
                let dst = lhs.ok_or_else(|| EvalError::Unchecked("qload without result".into()))?;
                let idx = self.address_index(&operands[0], &operands[1])?;
                let w = self.region_width[&operands[0]];
                self.storage_room(w as usize, 0, 0)?;
                let v = if self.controls_on()? {
                    self.regions[&operands[0]][idx].clone()
                } else {
                    BigUint::zero()
                };
                self.wires.insert(dst.clone(), bits_of(&v, w));
                self.live_alloc.insert(dst);
                Ok(())
            }
            "qunload" => {
                // inverse of qload: XOR cell into d under predicate, then free d
                let idx = self.address_index(&operands[0], &operands[1])?;
                let cells = self.regions[&operands[0]][idx].clone();
                let on = self.controls_on()?;
                let dname = &operands[2];
                if on {
                    let d = self
                        .wires
                        .get_mut(dname)
                        .ok_or_else(|| EvalError::Unchecked("qunload target not live".into()))?;
                    let cl = bits_of(&cells, d.len() as u64);
                    for (i, s) in cl.iter().enumerate() {
                        if *s {
                            d[i] = !d[i];
                        }
                    }
                }
                let bits = self.wires.remove(dname).unwrap_or_default();
                self.live_alloc.remove(dname);
                if bits.iter().any(|b| *b) {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_REPLAY",
                        "§12.1",
                        "",
                        "qunload leaves its bundle non-clean on this basis element",
                    )));
                }
                Ok(())
            }
            "qstore" => {
                let v = self.bundle_value(&operands[2])?;
                let on = self.controls_on()?;
                let idx = self.address_index(&operands[0], &operands[1])?;
                if on {
                    let w = self.region_width[&operands[0]];
                    let cells = self.regions.get_mut(&operands[0]).unwrap();
                    cells[idx] = masked(&(&cells[idx] ^ &v), w);
                }
                Ok(())
            }
            "qexchm" => {
                let v = self.bundle_value(&operands[2])?;
                let on = self.controls_on()?;
                let idx = self.address_index(&operands[0], &operands[1])?;
                if on {
                    let w = self.region_width[&operands[0]];
                    let old = self.regions[&operands[0]][idx].clone();
                    self.regions.get_mut(&operands[0]).unwrap()[idx] = masked(&v, w);
                    self.wires.insert(operands[2].clone(), bits_of(&old, w));
                }
                Ok(())
            }
            "qpred" => {
                let (bundle, idx) = self.single_wire(&operands[0])?;
                self.predicates.push((bundle, idx));
                Ok(())
            }
            "qunpred" => {
                let (bundle, idx) = self.single_wire(&operands[0])?;
                match self.predicates.pop() {
                    Some(top) if top == (bundle, idx) => Ok(()),
                    _ => unchecked("qunpred does not match the active predicate stack"),
                }
            }
            "qinplace" => self.exec_qinplace(toks),
            "oracle_call" | "action_call" => self.exec_call(toks, false),
            other => unchecked(format!(
                "action instruction {other:?} is outside this checker's basis subset"
            )),
        }
    }

    fn address_index(&self, region: &str, addr_tok: &str) -> Result<usize, EvalError> {
        let cells = self
            .regions
            .get(region)
            .ok_or_else(|| EvalError::Unchecked(format!("region {region:?} is not live")))?;
        if addr_tok == "()" {
            return Ok(0);
        }
        let (bundle, idx) = wire_ref(addr_tok);
        let value = match idx {
            Some(i) => {
                if self.bit(&bundle, i)? {
                    BigUint::one()
                } else {
                    BigUint::zero()
                }
            }
            None => self.bundle_value(&bundle)?,
        };
        let idx = value.to_string().parse::<usize>().unwrap_or(usize::MAX);
        if idx >= cells.len() {
            return unchecked("region address out of range in replay");
        }
        Ok(idx)
    }

    fn exec_perm_gate(&mut self, op: &str, operands: &[String]) -> Result<(), EvalError> {
        let bad = || {
            EvalError::Reject(Reject::new(
                "E_CERT_OBJECT",
                "§3.3/§4.4 K93",
                "object",
                "invalid permutation-gate arity or repeated/overlapping operands",
            ))
        };
        let arity = match op {
            "x" => 1,
            "ccx" => 3,
            _ => 2,
        };
        if operands.len() != arity {
            return Err(bad());
        }
        let mut controls = Vec::new();
        let target = if op == "mcx" {
            let text = &operands[0];
            let entries = canonical_list(text).ok_or_else(bad)?;
            let mut seen = HashSet::new();
            let mut retained = 0u128;
            for entry in entries {
                let expanded = if entry.contains('[') {
                    vec![self.single_wire(&entry)?]
                } else {
                    let bits = self.wires.get(&entry).ok_or_else(bad)?;
                    if bits.is_empty() {
                        return Err(bad());
                    }
                    retained += bits.len() as u128 * (entry.len() as u128 + 64);
                    if retained > 256 * 1024 * 1024 {
                        return unchecked(
                            "host limit: flattened MCX control identities exceed 256 MiB",
                        );
                    }
                    (0..bits.len()).map(|i| (entry.clone(), i)).collect()
                };
                for wire in expanded {
                    if !seen.insert(wire.clone()) {
                        return Err(bad());
                    }
                    controls.push(wire);
                }
            }
            self.single_wire(&operands[1])?
        } else {
            if matches!(op, "cx" | "ccx") {
                for source in &operands[..operands.len() - 1] {
                    controls.push(self.single_wire(source)?);
                }
            }
            self.single_wire(operands.last().unwrap())?
        };
        let other = if op == "swap" {
            Some(self.single_wire(&operands[0])?)
        } else {
            None
        };
        let mut all = HashSet::new();
        for wire in controls
            .iter()
            .chain(std::iter::once(&target))
            .chain(other.iter())
        {
            if !all.insert(wire) || self.predicates.contains(wire) {
                return Err(bad());
            }
        }
        let pred_on = self.controls_on()?;
        if let Some((b, i)) = other {
            if pred_on {
                let a = self.bit(&b, i)?;
                let v = self.bit(&target.0, target.1)?;
                self.flip_if(&b, i, a != v)?;
                self.flip_if(&target.0, target.1, a != v)?;
            }
            return Ok(());
        }
        let mut on = pred_on;
        for (bundle, index) in controls {
            on &= self.bit(&bundle, index)?;
        }
        self.flip_if(&target.0, target.1, on)
    }

    fn exec_qinplace(&mut self, toks: &[String]) -> Result<(), EvalError> {
        // qinplace @g @ginv tk x [(ins..) (mem..) (args..)]
        let g_name = toks[1].trim_start_matches('@').to_string();
        let tk_name = toks[3].clone();
        let (x_bundle, _) = wire_ref(&toks[4]);
        let ins_toks = toks.get(5).map(|x| list_items(x)).unwrap_or_default();
        let mem_toks = toks.get(6).map(|x| list_items(x)).unwrap_or_default();
        let arg_toks = toks.get(7).map(|x| list_items(x)).unwrap_or_default();
        let g = self.object.symbol(&g_name).cloned().ok_or_else(|| {
            EvalError::Unchecked(format!("qinplace oracle @{g_name} is not defined"))
        })?;
        let budget = match self.args.get(&tk_name) {
            Some(Classical::Token(t)) => t.clone(),
            _ => return unchecked(format!("qinplace token {tk_name:?} has no bound tuple")),
        };
        let x_val = self.bundle_value(&x_bundle)?;
        let mut ins = vec![x_val];
        for t in &ins_toks {
            ins.push(self.bundle_value(t)?);
        }
        let mem: Vec<Vec<BigUint>> = mem_toks
            .iter()
            .map(|n| self.regions.get(n).cloned().unwrap_or_default())
            .collect();
        let mut cargs = Vec::new();
        for t in &arg_toks {
            match self.args.get(t) {
                Some(v) => cargs.push(v.clone()),
                None => {
                    return unchecked(format!(
                        "qinplace argument {t:?} is not a classical argument here"
                    ))
                }
            }
        }
        let outcome = eval_oracle_work(
            self.object,
            &g,
            &ins,
            &mem,
            &cargs,
            &budget,
            self.depth + 1,
            self.work,
        )?;
        if !outcome.code.is_zero() {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§4.5",
                "",
                format!("qinplace oracle @{g_name} faults (code {}) on this basis element; its inverse-pair obligation requires fault-free evaluation", outcome.code),
            )));
        }
        let inverse_name = toks[2].trim_start_matches('@');
        let inverse = self.object.symbol(inverse_name).ok_or_else(|| {
            EvalError::Unchecked(format!("qinplace inverse @{inverse_name} is not linked"))
        })?;
        if inverse.kind != "oracle" || inverse.signature != g.signature {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§4.5",
                "",
                "qinplace inverse has an incompatible signature",
            )));
        }
        let mut inverse_ins = ins.clone();
        inverse_ins[0] = outcome.payload.clone();
        let recovered = eval_oracle_work(
            self.object,
            inverse,
            &inverse_ins,
            &mem,
            &cargs,
            &budget,
            self.depth + 1,
            self.work,
        )?;
        if !recovered.code.is_zero() || recovered.payload != ins[0] {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§4.5",
                "",
                "qinplace inverse does not clean the temporary target/fault at this basis element",
            )));
        }
        // Under control the whole qinplace is controlled (§4.5): on a basis
        // element with controls off it is the identity.
        if self.controls_on()? {
            let width = self
                .wires
                .get(&x_bundle)
                .map(|w| w.len() as u64)
                .unwrap_or(0);
            self.wires
                .insert(x_bundle, bits_of(&outcome.payload, width));
        }
        Ok(())
    }

    fn exec_call(&mut self, toks: &[String], adjoint: bool) -> Result<(), EvalError> {
        if let Some(op) = toks.first() {
            self.check_selector_writes(op, None, &toks[1..])?;
        }
        let op = toks.first().map(String::as_str).unwrap_or("");
        match op {
            "oracle_call" => {
                // oracle_call @o tk (ins..) tgt fr (mem..) (args..)
                let name = toks[1].trim_start_matches('@').to_string();
                let tk = toks[2].clone();
                let ins_toks = list_items(&toks[3]);
                let (tgt, _) = wire_ref(&toks[4]);
                let (fr, _) = wire_ref(&toks[5]);
                let mem_toks = toks.get(6).map(|x| list_items(x)).unwrap_or_default();
                let arg_toks = toks.get(7).map(|x| list_items(x)).unwrap_or_default();
                let budget = match self.args.get(&tk) {
                    Some(Classical::Token(t)) => t.clone(),
                    _ => return unchecked(format!("oracle_call token {tk:?} has no bound tuple")),
                };
                let callee = self.object.symbol(&name).cloned().ok_or_else(|| {
                    EvalError::Unchecked(format!("oracle @{name} is not defined"))
                })?;
                let ins: Vec<BigUint> = ins_toks
                    .iter()
                    .map(|t| self.bundle_value(t))
                    .collect::<Result<_, _>>()?;
                let mem: Vec<Vec<BigUint>> = mem_toks
                    .iter()
                    .map(|n| self.regions.get(n).cloned().unwrap_or_default())
                    .collect();
                let mut cargs = Vec::new();
                for t in &arg_toks {
                    match self.args.get(t) {
                        Some(v) => cargs.push(v.clone()),
                        None => {
                            return unchecked(format!(
                                "call argument {t:?} is not a classical argument here"
                            ))
                        }
                    }
                }
                let outcome = eval_oracle_work(
                    self.object,
                    &callee,
                    &ins,
                    &mem,
                    &cargs,
                    &budget,
                    self.depth + 1,
                    self.work,
                )?;
                if self.controls_on()? {
                    let tw = self.wires[&tgt].len() as u64;
                    let cur = self.bundle_value(&tgt)?;
                    self.wires.insert(
                        tgt.clone(),
                        bits_of(&masked(&(&cur ^ &outcome.payload), tw), tw),
                    );
                    let fw = self.wires[&fr].len() as u64;
                    let cur = self.bundle_value(&fr)?;
                    self.wires.insert(
                        fr.clone(),
                        bits_of(&masked(&(&cur ^ &outcome.code), fw), fw),
                    );
                }
                Ok(())
            }
            "action_call" => {
                // action_call @a (mut..) (ins..) (mem..) (args..)
                let name = toks[1].trim_start_matches('@').to_string();
                let mut_toks = list_items(&toks[2]);
                let in_toks = list_items(&toks[3]);
                let mem_toks = toks.get(4).map(|x| list_items(x)).unwrap_or_default();
                let arg_toks = toks.get(5).map(|x| list_items(x)).unwrap_or_default();
                let callee = self.object.symbol(&name).cloned().ok_or_else(|| {
                    EvalError::Unchecked(format!("action @{name} is not defined"))
                })?;
                let iface = self.object.interface(&name).map_err(EvalError::Unchecked)?;
                if !matches!(callee.kind.as_str(), "action" | "perm_action")
                    || mut_toks.len() != iface.bundles.iter().filter(|b| b.group == "mut").count()
                    || in_toks.len() != iface.bundles.iter().filter(|b| b.group == "in").count()
                    || mem_toks.len() != iface.regions.len()
                    || arg_toks.len() != iface.args.len()
                {
                    return Err(EvalError::Reject(Reject::new(
                        "E_CERT_OBJECT",
                        "§3.3/§5.3",
                        "object",
                        "action_call operands do not match the complete callee interface",
                    )));
                }
                let bad_actual = || {
                    EvalError::Reject(Reject::new("E_CERT_OBJECT","§3.3/§5.3","object","action actual widths/shapes disagree, or actuals overlap each other or effective controls"))
                };
                let mut occupied = self
                    .predicates
                    .iter()
                    .map(|(name, index)| (name.as_str(), *index))
                    .collect::<HashSet<_>>();
                for (actual, formal) in mut_toks.iter().chain(&in_toks).zip(&iface.bundles) {
                    let width = self.actual_width(actual)?;
                    if width as u64 != formal.width {
                        return Err(bad_actual());
                    }
                    let base = actual.split('[').next().unwrap();
                    let first = if actual.contains('[') {
                        self.single_wire(actual)?.1
                    } else {
                        0
                    };
                    for index in first..first + width {
                        if !occupied.insert((base, index)) {
                            return Err(bad_actual());
                        }
                    }
                }
                drop(occupied);
                let mut region_names = HashSet::new();
                for (actual, formal) in mem_toks.iter().zip(&iface.regions) {
                    let Some(cells) = self.regions.get(actual) else {
                        return unchecked("action memory actual needs mapped-region/local-entry lifetime resolution");
                    };
                    if !region_names.insert(actual)
                        || self.region_width.get(actual) != Some(&formal.w)
                        || !cells.len().is_power_of_two()
                        || cells.len().trailing_zeros() as u64 != formal.a
                    {
                        return Err(bad_actual());
                    }
                }
                let read_only = mut_toks
                    .iter()
                    .chain(&in_toks)
                    .zip(&iface.bundles)
                    .filter(|(actual, _)| self.selector_guards.contains(&wire_ref(actual).0))
                    .map(|(_, formal)| formal.name.clone())
                    .collect::<Vec<_>>();
                if !iface.borrowed_widths.is_empty() {
                    return unchecked(format!(
                        "callee @{name} borrows bundles; nested borrow frames need §5.6 frame verification, not single-value replay"
                    ));
                }
                // Inherited controls of the callee invocation are the current
                // predicate bits, prepended as control bundles.
                let mut bundles: Vec<BigUint> = Vec::new();
                let mut seen_controls = HashSet::new();
                for (b, i) in self.predicates.clone() {
                    if !seen_controls.insert((b.clone(), i)) {
                        continue;
                    }
                    bundles.push(if self.bit(&b, i)? {
                        BigUint::one()
                    } else {
                        BigUint::zero()
                    });
                }
                let nctl = bundles.len();
                for t in mut_toks.iter().chain(in_toks.iter()) {
                    bundles.push(self.actual_value(t)?);
                }
                let regs: Vec<Vec<BigUint>> = mem_toks
                    .iter()
                    .map(|n| self.regions.get(n).cloned().unwrap_or_default())
                    .collect();
                let mut cargs = Vec::new();
                for t in &arg_toks {
                    match self.args.get(t) {
                        Some(v) => cargs.push(v.clone()),
                        None => {
                            return unchecked(format!(
                                "call argument {t:?} is not a classical argument here"
                            ))
                        }
                    }
                }
                let before = Tuple {
                    bundles,
                    borrowed: vec![],
                    regions: regs.clone(),
                    args: vec![],
                };
                let after = eval_action_tuple_guarded(
                    self.object,
                    &callee,
                    &iface,
                    &before,
                    &cargs,
                    nctl,
                    adjoint,
                    self.depth + 1,
                    self.work,
                    &read_only,
                )?;
                let mut pos = nctl;
                for t in &mut_toks {
                    self.deliver_actual(t, &after.bundles[pos])?;
                    pos += 1;
                }
                // in actuals unchanged by construction of the interface tuple.
                for (n, cells) in mem_toks.iter().zip(after.regions.into_iter()) {
                    self.regions.insert(n.clone(), cells);
                }
                Ok(())
            }
            other => unchecked(format!("call form {other:?} is outside this subset")),
        }
    }
}

/// Adjoint of an action statement sequence (section 5.3 table), subset.
fn adjoint_stmts(stmts: &[Stmt]) -> Result<Vec<Stmt>, EvalError> {
    let mut out = Vec::new();
    for s in stmts.iter().rev() {
        match s {
            Stmt::Line(t) => out.push(Stmt::Line(adjoint_line(t)?)),
            Stmt::Block { head, body, rest } => match head.first().map(String::as_str) {
                Some("qloop") => {
                    if head.get(2).map(String::as_str) == Some("while") {
                        return unchecked("adjoint of qloop-while is outside this subset");
                    }
                    out.push(Stmt::Block {
                        head: head.clone(),
                        body: adjoint_stmts(body)?,
                        rest: None,
                    });
                }
                Some("qif") => {
                    out.push(Stmt::Block {
                        head: head.clone(),
                        body: adjoint_stmts(body)?,
                        rest: match rest {
                            Some(r) => Some(adjoint_stmts(r)?),
                            None => None,
                        },
                    });
                }
                other => return unchecked(format!("adjoint of {other:?} is outside this subset")),
            },
        }
    }
    Ok(out)
}

fn adjoint_line(toks: &[String]) -> Result<Vec<String>, EvalError> {
    if toks.is_empty() {
        return Ok(toks.to_vec());
    }
    if toks[0] == "ctl" {
        let mut out = vec![toks[0].clone(), toks[1].clone()];
        out.extend(adjoint_line(&toks[2..])?);
        return Ok(out);
    }
    if toks[0] == "adj" {
        return Ok(toks[1..].to_vec());
    }
    let (lhs, op, operands): (Option<String>, &str, &[String]) =
        if toks.len() >= 3 && toks[1] == "=" {
            (Some(toks[0].clone()), toks[2].as_str(), &toks[3..])
        } else {
            (None, toks[0].as_str(), &toks[1..])
        };
    Ok(match op {
        "qalloc" => vec!["qfree".into(), lhs.unwrap()],
        "qfree" => {
            return unchecked(
                "adjoint of a bare qfree needs its allocation site (use adj on whole bodies)",
            )
        }
        "qborrow" => vec!["qreturn".into(), lhs.unwrap()],
        "qreturn" => return unchecked("adjoint of a bare qreturn needs its borrow site"),
        "qcopy" => vec!["quncopy".into(), operands[0].clone(), lhs.unwrap()],
        "quncopy" => vec![
            operands[1].clone(),
            "=".into(),
            "qcopy".into(),
            operands[0].clone(),
        ],
        "qload" => vec![
            "qunload".into(),
            operands[0].clone(),
            operands[1].clone(),
            lhs.unwrap(),
        ],
        "qunload" => vec![
            operands[2].clone(),
            "=".into(),
            "qload".into(),
            operands[0].clone(),
            operands[1].clone(),
        ],
        "action_call" => {
            let mut out = vec!["adj".into()];
            out.extend(toks.iter().cloned());
            out
        }
        "qpred" => vec!["qunpred".into(), operands[0].clone()],
        "qunpred" => vec!["qpred".into(), operands[0].clone()],
        "qinplace" => {
            let mut out = toks.to_vec();
            out.swap(1, 2);
            out
        }
        // Self-adjoint in the supported subset.
        "x" | "cx" | "ccx" | "mcx" | "swap" | "qxor" | "qexch" | "qstore" | "qexchm"
        | "oracle_call" | "qregion" => toks.to_vec(),
        other => return unchecked(format!("adjoint of {other:?} is outside this subset")),
    })
}

/// Replay an action/perm_action on one full before-tuple (controls first).
pub fn eval_action_tuple(
    object: &Object,
    symbol: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    args: &[Classical],
    inherited_controls: usize,
    adjoint: bool,
    depth: usize,
) -> Result<Tuple, EvalError> {
    let mut work = crate::cert::Limits::default().max_reference_steps;
    eval_action_tuple_work(
        object,
        symbol,
        iface,
        before,
        args,
        inherited_controls,
        adjoint,
        depth,
        &mut work,
    )
}
fn eval_action_tuple_work(
    object: &Object,
    symbol: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    args: &[Classical],
    inherited_controls: usize,
    adjoint: bool,
    depth: usize,
    work: &mut u64,
) -> Result<Tuple, EvalError> {
    eval_action_tuple_guarded(
        object,
        symbol,
        iface,
        before,
        args,
        inherited_controls,
        adjoint,
        depth,
        work,
        &[],
    )
}
fn eval_action_tuple_guarded(
    object: &Object,
    symbol: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    args: &[Classical],
    inherited_controls: usize,
    adjoint: bool,
    depth: usize,
    work: &mut u64,
    read_only: &[String],
) -> Result<Tuple, EvalError> {
    if depth > 64 {
        return unchecked("action call depth exceeds this checker's replay cap (64)");
    }
    let mut initial_bits = inherited_controls as u128;
    for width in iface
        .bundles
        .iter()
        .map(|b| b.width)
        .chain(iface.borrowed_widths.iter().copied())
    {
        reference_width(width)?;
        initial_bits += u128::from(width);
    }
    for (region, cells) in iface.regions.iter().zip(&before.regions) {
        reference_width(region.w)?;
        initial_bits += u128::from(region.w) * cells.len() as u128;
    }
    if initial_bits >= 1_000_000 {
        return unchecked(
            "host limit: reference interface storage exceeds cap 999999 bits (§12.2)",
        );
    }
    if symbol.kind != "perm_action" && crate::coherent::needed(object, symbol) {
        if !read_only.is_empty() {
            return unchecked(
                "selector-write analysis of a coherent callee requires source-lifetime expansion",
            );
        }
        return crate::coherent::eval(
            object,
            symbol,
            iface,
            before,
            args,
            inherited_controls,
            adjoint,
            work,
        );
    }
    let func = object
        .function(&symbol.name)
        .ok_or_else(|| EvalError::Unchecked(format!("action @{} has no body", symbol.name)))?;
    let mut env = ActionEnv {
        object,
        wires: HashMap::new(),
        regions: HashMap::new(),
        region_width: HashMap::new(),
        local_region_image: HashMap::new(),
        args: HashMap::new(),
        predicates: Vec::new(),
        loop_serial: 0,
        selector_guards: read_only.to_vec(),
        borrowed_queue: before
            .borrowed
            .iter()
            .zip(iface.borrowed_widths.iter())
            .map(|(v, w)| bits_of(v, *w))
            .collect(),
        borrowed_live: Vec::new(),
        live_alloc: HashSet::new(),
        depth,
        kind: symbol.kind.clone(),
        work,
    };
    for i in 0..inherited_controls {
        let name = format!("$ctl{i}");
        env.wires
            .insert(name.clone(), vec![before.bundles[i] == BigUint::one()]);
        env.predicates.push((name, 0));
    }
    for (b, v) in iface
        .bundles
        .iter()
        .zip(before.bundles[inherited_controls..].iter())
    {
        env.wires.insert(b.name.clone(), bits_of(v, b.width));
    }
    for (r, cells) in iface.regions.iter().zip(before.regions.iter()) {
        env.region_width.insert(r.name.clone(), r.w);
        env.regions.insert(r.name.clone(), cells.clone());
    }
    for (spec, value) in func.groups[3].iter().zip(args.iter()) {
        env.args.insert(spec.name.clone(), value.clone());
    }
    // Also map args by the signature order for lifted call lookup parity.
    let mut blocks = parse_blocks(func);
    if adjoint {
        blocks.reverse();
    }
    for (_, stmts) in blocks {
        let stmts = if adjoint {
            adjoint_stmts(&stmts)?
        } else {
            stmts
        };
        env.exec_stmts(&stmts)?;
    }
    if !env.live_alloc.is_empty() {
        return Err(EvalError::Reject(Reject::new(
            "E_CERT_REPLAY",
            "§5.1",
            "",
            format!(
                "action @{} leaves allocations live at exit on this basis element",
                symbol.name
            ),
        )));
    }
    if !env.borrowed_live.is_empty() {
        return Err(EvalError::Reject(Reject::new(
            "E_CERT_REPLAY",
            "§5.1",
            "",
            "borrowed bundles not returned at exit",
        )));
    }
    for (name, image) in &env.local_region_image {
        if env.regions.get(name) != Some(image) {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§5.1",
                "",
                format!("local region {name:?} not restored to its image at exit"),
            )));
        }
    }
    let mut bundles: Vec<BigUint> = before.bundles[..inherited_controls].to_vec();
    for b in &iface.bundles {
        bundles.push(env.bundle_value(&b.name)?);
    }
    let regions: Vec<Vec<BigUint>> = iface
        .regions
        .iter()
        .map(|r| env.regions.get(&r.name).cloned().unwrap_or_default())
        .collect();
    for (i, bundle) in iface.bundles.iter().enumerate() {
        if bundle.group == "in"
            && bundles[inherited_controls + i] != before.bundles[inherited_controls + i]
        {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§12.1",
                "",
                format!(
                    "input bundle {:?} was not preserved by @{}",
                    bundle.name, symbol.name
                ),
            )));
        }
    }
    for (i, region) in iface.regions.iter().enumerate() {
        if region.role == "in" && regions[i] != before.regions[i] {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§12.1",
                "",
                format!(
                    "input region {:?} was not preserved by @{}",
                    region.name, symbol.name
                ),
            )));
        }
    }
    Ok(Tuple {
        bundles,
        borrowed: before.borrowed.clone(),
        regions,
        args: before.args.clone(),
    })
}

/// Top-level subject evaluation of one element: oracle via F_B contract,
/// action via basis replay.
pub fn eval_subject_tuple(
    object: &Object,
    symbol: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    invocation: &Invocation,
    inherited_controls: usize,
) -> Result<Tuple, EvalError> {
    eval_subject_tuple_bounded(
        object,
        symbol,
        iface,
        before,
        invocation,
        inherited_controls,
        crate::cert::Limits::default().max_reference_steps,
    )
}
fn eval_subject_tuple_bounded(
    object: &Object,
    symbol: &Symbol,
    iface: &crate::object::Interface,
    before: &Tuple,
    invocation: &Invocation,
    inherited_controls: usize,
    row_limit: u64,
) -> Result<Tuple, EvalError> {
    let mut work = row_limit;
    let args = decode_invocation_args(&invocation.args, &iface.args)?;
    let after = match symbol.kind.as_str() {
        "oracle" => eval_oracle_tuple_work(
            object,
            symbol,
            iface,
            before,
            &args,
            &invocation.budget,
            inherited_controls,
            &mut work,
        ),
        "action" | "perm_action" => eval_action_tuple_work(
            object,
            symbol,
            iface,
            before,
            &args,
            inherited_controls,
            false,
            0,
            &mut work,
        ),
        other => unchecked(format!(
            "subject kind {other} cannot be replayed as a region"
        )),
    }?;
    for (i, bundle) in iface.bundles.iter().enumerate() {
        if bundle.group == "in"
            && after.bundles[inherited_controls + i] != before.bundles[inherited_controls + i]
        {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§12.1",
                "",
                format!("input bundle {:?} was not preserved", bundle.name),
            )));
        }
    }
    for (i, region) in iface.regions.iter().enumerate() {
        if region.role == "in" && after.regions[i] != before.regions[i] {
            return Err(EvalError::Reject(Reject::new(
                "E_CERT_REPLAY",
                "§12.1",
                "",
                format!("input region {:?} was not preserved", region.name),
            )));
        }
    }
    if after.borrowed != before.borrowed
        || after.bundles[..inherited_controls] != before.bundles[..inherited_controls]
    {
        return Err(EvalError::Reject(Reject::new(
            "E_CERT_REPLAY",
            "§12.1",
            "",
            "borrowed systems or inherited controls were not preserved",
        )));
    }
    Ok(after)
}

// ---------- replay claim shell ----------

use crate::cert::{claim_subject, host_limit_outcome, parse_invocation, rej, schema_err, u64_of};
use crate::domain::{resolve_domain, ComponentId, ResolvedDomain};
use crate::fault::ResolveError;
use crate::json::Value;
use crate::object::Interface;
use crate::rat::{fraction_string, is_canonical_uint_str};

fn fmt_tuple(t: &Tuple) -> String {
    format!(
        "{{bundles:[{}],borrowed:[{}],regions:[{}]}}",
        t.bundles
            .iter()
            .map(BigUint::to_string)
            .collect::<Vec<_>>()
            .join(","),
        t.borrowed
            .iter()
            .map(BigUint::to_string)
            .collect::<Vec<_>>()
            .join(","),
        t.regions
            .iter()
            .map(|r| format!(
                "[{}]",
                r.iter()
                    .map(BigUint::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn parse_tuple(v: &Value, d: &ResolvedDomain, path: &str) -> Result<Tuple, Reject> {
    let bad = |msg: String| Reject::new("E_CERT_TABLE", "§12.1", path.to_string(), msg);
    let vals = |arr: &[Value], widths: &[u64], what: &str| -> Result<Vec<BigUint>, Reject> {
        if arr.len() != widths.len() {
            return Err(bad(format!(
                "{what} has {} entries; the interface has {}",
                arr.len(),
                widths.len()
            )));
        }
        let mut out = Vec::new();
        for (x, w) in arr.iter().zip(widths.iter()) {
            match x
                .as_str()
                .filter(|t| is_canonical_uint_str(t))
                .and_then(|t| t.parse::<BigUint>().ok())
            {
                Some(v) if v.bits() <= *w => out.push(v),
                _ => {
                    return Err(bad(format!(
                        "{what} value is not a canonical decimal string within {w} bits"
                    )))
                }
            }
        }
        Ok(out)
    };
    let obj = v
        .as_obj()
        .ok_or_else(|| bad("tuple is not an object".into()))?;
    for field in ["bundles", "borrowed", "regions"] {
        if v.get(field).and_then(Value::as_arr).is_none() {
            return Err(bad(format!("tuple lacks required {field} array")));
        }
    }
    for k in obj.keys() {
        if !matches!(k.as_str(), "bundles" | "borrowed" | "regions") {
            return Err(Reject::new(
                "E_CERT_SCHEMA",
                "§12.1",
                path.to_string(),
                format!("tuple has unknown field {k:?}"),
            ));
        }
    }
    let bundles = vals(
        v.get("bundles").and_then(Value::as_arr).unwrap_or(&[]),
        &d.bundle_widths,
        "bundles",
    )?;
    if v.get("bundles").and_then(Value::as_arr).is_none() {
        return Err(bad("tuple lacks bundles array".into()));
    }
    let borrowed = vals(
        v.get("borrowed").and_then(Value::as_arr).unwrap_or(&[]),
        &d.borrowed_widths,
        "borrowed",
    )?;
    let region_arrs = v
        .get("regions")
        .and_then(Value::as_arr)
        .map(|a| a.to_vec())
        .unwrap_or_default();
    if region_arrs.len() != d.region_shapes.len() {
        return Err(bad(format!(
            "regions has {} entries; the interface has {}",
            region_arrs.len(),
            d.region_shapes.len()
        )));
    }
    let mut regions = Vec::new();
    for (cells, (count, w)) in region_arrs.iter().zip(d.region_shapes.iter()) {
        let arr = cells
            .as_arr()
            .ok_or_else(|| bad("region cells are not an array".into()))?;
        if BigUint::from(arr.len()) != *count {
            return Err(bad(
                "region cell count does not equal 2^A; no cells may be omitted".into(),
            ));
        }
        regions.push(vals(arr, &vec![*w; arr.len()], "region cells")?);
    }
    Ok(Tuple {
        bundles,
        borrowed,
        regions,
        args: vec![],
    })
}

fn outcome_from_eval<T>(r: Result<T, EvalError>, path: &str) -> Result<T, ClaimOutcome> {
    match r {
        Ok(v) => Ok(v),
        Err(EvalError::Reject(mut e)) => {
            if e.path.is_empty() {
                e.path = path.to_string();
            }
            Err(ClaimOutcome::rejected(vec![e]))
        }
        Err(EvalError::Unchecked(reason)) => Err(ClaimOutcome::unchecked(reason)),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn check_replay_claim(
    claim: &Value,
    claim_index: usize,
    path: &str,
    region_hash: &str,
    _region_symbol: Option<&Symbol>,
    region: &Value,
    docs: &LoadedDocs,
    root_invocation: &Option<Invocation>,
    cert_schedule_hash: Option<&str>,
    inputs: &Inputs,
) -> ClaimOutcome {
    match check_replay_inner(
        claim,
        claim_index,
        path,
        region_hash,
        region,
        docs,
        root_invocation,
        cert_schedule_hash,
        inputs,
    ) {
        Ok(o) => o,
        Err(o) => o,
    }
}

#[allow(clippy::too_many_arguments)]
fn check_replay_inner(
    claim: &Value,
    claim_index: usize,
    path: &str,
    region_hash: &str,
    region: &Value,
    docs: &LoadedDocs,
    root_invocation: &Option<Invocation>,
    cert_schedule_hash: Option<&str>,
    inputs: &Inputs,
) -> Result<ClaimOutcome, ClaimOutcome> {
    let kind = claim.get("kind").and_then(Value::as_str).unwrap_or("");
    // Strict claim shape.
    {
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
    }
    let subject = claim_subject(claim, path, region_hash, false)?;
    let symbol = match docs.object.symbol_by_closure(&subject.closure_hash) {
        Some(s) => s.clone(),
        None if docs.object.imports.is_empty() => {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_BINDING",
                "§12.1",
                format!("{path}.subject.closure_hash"),
                "subject closure is absent from the complete supplied object (no imports)",
            )]))
        }
        None => {
            return Ok(ClaimOutcome::unchecked(
                "subject region is not in the supplied object",
            ))
        }
    };
    let iface = docs.object.interface(&symbol.name).map_err(|e| {
        ClaimOutcome::unchecked(format!("subject interface cannot be derived: {e}"))
    })?;

    // Expected relation.
    #[derive(PartialEq)]
    enum Expected {
        Oracle,
        InversePair,
        Table(Vec<Value>),
        Program(String),
    }
    let expected = match claim.get("expected") {
        Some(Value::Str(x)) if x == "oracle" => Expected::Oracle,
        Some(Value::Str(x)) if x == "inverse_pair" => Expected::InversePair,
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
                "expected must be \"oracle\", {table}, {program}, or (exhaustive) \"inverse_pair\"; there is no default relation",
            )]))
        }
    };
    let requested_label = match claim.get("label") {
        None => None,
        Some(Value::Str(x)) if x == "proof" || x == "evidence" => Some(x.clone()),
        Some(_) => {
            return Err(ClaimOutcome::rejected(vec![schema_err(
                format!("{path}.label"),
                "label must be proof or evidence",
            )]))
        }
    };
    if expected == Expected::Oracle && symbol.kind != "oracle" {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.expected"),
            "expected oracle requires an oracle subject and its §5.2 map",
        )]));
    }
    if expected == Expected::InversePair {
        if kind != "exhaustive" {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_BINDING",
                "§12.1",
                format!("{path}.expected"),
                "inverse_pair is not a sampled proof option",
            )]));
        }
        return check_inverse_pair(
            claim,
            claim_index,
            path,
            region,
            &subject,
            docs,
            inputs,
            requested_label,
        );
    }
    if subject.closure_hash != region_hash {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.3",
            format!("{path}.subject.closure_hash"),
            "a region record's claims are about that region; subject names a different closure",
        )]));
    }
    if expected == Expected::Oracle && subject.schedule_hash.is_none() {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.subject.schedule_hash"),
            "expected \"oracle\" compares the realised operation and requires schedule_hash",
        )]));
    }
    if claim.get("domain").and_then(|d| d.get("args")).is_some() {
        return Err(ClaimOutcome::rejected(vec![rej("E_CERT_DOMAIN", "§12.2", format!("{path}.domain.args"), "args domains are reserved for inverse-pair claims; this relation binds its arguments through invocation")]));
    }
    // Domain.
    let domain = match resolve_domain(
        claim.get("domain").unwrap_or(&Value::Null),
        &iface,
        subject.inherited_controls,
    ) {
        Ok(d) => d,
        Err(ResolveError::Reject(r)) => {
            return Err(ClaimOutcome::rejected(
                vec![r.at(&format!("{path}.domain"))],
            ))
        }
        Err(ResolveError::HostLimit(h)) => return Ok(host_limit_outcome(h)),
    };
    // Table correspondence.
    let table_rows: Option<Vec<(Tuple, Tuple)>> = match &expected {
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
                return Ok(host_limit_outcome(limit));
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
    };
    // Invocation.
    let scheduled = subject.schedule_hash.is_some();
    if scheduled && subject.schedule_hash.as_deref() != cert_schedule_hash {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.3",
            format!("{path}.subject.schedule_hash"),
            "subject schedule_hash does not equal the certificate's schedule binding",
        )]));
    }
    let invocation: Invocation = match claim.get("invocation") {
        Some(inv) => parse_invocation(inv, &iface.args, &docs.object, &format!("{path}.invocation"))
            .map_err(ClaimOutcome::rejected)?,
        None if scheduled && region_hash == docs.root_closure => match root_invocation {
            Some(inv) => inv.clone(),
            None => return Err(ClaimOutcome::rejected(vec![rej("E_CERT_BINDING", "§12.3", format!("{path}.invocation"), "no root invocation to replay under")])),
        },
        None if scheduled => {
            return Ok(ClaimOutcome::unchecked(
                "nested scheduled invocation: this checker does not yet resolve per-context arguments/budgets from the invocation tree",
            ))
        }
        None => {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_INVOCATION",
                "§12.3",
                format!("{path}.invocation"),
                "standalone claims must carry their own invocation (no implicit zero/default)",
            )]))
        }
    };
    if scheduled && region_hash == docs.root_closure {
        if let Some(root) = root_invocation {
            if invocation.args != root.args || invocation.budget != root.budget {
                return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    format!("{path}.invocation"),
                    "explicit root claim invocation conflicts with bindings.invocation",
                )]));
            }
        }
        if let Some(sb) = docs.schedule.as_ref().and_then(|s| s.get("bindings")) {
            if sb.get("variant").and_then(Value::as_str) != Some(subject.variant.as_str())
                || sb.get("inherited_controls").and_then(u64_of) != Some(subject.inherited_controls)
            {
                return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING",
                    "§12.1",
                    format!("{path}.subject"),
                    "root subject variant/controls do not match the scheduled root invocation",
                )]));
            }
        }
    } else if scheduled {
        return Ok(ClaimOutcome::unchecked("nested scheduled invocation: explicit arguments/budget cannot be trusted until every invocation context has been independently resolved (§12.3)"));
    }
    // Expected program compatibility (K73: positional, never by name).
    let program: Option<(Symbol, Interface)> = match &expected {
        Expected::Program(h) => {
            let sym = match docs.object.symbol_by_closure(h) {
                Some(s) => s.clone(),
                None if docs.object.imports.is_empty() => return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING", "§12.1", format!("{path}.expected.program"),
                    "expected program closure is absent from the complete supplied object (no imports)",
                )])),
                None => {
                    return Ok(ClaimOutcome::unchecked(
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
    };

    if expected == Expected::Oracle {
        return Ok(ClaimOutcome::unchecked(
            "expected \"oracle\" compares the realised (§5.2-constructed) operation with F_B; re-deriving that construction requires the §11.1 expansion/step-machine simulation this checker does not yet perform",
        ));
    }
    // Elements to replay.
    let (elements, draws_count, distinct_count): (Vec<Tuple>, Option<u64>, Option<usize>) = if kind
        == "exhaustive"
    {
        if domain.n > BigUint::from(inputs.limits.max_elements) {
            return Ok(host_limit_outcome(crate::fault::HostLimit {
                quantity: format!(
                    "domain element count {} (cap {})",
                    domain.n, inputs.limits.max_elements
                ),
            }));
        }
        if let Err(h) = domain.check_retained_elements(&domain.n) {
            return Ok(host_limit_outcome(h));
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
            return Ok(host_limit_outcome(crate::fault::HostLimit {
                quantity: format!("sampler count {count} (cap {})", inputs.limits.max_elements),
            }));
        }
        if let Err(h) = domain.check_retained_elements(&count) {
            return Ok(host_limit_outcome(h));
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
            Err(ResolveError::HostLimit(h)) => return Ok(host_limit_outcome(h)),
        };
        let distinct: std::collections::HashSet<String> = draws.iter().map(fmt_tuple).collect();
        (draws, Some(count), Some(distinct.len()))
    };

    // Scheduled replay compares the realised operation; require an exact
    // realisation so reference replay is the same operation.
    if scheduled {
        if docs.schedule.is_none() {
            return Ok(ClaimOutcome::unchecked(
                "scheduled claim but no schedule document supplied",
            ));
        }
        if docs.expansion_unknown {
            return Ok(ClaimOutcome::unchecked(
                "schedule binds an expansion library this checker does not implement",
            ));
        }
        if let Err(outcome) = crate::realisation::exact_native_subject(&subject, docs, inputs) {
            return Ok(outcome);
        }
    }
    // Replay.
    let c = subject.inherited_controls as usize;
    let mut replayed = 0usize;
    for (i, before) in elements.iter().enumerate() {
        let left = outcome_from_eval(
            eval_subject_tuple_bounded(
                &docs.object,
                &symbol,
                &iface,
                before,
                &invocation,
                c,
                inputs.limits.max_reference_steps,
            ),
            path,
        )?;
        let right: Tuple = match &expected {
            Expected::Table(_) => {
                let rows = table_rows.as_ref().unwrap();
                let index = if kind == "exhaustive" {
                    Some(i)
                } else {
                    domain.index_of(before).to_usize()
                };
                match index.and_then(|i| rows.get(i)) {
                    Some((_, after)) => after.clone(),
                    None => {
                        return Err(ClaimOutcome::rejected(vec![rej(
                            "E_CERT_TABLE",
                            "§12.1",
                            path,
                            "drawn element has no declared-domain table row",
                        )]))
                    }
                }
            }
            Expected::Program(_) => {
                let (psym, piface) = program.as_ref().unwrap();
                outcome_from_eval(
                    eval_subject_tuple_bounded(
                        &docs.object,
                        psym,
                        piface,
                        before,
                        &invocation,
                        c,
                        inputs.limits.max_reference_steps,
                    ),
                    path,
                )?
            }
            _ => unreachable!(),
        };
        replayed += 1;
        if left != right {
            return Err(ClaimOutcome::rejected(vec![Reject::new(
                "E_CERT_REPLAY",
                "§12.1",
                path.to_string(),
                format!(
                    "tuple mismatch at element {i}: before {} replayed {} but expected {}",
                    fmt_tuple(before),
                    fmt_tuple(&left),
                    fmt_tuple(&right)
                ),
            )]));
        }
    }
    let _ = replayed;

    let (computed, elements_s, fraction) = if kind == "exhaustive" {
        (
            if domain.covers_full {
                "proof"
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
        ..Default::default()
    };
    out.establishes = if computed == "proof" {
        format!(
            "every element of D ({elements_s} tuples) was replayed and full output tuples agree with the expected relation under the subject {} variant {:?} with {} inherited control(s); by §12.2 this is proof (and the §14.3 T3 premise for permutation subjects)",
            if scheduled { "realised (schedule-bound, exact) " } else { "" },
            subject.variant,
            subject.inherited_controls
        )
    } else if kind == "exhaustive" {
        format!("all {elements_s} domain elements were replayed and agree; this is evidence over a strict slice of D (covered fraction {fraction}), never proof")
    } else {
        format!(
            "{} sampled draws ({} distinct) were replayed on both sides and agree; sampled success is always evidence (distinct covered fraction {fraction}), even at full coverage",
            draws_count.unwrap_or(0),
            distinct_count.unwrap_or(0)
        )
    };
    if requested_label.as_deref() == Some("proof") && computed != "proof" {
        let mut rejected = ClaimOutcome::rejected(vec![Reject::new(
            "E_CERT_COVERAGE_LABEL",
            "§12.2",
            format!("{path}.label"),
            format!("producer requested proof; the checker-computed outcome is {computed} (fraction {fraction})"),
        )]);
        rejected.elements = out.elements;
        rejected.draws = out.draws;
        rejected.distinct = out.distinct;
        rejected.covered_fraction = out.covered_fraction;
        rejected.establishes = out.establishes;
        return Err(rejected);
    }
    Ok(out)
}

fn check_inverse_pair(
    claim: &Value,
    claim_index: usize,
    path: &str,
    region: &Value,
    subject: &crate::cert::Subject,
    docs: &LoadedDocs,
    inputs: &Inputs,
    requested_label: Option<String>,
) -> Result<ClaimOutcome, ClaimOutcome> {
    // Find the inverse_pairs entry that carries this claim.
    let pairs = region
        .get("verification")
        .and_then(|v| v.get("inverse_pairs"))
        .and_then(Value::as_arr)
        .map(|a| a.to_vec())
        .unwrap_or_default();
    let entry = pairs
        .iter()
        .find(|p| p.get("claim").and_then(u64_of) == Some(claim_index as u64))
        .ok_or_else(|| {
            ClaimOutcome::rejected(vec![rej(
                "E_CERT_BINDING",
                "§12.3",
                path.to_string(),
                "inverse_pair claim is not carried by any verification.inverse_pairs entry",
            )])
        })?
        .clone();
    let g_hash = entry
        .get("g")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let ginv_hash = entry
        .get("ginv")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if subject.closure_hash != g_hash && subject.closure_hash != ginv_hash {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.subject.closure_hash"),
            "inverse_pair claim subject must be the entry's g or ginv",
        )]));
    }
    if subject.inherited_controls != 0 {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.subject.inherited_controls"),
            "inverse_pair claims quantify over the uncontrolled pair",
        )]));
    }
    let (Some(g), Some(ginv)) = (
        docs.object.symbol_by_closure(&g_hash).cloned(),
        docs.object.symbol_by_closure(&ginv_hash).cloned(),
    ) else {
        return Ok(ClaimOutcome::unchecked(
            "inverse pair oracle is not in the supplied object",
        ));
    };
    let budget = crate::cert::parse_entry_budget(
        entry.get("budget").unwrap_or(&Value::Null),
        &docs.object,
        &format!("{path} (inverse_pairs entry budget)"),
    )
    .map_err(ClaimOutcome::rejected)?;
    if g.kind != "oracle" || ginv.kind != "oracle" || g.signature != ginv.signature {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§4.5",
            path,
            "inverse-pair endpoints must be oracles with compatible signatures",
        )]));
    }
    let giface = docs
        .object
        .interface(&g.name)
        .map_err(|e| ClaimOutcome::unchecked(format!("g interface: {e}")))?;
    let domain = match resolve_domain(claim.get("domain").unwrap_or(&Value::Null), &giface, 0) {
        Ok(d) => d,
        Err(ResolveError::Reject(r)) => {
            return Err(ClaimOutcome::rejected(
                vec![r.at(&format!("{path}.domain"))],
            ))
        }
        Err(ResolveError::HostLimit(h)) => return Ok(host_limit_outcome(h)),
    };
    // Bundles/regions must be the full domain of g's in and mem slots.
    {
        let mut errs = Vec::new();
        let mut in_covered: std::collections::HashSet<usize> = Default::default();
        for c in &domain.components {
            match &c.id {
                ComponentId::Bundle { group, slot } => {
                    if group != "in" {
                        errs.push(rej(
                            "E_CERT_DOMAIN",
                            "§12.1",
                            format!("{path}.domain"),
                            format!(
                                "inverse_pair domain bundles cover only g's in slots, not {group}"
                            ),
                        ));
                    } else if c.lo.is_zero() && c.size == (BigUint::one() << c.width) {
                        in_covered.insert(*slot);
                    }
                }
                ComponentId::Borrowed(_) => errs.push(rej(
                    "E_CERT_DOMAIN",
                    "§12.1",
                    format!("{path}.domain"),
                    "inverse_pair domain has no borrowed components",
                )),
                _ => {}
            }
        }
        let in_count = giface.bundles.iter().filter(|b| b.group == "in").count();
        if in_covered.len() != in_count {
            errs.push(rej(
                "E_CERT_DOMAIN",
                "§12.1",
                format!("{path}.domain"),
                "inverse_pair bundles must be the full domain of g's in slots",
            ));
        }
        // Mem fullness: every cell of every region present at full range.
        for (rpos, r) in giface.regions.iter().enumerate() {
            let _ = rpos;
            if r.a >= 64 {
                return Ok(host_limit_outcome(crate::fault::HostLimit {
                    quantity: format!("inverse_pair mem region has 2^{} cells", r.a),
                }));
            }
            let expect = 1u64 << r.a;
            let got = domain
                .components
                .iter()
                .filter(|c| matches!(&c.id, ComponentId::Cell { group, slot, .. } if *group == r.group && *slot == r.slot)
                        && c.lo.is_zero() && c.size == (BigUint::one() << c.width))
                .count() as u64;
            if got != expect {
                errs.push(rej(
                    "E_CERT_DOMAIN",
                    "§12.1",
                    format!("{path}.domain"),
                    "inverse_pair regions must be the full domain of g's mem slots",
                ));
                break;
            }
        }
        if !errs.is_empty() {
            return Err(ClaimOutcome::rejected(errs));
        }
    }
    if domain.n > BigUint::from(inputs.limits.max_elements) {
        return Ok(host_limit_outcome(crate::fault::HostLimit {
            quantity: format!(
                "inverse_pair element count {} (cap {})",
                domain.n, inputs.limits.max_elements
            ),
        }));
    }
    if let Err(h) = domain.check_retained_elements(&domain.n) {
        return Ok(host_limit_outcome(h));
    }
    // Classical arguments: tokens fixed by arg_tokens; others quantified by
    // the domain's args components in listed order.
    let arg_tokens = entry
        .get("arg_tokens")
        .and_then(Value::as_obj)
        .cloned()
        .unwrap_or_default();
    let gfunc = docs.object.function(&g.name).cloned().unwrap();
    let mut token_tuples: Vec<(usize, [BigUint; 4])> = Vec::new();
    let mut non_token_positions: Vec<usize> = Vec::new();
    for (i, ty) in giface.args.iter().enumerate() {
        if ty == "token" {
            let pname = gfunc.groups[4]
                .get(i)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let raw = arg_tokens.get(&pname).cloned().ok_or_else(|| {
                ClaimOutcome::rejected(vec![rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    path.to_string(),
                    format!("token argument {pname:?} has no arg_tokens tuple in the entry"),
                )])
            })?;
            let tuple = crate::cert::parse_entry_budget(
                &raw,
                &docs.object,
                &format!("{path}.arg_tokens.{pname}"),
            )
            .map_err(ClaimOutcome::rejected)?;
            token_tuples.push((i, tuple));
        } else {
            non_token_positions.push(i);
        }
    }
    if domain.arg_widths.len() != non_token_positions.len() {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_DOMAIN",
            "§12.2",
            format!("{path}.domain.args"),
            format!(
                "args domain lists {} entries; g has {} non-token classical arguments",
                domain.arg_widths.len(),
                non_token_positions.len()
            ),
        )]));
    }
    for (entry_i, pos) in non_token_positions.iter().enumerate() {
        let ty = &giface.args[*pos];
        let want = if ty == "ptr" {
            docs.object.header.ptr_width
        } else if ty == "f64" {
            64
        } else {
            ty.strip_prefix('i')
                .and_then(|n| n.parse().ok())
                .unwrap_or(0)
        };
        if domain.arg_widths[entry_i] != want {
            return Err(ClaimOutcome::rejected(vec![rej(
                "E_CERT_DOMAIN",
                "§12.2",
                format!("{path}.domain.args[{entry_i}]"),
                format!(
                    "args width {} != type width {want} of {ty}",
                    domain.arg_widths[entry_i]
                ),
            )]));
        }
    }
    let assemble_args = |element: &Tuple| -> Vec<Classical> {
        let mut out: Vec<Classical> = giface
            .args
            .iter()
            .map(|_| Classical::Bits(BigUint::zero()))
            .collect();
        for (i, t) in &token_tuples {
            out[*i] = Classical::Token(t.clone());
        }
        for (k, pos) in non_token_positions.iter().enumerate() {
            out[*pos] = Classical::Bits(element.args.get(k).cloned().unwrap_or_default());
        }
        out
    };
    let in_positions: Vec<usize> = giface
        .bundles
        .iter()
        .enumerate()
        .filter(|(_, b)| b.group == "in")
        .map(|(i, _)| i)
        .collect();
    let elements: Vec<Tuple> = domain.elements().collect();
    for (ei, element) in elements.iter().enumerate() {
        let ins: Vec<BigUint> = in_positions
            .iter()
            .map(|p| element.bundles[*p].clone())
            .collect();
        let cargs = assemble_args(element);
        let run = |sym: &Symbol, ins: &[BigUint]| -> Result<OracleOutcome, ClaimOutcome> {
            outcome_from_eval(
                eval_oracle_bounded(
                    &docs.object,
                    sym,
                    ins,
                    &element.regions,
                    &cargs,
                    &budget,
                    0,
                    inputs.limits.max_reference_steps,
                ),
                path,
            )
        };
        for (first, second, order) in [(&g, &ginv, "ginv(g(x))"), (&ginv, &g, "g(ginv(x))")] {
            let o1 = run(first, &ins)?;
            if !o1.code.is_zero() {
                return Err(ClaimOutcome::rejected(vec![Reject::new(
                    "E_CERT_REPLAY",
                    "§12.1",
                    path.to_string(),
                    format!(
                        "element {ei}: {} faults (code {}) in {order}; the pair must be fault-free",
                        first.name, o1.code
                    ),
                )]));
            }
            let mut ins2 = ins.clone();
            if ins2.is_empty() {
                return Err(ClaimOutcome::rejected(vec![rej(
                    "E_CERT_DOMAIN",
                    "§12.1",
                    path.to_string(),
                    "g has no in slots for the pair value x",
                )]));
            }
            ins2[0] = o1.payload.clone();
            let o2 = run(second, &ins2)?;
            if !o2.code.is_zero() {
                return Err(ClaimOutcome::rejected(vec![Reject::new(
                    "E_CERT_REPLAY",
                    "§12.1",
                    path.to_string(),
                    format!(
                        "element {ei}: {} faults (code {}) in {order}",
                        second.name, o2.code
                    ),
                )]));
            }
            if o2.payload != ins[0] {
                return Err(ClaimOutcome::rejected(vec![Reject::new(
                    "E_CERT_REPLAY",
                    "§12.1",
                    path.to_string(),
                    format!("element {ei}: {order} = {} != x = {}", o2.payload, ins[0]),
                )]));
            }
        }
    }
    let mut out = ClaimOutcome {
        outcome: "proof".into(),
        establishes: format!(
            "for every one of {} elements (full in/mem domain of g, quantified classical args as listed, tokens at the entry tuples) both compositions are fault-free and restore x at budget [{}]",
            domain.n,
            budget.iter().map(BigUint::to_string).collect::<Vec<_>>().join(",")
        ),
        elements: Some(domain.n.to_string()),
        covered_fraction: Some(domain.covered_fraction()),
        ..Default::default()
    };
    let _ = requested_label;
    out.outcome = if domain.covers_full {
        "proof".into()
    } else {
        "proof".into()
    };
    Ok(out)
}
