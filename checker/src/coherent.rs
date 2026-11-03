//! Exact reference basis replay in Q(sqrt(2), i), from ISA §4.4.
//! This retains phase and interference throughout the complete action call
//! tree. No target, producer matrix, reference interpreter or float is used.
use crate::cert::{ClaimOutcome, Invocation};
use crate::domain::Tuple;
use crate::eval::{tokenise, Classical, EvalError};
use crate::fault::Reject;
use crate::json::{s, Value};
use crate::object::{Interface, Object, Symbol};
use crate::rat::Rat;
use crate::realisation::{reference_gates, Context};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::collections::HashSet;

#[derive(Clone, Default, PartialEq, Eq)]
struct Amp {
    // Each pair denotes a + b sqrt(2).
    re: [Rat; 2],
    im: [Rat; 2],
}
impl Amp {
    fn one() -> Self {
        Self {
            re: [Rat::one(), Rat::zero()],
            ..Self::default()
        }
    }
    fn add(&self, rhs: &Self) -> Self {
        Self {
            re: std::array::from_fn(|i| &self.re[i] + &rhs.re[i]),
            im: std::array::from_fn(|i| &self.im[i] + &rhs.im[i]),
        }
    }
    fn neg(&self) -> Self {
        Self {
            re: std::array::from_fn(|i| -&self.re[i]),
            im: std::array::from_fn(|i| -&self.im[i]),
        }
    }
    fn mul(&self, rhs: &Self) -> Self {
        fn product(a: &[Rat; 2], b: &[Rat; 2]) -> [Rat; 2] {
            [
                &a[0] * &b[0] + Rat::from_integer(2.into()) * &a[1] * &b[1],
                &a[0] * &b[1] + &a[1] * &b[0],
            ]
        }
        let rr = product(&self.re, &rhs.re);
        let ii = product(&self.im, &rhs.im);
        let ri = product(&self.re, &rhs.im);
        let ir = product(&self.im, &rhs.re);
        Self {
            re: std::array::from_fn(|i| &rr[i] - &ii[i]),
            im: std::array::from_fn(|i| &ri[i] + &ir[i]),
        }
    }
    fn is_zero(&self) -> bool {
        self.re.iter().chain(&self.im).all(Zero::is_zero)
    }
}

pub(crate) fn needed(object: &Object, root: &Symbol) -> bool {
    let mut pending = vec![root.name.clone()];
    let mut seen = HashSet::new();
    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(f) = object.function(&name) else {
            continue;
        };
        for line in f.blocks.iter().flat_map(|b| &b.instructions) {
            let tokens = tokenise(line);
            let mut at = 0;
            loop {
                match tokens.get(at).map(String::as_str) {
                    Some("ctl") => at += 2,
                    Some("adj") => at += 1,
                    _ => break,
                }
            }
            match tokens.get(at).map(String::as_str) {
                Some("h" | "s" | "sdg" | "t" | "tdg" | "cz" | "rx" | "ry" | "rz") => return true,
                Some("action_call") => {
                    if let Some(name) = tokens.get(at + 1) {
                        pending.push(name.trim_start_matches('@').into());
                    }
                }
                _ => {}
            }
        }
    }
    false
}
fn reject(message: &str) -> EvalError {
    EvalError::Reject(Reject::new("E_CERT_REPLAY", "§12.1/§14.3", "", message))
}
fn trace_error(outcome: ClaimOutcome) -> EvalError {
    match outcome.errors.into_iter().next() {
        Some(error) => EvalError::Reject(error),
        None => EvalError::Unchecked(
            outcome
                .unchecked_reason
                .unwrap_or_else(|| "source reference expansion is unavailable".into()),
        ),
    }
}

pub(crate) fn eval(
    object: &Object,
    symbol: &Symbol,
    iface: &Interface,
    before: &Tuple,
    args: &[Classical],
    inherited: usize,
    adjoint: bool,
    work: &mut u64,
) -> Result<Tuple, EvalError> {
    let width = iface
        .bundles
        .iter()
        .try_fold(inherited as u64, |n, b| n.checked_add(b.width));
    let Some(width) = width.filter(|w| *w <= 10) else {
        return Err(EvalError::Unchecked(
            "host limit: exact coherent reference replay requires at most 10 wires".into(),
        ));
    };
    let width = width as usize;
    let dim = 1usize << width;
    if args.iter().any(|v| matches!(v, Classical::Token(_))) {
        return Err(EvalError::Unchecked("coherent replay's value-only entry API needs an identity-preserving token binding before forwarding token arguments (K91)".into()));
    }
    if args.len() != iface.args.len() {
        return Err(reject("reference argument count differs from signature"));
    }
    let mut encoded = Vec::new();
    for (value, ty) in args.iter().zip(&iface.args) {
        encoded.push(match value {
            Classical::Bits(bits) if ty == "f64" => s(&format!("0x{:0>16}", bits.to_str_radix(16))),
            Classical::Bits(bits) => s(&bits.to_string()),
            Classical::Token(tuple) => {
                Value::Arr(tuple.iter().map(|v| s(&v.to_string())).collect())
            }
        });
    }
    let params_hash = crate::sha256_hex(
        &crate::json::jcs(&Value::Arr(encoded.clone())).expect("encoded args are JSON"),
    );
    let context = Context {
        symbol: symbol.name.clone(),
        controls: inherited as u64,
        invocation: Invocation {
            args: encoded,
            budget: std::array::from_fn(|_| BigUint::zero()),
            params_hash,
            token_aliases: vec![],
        },
        key: vec![],
        adjoint,
    };
    // Bound amplitude work independently of the source instruction count.
    let max = (*work).min((1 << 25) / dim as u64);
    let gates = reference_gates(object, &context, max).map_err(trace_error)?;
    if before.bundles.len() != inherited + iface.bundles.len() {
        return Err(reject("basis tuple does not match interface"));
    }
    let widths: Vec<u64> = std::iter::repeat_n(1, inherited)
        .chain(iface.bundles.iter().map(|b| b.width))
        .collect();
    let mut index = 0usize;
    let mut shift = 0;
    for (value, bits) in before.bundles.iter().zip(&widths) {
        if value.bits() > *bits {
            return Err(reject("basis tuple component exceeds its width"));
        }
        let value = value.to_u64_digits().first().copied().unwrap_or(0) as usize;
        index |= value << shift;
        shift += *bits as usize;
    }
    let mut state = vec![Amp::default(); dim];
    state[index] = Amp::one();
    let half = Rat::new(1.into(), 2.into());
    let h = Amp {
        re: [Rat::zero(), half.clone()],
        ..Amp::default()
    };
    for gate in &gates {
        if *work == 0 {
            return Err(EvalError::Unchecked(
                "host limit: coherent reference instruction work exhausted".into(),
            ));
        }
        *work -= 1;
        let on = |i: usize| gate.controls.iter().all(|c| i & (1usize << c) != 0);
        let target = 1usize << gate.targets[0];
        match gate.realised_opcode() {
            "x"|"cx"|"ccx"|"mcx"=>for i in 0..dim {
                if on(i) && i&target==0 {state.swap(i,i|target);}
            },
            "swap"=>{
                let other=1usize<<gate.targets[1];
                for i in 0..dim {if on(i)&&i&target==0&&i&other!=0 {state.swap(i,i^target^other);}}
            },
            "h"=>for i in 0..dim {if on(i)&&i&target==0 {
                let j=i|target;let a=state[i].clone();let b=state[j].clone();
                state[i]=a.add(&b).mul(&h);state[j]=a.add(&b.neg()).mul(&h);
            }},
            "s"|"sdg"|"t"|"tdg"=>{
                let op=gate.realised_opcode();let mut factor=Amp::default();
                match op {
                    "s"=>factor.im[0]=Rat::one(),
                    "sdg"=>factor.im[0]=-Rat::one(),
                    _=>{factor.re[1]=half.clone();factor.im[1]=if op=="t" {half.clone()}else{-&half};},
                }
                for (i,amp) in state.iter_mut().enumerate() {if on(i)&&i&target!=0 {*amp=amp.mul(&factor);}}
            },
            "cz"=>{
                let other=1usize<<gate.targets[1];
                for (i,amp) in state.iter_mut().enumerate() {if on(i)&&i&target!=0&&i&other!=0 {*amp=amp.neg();}}
            },
            op=>return Err(EvalError::Unchecked(format!("exact coherent reference replay of {op} needs a further exact number representation"))),
        }
    }
    let mut nonzero = state.iter().enumerate().filter(|(_, v)| !v.is_zero());
    let (index, amp) = nonzero
        .next()
        .ok_or_else(|| reject("coherent reference output has zero norm"))?;
    if nonzero.next().is_some() {
        return Err(reject("coherent reference output is a superposition; a permutation assertion requires one basis state"));
    }
    if *amp != Amp::one() {
        return Err(reject("coherent reference basis phase is not exactly +1"));
    }
    let mut after = before.clone();
    let mut shift = 0;
    for (slot, bits) in after.bundles.iter_mut().zip(&widths) {
        *slot = BigUint::from((index >> shift) & ((1usize << *bits as usize) - 1));
        shift += *bits as usize;
    }
    if after.bundles[..inherited] != before.bundles[..inherited]
        || iface.bundles.iter().enumerate().any(|(i, b)| {
            b.group == "in" && after.bundles[inherited + i] != before.bundles[inherited + i]
        })
    {
        return Err(reject(
            "coherent reference action changes an input or inherited control",
        ));
    }
    Ok(after)
}
