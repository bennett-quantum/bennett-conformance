use super::*;
use crate::eval::{list_items, parse_blocks};
use crate::realisation::{bundle, one_wire};
impl Trace {
    fn local_regions(
        &mut self,
        docs: &LoadedDocs,
        call: &Call,
        wires: &mut HashMap<String, Vec<usize>>,
        hash: &str,
    ) -> Result<Vec<(Vec<usize>, Vec<usize>, Value)>> {
        let function = docs
            .object
            .function(&call.context.symbol)
            .ok_or_else(|| ClaimOutcome::unchecked("construction function absent"))?;
        let declarations = crate::eval::find_opcodes(function)
            .into_iter()
            .filter(|v| v.first().map(String::as_str) == Some("qregion"));
        let mut locals = vec![];
        let mut ordinal = 0;
        for tokens in declarations {
            let a = tokens[2]
                .parse::<u64>()
                .map_err(|_| ClaimOutcome::unchecked("qregion A exceeds host integer"))?;
            let w = tokens[3]
                .parse::<u64>()
                .map_err(|_| ClaimOutcome::unchecked("qregion W exceeds host integer"))?;
            let bits = self.region(a, w)?;
            if wires.insert(tokens[1].clone(), bits.clone()).is_some() {
                return invalid("local region name aliases another allocation");
            }
            let allocation = address(&call.context, hash, "$construct", &[], ordinal, None);
            let addr = address(
                &call.context,
                hash,
                "$construct",
                &[],
                ordinal,
                Some(&allocation),
            );
            ordinal += 1;
            self.push(addr, Kind::Allocate(bits.clone()))?;
            let image = if let Some(name) = tokens.get(4) {
                docs.object
                    .data
                    .get(name.trim_start_matches('@'))
                    .ok_or_else(|| ClaimOutcome::unchecked("qregion image absent"))?
                    .clone()
            } else {
                String::new()
            };
            let mut ones = vec![];
            for bit in 0..bits.len() {
                let on = if image.is_empty() {
                    false
                } else {
                    u8::from_str_radix(&image[2 * (bit / 8)..2 * (bit / 8) + 2], 16)
                        .map_err(|_| ClaimOutcome::unchecked("invalid image bytes"))?
                        & (1 << (bit % 8))
                        != 0
                };
                if on {
                    let addr = address(
                        &call.context,
                        hash,
                        "$construct",
                        &[],
                        ordinal,
                        Some(&allocation),
                    );
                    ordinal += 1;
                    self.push(
                        addr.clone(),
                        Kind::Gate(gate("x", vec![], vec![bits[bit]], &call.context, &addr, 0)),
                    )?;
                    ones.push(bits[bit]);
                }
            }
            locals.push((bits, ones, allocation));
        }
        Ok(locals)
    }
    pub(super) fn collect(
        &mut self,
        docs: &LoadedDocs,
        call: Call,
        mut wires: HashMap<String, Vec<usize>>,
        inherited: Vec<usize>,
        depth: usize,
    ) -> Result<()> {
        if depth > 64 {
            return pending("host limit: linked construction invocation depth exceeds 64");
        }
        let context = &call.context;
        if context.adjoint {
            return pending(
                "source-adjoint construction with allocation lifetimes remains unavailable",
            );
        }
        let f = docs
            .object
            .function(&context.symbol)
            .ok_or_else(|| ClaimOutcome::unchecked("linked function absent"))?;
        if !matches!(f.kind.as_str(), "action" | "perm_action") {
            return pending(format!(
                "public-leaf construction for {} @{} remains unavailable",
                f.kind, f.name
            ));
        }
        let hash = docs
            .object
            .symbol(&context.symbol)
            .ok_or_else(|| ClaimOutcome::unchecked("function symbol absent"))?
            .interface_hash
            .clone();
        self.push(Value::Null, Kind::Enter(call.clone()))?;
        let locals = self.local_regions(docs, &call, &mut wires, &hash)?;
        let mut rows = vec![];
        for (block, stmts) in parse_blocks(f) {
            crate::realisation::expand_rows(&stmts, &block, 0, &mut vec![], &mut rows, self.max)?;
        }
        let mut predicates = inherited.clone();
        for row in rows {
            let mut controls = predicates.clone();
            let mut adjoint = false;
            let tokens =
                crate::realisation::prefixes(&row.tokens, &mut controls, &wires, &mut adjoint)?;
            let mut distinct = HashSet::new();
            controls.retain(|x| distinct.insert(*x));
            let point = format!("{}:{}", row.block, row.ordinal);
            let addr = address(context, &hash, &point, &row.iteration, row.generated, None);
            let (lhs, op, args) = if tokens.get(1).map(String::as_str) == Some("=") {
                (Some(tokens[0].as_str()), tokens[2].as_str(), &tokens[3..])
            } else {
                (None, tokens[0].as_str(), &tokens[1..])
            };
            match op {
                "qregion" => {}
                "qpred" => predicates.push(one_wire(&args[0], &wires)?),
                "qunpred" => {
                    if predicates.len() <= inherited.len()
                        || predicates.pop() != Some(one_wire(&args[0], &wires)?)
                    {
                        return invalid("predicate stack not balanced");
                    }
                }
                "qalloc" => {
                    if adjoint {
                        return pending("adjoint allocation construction remains unavailable");
                    }
                    let name =
                        lhs.ok_or_else(|| ClaimOutcome::unchecked("qalloc destination absent"))?;
                    let bits = self.wires(args[1].parse().map_err(|_| {
                        ClaimOutcome::unchecked("qalloc width exceeds host integer")
                    })?)?;
                    if wires.insert(name.into(), bits.clone()).is_some() {
                        return invalid("qalloc reuses a live allocation name");
                    }
                    self.push(addr, Kind::Allocate(bits))?;
                }
                "qfree" => {
                    let bits = wires
                        .remove(&args[0])
                        .ok_or_else(|| ClaimOutcome::unchecked("qfree allocation absent"))?;
                    self.push(addr, Kind::Release(bits))?;
                }
                "action_call" => {
                    let name = args[0].trim_start_matches('@');
                    let iface = docs
                        .object
                        .interface(name)
                        .map_err(ClaimOutcome::unchecked)?;
                    let actuals: Vec<_> =
                        [1, 2].iter().flat_map(|i| list_items(&args[*i])).collect();
                    if actuals.len() != iface.bundles.len() {
                        return invalid("call bundle mapping arity differs");
                    }
                    let mut child_wires = HashMap::new();
                    let mut bundles = vec![];
                    for (b, actual) in iface.bundles.iter().zip(actuals) {
                        let bits = bundle(&actual, &wires)?;
                        if bits.len() != b.width as usize {
                            return invalid("call bundle mapping width differs");
                        }
                        child_wires.insert(b.name.clone(), bits.clone());
                        bundles.push(bits);
                    }
                    let actuals = list_items(&args[3]);
                    if actuals.len() != iface.regions.len() {
                        return invalid("call region mapping arity differs");
                    }
                    let mut regions = vec![];
                    for (r, actual) in iface.regions.iter().zip(actuals) {
                        let bits = wires
                            .get(&actual)
                            .ok_or_else(|| {
                                ClaimOutcome::unchecked("call region allocation absent")
                            })?
                            .clone();
                        let expected = 1u64
                            .checked_shl(u32::try_from(r.a).unwrap_or(u32::MAX))
                            .and_then(|n| n.checked_mul(r.w));
                        if expected != Some(bits.len() as u64) {
                            return invalid("call region shape differs");
                        }
                        child_wires.insert(r.name.clone(), bits.clone());
                        regions.push(bits);
                    }
                    let g = gate("x", vec![], vec![], context, &addr, 0);
                    let child = Context {
                        symbol: name.into(),
                        controls: controls.len() as u64,
                        invocation: crate::realisation::child_invocation(
                            tokens,
                            context,
                            &docs.object,
                        )?,
                        key: g.path,
                        adjoint,
                    };
                    self.collect(
                        docs,
                        Call {
                            context: child,
                            regions,
                            bundles,
                        },
                        child_wires,
                        controls,
                        depth + 1,
                    )?;
                }
                "x" | "cx" | "ccx" | "mcx" | "swap" | "h" | "s" | "sdg" | "t" | "tdg" | "cz"
                | "rx" | "ry" | "rz" => {
                    let inherited_count = controls.len();
                    let mut targets = vec![];
                    match op {
                        "cx" | "ccx" => {
                            for actual in &args[..args.len() - 1] {
                                controls.push(one_wire(actual, &wires)?);
                            }
                            targets.push(one_wire(args.last().unwrap(), &wires)?);
                        }
                        "mcx" => {
                            for actual in list_items(&args[0]) {
                                controls.extend(bundle(&actual, &wires)?);
                            }
                            targets.push(one_wire(&args[1], &wires)?);
                        }
                        "swap" | "cz" => {
                            for actual in args {
                                targets.push(one_wire(actual, &wires)?);
                            }
                        }
                        "rx" | "ry" | "rz" => targets.push(one_wire(&args[1], &wires)?),
                        _ => targets.push(one_wire(&args[0], &wires)?),
                    }
                    if controls
                        .iter()
                        .chain(&targets)
                        .copied()
                        .collect::<HashSet<_>>()
                        .len()
                        != controls.len() + targets.len()
                    {
                        return invalid("gate controls/targets overlap");
                    }
                    let own = controls.len() - inherited_count;
                    let mut g = gate(op, controls, targets, context, &addr, own);
                    g.adjoint = adjoint;
                    if matches!(op, "rx" | "ry" | "rz") {
                        g.angle_bits = Some(crate::realisation::resolve_angle(
                            &args[0],
                            context,
                            &docs.object,
                        )?);
                    }
                    self.push(addr, Kind::Gate(g))?;
                }
                _ => {
                    return pending(format!(
                        "public expansion of {op} in @{} is unavailable for prefix reconstruction",
                        context.symbol
                    ))
                }
            }
        }
        if predicates != inherited {
            return invalid("predicate stack was not restored");
        }
        let mut ordinal = 0;
        for (bits, ones, allocation) in locals.into_iter().rev() {
            for bit in ones.into_iter().rev() {
                let addr = address(context, &hash, "$release", &[], ordinal, Some(&allocation));
                ordinal += 1;
                self.push(
                    addr.clone(),
                    Kind::Gate(gate("x", vec![], vec![bit], context, &addr, 0)),
                )?;
            }
            let addr = address(context, &hash, "$release", &[], ordinal, Some(&allocation));
            ordinal += 1;
            self.push(addr, Kind::Release(bits))?;
        }
        self.push(Value::Null, Kind::Leave(call))?;
        Ok(())
    }
}
