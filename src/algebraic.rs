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
}
