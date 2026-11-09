//! Exact sparse pure-state reconstruction; matrix columns preserve external references.
use super::*;
use crate::coherent::Amp;
use crate::rat::Rat;
use num_bigint::BigUint;
use num_traits::{One, Zero};

#[derive(Clone)]
pub(crate) struct State {
    pub amplitudes: BTreeMap<BigUint, Amp>,
}
impl State {
    pub(crate) fn basis(value: BigUint) -> Self {
        Self {
            amplitudes: BTreeMap::from([(value, Amp::one())]),
        }
    }
    pub(crate) fn apply(&mut self, event: &Event, docs: &LoadedDocs) -> Result<()> {
        match &event.kind {
            Kind::Allocate(bits) | Kind::Release(bits) => {
                if self
                    .amplitudes
                    .keys()
                    .any(|v| bits.iter().any(|bit| v.bit(*bit as u64)))
                {
                    return invalid("construction allocation/release is not a clean zero factor");
                }
            }
            Kind::Gate(gate) => self.gate(docs, gate)?,
            Kind::Enter(_) | Kind::Leave(_) => {}
        }
        Ok(())
    }
    fn add(output: &mut BTreeMap<BigUint, Amp>, basis: BigUint, amp: Amp) {
        let value = output.entry(basis).or_default();
        *value = value.add(&amp);
    }
    fn gate(&mut self, docs: &LoadedDocs, source: &Gate) -> Result<()> {
        let gate = crate::realisation::realised_leaf(docs, source)?;
        let entry = crate::realisation::native(docs, &gate)?;
        if entry.get("realisation_model").is_some() {
            if !gate.controls.is_empty() || gate.targets.len() != 1 {
                return pending("exact prefix reconstruction of controlled/multi-wire native models is unavailable");
            }
            let mut local = gate.clone().into_owned();
            local.targets = vec![0];
            let matrix = crate::realisation::native_model::matrix(docs, &local, 1, 128)?;
            let mut out = BTreeMap::new();
            let mask = BigUint::one() << gate.targets[0];
            for (basis, amp) in &self.amplitudes {
                let col = usize::from(basis.bit(gate.targets[0] as u64));
                for row in 0..2 {
                    let z = matrix.get(row, col);
                    if z.re.lo != z.re.hi || z.im.lo != z.im.hi {
                        return pending("native prefix model has no exact rational reconstruction");
                    }
                    let factor = Amp {
                        re: [z.re.lo.clone(), Rat::zero()],
                        im: [z.im.lo.clone(), Rat::zero()],
                    };
                    let target = if row == col {
                        basis.clone()
                    } else {
                        basis ^ &mask
                    };
                    Self::add(&mut out, target, amp.mul(&factor));
                }
            }
            out.retain(|_, amp| !amp.is_zero());
            self.amplitudes = out;
            return Ok(());
        }
        // select/native_numbers requires an exact model when epsilon is positive.
        let mut out = BTreeMap::new();
        let target = gate.targets[0];
        let mask = BigUint::one() << target;
        for (basis, amp) in &self.amplitudes {
            if !gate.controls.iter().all(|bit| basis.bit(*bit as u64)) {
                Self::add(&mut out, basis.clone(), amp.clone());
                continue;
            }
            let on = basis.bit(target as u64);
            match gate.realised_opcode() {
                "x"|"cx"|"ccx"|"mcx"=>Self::add(&mut out,basis^&mask,amp.clone()),
                "swap"=>{let other=gate.targets[1];let value=if on!=basis.bit(other as u64){basis^&mask^(BigUint::one()<<other)}else{basis.clone()};Self::add(&mut out,value,amp.clone());},
                "h"=>{
                    let scale=Amp{re:[Rat::zero(),Rat::new(1.into(),2.into())],im:[Rat::zero(),Rat::zero()]};
                    let scaled=amp.mul(&scale);Self::add(&mut out,basis.clone(),if on{scaled.neg()}else{scaled.clone()});Self::add(&mut out,basis^&mask,scaled);
                },
                "s"|"sdg"|"t"|"tdg"=>{
                    let mut factor=Amp::one();if on {factor=Amp::default();match gate.realised_opcode(){"s"=>factor.im[0]=Rat::one(),"sdg"=>factor.im[0]=-Rat::one(),op=>{factor.re[1]=Rat::new(1.into(),2.into());factor.im[1]=if op=="t"{factor.re[1].clone()}else{-&factor.re[1]};}}}
                    Self::add(&mut out,basis.clone(),amp.mul(&factor));
                },
                "cz"=>Self::add(&mut out,basis.clone(),if on&&basis.bit(gate.targets[1] as u64){amp.neg()}else{amp.clone()}),
                op=>return pending(format!("exact prefix state reconstruction of {op} needs a further exact number representation")),
            }
        }
        out.retain(|_, amp| !amp.is_zero());
        if out.len() > 1 << 20 {
            return pending("host limit: exact prefix state exceeds 1048576 amplitudes");
        }
        self.amplitudes = out;
        Ok(())
    }
    pub(crate) fn image(&self, bits: &[usize], cells: &[BigUint], width: u64) -> bool {
        self.amplitudes.keys().all(|basis| {
            bits.iter().enumerate().all(|(i, bit)| {
                basis.bit(*bit as u64)
                    == cells[i / (width as usize)].bit((i % (width as usize)) as u64)
            })
        })
    }
}
