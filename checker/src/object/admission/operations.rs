use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Gate {
    X,
    H,
    S,
    Sdg,
    T,
    Tdg,
    Cx,
    Ccx,
    Mcx,
    Swap,
    Cz,
    Rx,
    Ry,
    Rz,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Binary {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Shl,
    Lshr,
    Ashr,
    Udiv,
    Urem,
    Sdiv,
    Srem,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Compare {
    Eq,
    Ne,
    Ult,
    Ule,
    Slt,
    Sle,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Cast {
    Zext,
    Sext,
    Trunc,
    PtrToInt,
    IntToPtr,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Code {
    Gate(Gate),
    Alloc,
    Borrow,
    Free,
    Return,
    Copy,
    Uncopy,
    Xor,
    Exchange,
    Inplace,
    Region,
    Load,
    Unload,
    Store,
    ExchangeMemory,
    Pred,
    Unpred,
    If,
    Loop,
    ActionCall,
    OracleCall,
    PermCall,
    OracleUncall,
    Const(BigUint),
    Binary(Binary),
    Compare(Compare),
    Select,
    Cast(Cast, Option<BigUint>),
    Move,
    ParallelMove,
    Label,
    Address,
    BoundsCheck,
    LoadScalar(BigUint),
    StoreScalar(BigUint),
    AllocateScalar,
    FreeScalar,
    FaultIf,
    ProcedureCall,
    ProcedureReturn,
    ReturnValue,
    Jump,
    Branch,
    JumpRegister,
    Fault,
    Br,
    Call,
    Ret,
    Uncall,
    Trap,
    RegisterNew,
    RegisterFree,
    RegisterSlice,
    RegisterConcat,
    RegisterWidth,
    RegisterId,
    RegisterFromId,
    Measure(Option<BigUint>),
    Budget,
    Effect,
    HostCall,
}
#[derive(Clone, Debug)]
pub(super) struct Row {
    pub code: Code,
    pub results: Vec<String>,
    pub args: Vec<String>,
    pub controls: Vec<String>,
    pub lifted: bool,
    pub adjoint: bool,
}
impl Row {
    pub fn parse(line: &str, kind: &str) -> Result<Self, String> {
        let tokens = tokenise(line);
        let mut at = 0;
        let mut controls = vec![];
        let mut adjoint = false;
        loop {
            match tokens.get(at).map(String::as_str) {
                Some("ctl") => {
                    controls.push(
                        tokens
                            .get(at + 1)
                            .ok_or_else(|| sec_err("§4.8", "missing ctl wire"))?
                            .clone(),
                    );
                    at += 2;
                }
                Some("adj") => {
                    adjoint = !adjoint;
                    at += 1;
                }
                _ => break,
            }
        }
        let results = if tokens.get(at + 1).map(String::as_str) == Some("=") {
            let lhs = tokens[at].clone();
            at += 2;
            if lhs.starts_with('(') {
                let items = list(&lhs)?;
                require(!items.is_empty(), "§16.2", "lhs requires at least one NAME")?;
                items
            } else {
                vec![lhs]
            }
        } else {
            vec![]
        };
        for result in &results {
            require(wires::valid_name(result), "§16.2", "invalid result NAME")?;
        }
        let mnemonic = tokens
            .get(at)
            .ok_or_else(|| sec_err("§4.8", "missing opcode"))?;
        let lifted = mnemonic.starts_with("lift.");
        let base = mnemonic.strip_prefix("lift.").unwrap_or(mnemonic);
        let shorthand = base.starts_with('@');
        let code = if shorthand {
            Code::ActionCall
        } else {
            decode(base, lifted)?
        };
        let args = if shorthand {
            tokens[at..].to_vec()
        } else {
            tokens[at + 1..].to_vec()
        };
        let row = Self {
            code,
            results,
            args,
            controls,
            lifted,
            adjoint,
        };
        for arg in &row.args {
            operand(arg)?;
        }
        row.shape(kind)?;
        Ok(row)
    }
    fn shape(&self, kind: &str) -> Result<(), String> {
        let (lo, hi, results) = schema(&self.code);
        require(
            (lo..=hi).contains(&self.args.len()),
            "§4.8",
            "wrong operand arity",
        )?;
        require(
            results.map_or(true, |n| self.results.len() == n),
            "§4.8",
            "wrong result placement/arity",
        )?;
        require(
            self.results.iter().collect::<BTreeSet<_>>().len() == self.results.len(),
            "§3.3",
            "repeated destinations",
        )?;
        let action = matches!(kind, "action" | "perm_action");
        let rows = matches!(kind, "oracle" | "lproc");
        let quantum = action_code(&self.code);
        let permitted = match self.code {
            Code::ActionCall => !self.lifted && (action || matches!(kind, "cc" | "r")),
            Code::OracleCall => {
                if self.lifted {
                    rows
                } else {
                    action || matches!(kind, "cc" | "r")
                }
            }
            Code::PermCall | Code::OracleUncall => rows && self.lifted,
            Code::Jump | Code::Branch | Code::JumpRegister => rows && !self.lifted,
            Code::Fault => !self.lifted && (rows || matches!(kind, "cc" | "r")),
            Code::ReturnValue => kind == "oracle" && self.lifted,
            Code::ProcedureReturn => kind == "lproc" && self.lifted,
            Code::ProcedureCall
            | Code::Move
            | Code::ParallelMove
            | Code::Label
            | Code::Address
            | Code::BoundsCheck
            | Code::FaultIf => rows && self.lifted,
            _ if quantum => action && !self.lifted,
            _ if self.lifted => rows && lifted_code(&self.code),
            _ => matches!(kind, "cc" | "r") && classical_code(&self.code),
        };
        require(
            permitted,
            "§4.8",
            "opcode is not allowed in this function tier",
        )?;
        require(
            kind != "r" || !matches!(self.code, Code::HostCall | Code::Measure(_)),
            "§4.8",
            "CC-only opcode in R",
        )?;
        require(
            kind != "cc" || self.code != Code::Effect,
            "§4.8",
            "effect is R-only",
        )?;
        require(
            self.controls.is_empty()
                || (!self.lifted && matches!(self.code, Code::ActionCall | Code::OracleCall)),
            "§4.8",
            "invalid controlled instruction",
        )?;
        require(
            !self.adjoint
                || (!self.lifted && matches!(self.code, Code::ActionCall | Code::OracleCall)),
            "§4.8",
            "adj requires a region call",
        )
    }
}

fn width(s: &str) -> Result<BigUint, String> {
    let n = positive(
        s.strip_prefix('i')
            .ok_or_else(|| sec_err("§4.8", "expected .iN suffix"))?,
    )?;
    Ok(n)
}
fn no_suffix(parts: &[&str], code: Code) -> Result<Code, String> {
    require(parts.len() == 1, "§4.8", "unexpected opcode suffix")?;
    Ok(code)
}
fn decode(base: &str, lifted: bool) -> Result<Code, String> {
    let p: Vec<_> = base.split('.').collect();
    let suffix = || {
        p.get(1)
            .ok_or_else(|| sec_err("§4.8", "missing type suffix"))
            .and_then(|s| width(s))
    };
    let code = match p[0] {
        "const" => Code::Const(suffix()?),
        "zext" => Code::Cast(Cast::Zext, Some(suffix()?)),
        "sext" => Code::Cast(Cast::Sext, Some(suffix()?)),
        "trunc" => Code::Cast(Cast::Trunc, Some(suffix()?)),
        "ptrtoint" => Code::Cast(Cast::PtrToInt, Some(suffix()?)),
        "load" => Code::LoadScalar(suffix()?),
        "store" => Code::StoreScalar(suffix()?),
        "measure" => Code::Measure(match p.get(1) {
            Some(s) => Some(width(s)?),
            None => None,
        }),
        "icmp" => Code::Compare(match p.get(1).copied() {
            Some("eq") => Compare::Eq,
            Some("ne") => Compare::Ne,
            Some("ult") => Compare::Ult,
            Some("ule") => Compare::Ule,
            Some("slt") => Compare::Slt,
            Some("sle") => Compare::Sle,
            _ => return Err(sec_err("§4.8", "unknown comparison predicate")),
        }),
        "qreg" => {
            return match p.as_slice() {
                ["qreg", "new"] => Ok(Code::RegisterNew),
                ["qreg", "free"] => Ok(Code::RegisterFree),
                ["qreg", "slice"] => Ok(Code::RegisterSlice),
                ["qreg", "concat"] => Ok(Code::RegisterConcat),
                ["qreg", "width"] => Ok(Code::RegisterWidth),
                ["qreg", "id"] => Ok(Code::RegisterId),
                ["qreg", "from_id"] => Ok(Code::RegisterFromId),
                _ => Err(sec_err("§4.8", "unknown qreg opcode")),
            }
        }
        "budget" => {
            return match p.as_slice() {
                ["budget", "mint"] => Ok(Code::Budget),
                _ => Err(sec_err("§4.8", "unknown budget opcode")),
            }
        }
        _ => return no_suffix(&p, bare(p[0], lifted)?),
    };
    require(p.len() <= 2, "§4.8", "trailing opcode suffix")?;
    Ok(code)
}
fn bare(word: &str, lifted: bool) -> Result<Code, String> {
    Ok(match word {
        "x" => Code::Gate(Gate::X),
        "h" => Code::Gate(Gate::H),
        "s" => Code::Gate(Gate::S),
        "sdg" => Code::Gate(Gate::Sdg),
        "t" => Code::Gate(Gate::T),
        "tdg" => Code::Gate(Gate::Tdg),
        "cx" => Code::Gate(Gate::Cx),
        "ccx" => Code::Gate(Gate::Ccx),
        "mcx" => Code::Gate(Gate::Mcx),
        "swap" => Code::Gate(Gate::Swap),
        "cz" => Code::Gate(Gate::Cz),
        "rx" => Code::Gate(Gate::Rx),
        "ry" => Code::Gate(Gate::Ry),
        "rz" => Code::Gate(Gate::Rz),
        "qalloc" => Code::Alloc,
        "qborrow" => Code::Borrow,
        "qfree" => Code::Free,
        "qreturn" => Code::Return,
        "qcopy" => Code::Copy,
        "quncopy" => Code::Uncopy,
        "qxor" => Code::Xor,
        "qexch" => Code::Exchange,
        "qinplace" => Code::Inplace,
        "qregion" => Code::Region,
        "qload" => Code::Load,
        "qunload" => Code::Unload,
        "qstore" => Code::Store,
        "qexchm" => Code::ExchangeMemory,
        "qpred" => Code::Pred,
        "qunpred" => Code::Unpred,
        "qif" => Code::If,
        "qloop" => Code::Loop,
        "action_call" => Code::ActionCall,
        "oracle_call" => Code::OracleCall,
        "oracle_uncall" => Code::OracleUncall,
        "perm_call" => Code::PermCall,
        "add" => Code::Binary(Binary::Add),
        "sub" => Code::Binary(Binary::Sub),
        "mul" => Code::Binary(Binary::Mul),
        "and" => Code::Binary(Binary::And),
        "or" => Code::Binary(Binary::Or),
        "xor" => Code::Binary(Binary::Xor),
        "shl" => Code::Binary(Binary::Shl),
        "lshr" => Code::Binary(Binary::Lshr),
        "ashr" => Code::Binary(Binary::Ashr),
        "udiv" => Code::Binary(Binary::Udiv),
        "urem" => Code::Binary(Binary::Urem),
        "sdiv" => Code::Binary(Binary::Sdiv),
        "srem" => Code::Binary(Binary::Srem),
        "select" => Code::Select,
        "inttoptr" => Code::Cast(Cast::IntToPtr, None),
        "mov" => Code::Move,
        "parmov" => Code::ParallelMove,
        "label" => Code::Label,
        "addr" => Code::Address,
        "boundscheck" => Code::BoundsCheck,
        "alloc" => Code::AllocateScalar,
        "free" => Code::FreeScalar,
        "faultif" => Code::FaultIf,
        "call" if lifted => Code::ProcedureCall,
        "ret" if lifted => Code::ProcedureReturn,
        "retval" => Code::ReturnValue,
        "qjump" => Code::Jump,
        "qbranch" => Code::Branch,
        "qjumpr" => Code::JumpRegister,
        "fault" => Code::Fault,
        "br" => Code::Br,
        "call" => Code::Call,
        "ret" => Code::Ret,
        "uncall" => Code::Uncall,
        "trap" => Code::Trap,
        "effect" => Code::Effect,
        "hostcall" => Code::HostCall,
        _ => return Err(sec_err("§4.8", format!("unknown opcode {word}"))),
    })
}
fn action_code(code: &Code) -> bool {
    matches!(
        code,
        Code::Gate(_)
            | Code::Alloc
            | Code::Borrow
            | Code::Free
            | Code::Return
            | Code::Copy
            | Code::Uncopy
            | Code::Xor
            | Code::Exchange
            | Code::Inplace
            | Code::Region
            | Code::Load
            | Code::Unload
            | Code::Store
            | Code::ExchangeMemory
            | Code::Pred
            | Code::Unpred
            | Code::If
            | Code::Loop
    )
}
fn lifted_code(code: &Code) -> bool {
    matches!(
        code,
        Code::Const(_)
            | Code::Binary(_)
            | Code::Compare(_)
            | Code::Select
            | Code::Cast(Cast::Zext | Cast::Sext | Cast::Trunc, _)
            | Code::LoadScalar(_)
            | Code::StoreScalar(_)
            | Code::AllocateScalar
            | Code::FreeScalar
    )
}
fn classical_code(code: &Code) -> bool {
    lifted_code(code)
        || matches!(
            code,
            Code::Cast(_, _)
                | Code::Br
                | Code::Call
                | Code::Ret
                | Code::Uncall
                | Code::Trap
                | Code::Fault
                | Code::RegisterNew
                | Code::RegisterFree
                | Code::RegisterSlice
                | Code::RegisterConcat
                | Code::RegisterWidth
                | Code::RegisterId
                | Code::RegisterFromId
                | Code::Measure(_)
                | Code::Budget
                | Code::Effect
                | Code::HostCall
        )
}
fn schema(code: &Code) -> (usize, usize, Option<usize>) {
    match code {
        Code::Gate(Gate::X | Gate::H | Gate::S | Gate::Sdg | Gate::T | Gate::Tdg) => {
            (1, 1, Some(0))
        }
        Code::Gate(Gate::Ccx) => (3, 3, Some(0)),
        Code::Gate(_) => (2, 2, Some(0)),
        Code::Alloc | Code::Borrow => (2, 2, Some(1)),
        Code::Free | Code::Return | Code::Pred | Code::Unpred => (1, 1, Some(0)),
        Code::Copy => (1, 1, Some(1)),
        Code::Uncopy | Code::Xor | Code::Exchange => (2, 2, Some(0)),
        Code::Inplace => (4, 7, Some(0)),
        Code::Region => (3, 4, Some(0)),
        Code::Load => (2, 2, Some(1)),
        Code::Unload | Code::Store | Code::ExchangeMemory => (3, 3, Some(0)),
        Code::If => (1, 1, Some(0)),
        Code::Loop => (1, 3, Some(0)),
        Code::ActionCall | Code::PermCall => (5, 5, Some(0)),
        Code::OracleCall | Code::OracleUncall => (7, 7, Some(0)),
        Code::Const(_)
        | Code::Cast(_, _)
        | Code::Move
        | Code::Label
        | Code::Address
        | Code::LoadScalar(_)
        | Code::AllocateScalar
        | Code::RegisterWidth
        | Code::RegisterId
        | Code::RegisterFromId
        | Code::Measure(_) => (1, 1, Some(1)),
        Code::Binary(_) | Code::Compare(_) => (2, 2, Some(1)),
        Code::Select => (3, 3, Some(1)),
        Code::ParallelMove => (1, 1, None),
        Code::BoundsCheck | Code::FaultIf | Code::StoreScalar(_) => (2, 2, Some(0)),
        Code::FreeScalar
        | Code::ProcedureReturn
        | Code::ReturnValue
        | Code::Jump
        | Code::JumpRegister
        | Code::Fault => (1, 1, Some(0)),
        Code::ProcedureCall => (2, 5, Some(1)),
        Code::Branch => (3, 3, Some(0)),
        Code::Br => (1, 3, Some(0)),
        Code::Call => (1, usize::MAX, None), // Results checked against callee.
        Code::Ret => (0, usize::MAX, Some(0)),
        Code::Uncall => (3, 3, Some(0)),
        Code::Trap => (0, 0, Some(0)),
        Code::RegisterNew | Code::RegisterConcat => (2, 2, Some(1)),
        Code::RegisterFree => (1, 1, Some(0)),
        Code::RegisterSlice => (3, 3, Some(1)),
        Code::Budget => (5, 5, Some(1)),
        Code::Effect => (3, usize::MAX, Some(0)),
        Code::HostCall => (3, 3, Some(1)),
    }
}

fn operand(text: &str) -> Result<(), String> {
    if wires::valid_name(text) {
        return Ok(());
    }
    if let Some(name) = text.strip_prefix('@') {
        return require(wires::valid_name(name), "§16.2", "invalid symbol NAME");
    }
    if let Some(text) = text.strip_prefix('^') {
        if let Some((name, rest)) = text.split_once('(') {
            require(wires::valid_name(name), "§16.2", "invalid block NAME")?;
            for item in list(&format!("({rest}"))? {
                require(
                    wires::valid_name(&item),
                    "§16.2",
                    "block arguments must be NAMEs",
                )?;
            }
            return Ok(());
        }
        return require(wires::valid_name(text), "§16.2", "invalid block NAME");
    }
    if text.starts_with('(') {
        for item in list(text)? {
            require(
                wires::valid_name(&item),
                "§16.2",
                "operand lists require NAME entries",
            )?;
        }
        return Ok(());
    }
    if let Some((name, index)) = text.split_once('[') {
        require(wires::valid_name(name), "§16.2", "invalid wire NAME")?;
        uint(
            index
                .strip_suffix(']')
                .ok_or_else(|| sec_err("§16.2", "unterminated index"))?,
        )?;
        return Ok(());
    }
    if text.starts_with('"') && text.ends_with('"') {
        return Ok(());
    }
    if integer(text).is_ok() {
        return Ok(());
    }
    super::super::lexical::float(text).map_err(|e| sec_err("§16.1", e))
}
