use std::fmt;

/// Dense polynomial over Z_p (integers mod a prime p).
///
/// Coefficients stored least-degree first: `coeffs[i]` is the coefficient of x^i.
/// Each coefficient is in [0, p-1]. Empty vec = zero polynomial.
///
/// Reference: TAOCP Volume 2, Section 4.6.2.
#[derive(Debug, Clone)]
pub struct ModPoly {
    coeffs: Vec<i64>,
    p: i64,
}

/// Reduce val into [0, p-1].
fn mod_reduce(val: i64, p: i64) -> i64 {
    ((val % p) + p) % p
}

/// Modular inverse of a mod p via the extended Euclidean algorithm.
/// Requires gcd(a, p) = 1 (i.e., p is prime and a ≢ 0 mod p).
fn mod_inverse(a: i64, p: i64) -> i64 {
    let a = mod_reduce(a, p);
    if a == 0 {
        panic!("mod_inverse(0, {}) is undefined", p);
    }
    let mut old_r = a;
    let mut r = p;
    let mut old_s: i64 = 1;
    let mut s: i64 = 0;
    while r != 0 {
        let q = old_r / r;
        let tmp_r = r;
        r = old_r - q * r;
        old_r = tmp_r;
        let tmp_s = s;
        s = old_s - q * s;
        old_s = tmp_s;
    }
    mod_reduce(old_s, p)
}

impl ModPoly {
    pub fn zero(p: i64) -> Self {
        ModPoly { coeffs: vec![], p }
    }

    pub fn one(p: i64) -> Self {
        ModPoly { coeffs: vec![1], p }
    }

    pub fn x_poly(p: i64) -> Self {
        ModPoly {
            coeffs: vec![0, 1],
            p,
        }
    }

    pub fn from_coeffs(coeffs: &[i64], p: i64) -> Self {
        let mut reduced: Vec<i64> = coeffs.iter().map(|&c| mod_reduce(c, p)).collect();
        while reduced.last() == Some(&0) {
            reduced.pop();
        }
        ModPoly { coeffs: reduced, p }
    }

    pub fn constant(c: i64, p: i64) -> Self {
        let c = mod_reduce(c, p);
        if c == 0 {
            Self::zero(p)
        } else {
            ModPoly { coeffs: vec![c], p }
        }
    }

    pub fn monomial(coeff: i64, degree: usize, p: i64) -> Self {
        let c = mod_reduce(coeff, p);
        if c == 0 {
            return Self::zero(p);
        }
        let mut coeffs = vec![0i64; degree + 1];
        coeffs[degree] = c;
        ModPoly { coeffs, p }
    }

    pub fn degree(&self) -> Option<usize> {
        if self.coeffs.is_empty() {
            None
        } else {
            Some(self.coeffs.len() - 1)
        }
    }

    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.coeffs.len() == 1 && self.coeffs[0] == 1
    }

    pub fn leading_coeff(&self) -> Option<i64> {
        self.coeffs.last().copied()
    }

    pub fn coeff(&self, i: usize) -> i64 {
        self.coeffs.get(i).copied().unwrap_or(0)
    }

    pub fn modulus(&self) -> i64 {
        self.p
    }

    pub fn coeffs(&self) -> &[i64] {
        &self.coeffs
    }

    // --- Arithmetic ---

    pub fn add(&self, other: &ModPoly) -> ModPoly {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = other.coeff(i);
            coeffs.push(mod_reduce(a + b, self.p));
        }
        while coeffs.last() == Some(&0) {
            coeffs.pop();
        }
        ModPoly { coeffs, p: self.p }
    }

    pub fn sub(&self, other: &ModPoly) -> ModPoly {
        let len = self.coeffs.len().max(other.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = other.coeff(i);
            coeffs.push(mod_reduce(a - b, self.p));
        }
        while coeffs.last() == Some(&0) {
            coeffs.pop();
        }
        ModPoly { coeffs, p: self.p }
    }

    pub fn neg(&self) -> ModPoly {
        let coeffs = self
            .coeffs
            .iter()
            .map(|&c| mod_reduce(-c, self.p))
            .collect();
        ModPoly { coeffs, p: self.p }
    }

    pub fn scalar_mul(&self, s: i64) -> ModPoly {
        let s = mod_reduce(s, self.p);
        if s == 0 {
            return Self::zero(self.p);
        }
        let mut coeffs: Vec<i64> = self
            .coeffs
            .iter()
            .map(|&c| mod_reduce(c * s, self.p))
            .collect();
        while coeffs.last() == Some(&0) {
            coeffs.pop();
        }
        ModPoly { coeffs, p: self.p }
    }

    pub fn mul(&self, other: &ModPoly) -> ModPoly {
        if self.is_zero() || other.is_zero() {
            return Self::zero(self.p);
        }
        let len = self.coeffs.len() + other.coeffs.len() - 1;
        let mut coeffs = vec![0i64; len];
        for (i, &a) in self.coeffs.iter().enumerate() {
            if a == 0 {
                continue;
            }
            for (j, &b) in other.coeffs.iter().enumerate() {
                coeffs[i + j] = mod_reduce(coeffs[i + j] + a * b, self.p);
            }
        }
        while coeffs.last() == Some(&0) {
            coeffs.pop();
        }
        ModPoly { coeffs, p: self.p }
    }

    pub fn make_monic(&self) -> ModPoly {
        if self.is_zero() {
            return self.clone();
        }
        let lc = self.leading_coeff().unwrap();
        let inv = mod_inverse(lc, self.p);
        self.scalar_mul(inv)
    }

    // --- Division and GCD ---

    /// Polynomial long division over Z_p. Returns (quotient, remainder).
    pub fn div_rem(&self, divisor: &ModPoly) -> Result<(ModPoly, ModPoly), String> {
        if divisor.is_zero() {
            return Err("Division by zero polynomial".to_string());
        }

        let divisor_deg = divisor.degree().unwrap();
        let divisor_lc_inv = mod_inverse(divisor.leading_coeff().unwrap(), self.p);

        let mut remainder = self.clone();
        let self_deg = match self.degree() {
            Some(d) if d >= divisor_deg => d,
            _ => return Ok((Self::zero(self.p), self.clone())),
        };

        let mut q_coeffs = vec![0i64; self_deg - divisor_deg + 1];

        while let Some(rem_deg) = remainder.degree() {
            if rem_deg < divisor_deg {
                break;
            }
            let rem_lc = remainder.leading_coeff().unwrap();
            let q_coeff = mod_reduce(rem_lc * divisor_lc_inv, self.p);
            let deg_diff = rem_deg - divisor_deg;
            q_coeffs[deg_diff] = q_coeff;

            let term = ModPoly::monomial(q_coeff, deg_diff, self.p);
            let sub = term.mul(divisor);
            remainder = remainder.sub(&sub);
        }

        Ok((ModPoly::from_coeffs(&q_coeffs, self.p), remainder))
    }

    /// GCD via the Euclidean algorithm. Result is monic.
    pub fn gcd(&self, other: &ModPoly) -> ModPoly {
        if self.is_zero() {
            return if other.is_zero() {
                Self::zero(self.p)
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
            let (_, r) = a.div_rem(&b).unwrap();
            a = b;
            b = r;
        }

        a.make_monic()
    }

    /// Extended GCD: returns (gcd, s, t) such that s*a + t*b = gcd.
    /// The gcd is made monic; s, t are adjusted accordingly.
    pub fn extended_gcd(a: &ModPoly, b: &ModPoly) -> (ModPoly, ModPoly, ModPoly) {
        let p = a.p;
        if b.is_zero() {
            if a.is_zero() {
                return (Self::zero(p), Self::one(p), Self::zero(p));
            }
            let lc_inv = mod_inverse(a.leading_coeff().unwrap(), p);
            return (
                a.scalar_mul(lc_inv),
                ModPoly::constant(lc_inv, p),
                Self::zero(p),
            );
        }

        let mut old_r = a.clone();
        let mut r = b.clone();
        let mut old_s = Self::one(p);
        let mut s = Self::zero(p);
        let mut old_t = Self::zero(p);
        let mut t = Self::one(p);

        while !r.is_zero() {
            let (q, rem) = old_r.div_rem(&r).unwrap();
            old_r = r;
            r = rem;
            let new_s = old_s.sub(&q.mul(&s));
            old_s = s;
            s = new_s;
            let new_t = old_t.sub(&q.mul(&t));
            old_t = t;
            t = new_t;
        }

        // Make gcd monic
        let lc_inv = mod_inverse(old_r.leading_coeff().unwrap(), p);
        (
            old_r.scalar_mul(lc_inv),
            old_s.scalar_mul(lc_inv),
            old_t.scalar_mul(lc_inv),
        )
    }

    /// Compute base^exp mod modulus, all in Z_p[x]. Repeated squaring.
    pub fn powmod(base: &ModPoly, exp: u64, modulus: &ModPoly) -> ModPoly {
        let p = base.p;
        if exp == 0 {
            return Self::one(p);
        }

        let mut result = Self::one(p);
        let mut b = base.div_rem(modulus).unwrap().1; // reduce base mod modulus
        let mut e = exp;

        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&b);
                result = result.div_rem(modulus).unwrap().1;
            }
            b = b.mul(&b);
            b = b.div_rem(modulus).unwrap().1;
            e >>= 1;
        }

        result
    }

    /// Formal derivative over Z_p.
    pub fn derivative(&self) -> ModPoly {
        if self.coeffs.len() <= 1 {
            return Self::zero(self.p);
        }
        let coeffs: Vec<i64> = self
            .coeffs
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, &c)| mod_reduce(c * i as i64, self.p))
            .collect();
        ModPoly::from_coeffs(&coeffs, self.p)
    }

    /// Square-free part: f / gcd(f, f').
    pub fn square_free_part(&self) -> ModPoly {
        if self.degree().unwrap_or(0) <= 1 {
            return self.make_monic();
        }
        let d = self.derivative();
        if d.is_zero() {
            return self.make_monic();
        }
        let g = self.gcd(&d);
        if g.degree().unwrap_or(0) == 0 {
            return self.make_monic();
        }
        let (q, _) = self.div_rem(&g).unwrap();
        q.make_monic()
    }

    // --- Conversion ---

    /// Reduce a Polynomial (BigRational coefficients) mod p.
    /// Takes primitive part first to get integer coefficients.
    pub fn from_polynomial(poly: &crate::polynomial::Polynomial, p: i64) -> Self {
        use num_traits::ToPrimitive;
        let prim = poly.primitive_part();
        let deg = match prim.degree() {
            Some(d) => d,
            None => return Self::zero(p),
        };
        let mut coeffs = Vec::with_capacity(deg + 1);
        for i in 0..=deg {
            let c = prim.coeff(i);
            let val = c.numer().to_i64().unwrap_or(0);
            coeffs.push(mod_reduce(val, p));
        }
        ModPoly::from_coeffs(&coeffs, p)
    }

    /// Lift to a Polynomial with BigRational coefficients.
    pub fn to_polynomial(&self, var: &str) -> crate::polynomial::Polynomial {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        let coeffs: Vec<BigRational> = self
            .coeffs
            .iter()
            .map(|&c| BigRational::from_integer(BigInt::from(c)))
            .collect();
        crate::polynomial::Polynomial::from_coeffs(coeffs, var)
    }
}

// --- Berlekamp's algorithm ---

#[allow(clippy::needless_range_loop)]
/// Build the Berlekamp Q-matrix for polynomial f over Z_p.
///
/// Q is n×n where n = deg(f).
/// Row i contains the coefficients of x^(i*p) mod f(x).
fn berlekamp_matrix(f: &ModPoly) -> Vec<Vec<i64>> {
    let n = f.degree().unwrap();
    let p = f.p;

    // Compute x^p mod f
    let x = ModPoly::x_poly(p);
    let x_to_p = ModPoly::powmod(&x, p as u64, f);

    let mut matrix = vec![vec![0i64; n]; n];

    // Row 0: x^0 mod f = 1
    matrix[0][0] = 1;

    if n > 1 {
        // Row 1: x^p mod f
        for j in 0..n {
            matrix[1][j] = x_to_p.coeff(j);
        }

        // Row i: x^(ip) mod f = (row_{i-1} · x^p) mod f
        let mut prev = x_to_p.clone();
        for i in 2..n {
            prev = prev.mul(&x_to_p);
            prev = prev.div_rem(f).unwrap().1;
            for j in 0..n {
                matrix[i][j] = prev.coeff(j);
            }
        }
    }

    matrix
}

#[allow(clippy::needless_range_loop)]
/// Compute the null space of (Q - I) over Z_p via Gaussian elimination.
/// Returns basis vectors as coefficient vectors (least-degree first).
fn null_space(q_matrix: &[Vec<i64>], p: i64) -> Vec<Vec<i64>> {
    let n = q_matrix.len();
    if n == 0 {
        return vec![];
    }

    // Build Q - I
    let mut mat: Vec<Vec<i64>> = q_matrix.to_vec();
    for i in 0..n {
        mat[i][i] = mod_reduce(mat[i][i] - 1, p);
    }

    // We want vectors v with v·(Q−I) = 0, i.e. the null space of (Q−I)ᵀ.
    let mut a = vec![vec![0i64; n]; n];
    for i in 0..n {
        for j in 0..n {
            a[i][j] = mat[j][i];
        }
    }

    // Reduced row echelon form with ROW operations only. The previous
    // implementation swapped COLUMNS to position pivots and then read the
    // basis vectors off in the permuted coordinate order — whenever a swap
    // occurred, the returned vectors were scrambled, Berlekamp's gcd
    // splits all failed, and reducible polynomials (x⁴+4, x⁴+64) were
    // declared irreducible over Q: false theorems at the exact tier.
    let mut pivot_col_of_row: Vec<Option<usize>> = vec![None; n];
    let mut pivot_row_of_col: Vec<Option<usize>> = vec![None; n];
    let mut row = 0;
    for col in 0..n {
        // Find a pivot for this column at or below `row`.
        let pivot = (row..n).find(|&r| a[r][col] != 0);
        let pr = match pivot {
            Some(r) => r,
            None => continue, // free column
        };
        a.swap(row, pr);

        let inv = mod_inverse(a[row][col], p);
        for c in 0..n {
            a[row][c] = mod_reduce(a[row][c] * inv, p);
        }
        for r in 0..n {
            if r != row && a[r][col] != 0 {
                let factor = a[r][col];
                for c in 0..n {
                    a[r][c] = mod_reduce(a[r][c] - factor * a[row][c], p);
                }
            }
        }
        pivot_col_of_row[row] = Some(col);
        pivot_row_of_col[col] = Some(row);
        row += 1;
        if row == n {
            break;
        }
    }

    // Each free column contributes one basis vector: set that coordinate
    // to 1 and read the pivot coordinates from the RREF.
    let mut basis = Vec::new();
    for free_col in 0..n {
        if pivot_row_of_col[free_col].is_some() {
            continue;
        }
        let mut v = vec![0i64; n];
        v[free_col] = 1;
        for r in 0..n {
            if let Some(pc) = pivot_col_of_row[r] {
                v[pc] = mod_reduce(-a[r][free_col], p);
            }
        }
        basis.push(v);
    }

    basis
}

/// Factor a monic square-free polynomial over Z_p using Berlekamp's algorithm.
///
/// Returns a sorted list of monic irreducible factors.
pub fn factor_mod_p(f: &ModPoly) -> Vec<ModPoly> {
    let n = match f.degree() {
        Some(d) if d >= 1 => d,
        _ => return vec![f.clone()],
    };
    let p = f.p;

    let f_monic = f.make_monic();
    let f_sqfree = f_monic.square_free_part();

    if f_sqfree.degree().unwrap_or(0) <= 1 {
        return vec![f_sqfree];
    }

    let q = berlekamp_matrix(&f_sqfree);
    let basis = null_space(&q, p);

    // Null space dimension = number of irreducible factors.
    // If dim = 1, f is irreducible.
    if basis.len() <= 1 {
        return vec![f_sqfree];
    }

    // Split using basis vectors
    let mut factors = vec![f_sqfree];

    for bv in &basis {
        // Skip trivial constant vectors
        if bv.iter().skip(1).all(|&c| c == 0) {
            continue;
        }

        let v = ModPoly::from_coeffs(bv, p);
        let mut new_factors = Vec::new();

        for factor in &factors {
            if factor.degree().unwrap_or(0) <= 1 {
                new_factors.push(factor.clone());
                continue;
            }

            let mut splits: Vec<ModPoly> = Vec::new();
            let mut remaining = factor.clone();

            for c in 0..p {
                if remaining.degree().unwrap_or(0) <= 1 {
                    break;
                }
                let v_minus_c = v.sub(&ModPoly::constant(c, p));
                let g = remaining.gcd(&v_minus_c);
                if g.degree().unwrap_or(0) >= 1 && g.degree() < remaining.degree() {
                    let (q, _) = remaining.div_rem(&g).unwrap();
                    splits.push(g);
                    remaining = q;
                }
            }

            if splits.is_empty() {
                new_factors.push(factor.clone());
            } else {
                if remaining.degree().unwrap_or(0) >= 1 {
                    splits.push(remaining);
                }
                new_factors.extend(splits);
            }
        }

        factors = new_factors;

        // Check if we've found all factors
        if factors.len() >= n {
            break;
        }
    }

    // Ensure all factors are monic
    factors = factors.into_iter().map(|f| f.make_monic()).collect();

    // Sort by degree then by coefficients for deterministic output
    factors.sort_by(|a, b| {
        a.degree()
            .cmp(&b.degree())
            .then_with(|| a.coeffs.cmp(&b.coeffs))
    });

    factors
}

// --- Factor Recombination (Layer 4) ---
//
// Given lifted factors mod p^k, find true factors over Z by testing
// subsets. A subset S of lifted factors produces a candidate g;
// if g divides f over Z (not just mod p^k), it's a true factor.
// Coefficients are centered: mapped from [0, p^k) to (-p^k/2, p^k/2].

/// Choose a suitable prime for factoring: p must not divide the leading
/// coefficient, and f mod p must be square-free.
fn choose_prime(f: &crate::polynomial::Polynomial) -> i64 {
    let small_primes: &[i64] = &[3, 5, 7, 11, 13, 17, 19, 23, 29, 31];
    let lc = f.leading_coeff().unwrap().to_integer();

    for &p in small_primes {
        let big_p = BigInt::from(p);
        if (&lc % &big_p).is_zero() {
            continue;
        }
        let fp = ModPoly::from_polynomial(f, p);
        let fp_deriv = fp.derivative();
        let g = fp.gcd(&fp_deriv);
        if g.degree().unwrap_or(0) == 0 {
            return p;
        }
    }
    let mut p = 37i64;
    loop {
        let big_p = BigInt::from(p);
        if !(&lc % &big_p).is_zero() {
            let fp = ModPoly::from_polynomial(f, p);
            let fp_deriv = fp.derivative();
            let g = fp.gcd(&fp_deriv);
            if g.degree().unwrap_or(0) == 0 {
                return p;
            }
        }
        p += 2;
        if p > 1000 {
            panic!("Could not find a suitable prime for factoring");
        }
    }
}

/// Center-lift a polynomial from [0, m) to (-m/2, m/2].
fn center_lift(poly: &crate::polynomial::Polynomial, m: &BigInt) -> crate::polynomial::Polynomial {
    let half = m / BigInt::from(2);
    let deg = match poly.degree() {
        Some(d) => d,
        None => return crate::polynomial::Polynomial::zero(poly.variable()),
    };
    let mut coeffs = Vec::with_capacity(deg + 1);
    for i in 0..=deg {
        let c = poly.coeff(i).to_integer();
        let centered = if c > half { &c - m } else { c };
        coeffs.push(BigRational::from_integer(centered));
    }
    crate::polynomial::Polynomial::from_coeffs(coeffs, poly.variable())
}

/// Factor a polynomial over Q into irreducible factors.
///
/// Returns (content, factors) where content is the rational content
/// and factors are monic irreducible polynomials over Q such that
/// f = content * product(factors).
pub fn factor_over_q(
    f: &crate::polynomial::Polynomial,
) -> (BigRational, Vec<crate::polynomial::Polynomial>) {
    let content = f.content();
    if content.is_zero() {
        return (BigRational::zero(), vec![]);
    }
    let prim = f.primitive_part();

    let n = match prim.degree() {
        Some(d) => d,
        None => return (content, vec![]),
    };

    if n == 0 {
        return (content, vec![]);
    }

    if n == 1 {
        return (content, vec![prim.clone()]);
    }

    // SFD produces monic factors (divides by leading coefficient).
    // Track lc so we can put it back into the content.
    let lc = prim.leading_coeff().unwrap().clone();
    let adjusted_content = &content * &lc;

    let sfd = prim.square_free_decomposition();
    let mut all_factors = Vec::new();

    for (sq_free, multiplicity) in &sfd {
        let sf_factors = factor_square_free(sq_free);
        for _ in 0..*multiplicity {
            all_factors.extend(sf_factors.iter().cloned());
        }
    }

    // Convert monic rational-coefficient factors to primitive integer-coefficient
    // factors. E.g. (x + 1/2) → content 1/2, primitive (2x + 1).
    // Absorb the content multipliers into adjusted_content so the product is preserved.
    let mut final_content = adjusted_content;
    let mut int_factors = Vec::with_capacity(all_factors.len());
    for f in &all_factors {
        let c = f.content();
        if !c.is_one() {
            final_content = &final_content * &c;
            int_factors.push(f.primitive_part());
        } else {
            int_factors.push(f.clone());
        }
    }

    (final_content, int_factors)
}

/// Factor a square-free polynomial over Q.
/// Input may be monic with rational coefficients (from SFD) or primitive with integers.
/// Internally converts to primitive integer form for the Berlekamp-Zassenhaus pipeline.
fn factor_square_free(f: &crate::polynomial::Polynomial) -> Vec<crate::polynomial::Polynomial> {
    let n = match f.degree() {
        Some(d) => d,
        None => return vec![],
    };
    if n <= 1 {
        return vec![f.make_monic()];
    }

    // Ensure integer coefficients: primitive part clears denominators
    let f_int = f.primitive_part();

    let p = choose_prime(&f_int);
    let fp = ModPoly::from_polynomial(&f_int, p);
    let mod_factors = factor_mod_p(&fp);

    if mod_factors.len() == 1 {
        return vec![f_int.make_monic()];
    }

    // factor_mod_p returns monic factors (factors of f/lc mod p).
    // Hensel lifting requires product of factors ≡ f mod p.
    // Adjust the first factor by the leading coefficient to match.
    let lc_int = f_int
        .leading_coeff()
        .unwrap()
        .to_integer()
        .to_i64()
        .unwrap_or(1);
    let lc_mod = mod_reduce(lc_int, p);
    let adjusted_factors: Vec<ModPoly> = if lc_mod != 1 {
        let mut af = mod_factors.clone();
        af[0] = af[0].scalar_mul(lc_mod);
        af
    } else {
        mod_factors.clone()
    };

    let k = lifting_target(&f_int, p);
    let lifted = hensel_lift_factors(&f_int, &adjusted_factors, p, k);

    let mut pk = BigInt::from(p);
    for _ in 1..k {
        pk *= BigInt::from(p);
    }

    let r = lifted.len();
    let mut remaining = f_int.make_monic();
    let mut used = vec![false; r];
    let mut true_factors: Vec<crate::polynomial::Polynomial> = Vec::new();

    let mut s = 1;
    while 2 * s <= r - used.iter().filter(|&&u| u).count() {
        let available: Vec<usize> = (0..r).filter(|&i| !used[i]).collect();
        let mut found = false;

        for subset in combinations(&available, s) {
            let mut candidate_mod = crate::polynomial::Polynomial::one(f_int.variable());
            let mut deg_sum = 0;
            for &idx in &subset {
                candidate_mod = &candidate_mod * &lifted[idx];
                deg_sum += mod_factors[idx].degree().unwrap_or(0);
            }

            if deg_sum > remaining.degree().unwrap_or(0) {
                continue;
            }

            let candidate_mod = poly_mod(&candidate_mod, &pk);
            let candidate = center_lift(&candidate_mod, &pk);
            let candidate_monic = candidate.primitive_part().make_monic();

            if let Ok((q, rem)) = remaining.div_rem(&candidate_monic) {
                if rem.is_zero() {
                    true_factors.push(candidate_monic);
                    remaining = q;
                    for &idx in &subset {
                        used[idx] = true;
                    }
                    found = true;
                    break;
                }
            }
        }

        if !found {
            s += 1;
        }
    }

    if remaining.degree().unwrap_or(0) >= 1 {
        true_factors.push(remaining.make_monic());
    }

    true_factors.sort_by(|a, b| {
        a.degree()
            .cmp(&b.degree())
            .then_with(|| format!("{}", a).cmp(&format!("{}", b)))
    });

    true_factors
}

/// Generate all combinations of `k` elements from `items`.
fn combinations(items: &[usize], k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = Vec::with_capacity(k);
    combinations_inner(items, k, 0, &mut current, &mut result);
    result
}

fn combinations_inner(
    items: &[usize],
    k: usize,
    start: usize,
    current: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        result.push(current.clone());
        return;
    }
    for i in start..items.len() {
        if items.len() - i < k - current.len() {
            break;
        }
        current.push(items[i]);
        combinations_inner(items, k, i + 1, current, result);
        current.pop();
    }
}

// --- Hensel Lifting ---
//
// Given f ≡ g·h (mod p) with gcd(g,h) = 1 mod p,
// lift to f ≡ g*·h* (mod p^k) for any target k.
//
// Uses linear Hensel lifting: each step raises the modulus from p^j to p^(j+1).
// All "big" arithmetic uses Polynomial (BigRational with integer values);
// the correction step uses ModPoly (mod p) via the Bezout coefficients.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Reduce a Polynomial's coefficients into [0, m) where m is a BigInt modulus.
fn poly_mod(poly: &crate::polynomial::Polynomial, m: &BigInt) -> crate::polynomial::Polynomial {
    let deg = match poly.degree() {
        Some(d) => d,
        None => return crate::polynomial::Polynomial::zero("x"),
    };
    let var = poly.variable().to_string();
    let mut coeffs = Vec::with_capacity(deg + 1);
    for i in 0..=deg {
        let c = poly.coeff(i);
        let n = c.to_integer();
        let r = ((n % m) + m) % m;
        coeffs.push(BigRational::from_integer(r));
    }
    crate::polynomial::Polynomial::from_coeffs(coeffs, &var)
}

/// Convert a ModPoly to a Polynomial (BigRational coefficients).
fn modpoly_to_poly(mp: &ModPoly, var: &str) -> crate::polynomial::Polynomial {
    let coeffs: Vec<BigRational> = mp
        .coeffs
        .iter()
        .map(|&c| BigRational::from_integer(BigInt::from(c)))
        .collect();
    crate::polynomial::Polynomial::from_coeffs(coeffs, var)
}

/// Convert a Polynomial to a ModPoly by reducing coefficients mod p.
fn poly_to_modpoly(poly: &crate::polynomial::Polynomial, p: i64) -> ModPoly {
    use num_traits::ToPrimitive;
    let deg = match poly.degree() {
        Some(d) => d,
        None => return ModPoly::zero(p),
    };
    let mut coeffs = Vec::with_capacity(deg + 1);
    for i in 0..=deg {
        let c = poly.coeff(i);
        let val = c.to_integer().to_i64().unwrap_or_else(|| {
            let big_p = BigInt::from(p);
            let r = ((c.to_integer() % &big_p) + &big_p) % &big_p;
            r.to_i64().unwrap_or(0)
        });
        coeffs.push(mod_reduce(val, p));
    }
    ModPoly::from_coeffs(&coeffs, p)
}

/// Single Hensel lifting step: given f ≡ g·h (mod p^k), lift to mod p^(k+1).
///
/// Parameters:
/// - f: the original polynomial over Z (Polynomial with integer coefficients)
/// - g, h: current lifted factors (Polynomial with integer coefficients, mod p^k)
/// - s, t: Bezout coefficients mod p (s·g₀ + t·h₀ ≡ 1 mod p, where g₀,h₀ are original mod-p factors)
/// - p: the prime
/// - pk: p^k (current modulus, as BigInt)
///
/// Returns (g', h') such that f ≡ g'·h' (mod p^(k+1)).
fn hensel_step(
    f: &crate::polynomial::Polynomial,
    g: &crate::polynomial::Polynomial,
    h: &crate::polynomial::Polynomial,
    s: &ModPoly,
    t: &ModPoly,
    p: i64,
    pk: &BigInt,
) -> (crate::polynomial::Polynomial, crate::polynomial::Polynomial) {
    let var = f.variable().to_string();
    let pk1 = pk * BigInt::from(p); // p^(k+1)

    // Step 1: e = f - g*h (over Z[x])
    let gh = g * h;
    let e = f - &gh;

    // Step 2: e_bar = (e / p^k) mod p — each coefficient of e is divisible by p^k
    let e_deg = match e.degree() {
        Some(d) => d,
        None => return (poly_mod(g, &pk1), poly_mod(h, &pk1)),
    };
    let big_p = BigInt::from(p);
    let mut e_bar_coeffs = Vec::with_capacity(e_deg + 1);
    for i in 0..=e_deg {
        let c = e.coeff(i).to_integer();
        let divided = &c / pk;
        let r = ((&divided % &big_p) + &big_p) % &big_p;
        e_bar_coeffs.push(r.to_i64().unwrap_or(0));
    }
    let e_bar = ModPoly::from_coeffs(&e_bar_coeffs, p);

    // Step 3: solve e_bar ≡ σ·h₀ + τ·g₀ (mod p)
    // (q, σ) = divmod(s · e_bar, h₀), then τ = t · e_bar + q · g₀
    let h0 = poly_to_modpoly(h, p);
    let se = s.mul(&e_bar);
    let (q_bar, sigma) = se.div_rem(&h0).unwrap();

    let g0 = poly_to_modpoly(g, p);
    let tau = t.mul(&e_bar).add(&q_bar.mul(&g0));

    // Step 4: g' = g + tau * p^k (mod p^(k+1))
    let tau_poly = modpoly_to_poly(&tau, &var);
    let tau_scaled = tau_poly.scalar_mul(&BigRational::from_integer(pk.clone()));
    let g_new = poly_mod(&(g + &tau_scaled), &pk1);

    // Step 5: h' = h + sigma * p^k (mod p^(k+1))
    let sigma_poly = modpoly_to_poly(&sigma, &var);
    let sigma_scaled = sigma_poly.scalar_mul(&BigRational::from_integer(pk.clone()));
    let h_new = poly_mod(&(h + &sigma_scaled), &pk1);

    (g_new, h_new)
}

/// Hensel lift a two-factor decomposition from mod p to mod p^target_k.
///
/// Given f ≡ g₀·h₀ (mod p) with gcd(g₀, h₀) = 1, returns (g, h)
/// such that f ≡ g·h (mod p^target_k), with g monic and deg(g) = deg(g₀).
pub fn hensel_lift_pair(
    f: &crate::polynomial::Polynomial,
    g0: &ModPoly,
    h0: &ModPoly,
    p: i64,
    target_k: u32,
) -> (crate::polynomial::Polynomial, crate::polynomial::Polynomial) {
    let var = f.variable().to_string();

    // Compute Bezout coefficients: s·g₀ + t·h₀ ≡ 1 (mod p)
    let (_, s, t) = ModPoly::extended_gcd(g0, h0);

    // Start with mod-p factors lifted to Polynomial
    let mut g = modpoly_to_poly(g0, &var);
    let mut h = modpoly_to_poly(h0, &var);
    let mut pk = BigInt::from(p);

    for _ in 1..target_k {
        let (g_new, h_new) = hensel_step(f, &g, &h, &s, &t, p, &pk);
        g = g_new;
        h = h_new;
        pk *= BigInt::from(p);
    }

    (g, h)
}

/// Hensel lift all factors from mod p to mod p^target_k.
///
#[allow(clippy::needless_range_loop)]
/// Uses sequential pair-lifting: peels off one factor at a time.
/// For r factors, performs r-1 pair lifts.
pub fn hensel_lift_factors(
    f: &crate::polynomial::Polynomial,
    factors: &[ModPoly],
    p: i64,
    target_k: u32,
) -> Vec<crate::polynomial::Polynomial> {
    if factors.len() <= 1 {
        return factors
            .iter()
            .map(|mp| modpoly_to_poly(mp, f.variable()))
            .collect();
    }

    let mut result = Vec::with_capacity(factors.len());

    // Remaining polynomial to factor (starts as f)
    let mut remaining = f.clone();

    for i in 0..factors.len() - 1 {
        // g₀ = factors[i], h₀ = product of remaining factors mod p
        let g0 = &factors[i];
        let mut h0 = ModPoly::one(p);
        for j in (i + 1)..factors.len() {
            h0 = h0.mul(&factors[j]);
        }

        // Lift the pair
        let (g_lifted, h_lifted) = hensel_lift_pair(&remaining, g0, &h0, p, target_k);
        result.push(g_lifted);
        remaining = h_lifted;
    }

    // The last factor is whatever remains
    result.push(remaining);
    result
}

/// Compute the Mignotte bound: the maximum absolute value of any coefficient
/// of any factor of f.
///
/// For f of degree n with ||f||₂ = B, any factor g of degree d has
/// |g_i| ≤ C(d, d/2) · B where C is the binomial coefficient.
/// We use the simpler bound: |g_i| < 2^n · ||f||_∞.
pub fn mignotte_bound(f: &crate::polynomial::Polynomial) -> BigInt {
    let n = match f.degree() {
        Some(d) => d,
        None => return BigInt::one(),
    };

    // ||f||_∞ = max absolute coefficient
    let mut max_coeff = BigInt::zero();
    for i in 0..=n {
        let c = f.coeff(i).to_integer().abs();
        if c > max_coeff {
            max_coeff = c;
        }
    }

    // Bound: 2^n * ||f||_∞
    let two_n = BigInt::from(1i64) << n;
    two_n * max_coeff
}

/// Determine the lifting target k such that p^k > 2 * mignotte_bound(f).
/// The factor of 2 accounts for centered representation (coefficients may be negative).
pub fn lifting_target(f: &crate::polynomial::Polynomial, p: i64) -> u32 {
    let bound = mignotte_bound(f) * BigInt::from(2i64);
    let mut pk = BigInt::from(p);
    let mut k = 1u32;
    while pk <= bound {
        pk *= BigInt::from(p);
        k += 1;
    }
    k
}

impl PartialEq for ModPoly {
    fn eq(&self, other: &Self) -> bool {
        self.p == other.p && self.coeffs == other.coeffs
    }
}

impl Eq for ModPoly {}

impl fmt::Display for ModPoly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }
        let mut first = true;
        for (i, &c) in self.coeffs.iter().enumerate().rev() {
            if c == 0 {
                continue;
            }
            if !first {
                write!(f, " + ")?;
            }
            if i == 0 || c != 1 {
                write!(f, "{}", c)?;
            }
            if i >= 1 {
                write!(f, "x")?;
            }
            if i >= 2 {
                write!(f, "^{}", i)?;
            }
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    // --- ModPoly basics ---

    #[test]
    fn test_constructors() {
        let z = ModPoly::zero(5);
        assert!(z.is_zero());
        assert_eq!(z.degree(), None);

        let one = ModPoly::one(5);
        assert!(one.is_one());
        assert_eq!(one.degree(), Some(0));

        let x = ModPoly::x_poly(5);
        assert_eq!(x.degree(), Some(1));
        assert_eq!(x.coeff(0), 0);
        assert_eq!(x.coeff(1), 1);
    }

    #[test]
    fn test_from_coeffs_reduces() {
        let p = ModPoly::from_coeffs(&[7, -3, 12], 5);
        assert_eq!(p.coeff(0), 2); // 7 mod 5
        assert_eq!(p.coeff(1), 2); // -3 mod 5
        assert_eq!(p.coeff(2), 2); // 12 mod 5
    }

    #[test]
    fn test_from_coeffs_strips_zeros() {
        let p = ModPoly::from_coeffs(&[1, 2, 0, 0], 5);
        assert_eq!(p.degree(), Some(1));
    }

    #[test]
    fn test_mod_inverse() {
        // 3 * 2 = 6 ≡ 1 (mod 5)
        assert_eq!(mod_inverse(3, 5), 2);
        // 2 * 4 = 8 ≡ 1 (mod 7)
        assert_eq!(mod_inverse(2, 7), 4);
        // Self-inverse: 1^(-1) = 1
        assert_eq!(mod_inverse(1, 7), 1);
        // 6^(-1) mod 7: 6 * 6 = 36 ≡ 1 (mod 7)
        assert_eq!(mod_inverse(6, 7), 6);
    }

    // --- Arithmetic ---

    #[test]
    fn test_add_sub() {
        let a = ModPoly::from_coeffs(&[1, 2, 3], 5); // 3x²+2x+1
        let b = ModPoly::from_coeffs(&[4, 4, 3], 5); // 3x²+4x+4
        let sum = a.add(&b);
        assert_eq!(sum, ModPoly::from_coeffs(&[0, 1, 1], 5)); // x²+x (mod 5)
        let diff = a.sub(&b);
        assert_eq!(diff, ModPoly::from_coeffs(&[2, 3], 5)); // 3x+2 (mod 5)
    }

    #[test]
    fn test_mul() {
        // (x+1)(x+2) = x²+3x+2 over Z_5
        let a = ModPoly::from_coeffs(&[1, 1], 5);
        let b = ModPoly::from_coeffs(&[2, 1], 5);
        let prod = a.mul(&b);
        assert_eq!(prod, ModPoly::from_coeffs(&[2, 3, 1], 5));
    }

    #[test]
    fn test_make_monic() {
        let a = ModPoly::from_coeffs(&[2, 4, 3], 5); // 3x²+4x+2
        let m = a.make_monic();
        assert_eq!(m.leading_coeff(), Some(1));
        // 3^(-1) mod 5 = 2, so coeffs * 2: [4, 3, 1]
        assert_eq!(m, ModPoly::from_coeffs(&[4, 3, 1], 5));
    }

    // --- Division and GCD ---

    #[test]
    fn test_div_rem() {
        // (x²+3x+2) / (x+1) = (x+2) remainder 0 over Z_5
        let f = ModPoly::from_coeffs(&[2, 3, 1], 5);
        let d = ModPoly::from_coeffs(&[1, 1], 5);
        let (q, r) = f.div_rem(&d).unwrap();
        assert_eq!(q, ModPoly::from_coeffs(&[2, 1], 5));
        assert!(r.is_zero());
    }

    #[test]
    fn test_div_rem_with_remainder() {
        // (x²+1) / (x+1) over Z_5: x²+1 = (x-1)(x+1) + 2 → q = x+4, r = 2
        let f = ModPoly::from_coeffs(&[1, 0, 1], 5);
        let d = ModPoly::from_coeffs(&[1, 1], 5);
        let (q, r) = f.div_rem(&d).unwrap();
        // Verify: q*d + r = f
        let check = q.mul(&d).add(&r);
        assert_eq!(check, f);
    }

    #[test]
    fn test_gcd() {
        // gcd(x²-1, x²+2x+1) over Z_5 = gcd((x-1)(x+1), (x+1)²) = x+1
        let a = ModPoly::from_coeffs(&[4, 0, 1], 5); // x²-1 = x²+4 mod 5
        let b = ModPoly::from_coeffs(&[1, 2, 1], 5); // x²+2x+1
        let g = a.gcd(&b);
        assert_eq!(g, ModPoly::from_coeffs(&[1, 1], 5)); // x+1
    }

    // --- Powmod ---

    #[test]
    fn test_powmod_simple() {
        // x^2 mod (x^2+1) over Z_5 = -1 = 4
        let x = ModPoly::x_poly(5);
        let m = ModPoly::from_coeffs(&[1, 0, 1], 5);
        let result = ModPoly::powmod(&x, 2, &m);
        assert_eq!(result, ModPoly::from_coeffs(&[4], 5));
    }

    #[test]
    fn test_powmod_large() {
        // x^5 mod (x^3+x+1) over Z_5
        // x^3 ≡ -x-1 = 4x+4 mod (x^3+x+1) mod 5
        // x^4 ≡ 4x²+4x
        // x^5 ≡ 4x³+4x² ≡ 4(4x+4)+4x² = 4x²+16x+16 ≡ 4x²+x+1 mod 5
        let x = ModPoly::x_poly(5);
        let m = ModPoly::from_coeffs(&[1, 1, 0, 1], 5); // x^3+x+1
        let result = ModPoly::powmod(&x, 5, &m);
        assert_eq!(result, ModPoly::from_coeffs(&[1, 1, 4], 5)); // 4x²+x+1
    }

    // --- Derivative ---

    #[test]
    fn test_derivative() {
        // d/dx(x^3 + 2x + 1) = 3x^2 + 2 over Z_5
        let f = ModPoly::from_coeffs(&[1, 2, 0, 1], 5);
        let d = f.derivative();
        assert_eq!(d, ModPoly::from_coeffs(&[2, 0, 3], 5));
    }

    // --- Berlekamp factoring ---

    #[test]
    fn test_factor_linear() {
        // x+1 mod 5 → irreducible
        let f = ModPoly::from_coeffs(&[1, 1], 5);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 1);
        assert_eq!(factors[0], f);
    }

    #[test]
    fn test_factor_x2_minus_1_mod5() {
        // x²-1 mod 5 = (x+1)(x+4) = (x+1)(x-1)
        let f = ModPoly::from_coeffs(&[4, 0, 1], 5);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 2);
        // Verify product
        let product = factors[0].mul(&factors[1]);
        assert_eq!(product, f.make_monic());
    }

    #[test]
    fn test_factor_irreducible_mod2() {
        // x²+x+1 mod 2 is irreducible (no roots: f(0)=1, f(1)=1)
        let f = ModPoly::from_coeffs(&[1, 1, 1], 2);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 1);
    }

    #[test]
    fn test_factor_x4_minus_1_mod5() {
        // x⁴-1 mod 5 = (x-1)(x+1)(x²+1) but x²+1 factors as (x+2)(x+3) mod 5
        // since 2²+1=5≡0, 3²+1=10≡0 mod 5
        let f = ModPoly::from_coeffs(&[4, 0, 0, 0, 1], 5); // x⁴+4
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 4);
        let mut product = ModPoly::one(5);
        for fac in &factors {
            product = product.mul(fac);
        }
        assert_eq!(product, f.make_monic());
    }

    #[test]
    fn test_factor_x3_plus_x_plus_1_mod2() {
        // x³+x+1 mod 2 is irreducible (f(0)=1, f(1)=1)
        let f = ModPoly::from_coeffs(&[1, 1, 0, 1], 2);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 1);
    }

    #[test]
    fn test_factor_x6_minus_1_mod7() {
        // x⁶-1 mod 7 factors into (x-1)(x+1)(x²+x+1)(x²-x+1) mod 7
        // but some of these may factor further over Z_7
        let f = ModPoly::from_coeffs(&[6, 0, 0, 0, 0, 0, 1], 7); // x⁶+6
        let factors = factor_mod_p(&f);
        let mut product = ModPoly::one(7);
        for fac in &factors {
            product = product.mul(fac);
        }
        assert_eq!(product, f.make_monic());
        assert!(factors.len() >= 2);
    }

    #[test]
    fn test_factor_product_of_three() {
        // (x+1)(x+2)(x+3) mod 7 = x³+6x²+11x+6 ≡ x³+6x²+4x+6 mod 7
        let f1 = ModPoly::from_coeffs(&[1, 1], 7);
        let f2 = ModPoly::from_coeffs(&[2, 1], 7);
        let f3 = ModPoly::from_coeffs(&[3, 1], 7);
        let f = f1.mul(&f2).mul(&f3);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 3);
        let mut product = ModPoly::one(7);
        for fac in &factors {
            product = product.mul(fac);
        }
        assert_eq!(product, f.make_monic());
    }

    #[test]
    fn test_factor_higher_degree_irreducible() {
        // x⁴+x+1 mod 2 is irreducible (it has no roots mod 2: f(0)=1, f(1)=1,
        // and is not divisible by the only irreducible quadratic x²+x+1 mod 2)
        let f = ModPoly::from_coeffs(&[1, 1, 0, 0, 1], 2);
        let factors = factor_mod_p(&f);
        assert_eq!(factors.len(), 1);
    }

    // --- Conversion ---

    #[test]
    fn test_from_polynomial() {
        use num_bigint::BigInt;
        use num_rational::BigRational;
        let int = |n: i64| BigRational::from_integer(BigInt::from(n));
        let poly = crate::polynomial::Polynomial::from_coeffs(vec![int(7), int(-3), int(12)], "x");
        let mp = ModPoly::from_polynomial(&poly, 5);
        assert_eq!(mp.coeff(0), 2); // 7 mod 5
        assert_eq!(mp.coeff(1), 2); // -3 mod 5
        assert_eq!(mp.coeff(2), 2); // 12 mod 5
    }

    #[test]
    fn test_display() {
        let p = ModPoly::from_coeffs(&[1, 2, 3], 5);
        assert_eq!(format!("{}", p), "3x^2 + 2x + 1");
        let z = ModPoly::zero(5);
        assert_eq!(format!("{}", z), "0");
        let c = ModPoly::constant(4, 5);
        assert_eq!(format!("{}", c), "4");
    }

    // --- Extended GCD ---

    #[test]
    fn test_extended_gcd() {
        // gcd(x²-1, x+1) mod 5 = x+1
        // s*(x²-1) + t*(x+1) = x+1
        let a = ModPoly::from_coeffs(&[4, 0, 1], 5); // x²-1 = x²+4
        let b = ModPoly::from_coeffs(&[1, 1], 5); // x+1
        let (g, s, t) = ModPoly::extended_gcd(&a, &b);
        assert_eq!(g, ModPoly::from_coeffs(&[1, 1], 5)); // x+1
                                                         // Verify: s*a + t*b = g
        let check = s.mul(&a).add(&t.mul(&b));
        assert_eq!(check, g);
    }

    #[test]
    fn test_extended_gcd_coprime() {
        // gcd(x+1, x+2) mod 5 = 1
        let a = ModPoly::from_coeffs(&[1, 1], 5);
        let b = ModPoly::from_coeffs(&[2, 1], 5);
        let (g, s, t) = ModPoly::extended_gcd(&a, &b);
        assert!(g.is_one());
        let check = s.mul(&a).add(&t.mul(&b));
        assert_eq!(check, g);
    }

    // --- Hensel Lifting ---

    #[test]
    fn test_hensel_lift_x2_minus_1() {
        // f = x²-1, factors mod 3: (x+1)(x+2) since -1 ≡ 2 mod 3
        let f = crate::polynomial::Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let g0 = ModPoly::from_coeffs(&[1, 1], 3); // x+1
        let h0 = ModPoly::from_coeffs(&[2, 1], 3); // x+2 = x-1 mod 3

        // Lift to mod 3^4 = 81
        let (g, h) = hensel_lift_pair(&f, &g0, &h0, 3, 4);

        // Verify: g*h ≡ f (mod 81)
        let gh = &g * &h;
        let diff = &f - &gh;
        let m = BigInt::from(81i64);
        for i in 0..=diff.degree().unwrap_or(0) {
            let c = diff.coeff(i).to_integer();
            assert!(
                (&c % &m).is_zero(),
                "coeff {} of (f - g*h) = {} not divisible by 81",
                i,
                c
            );
        }
    }

    #[test]
    fn test_hensel_lift_cubic() {
        // f = x³ + 2x² - x - 2 = (x-1)(x+1)(x+2)
        // mod 5: factors are (x+4)(x+1)(x+2)
        let f =
            crate::polynomial::Polynomial::from_coeffs(vec![int(-2), int(-1), int(2), int(1)], "x");
        // Two-factor: g₀ = x+4, h₀ = (x+1)(x+2) = x²+3x+2 mod 5
        let g0 = ModPoly::from_coeffs(&[4, 1], 5);
        let h0 = ModPoly::from_coeffs(&[2, 3, 1], 5);

        let (g, h) = hensel_lift_pair(&f, &g0, &h0, 5, 5); // mod 5^5 = 3125

        let gh = &g * &h;
        let diff = &f - &gh;
        let m = BigInt::from(3125i64);
        for i in 0..=diff.degree().unwrap_or(0) {
            let c = diff.coeff(i).to_integer();
            assert!(
                (&c % &m).is_zero(),
                "coeff {} of (f - g*h) = {} not divisible by 3125",
                i,
                c
            );
        }
    }

    #[test]
    fn test_hensel_lift_factors_multi() {
        // f = x³ - 1 = (x-1)(x²+x+1)
        // mod 7: x³-1 ≡ x³+6 mod 7
        // Factors mod 7: (x+6)(x²+x+1) since x=1 is a root
        let f =
            crate::polynomial::Polynomial::from_coeffs(vec![int(-1), int(0), int(0), int(1)], "x");
        let factors_mod7 = vec![
            ModPoly::from_coeffs(&[6, 1], 7),    // x+6 = x-1 mod 7
            ModPoly::from_coeffs(&[1, 1, 1], 7), // x²+x+1
        ];

        let lifted = hensel_lift_factors(&f, &factors_mod7, 7, 4); // mod 7^4 = 2401
        assert_eq!(lifted.len(), 2);

        // Product should equal f mod 7^4
        let product = &lifted[0] * &lifted[1];
        let diff = &f - &product;
        let m = BigInt::from(2401i64);
        for i in 0..=diff.degree().unwrap_or(0) {
            let c = diff.coeff(i).to_integer();
            assert!((&c % &m).is_zero(), "coeff {} not divisible by 2401", i);
        }
    }

    #[test]
    fn test_mignotte_bound() {
        // f = x² - 1: ||f||_∞ = 1, n = 2, bound = 4
        let f = crate::polynomial::Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let b = mignotte_bound(&f);
        assert_eq!(b, BigInt::from(4i64)); // 2^2 * 1

        // f = 6x² + 5x + 1: ||f||_∞ = 6, n = 2, bound = 24
        let f2 = crate::polynomial::Polynomial::from_coeffs(vec![int(1), int(5), int(6)], "x");
        let b2 = mignotte_bound(&f2);
        assert_eq!(b2, BigInt::from(24i64));
    }

    #[test]
    fn test_lifting_target() {
        // f = x² - 1, p = 3
        // bound = 2 * 4 = 8, need 3^k > 8, so k = 2 (3² = 9 > 8)
        let f = crate::polynomial::Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let k = lifting_target(&f, 3);
        assert_eq!(k, 2);
    }

    // --- Factor recombination (full pipeline) ---

    fn poly(coeffs: &[i64], var: &str) -> crate::polynomial::Polynomial {
        crate::polynomial::Polynomial::from_coeffs(coeffs.iter().map(|&c| int(c)).collect(), var)
    }

    fn verify_factorization(f: &crate::polynomial::Polynomial) {
        let (content, factors) = factor_over_q(f);
        // Rebuild: content * product(factors)
        let mut product = crate::polynomial::Polynomial::one(f.variable());
        for fac in &factors {
            product = &product * fac;
        }
        let rebuilt = product.scalar_mul(&content);
        // Check equality: f == rebuilt
        let diff = f - &rebuilt;
        assert!(
            diff.is_zero(),
            "Factorization failed for {}:\n  content = {}\n  factors = {:?}\n  rebuilt = {}",
            f,
            content,
            factors.iter().map(|f| format!("{}", f)).collect::<Vec<_>>(),
            rebuilt
        );
    }

    #[test]
    fn test_factor_x2_minus_1() {
        // x²-1 = (x-1)(x+1)
        let f = poly(&[-1, 0, 1], "x");
        let (content, factors) = factor_over_q(&f);
        assert!(content.is_one());
        assert_eq!(factors.len(), 2);
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_cubic_three_roots() {
        // x³ - 6x² + 11x - 6 = (x-1)(x-2)(x-3)
        let f = poly(&[-6, 11, -6, 1], "x");
        let (content, factors) = factor_over_q(&f);
        assert!(content.is_one());
        assert_eq!(factors.len(), 3);
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_irreducible_quadratic() {
        // x²+1 is irreducible over Q
        let f = poly(&[1, 0, 1], "x");
        let (_, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 1);
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_with_content() {
        // 6x²-6 = 6(x-1)(x+1)
        let f = poly(&[-6, 0, 6], "x");
        let (content, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 2);
        verify_factorization(&f);
        assert_eq!(content, BigRational::from_integer(BigInt::from(6)));
    }

    #[test]
    fn test_factor_quartic() {
        // x⁴-1 = (x-1)(x+1)(x²+1)
        let f = poly(&[-1, 0, 0, 0, 1], "x");
        let (_, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 3);
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_repeated() {
        // x⁴ + 2x³ + x² = x²(x+1)² — not square-free
        // After square-free decomposition: x (mult 2) and (x+1) (mult 2)
        let f = poly(&[0, 0, 1, 2, 1], "x");
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_q_linear() {
        // 3x + 6 = 3(x + 2)
        let f = poly(&[6, 3], "x");
        let (content, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 1);
        verify_factorization(&f);
        assert_eq!(content, BigRational::from_integer(BigInt::from(3)));
    }

    #[test]
    fn test_factor_cyclotomic_6() {
        // x⁶-1 = (x-1)(x+1)(x²+x+1)(x²-x+1)
        let f = poly(&[-1, 0, 0, 0, 0, 0, 1], "x");
        let (_, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 4);
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_large_coeffs() {
        // (2x+3)(3x-5) = 6x² - x - 15
        let f = poly(&[-15, -1, 6], "x");
        verify_factorization(&f);
    }

    #[test]
    fn test_factor_non_monic_quadratic() {
        // 2x²-x-1 = (2x+1)(x-1) — Ada bug report
        let f = poly(&[-1, -1, 2], "x");
        let (content, factors) = factor_over_q(&f);
        assert_eq!(factors.len(), 2);
        assert!(content.is_one(), "Content should be 1, got {}", content);
        verify_factorization(&f);

        // 6x²+x-1 = (2x+1)(3x-1)
        let f2 = poly(&[-1, 1, 6], "x");
        let (content2, factors2) = factor_over_q(&f2);
        assert_eq!(factors2.len(), 2);
        assert!(content2.is_one(), "Content should be 1, got {}", content2);
        verify_factorization(&f2);

        // 6x²+x-2 = (3x+2)(2x-1)
        let f3 = poly(&[-2, 1, 6], "x");
        let (content3, factors3) = factor_over_q(&f3);
        assert_eq!(factors3.len(), 2);
        assert!(content3.is_one(), "Content should be 1, got {}", content3);
        verify_factorization(&f3);
    }
}
