//! Independent interval complex matrices for ISA §12.1. Every arithmetic
//! operation rounds outward through `rat::Interval`; no floating point is
//! used for a bound or a positivity decision.
use crate::json::Value;
use crate::rat::{parse_decimal, Interval, Rat};
use num_traits::{One, Zero};

#[derive(Debug, Clone, PartialEq)]
pub struct ComplexInterval {
    pub re: Interval,
    pub im: Interval,
}
impl ComplexInterval {
    pub fn zero() -> Self {
        Self::real(Rat::zero())
    }
    pub fn real(v: Rat) -> Self {
        Self {
            re: Interval::exact(v),
            im: Interval::exact(Rat::zero()),
        }
    }
    pub fn conj(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: self.im.neg(),
        }
    }
    pub fn add(&self, b: &Self, p: u32) -> Self {
        Self {
            re: self.re.add(&b.re, p),
            im: self.im.add(&b.im, p),
        }
    }
    pub fn sub(&self, b: &Self, p: u32) -> Self {
        Self {
            re: self.re.sub(&b.re, p),
            im: self.im.sub(&b.im, p),
        }
    }
    pub fn mul(&self, b: &Self, p: u32) -> Self {
        Self {
            re: self.re.mul(&b.re, p).sub(&self.im.mul(&b.im, p), p),
            im: self.re.mul(&b.im, p).add(&self.im.mul(&b.re, p), p),
        }
    }
    pub fn scale(&self, b: &Interval, p: u32) -> Self {
        Self {
            re: self.re.mul(b, p),
            im: self.im.mul(b, p),
        }
    }
    pub fn div_real(&self, b: &Interval, p: u32) -> Option<Self> {
        Some(Self {
            re: self.re.div(b, p)?,
            im: self.im.div(b, p)?,
        })
    }
    pub fn from_endpoints(v: &Value, p: u32) -> Result<Self, String> {
        let a = v
            .as_arr()
            .filter(|a| a.len() == 4)
            .ok_or("complex interval must have four decimal-string endpoints")?;
        let a = a
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| "interval endpoint must be a decimal string".to_string())
                    .and_then(parse_decimal)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if a[0] > a[1] || a[2] > a[3] {
            return Err("interval endpoints are reversed".into());
        }
        Ok(Self {
            re: Interval {
                lo: crate::rat::round_down(&a[0], p),
                hi: crate::rat::round_up(&a[1], p),
            },
            im: Interval {
                lo: crate::rat::round_down(&a[2], p),
                hi: crate::rat::round_up(&a[3], p),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub dim: usize,
    pub data: Vec<ComplexInterval>,
}
impl Matrix {
    pub fn zeros(dim: usize) -> Self {
        Self {
            dim,
            data: vec![ComplexInterval::zero(); dim * dim],
        }
    }
    pub fn identity(dim: usize) -> Self {
        let mut m = Self::zeros(dim);
        for i in 0..dim {
            m.set(i, i, ComplexInterval::real(Rat::one()));
        }
        m
    }
    pub fn from_rows(rows: Vec<Vec<ComplexInterval>>) -> Result<Self, String> {
        let dim = rows.len();
        if dim == 0 || rows.iter().any(|r| r.len() != dim) {
            return Err("matrix must be nonempty and square".into());
        }
        Ok(Self {
            dim,
            data: rows.into_iter().flatten().collect(),
        })
    }
    pub fn from_encoding(v: &Value, dim: usize, p: u32) -> Result<Self, String> {
        let a = v.as_arr().ok_or("Y must be a row-major array")?;
        if a.len() != dim.checked_mul(dim).ok_or("matrix dimension overflow")? {
            return Err(format!(
                "Y has {} entries; dimension {dim} requires {}",
                a.len(),
                dim * dim
            ));
        }
        let m = Self {
            dim,
            data: a
                .iter()
                .map(|e| ComplexInterval::from_endpoints(e, p))
                .collect::<Result<_, _>>()?,
        };
        if !m.is_hermitian() {
            return Err("Y interval encoding is not Hermitian (diagonal imaginary parts must be zero and off-diagonal enclosures conjugate)".into());
        }
        Ok(m)
    }
    pub fn get(&self, i: usize, j: usize) -> &ComplexInterval {
        &self.data[i * self.dim + j]
    }
    pub fn set(&mut self, i: usize, j: usize, v: ComplexInterval) {
        self.data[i * self.dim + j] = v;
    }
    pub fn adjoint(&self) -> Self {
        let mut m = Self::zeros(self.dim);
        for i in 0..self.dim {
            for j in 0..self.dim {
                m.set(i, j, self.get(j, i).conj());
            }
        }
        m
    }
    pub fn add(&self, b: &Self, p: u32) -> Self {
        assert_eq!(self.dim, b.dim);
        Self {
            dim: self.dim,
            data: self
                .data
                .iter()
                .zip(&b.data)
                .map(|(a, b)| a.add(b, p))
                .collect(),
        }
    }
    pub fn sub(&self, b: &Self, p: u32) -> Self {
        assert_eq!(self.dim, b.dim);
        Self {
            dim: self.dim,
            data: self
                .data
                .iter()
                .zip(&b.data)
                .map(|(a, b)| a.sub(b, p))
                .collect(),
        }
    }
    pub fn scale(&self, b: &Interval, p: u32) -> Self {
        Self {
            dim: self.dim,
            data: self.data.iter().map(|a| a.scale(b, p)).collect(),
        }
    }
    pub fn mul(&self, b: &Self, p: u32) -> Self {
        assert_eq!(self.dim, b.dim);
        let mut m = Self::zeros(self.dim);
        for i in 0..self.dim {
            for j in 0..self.dim {
                let mut sum = ComplexInterval::zero();
                for k in 0..self.dim {
                    sum = sum.add(&self.get(i, k).mul(b.get(k, j), p), p);
                }
                m.set(i, j, sum);
            }
        }
        m
    }
    pub fn frobenius_upper(&self, p: u32) -> Rat {
        let mut sum = Interval::exact(Rat::zero());
        for z in &self.data {
            for a in [&z.re, &z.im] {
                let a = Interval::point(a.max_abs(), p);
                sum = sum.add(&a.mul(&a, p), p);
            }
        }
        // The summands are squared nonnegative endpoints, so the exact
        // sum is nonnegative even if an earlier enclosure touched zero.
        sum.sqrt(p).expect("sum of nonnegative squares").hi
    }
    /// Choi of rho -> U rho U†, with input index first, unnormalised.
    pub fn choi(&self, p: u32) -> Self {
        let d = self.dim;
        let mut j = Self::zeros(d * d);
        for i in 0..d {
            for k in 0..d {
                for a in 0..d {
                    for b in 0..d {
                        j.set(
                            i * d + a,
                            k * d + b,
                            self.get(a, i).mul(&self.get(b, k).conj(), p),
                        );
                    }
                }
            }
        }
        // U|i><j|U† is Hermitian across the composite indices.
        // Preserve that exact algebraic identity in its enclosure instead
        // of introducing an artificial imaginary diagonal uncertainty.
        for row in 0..j.dim {
            j.data[row * j.dim + row].im = Interval::exact(Rat::zero());
            for col in 0..row {
                j.set(row, col, j.get(col, row).conj());
            }
        }
        j
    }
    pub fn partial_trace_output(&self, d: usize, p: u32) -> Self {
        assert_eq!(self.dim, d * d);
        let mut m = Self::zeros(d);
        for i in 0..d {
            for j in 0..d {
                let mut sum = ComplexInterval::zero();
                for a in 0..d {
                    sum = sum.add(self.get(i * d + a, j * d + a), p);
                }
                m.set(i, j, sum);
            }
        }
        m
    }
    pub fn is_hermitian(&self) -> bool {
        for i in 0..self.dim {
            if !self.get(i, i).im.lo.is_zero() || !self.get(i, i).im.hi.is_zero() {
                return false;
            }
            for j in 0..i {
                if self.get(i, j) != &self.get(j, i).conj() {
                    return false;
                }
            }
        }
        true
    }
    /// Hermitian LDL† with a strictly positive lower endpoint for every
    /// pivot. Zero/negative/inconclusive pivots reject, including diag(0,-1).
    pub fn positive_definite(&self, p: u32) -> Result<(), String> {
        if !self.is_hermitian() {
            return Err("matrix is not a Hermitian interval enclosure".into());
        }
        let mut l = Self::identity(self.dim);
        let mut pivots: Vec<Interval> = Vec::new();
        for j in 0..self.dim {
            let mut pivot = self.get(j, j).re.clone();
            for (k, diagonal) in pivots.iter().enumerate() {
                let z = l.get(j, k);
                let abs = z.re.mul(&z.re, p).add(&z.im.mul(&z.im, p), p);
                pivot = pivot.sub(&abs.mul(diagonal, p), p);
            }
            if pivot.lo <= Rat::zero() {
                return Err(format!(
                    "LDL pivot {j} has nonpositive lower endpoint {}",
                    pivot.lo
                ));
            }
            for i in j + 1..self.dim {
                let mut value = self.get(i, j).clone();
                for (k, diagonal) in pivots.iter().enumerate() {
                    value = value.sub(
                        &l.get(i, k).mul(&l.get(j, k).conj(), p).scale(diagonal, p),
                        p,
                    );
                }
                let entry = value
                    .div_real(&pivot, p)
                    .ok_or_else(|| format!("LDL division at pivot {j} is inconclusive"))?;
                l.set(i, j, entry);
            }
            pivots.push(pivot);
        }
        Ok(())
    }
}

/// Verify the three strict dual inequalities of §12.1, plus mu <= bound.
/// No eigenvalue tolerance, determinant heuristic or zero-pivot exception.
pub fn verify_dual(j: &Matrix, y: &Matrix, mu: &Rat, bound: &Rat, p: u32) -> Result<(), String> {
    if mu > bound {
        return Err(format!("dual mu {mu} exceeds claimed bound {bound}"));
    }
    if y.dim != j.dim || !y.is_hermitian() || !j.is_hermitian() {
        return Err(
            "dual and Choi difference must be Hermitian matrices of equal dimension".into(),
        );
    }
    let d = crate::rat::isqrt(&y.dim.into())
        .to_string()
        .parse::<usize>()
        .map_err(|_| "dimension exceeds host")?;
    if d * d != y.dim {
        return Err("Choi dimension is not a square".into());
    }
    y.sub(j, p)
        .positive_definite(p)
        .map_err(|e| format!("Y-J: {e}"))?;
    y.add(j, p)
        .positive_definite(p)
        .map_err(|e| format!("Y+J: {e}"))?;
    Matrix::identity(d)
        .scale(&Interval::point(mu.clone(), p), p)
        .sub(&y.partial_trace_output(d, p), p)
        .positive_definite(p)
        .map_err(|e| format!("mu I-Tr_out(Y): {e}"))
}
