use super::operations::Cast;
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Type {
    Integer(BigUint),
    Pointer,
    Float,
    Token,
    Register(Option<BigUint>),
    Wire(BigUint),
    Region(BigUint, BigUint),
    Location(BigUint),
}
impl Type {
    pub fn parse(text: &str) -> Result<Self, String> {
        Ok(match text {
            "ptr" => Self::Pointer,
            "f64" => Self::Float,
            "token" => Self::Token,
            "wire" => Self::Wire(1u32.into()),
            "qreg[*]" => Self::Register(None),
            _ => {
                if let Some(n) = text.strip_prefix('i') {
                    let n = positive(n)?;
                    require(n <= MAX_INLINE_WIDTH.into(), "§3.3", "iN requires N<=128")?;
                    Self::Integer(n)
                } else if let Some(n) = text.strip_prefix("wire[").and_then(|s| s.strip_suffix(']'))
                {
                    Self::Wire(positive(n)?)
                } else if let Some(n) = text.strip_prefix("reg[").and_then(|s| s.strip_suffix(']'))
                {
                    Self::Location(positive(n)?)
                } else if let Some(n) = text.strip_prefix("qreg[").and_then(|s| s.strip_suffix(']'))
                {
                    Self::Register(Some(positive(n)?))
                } else if let Some(s) = text
                    .strip_prefix("region[")
                    .and_then(|s| s.strip_suffix(']'))
                {
                    let (a, w) = s
                        .split_once(',')
                        .ok_or_else(|| sec_err("§16.2", "invalid region type"))?;
                    Self::Region(uint(a)?, positive(w)?)
                } else {
                    return Err(sec_err("§3.3", format!("unknown type {text}")));
                }
            }
        })
    }
    pub fn accepts_transfer_to(&self, expected: &Self) -> bool {
        match (self, expected) {
            (Self::Register(actual), Self::Register(wanted)) => {
                actual.is_none() || wanted.is_none() || actual == wanted
            }
            _ => self == expected,
        }
    }
    pub fn bits(&self, ptr: u64) -> Result<BigUint, String> {
        match self {
            Self::Integer(n) | Self::Wire(n) | Self::Location(n) => Ok(n.clone()),
            Self::Pointer => Ok(ptr.into()),
            Self::Float => Ok(FLOAT_WIDTH.into()),
            _ => Err(sec_err("§3.3", "expected a bit-vector operand")),
        }
    }
}
pub(super) struct Scalar<'a> {
    pub object: &'a Object,
    pub function: &'a Function,
    pub values: BTreeMap<String, Type>,
    pub locations: BTreeSet<String>,
    labels: BTreeMap<String, Vec<Param>>,
}
impl<'a> Scalar<'a> {
    pub fn new(object: &'a Object, function: &'a Function) -> Result<Self, String> {
        let mut values = BTreeMap::new();
        let mut locations = BTreeSet::new();
        for (group, params) in function.groups.iter().enumerate() {
            for param in params {
                let ty = Type::parse(&param.ty)?;
                if (matches!(function.kind.as_str(), "cc" | "r") && group == 1)
                    || (function.kind == "oracle" && matches!(group, 1 | 2))
                {
                    continue;
                }
                require(
                    values.insert(param.name.clone(), ty.clone()).is_none(),
                    "§3.3",
                    "repeated parameter name",
                )?;
                match ty {
                    Type::Location(_) => {
                        locations.insert(param.name.clone());
                    }
                    _ => {}
                }
            }
        }
        for line in function.bytes.lines().filter(|s| s.starts_with("regs ")) {
            let body = line
                .strip_prefix("regs { ")
                .and_then(|s| s.split_once(" }").map(|p| p.0))
                .ok_or_else(|| sec_err("§16.2", "malformed regs"))?;
            if body.is_empty() {
                continue;
            }
            for declaration in super::super::parse_plist(body)? {
                let ty = Type::parse(&declaration.ty)?;
                require(
                    values
                        .insert(declaration.name.clone(), ty.clone())
                        .is_none(),
                    "§3.3",
                    "duplicate location",
                )?;
                if matches!(function.kind.as_str(), "oracle" | "lproc") {
                    require(
                        matches!(ty, Type::Location(_)),
                        "§3.3",
                        "lifted regs must be reg[N] locations",
                    )?;
                }
                locations.insert(declaration.name);
            }
        }
        let labels = function
            .blocks
            .iter()
            .map(|b| (b.name.clone(), b.params.clone()))
            .collect();
        Ok(Self {
            object,
            function,
            values,
            locations,
            labels,
        })
    }
    pub fn validate(
        object: &'a Object,
        function: &'a Function,
        rows: &[Row],
    ) -> Result<(), String> {
        let mut scope = Self::new(object, function)?;
        for block in &function.blocks {
            for param in &block.params {
                let ty = Type::parse(&param.ty)?;
                classical::declare(&mut scope, &param.name, ty)?;
            }
        }
        if matches!(function.kind.as_str(), "cc" | "r") {
            classical::definitions(&mut scope, rows)?;
        }
        for row in rows {
            scope.row(row)?;
        }
        scope.terminators()?;
        if matches!(function.kind.as_str(), "cc" | "r") {
            views::validate(function, &scope)?;
        }
        Ok(())
    }
    pub fn value(&self, name: &str) -> Result<&Type, String> {
        self.values
            .get(name)
            .ok_or_else(|| sec_err("§3.3", format!("unknown value/location {name}")))
    }
    pub fn bits(&self, name: &str) -> Result<BigUint, String> {
        self.value(name)?.bits(self.object.header.ptr_width)
    }
    pub fn result(&mut self, row: &Row, types: Vec<Type>) -> Result<(), String> {
        require(
            row.results.len() == types.len(),
            "§4.8",
            "result count differs from opcode/callee",
        )?;
        for (name, ty) in row.results.iter().zip(types) {
            match self.function.kind.as_str() {
                "oracle" | "lproc" => {
                    require(
                        self.locations.contains(name),
                        "§3.3",
                        "destination is not a declared location",
                    )?;
                    require(
                        self.bits(name)? == ty.bits(self.object.header.ptr_width)?,
                        "§3.3",
                        "destination width mismatch",
                    )?;
                }
                _ => {
                    classical::declare(self, name, ty)?;
                }
            }
        }
        Ok(())
    }
    fn integer_value(&self, name: &str) -> Result<Type, String> {
        let t = self.value(name)?.clone();
        require(
            matches!(t, Type::Integer(_)),
            "§3.3",
            "expected integer SSA value",
        )?;
        Ok(t)
    }
    fn same_bits(&self, args: &[String]) -> Result<BigUint, String> {
        let first = self.bits(&args[0])?;
        for value in &args[1..] {
            require(self.bits(value)? == first, "§3.3", "operand widths differ")?;
        }
        Ok(first)
    }
    fn arithmetic(&self, row: &Row) -> Result<Type, String> {
        if row.lifted {
            Ok(Type::Location(self.same_bits(&row.args)?))
        } else {
            let first = self.integer_value(&row.args[0])?;
            for arg in &row.args[1..] {
                require(
                    self.value(arg)? == &first,
                    "§3.3",
                    "integer operand types differ",
                )?;
            }
            Ok(first)
        }
    }
    fn constant(&self, row: &Row, n: &BigUint) -> Result<Type, String> {
        if !row.lifted {
            require(
                n <= &MAX_INLINE_WIDTH.into(),
                "§3.3",
                "const.iN requires N<=128 outside lifted rows",
            )?;
        }
        let value = integer(&row.args[0])?;
        let magnitude = value.magnitude();
        let bits = BigUint::from(magnitude.bits());
        let valid = match value.sign() {
            num_bigint::Sign::Minus => {
                bits < n.clone()
                    || (bits == *n
                        && !magnitude.is_zero()
                        && (magnitude & (magnitude - 1u32)).is_zero())
            }
            _ => bits <= *n,
        };
        require(
            valid,
            "§3.3",
            "constant lies outside its signed/unsigned width range",
        )?;
        Ok(if row.lifted {
            Type::Location(n.clone())
        } else {
            Type::Integer(n.clone())
        })
    }
    fn cast(&self, row: &Row, cast: Cast, n: &Option<BigUint>) -> Result<Type, String> {
        let old = self.bits(&row.args[0])?;
        let width = match cast {
            Cast::IntToPtr => BigUint::from(self.object.header.ptr_width),
            _ => n
                .clone()
                .ok_or_else(|| sec_err("§4.8", "cast width absent"))?,
        };
        let valid = match cast {
            Cast::Zext | Cast::Sext => width > old,
            Cast::Trunc => width < old,
            _ => true,
        };
        require(valid, "§3.3", "cast must widen/narrow strictly")?;
        if row.lifted {
            return Ok(Type::Location(width));
        }
        match cast {
            Cast::PtrToInt => require(
                self.value(&row.args[0])? == &Type::Pointer,
                "§3.3",
                "ptrtoint requires ptr",
            )?,
            _ => {
                self.integer_value(&row.args[0])?;
            }
        }
        Ok(if cast == Cast::IntToPtr {
            Type::Pointer
        } else {
            Type::Integer(width)
        })
    }
    fn compare_type(&self, row: &Row) -> Result<Type, String> {
        self.same_bits(&row.args)?;
        if !row.lifted {
            let ty = self.value(&row.args[0])?;
            require(
                matches!(ty, Type::Integer(_) | Type::Pointer) && self.value(&row.args[1])? == ty,
                "§3.3",
                "icmp operand types differ",
            )?;
        }
        Ok(if row.lifted {
            Type::Location(1u32.into())
        } else {
            Type::Integer(1u32.into())
        })
    }
    fn select_type(&self, row: &Row) -> Result<Type, String> {
        require(
            self.bits(&row.args[0])? == 1u32.into(),
            "§3.3",
            "select condition is not one bit",
        )?;
        let width = self.same_bits(&row.args[1..])?;
        if row.lifted {
            return Ok(Type::Location(width));
        }
        require(
            self.value(&row.args[0])? == &Type::Integer(1u32.into()),
            "§3.3",
            "classical select condition requires i1",
        )?;
        require(
            self.value(&row.args[1])? == self.value(&row.args[2])?,
            "§3.3",
            "select arm types differ",
        )?;
        Ok(self.value(&row.args[1])?.clone())
    }
    fn row(&mut self, row: &Row) -> Result<(), String> {
        let result = match &row.code {
            Code::Const(n) => Some(self.constant(row, n)?),
            Code::Binary(_) => Some(self.arithmetic(row)?),
            Code::Compare(_) => Some(self.compare_type(row)?),
            Code::Select => Some(self.select_type(row)?),
            Code::Cast(c, n) => Some(self.cast(row, *c, n)?),
            Code::Move => Some(Type::Location(self.bits(&row.args[0])?)),
            Code::ParallelMove => {
                let args = list(&row.args[0])?;
                let types = args
                    .iter()
                    .map(|a| self.bits(a).map(Type::Location))
                    .collect::<Result<Vec<_>, _>>()?;
                return self.result(row, types);
            }
            Code::Label => {
                self.label(&row.args[0], false)?;
                require(
                    row.results.len() == 1 && self.locations.contains(&row.results[0]),
                    "§3.3",
                    "label destination is a location",
                )?;
                return Ok(());
            }
            Code::Address => {
                let name = row.args[0]
                    .strip_prefix('@')
                    .ok_or_else(|| sec_err("§4.8", "address requires symbol"))?;
                let global = self
                    .object
                    .layout
                    .as_ref()
                    .and_then(|v| v.get("globals"))
                    .and_then(|v| v.get(name))
                    .is_some();
                let region = self.function.kind == "oracle"
                    && self
                        .values
                        .get(name)
                        .is_some_and(|t| matches!(t, Type::Region(_, _)));
                require(
                    global || region,
                    "§3.3",
                    "unknown global or mapped oracle region",
                )?;
                Some(Type::Location(self.object.header.ptr_width.into()))
            }
            Code::LoadScalar(n) | Code::StoreScalar(n) => {
                require(
                    n >= &8u32.into() && n <= &MAX_INLINE_WIDTH.into() && (n % 8u32).is_zero(),
                    "§4.8",
                    "memory row width must be 8..128 in bytes",
                )?;
                self.address(&row.args[0], row.lifted)?;
                require(
                    self.object.layout.is_some(),
                    "§7.2",
                    "memory instructions require layout",
                )?;
                match row.code {
                    Code::StoreScalar(_) => {
                        require(
                            row.lifted || self.value(&row.args[1])? == &Type::Integer(n.clone()),
                            "§3.3",
                            "classical store requires an integer value",
                        )?;
                        require(
                            self.bits(&row.args[1])? == *n,
                            "§3.3",
                            "store value width differs",
                        )?;
                        None
                    }
                    _ => Some(if row.lifted {
                        Type::Location(n.clone())
                    } else {
                        Type::Integer(n.clone())
                    }),
                }
            }
            Code::AllocateScalar => {
                self.length(&row.args[0], row.lifted)?;
                require(
                    self.object.layout.is_some(),
                    "§7.2",
                    "allocation requires layout",
                )?;
                Some(if row.lifted {
                    Type::Location(self.object.header.ptr_width.into())
                } else {
                    Type::Pointer
                })
            }
            Code::FreeScalar | Code::BoundsCheck => {
                self.address(&row.args[0], row.lifted)?;
                if row.code == Code::BoundsCheck {
                    self.length(&row.args[1], true)?;
                }
                require(
                    self.object.layout.is_some(),
                    "§7.2",
                    "memory instructions require layout",
                )?;
                None
            }
            Code::Fault | Code::FaultIf => {
                let cause = match row.code {
                    Code::FaultIf => {
                        require(
                            self.bits(&row.args[0])? == 1u32.into(),
                            "§3.3",
                            "faultif requires one bit",
                        )?;
                        &row.args[1]
                    }
                    _ => &row.args[0],
                };
                let cause = match cause.as_str() {
                    "FAULT_BUDGET_STEPS" => 1u32.into(),
                    "FAULT_BUDGET_HEAP" => 2u32.into(),
                    "FAULT_BUDGET_STACK" => 3u32.into(),
                    "FAULT_UB_TRAP" => 4u32.into(),
                    "FAULT_UB_DIV0" => 5u32.into(),
                    "FAULT_UB_SDIV_OVERFLOW" => 6u32.into(),
                    "FAULT_UB_MEMFAULT" => 7u32.into(),
                    "FAULT_EFFECT_UNMODELLED" => 8u32.into(),
                    "FAULT_BUDGET_DEPTH" => 9u32.into(),
                    "FAULT_ENTROPY_EXHAUSTED" => 10u32.into(),
                    _ => uint(cause)?,
                };
                if let Some(width) = procedures::fault_width(self.object, self.function)? {
                    require(
                        BigUint::from(cause.bits()) <= width,
                        "§3.3",
                        "fault literal exceeds its cause width",
                    )?;
                }
                None
            }
            Code::Jump => {
                self.label(&row.args[0], false)?;
                None
            }
            Code::Branch => {
                require(
                    self.bits(&row.args[0])? == 1u32.into(),
                    "§3.3",
                    "branch predicate must have width1",
                )?;
                self.label(&row.args[1], false)?;
                self.label(&row.args[2], false)?;
                None
            }
            Code::JumpRegister => {
                self.bits(&row.args[0])?;
                None
            }
            Code::ReturnValue => {
                require(
                    self.bits(&row.args[0])?
                        == Type::parse(&self.function.groups[1][0].ty)?
                            .bits(self.object.header.ptr_width)?,
                    "§3.3",
                    "retval width differs",
                )?;
                None
            }
            Code::ProcedureReturn => {
                require(
                    self.bits(&row.args[0])?
                        == Type::parse(&self.function.groups[1][0].ty)?
                            .bits(self.object.header.ptr_width)?,
                    "§3.3",
                    "lproc return width differs",
                )?;
                None
            }
            _ => return self.other(row),
        };
        match result {
            Some(t) => self.result(row, vec![t]),
            None => Ok(()),
        }
    }
    fn address(&self, name: &str, lifted: bool) -> Result<(), String> {
        let valid = if lifted {
            self.bits(name)? == self.object.header.ptr_width.into()
        } else {
            self.value(name)? == &Type::Pointer
        };
        require(
            valid,
            "§3.3",
            "memory address type/width differs from pointer",
        )
    }
    fn length(&self, name: &str, lifted: bool) -> Result<(), String> {
        match (lifted, crate::rat::is_canonical_uint_str(name)) {
            (true, true) => require(
                uint(name)?.bits() <= self.object.header.ptr_width,
                "§3.3",
                "memory length literal exceeds ptr width",
            ),
            (false, _) => {
                self.integer_value(name)?;
                Ok(())
            }
            (true, false) => {
                self.bits(name)?;
                Ok(())
            }
        }
    }
    fn label(&self, text: &str, args: bool) -> Result<(), String> {
        let text = text
            .strip_prefix('^')
            .ok_or_else(|| sec_err("§4.8", "expected block label"))?;
        let (name, actuals) = match text.split_once('(') {
            Some((name, rest)) => (name, Some(list(&format!("({rest}"))?)),
            None => (text, None),
        };
        let params = self
            .labels
            .get(name)
            .ok_or_else(|| sec_err("§3.3", "unknown block label"))?;
        if args {
            let actuals = actuals.unwrap_or_else(Vec::new);
            require(
                actuals.len() == params.len(),
                "§3.3",
                "branch block argument count differs",
            )?;
            for (a, p) in actuals.iter().zip(params) {
                require(
                    self.value(a)?.accepts_transfer_to(&Type::parse(&p.ty)?),
                    "§3.3",
                    "branch block argument type differs",
                )?;
            }
        } else {
            require(
                actuals.is_none() && params.is_empty(),
                "§4.8",
                "lifted labels have no block actuals",
            )?;
        }
        Ok(())
    }
    fn terminators(&self) -> Result<(), String> {
        for block in &self.function.blocks {
            let line = block
                .instructions
                .last()
                .ok_or_else(|| sec_err("§16.2", "empty block"))?;
            let row = Row::parse(line, &self.function.kind)?;
            let valid = match self.function.kind.as_str() {
                "oracle" | "lproc" => matches!(
                    row.code,
                    Code::Jump
                        | Code::Branch
                        | Code::JumpRegister
                        | Code::ReturnValue
                        | Code::ProcedureReturn
                        | Code::Fault
                ),
                _ => matches!(row.code, Code::Br | Code::Ret | Code::Trap | Code::Fault),
            };
            require(valid, "§3.6/§4.8", "block lacks a terminating instruction")?;
        }
        Ok(())
    }
    fn other(&mut self, row: &Row) -> Result<(), String> {
        match &row.code {
            Code::Br => {
                match row.args.len() {
                    1 => self.label(&row.args[0], true)?,
                    3 => {
                        require(
                            self.value(&row.args[0])? == &Type::Integer(1u32.into()),
                            "§3.3",
                            "br requires i1",
                        )?;
                        self.label(&row.args[1], true)?;
                        self.label(&row.args[2], true)?;
                    }
                    _ => return Err(sec_err("§4.8", "br operand count differs")),
                }
                Ok(())
            }
            Code::Call
            | Code::Ret
            | Code::Uncall
            | Code::ProcedureCall
            | Code::PermCall
            | Code::OracleCall
            | Code::OracleUncall
            | Code::ActionCall => calls::scalar(self, row),
            Code::RegisterNew
            | Code::RegisterSlice
            | Code::RegisterConcat
            | Code::RegisterFromId
            | Code::RegisterWidth
            | Code::RegisterId
            | Code::RegisterFree
            | Code::Measure(_) => self.register(row),
            Code::Budget => {
                let c = calls::signature(self.object, &row.args[0])?;
                require(c.kind == "oracle", "§3.3", "budget.mint requires oracle")?;
                for a in &row.args[1..] {
                    require(
                        self.value(a)? == &Type::Integer(REGISTER_INTEGER_WIDTH.into()),
                        "§3.3",
                        "budget values require i64",
                    )?;
                }
                self.result(row, vec![Type::Token])
            }
            Code::HostCall => {
                require(
                    row.args[0].starts_with('"') && row.args[0].ends_with('"'),
                    "§4.8",
                    "hostcall service must be STRING",
                )?;
                self.address(&row.args[1], false)?;
                self.integer_value(&row.args[2])?;
                self.result(
                    row,
                    vec![Type::Integer(self.object.header.ptr_width.into())],
                )
            }
            Code::Effect => {
                for a in &row.args[1..] {
                    self.value(a)?;
                }
                Ok(())
            }
            Code::Trap => Ok(()),
            _ => Err(sec_err("§4.8", "unexpected scalar opcode")),
        }
    }
    fn register(&mut self, row: &Row) -> Result<(), String> {
        let q = |s: &str| -> Result<Option<BigUint>, String> {
            match self.value(s)? {
                Type::Register(n) => Ok(n.clone()),
                _ => Err(sec_err("§3.3", "expected qreg")),
            }
        };
        let result = match &row.code {
            Code::RegisterNew => {
                let n = match uint(&row.args[1]) {
                    Ok(n) => {
                        require(!n.is_zero(), "§3.3", "literal qreg count must be positive")?;
                        Some(n)
                    }
                    Err(_) => {
                        require(
                            self.value(&row.args[1])?
                                == &Type::Integer(REGISTER_INTEGER_WIDTH.into()),
                            "§3.3",
                            "runtime qreg width requires i64",
                        )?;
                        None
                    }
                };
                Some(Type::Register(n))
            }
            Code::RegisterSlice => {
                q(&row.args[0])?;
                for a in &row.args[1..] {
                    if uint(a).is_err() {
                        require(
                            self.value(a)? == &Type::Integer(REGISTER_INTEGER_WIDTH.into()),
                            "§3.3",
                            "runtime slice operands require i64",
                        )?;
                    }
                }
                let len = uint(&row.args[2]).ok();
                if let Some(n) = &len {
                    require(
                        !n.is_zero(),
                        "§3.3",
                        "literal slice length must be positive",
                    )?;
                }
                Some(Type::Register(len))
            }
            Code::RegisterConcat => {
                require(row.args[0] != row.args[1], "§3.3", "qreg operands overlap")?;
                let n = match (q(&row.args[0])?, q(&row.args[1])?) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                };
                Some(Type::Register(n))
            }
            Code::RegisterWidth | Code::RegisterId => {
                q(&row.args[0])?;
                Some(Type::Integer(REGISTER_INTEGER_WIDTH.into()))
            }
            Code::RegisterFromId => {
                require(
                    self.value(&row.args[0])? == &Type::Integer(REGISTER_INTEGER_WIDTH.into()),
                    "§3.3",
                    "qreg id requires i64",
                )?;
                Some(Type::Register(None))
            }
            Code::RegisterFree => {
                q(&row.args[0])?;
                None
            }
            Code::Measure(suffix) => {
                let n = match (q(&row.args[0])?, suffix) {
                    (Some(n), None) => n,
                    (n, Some(w)) => {
                        require(
                            n.is_none() || n == Some(w.clone()),
                            "§3.3",
                            "measure width differs",
                        )?;
                        w.clone()
                    }
                    _ => return Err(sec_err("§3.3", "runtime qreg requires measure.iN")),
                };
                require(
                    !n.is_zero() && n <= MAX_INLINE_WIDTH.into(),
                    "§3.3",
                    "measurement width must be1..128",
                )?;
                Some(Type::Integer(n))
            }
            _ => return Err(sec_err("§4.8", "unexpected qreg opcode")),
        };
        match result {
            Some(t) => self.result(row, vec![t]),
            None => Ok(()),
        }
    }
}
