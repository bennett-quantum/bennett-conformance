//! Global definition types and CFG checks, independently derived from §§3.3–3.4.
use super::operations::Cast;
use super::*;

pub(super) fn definitions(scope: &mut Scalar, rows: &[Row]) -> Result<(), String> {
    let defined: BTreeSet<_> = rows
        .iter()
        .flat_map(|r| r.results.iter())
        .cloned()
        .collect();
    for row in rows.iter().filter(|r| r.code == Code::Ret) {
        require(
            row.args.len() == scope.function.groups[1].len(),
            "§3.4",
            "return value count differs",
        )?;
        let return_bindings = row.args.iter().zip(&scope.function.groups[1]);
        for (name, param) in return_bindings.filter(|(name, _)| defined.contains(*name)) {
            declare(scope, name, Type::parse(&param.ty)?)?;
        }
    }
    let mut pending: Vec<_> = rows.iter().filter(|r| !r.results.is_empty()).collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut next = vec![];
        for row in pending {
            match infer(scope, row)? {
                Some(types) => {
                    require(
                        types.len() == row.results.len(),
                        "§3.4",
                        "definition result count differs",
                    )?;
                    for (name, ty) in row.results.iter().zip(types) {
                        declare(scope, name, ty)?;
                    }
                }
                None => next.push(row),
            }
        }
        require(
            next.len() < before,
            "§3.4",
            "cannot resolve types of forward definitions",
        )?;
        pending = next;
    }
    Ok(())
}
pub(super) fn declare(scope: &mut Scalar, name: &str, ty: Type) -> Result<(), String> {
    if let Type::Integer(n) = &ty {
        require(
            n <= &MAX_INLINE_WIDTH.into(),
            "§3.3",
            "classical result width exceeds i128",
        )?;
    }
    match scope.values.get(name) {
        Some(existing) => require(
            existing == &ty,
            "§3.4",
            "definitions of one name have incompatible types",
        ),
        None => {
            scope.values.insert(name.into(), ty);
            Ok(())
        }
    }
}
fn infer(scope: &Scalar, row: &Row) -> Result<Option<Vec<Type>>, String> {
    let lookup = |i: usize| scope.values.get(&row.args[i]).cloned();
    let result = match &row.code {
        Code::Const(n) | Code::LoadScalar(n) => Some(Type::Integer(n.clone())),
        Code::Compare(_) => Some(Type::Integer(1u32.into())),
        Code::Cast(Cast::IntToPtr, _) | Code::AllocateScalar => Some(Type::Pointer),
        Code::Cast(_, Some(n)) => Some(Type::Integer(n.clone())),
        Code::Binary(_) => lookup(0),
        Code::Select => lookup(1),
        Code::Call => {
            return Ok(Some(
                calls::signature(scope.object, &row.args[0])?.groups[1].clone(),
            ))
        }
        Code::Budget => Some(Type::Token),
        Code::HostCall => Some(Type::Integer(scope.object.header.ptr_width.into())),
        Code::RegisterNew => Some(Type::Register(uint(&row.args[1]).ok())),
        Code::RegisterSlice => Some(Type::Register(uint(&row.args[2]).ok())),
        Code::RegisterConcat => match (lookup(0), lookup(1)) {
            (Some(Type::Register(a)), Some(Type::Register(b))) => {
                Some(Type::Register(match (a, b) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                }))
            }
            (Some(_), Some(_)) => return Err(sec_err("§3.3", "concat requires qreg operands")),
            _ => None,
        },
        Code::RegisterFromId => Some(Type::Register(None)),
        Code::RegisterWidth | Code::RegisterId => {
            Some(Type::Integer(REGISTER_INTEGER_WIDTH.into()))
        }
        Code::Measure(Some(n)) => Some(Type::Integer(n.clone())),
        Code::Measure(None) => match lookup(0) {
            Some(Type::Register(Some(n))) => Some(Type::Integer(n)),
            Some(_) => {
                return Err(sec_err(
                    "§3.3",
                    "measure requires a static qreg or width suffix",
                ))
            }
            None => None,
        },
        _ => return Err(sec_err("§4.8", "unexpected classical result opcode")),
    };
    Ok(result.map(|t| vec![t]))
}
pub(super) fn repeated_definitions(function: &Function, rows: &[Row]) -> bool {
    let mut names: BTreeSet<_> = function.groups[0].iter().map(|p| p.name.clone()).collect();
    for block in &function.blocks {
        for param in &block.params {
            if !names.insert(param.name.clone()) {
                return true;
            }
        }
    }
    rows.iter()
        .flat_map(|r| &r.results)
        .any(|name| !names.insert(name.clone()))
}
