//! Public source-addressed Q construction for applicability/replay, ISA §§4–5/11.
//! All operations are reconstructed; producer occurrence lists never drive execution.
use crate::cert::{ClaimOutcome, Inputs, LoadedDocs};
use crate::json::{n, s, Value};
use crate::realisation::{Context, Gate};
use std::collections::{BTreeMap, HashMap, HashSet};
mod source;
pub(crate) mod state;

type Result<T> = std::result::Result<T, ClaimOutcome>;
fn pending<T>(message: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::unchecked(message))
}
fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(ClaimOutcome::rejected(vec![crate::cert::rej(
        "E_CERT_PREPARATION",
        "§11/§12.3",
        "bindings.preparation",
        message.into(),
    )]))
}
#[derive(Clone)]
pub(crate) struct Call {
    pub context: Context,
    pub regions: Vec<Vec<usize>>,
    pub bundles: Vec<Vec<usize>>,
}
#[derive(Clone)]
pub(crate) enum Kind {
    Allocate(Vec<usize>),
    Release(Vec<usize>),
    Gate(Gate),
    Enter(Call),
    Leave(Call),
}
#[derive(Clone)]
pub(crate) struct Event {
    pub address: Value,
    pub kind: Kind,
}
pub(crate) struct Trace {
    pub events: Vec<Event>,
    pub input_width: usize,
    next: usize,
    max: u64,
}
impl Trace {
    pub(crate) fn build(docs: &LoadedDocs, inputs: &Inputs) -> Result<Self> {
        if docs
            .schedule
            .as_ref()
            .and_then(|s| s.get("entries"))
            .and_then(Value::as_arr)
            .unwrap_or(&[])
            .iter()
            .any(|e| {
                e.get("kind").and_then(Value::as_str) == Some("cleanup")
                    && e.get("cleanup")
                        .and_then(|c| c.get("realisation"))
                        .and_then(Value::as_str)
                        != Some("reverse")
            })
        {
            return pending(
                "complete realised prefix reconstruction of non-reverse cleanup is unavailable",
            );
        }
        if docs.expansion_unknown {
            return pending("bound expansion library unavailable for preparation reconstruction");
        }
        let symbol = docs
            .object
            .symbol_by_closure(&docs.root_closure)
            .ok_or_else(|| ClaimOutcome::unchecked("device root not linked"))?;
        let invocation = docs.root_invocation.clone().ok_or_else(|| {
            ClaimOutcome::unchecked("root invocation unavailable for realised prefix")
        })?;
        let controls = docs
            .schedule
            .as_ref()
            .and_then(|v| v.get("bindings"))
            .and_then(|v| v.get("inherited_controls"))
            .and_then(crate::cert::u64_of)
            .ok_or_else(|| {
                ClaimOutcome::unchecked(
                    "realised prefix requires a bound root schedule and control count",
                )
            })?;
        Self::subject(
            docs,
            inputs,
            Context {
                symbol: symbol.name.clone(),
                controls,
                invocation,
                key: vec![],
                adjoint: false,
            },
        )
    }
    pub(crate) fn subject(docs: &LoadedDocs, inputs: &Inputs, context: Context) -> Result<Self> {
        let iface = docs
            .object
            .interface(&context.symbol)
            .map_err(ClaimOutcome::unchecked)?;
        if !iface.borrowed_widths.is_empty() {
            return pending("construction of borrowed lifetime inputs remains unavailable");
        }
        let mut trace = Self {
            events: vec![],
            input_width: 0,
            next: context.controls as usize,
            max: inputs.limits.max_reference_steps,
        };
        let mut wires = HashMap::new();
        let mut bundles = vec![];
        let mut regions = vec![];
        for b in &iface.bundles {
            let v = trace.wires(b.width)?;
            wires.insert(b.name.clone(), v.clone());
            bundles.push(v);
        }
        for r in &iface.regions {
            let v = trace.region(r.a, r.w)?;
            wires.insert(r.name.clone(), v.clone());
            regions.push(v);
        }
        trace.input_width = trace.next;
        trace.collect(
            docs,
            Call {
                context: context.clone(),
                regions,
                bundles,
            },
            wires,
            (0..context.controls as usize).collect(),
            0,
        )?;
        Ok(trace)
    }
    fn wires(&mut self, width: u64) -> Result<Vec<usize>> {
        let width = usize::try_from(width)
            .map_err(|_| ClaimOutcome::unchecked("host limit: construction width exceeds usize"))?;
        let end = self
            .next
            .checked_add(width)
            .filter(|n| *n <= 999_999)
            .ok_or_else(|| {
                ClaimOutcome::unchecked(
                    "host limit: construction has more than 999999 wire identities",
                )
            })?;
        let v = (self.next..end).collect();
        self.next = end;
        Ok(v)
    }
    fn region(&mut self, a: u64, w: u64) -> Result<Vec<usize>> {
        let width = 1u64
            .checked_shl(u32::try_from(a).unwrap_or(u32::MAX))
            .and_then(|n| n.checked_mul(w))
            .ok_or_else(|| {
                ClaimOutcome::unchecked("host limit: construction region bit count exceeds u64")
            })?;
        self.wires(width)
    }
    fn push(&mut self, address: Value, kind: Kind) -> Result<()> {
        if self.events.len() as u64 >= self.max {
            return pending(
                "host limit: source-addressed construction event count exceeds replay work limit",
            );
        }
        self.events.push(Event { address, kind });
        Ok(())
    }
}
fn address(
    context: &Context,
    hash: &str,
    point: &str,
    iteration: &[u64],
    generated: u64,
    allocation: Option<&Value>,
) -> Value {
    let mut fields = BTreeMap::from([
        ("context_key".into(), Value::Arr(context.key.clone())),
        ("symbol".into(), s(hash)),
        ("point".into(), s(point)),
        (
            "iteration".into(),
            Value::Arr(iteration.iter().map(n).collect()),
        ),
        ("instance".into(), Value::Arr(vec![])),
        ("generated".into(), n(generated)),
    ]);
    if let Some(a) = allocation {
        fields.insert("allocation".into(), a.clone());
    }
    Value::Obj(fields)
}
fn gate(
    op: &str,
    controls: Vec<usize>,
    targets: Vec<usize>,
    context: &Context,
    addr: &Value,
    own_controls: usize,
) -> Gate {
    let mut path = context.key.clone();
    let mut terminal = addr.as_obj().expect("constructed address").clone();
    terminal.remove("context_key");
    terminal.remove("generated");
    terminal.remove("allocation");
    terminal.insert("call_index".into(), n(0));
    path.push(Value::Obj(terminal));
    Gate {
        op: op.into(),
        own_controls,
        controls,
        targets,
        path,
        generated: crate::cert::u64_of(addr.get("generated").unwrap()).unwrap(),
        angle_bits: None,
        adjoint: false,
        template_leaf: false,
        requested_share: None,
    }
}
