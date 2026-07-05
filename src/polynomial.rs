use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use crate::exact::ExactNum;
use crate::node::Node;

/// Dense univariate polynomial over Q.
///
/// Coefficients stored least-degree first: `coeffs[i]` is the coefficient of x^i.
/// Invariant: the last element (if any) is nonzero. Empty vec = zero polynomial.
#[derive(Debug, Clone)]
pub struct Polynomial {
    coeffs: Vec<BigRational>,
    variable: String,
}

impl Polynomial {
    pub fn zero(var: &str) -> Self {
        Polynomial {
            coeffs: vec![],
            variable: var.to_string(),
        }
    }

    pub fn one(var: &str) -> Self {
        Polynomial {
            coeffs: vec![BigRational::one()],
            variable: var.to_string(),
        }
    }

    pub fn constant(c: BigRational, var: &str) -> Self {
        if c.is_zero() {
            Self::zero(var)
        } else {
            Polynomial {
                coeffs: vec![c],
                variable: var.to_string(),
            }
        }
    }

    pub fn from_coeffs(mut coeffs: Vec<BigRational>, var: &str) -> Self {
        while coeffs.last().is_some_and(|c| c.is_zero()) {
            coeffs.pop();
        }
        Polynomial {
            coeffs,
            variable: var.to_string(),
        }
    }

    pub fn monomial(coeff: BigRational, degree: usize, var: &str) -> Self {
        if coeff.is_zero() {
            return Self::zero(var);
        }
        let mut coeffs = vec![BigRational::zero(); degree + 1];
        coeffs[degree] = coeff;
        Polynomial {
            coeffs,
            variable: var.to_string(),
        }
    }

    /// The identity polynomial: just x
    pub fn x(var: &str) -> Self {
        Self::monomial(BigRational::one(), 1, var)
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

    pub fn is_constant(&self) -> bool {
        self.coeffs.len() <= 1
    }

    pub fn leading_coeff(&self) -> Option<&BigRational> {
        self.coeffs.last()
    }

    pub fn coeff(&self, i: usize) -> BigRational {
        self.coeffs
            .get(i)
            .cloned()
            .unwrap_or_else(BigRational::zero)
    }

    pub fn variable(&self) -> &str {
        &self.variable
    }

    /// Evaluate at a point using Horner's method.
    /// O(n) multiplications, where n = degree.
    pub fn evaluate(&self, x: &BigRational) -> BigRational {
        let mut result = BigRational::zero();
        for coeff in self.coeffs.iter().rev() {
            result = result * x + coeff;
        }
        result
    }

    /// Make monic (leading coefficient = 1) by dividing by the leading coefficient.
    pub fn make_monic(&self) -> Self {
        match self.leading_coeff() {
            None => self.clone(),
            Some(lc) => {
                let inv = BigRational::one() / lc;
                self.scalar_mul(&inv)
            }
        }
    }

    /// Multiply every coefficient by a scalar.
    pub fn scalar_mul(&self, s: &BigRational) -> Self {
        if s.is_zero() {
            return Self::zero(&self.variable);
        }
        let coeffs = self.coeffs.iter().map(|c| c * s).collect();
        Polynomial {
            coeffs,
            variable: self.variable.clone(),
        }
    }

    /// Content: the GCD of the numerators divided by the LCM of the denominators.
    /// For a polynomial in Q[x], this makes the polynomial "primitive" over Z
    /// when you divide by it.
    pub fn content(&self) -> BigRational {
        if self.is_zero() {
            return BigRational::zero();
        }
        let mut numer_gcd = BigInt::zero();
        let mut denom_lcm = BigInt::one();
        for c in &self.coeffs {
            numer_gcd = gcd_bigint(&numer_gcd, c.numer());
            denom_lcm = lcm_bigint(&denom_lcm, c.denom());
        }
        if numer_gcd.is_zero() {
            BigRational::zero()
        } else {
            BigRational::new(numer_gcd, denom_lcm)
        }
    }

    /// Primitive part: self / content(self). The result has integer coefficients
    /// with GCD 1.
    pub fn primitive_part(&self) -> Self {
        let c = self.content();
        if c.is_zero() {
            return self.clone();
        }
        let inv = BigRational::one() / c;
        self.scalar_mul(&inv)
    }

    /// Polynomial long division: self = quotient * divisor + remainder.
    /// Returns (quotient, remainder).
    pub fn div_rem(&self, divisor: &Polynomial) -> Result<(Polynomial, Polynomial), String> {
        if divisor.is_zero() {
            return Err("Division by zero polynomial".to_string());
        }

        let divisor_deg = divisor.degree().unwrap();
        let divisor_lc = divisor.leading_coeff().unwrap();

        let mut remainder = self.clone();
        let self_deg = match self.degree() {
            Some(d) if d >= divisor_deg => d,
            _ => return Ok((Self::zero(&self.variable), self.clone())),
        };

        let mut quotient_coeffs = vec![BigRational::zero(); self_deg - divisor_deg + 1];

        while let Some(rem_deg) = remainder.degree() {
            if rem_deg < divisor_deg {
                break;
            }
            let rem_lc = remainder.leading_coeff().unwrap().clone();
            let q_coeff = &rem_lc / divisor_lc;
            let deg_diff = rem_deg - divisor_deg;

            quotient_coeffs[deg_diff] = q_coeff.clone();

            let term = Self::monomial(q_coeff, deg_diff, &self.variable);
            let sub = &term * divisor;
            remainder = &remainder - &sub;
        }

        Ok((
            Polynomial::from_coeffs(quotient_coeffs, &self.variable),
            remainder,
        ))
    }

    /// GCD of two polynomials over Q using the Euclidean algorithm.
    /// Result is monic.
    pub fn gcd(&self, other: &Polynomial) -> Self {
        if self.is_zero() {
            return if other.is_zero() {
                Self::zero(&self.variable)
            } else {
                other.make_monic()
            };
        }
        if other.is_zero() {
            return self.make_monic();
        }

        let mut a = self.clone();
        let mut b = other.clone();

        // Ensure deg(a) >= deg(b)
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
    /// The gcd is monic; s, t are adjusted accordingly.
    pub fn extended_gcd(a: &Polynomial, b: &Polynomial) -> (Polynomial, Polynomial, Polynomial) {
        let var = a.variable().to_string();
        if b.is_zero() {
            if a.is_zero() {
                return (Self::zero(&var), Self::one(&var), Self::zero(&var));
            }
            let lc = a.leading_coeff().unwrap().clone();
            let lc_inv = BigRational::one() / lc;
            return (
                a.scalar_mul(&lc_inv),
                Self::constant(lc_inv, &var),
                Self::zero(&var),
            );
        }

        let mut old_r = a.clone();
        let mut r = b.clone();
        let mut old_s = Self::one(&var);
        let mut s = Self::zero(&var);
        let mut old_t = Self::zero(&var);
        let mut t = Self::one(&var);

        while !r.is_zero() {
            let (q, rem) = old_r.div_rem(&r).unwrap();
            old_r = r;
            r = rem;
            let new_s = &old_s - &(&q * &s);
            old_s = s;
            s = new_s;
            let new_t = &old_t - &(&q * &t);
            old_t = t;
            t = new_t;
        }

        let lc = old_r.leading_coeff().unwrap().clone();
        let lc_inv = BigRational::one() / lc;
        (
            old_r.scalar_mul(&lc_inv),
            old_s.scalar_mul(&lc_inv),
            old_t.scalar_mul(&lc_inv),
        )
    }

    /// Formal derivative: d/dx(a_n x^n + ... + a_0) = n*a_n x^(n-1) + ...
    pub fn derivative(&self) -> Self {
        if self.coeffs.len() <= 1 {
            return Self::zero(&self.variable);
        }
        let coeffs = self
            .coeffs
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, c)| c * &BigRational::from_integer(BigInt::from(i)))
            .collect();
        Polynomial {
            coeffs,
            variable: self.variable.clone(),
        }
    }

    /// Formal integral: ∫(a_n x^n + ... + a_0) dx = a_n/(n+1) x^(n+1) + ... + a_0 x
    /// The constant of integration is zero.
    pub fn integral(&self) -> Self {
        if self.is_zero() {
            return self.clone();
        }
        let mut coeffs = Vec::with_capacity(self.coeffs.len() + 1);
        coeffs.push(BigRational::zero());
        for (i, c) in self.coeffs.iter().enumerate() {
            coeffs.push(c / &BigRational::from_integer(BigInt::from(i + 1)));
        }
        Polynomial {
            coeffs,
            variable: self.variable.clone(),
        }
    }

    /// Square-free factorization. Returns the square-free part of the polynomial:
    /// the largest factor with no repeated roots.
    /// For f(x), the square-free part is f(x) / gcd(f(x), f'(x)).
    pub fn square_free_part(&self) -> Self {
        if self.degree().unwrap_or(0) <= 1 {
            return self.clone();
        }
        let deriv = self.derivative();
        let g = self.gcd(&deriv);
        if g.is_constant() {
            return self.make_monic();
        }
        let (q, _) = self.div_rem(&g).unwrap();
        q.make_monic()
    }

    /// Full square-free decomposition: f = a_1 * a_2^2 * a_3^3 * ...
    /// Returns vec of (factor, multiplicity) pairs where each factor is square-free
    /// and coprime to all others.
    pub fn square_free_decomposition(&self) -> Vec<(Polynomial, usize)> {
        if self.is_zero() {
            return vec![];
        }
        if self.degree().unwrap_or(0) == 0 {
            return vec![(self.make_monic(), 1)];
        }

        let mut factors = Vec::new();
        let f = self.make_monic();
        let mut g = self.gcd(&self.derivative());
        let mut h = {
            let (q, _) = f.div_rem(&g).unwrap();
            q
        };
        let mut multiplicity = 1;

        while !h.is_constant() {
            let next_g = h.gcd(&g);
            let factor = {
                let (q, _) = h.div_rem(&next_g).unwrap();
                q
            };
            if !factor.is_constant() {
                factors.push((factor.make_monic(), multiplicity));
            }
            h = next_g.clone();
            let (new_g, _) = g.div_rem(&next_g).unwrap();
            g = new_g;
            multiplicity += 1;
        }

        if !g.is_constant() {
            factors.push((g.make_monic(), multiplicity));
        }

        factors
    }

    /// Convert a Node AST into a polynomial, if the expression is polynomial
    /// in the given variable.
    pub fn from_node(node: &Node, var: &str) -> Result<Self, String> {
        match node {
            Node::Num(n) => {
                let r = exact_to_rational(n)?;
                Ok(Self::constant(r, var))
            }
            Node::Variable(v) => {
                if v == var {
                    Ok(Self::x(var))
                } else {
                    Err(format!(
                        "Expression contains variable '{}', expected only '{}'",
                        v, var
                    ))
                }
            }
            Node::Add(left, right) => {
                let l = Self::from_node(left, var)?;
                let r = Self::from_node(right, var)?;
                Ok(&l + &r)
            }
            Node::Subtract(left, right) => {
                let l = Self::from_node(left, var)?;
                let r = Self::from_node(right, var)?;
                Ok(&l - &r)
            }
            Node::Multiply(left, right) => {
                let l = Self::from_node(left, var)?;
                let r = Self::from_node(right, var)?;
                Ok(&l * &r)
            }
            Node::Negate(inner) => {
                let p = Self::from_node(inner, var)?;
                Ok(-&p)
            }
            Node::Power(base, exp) => {
                let base_poly = Self::from_node(base, var)?;
                match exp.as_ref() {
                    Node::Num(n) => {
                        let e = n.to_i64().ok_or("Non-integer exponent")?;
                        if e < 0 {
                            return Err("Negative exponent in polynomial".to_string());
                        }
                        let mut result = Self::one(var);
                        for _ in 0..e {
                            result = &result * &base_poly;
                        }
                        Ok(result)
                    }
                    _ => Err("Non-constant exponent in polynomial".to_string()),
                }
            }
            Node::Divide(num, den) => {
                let n = Self::from_node(num, var)?;
                let d = Self::from_node(den, var)?;
                if !d.is_constant() {
                    return Err("Non-constant denominator in polynomial".to_string());
                }
                let d_val = d.coeff(0);
                if d_val.is_zero() {
                    return Err("Division by zero".to_string());
                }
                Ok(n.scalar_mul(&(BigRational::one() / d_val)))
            }
            _ => Err(format!(
                "Cannot convert {:?} to polynomial",
                std::mem::discriminant(node)
            )),
        }
    }

    /// Synthetic division: divide by (x - root), assuming root is a root of self.
    /// Returns the quotient polynomial of degree (n - 1).
    pub fn deflate(&self, root: &BigRational) -> Self {
        let n = self.coeffs.len();
        if n <= 1 {
            return Self::zero(&self.variable);
        }
        let mut result = vec![BigRational::zero(); n - 1];
        result[n - 2] = self.coeffs[n - 1].clone();
        for i in (0..n - 2).rev() {
            result[i] = &self.coeffs[i + 1] + root * &result[i + 1];
        }
        Polynomial::from_coeffs(result, &self.variable)
    }

    /// Find all rational roots using the rational root theorem.
    /// For p(x) with integer coefficients, any rational root p/q (in lowest terms)
    /// has p | a_0 and q | a_n. We convert to primitive part first to ensure
    /// integer coefficients.
    pub fn rational_roots(&self) -> Vec<BigRational> {
        if self.is_zero() || self.is_constant() {
            return vec![];
        }

        // Degree 1 is solved by division, not by root search: a·x + b = 0
        // has exactly the root −b/a. The candidate enumeration below
        // factors the constant term, which for large coefficients (an
        // equation mentioning 10⁻¹⁵ has a 16-digit constant) turns a
        // one-step solve into seconds of trial division.
        if self.degree() == Some(1) {
            let a = self.leading_coeff().unwrap().clone();
            let b = self.coeff(0);
            return vec![-b / a];
        }

        let prim = self.primitive_part();

        if prim.coeff(0).is_zero() {
            let mut roots = vec![BigRational::zero()];
            let deflated = prim.deflate(&BigRational::zero());
            roots.extend(deflated.rational_roots());
            return roots;
        }

        let a0 = prim.coeff(0);
        let an = prim.leading_coeff().unwrap().clone();

        let a0_i64 = match a0.numer().to_i64() {
            Some(v) => v,
            None => return vec![],
        };
        let an_i64 = match an.numer().to_i64() {
            Some(v) => v,
            None => return vec![],
        };

        let p_divs = divisors_i64(a0_i64);
        let q_divs = divisors_i64(an_i64);

        let mut roots = Vec::new();
        let mut seen = Vec::new();

        for p in &p_divs {
            for q in &q_divs {
                for sign in &[1i64, -1i64] {
                    let candidate = BigRational::new(BigInt::from(sign * p), BigInt::from(*q));
                    if seen.contains(&candidate) {
                        continue;
                    }
                    seen.push(candidate.clone());
                    if self.evaluate(&candidate).is_zero() {
                        roots.push(candidate);
                    }
                }
            }
        }

        roots
    }

    /// Convert back to a Node AST.
    pub fn to_node(&self) -> Node {
        if self.is_zero() {
            return Node::Num(ExactNum::zero());
        }

        // Collect (degree, coefficient) pairs for nonzero terms, highest degree first
        let mut terms: Vec<(usize, BigRational)> = self
            .coeffs
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_zero())
            .map(|(i, c)| (i, c.clone()))
            .collect();
        terms.reverse();

        if terms.is_empty() {
            return Node::Num(ExactNum::zero());
        }

        let make_term = |deg: usize, coeff: &BigRational, var: &str| -> Node {
            let abs_coeff = coeff.abs();
            if deg == 0 {
                rational_to_node(&abs_coeff)
            } else {
                let var_part = if deg == 1 {
                    Node::Variable(var.to_string())
                } else {
                    Node::Power(
                        Box::new(Node::Variable(var.to_string())),
                        Box::new(Node::Num(ExactNum::integer(deg as i64))),
                    )
                };
                if abs_coeff.is_one() {
                    var_part
                } else {
                    Node::Multiply(Box::new(rational_to_node(&abs_coeff)), Box::new(var_part))
                }
            }
        };

        let (first_deg, first_coeff) = terms.remove(0);
        let first_term = make_term(first_deg, &first_coeff, &self.variable);
        let mut result = if first_coeff.is_negative() {
            Node::Negate(Box::new(first_term))
        } else {
            first_term
        };

        for (deg, coeff) in terms {
            let term = make_term(deg, &coeff, &self.variable);
            if coeff.is_negative() {
                result = Node::Subtract(Box::new(result), Box::new(term));
            } else {
                result = Node::Add(Box::new(result), Box::new(term));
            }
        }

        result
    }
}

pub(crate) fn rational_to_node(r: &BigRational) -> Node {
    if r.is_integer() {
        Node::Num(ExactNum::integer(r.numer().try_into().unwrap_or(0)))
    } else {
        Node::Num(ExactNum::rational(
            r.numer().try_into().unwrap_or(0),
            r.denom().try_into().unwrap_or(1),
        ))
    }
}

fn exact_to_rational(n: &ExactNum) -> Result<BigRational, String> {
    match n {
        ExactNum::Rational(r) => Ok(r.clone()),
        ExactNum::Float(f) => BigRational::from_float(*f)
            .ok_or_else(|| "Cannot convert float to exact rational for polynomial".to_string()),
    }
}

fn divisors_i64(n: i64) -> Vec<i64> {
    let n = n.abs();
    if n == 0 {
        return vec![];
    }
    let mut result = Vec::new();
    let mut i = 1i64;
    while i * i <= n {
        if n % i == 0 {
            result.push(i);
            if i != n / i {
                result.push(n / i);
            }
        }
        i += 1;
    }
    result
}

pub(crate) fn gcd_bigint(a: &BigInt, b: &BigInt) -> BigInt {
    let mut a = a.abs();
    let mut b = b.abs();
    while !b.is_zero() {
        let t = b.clone();
        b = &a % &b;
        a = t;
    }
    a
}

pub(crate) fn lcm_bigint(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_zero() || b.is_zero() {
        return BigInt::zero();
    }
    let g = gcd_bigint(a, b);
    (a / &g) * b
}

/// GCD of two rational numbers: gcd(a/b, c/d) = gcd(a,c) / lcm(b,d)
pub(crate) fn rational_gcd(a: &BigRational, b: &BigRational) -> BigRational {
    if a.is_zero() {
        return b.abs();
    }
    if b.is_zero() {
        return a.abs();
    }
    let numer = gcd_bigint(a.numer(), b.numer());
    let denom = lcm_bigint(a.denom(), b.denom());
    BigRational::new(numer, denom)
}

// --- Operator implementations ---

impl<'a> Add for &'a Polynomial {
    type Output = Polynomial;
    fn add(self, rhs: &'a Polynomial) -> Polynomial {
        let len = self.coeffs.len().max(rhs.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self
                .coeffs
                .get(i)
                .cloned()
                .unwrap_or_else(BigRational::zero);
            let b = rhs.coeffs.get(i).cloned().unwrap_or_else(BigRational::zero);
            coeffs.push(a + b);
        }
        Polynomial::from_coeffs(coeffs, &self.variable)
    }
}

impl<'a> Sub for &'a Polynomial {
    type Output = Polynomial;
    fn sub(self, rhs: &'a Polynomial) -> Polynomial {
        let len = self.coeffs.len().max(rhs.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self
                .coeffs
                .get(i)
                .cloned()
                .unwrap_or_else(BigRational::zero);
            let b = rhs.coeffs.get(i).cloned().unwrap_or_else(BigRational::zero);
            coeffs.push(a - b);
        }
        Polynomial::from_coeffs(coeffs, &self.variable)
    }
}

impl<'a> Mul for &'a Polynomial {
    type Output = Polynomial;
    fn mul(self, rhs: &'a Polynomial) -> Polynomial {
        if self.is_zero() || rhs.is_zero() {
            return Polynomial::zero(&self.variable);
        }
        let len = self.coeffs.len() + rhs.coeffs.len() - 1;
        let mut coeffs = vec![BigRational::zero(); len];
        for (i, a) in self.coeffs.iter().enumerate() {
            if a.is_zero() {
                continue;
            }
            for (j, b) in rhs.coeffs.iter().enumerate() {
                coeffs[i + j] = &coeffs[i + j] + &(a * b);
            }
        }
        Polynomial::from_coeffs(coeffs, &self.variable)
    }
}

impl Neg for &Polynomial {
    type Output = Polynomial;
    fn neg(self) -> Polynomial {
        let coeffs = self.coeffs.iter().map(|c| -c).collect();
        Polynomial {
            coeffs,
            variable: self.variable.clone(),
        }
    }
}

impl PartialEq for Polynomial {
    fn eq(&self, other: &Self) -> bool {
        self.coeffs == other.coeffs
    }
}

impl fmt::Display for Polynomial {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }

        let mut first = true;
        for (i, coeff) in self.coeffs.iter().enumerate().rev() {
            if coeff.is_zero() {
                continue;
            }

            let is_negative = coeff.is_negative();
            let abs_coeff = coeff.abs();

            if !first {
                if is_negative {
                    write!(f, " - ")?;
                } else {
                    write!(f, " + ")?;
                }
            } else if is_negative {
                write!(f, "-")?;
            }

            let show_coeff = i == 0 || !abs_coeff.is_one();

            if show_coeff {
                if abs_coeff.is_integer() {
                    write!(f, "{}", abs_coeff.numer())?;
                } else {
                    write!(f, "{}/{}", abs_coeff.numer(), abs_coeff.denom())?;
                }
            }

            if i > 0 {
                write!(f, "{}", self.variable)?;
                if i > 1 {
                    write!(f, "^{}", i)?;
                }
            }

            first = false;
        }
        Ok(())
    }
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
    fn test_construction() {
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        assert_eq!(p.degree(), Some(2));
        assert_eq!(*p.leading_coeff().unwrap(), int(3));
        assert_eq!(format!("{}", p), "3x^2 + 2x + 1");
    }

    #[test]
    fn test_zero_stripping() {
        let p = Polynomial::from_coeffs(vec![int(1), int(0), int(0)], "x");
        assert_eq!(p.degree(), Some(0));
        assert_eq!(format!("{}", p), "1");
    }

    #[test]
    fn test_addition() {
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        let q = Polynomial::from_coeffs(vec![int(4), int(5)], "x");
        let sum = &p + &q;
        assert_eq!(format!("{}", sum), "3x^2 + 7x + 5");
    }

    #[test]
    fn test_subtraction() {
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        let q = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        let diff = &p - &q;
        assert!(diff.is_zero());
    }

    #[test]
    fn test_multiplication() {
        // (x + 1)(x + 1) = x^2 + 2x + 1
        let p = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let sq = &p * &p;
        assert_eq!(format!("{}", sq), "x^2 + 2x + 1");
    }

    #[test]
    fn test_multiplication_by_zero() {
        let p = Polynomial::from_coeffs(vec![int(1), int(2)], "x");
        let z = Polynomial::zero("x");
        assert!((&p * &z).is_zero());
    }

    #[test]
    fn test_div_rem() {
        // (x^2 + 2x + 1) / (x + 1) = (x + 1), remainder 0
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(1)], "x");
        let d = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let (q, r) = p.div_rem(&d).unwrap();
        assert_eq!(format!("{}", q), "x + 1");
        assert!(r.is_zero());
    }

    #[test]
    fn test_div_rem_with_remainder() {
        // (x^2 + 1) / (x + 1) = (x - 1), remainder 2
        let p = Polynomial::from_coeffs(vec![int(1), int(0), int(1)], "x");
        let d = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let (q, r) = p.div_rem(&d).unwrap();
        assert_eq!(format!("{}", q), "x - 1");
        assert_eq!(format!("{}", r), "2");
    }

    #[test]
    fn test_gcd_common_factor() {
        // gcd(x^2 - 1, x^2 + 2x + 1) = x + 1
        // x^2 - 1 = (x-1)(x+1), x^2 + 2x + 1 = (x+1)^2
        let p = Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let q = Polynomial::from_coeffs(vec![int(1), int(2), int(1)], "x");
        let g = p.gcd(&q);
        assert_eq!(format!("{}", g), "x + 1");
    }

    #[test]
    fn test_gcd_coprime() {
        // gcd(x + 1, x + 2) = 1
        let p = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let q = Polynomial::from_coeffs(vec![int(2), int(1)], "x");
        let g = p.gcd(&q);
        assert!(g.is_constant());
        assert_eq!(g.coeff(0), int(1));
    }

    #[test]
    fn test_evaluate_horner() {
        // p(x) = 2x^2 + 3x + 1, p(2) = 8 + 6 + 1 = 15
        let p = Polynomial::from_coeffs(vec![int(1), int(3), int(2)], "x");
        assert_eq!(p.evaluate(&int(2)), int(15));
    }

    #[test]
    fn test_evaluate_horner_rational() {
        // p(x) = x^2, p(1/2) = 1/4
        let p = Polynomial::from_coeffs(vec![int(0), int(0), int(1)], "x");
        assert_eq!(p.evaluate(&rat(1, 2)), rat(1, 4));
    }

    #[test]
    fn test_from_node_linear() {
        let node = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(3))),
                Box::new(Node::Variable("x".to_string())),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let p = Polynomial::from_node(&node, "x").unwrap();
        assert_eq!(format!("{}", p), "3x + 1");
    }

    #[test]
    fn test_from_node_power() {
        // x^2 + 1
        let node = Node::Add(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let p = Polynomial::from_node(&node, "x").unwrap();
        assert_eq!(format!("{}", p), "x^2 + 1");
    }

    #[test]
    fn test_to_node_roundtrip() {
        let p = Polynomial::from_coeffs(vec![int(1), int(-2), int(3)], "x");
        let node = p.to_node();
        let p2 = Polynomial::from_node(&node, "x").unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn test_content_and_primitive() {
        // 2x^2 + 4x + 6 → content = 2, primitive = x^2 + 2x + 3
        let p = Polynomial::from_coeffs(vec![int(6), int(4), int(2)], "x");
        assert_eq!(p.content(), int(2));
        let pp = p.primitive_part();
        assert_eq!(format!("{}", pp), "x^2 + 2x + 3");
    }

    #[test]
    fn test_content_rational_coefficients() {
        // (1/2)x + (1/3) → content = 1/6, primitive = 3x + 2
        let p = Polynomial::from_coeffs(vec![rat(1, 3), rat(1, 2)], "x");
        assert_eq!(p.content(), rat(1, 6));
        let pp = p.primitive_part();
        assert_eq!(format!("{}", pp), "3x + 2");
    }

    #[test]
    fn test_make_monic() {
        let p = Polynomial::from_coeffs(vec![int(2), int(4)], "x");
        let m = p.make_monic();
        assert_eq!(format!("{}", m), "x + 1/2");
    }

    #[test]
    fn test_display_negative_leading() {
        let p = Polynomial::from_coeffs(vec![int(1), int(-1)], "x");
        assert_eq!(format!("{}", p), "-x + 1");
    }

    #[test]
    fn test_display_constant() {
        let p = Polynomial::from_coeffs(vec![int(42)], "x");
        assert_eq!(format!("{}", p), "42");
    }

    #[test]
    fn test_display_zero() {
        let p = Polynomial::zero("x");
        assert_eq!(format!("{}", p), "0");
    }

    #[test]
    fn test_derivative() {
        // d/dx(3x^2 + 2x + 1) = 6x + 2
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        let dp = p.derivative();
        assert_eq!(format!("{}", dp), "6x + 2");
    }

    #[test]
    fn test_derivative_constant() {
        let p = Polynomial::from_coeffs(vec![int(5)], "x");
        assert!(p.derivative().is_zero());
    }

    #[test]
    fn test_derivative_linear() {
        // d/dx(3x + 1) = 3
        let p = Polynomial::from_coeffs(vec![int(1), int(3)], "x");
        let dp = p.derivative();
        assert_eq!(format!("{}", dp), "3");
    }

    #[test]
    fn test_integral() {
        // ∫(6x + 2) dx = 3x^2 + 2x
        let p = Polynomial::from_coeffs(vec![int(2), int(6)], "x");
        let ip = p.integral();
        assert_eq!(format!("{}", ip), "3x^2 + 2x");
    }

    #[test]
    fn test_integral_of_derivative() {
        // ∫(d/dx(x^3 + x)) dx = x^3 + x (up to constant)
        let p = Polynomial::from_coeffs(vec![int(0), int(1), int(0), int(1)], "x");
        let dp = p.derivative();
        let idp = dp.integral();
        assert_eq!(format!("{}", idp), "x^3 + x");
    }

    #[test]
    fn test_integral_rational() {
        // ∫x^2 dx = (1/3)x^3
        let p = Polynomial::from_coeffs(vec![int(0), int(0), int(1)], "x");
        let ip = p.integral();
        assert_eq!(ip.coeff(3), rat(1, 3));
    }

    #[test]
    fn test_deflate() {
        // (x^2 - 5x + 6) = (x-2)(x-3), deflate by root 2 → (x-3)
        let p = Polynomial::from_coeffs(vec![int(6), int(-5), int(1)], "x");
        let q = p.deflate(&int(2));
        assert_eq!(format!("{}", q), "x - 3");
    }

    #[test]
    fn test_deflate_cubic() {
        // x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3), deflate by 1
        let p = Polynomial::from_coeffs(vec![int(-6), int(11), int(-6), int(1)], "x");
        let q = p.deflate(&int(1));
        assert_eq!(format!("{}", q), "x^2 - 5x + 6");
    }

    #[test]
    fn test_rational_roots_cubic() {
        // x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3)
        let p = Polynomial::from_coeffs(vec![int(-6), int(11), int(-6), int(1)], "x");
        let mut roots = p.rational_roots();
        roots.sort();
        assert_eq!(roots, vec![int(1), int(2), int(3)]);
    }

    #[test]
    fn test_rational_roots_with_zero() {
        // x^2 - x = x(x-1)
        let p = Polynomial::from_coeffs(vec![int(0), int(-1), int(1)], "x");
        let mut roots = p.rational_roots();
        roots.sort();
        assert_eq!(roots, vec![int(0), int(1)]);
    }

    #[test]
    fn test_rational_roots_none() {
        // x^2 + 1 has no rational roots
        let p = Polynomial::from_coeffs(vec![int(1), int(0), int(1)], "x");
        let roots = p.rational_roots();
        assert!(roots.is_empty());
    }

    #[test]
    fn test_rational_roots_fractional() {
        // 2x - 1 has root 1/2
        let p = Polynomial::from_coeffs(vec![int(-1), int(2)], "x");
        let roots = p.rational_roots();
        assert_eq!(roots, vec![rat(1, 2)]);
    }

    #[test]
    fn test_rational_roots_quartic() {
        // x^4 - 5x^2 + 4 = (x-1)(x+1)(x-2)(x+2)
        let p = Polynomial::from_coeffs(vec![int(4), int(0), int(-5), int(0), int(1)], "x");
        let mut roots = p.rational_roots();
        roots.sort();
        assert_eq!(roots, vec![int(-2), int(-1), int(1), int(2)]);
    }

    #[test]
    fn test_square_free_part_no_repeats() {
        // x^2 - 1 = (x-1)(x+1) — already square-free
        let p = Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let sf = p.square_free_part();
        assert_eq!(sf.degree(), Some(2));
    }

    #[test]
    fn test_square_free_part_with_repeats() {
        // (x+1)^2 = x^2 + 2x + 1 → square-free part = x + 1
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(1)], "x");
        let sf = p.square_free_part();
        assert_eq!(format!("{}", sf), "x + 1");
    }

    #[test]
    fn test_square_free_decomposition_simple() {
        // (x+1)^2 * (x-1) = x^3 + x^2 - x - 1
        let x_plus_1 = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let x_minus_1 = Polynomial::from_coeffs(vec![int(-1), int(1)], "x");
        let f = &(&x_plus_1 * &x_plus_1) * &x_minus_1;

        let decomp = f.square_free_decomposition();
        assert_eq!(decomp.len(), 2);

        let (f1, m1) = &decomp[0];
        assert_eq!(*m1, 1);
        assert_eq!(format!("{}", f1), "x - 1");

        let (f2, m2) = &decomp[1];
        assert_eq!(*m2, 2);
        assert_eq!(format!("{}", f2), "x + 1");
    }

    #[test]
    fn test_square_free_decomposition_cube() {
        // (x+1)^3 = x^3 + 3x^2 + 3x + 1
        let x_plus_1 = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let f = &(&x_plus_1 * &x_plus_1) * &x_plus_1;

        let decomp = f.square_free_decomposition();
        assert_eq!(decomp.len(), 1);
        let (factor, mult) = &decomp[0];
        assert_eq!(*mult, 3);
        assert_eq!(format!("{}", factor), "x + 1");
    }
}
