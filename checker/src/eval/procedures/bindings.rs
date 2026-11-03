//! Static origin bindings. Keys describe entry positions, never entry values.
use super::*;

#[derive(Clone, Default)]
pub(super) struct Bindings {
    pub args: HashMap<String, usize>,
    pub regions: HashMap<String, usize>,
    pub key: Vec<usize>,
}
impl Bindings {
    pub fn root(function: &Function) -> Self {
        Self {
            args: function.groups[4]
                .iter()
                .enumerate()
                .map(|(i, p)| (p.name.clone(), i))
                .collect(),
            regions: function.groups[3]
                .iter()
                .enumerate()
                .map(|(i, p)| (p.name.clone(), i))
                .collect(),
            key: vec![],
        }
    }

    pub fn resolve(
        &self,
        root: &Function,
        callee: &Function,
        operands: &[String],
    ) -> Result<Self, EvalError> {
        if !matches!(operands.len(), 2 | 5) {
            return Err(bad(
                "lift.call requires location and fixed-binding operand lists",
            ));
        }
        let mut binding = Self::default();
        for (group, list) in [(2, 2), (3, 3), (4, 4)] {
            let actual = if operands.len() == 2 {
                vec![]
            } else {
                operand_list(&operands[list])?
            };
            let formal = &callee.groups[group];
            if actual.len() != formal.len() {
                return Err(bad("lift.call fixed-binding argument count mismatch"));
            }
            for (name, param) in actual.iter().zip(formal) {
                let origin = if group == 4 {
                    let pos = *self.regions.get(name).ok_or_else(|| {
                        bad("region argument is not a mapped interface or fixed region parameter")
                    })?;
                    let source = &root.groups[3][pos];
                    if source.ty != param.ty
                        || source.suffix.as_deref() != Some("in")
                        || param.suffix.as_deref() != Some("in")
                    {
                        return Err(bad("lift.call region argument type or role mismatch"));
                    }
                    binding.regions.insert(param.name.clone(), pos);
                    pos
                } else {
                    let pos = *self.args.get(name).ok_or_else(|| bad("token/classical argument must be an immutable entry argument or fixed parameter"))?;
                    let source = &root.groups[4][pos];
                    if source.ty != param.ty || (group == 2) != (source.ty == "token") {
                        return Err(bad("lift.call token/classical argument type mismatch"));
                    }
                    binding.args.insert(param.name.clone(), pos);
                    pos
                };
                binding.key.push(origin);
            }
        }
        Ok(binding)
    }
}

pub(super) fn operand_list(text: &str) -> Result<Vec<String>, EvalError> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return Err(bad("lift.call arguments require parenthesised lists"));
    }
    let items = list_items(text);
    if format!("({})", items.join(", ")) != text {
        return Err(bad("lift.call argument list is not canonical"));
    }
    Ok(items)
}

pub(super) fn fixed_scope(
    scope: &mut Scope,
    function: &Function,
    symbol: &Symbol,
    root: &Function,
    binding: &Bindings,
    args: &[Classical],
    ptr: u64,
) -> Result<(), EvalError> {
    let mut used = scope.locations.clone();
    for (key, group) in [("tokens", 2), ("args", 3), ("mem", 4)] {
        let values = symbol
            .signature
            .get(key)
            .and_then(Value::as_arr)
            .ok_or_else(|| bad("lproc tokens/args/mem arrays are mandatory"))?;
        if values.len() != function.groups[group].len() {
            return Err(bad("lproc binding groups disagree with the signature"));
        }
        for (param, declared) in function.groups[group].iter().zip(values) {
            if !used.insert(param.name.clone()) {
                return Err(bad(
                    "all lproc parameter and location names must be distinct",
                ));
            }
            if group == 4 {
                let mut errors = Vec::new();
                if !crate::cert::check_keys(
                    declared,
                    &["A", "W", "role"],
                    &["A", "W", "role"],
                    "lproc.signature.mem",
                    &mut errors,
                ) {
                    return Err(bad("lproc mem signature is not a closed region record"));
                }
                let (a, w) = crate::object::region_aw_of(&param.ty)
                    .ok_or_else(|| bad("lproc mem parameters require region[A,W] types"))?;
                if param.suffix.as_deref() != Some("in")
                    || declared.get("role").and_then(Value::as_str) != Some("in")
                    || declared.get("A").and_then(crate::cert::u64_of) != Some(a)
                    || declared.get("W").and_then(crate::cert::u64_of) != Some(w)
                    || w == 0
                {
                    return Err(bad("lproc region signature type/role mismatch"));
                }
                scope.regions.insert(
                    param.name.clone(),
                    root.groups[3][binding.regions[&param.name]].name.clone(),
                );
                continue;
            }
            let role = if group == 2 { "token" } else { "classical" };
            if param.suffix.as_deref().is_some_and(|s| s != role)
                || declared.as_str() != Some(param.ty.as_str())
                || (group == 2) != (param.ty == "token")
            {
                return Err(bad("lproc token/classical signature or role mismatch"));
            }
            let value = args
                .get(binding.args[&param.name])
                .ok_or_else(|| bad("oracle entry argument is missing"))?;
            match value {
                Classical::Token(t) if group == 2 => {
                    scope.tokens.insert(param.name.clone(), t.clone());
                }
                Classical::Bits(v) if group == 3 => {
                    let w = classical_width(&param.ty, ptr)?;
                    reference_width(w)?;
                    if v.bits() > w {
                        return Err(bad("classical entry argument exceeds its declared width"));
                    }
                    scope.widths.insert(param.name.clone(), w);
                    scope.vals.insert(param.name.clone(), v.clone());
                }
                _ => return Err(bad("oracle entry argument value has the wrong type")),
            }
        }
    }
    scope.in_lproc = true;
    Ok(())
}
