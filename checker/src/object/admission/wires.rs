use super::operations::Gate;
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Slice {
    pub name: String,
    pub start: BigUint,
    pub width: BigUint,
}
impl Slice {
    pub fn overlaps(&self, other: &Self) -> bool {
        self.name == other.name
            && self.start < &other.start + &other.width
            && other.start < &self.start + &self.width
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Origin {
    Interface,
    Allocated,
    Borrowed,
    Copied,
    Loaded,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bundle {
    width: BigUint,
    origin: Origin,
}
use super::predicates::{Effect, Word as Predicates};
struct Frame {
    row: Row,
    start: usize,
    wires: BTreeMap<String, Bundle>,
    predicates: Predicates,
    effect: Effect,
}
pub(super) struct Action<'a> {
    pub object: &'a Object,
    pub function: &'a Function,
    wires: BTreeMap<String, Bundle>,
    readonly: BTreeSet<String>,
    pub regions: BTreeMap<String, (BigUint, BigUint, bool)>,
    pub args: BTreeMap<String, Type>,
    predicates: Predicates,
    guards: Vec<Slice>,
}
impl<'a> Action<'a> {
    pub(super) fn new(object: &'a Object, function: &'a Function) -> Result<Self, String> {
        let mut scope = Self {
            object,
            function,
            wires: BTreeMap::new(),
            readonly: BTreeSet::new(),
            regions: BTreeMap::new(),
            args: BTreeMap::new(),
            predicates: Predicates::default(),
            guards: vec![],
        };
        let mut names = BTreeSet::new();
        for (group, params) in function.groups.iter().enumerate() {
            for p in params {
                require(
                    names.insert(p.name.clone()),
                    "§3.3",
                    "duplicate interface name",
                )?;
                match Type::parse(&p.ty)? {
                    Type::Wire(width) => {
                        require(group < 2, "§16.2", "wire parameter in wrong action group")?;
                        scope.wires.insert(
                            p.name.clone(),
                            Bundle {
                                width,
                                origin: Origin::Interface,
                            },
                        );
                        if group == 1 {
                            scope.readonly.insert(p.name.clone());
                        }
                    }
                    Type::Region(a, w) => {
                        require(
                            group == 2,
                            "§16.2",
                            "region parameter in wrong action group",
                        )?;
                        scope
                            .regions
                            .insert(p.name.clone(), (a, w, p.suffix.as_deref() == Some("in")));
                    }
                    ty => {
                        require(
                            group == 3
                                && matches!(
                                    ty,
                                    Type::Integer(_) | Type::Pointer | Type::Float | Type::Token
                                ),
                            "§16.2",
                            "classical parameter in wrong action group",
                        )?;
                        scope.args.insert(p.name.clone(), ty);
                    }
                }
            }
        }
        // §4.6: qregion declarations take effect at function entry, even in a
        // zero-trip loop or an unselected arm.
        for line in function.bytes.lines().skip(1).filter(|l| {
            !l.starts_with('^') && !l.starts_with("regs ") && !matches!(*l, "}" | "} {")
        }) {
            let row = Row::parse(line.strip_suffix(" {").unwrap_or(line), &function.kind)?;
            if row.code == Code::Region {
                scope.region(&row)?;
            }
        }
        Ok(scope)
    }
    pub fn validate(object: &'a Object, function: &'a Function) -> Result<(), String> {
        let mut scope = Self::new(object, function)?;
        let loops = function
            .bytes
            .lines()
            .filter(|l| l.starts_with("qloop "))
            .map(|l| {
                let tokens = tokenise(l);
                uint(&tokens[1])
            })
            .collect::<Result<Vec<_>, _>>()?;
        require(
            loops == function.bounds,
            "§5.3",
            "bounds must list every loop count in source order",
        )?;
        let entry = scope.wires.clone();
        scope.source()?;
        require(
            scope.predicates.is_empty(),
            "§3.5",
            "unbalanced predicate stack at function exit",
        )?;
        require(
            scope.wires == entry,
            "§3.3",
            "local wire lifetime escapes function or interface wire consumed",
        )?;
        Ok(())
    }
    pub fn wire(&self, text: &str) -> Result<Slice, String> {
        let (name, index) = if let Some((name, rest)) = text.split_once('[') {
            let digits = rest
                .strip_suffix(']')
                .ok_or_else(|| sec_err("§16.2", "malformed indexed operand"))?;
            (name, Some(uint(digits)?))
        } else {
            (text, None)
        };
        let b = self
            .wires
            .get(name)
            .ok_or_else(|| sec_err("§3.3", format!("wire {name} is not live")))?;
        let (start, width) = match index {
            Some(i) => {
                require(i < b.width, "§3.3", "wire index outside live bundle")?;
                (i, 1u32.into())
            }
            None => (BigUint::zero(), b.width.clone()),
        };
        Ok(Slice {
            name: name.into(),
            start,
            width,
        })
    }
    fn single(&self, text: &str) -> Result<Slice, String> {
        let w = self.wire(text)?;
        require(
            w.width == 1u32.into(),
            "§3.3",
            "operand must denote one wire",
        )?;
        Ok(w)
    }
    pub fn disjoint(wires: &[Slice]) -> Result<(), String> {
        for (i, w) in wires.iter().enumerate() {
            for other in &wires[..i] {
                require(!w.overlaps(other), "§3.3", "overlapping wire operands")?;
            }
        }
        Ok(())
    }
    pub fn write(&self, wire: &Slice, read_only_ok: bool) -> Result<(), String> {
        require(
            read_only_ok || !self.readonly.contains(&wire.name),
            "§5.1",
            "operation writes an in-role bundle",
        )?;
        for guard in self.guards.iter().chain(self.predicates.wires()) {
            require(
                !wire.overlaps(guard),
                "§3.3",
                "operation writes/consumes an active predicate or selector",
            )?;
        }
        Ok(())
    }
    fn fresh(&mut self, name: &str, width: BigUint, origin: Origin) -> Result<(), String> {
        require(
            valid_name(name)
                && !self.wires.contains_key(name)
                && !self.regions.contains_key(name)
                && !self.args.contains_key(name),
            "§3.3",
            "result must be a fresh local wire name",
        )?;
        self.wires.insert(name.into(), Bundle { width, origin });
        Ok(())
    }
    fn release(&mut self, text: &str, origins: &[Origin]) -> Result<(), String> {
        let wire = self.wire(text)?;
        self.write(&wire, false)?;
        let bundle = &self.wires[&wire.name];
        require(
            wire.start.is_zero() && wire.width == bundle.width && origins.contains(&bundle.origin),
            "§3.3",
            "release requires a whole matching local allocation",
        )?;
        self.wires.remove(&wire.name);
        Ok(())
    }
    fn region(&mut self, row: &Row) -> Result<(), String> {
        let name = &row.args[0];
        require(
            valid_name(name)
                && !self.regions.contains_key(name)
                && !self.wires.contains_key(name)
                && !self.args.contains_key(name),
            "§3.3",
            "region requires a fresh name",
        )?;
        let a = uint(&row.args[1])?;
        let w = positive(&row.args[2])?;
        if let Some(image) = row.args.get(3) {
            let name = image
                .strip_prefix('@')
                .ok_or_else(|| sec_err("§4.6", "region image must be a data reference"))?;
            let hex = self
                .object
                .data
                .get(name)
                .ok_or_else(|| sec_err("§4.6", "unknown region image"))?;
            let bits = BigUint::from(hex.len()) * 4u32;
            // Test the exact product without materialising 2^A for huge A.
            require(
                !bits.is_zero() && (&bits % &w).is_zero(),
                "§4.6",
                "region image size differs",
            )?;
            let cells = &bits / &w;
            require(
                !cells.is_zero()
                    && (&cells & (&cells - 1u32)).is_zero()
                    && BigUint::from(cells.bits() - 1) == a,
                "§4.6",
                "region image size differs",
            )?;
        }
        self.regions.insert(name.clone(), (a, w, false));
        Ok(())
    }
    fn source(&mut self) -> Result<(), String> {
        let lines: Vec<_> = self.function.bytes.lines().skip(1).collect();
        self.source_lines(&lines, true)
    }
    fn source_lines(&mut self, lines: &[&str], recheck: bool) -> Result<(), String> {
        let mut frames: Vec<Frame> = vec![];
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with('^') || line.starts_with("regs ") {
                continue;
            }
            if matches!(*line, "}" | "} {") {
                let mut frame = frames
                    .pop()
                    .ok_or_else(|| sec_err("§16.2", "unmatched brace"))?;
                match frame.row.code {
                    Code::If => {
                        let selector = self.single(&frame.row.args[0])?;
                        self.predicates.pop(&selector)?;
                        require(
                            same_predicates(&self.predicates, &frame.predicates),
                            "§3.5",
                            "unbalanced predicates in qif arm",
                        )?;
                        if *line == "} {" {
                            self.predicates.push(selector);
                            frame.start = index;
                            frames.push(frame);
                        } else {
                            self.guards.pop();
                        }
                    }
                    Code::Loop => {
                        let count = uint(&frame.row.args[0])?;
                        require(
                            same_live(&frame.wires, &self.wires),
                            "§3.3",
                            "loop entry and exit live names/types differ",
                        )?;
                        if recheck && count > 1u32.into() {
                            let wires = self.wires.clone();
                            let predicates = self.predicates.clone();
                            self.source_lines(&lines[frame.start + 1..index], false)?;
                            self.wires = wires;
                            self.predicates = predicates;
                        }
                        require(
                            count.is_zero() || self.predicates.agrees(&frame.predicates),
                            "§3.5/§4.6",
                            "loop body altered predicate stack",
                        )?;
                        let effect = frame.effect.counted(&count)?;
                        self.predicates = effect.apply(&frame.predicates)?;
                        if let Some(parent) = frames.last_mut() {
                            parent.effect.then(&effect)?;
                        }
                        if count.is_zero() {
                            self.wires = frame.wires;
                        }
                        if frame.row.args.len() == 3 {
                            self.guards.pop();
                        }
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            let row = Row::parse(line.strip_suffix(" {").unwrap_or(line), &self.function.kind)?;
            if matches!(row.code, Code::If | Code::Loop) {
                let frame = Frame {
                    row: row.clone(),
                    start: index,
                    wires: self.wires.clone(),
                    predicates: self.predicates.clone(),
                    effect: Effect::default(),
                };
                match row.code {
                    Code::If => {
                        let selector = self.single(&row.args[0])?;
                        self.guards.push(selector.clone());
                        self.predicates.push(selector);
                    }
                    Code::Loop => {
                        let count = uint(&row.args[0])?;
                        require(
                            row.args.len() == 1 || (row.args.len() == 3 && row.args[1] == "while"),
                            "§4.8",
                            "invalid qloop operand form",
                        )?;
                        if row.args.len() == 3 {
                            let condition = self.wire(&row.args[2])?;
                            require(
                                condition.width == count,
                                "§3.3",
                                "while condition width must equal count",
                            )?;
                            self.guards.push(condition);
                        }
                    }
                    _ => unreachable!(),
                }
                frames.push(frame);
            } else {
                self.row(&row)?;
                if matches!(row.code, Code::Pred | Code::Unpred) {
                    if let Some(frame) = frames.last_mut() {
                        frame
                            .effect
                            .then(&Effect::instruction(&row, self.single(&row.args[0])?))?;
                    }
                }
            }
        }
        require(frames.is_empty(), "§16.2", "unterminated structured body")
    }
    fn row(&mut self, row: &Row) -> Result<(), String> {
        let controls = row
            .controls
            .iter()
            .map(|s| self.single(s))
            .collect::<Result<Vec<_>, _>>()?;
        let saved = self.predicates.clone();
        for c in controls {
            self.predicates.push(c);
        }
        let result = self.operation(row);
        self.predicates = saved_if_call(row, saved, self.predicates.clone());
        result
    }
    fn operation(&mut self, row: &Row) -> Result<(), String> {
        match &row.code {
            Code::Gate(gate) => self.gate(row, *gate),
            Code::Alloc | Code::Borrow => {
                require(valid_name(&row.args[0]), "§16.2", "invalid target class")?;
                self.fresh(
                    &row.results[0],
                    positive(&row.args[1])?,
                    if row.code == Code::Alloc {
                        Origin::Allocated
                    } else {
                        Origin::Borrowed
                    },
                )
            }
            Code::Free => self.release(&row.args[0], &[Origin::Allocated]),
            Code::Return => self.release(&row.args[0], &[Origin::Borrowed]),
            Code::Copy => {
                let source = self.wire(&row.args[0])?;
                self.fresh(&row.results[0], source.width, Origin::Copied)
            }
            Code::Uncopy | Code::Xor | Code::Exchange => {
                let a = self.wire(&row.args[0])?;
                let b = self.wire(&row.args[1])?;
                Self::disjoint(&[a.clone(), b.clone()])?;
                require(a.width == b.width, "§3.3", "bulk operand widths differ")?;
                self.write(&b, false)?;
                if row.code == Code::Exchange {
                    self.write(&a, false)?;
                }
                if row.code == Code::Uncopy {
                    self.release(&row.args[1], &[Origin::Copied, Origin::Allocated])?;
                }
                Ok(())
            }
            Code::Pred => {
                let w = self.single(&row.args[0])?;
                self.predicates.push(w);
                Ok(())
            }
            Code::Unpred => {
                let w = self.single(&row.args[0])?;
                self.predicates.pop(&w)
            }
            Code::Region => Ok(()),
            Code::Load | Code::Unload | Code::Store | Code::ExchangeMemory => self.memory(row),
            Code::ActionCall | Code::OracleCall | Code::Inplace => calls::action(self, row),
            _ => Err(sec_err("§4.8", "unexpected action opcode")),
        }
    }
    fn gate(&self, row: &Row, gate: Gate) -> Result<(), String> {
        let (wires, writes) = match gate {
            Gate::Rx | Gate::Ry | Gate::Rz => {
                let angle = &row.args[0];
                if self.args.get(angle) != Some(&Type::Float) {
                    super::super::lexical::float(angle).map_err(|e| sec_err("§16.1", e))?;
                }
                (vec![self.single(&row.args[1])?], vec![0])
            }
            Gate::Mcx => {
                let mut wires = list(&row.args[0])?
                    .iter()
                    .map(|a| self.wire(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let target = wires.len();
                wires.push(self.single(&row.args[1])?);
                (wires, vec![target])
            }
            Gate::Cx => (
                row.args
                    .iter()
                    .map(|a| self.single(a))
                    .collect::<Result<Vec<_>, _>>()?,
                vec![1],
            ),
            Gate::Ccx => (
                row.args
                    .iter()
                    .map(|a| self.single(a))
                    .collect::<Result<Vec<_>, _>>()?,
                vec![2],
            ),
            Gate::Swap | Gate::Cz => (
                row.args
                    .iter()
                    .map(|a| self.single(a))
                    .collect::<Result<Vec<_>, _>>()?,
                vec![0, 1],
            ),
            _ => (vec![self.single(&row.args[0])?], vec![0]),
        };
        Self::disjoint(&wires)?;
        require(
            self.function.kind != "perm_action"
                || matches!(
                    gate,
                    Gate::X | Gate::Cx | Gate::Ccx | Gate::Mcx | Gate::Swap
                ),
            "§5.1",
            "non-permutation gate in perm_action",
        )?;
        let diagonal = matches!(
            gate,
            Gate::S | Gate::Sdg | Gate::T | Gate::Tdg | Gate::Cz | Gate::Rz
        );
        for i in writes {
            self.write(&wires[i], diagonal)?;
        }
        Ok(())
    }
    fn memory(&mut self, row: &Row) -> Result<(), String> {
        let (a, w, readonly) = self
            .regions
            .get(&row.args[0])
            .cloned()
            .ok_or_else(|| sec_err("§3.3", "unknown region"))?;
        let mut wires = vec![];
        if a.is_zero() {
            require(
                row.args[1] == "()",
                "§3.3",
                "zero-address region requires ()",
            )?;
        } else {
            let address = self.wire(&row.args[1])?;
            require(address.width == a, "§3.3", "region address width differs")?;
            wires.push(address);
        }
        if row.code == Code::Load {
            self.fresh(&row.results[0], w, Origin::Loaded)?;
        } else {
            let value = self.wire(&row.args[2])?;
            require(value.width == w, "§3.3", "region value width differs")?;
            wires.push(value.clone());
            match row.code {
                Code::Unload => self.release(&row.args[2], &[Origin::Loaded, Origin::Allocated])?,
                Code::ExchangeMemory => {
                    self.write(&value, false)?;
                    require(!readonly, "§5.1", "cannot exchange in-region")?;
                }
                Code::Store => require(!readonly, "§5.1", "cannot store in-region")?,
                _ => unreachable!(),
            }
        }
        Self::disjoint(&wires)
    }
}
fn saved_if_call(row: &Row, before: Predicates, after: Predicates) -> Predicates {
    if row.controls.is_empty() {
        after
    } else {
        before
    }
}
fn same_live(a: &BTreeMap<String, Bundle>, b: &BTreeMap<String, Bundle>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(n, v)| b.get(n).is_some_and(|w| w.width == v.width))
}
fn same_predicates(a: &Predicates, b: &Predicates) -> bool {
    a.agrees(b)
}
pub(super) fn valid_name(text: &str) -> bool {
    let mut b = text.bytes();
    b.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && b.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.'))
}
