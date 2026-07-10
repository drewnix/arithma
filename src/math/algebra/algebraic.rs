use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// An algebraic number field Q[t]/(m(t)), where m(t) is a monic irreducible
/// polynomial over Q. Elements are represented as coefficient vectors
/// [a₀, a₁, ..., aₙ₋₁] meaning a₀ + a₁α + ... + aₙ₋₁αⁿ⁻¹, where α is
/// a root of m(t).
#[derive(Debug, Clone)]
pub struct NumberField {
    /// Monic minimal polynomial, all coefficients [c₀, c₁, ..., cₙ₋₁].
    /// The leading coefficient (= 1 for tⁿ) is implicit.
    /// So min_poly_coeffs has length n = degree of extension.
    min_poly_coeffs: Vec<BigRational>,
    /// Degree of the extension (= length of min_poly_coeffs).
    pub degree: usize,
    /// f64 approximation of the chosen root.
    root_approx: f64,
}

/// Type alias for field elements: a₀ + a₁α + ... + aₙ₋₁αⁿ⁻¹.
pub type Elem = Vec<BigRational>;

impl NumberField {
    /// Create a number field Q[t]/(m(t)).
    ///
    /// `min_poly_coeffs`: coefficients [c₀, ..., cₙ₋₁] of the monic minimal
    /// polynomial tⁿ + cₙ₋₁tⁿ⁻¹ + ... + c₁t + c₀. Length = degree.
    ///
    /// `root_approx`: f64 approximation of the root being adjoined.
    pub fn new(min_poly_coeffs: Vec<BigRational>, root_approx: f64) -> Self {
        let degree = min_poly_coeffs.len();
        assert!(degree >= 1, "Minimal polynomial must have degree ≥ 1");
        NumberField {
            min_poly_coeffs,
            degree,
            root_approx,
        }
    }

    pub fn zero(&self) -> Elem {
        vec![BigRational::zero(); self.degree]
    }

    pub fn one(&self) -> Elem {
        let mut v = self.zero();
        v[0] = BigRational::one();
        v
    }

    /// The generator α (root of the minimal polynomial).
    pub fn gen(&self) -> Elem {
        let mut v = self.zero();
        if self.degree > 1 {
            v[1] = BigRational::one();
        } else {
            // Degree 1: α = -c₀
            v[0] = -self.min_poly_coeffs[0].clone();
        }
        v
    }

    /// Embed a rational number into the field.
    pub fn from_rational(&self, r: &BigRational) -> Elem {
        let mut v = self.zero();
        v[0] = r.clone();
        v
    }

    pub fn is_zero(&self, a: &[BigRational]) -> bool {
        a.iter().all(|c| c.is_zero())
    }

    pub fn add(&self, a: &[BigRational], b: &[BigRational]) -> Elem {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
    }

    pub fn sub(&self, a: &[BigRational], b: &[BigRational]) -> Elem {
        a.iter().zip(b.iter()).map(|(x, y)| x - y).collect()
    }

    pub fn neg(&self, a: &[BigRational]) -> Elem {
        a.iter().map(|x| -x).collect()
    }

    /// Multiply by a rational scalar.
    pub fn scale(&self, a: &[BigRational], s: &BigRational) -> Elem {
        a.iter().map(|x| x * s).collect()
    }

    /// Multiply two field elements.
    pub fn mul(&self, a: &[BigRational], b: &[BigRational]) -> Elem {
        let n = self.degree;
        // Polynomial multiply: result has degree ≤ 2(n-1)
        let mut product = vec![BigRational::zero(); 2 * n - 1];
        for (i, ai) in a.iter().enumerate() {
            if ai.is_zero() {
                continue;
            }
            for (j, bj) in b.iter().enumerate() {
                product[i + j] = &product[i + j] + &(ai * bj);
            }
        }
        self.reduce(&product)
    }

    /// Square a field element (slightly more efficient than self.mul(a, a)).
    pub fn sqr(&self, a: &[BigRational]) -> Elem {
        self.mul(a, a)
    }

    /// Reduce a polynomial (coefficient vector of arbitrary length) modulo
    /// the minimal polynomial. Returns a vector of length self.degree.
    fn reduce(&self, poly: &[BigRational]) -> Elem {
        let n = self.degree;
        if poly.len() <= n {
            let mut result = poly.to_vec();
            result.resize(n, BigRational::zero());
            return result;
        }

        // min_poly is tⁿ + cₙ₋₁tⁿ⁻¹ + ... + c₀
        // So tⁿ ≡ -(cₙ₋₁tⁿ⁻¹ + ... + c₀) mod min_poly
        let mut result = poly.to_vec();
        for i in (n..result.len()).rev() {
            let coeff = result[i].clone();
            if coeff.is_zero() {
                continue;
            }
            result[i] = BigRational::zero();
            for (j, cj) in self.min_poly_coeffs.iter().enumerate() {
                // tⁱ = t^(i-n) · tⁿ ≡ -t^(i-n) · (cₙ₋₁tⁿ⁻¹+...+c₀)
                result[i - n + j] = &result[i - n + j] - &(&coeff * cj);
            }
        }
        result.truncate(n);
        result
    }

    /// Multiplicative inverse via extended GCD in Q[t].
    /// Returns Err if the element is zero.
    pub fn inv(&self, a: &[BigRational]) -> Result<Elem, String> {
        if self.is_zero(a) {
            return Err("Cannot invert zero element".to_string());
        }

        // Extended GCD of a(t) and m(t) in Q[t].
        // Since m is irreducible and a ≠ 0, gcd = 1, so s(t)·a(t) ≡ 1 mod m(t).
        let a_poly = trim_trailing_zeros(a);
        let mut m_poly = self.min_poly_coeffs.clone();
        m_poly.push(BigRational::one()); // add the implicit leading 1

        let (_, s, _) = poly_extended_gcd(&a_poly, &m_poly);
        Ok(self.reduce(&s))
    }

    pub fn div(&self, a: &[BigRational], b: &[BigRational]) -> Result<Elem, String> {
        let b_inv = self.inv(b)?;
        Ok(self.mul(a, &b_inv))
    }

    /// Evaluate the field element at the root approximation → f64.
    pub fn to_f64(&self, a: &[BigRational]) -> f64 {
        let mut result = 0.0f64;
        let mut power = 1.0f64;
        for coeff in a {
            result += coeff.to_f64().unwrap_or(0.0) * power;
            power *= self.root_approx;
        }
        result
    }

    /// Return the f64 approximation of the generator (root).
    pub fn root_f64(&self) -> f64 {
        self.root_approx
    }

    /// Refine root_approx using Newton's method on the minimal polynomial.
    pub fn refine_root(&mut self, iterations: usize) {
        let n = self.degree;
        let mut x = self.root_approx;
        for _ in 0..iterations {
            // Evaluate min_poly at x: x^n + c_{n-1}x^{n-1} + ... + c_0
            let mut val = 1.0f64;
            for i in (0..n).rev() {
                val = val * x + self.min_poly_coeffs[i].to_f64().unwrap_or(0.0);
            }
            // Evaluate derivative: n·x^{n-1} + (n-1)·c_{n-1}·x^{n-2} + ... + c_1
            let mut dval = n as f64;
            for i in (1..n).rev() {
                dval = dval * x + (i as f64) * self.min_poly_coeffs[i].to_f64().unwrap_or(0.0);
            }
            if dval.abs() < 1e-30 {
                break;
            }
            x -= val / dval;
        }
        self.root_approx = x;
    }
}

// --- Polynomial arithmetic over Q (for extended GCD) ---

fn trim_trailing_zeros(p: &[BigRational]) -> Vec<BigRational> {
    let mut v = p.to_vec();
    while v.last().is_some_and(|c| c.is_zero()) {
        v.pop();
    }
    v
}

fn poly_degree(p: &[BigRational]) -> Option<usize> {
    let t = trim_trailing_zeros(p);
    if t.is_empty() {
        None
    } else {
        Some(t.len() - 1)
    }
}

fn poly_is_zero(p: &[BigRational]) -> bool {
    p.iter().all(|c| c.is_zero())
}

/// Polynomial long division over Q. Returns (quotient, remainder).
fn poly_div_rem(a: &[BigRational], b: &[BigRational]) -> (Vec<BigRational>, Vec<BigRational>) {
    let b = trim_trailing_zeros(b);
    if b.is_empty() {
        panic!("Division by zero polynomial");
    }
    let mut rem = a.to_vec();
    let b_deg = b.len() - 1;
    let b_lc = &b[b_deg];

    let a_deg = match poly_degree(&rem) {
        Some(d) if d >= b_deg => d,
        _ => return (vec![], rem),
    };

    let mut quot = vec![BigRational::zero(); a_deg - b_deg + 1];

    while let Some(r_deg) = poly_degree(&rem) {
        if r_deg < b_deg {
            break;
        }
        let r_lc = rem[r_deg].clone();
        let q = &r_lc / b_lc;
        let shift = r_deg - b_deg;
        quot[shift] = q.clone();
        for (i, bi) in b.iter().enumerate() {
            rem[shift + i] = &rem[shift + i] - &(&q * bi);
        }
    }

    (quot, trim_trailing_zeros(&rem))
}

/// Extended GCD in Q[t]: returns (gcd, s, t) such that s·a + t·b = gcd.
/// The gcd is made monic.
fn poly_extended_gcd(
    a: &[BigRational],
    b: &[BigRational],
) -> (Vec<BigRational>, Vec<BigRational>, Vec<BigRational>) {
    let a = trim_trailing_zeros(a);
    let b = trim_trailing_zeros(b);

    if b.is_empty() {
        if a.is_empty() {
            return (vec![], vec![BigRational::one()], vec![]);
        }
        let lc = a.last().unwrap().clone();
        let s = vec![BigRational::one() / &lc];
        let a_monic: Vec<_> = a.iter().map(|c| c / &lc).collect();
        return (a_monic, s, vec![]);
    }

    let mut old_r = a.clone();
    let mut r = b.clone();
    let mut old_s = vec![BigRational::one()];
    let mut s: Vec<BigRational> = vec![];
    let mut old_t: Vec<BigRational> = vec![];
    let mut t = vec![BigRational::one()];

    while !poly_is_zero(&r) {
        let (q, rem) = poly_div_rem(&old_r, &r);
        old_r = r;
        r = rem;

        let new_s = poly_sub(&old_s, &poly_mul_raw(&q, &s));
        old_s = s;
        s = new_s;

        let new_t = poly_sub(&old_t, &poly_mul_raw(&q, &t));
        old_t = t;
        t = new_t;
    }

    // Make monic
    let old_r = trim_trailing_zeros(&old_r);
    if let Some(lc) = old_r.last() {
        if !lc.is_zero() && !lc.is_one() {
            let inv_lc = BigRational::one() / lc;
            let gcd: Vec<_> = old_r.iter().map(|c| c * &inv_lc).collect();
            let s: Vec<_> = old_s.iter().map(|c| c * &inv_lc).collect();
            let t: Vec<_> = old_t.iter().map(|c| c * &inv_lc).collect();
            return (gcd, s, t);
        }
    }

    (old_r, old_s, old_t)
}

fn poly_sub(a: &[BigRational], b: &[BigRational]) -> Vec<BigRational> {
    let len = a.len().max(b.len());
    let mut result = vec![BigRational::zero(); len];
    for (i, r) in result.iter_mut().enumerate() {
        let ai = a.get(i).cloned().unwrap_or_else(BigRational::zero);
        let bi = b.get(i).cloned().unwrap_or_else(BigRational::zero);
        *r = ai - bi;
    }
    trim_trailing_zeros(&result)
}

fn poly_mul_raw(a: &[BigRational], b: &[BigRational]) -> Vec<BigRational> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut result = vec![BigRational::zero(); a.len() + b.len() - 1];
    for (i, ai) in a.iter().enumerate() {
        if ai.is_zero() {
            continue;
        }
        for (j, bj) in b.iter().enumerate() {
            result[i + j] = &result[i + j] + &(ai * bj);
        }
    }
    result
}

/// Find a real root of a polynomial using Newton's method, starting from initial guess.
pub fn find_real_root(coeffs: &[BigRational], initial_guess: f64, iterations: usize) -> f64 {
    let fc: Vec<f64> = coeffs.iter().map(|c| c.to_f64().unwrap_or(0.0)).collect();
    let mut x = initial_guess;
    for _ in 0..iterations {
        let mut val = 0.0;
        let mut dval = 0.0;
        let mut xp = 1.0;
        for (i, &ci) in fc.iter().enumerate() {
            val += ci * xp;
            if i > 0 {
                dval += (i as f64) * ci * xp / x;
            }
            xp *= x;
        }
        if dval.abs() < 1e-30 {
            break;
        }
        x -= val / dval;
    }
    x
}

/// Try to find a rational root of the polynomial c₀ + c₁t + ... + cₙtⁿ.
/// Uses the rational root theorem: candidates are ±(factors of c₀)/(factors of cₙ).
pub fn try_rational_root(coeffs: &[BigRational]) -> Option<BigRational> {
    let coeffs = trim_trailing_zeros(coeffs);
    if coeffs.is_empty() {
        return Some(BigRational::zero());
    }
    if coeffs.len() == 1 {
        return None; // nonzero constant
    }

    // Convert to integer coefficients by clearing denominators
    let mut lcm_denom = BigInt::one();
    for c in &coeffs {
        lcm_denom = lcm_big(&lcm_denom, c.denom());
    }
    let int_coeffs: Vec<BigInt> = coeffs
        .iter()
        .map(|c| (c * &BigRational::from_integer(lcm_denom.clone())).to_integer())
        .collect();

    let c0 = &int_coeffs[0];
    let cn = int_coeffs.last().unwrap();

    if c0.is_zero() {
        return Some(BigRational::zero());
    }

    let c0_factors = small_factors(&c0.abs());
    let cn_factors = small_factors(&cn.abs());

    for p in &c0_factors {
        for q in &cn_factors {
            for sign in &[1i64, -1i64] {
                let candidate = BigRational::new(BigInt::from(*sign) * p, q.clone());
                if eval_poly_rational(&coeffs, &candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn eval_poly_rational(coeffs: &[BigRational], x: &BigRational) -> BigRational {
    let mut result = BigRational::zero();
    let mut power = BigRational::one();
    for c in coeffs {
        result += c * &power;
        power = &power * x;
    }
    result
}

fn lcm_big(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_zero() || b.is_zero() {
        return BigInt::zero();
    }
    let g = gcd_big(a, b);
    (a / &g) * b
}

fn gcd_big(a: &BigInt, b: &BigInt) -> BigInt {
    let mut a = a.abs();
    let mut b = b.abs();
    while !b.is_zero() {
        let t = b.clone();
        b = &a % &b;
        a = t;
    }
    a
}

fn small_factors(n: &BigInt) -> Vec<BigInt> {
    if n.is_zero() {
        return vec![BigInt::one()];
    }
    let n_abs = n.abs();
    let mut factors = Vec::new();
    let mut i = BigInt::one();
    while &i * &i <= n_abs {
        if (&n_abs % &i).is_zero() {
            let other = &n_abs / &i;
            factors.push(i.clone());
            if other != i {
                factors.push(other);
            }
        }
        i += BigInt::one();
        // Cap at a reasonable number of factors to avoid slowdown
        if factors.len() > 200 {
            break;
        }
    }
    if factors.is_empty() {
        factors.push(BigInt::one());
    }
    factors
}

/// Solve a dense n×n linear system over the number field using Gaussian elimination.
/// `matrix` is stored row-major: matrix[i][j] is the element at row i, column j.
/// `rhs` is the right-hand side vector.
/// Returns the solution vector, or Err if the system is singular.
pub fn solve_linear_system(
    nf: &NumberField,
    matrix: &[Vec<Elem>],
    rhs: &[Elem],
) -> Result<Vec<Elem>, String> {
    let n = matrix.len();
    assert_eq!(rhs.len(), n);

    // Augmented matrix: [A | b]
    let mut aug: Vec<Vec<Elem>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = matrix[i].clone();
        row.push(rhs[i].clone());
        aug.push(row);
    }

    // Forward elimination with partial pivoting
    for col in 0..n {
        // Find pivot
        let pivot_row = (col..n)
            .find(|&r| !nf.is_zero(&aug[r][col]))
            .ok_or_else(|| "Singular matrix in linear system".to_string())?;

        if pivot_row != col {
            aug.swap(col, pivot_row);
        }

        let pivot_inv = nf.inv(&aug[col][col])?;

        // Scale pivot row
        for val in &mut aug[col][col..=n] {
            *val = nf.mul(val, &pivot_inv);
        }

        // Eliminate column in other rows
        for i in 0..n {
            if i == col {
                continue;
            }
            let factor = aug[i][col].clone();
            if nf.is_zero(&factor) {
                continue;
            }
            // Can't borrow aug[i] and aug[col] simultaneously, so clone the pivot row segment
            let pivot_row: Vec<_> = aug[col][col..=n].to_vec();
            for (j, pj) in (col..=n).zip(pivot_row.iter()) {
                let sub = nf.mul(&factor, pj);
                aug[i][j] = nf.sub(&aug[i][j], &sub);
            }
        }
    }

    // Extract solution from augmented matrix
    Ok(aug
        .into_iter()
        .map(|row| row.into_iter().last().unwrap())
        .collect())
}

// --- Polynomials over Q(α) for Risch coefficient field extension ---

/// Univariate polynomial over an algebraic number field Q(α).
/// Coefficients are elements of Q(α), stored least-degree first.
#[derive(Debug, Clone)]
pub struct AlgPoly {
    pub coeffs: Vec<Elem>,
    field: NumberField,
    variable: String,
}

impl AlgPoly {
    pub fn zero(field: &NumberField, var: &str) -> Self {
        AlgPoly {
            coeffs: vec![],
            field: field.clone(),
            variable: var.to_string(),
        }
    }

    pub fn one(field: &NumberField, var: &str) -> Self {
        AlgPoly {
            coeffs: vec![field.one()],
            field: field.clone(),
            variable: var.to_string(),
        }
    }

    pub fn constant(c: Elem, field: &NumberField, var: &str) -> Self {
        if field.is_zero(&c) {
            Self::zero(field, var)
        } else {
            AlgPoly {
                coeffs: vec![c],
                field: field.clone(),
                variable: var.to_string(),
            }
        }
    }

    pub fn from_coeffs(coeffs: Vec<Elem>, field: &NumberField, var: &str) -> Self {
        let mut p = AlgPoly {
            coeffs,
            field: field.clone(),
            variable: var.to_string(),
        };
        p.strip_trailing();
        p
    }

    /// The variable x as a polynomial: 0 + 1·x
    pub fn x_poly(field: &NumberField, var: &str) -> Self {
        AlgPoly {
            coeffs: vec![field.zero(), field.one()],
            field: field.clone(),
            variable: var.to_string(),
        }
    }

    fn strip_trailing(&mut self) {
        while self.coeffs.last().is_some_and(|c| self.field.is_zero(c)) {
            self.coeffs.pop();
        }
    }

    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn degree(&self) -> Option<usize> {
        if self.coeffs.is_empty() {
            None
        } else {
            Some(self.coeffs.len() - 1)
        }
    }

    pub fn leading_coeff(&self) -> Option<&Elem> {
        self.coeffs.last()
    }

    pub fn coeff(&self, i: usize) -> Elem {
        self.coeffs
            .get(i)
            .cloned()
            .unwrap_or_else(|| self.field.zero())
    }

    pub fn field(&self) -> &NumberField {
        &self.field
    }

    pub fn add(&self, other: &AlgPoly) -> AlgPoly {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = other.coeff(i);
            coeffs.push(self.field.add(&a, &b));
        }
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }

    pub fn sub(&self, other: &AlgPoly) -> AlgPoly {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = other.coeff(i);
            coeffs.push(self.field.sub(&a, &b));
        }
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }

    pub fn mul(&self, other: &AlgPoly) -> AlgPoly {
        if self.is_zero() || other.is_zero() {
            return Self::zero(&self.field, &self.variable);
        }
        let len = self.coeffs.len() + other.coeffs.len() - 1;
        let mut coeffs: Vec<Elem> = (0..len).map(|_| self.field.zero()).collect();
        for (i, ai) in self.coeffs.iter().enumerate() {
            if self.field.is_zero(ai) {
                continue;
            }
            for (j, bj) in other.coeffs.iter().enumerate() {
                let prod = self.field.mul(ai, bj);
                coeffs[i + j] = self.field.add(&coeffs[i + j], &prod);
            }
        }
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }

    pub fn scalar_mul(&self, s: &Elem) -> AlgPoly {
        let coeffs = self.coeffs.iter().map(|c| self.field.mul(c, s)).collect();
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }

    pub fn neg(&self) -> AlgPoly {
        let coeffs = self.coeffs.iter().map(|c| self.field.neg(c)).collect();
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }

    pub fn make_monic(&self) -> Result<AlgPoly, String> {
        let lc = self.leading_coeff().ok_or("Zero polynomial")?;
        let lc_inv = self.field.inv(lc)?;
        Ok(self.scalar_mul(&lc_inv))
    }

    /// Polynomial long division: self = quotient * divisor + remainder.
    pub fn div_rem(&self, divisor: &AlgPoly) -> Result<(AlgPoly, AlgPoly), String> {
        if divisor.is_zero() {
            return Err("Division by zero polynomial".to_string());
        }
        let d_deg = divisor.degree().unwrap();
        let d_lc = divisor.leading_coeff().unwrap();
        let d_lc_inv = self.field.inv(d_lc)?;

        let mut remainder = self.clone();
        let s_deg = match self.degree() {
            Some(d) if d >= d_deg => d,
            _ => return Ok((Self::zero(&self.field, &self.variable), self.clone())),
        };

        let mut q_coeffs: Vec<Elem> = (0..=s_deg - d_deg).map(|_| self.field.zero()).collect();

        while let Some(r_deg) = remainder.degree() {
            if r_deg < d_deg {
                break;
            }
            let r_lc = remainder.leading_coeff().unwrap().clone();
            let q_coeff = self.field.mul(&r_lc, &d_lc_inv);
            let deg_diff = r_deg - d_deg;
            q_coeffs[deg_diff] = q_coeff.clone();

            for (j, dj) in divisor.coeffs.iter().enumerate() {
                let sub = self.field.mul(&q_coeff, dj);
                remainder.coeffs[deg_diff + j] =
                    self.field.sub(&remainder.coeffs[deg_diff + j], &sub);
            }
            remainder.strip_trailing();
        }

        Ok((
            AlgPoly::from_coeffs(q_coeffs, &self.field, &self.variable),
            remainder,
        ))
    }

    /// GCD via Euclidean algorithm. Result is monic.
    pub fn gcd(&self, other: &AlgPoly) -> Result<AlgPoly, String> {
        if self.is_zero() {
            return if other.is_zero() {
                Ok(Self::zero(&self.field, &self.variable))
            } else {
                other.make_monic()
            };
        }
        if other.is_zero() {
            return self.make_monic();
        }

        let mut a = self.clone();
        let mut b = other.clone();
        if a.degree() < b.degree() {
            std::mem::swap(&mut a, &mut b);
        }

        while !b.is_zero() {
            let (_, r) = a.div_rem(&b)?;
            a = b;
            b = r;
        }
        a.make_monic()
    }

    /// Formal derivative d/dx.
    pub fn derivative(&self) -> AlgPoly {
        if self.coeffs.len() <= 1 {
            return Self::zero(&self.field, &self.variable);
        }
        let mut coeffs = Vec::with_capacity(self.coeffs.len() - 1);
        for (i, c) in self.coeffs.iter().enumerate().skip(1) {
            let scalar = BigRational::from_integer(BigInt::from(i));
            coeffs.push(self.field.scale(c, &scalar));
        }
        AlgPoly::from_coeffs(coeffs, &self.field, &self.variable)
    }
}

/// Hermite reduction over Q(α)(x): given a/d where a, d are polynomials
/// over Q(α), returns (g, a_red, d_red) such that ∫a/d = g + ∫a_red/d_red
/// where d_red is squarefree.
///
/// This is the core Risch operation, extended to work with algebraic
/// coefficient fields via the NumberField infrastructure.
pub fn hermite_reduce_algebraic(
    a: &AlgPoly,
    d: &AlgPoly,
) -> Result<(AlgPoly, AlgPoly, AlgPoly, AlgPoly), String> {
    let nf = a.field();
    let var = &a.variable;

    let d_deriv = d.derivative();
    let g = d.gcd(&d_deriv)?;

    if g.degree().unwrap_or(0) == 0 {
        // d is already squarefree — nothing to reduce
        return Ok((
            AlgPoly::zero(nf, var),
            AlgPoly::one(nf, var),
            a.clone(),
            d.clone(),
        ));
    }

    // d = d_s * d_m² * ... where d_s is squarefree part
    let (d_star, _) = d.div_rem(&g)?; // d* = d/gcd(d,d')

    // Hermite's formula: for d = d_star * v where v = gcd(d, d'):
    //   ∫a/d = -b/(k·v) + ∫(a_new)/(d_star·v_new) where k = multiplicity
    // For simplicity, apply one step of reduction.

    // Extended GCD approach: find b, c such that b·(d*/dx v) + c·v_star = a/(k)
    // where v_star = d_star, v = g.
    // Actually, let me use the standard one-step Hermite reduction:
    //   d = d_star · g, where g = gcd(d, d')
    //   Then ∫a/d = ∫a/(d_star·g)
    //   If g has degree > 0, solve: a = b'·g + b·(-g'·d_star/(deg) + ...) + c·d_star
    //
    // Simpler approach: iterative square-free reduction.
    // Split d = ∏ dᵢ^i (square-free decomposition)
    // For each factor with multiplicity > 1, reduce.

    // For demonstration, handle the common case: d = (something)², single factor.
    // ∫a/p² = -b/p + ∫c/p where b,c satisfy a = -b'p + bp' + cp (from integration by parts)

    // More precisely: ∫a/(p^k) for k > 1.
    // Set a = b'·p - (k-1)·b·p' + c·p^{k-1}·(something)
    // The Hermite formula: find b, c ∈ Q(α)[x] with deg(b) < deg(p), deg(c) < deg(p) such that
    //   a = -b'·p + (k-1)·b·p' + c·p   (when d = p^k with p squarefree)
    // This is the extended Euclidean algorithm on (p, p') applied to a.

    // We already have the squarefree part d_star and g = d/d_star.
    // If g = d_star (i.e., d = d_star²), then k=2 and p = d_star.
    // General case: d = d_star * g where g = gcd(d, d').
    // Apply one reduction step: ∫a/(d_star·g) = b/g + ∫c/d_star
    // where a = b'·d_star - b·((k-1)/k)·d_star' + c·g for appropriate k.

    // For the demonstration, use the direct formula for one Hermite step:
    // Given ∫a/(d_star·g) where d_star is squarefree and g = gcd(d,d'):
    //   Find b (deg < deg(g)) and c (deg < deg(d_star)) such that
    //   a = b' · d_star + b · (d_star' · (-g/g_check) ) + c · g
    // where g_check adjusts for multiplicity.

    // Actually, the cleanest one-step Hermite reduction:
    // d₁ = gcd(d, d'), d₂ = d/d₁
    // Then ∫a/d = -b/d₁ + ∫c/d₂
    // where b, c are found from: a = -b'·(d/d₁) + b·(d/d₁)' + c·d₁
    // Equivalently: a = -b'·d₂ + b·d₂' + c·d₁

    // Solve for b and c using the extended Euclidean algorithm.
    // Since gcd(d₂, d₂') divides gcd(d₂, d₁) = 1 (d₂ is squarefree? Not necessarily after one step)
    // Actually d₂ = d/d₁ = d/gcd(d,d'). We need repeated reduction if d₁ has repeated factors.

    // For a clean one-step reduction:
    let d2 = d_star; // d/gcd(d,d')
    let d1 = g; // gcd(d,d')
    let d2_deriv = d2.derivative();

    // We need: a = -b'·d₂ + b·d₂' + c·d₁
    // This is equivalent to: a ≡ b·d₂' (mod d₁) when we consider b mod d₁
    // (since -b'·d₂ ≡ 0 mod d₁ is not guaranteed, we use full extended GCD)

    // Extended GCD: gcd(d₂', d₁) should be 1 (since d₁·d₂ = d, d₂' involves d₂).
    // Actually this is subtle. Let me just use the direct solve:
    // Reduce modulo d₁: a mod d₁ = b · (d₂' mod d₁) mod d₁
    // Then b = a · (d₂')⁻¹ mod d₁
    let (_, a_mod_d1) = a.div_rem(&d1)?;
    let (_, d2p_mod_d1) = d2_deriv.div_rem(&d1)?;

    // Solve: b ≡ a · (d₂')⁻¹ mod d₁
    // This requires d₂' and d₁ to be coprime mod d₁.
    // For a proper Hermite reduction this should hold.
    // Use AlgPoly division in the quotient ring Q(α)[x]/(d₁).
    if d2p_mod_d1.is_zero() {
        return Err("Degenerate Hermite reduction: d₂' ≡ 0 mod d₁".to_string());
    }

    // For the quotient ring inversion, we use the polynomial extended GCD.
    // Since d₁ divides gcd(d,d'), and d₂'  has specific structure, this should work.
    // For now, compute b by solving b·d₂' ≡ a (mod d₁) directly.
    let b_mod = alg_poly_mod_solve(&a_mod_d1, &d2p_mod_d1, &d1)?;

    // b' · d₂
    let b_deriv = b_mod.derivative();
    let b_prime_d2 = b_deriv.mul(&d2);

    // b · d₂'
    let b_d2_prime = b_mod.mul(&d2_deriv);

    // c · d₁ = a + b'·d₂ - b·d₂'
    let c_times_d1 = a.add(&b_prime_d2).sub(&b_d2_prime);
    let (c, remainder) = c_times_d1.div_rem(&d1)?;

    if !remainder.is_zero() {
        return Err("Hermite reduction: remainder in c computation is nonzero".to_string());
    }

    // Result: ∫a/d = -b/d₁ + ∫c/d₂
    Ok((b_mod.neg(), d1, c, d2))
}

/// Solve b·f ≡ a (mod m) for AlgPoly, where f and m are coprime.
fn alg_poly_mod_solve(a: &AlgPoly, f: &AlgPoly, m: &AlgPoly) -> Result<AlgPoly, String> {
    // Extended GCD: s·f + t·m = gcd(f, m) = 1 (since coprime)
    // Then b = s·a mod m
    let (gcd, s, _) = alg_poly_extended_gcd(f, m)?;

    // Verify gcd is constant (should be 1 for coprime)
    if gcd.degree().unwrap_or(0) != 0 {
        return Err("Polynomials not coprime in mod_solve".to_string());
    }

    // b = s · a mod m
    let sa = s.mul(a);
    let (_, result) = sa.div_rem(m)?;
    Ok(result)
}

/// Extended GCD for AlgPoly: returns (gcd, s, t) such that s·a + t·b = gcd.
fn alg_poly_extended_gcd(a: &AlgPoly, b: &AlgPoly) -> Result<(AlgPoly, AlgPoly, AlgPoly), String> {
    let nf = a.field();
    let var = &a.variable;

    if b.is_zero() {
        if a.is_zero() {
            return Ok((
                AlgPoly::zero(nf, var),
                AlgPoly::one(nf, var),
                AlgPoly::zero(nf, var),
            ));
        }
        let a_monic = a.make_monic()?;
        let lc_inv = nf.inv(a.leading_coeff().unwrap())?;
        return Ok((
            a_monic,
            AlgPoly::constant(lc_inv, nf, var),
            AlgPoly::zero(nf, var),
        ));
    }

    let mut old_r = a.clone();
    let mut r = b.clone();
    let mut old_s = AlgPoly::one(nf, var);
    let mut s = AlgPoly::zero(nf, var);
    let mut old_t = AlgPoly::zero(nf, var);
    let mut t = AlgPoly::one(nf, var);

    while !r.is_zero() {
        let (q, rem) = old_r.div_rem(&r)?;
        old_r = r;
        r = rem;
        let new_s = old_s.sub(&q.mul(&s));
        old_s = s;
        s = new_s;
        let new_t = old_t.sub(&q.mul(&t));
        old_t = t;
        t = new_t;
    }

    let monic = old_r.make_monic()?;
    let lc_inv = nf.inv(old_r.leading_coeff().unwrap())?;
    Ok((monic, old_s.scalar_mul(&lc_inv), old_t.scalar_mul(&lc_inv)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rat(n: i64, d: i64) -> BigRational {
        BigRational::new(BigInt::from(n), BigInt::from(d))
    }

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    #[test]
    fn test_sqrt2_field() {
        // Q(√2): minimal polynomial t² - 2, root ≈ 1.41421
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        assert_eq!(nf.degree, 2);

        let one = nf.one();
        let alpha = nf.gen(); // √2

        // (1 + √2)(1 - √2) = 1 - 2 = -1
        let one_plus = nf.add(&one, &alpha);
        let one_minus = nf.sub(&one, &alpha);
        let product = nf.mul(&one_plus, &one_minus);
        assert_eq!(product[0], int(-1));
        assert!(product[1].is_zero());

        // √2 · √2 = 2
        let sq = nf.sqr(&alpha);
        assert_eq!(sq[0], int(2));
        assert!(sq[1].is_zero());

        // 1/(1+√2) = -1+√2
        let inv = nf.inv(&one_plus).unwrap();
        assert_eq!(inv[0], int(-1));
        assert_eq!(inv[1], int(1));

        // Verify: (1+√2)(-1+√2) = -1+√2-√2+2 = 1
        let check = nf.mul(&one_plus, &inv);
        assert_eq!(check, nf.one());
    }

    #[test]
    fn test_cbrt2_field() {
        // Q(∛2): minimal polynomial t³ - 2, root ≈ 1.2599
        let nf = NumberField::new(vec![int(-2), int(0), int(0)], 2.0f64.cbrt());
        assert_eq!(nf.degree, 3);

        let beta = nf.gen(); // ∛2

        // ∛2 · (∛2)² = (∛2)³ = 2
        let beta_sq = nf.sqr(&beta);
        let beta_cube = nf.mul(&beta, &beta_sq);
        assert_eq!(beta_cube[0], int(2));
        assert!(beta_cube[1].is_zero());
        assert!(beta_cube[2].is_zero());

        // 1/(1+∛2) = (1-∛2+(∛2)²)/3
        // Verify: (1+β)(1-β+β²) = 1-β+β²+β-β²+β³ = 1+2 = 3
        let one = nf.one();
        let one_plus_beta = nf.add(&one, &beta);
        let inv = nf.inv(&one_plus_beta).unwrap();
        // Expected: [1/3, -1/3, 1/3]
        assert_eq!(inv[0], rat(1, 3));
        assert_eq!(inv[1], rat(-1, 3));
        assert_eq!(inv[2], rat(1, 3));

        // Verify round-trip
        let check = nf.mul(&one_plus_beta, &inv);
        assert_eq!(check, nf.one());
    }

    #[test]
    fn test_degree6_field() {
        // Q(s) where s⁶ = 4s² + 1 (for x⁴+x+1 quartic)
        // min poly: t⁶ - 4t² - 1 = 0
        // coeffs of t⁰..t⁵: [-1, 0, -4, 0, 0, 0]
        let s_approx = find_real_root(
            &[int(-1), int(0), int(-4), int(0), int(0), int(0), int(1)],
            1.5,
            50,
        );
        let nf = NumberField::new(
            vec![int(-1), int(0), int(-4), int(0), int(0), int(0)],
            s_approx,
        );
        assert_eq!(nf.degree, 6);

        let s = nf.gen();

        // s · (s⁵ - 4s) should equal 1
        // s⁵: [0, 0, 0, 0, 0, 1]
        let s5 = {
            let s2 = nf.sqr(&s);
            let s4 = nf.sqr(&s2);
            nf.mul(&s4, &s)
        };
        let four_s = nf.scale(&s, &int(4));
        let s5_minus_4s = nf.sub(&s5, &four_s);
        let product = nf.mul(&s, &s5_minus_4s);
        assert_eq!(product, nf.one());

        // Test inverse of s
        let s_inv = nf.inv(&s).unwrap();
        let check = nf.mul(&s, &s_inv);
        assert_eq!(check, nf.one());

        // to_f64 should agree with direct computation
        let s_f64 = nf.to_f64(&s);
        assert!((s_f64 - s_approx).abs() < 1e-10);
    }

    #[test]
    fn test_to_f64() {
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        // 3 + 2√2 ≈ 3 + 2.828 = 5.828
        let elem = vec![int(3), int(2)];
        let val = nf.to_f64(&elem);
        assert!((val - (3.0 + 2.0 * std::f64::consts::SQRT_2)).abs() < 1e-10);
    }

    #[test]
    fn test_rational_root_finding() {
        // t² - 1 has roots ±1
        let coeffs = vec![int(-1), int(0), int(1)];
        let root = try_rational_root(&coeffs);
        assert!(root.is_some());
        let r = root.unwrap();
        assert!(r == int(1) || r == int(-1));

        // t³ - 4t - 1 has no rational roots
        let coeffs2 = vec![int(-1), int(-4), int(0), int(1)];
        assert!(try_rational_root(&coeffs2).is_none());
    }

    #[test]
    fn test_linear_system() {
        // Simple 2x2 over Q(√2):
        // x + √2·y = 1
        // √2·x + y = √2
        // Solution: x = 0, y = 1/√2 = √2/2
        // Actually: from eq1: x = 1 - √2·y
        // Substitute: √2(1-√2y) + y = √2 → √2 - 2y + y = √2 → -y = 0 → y = 0
        // Then x = 1. Hmm wait:
        // √2·x + y = √2, with x=1: √2+y = √2 → y = 0.
        // Solution: x=1, y=0.
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen();
        let one = nf.one();
        let zero = nf.zero();

        let matrix = vec![
            vec![one.clone(), alpha.clone()],
            vec![alpha.clone(), one.clone()],
        ];
        let rhs = vec![one.clone(), alpha.clone()];
        let solution = solve_linear_system(&nf, &matrix, &rhs).unwrap();

        assert_eq!(solution[0], one);
        assert_eq!(solution[1], zero);
    }

    #[test]
    fn test_find_real_root() {
        // t² - 2 = 0, root near 1.4
        let coeffs = vec![int(-2), int(0), int(1)];
        let root = find_real_root(&coeffs, 1.4, 50);
        assert!((root - std::f64::consts::SQRT_2).abs() < 1e-12);
    }

    #[test]
    fn test_poly_extended_gcd() {
        // gcd(t+1, t²-1) = t+1
        let a = vec![int(1), int(1)]; // t+1
        let b = vec![int(-1), int(0), int(1)]; // t²-1
        let (gcd, s, t) = poly_extended_gcd(&a, &b);
        // gcd should be t+1 (monic)
        assert_eq!(gcd.len(), 2);
        assert_eq!(gcd[0], int(1));
        assert_eq!(gcd[1], int(1));
        // Verify s*a + t*b = gcd
        let sa = poly_mul_raw(&s, &a);
        let tb = poly_mul_raw(&t, &b);
        let sum: Vec<BigRational> = {
            let len = sa.len().max(tb.len());
            (0..len)
                .map(|i| {
                    let ai = sa.get(i).cloned().unwrap_or_else(BigRational::zero);
                    let bi = tb.get(i).cloned().unwrap_or_else(BigRational::zero);
                    ai + bi
                })
                .collect()
        };
        let sum = trim_trailing_zeros(&sum);
        assert_eq!(sum, gcd);
    }

    #[test]
    fn test_poly_div_rem() {
        // (t²-1) / (t+1) = (t-1), remainder 0
        let a = vec![int(-1), int(0), int(1)];
        let b = vec![int(1), int(1)];
        let (q, r) = poly_div_rem(&a, &b);
        assert_eq!(q, vec![int(-1), int(1)]);
        assert!(r.is_empty());
    }

    // --- AlgPoly tests ---

    #[test]
    fn test_algpoly_arithmetic_over_sqrt2() {
        // Polynomials over Q(√2)
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen(); // √2

        // p = x + √2, q = x - √2
        let p = AlgPoly::from_coeffs(vec![alpha.clone(), nf.one()], &nf, "x");
        let q = AlgPoly::from_coeffs(vec![nf.neg(&alpha), nf.one()], &nf, "x");

        // p * q = x² - 2
        let product = p.mul(&q);
        assert_eq!(product.degree(), Some(2));
        assert_eq!(product.coeff(0), nf.from_rational(&int(-2)));
        assert!(nf.is_zero(&product.coeff(1)));
        assert_eq!(product.coeff(2), nf.one());
    }

    #[test]
    fn test_algpoly_div_rem_over_sqrt2() {
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen();

        // (x² - 2) / (x + √2) = (x - √2), remainder 0
        let x2_minus_2 = AlgPoly::from_coeffs(
            vec![nf.from_rational(&int(-2)), nf.zero(), nf.one()],
            &nf,
            "x",
        );
        let x_plus_sqrt2 = AlgPoly::from_coeffs(vec![alpha.clone(), nf.one()], &nf, "x");
        let (q, r) = x2_minus_2.div_rem(&x_plus_sqrt2).unwrap();

        assert!(r.is_zero());
        assert_eq!(q.degree(), Some(1));
        // q should be x - √2
        assert_eq!(q.coeff(0), nf.neg(&alpha));
        assert_eq!(q.coeff(1), nf.one());
    }

    #[test]
    fn test_algpoly_gcd_over_sqrt2() {
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen();

        // gcd(x²-2, (x+√2)²) = x+√2 (monic)
        let x2_minus_2 = AlgPoly::from_coeffs(
            vec![nf.from_rational(&int(-2)), nf.zero(), nf.one()],
            &nf,
            "x",
        );
        let x_plus_sqrt2 = AlgPoly::from_coeffs(vec![alpha.clone(), nf.one()], &nf, "x");
        let sq = x_plus_sqrt2.mul(&x_plus_sqrt2);

        let g = x2_minus_2.gcd(&sq).unwrap();
        assert_eq!(g.degree(), Some(1));
        assert_eq!(g.coeff(1), nf.one());
        assert_eq!(g.coeff(0), alpha);
    }

    #[test]
    fn test_algpoly_derivative() {
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen();

        // d/dx (x² + √2·x + 3) = 2x + √2
        let p = AlgPoly::from_coeffs(
            vec![nf.from_rational(&int(3)), alpha.clone(), nf.one()],
            &nf,
            "x",
        );
        let dp = p.derivative();
        assert_eq!(dp.degree(), Some(1));
        assert_eq!(dp.coeff(0), alpha);
        assert_eq!(dp.coeff(1), nf.from_rational(&int(2)));
    }

    #[test]
    fn test_hermite_reduce_over_sqrt2() {
        // Hermite reduction of 1/(x+√2)² over Q(√2)
        // Expected: ∫1/(x+√2)² = -1/(x+√2)
        // So: rational part = -1/(x+√2), reduced integral = 0
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);
        let alpha = nf.gen();

        // numerator a = 1
        let a = AlgPoly::one(&nf, "x");
        // denominator d = (x+√2)² = x² + 2√2·x + 2
        let x_plus_sqrt2 = AlgPoly::from_coeffs(vec![alpha.clone(), nf.one()], &nf, "x");
        let d = x_plus_sqrt2.mul(&x_plus_sqrt2);

        let (b, d1, c, d2) = hermite_reduce_algebraic(&a, &d).unwrap();

        // After Hermite reduction: ∫1/(x+√2)² = b/d₁ + ∫c/d₂
        // d₁ = gcd(d, d') = x+√2 (since d = (x+√2)², d' = 2(x+√2))
        assert_eq!(d1.degree(), Some(1));
        assert_eq!(d1.coeff(0), alpha);

        // d₂ = d/d₁ = x+√2 (squarefree)
        assert_eq!(d2.degree(), Some(1));

        // c/d₂ should be 0 (the integral has no log part)
        assert!(c.is_zero(), "Reduced numerator should be zero for 1/(x+a)²");

        // b/d₁ = -1/(x+√2), so b = -1
        assert_eq!(b.degree(), Some(0));
        // b should be -1
        assert_eq!(b.coeff(0), nf.from_rational(&int(-1)));

        // Verify numerically: at x=1, b/(d₁) = -1/(1+√2) ≈ -0.4142
        let b_val = nf.to_f64(&b.coeff(0));
        let d1_at_1 = 1.0 + std::f64::consts::SQRT_2;
        let rational_part = b_val / d1_at_1;
        assert!(
            (rational_part - (-1.0 / (1.0 + std::f64::consts::SQRT_2))).abs() < 1e-10,
            "Numerical check: b/d₁ at x=1 = {}, expected {}",
            rational_part,
            -1.0 / (1.0 + std::f64::consts::SQRT_2)
        );
    }

    #[test]
    fn test_hermite_reduce_squarefree_noop() {
        // For a squarefree denominator, Hermite reduction is a no-op
        let nf = NumberField::new(vec![int(-2), int(0)], std::f64::consts::SQRT_2);

        // ∫1/(x²+1) — denominator is already squarefree
        let a = AlgPoly::one(&nf, "x");
        let d = AlgPoly::from_coeffs(
            vec![nf.from_rational(&int(1)), nf.zero(), nf.one()],
            &nf,
            "x",
        );

        let (b, _d1, c, d2) = hermite_reduce_algebraic(&a, &d).unwrap();

        // b should be 0 (no rational part)
        assert!(b.is_zero());
        // c/d₂ should be 1/(x²+1)
        assert_eq!(c.degree(), Some(0));
        assert_eq!(d2.degree(), Some(2));
    }
}
