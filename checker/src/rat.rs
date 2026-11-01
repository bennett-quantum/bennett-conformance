//! Exact rational arithmetic over decimal text (ISA sections 8.1, 10.1, 12),
//! plus interval arithmetic with outward rounding to `precision_bits`-bit
//! binary rationals (section 12.1).

use crate::json::Value;
use num_bigint::{BigInt, BigUint, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::str::FromStr;

pub type Rat = BigRational;

/// Parse canonical decimal text exactly. Accepts JSON-number grammar
/// (optional exponent); rejects fractions, leading zeros like `01`,
/// bare `.`, and empty text. `+` signs are rejected (no spec decimal has one).
pub fn parse_decimal(text: &str) -> Result<Rat, String> {
    if text.is_empty() {
        return Err("empty decimal".to_string());
    }
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if body.is_empty() || body.starts_with('+') {
        return Err(format!("invalid decimal {text:?}"));
    }
    // Split exponent.
    let (mant, exp): (&str, i64) = match body.find(['e', 'E']) {
        Some(pos) => {
            let e: i64 = body[pos + 1..]
                .parse()
                .map_err(|_| format!("invalid decimal exponent in {text:?}"))?;
            (&body[..pos], e)
        }
        None => (body, 0),
    };
    let (int_part, frac_part) = match mant.split_once('.') {
        Some((a, b)) => (a, b),
        None => (mant, ""),
    };
    if int_part.is_empty()
        || !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
        || (int_part.len() > 1 && int_part.starts_with('0'))
        || (mant.contains('.') && frac_part.is_empty())
    {
        return Err(format!("non-canonical decimal {text:?}"));
    }
    let mut digits = int_part.to_string();
    digits.push_str(frac_part);
    let numer = BigInt::from_str(&digits).map_err(|_| format!("invalid decimal {text:?}"))?;
    let scale = frac_part.len() as i64 - exp;
    let mut r = if scale >= 0 {
        Rat::new(numer, BigInt::from(10u32).pow(scale as u32))
    } else {
        Rat::new(
            numer * BigInt::from(10u32).pow((-scale) as u32),
            BigInt::one(),
        )
    };
    if neg {
        r = -r;
    }
    Ok(r)
}

/// A spec decimal appears either as a JSON number or as a decimal string;
/// both denote the exact rational of their text (section 11).
pub fn rat_of_json(v: &Value) -> Result<Rat, String> {
    match v {
        Value::Num(lex) => parse_decimal(lex),
        Value::Str(s) => parse_decimal(s),
        other => Err(format!(
            "expected decimal number or string, got {}",
            other.kind()
        )),
    }
}

/// Canonical unsigned decimal string: `0`, or digits with no leading zero.
pub fn is_canonical_uint_str(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s.len() == 1 || !s.starts_with('0'))
}

pub fn parse_biguint_str(s: &str) -> Option<BigUint> {
    if is_canonical_uint_str(s) {
        BigUint::from_str(s).ok()
    } else {
        None
    }
}

/// Reduced fraction "n/d" for verdicts.
pub fn fraction_string(num: &BigUint, den: &BigUint) -> String {
    if num.is_zero() {
        return "0/1".to_string();
    }
    let g = gcd(num.clone(), den.clone());
    format!("{}/{}", num / &g, den / &g)
}

pub fn gcd(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a
}

/// Section 10.1 display: least nonnegative decimal with at most 34
/// significant digits that is >= q, spelled without exponent.
pub fn upward_decimal_34(q: &Rat) -> Result<String, String> {
    if q.is_negative() {
        return Err("upward_decimal_34 of a negative value".to_string());
    }
    if q.is_zero() {
        return Ok("0".to_string());
    }
    // e with 10^e <= q < 10^(e+1); unit = 10^(e-33); result = ceil(q/unit)*unit.
    let e = floor_log10(q);
    let shift = e - 33;
    let scaled_num;
    let scaled_den;
    if shift >= 0 {
        // q / 10^shift
        scaled_num = q.numer().clone();
        scaled_den = q.denom().clone() * BigInt::from(10u32).pow(shift as u32);
    } else {
        scaled_num = q.numer().clone() * BigInt::from(10u32).pow((-shift) as u32);
        scaled_den = q.denom().clone();
    }
    let ceil = ceil_div_nonneg(&scaled_num, &scaled_den);
    // ceil * 10^shift rendered without exponent, no trailing fractional zeros.
    let digits = ceil.to_string();
    let out = if shift >= 0 {
        format!("{digits}{}", "0".repeat(shift as usize))
    } else {
        let point = digits.len() as i64 + shift;
        if point > 0 {
            let (a, b) = digits.split_at(point as usize);
            let b = b.trim_end_matches('0');
            if b.is_empty() {
                a.to_string()
            } else {
                format!("{a}.{b}")
            }
        } else {
            let b = format!("{}{digits}", "0".repeat((-point) as usize));
            let b = b.trim_end_matches('0');
            format!("0.{b}")
        }
    };
    Ok(out)
}

fn ceil_div_nonneg(num: &BigInt, den: &BigInt) -> BigInt {
    debug_assert!(num.sign() != Sign::Minus && den.sign() == Sign::Plus);
    let q = num / den;
    if (num % den).is_zero() {
        q
    } else {
        q + BigInt::one()
    }
}

fn floor_log10(q: &Rat) -> i64 {
    debug_assert!(!q.is_zero() && !q.is_negative());
    let mut e: i64 = (decimal_digits(q.numer()) as i64) - (decimal_digits(q.denom()) as i64);
    // Compare q against 10^e and 10^(e+1), adjust by at most a few steps.
    while cmp_pow10(q, e) < 0 {
        e -= 1;
    }
    while cmp_pow10(q, e + 1) >= 0 {
        e += 1;
    }
    e
}

/// Sign of q - 10^e for q > 0.
fn cmp_pow10(q: &Rat, e: i64) -> i32 {
    let lhs = if e >= 0 {
        // q >= 10^e  <=>  numer >= denom * 10^e
        q.numer().clone() - q.denom().clone() * BigInt::from(10u32).pow(e as u32)
    } else {
        q.numer().clone() * BigInt::from(10u32).pow((-e) as u32) - q.denom().clone()
    };
    match lhs.sign() {
        Sign::Plus => 1,
        Sign::Minus => -1,
        Sign::NoSign => 0,
    }
}

fn decimal_digits(v: &BigInt) -> usize {
    v.magnitude().to_string().len()
}

// ---------- intervals ----------

/// Closed rational interval. Every operation rounds outward to
/// `precision_bits`-bit binary rationals (section 12.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub lo: Rat,
    pub hi: Rat,
}

impl Interval {
    pub fn point(v: Rat, p: u32) -> Interval {
        Interval {
            lo: round_down(&v, p),
            hi: round_up(&v, p),
        }
    }
    pub fn exact(v: Rat) -> Interval {
        Interval {
            lo: v.clone(),
            hi: v,
        }
    }
    pub fn add(&self, o: &Interval, p: u32) -> Interval {
        Interval {
            lo: round_down(&(&self.lo + &o.lo), p),
            hi: round_up(&(&self.hi + &o.hi), p),
        }
    }
    pub fn sub(&self, o: &Interval, p: u32) -> Interval {
        Interval {
            lo: round_down(&(&self.lo - &o.hi), p),
            hi: round_up(&(&self.hi - &o.lo), p),
        }
    }
    pub fn neg(&self) -> Interval {
        Interval {
            lo: -self.hi.clone(),
            hi: -self.lo.clone(),
        }
    }
    pub fn mul(&self, o: &Interval, p: u32) -> Interval {
        let products = [
            &self.lo * &o.lo,
            &self.lo * &o.hi,
            &self.hi * &o.lo,
            &self.hi * &o.hi,
        ];
        let lo = products.iter().min().unwrap().clone();
        let hi = products.iter().max().unwrap().clone();
        Interval {
            lo: round_down(&lo, p),
            hi: round_up(&hi, p),
        }
    }
    pub fn div(&self, o: &Interval, p: u32) -> Option<Interval> {
        if o.lo <= Rat::zero() && o.hi >= Rat::zero() {
            return None; // division by an interval containing 0: inconclusive
        }
        let recip = Interval {
            lo: round_down(&o.hi.recip(), p),
            hi: round_up(&o.lo.recip(), p),
        };
        Some(self.mul(&recip, p))
    }
    /// sqrt of a nonnegative interval; None when lo < 0 (inconclusive).
    pub fn sqrt(&self, p: u32) -> Option<Interval> {
        if self.lo.is_negative() {
            return None;
        }
        Some(Interval {
            lo: sqrt_down(&self.lo, p),
            hi: sqrt_up(&self.hi, p),
        })
    }
    pub fn max_abs(&self) -> Rat {
        let a = self.lo.abs();
        let b = self.hi.abs();
        if a >= b {
            a
        } else {
            b
        }
    }
    pub fn contains_zero(&self) -> bool {
        self.lo <= Rat::zero() && self.hi >= Rat::zero()
    }
}

/// Round down to a binary rational with at most p significant bits.
pub fn round_down(v: &Rat, p: u32) -> Rat {
    round_binary(v, p, true)
}
pub fn round_up(v: &Rat, p: u32) -> Rat {
    round_binary(v, p, false)
}

fn round_binary(v: &Rat, p: u32, down: bool) -> Rat {
    if v.is_zero() || p == 0 {
        return v.clone();
    }
    // Find e = floor(log2 |v|); quantum = 2^(e-p+1); round v/quantum to integer.
    let e = floor_log2_abs(v);
    let q_exp = e - (p as i64) + 1;
    let scaled = scale_pow2(v, -q_exp); // v / 2^q_exp
    let n = if down {
        floor_int(&scaled)
    } else {
        ceil_int(&scaled)
    };
    scale_pow2(&Rat::from(n), q_exp)
}

pub fn scale_pow2(v: &Rat, e: i64) -> Rat {
    if e >= 0 {
        v * Rat::new(BigInt::one() << (e as usize), BigInt::one())
    } else {
        v / Rat::new(BigInt::one() << ((-e) as usize), BigInt::one())
    }
}

fn floor_int(v: &Rat) -> BigInt {
    v.floor().to_integer()
}
fn ceil_int(v: &Rat) -> BigInt {
    v.ceil().to_integer()
}

fn floor_log2_abs(v: &Rat) -> i64 {
    debug_assert!(!v.is_zero());
    let n = v.numer().magnitude();
    let d = v.denom().magnitude();
    let mut e = (n.bits() as i64) - (d.bits() as i64);
    // |v| in (2^(e-1), 2^(e+1)); adjust by comparison with 2^e.
    while cmp_pow2_abs(v, e) < 0 {
        e -= 1;
    }
    while cmp_pow2_abs(v, e + 1) >= 0 {
        e += 1;
    }
    e
}

/// Sign of |v| - 2^e.
fn cmp_pow2_abs(v: &Rat, e: i64) -> i32 {
    let n = v.numer().magnitude().clone();
    let d = v.denom().magnitude().clone();
    let lhs = if e >= 0 {
        // n ? d * 2^e
        let rhs = d << (e as usize);
        n.cmp(&rhs)
    } else {
        let lhs = n << ((-e) as usize);
        lhs.cmp(&d)
    };
    match lhs {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    }
}

fn sqrt_down(v: &Rat, p: u32) -> Rat {
    sqrt_bounds(v, p).0
}
fn sqrt_up(v: &Rat, p: u32) -> Rat {
    sqrt_bounds(v, p).1
}

fn sqrt_bounds(v: &Rat, p: u32) -> (Rat, Rat) {
    if v.is_zero() {
        return (Rat::zero(), Rat::zero());
    }
    // floor(log2(sqrt(v))) = floor(floor(log2(v))/2), including
    // negative exponents. The quantum must be relative to v: a fixed
    // 2^-p grid loses every significant bit for tiny positive pivots.
    let exponent = floor_log2_abs(v).div_euclid(2);
    let quantum = exponent - i64::from(p.max(1)) + 1;
    let scaled = scale_pow2(v, -2 * quantum);
    let root = isqrt(floor_int(&scaled).magnitude());
    let lo = scale_pow2(&Rat::from(BigInt::from(root.clone())), quantum);
    let exact = Rat::from(BigInt::from(&root * &root)) == scaled;
    let hi = if exact {
        lo.clone()
    } else {
        scale_pow2(&Rat::from(BigInt::from(root + BigUint::one())), quantum)
    };
    (lo, hi)
}

/// Enclosure of sqrt(2)/2 etc. come through Interval::sqrt of exact values.
pub fn isqrt(n: &BigUint) -> BigUint {
    if n.is_zero() {
        return BigUint::zero();
    }
    // Newton iteration from a power-of-two over-estimate.
    let bits = n.bits();
    let mut x = BigUint::one() << ((bits / 2 + 1) as usize);
    loop {
        let y = (&x + n / &x) >> 1usize;
        if y >= x {
            break;
        }
        x = y;
    }
    while &(&x * &x) > n {
        x -= BigUint::one();
    }
    while &((&x + BigUint::one()) * (&x + BigUint::one())) <= n {
        x += BigUint::one();
    }
    x
}

pub fn rat_to_f64(v: &Rat) -> Option<f64> {
    v.to_f64()
}
