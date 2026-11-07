use super::wires::{Action, Slice};
use super::*;
use crate::json::Value;

pub(super) struct Signature {
    pub kind: String,
    pub groups: Vec<Vec<Type>>,
}
fn number(v: &Value) -> Result<BigUint, String> {
    match v {
        Value::Str(s) | Value::Num(s) => uint(s),
        _ => Err(sec_err("§9.1", "signature size is not an integer")),
    }
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    v.get(key)
        .ok_or_else(|| sec_err("§9.1", format!("missing signature {key}")))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a [Value], String> {
    field(v, key)?
        .as_arr()
        .ok_or_else(|| sec_err("§9.1", "signature field must be an array"))
}
pub(super) fn signature(object: &Object, text: &str) -> Result<Signature, String> {
    let name = text
        .strip_prefix('@')
        .ok_or_else(|| sec_err("§16.2", "callee must be @NAME"))?;
    if let Some(f) = object.function(name) {
        return Ok(Signature {
            kind: f.kind.clone(),
            groups: f
                .groups
                .iter()
                .map(|g| g.iter().map(|p| Type::parse(&p.ty)).collect())
                .collect::<Result<_, _>>()?,
        });
    }
    let s = object
        .imports
        .get(name)
        .ok_or_else(|| sec_err("§9.1", format!("unknown callee @{name}")))?;
    let kind = field(s, "kind")?
        .as_str()
        .ok_or_else(|| sec_err("§9.1", "signature kind must be a string"))?
        .to_string();
    let wires = |key: &str| {
        array(s, key)?
            .iter()
            .map(|v| number(v).map(Type::Wire))
            .collect::<Result<Vec<_>, _>>()
    };
    let types = |key: &str| {
        array(s, key)?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| sec_err("§9.1", "signature type must be a string"))
                    .and_then(Type::parse)
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let mem = || {
        array(s, "mem")?
            .iter()
            .map(|m| {
                Ok(Type::Region(
                    number(field(m, "A")?)?,
                    number(field(m, "W")?)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()
    };
    let groups = match kind.as_str() {
        "action" | "perm_action" => {
            vec![wires("mut")?, wires("in")?, mem()?, types("args")?, vec![]]
        }
        "oracle" => vec![
            wires("in")?,
            vec![Type::Wire(number(field(s, "tgt")?)?)],
            vec![Type::Wire(number(field(s, "fault")?)?)],
            mem()?,
            types("args")?,
        ],
        "cc" | "r" => vec![types("params")?, types("results")?, vec![], vec![], vec![]],
        "lproc" => vec![
            array(s, "params")?
                .iter()
                .map(|v| number(v).map(Type::Location))
                .collect::<Result<_, _>>()?,
            vec![Type::Location(number(field(s, "result")?)?)],
            types("tokens")?,
            types("args")?,
            mem()?,
        ],
        _ => return Err(sec_err("§9.1", "unknown callee kind")),
    };
    Ok(Signature { kind, groups })
}
fn width(ty: &Type, ptr: u64) -> Result<BigUint, String> {
    ty.bits(ptr)
}
fn matching(
    actuals: &[String],
    types: &[Type],
    value: impl Fn(&str) -> Result<Type, String>,
) -> Result<(), String> {
    require(
        actuals.len() == types.len(),
        "§3.3",
        "call argument count differs",
    )?;
    for (a, t) in actuals.iter().zip(types) {
        require(
            value(a)?.accepts_transfer_to(t),
            "§3.3",
            "call argument type differs",
        )?;
    }
    Ok(())
}
fn action_args(scope: &Action, actuals: &str, types: &[Type]) -> Result<(), String> {
    matching(&list(actuals)?, types, |a| {
        scope
            .args
            .get(a)
            .cloned()
            .ok_or_else(|| sec_err("§3.3", "unknown fixed classical argument"))
    })
}
fn action_mem(
    scope: &Action,
    actuals: &str,
    types: &[Type],
    callee_name: &str,
    group: usize,
) -> Result<(), String> {
    let actuals = list(actuals)?;
    require(
        actuals.len() == types.len(),
        "§3.3",
        "memory argument count differs",
    )?;
    require(
        actuals.iter().collect::<BTreeSet<_>>().len() == actuals.len(),
        "§3.3",
        "overlapping region actuals",
    )?;
    for (i, (name, ty)) in actuals.iter().zip(types).enumerate() {
        let (a, w, readonly) = scope
            .regions
            .get(name)
            .ok_or_else(|| sec_err("§3.3", "unknown region actual"))?;
        require(
            ty == &Type::Region(a.clone(), w.clone()),
            "§3.3",
            "memory shape differs",
        )?;
        let formal_readonly = scope
            .object
            .function(callee_name.trim_start_matches('@'))
            .and_then(|f| f.groups[group].get(i))
            .is_some_and(|p| p.suffix.as_deref() == Some("in"));
        let imported_readonly = scope
            .object
            .imports
            .get(callee_name.trim_start_matches('@'))
            .and_then(|s| s.get("mem"))
            .and_then(Value::as_arr)
            .and_then(|m| m.get(i))
            .and_then(|m| m.get("role"))
            .and_then(Value::as_str)
            == Some("in");
        require(
            !readonly || formal_readonly || imported_readonly,
            "§5.1",
            "cannot pass in-region to mut slot",
        )?;
    }
    Ok(())
}
fn wire_group(scope: &Action, actuals: &str, types: &[Type]) -> Result<Vec<Slice>, String> {
    let actuals = list(actuals)?;
    require(
        actuals.len() == types.len(),
        "§3.3",
        "wire argument count differs",
    )?;
    actuals
        .iter()
        .zip(types)
        .map(|(a, t)| {
            let w = scope.wire(a)?;
            require(
                w.width == width(t, scope.object.header.ptr_width)?,
                "§3.3",
                "wire argument width differs",
            )?;
            Ok(w)
        })
        .collect()
}
pub(super) fn action(scope: &mut Action, row: &Row) -> Result<(), String> {
    if row.code == Code::Inplace {
        return inplace(scope, row);
    }
    let sig = signature(scope.object, &row.args[0])?;
    let oracle = row.code == Code::OracleCall;
    require(
        if oracle {
            sig.kind == "oracle"
        } else {
            matches!(sig.kind.as_str(), "action" | "perm_action")
        },
        "§3.1",
        "wrong region call tier/kind",
    )?;
    require(
        scope.function.kind != "perm_action" || sig.kind != "action",
        "§5.1",
        "permutation action calls a general action",
    )?;
    let mut actuals = vec![];
    if oracle {
        require(
            scope.args.get(&row.args[1]) == Some(&Type::Token),
            "§3.3",
            "oracle invocation requires a token",
        )?;
        actuals.extend(wire_group(scope, &row.args[2], &sig.groups[0])?);
        for (i, g) in [(3, 1), (4, 2)] {
            let w = scope.wire(&row.args[i])?;
            require(
                sig.groups[g].len() == 1
                    && w.width == width(&sig.groups[g][0], scope.object.header.ptr_width)?,
                "§3.3",
                "oracle target/fault width differs",
            )?;
            scope.write(&w, false)?;
            actuals.push(w);
        }
        action_mem(scope, &row.args[5], &sig.groups[3], &row.args[0], 3)?;
        action_args(scope, &row.args[6], &sig.groups[4])?;
    } else {
        let mutables = wire_group(scope, &row.args[1], &sig.groups[0])?;
        actuals.extend(mutables.iter().cloned());
        actuals.extend(wire_group(scope, &row.args[2], &sig.groups[1])?);
        action_mem(scope, &row.args[3], &sig.groups[2], &row.args[0], 2)?;
        action_args(scope, &row.args[4], &sig.groups[3])?;
        // Propagate bit-precise writes through calls. A mutable formal may be
        // used purely as a reader; guarding another bit of it is legal.
        if let Some(f) = scope.object.function(row.args[0].trim_start_matches('@')) {
            let writes = write_set(scope.object, f, &mut BTreeSet::new())?;
            for (formal, actual) in f.groups[0].iter().zip(&mutables) {
                for write in writes.iter().filter(|w| w.name == formal.name) {
                    scope.write(
                        &Slice {
                            name: actual.name.clone(),
                            start: &actual.start + &write.start,
                            width: write.width.clone(),
                        },
                        false,
                    )?;
                }
            }
        } else {
            for w in mutables {
                scope.write(&w, false)?;
            }
        }
    }
    Action::disjoint(&actuals)
}
fn inplace(scope: &Action, row: &Row) -> Result<(), String> {
    require(
        matches!(row.args.len(), 4 | 7),
        "§4.8",
        "qinplace requires short or complete form",
    )?;
    require(
        scope.args.get(&row.args[2]) == Some(&Type::Token),
        "§3.3",
        "qinplace requires token",
    )?;
    let x = scope.wire(&row.args[3])?;
    scope.write(&x, false)?;
    let ins = row.args.get(4).map(String::as_str).unwrap_or("()");
    let mem = row.args.get(5).map(String::as_str).unwrap_or("()");
    let args = row.args.get(6).map(String::as_str).unwrap_or("()");
    let mut faults = vec![];
    for name in &row.args[..2] {
        let s = signature(scope.object, name)?;
        require(
            s.kind == "oracle"
                && !s.groups[0].is_empty()
                && s.groups[1].len() == 1
                && s.groups[2].len() == 1,
            "§4.5",
            "qinplace requires two oracle interfaces",
        )?;
        require(
            width(&s.groups[0][0], scope.object.header.ptr_width)? == x.width
                && width(&s.groups[1][0], scope.object.header.ptr_width)? == x.width,
            "§3.3",
            "qinplace input/target width differs",
        )?;
        let mut wires = wire_group(scope, ins, &s.groups[0][1..])?;
        wires.push(x.clone());
        Action::disjoint(&wires)?;
        action_mem(scope, mem, &s.groups[3], name, 3)?;
        action_args(scope, args, &s.groups[4])?;
        faults.push(s.groups[2][0].clone());
    }
    require(
        faults[0] == faults[1],
        "§4.5",
        "qinplace fault widths differ",
    )
}
pub(super) fn scalar(scope: &mut Scalar, row: &Row) -> Result<(), String> {
    if row.code == Code::Ret {
        let types = scope.function.groups[1]
            .iter()
            .map(|p| Type::parse(&p.ty))
            .collect::<Result<Vec<_>, _>>()?;
        return matching(&row.args, &types, |a| scope.value(a).cloned());
    }
    let sig = signature(scope.object, &row.args[0])?;
    match row.code {
        Code::Call | Code::Uncall => {
            require(
                matches!(sig.kind.as_str(), "cc" | "r")
                    && (scope.function.kind != "r" || sig.kind == "r"),
                "§3.1",
                "classical call tier differs",
            )?;
            let (args, results) = if row.code == Code::Uncall {
                (list(&row.args[1])?, list(&row.args[2])?)
            } else {
                (row.args[1..].to_vec(), row.results.clone())
            };
            matching(&args, &sig.groups[0], |a| scope.value(a).cloned())?;
            if row.code == Code::Uncall {
                matching(&results, &sig.groups[1], |a| scope.value(a).cloned())
            } else {
                scope.result(row, sig.groups[1].clone())
            }
        }
        Code::ProcedureCall => {
            require(
                sig.kind == "lproc" && matches!(row.args.len(), 2 | 5),
                "§3.1",
                "lift.call requires lproc and complete argument form",
            )?;
            let args = list(&row.args[1])?;
            require(
                args.len() == sig.groups[0].len(),
                "§3.3",
                "lproc location argument count differs",
            )?;
            for (a, t) in args.iter().zip(&sig.groups[0]) {
                require(
                    scope.bits(a)? == width(t, scope.object.header.ptr_width)?,
                    "§3.3",
                    "lproc location width differs",
                )?;
            }
            for (i, group) in [(2, 2), (3, 3), (4, 4)] {
                let args = if row.args.len() == 2 {
                    vec![]
                } else {
                    list(&row.args[i])?
                };
                matching(&args, &sig.groups[group], |a| scope.value(a).cloned())?;
            }
            scope.result(row, sig.groups[1].clone())
        }
        _ => scalar_region(scope, row, &sig),
    }
}
fn scalar_region(scope: &mut Scalar, row: &Row, sig: &Signature) -> Result<(), String> {
    let oracle = matches!(row.code, Code::OracleCall | Code::OracleUncall);
    require(
        if oracle {
            sig.kind == "oracle"
        } else if row.lifted {
            sig.kind == "perm_action"
        } else {
            matches!(sig.kind.as_str(), "action" | "perm_action")
        },
        "§3.1",
        "region call kind differs",
    )?;
    let (wires, mem, args) = if oracle {
        require(
            scope.value(&row.args[1])? == &Type::Token,
            "§3.3",
            "oracle invocation requires token",
        )?;
        let mut wires = list(&row.args[2])?;
        wires.push(row.args[3].clone());
        wires.push(row.args[4].clone());
        (wires, 5, 6)
    } else {
        let mut wires = list(&row.args[1])?;
        wires.extend(list(&row.args[2])?);
        (wires, 3, 4)
    };
    let expected = if oracle {
        sig.groups[..3].concat()
    } else {
        sig.groups[..2].concat()
    };
    require(
        wires.len() == expected.len(),
        "§3.3",
        "region wire count differs",
    )?;
    require(
        wires.iter().collect::<BTreeSet<_>>().len() == wires.len(),
        "§3.3",
        "overlapping region wire operands",
    )?;
    for (a, t) in wires.iter().zip(expected) {
        let width = width(&t, scope.object.header.ptr_width)?;
        if row.lifted {
            require(
                scope.bits(a)? == width,
                "§3.3",
                "lifted region operand width differs",
            )?;
        } else {
            match scope.value(a)? {
                Type::Register(Some(n)) => {
                    require(n == &width, "§3.3", "qreg region width differs")?
                }
                Type::Register(None) => (),
                _ => return Err(sec_err("§3.3", "CC/R region wires require qreg operands")),
            }
        }
    }
    let group = if oracle { 3 } else { 2 };
    if row.lifted {
        matching(&list(&row.args[mem])?, &sig.groups[group], |a| {
            scope.value(a).cloned()
        })?;
    } else {
        let regs = list(&row.args[mem])?;
        require(
            regs.len() == sig.groups[group].len(),
            "§3.3",
            "region count differs",
        )?;
        for (a, t) in regs.iter().zip(&sig.groups[group]) {
            let Type::Region(address, cell) = t else {
                return Err(sec_err("§3.3", "expected region type"));
            };
            match scope.value(a)? {
                Type::Register(Some(n)) => {
                    let cells = n / cell;
                    require(
                        (n % cell).is_zero()
                            && !cells.is_zero()
                            && (&cells & (&cells - 1u32)).is_zero()
                            && BigUint::from(cells.bits() - 1) == *address,
                        "§3.3",
                        "qreg region width differs",
                    )?;
                }
                Type::Register(None) => (),
                _ => return Err(sec_err("§3.3", "CC/R regions require qreg operands")),
            }
        }
    }
    matching(&list(&row.args[args])?, &sig.groups[group + 1], |a| {
        scope.value(a).cloned()
    })
}
fn write_set(
    object: &Object,
    function: &Function,
    visiting: &mut BTreeSet<String>,
) -> Result<Vec<Slice>, String> {
    if !visiting.insert(function.name.clone()) {
        return Err(sec_err("§5.3", "recursive structured action call"));
    }
    let scope = Action::new(object, function)?;
    let mut writes = vec![];
    for line in
        function.bytes.lines().skip(1).filter(|l| {
            !l.starts_with('^') && !l.starts_with("regs ") && !matches!(*l, "}" | "} {")
        })
    {
        let r = Row::parse(line.strip_suffix(" {").unwrap_or(line), &function.kind)?;
        let indices: &[usize] = match r.code {
            Code::Gate(
                operations::Gate::Cx
                | operations::Gate::Mcx
                | operations::Gate::Rx
                | operations::Gate::Ry
                | operations::Gate::Rz,
            )
            | Code::Uncopy
            | Code::Xor => &[1],
            Code::Gate(operations::Gate::Ccx) | Code::Unload | Code::ExchangeMemory => &[2],
            Code::Gate(operations::Gate::Swap | operations::Gate::Cz) | Code::Exchange => &[0, 1],
            Code::Gate(_) | Code::Free | Code::Return => &[0],
            Code::Inplace => &[3],
            Code::OracleCall => &[3, 4],
            _ => &[],
        };
        for i in indices {
            if let Ok(w) = scope.wire(&r.args[*i]) {
                writes.push(w);
            }
        }
        if r.code == Code::ActionCall {
            if let Some(callee) = object.function(r.args[0].trim_start_matches('@')) {
                let actuals = list(&r.args[1])?;
                for w in write_set(object, callee, visiting)? {
                    if let Some(i) = callee.groups[0].iter().position(|p| p.name == w.name) {
                        if let Some(actual) = actuals.get(i).and_then(|a| scope.wire(a).ok()) {
                            writes.push(Slice {
                                name: actual.name,
                                start: actual.start + w.start,
                                width: w.width,
                            });
                        }
                    }
                }
            }
        }
    }
    visiting.remove(&function.name);
    Ok(writes)
}
