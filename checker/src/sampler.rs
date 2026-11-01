//! Section 12.1 sampling: xoshiro256** (Blackman & Vigna's published
//! generator of that name) with the spec's component-draw procedure.

use crate::domain::{Component, ResolvedDomain, Tuple};
use crate::fault::{HostLimit, ResolveError};
use crate::rat::is_canonical_uint_str;
use num_bigint::BigUint;
use num_traits::{One, Zero};

pub struct Sampler {
    s: [u64; 4],
}

impl Sampler {
    pub fn new(seed: [&str; 4]) -> Result<Sampler, String> {
        let mut s = [0u64; 4];
        for (i, word) in seed.iter().enumerate() {
            if !is_canonical_uint_str(word) {
                return Err(format!("sampler seed[{i}] is not a canonical decimal string: {word:?}"));
            }
            s[i] = word
                .parse::<u64>()
                .map_err(|_| format!("sampler seed[{i}] does not fit u64: {word:?}"))?;
        }
        if s.iter().all(|w| *w == 0) {
            return Err("sampler seed must not be all zero".to_string());
        }
        Ok(Sampler { s })
    }

    pub fn state(&self) -> [u64; 4] {
        self.s
    }

    fn next_u64(&mut self) -> u64 {
        let result = (self.s[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Draw one component value from [lo, lo+size): m consecutive words
    /// (least significant first), masked to ceil(log2(size)) bits,
    /// rejected and redrawn while >= size; size 1 draws no words.
    pub fn draw_range(&mut self, lo: &BigUint, size: &BigUint) -> BigUint {
        if *size == BigUint::one() {
            return lo.clone();
        }
        let bits = size.bits(); // ceil(log2(size)) == bits-1 unless size is a power of two
        let need_bits = if (size & (size - BigUint::one())).is_zero() { bits - 1 } else { bits };
        let words = need_bits.div_ceil(64);
        loop {
            let mut acc = BigUint::zero();
            for i in 0..words {
                let w = self.next_u64();
                acc |= BigUint::from(w) << (64 * i);
            }
            if need_bits > 0 {
                acc &= (BigUint::one() << need_bits) - BigUint::one();
            }
            if acc < *size {
                return lo + acc;
            }
        }
    }

    pub fn draw_component(&mut self, lo: u64, hi: u64) -> BigUint {
        debug_assert!(lo <= hi);
        self.draw_range(&BigUint::from(lo), &BigUint::from(hi - lo + 1))
    }
}

/// Draw `count` elements (with replacement) in enumeration-component order.
pub fn draw_elements(
    d: &ResolvedDomain,
    seed: [&str; 4],
    count: u64,
) -> Result<Vec<Tuple>, ResolveError> {
    let mut sampler = Sampler::new(seed).map_err(|e| {
        ResolveError::Reject(crate::fault::Reject::new("E_CERT_SAMPLER", "§12.1", "", e))
    })?;
    if count > 10_000_000 {
        return Err(ResolveError::HostLimit(HostLimit {
            quantity: format!("sampler count {count} (§12.2 host limit)"),
        }));
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut tuple = d.zero_tuple();
        let comps: Vec<Component> = d.components.clone();
        for c in &comps {
            let v = sampler.draw_range(&c.lo, &c.size);
            d.set_component(&mut tuple, c, v);
        }
        out.push(tuple);
    }
    Ok(out)
}
