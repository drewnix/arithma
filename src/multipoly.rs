use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeSet;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::{gcd_bigint, lcm_bigint, Polynomial};

/// Multivariate polynomial over Q with recursive representation.
///
/// Variable ordering is lexicographic (alphabetical): the earliest variable
/// alphabetically is the outermost. A polynomial in x, y is stored as a
/// polynomial in x whose coefficients are polynomials in y.
///
/// Invariants:
/// - In `Poly { var, coeffs }`, the last element of `coeffs` is non-zero
/// - `coeffs` is never empty (use `Constant(zero)` for the zero polynomial)
/// - Variables are properly nested: a coefficient of a Poly with var "x"
///   contains only variables that sort after "x"
#[derive(Debug, Clone)]
pub enum MultiPoly {
    Constant(BigRational),
    Poly {
        var: String,
        coeffs: Vec<MultiPoly>,
    },
}

impl MultiPoly {
    pub fn zero() -> Self {
        MultiPoly::Constant(BigRational::zero())
    }

    pub fn one() -> Self {
        MultiPoly::Constant(BigRational::one())
    }

    pub fn constant(c: BigRational) -> Self {
        MultiPoly::Constant(c)
    }

    pub fn integer(n: i64) -> Self {
        MultiPoly::Constant(BigRational::from_integer(BigInt::from(n)))
    }

    pub fn variable(var: &str) -> Self {
        MultiPoly::Poly {
            var: var.to_string(),
            coeffs: vec![MultiPoly::zero(), MultiPoly::one()],
        }
    }

    pub fn monomial(coeff: MultiPoly, var: &str, degree: usize) -> Self {
        if degree == 0 {
            return coeff;
        }
        let mut coeffs = vec![MultiPoly::zero(); degree];
        coeffs.push(coeff);
        MultiPoly::from_coeffs(var, coeffs)
    }

    fn from_coeffs(var: &str, mut coeffs: Vec<MultiPoly>) -> Self {
        while coeffs.last().map_or(false, |c| c.is_zero()) {
            coeffs.pop();
        }
        if coeffs.is_empty() {
            MultiPoly::zero()
        } else if coeffs.len() == 1 && matches!(&coeffs[0], MultiPoly::Constant(_)) {
            coeffs.pop().unwrap()
        } else if coeffs.len() == 1 {
            // degree-0 poly: just the coefficient itself, unless it references
            // a variable that should sort after var (in which case we collapse)
            let c = &coeffs[0];
            if c.main_var().map_or(true, |v| v.as_str() > var) {
                coeffs.pop().unwrap()
            } else {
                MultiPoly::Poly {
                    var: var.to_string(),
                    coeffs,
                }
            }
        } else {
            MultiPoly::Poly {
                var: var.to_string(),
                coeffs,
            }
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            MultiPoly::Constant(c) => c.is_zero(),
            MultiPoly::Poly { .. } => false,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, MultiPoly::Constant(_))
    }

    pub fn as_constant(&self) -> Option<&BigRational> {
        match self {
            MultiPoly::Constant(c) => Some(c),
            _ => None,
        }
    }

    pub fn main_var(&self) -> Option<&String> {
        match self {
            MultiPoly::Constant(_) => None,
            MultiPoly::Poly { var, .. } => Some(var),
        }
    }

    /// Total degree of the polynomial.
    pub fn total_degree(&self) -> usize {
        match self {
            MultiPoly::Constant(c) => {
                if c.is_zero() {
                    0
                } else {
                    0
                }
            }
            MultiPoly::Poly { coeffs, .. } => {
                let mut max_deg = 0;
                for (i, c) in coeffs.iter().enumerate() {
                    if !c.is_zero() {
                        max_deg = max_deg.max(i + c.total_degree());
                    }
                }
                max_deg
            }
        }
    }

    /// Degree in a specific variable.
    pub fn degree_in(&self, target: &str) -> usize {
        match self {
            MultiPoly::Constant(_) => 0,
            MultiPoly::Poly { var, coeffs } => {
                if var == target {
                    coeffs.len().saturating_sub(1)
                } else {
                    coeffs.iter().map(|c| c.degree_in(target)).max().unwrap_or(0)
                }
            }
        }
    }

    /// All variables appearing in the polynomial, sorted alphabetically.
    pub fn variables(&self) -> Vec<String> {
        let mut vars = BTreeSet::new();
        self.collect_vars(&mut vars);
        vars.into_iter().collect()
    }

    fn collect_vars(&self, vars: &mut BTreeSet<String>) {
        match self {
            MultiPoly::Constant(_) => {}
            MultiPoly::Poly { var, coeffs } => {
                vars.insert(var.clone());
                for c in coeffs {
                    c.collect_vars(vars);
                }
            }
        }
    }

    /// Coefficient of var^i in the main variable.
    pub fn coeff(&self, i: usize) -> MultiPoly {
        match self {
            MultiPoly::Constant(_) => {
                if i == 0 {
                    self.clone()
                } else {
                    MultiPoly::zero()
                }
            }
            MultiPoly::Poly { coeffs, .. } => {
                coeffs.get(i).cloned().unwrap_or_else(MultiPoly::zero)
            }
        }
    }

    /// Leading coefficient in the main variable.
    pub fn leading_coeff(&self) -> MultiPoly {
        match self {
            MultiPoly::Constant(_) => self.clone(),
            MultiPoly::Poly { coeffs, .. } => {
                coeffs.last().cloned().unwrap_or_else(MultiPoly::zero)
            }
        }
    }

    /// Partial derivative with respect to a variable.
    pub fn partial_derivative(&self, target: &str) -> Self {
        match self {
            MultiPoly::Constant(_) => MultiPoly::zero(),
            MultiPoly::Poly { var, coeffs } => {
                if var == target {
                    if coeffs.len() <= 1 {
                        return MultiPoly::zero();
                    }
                    let new_coeffs: Vec<MultiPoly> = coeffs
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, c)| {
                            let i_rat = BigRational::from_integer(BigInt::from(i));
                            c * &MultiPoly::Constant(i_rat)
                        })
                        .collect();
                    MultiPoly::from_coeffs(var, new_coeffs)
                } else {
                    let new_coeffs: Vec<MultiPoly> = coeffs
                        .iter()
                        .map(|c| c.partial_derivative(target))
                        .collect();
                    MultiPoly::from_coeffs(var, new_coeffs)
                }
            }
        }
    }

    /// Evaluate: substitute a rational value for a variable.
    pub fn evaluate_at(&self, target: &str, value: &BigRational) -> Self {
        match self {
            MultiPoly::Constant(_) => self.clone(),
            MultiPoly::Poly { var, coeffs } => {
                if var == target {
                    // Horner's method
                    let val_poly = MultiPoly::Constant(value.clone());
                    let mut result = MultiPoly::zero();
                    for c in coeffs.iter().rev() {
                        result = &(&result * &val_poly) + c;
                    }
                    result
                } else {
                    let new_coeffs: Vec<MultiPoly> = coeffs
                        .iter()
                        .map(|c| c.evaluate_at(target, value))
                        .collect();
                    MultiPoly::from_coeffs(var, new_coeffs)
                }
            }
        }
    }

    /// Substitute a polynomial for a variable.
    pub fn substitute(&self, target: &str, replacement: &MultiPoly) -> Self {
        match self {
            MultiPoly::Constant(_) => self.clone(),
            MultiPoly::Poly { var, coeffs } => {
                if var == target {
                    // Horner's method with polynomial replacement
                    let mut result = MultiPoly::zero();
                    for c in coeffs.iter().rev() {
                        let c_sub = c.substitute(target, replacement);
                        result = &(&result * replacement) + &c_sub;
                    }
                    result
                } else {
                    // Substitute in each coefficient, then rebuild.
                    // The substituted coefficients may now contain `var`,
                    // so we expand via Horner in var.
                    let var_poly = MultiPoly::variable(var);
                    let mut result = MultiPoly::zero();
                    for c in coeffs.iter().rev() {
                        let c_sub = c.substitute(target, replacement);
                        result = &(&result * &var_poly) + &c_sub;
                    }
                    result
                }
            }
        }
    }

    /// Scalar multiplication.
    pub fn scalar_mul(&self, s: &BigRational) -> Self {
        if s.is_zero() {
            return MultiPoly::zero();
        }
        if s.is_one() {
            return self.clone();
        }
        match self {
            MultiPoly::Constant(c) => MultiPoly::Constant(c * s),
            MultiPoly::Poly { var, coeffs } => {
                let new_coeffs = coeffs.iter().map(|c| c.scalar_mul(s)).collect();
                MultiPoly::Poly {
                    var: var.clone(),
                    coeffs: new_coeffs,
                }
            }
        }
    }

    /// Degree in the main variable.
    fn main_degree(&self) -> usize {
        match self {
            MultiPoly::Constant(_) => 0,
            MultiPoly::Poly { coeffs, .. } => coeffs.len().saturating_sub(1),
        }
    }

    /// GCD of two rationals: gcd(numer) / lcm(denom).
    fn gcd_rational(a: &BigRational, b: &BigRational) -> BigRational {
        if a.is_zero() {
            return b.abs();
        }
        if b.is_zero() {
            return a.abs();
        }
        let n = gcd_bigint(a.numer(), b.numer());
        let d = lcm_bigint(a.denom(), b.denom());
        BigRational::new(n.abs(), d)
    }

    /// Pseudo-remainder of self divided by other in the main variable.
    /// Both must share the same main variable.
    /// Returns r such that lc(other)^(δ+1) · self ≡ q·other + r with deg(r) < deg(other),
    /// where δ = deg(self) - deg(other).
    pub fn pseudo_remainder(&self, other: &MultiPoly) -> MultiPoly {
        let var = match self.main_var() {
            Some(v) => v.clone(),
            None => return self.clone(),
        };

        let other_deg = match other.main_var() {
            Some(v) if *v == var => other.main_degree(),
            _ => return MultiPoly::zero(),
        };

        let lc_other = other.leading_coeff();
        let mut rem = self.clone();

        // At each step: rem ← lc(other)·rem - lc(rem)·x^(deg_diff)·other
        // Leading terms cancel, degree drops by ≥1. After δ+1 steps we have
        // lc(other)^(δ+1)·self ≡ q·other + rem.
        while !rem.is_zero() {
            let still_in_var = rem.main_var().map_or(false, |v| *v == var);
            if !still_in_var || rem.main_degree() < other_deg {
                break;
            }
            let rem_lc = rem.leading_coeff();
            let deg_diff = rem.main_degree() - other_deg;

            let scaled_rem = &rem * &lc_other;
            let term = MultiPoly::monomial(rem_lc, &var, deg_diff);
            let sub = &term * other;
            rem = &scaled_rem - &sub;
        }

        rem
    }

    /// Content: GCD of all coefficients in the main variable.
    pub fn content(&self) -> MultiPoly {
        match self {
            MultiPoly::Constant(_) => self.abs_rational(),
            MultiPoly::Poly { coeffs, .. } => {
                let mut g = MultiPoly::zero();
                for c in coeffs {
                    if c.is_zero() {
                        continue;
                    }
                    g = MultiPoly::gcd(&g, c);
                    if g.is_one() {
                        return g;
                    }
                }
                g
            }
        }
    }

    fn abs_rational(&self) -> MultiPoly {
        match self {
            MultiPoly::Constant(c) => MultiPoly::Constant(c.abs()),
            _ => self.clone(),
        }
    }

    /// Primitive part: self with the content divided out.
    pub fn primitive_part(&self) -> MultiPoly {
        let c = self.content();
        if c.is_zero() {
            return MultiPoly::zero();
        }
        if c.is_one() {
            return self.clone();
        }
        let result = self.exact_div(&c);
        // Normalize sign: leading coefficient should be positive
        if result.leading_constant().map_or(false, |lc| lc.is_negative()) {
            result.negate()
        } else {
            result
        }
    }

    /// The leading constant: recursively follow leading coefficients to a rational.
    fn leading_constant(&self) -> Option<BigRational> {
        match self {
            MultiPoly::Constant(c) => Some(c.clone()),
            MultiPoly::Poly { coeffs, .. } => {
                coeffs.last().and_then(|c| c.leading_constant())
            }
        }
    }

    /// Exact division: self / divisor where the division is known to be exact.
    pub fn exact_div(&self, divisor: &MultiPoly) -> MultiPoly {
        if divisor.is_one() {
            return self.clone();
        }
        match (self, divisor) {
            (_, MultiPoly::Constant(d)) => {
                if d.is_zero() {
                    panic!("exact_div: division by zero");
                }
                let inv = BigRational::one() / d;
                self.scalar_mul(&inv)
            }
            (MultiPoly::Constant(n), MultiPoly::Poly { .. }) => {
                if n.is_zero() {
                    MultiPoly::zero()
                } else {
                    panic!("exact_div: non-zero constant not divisible by polynomial");
                }
            }
            (
                MultiPoly::Poly {
                    var: var_a,
                    coeffs: _,
                },
                MultiPoly::Poly {
                    var: var_b,
                    coeffs: _,
                },
            ) => {
                if var_a == var_b {
                    self.poly_exact_div(divisor)
                } else if var_a < var_b {
                    // divisor is in our coefficient ring
                    match self {
                        MultiPoly::Poly { var, coeffs } => {
                            let new_coeffs: Vec<MultiPoly> = coeffs
                                .iter()
                                .map(|c| c.exact_div(divisor))
                                .collect();
                            MultiPoly::from_coeffs(var, new_coeffs)
                        }
                        _ => unreachable!(),
                    }
                } else {
                    panic!(
                        "exact_div: divisor variable {} precedes dividend variable {}",
                        var_b, var_a
                    );
                }
            }
        }
    }

    /// Polynomial exact division in the main variable.
    /// Standard long division — works because Q is the ultimate base field.
    fn poly_exact_div(&self, divisor: &MultiPoly) -> MultiPoly {
        let var = match self {
            MultiPoly::Poly { var, .. } => var.clone(),
            _ => unreachable!(),
        };

        let divisor_deg = divisor.main_degree();
        let divisor_lc = divisor.leading_coeff();
        let self_deg = self.main_degree();

        if self_deg < divisor_deg {
            if self.is_zero() {
                return MultiPoly::zero();
            }
            panic!("exact_div: dividend degree {} < divisor degree {}", self_deg, divisor_deg);
        }

        let mut q_coeffs = vec![MultiPoly::zero(); self_deg - divisor_deg + 1];
        let mut rem = self.clone();

        while !rem.is_zero() {
            let in_var = rem.main_var().map_or(false, |v| *v == var);
            if !in_var {
                panic!("exact_div: nonzero remainder in wrong variable");
            }
            let rem_deg = rem.main_degree();
            if rem_deg < divisor_deg {
                panic!("exact_div: nonzero remainder (deg {} < {})", rem_deg, divisor_deg);
            }
            let rem_lc = rem.leading_coeff();
            let q_coeff = rem_lc.exact_div(&divisor_lc);
            let deg_diff = rem_deg - divisor_deg;
            q_coeffs[deg_diff] = q_coeff.clone();

            let q_term = MultiPoly::monomial(q_coeff, &var, deg_diff);
            let sub = &q_term * divisor;
            rem = &rem - &sub;
        }

        MultiPoly::from_coeffs(&var, q_coeffs)
    }

    /// Negate if leading constant is negative, so the result has positive
    /// leading coefficient. Used for GCD normalization.
    fn make_positive(&self) -> MultiPoly {
        if self.leading_constant().map_or(false, |lc| lc.is_negative()) {
            self.negate()
        } else {
            self.clone()
        }
    }

    /// GCD of two multivariate polynomials via the primitive polynomial
    /// remainder sequence (PRS). Result has positive leading constant.
    pub fn gcd(a: &MultiPoly, b: &MultiPoly) -> MultiPoly {
        if a.is_zero() {
            return b.make_positive();
        }
        if b.is_zero() {
            return a.make_positive();
        }

        match (a, b) {
            (MultiPoly::Constant(ra), MultiPoly::Constant(rb)) => {
                MultiPoly::Constant(Self::gcd_rational(ra, rb))
            }
            (MultiPoly::Constant(_), MultiPoly::Poly { .. }) => {
                MultiPoly::gcd(a, &b.content())
            }
            (MultiPoly::Poly { .. }, MultiPoly::Constant(_)) => {
                MultiPoly::gcd(&a.content(), b)
            }
            (
                MultiPoly::Poly { var: va, .. },
                MultiPoly::Poly { var: vb, .. },
            ) => {
                if va < vb {
                    MultiPoly::gcd(&a.content(), b)
                } else if vb < va {
                    MultiPoly::gcd(a, &b.content())
                } else {
                    // Same main variable — primitive PRS
                    let var = va.clone();
                    let ca = a.content();
                    let cb = b.content();
                    let d = MultiPoly::gcd(&ca, &cb);

                    let mut f = a.exact_div(&ca);
                    let mut g = b.exact_div(&cb);

                    if f.main_degree() < g.main_degree() {
                        std::mem::swap(&mut f, &mut g);
                    }

                    loop {
                        // If g dropped below the main variable, primitives are coprime
                        if g.is_zero() || g.main_var().map_or(true, |v| *v != var) {
                            return d.make_positive();
                        }
                        let r = f.pseudo_remainder(&g);
                        if r.is_zero() {
                            let g = g.primitive_part();
                            return (&d * &g).make_positive();
                        }
                        f = g;
                        g = r.primitive_part();
                    }
                }
            }
        }
    }

    /// Convert from a Node AST. Detects all variables automatically.
    pub fn from_node(node: &Node) -> Result<Self, String> {
        match node {
            Node::Num(n) => {
                let r = exact_to_rational(n)?;
                Ok(MultiPoly::Constant(r))
            }
            Node::Variable(v) => Ok(MultiPoly::variable(v)),
            Node::Add(left, right) => {
                let l = Self::from_node(left)?;
                let r = Self::from_node(right)?;
                Ok(&l + &r)
            }
            Node::Subtract(left, right) => {
                let l = Self::from_node(left)?;
                let r = Self::from_node(right)?;
                Ok(&l - &r)
            }
            Node::Multiply(left, right) => {
                let l = Self::from_node(left)?;
                let r = Self::from_node(right)?;
                Ok(&l * &r)
            }
            Node::Negate(inner) => {
                let p = Self::from_node(inner)?;
                Ok(-&p)
            }
            Node::Power(base, exp) => {
                let base_poly = Self::from_node(base)?;
                match exp.as_ref() {
                    Node::Num(n) => {
                        let e = n.to_i64().ok_or("Non-integer exponent")?;
                        if e < 0 {
                            return Err("Negative exponent in polynomial".to_string());
                        }
                        let mut result = MultiPoly::one();
                        for _ in 0..e {
                            result = &result * &base_poly;
                        }
                        Ok(result)
                    }
                    _ => Err("Non-constant exponent in polynomial".to_string()),
                }
            }
            Node::Divide(num, den) => {
                let n = Self::from_node(num)?;
                let d = Self::from_node(den)?;
                match d.as_constant() {
                    Some(dval) => {
                        if dval.is_zero() {
                            Err("Division by zero".to_string())
                        } else {
                            let inv = BigRational::one() / dval;
                            Ok(n.scalar_mul(&inv))
                        }
                    }
                    None => Err("Non-constant denominator in polynomial".to_string()),
                }
            }
            _ => Err(format!(
                "Cannot convert {:?} to multivariate polynomial",
                std::mem::discriminant(node)
            )),
        }
    }

    /// Convert back to a Node AST.
    pub fn to_node(&self) -> Node {
        match self {
            MultiPoly::Constant(c) => rational_to_node(c),
            MultiPoly::Poly { var, coeffs } => {
                // Collect non-zero terms, highest degree first
                let mut terms: Vec<(usize, &MultiPoly)> = coeffs
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_zero())
                    .collect();
                terms.reverse();

                if terms.is_empty() {
                    return Node::Num(ExactNum::zero());
                }

                let make_var_power = |deg: usize| -> Node {
                    if deg == 1 {
                        Node::Variable(var.clone())
                    } else {
                        Node::Power(
                            Box::new(Node::Variable(var.clone())),
                            Box::new(Node::Num(ExactNum::integer(deg as i64))),
                        )
                    }
                };

                let make_term = |deg: usize, coeff: &MultiPoly| -> (Node, bool) {
                    if deg == 0 {
                        let node = coeff.to_node();
                        let is_neg = coeff.is_negative();
                        (if is_neg { negate_node(node) } else { node }, is_neg)
                    } else {
                        let is_neg = coeff.is_negative();
                        let abs_coeff = if is_neg { coeff.negate() } else { coeff.clone() };

                        let term = if abs_coeff.is_one() {
                            make_var_power(deg)
                        } else {
                            Node::Multiply(
                                Box::new(abs_coeff.to_node()),
                                Box::new(make_var_power(deg)),
                            )
                        };
                        (term, is_neg)
                    }
                };

                let (first_deg, first_coeff) = terms.remove(0);
                let (first_term, first_neg) = make_term(first_deg, first_coeff);
                let mut result = if first_neg {
                    Node::Negate(Box::new(first_term))
                } else {
                    first_term
                };

                for (deg, coeff) in terms {
                    let (term, is_neg) = make_term(deg, coeff);
                    if is_neg {
                        result = Node::Subtract(Box::new(result), Box::new(term));
                    } else {
                        result = Node::Add(Box::new(result), Box::new(term));
                    }
                }

                result
            }
        }
    }

    fn is_negative(&self) -> bool {
        match self {
            MultiPoly::Constant(c) => c.is_negative(),
            MultiPoly::Poly { coeffs, .. } => {
                coeffs.last().map_or(false, |lc| lc.is_negative())
            }
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            MultiPoly::Constant(c) => c.is_one(),
            _ => false,
        }
    }

    /// True if this is a single monomial (needs no parens when used as coefficient).
    fn is_single_term(&self) -> bool {
        match self {
            MultiPoly::Constant(_) => true,
            MultiPoly::Poly { coeffs, .. } => {
                coeffs.iter().filter(|c| !c.is_zero()).count() == 1
                    && coeffs.last().map_or(true, |c| c.is_single_term())
            }
        }
    }

    fn negate(&self) -> Self {
        match self {
            MultiPoly::Constant(c) => MultiPoly::Constant(-c),
            MultiPoly::Poly { var, coeffs } => {
                let new_coeffs = coeffs.iter().map(|c| c.negate()).collect();
                MultiPoly::Poly {
                    var: var.clone(),
                    coeffs: new_coeffs,
                }
            }
        }
    }

    /// Convert from a univariate Polynomial.
    pub fn from_univariate(poly: &Polynomial) -> Self {
        if poly.is_zero() {
            return MultiPoly::zero();
        }
        let var = poly.variable();
        let mut coeffs = Vec::new();
        for i in 0..=poly.degree().unwrap_or(0) {
            let c = poly.coeff(i);
            coeffs.push(MultiPoly::Constant(c));
        }
        MultiPoly::from_coeffs(var, coeffs)
    }

    /// Convert to a univariate Polynomial, if all coefficients are constants.
    pub fn to_univariate(&self) -> Result<Polynomial, String> {
        match self {
            MultiPoly::Constant(c) => Ok(Polynomial::constant(c.clone(), "x")),
            MultiPoly::Poly { var, coeffs } => {
                let mut rat_coeffs = Vec::with_capacity(coeffs.len());
                for c in coeffs {
                    match c {
                        MultiPoly::Constant(r) => rat_coeffs.push(r.clone()),
                        _ => {
                            return Err(format!(
                                "Cannot convert to univariate: coefficient contains variable"
                            ))
                        }
                    }
                }
                Ok(Polynomial::from_coeffs(rat_coeffs, var))
            }
        }
    }
}

fn negate_node(node: Node) -> Node {
    match node {
        Node::Negate(inner) => *inner,
        Node::Num(n) => Node::Num(-n),
        _ => Node::Negate(Box::new(node)),
    }
}

/// Addition: align on the same main variable.
impl<'a> Add for &'a MultiPoly {
    type Output = MultiPoly;

    fn add(self, rhs: &'a MultiPoly) -> MultiPoly {
        match (self, rhs) {
            (MultiPoly::Constant(a), MultiPoly::Constant(b)) => {
                MultiPoly::Constant(a + b)
            }
            (MultiPoly::Constant(_), MultiPoly::Poly { var, coeffs }) => {
                let mut new_coeffs = coeffs.clone();
                if new_coeffs.is_empty() {
                    new_coeffs.push(self.clone());
                } else {
                    new_coeffs[0] = &new_coeffs[0] + self;
                }
                MultiPoly::from_coeffs(var, new_coeffs)
            }
            (MultiPoly::Poly { var, coeffs }, MultiPoly::Constant(_)) => {
                let mut new_coeffs = coeffs.clone();
                if new_coeffs.is_empty() {
                    new_coeffs.push(rhs.clone());
                } else {
                    new_coeffs[0] = &new_coeffs[0] + rhs;
                }
                MultiPoly::from_coeffs(var, new_coeffs)
            }
            (
                MultiPoly::Poly {
                    var: var_a,
                    coeffs: coeffs_a,
                },
                MultiPoly::Poly {
                    var: var_b,
                    coeffs: coeffs_b,
                },
            ) => {
                if var_a == var_b {
                    let len = coeffs_a.len().max(coeffs_b.len());
                    let zero = MultiPoly::zero();
                    let mut new_coeffs = Vec::with_capacity(len);
                    for i in 0..len {
                        let a = coeffs_a.get(i).unwrap_or(&zero);
                        let b = coeffs_b.get(i).unwrap_or(&zero);
                        new_coeffs.push(a + b);
                    }
                    MultiPoly::from_coeffs(var_a, new_coeffs)
                } else if var_a < var_b {
                    // var_a is outermost; rhs is a "constant" wrt var_a
                    let mut new_coeffs = coeffs_a.clone();
                    if new_coeffs.is_empty() {
                        new_coeffs.push(rhs.clone());
                    } else {
                        new_coeffs[0] = &new_coeffs[0] + rhs;
                    }
                    MultiPoly::from_coeffs(var_a, new_coeffs)
                } else {
                    // var_b is outermost; self is a "constant" wrt var_b
                    let mut new_coeffs = coeffs_b.clone();
                    if new_coeffs.is_empty() {
                        new_coeffs.push(self.clone());
                    } else {
                        new_coeffs[0] = &new_coeffs[0] + self;
                    }
                    MultiPoly::from_coeffs(var_b, new_coeffs)
                }
            }
        }
    }
}

impl<'a> Sub for &'a MultiPoly {
    type Output = MultiPoly;

    fn sub(self, rhs: &'a MultiPoly) -> MultiPoly {
        self + &rhs.negate()
    }
}

impl<'a> Mul for &'a MultiPoly {
    type Output = MultiPoly;

    fn mul(self, rhs: &'a MultiPoly) -> MultiPoly {
        match (self, rhs) {
            (MultiPoly::Constant(a), MultiPoly::Constant(b)) => {
                MultiPoly::Constant(a * b)
            }
            (MultiPoly::Constant(a), MultiPoly::Poly { var, coeffs }) => {
                if a.is_zero() {
                    return MultiPoly::zero();
                }
                let new_coeffs = coeffs.iter().map(|c| c.scalar_mul(a)).collect();
                MultiPoly::from_coeffs(var, new_coeffs)
            }
            (MultiPoly::Poly { var, coeffs }, MultiPoly::Constant(b)) => {
                if b.is_zero() {
                    return MultiPoly::zero();
                }
                let new_coeffs = coeffs.iter().map(|c| c.scalar_mul(b)).collect();
                MultiPoly::from_coeffs(var, new_coeffs)
            }
            (
                MultiPoly::Poly {
                    var: var_a,
                    coeffs: coeffs_a,
                },
                MultiPoly::Poly {
                    var: var_b,
                    coeffs: coeffs_b,
                },
            ) => {
                if var_a == var_b {
                    // Standard convolution
                    if coeffs_a.is_empty() || coeffs_b.is_empty() {
                        return MultiPoly::zero();
                    }
                    let len = coeffs_a.len() + coeffs_b.len() - 1;
                    let mut result = vec![MultiPoly::zero(); len];
                    for (i, a) in coeffs_a.iter().enumerate() {
                        if a.is_zero() {
                            continue;
                        }
                        for (j, b) in coeffs_b.iter().enumerate() {
                            let prod = a * b;
                            result[i + j] = &result[i + j] + &prod;
                        }
                    }
                    MultiPoly::from_coeffs(var_a, result)
                } else if var_a < var_b {
                    // var_a outermost; rhs is constant wrt var_a
                    let new_coeffs = coeffs_a.iter().map(|c| c * rhs).collect();
                    MultiPoly::from_coeffs(var_a, new_coeffs)
                } else {
                    // var_b outermost; self is constant wrt var_b
                    let new_coeffs = coeffs_b.iter().map(|c| c * self).collect();
                    MultiPoly::from_coeffs(var_b, new_coeffs)
                }
            }
        }
    }
}

impl<'a> Neg for &'a MultiPoly {
    type Output = MultiPoly;

    fn neg(self) -> MultiPoly {
        self.negate()
    }
}

impl PartialEq for MultiPoly {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MultiPoly::Constant(a), MultiPoly::Constant(b)) => a == b,
            (
                MultiPoly::Poly {
                    var: va,
                    coeffs: ca,
                },
                MultiPoly::Poly {
                    var: vb,
                    coeffs: cb,
                },
            ) => va == vb && ca == cb,
            _ => false,
        }
    }
}

impl fmt::Display for MultiPoly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MultiPoly::Constant(c) => {
                if c.is_integer() {
                    write!(f, "{}", c.numer())
                } else {
                    write!(f, "{}/{}", c.numer(), c.denom())
                }
            }
            MultiPoly::Poly { var, coeffs } => {
                let mut first = true;
                for (i, coeff) in coeffs.iter().enumerate().rev() {
                    if coeff.is_zero() {
                        continue;
                    }

                    let is_neg = coeff.is_negative();
                    let abs_coeff = if is_neg { coeff.negate() } else { coeff.clone() };

                    if !first {
                        if is_neg {
                            write!(f, " - ")?;
                        } else {
                            write!(f, " + ")?;
                        }
                    } else if is_neg {
                        write!(f, "-")?;
                    }

                    let needs_parens = i > 0 && !abs_coeff.is_single_term() && !abs_coeff.is_one();

                    if i == 0 {
                        write!(f, "{}", abs_coeff)?;
                    } else if abs_coeff.is_one() {
                        if i == 1 {
                            write!(f, "{}", var)?;
                        } else {
                            write!(f, "{}^{}", var, i)?;
                        }
                    } else {
                        if needs_parens {
                            write!(f, "({})", abs_coeff)?;
                        } else {
                            write!(f, "{}", abs_coeff)?;
                        }
                        if i == 1 {
                            write!(f, "{}", var)?;
                        } else {
                            write!(f, "{}^{}", var, i)?;
                        }
                    }

                    first = false;
                }
                if first {
                    write!(f, "0")
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn exact_to_rational(n: &ExactNum) -> Result<BigRational, String> {
    match n {
        ExactNum::Rational(r) => Ok(r.clone()),
        ExactNum::Float(_) => {
            Err("Cannot convert float to exact rational for polynomial".to_string())
        }
    }
}

fn rational_to_node(r: &BigRational) -> Node {
    if r.is_integer() {
        Node::Num(ExactNum::integer(r.numer().try_into().unwrap_or(0)))
    } else {
        Node::Num(ExactNum::rational(
            r.numer().try_into().unwrap_or(0),
            r.denom().try_into().unwrap_or(1),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    #[test]
    fn test_construction_variable() {
        let x = MultiPoly::variable("x");
        assert_eq!(format!("{}", x), "x");
        assert_eq!(x.degree_in("x"), 1);
        assert_eq!(x.total_degree(), 1);
    }

    #[test]
    fn test_construction_constant() {
        let c = MultiPoly::integer(5);
        assert_eq!(format!("{}", c), "5");
        assert!(c.is_constant());
        assert_eq!(c.total_degree(), 0);
    }

    #[test]
    fn test_addition_same_var() {
        let x = MultiPoly::variable("x");
        let two_x = &x + &x;
        assert_eq!(format!("{}", two_x), "2x");
    }

    #[test]
    fn test_addition_different_vars() {
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let sum = &x + &y;
        assert_eq!(format!("{}", sum), "x + y");
        assert_eq!(sum.variables(), vec!["x", "y"]);
    }

    #[test]
    fn test_addition_constant() {
        let x = MultiPoly::variable("x");
        let one = MultiPoly::one();
        let sum = &x + &one;
        assert_eq!(format!("{}", sum), "x + 1");
    }

    #[test]
    fn test_subtraction() {
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let diff = &x - &y;
        assert_eq!(format!("{}", diff), "x - y");
    }

    #[test]
    fn test_multiplication_same_var() {
        let x = MultiPoly::variable("x");
        let x_sq = &x * &x;
        assert_eq!(format!("{}", x_sq), "x^2");
        assert_eq!(x_sq.degree_in("x"), 2);
    }

    #[test]
    fn test_multiplication_different_vars() {
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let xy = &x * &y;
        assert_eq!(format!("{}", xy), "yx");
        assert_eq!(xy.degree_in("x"), 1);
        assert_eq!(xy.degree_in("y"), 1);
        assert_eq!(xy.total_degree(), 2);
    }

    #[test]
    fn test_polynomial_x_plus_y_squared() {
        // (x + y)^2 = x^2 + 2xy + y^2
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let sum = &x + &y;
        let sq = &sum * &sum;
        assert_eq!(sq.degree_in("x"), 2);
        assert_eq!(sq.degree_in("y"), 2);
        assert_eq!(sq.total_degree(), 2);
    }

    #[test]
    fn test_partial_derivative_x() {
        // d/dx(x^2 + xy + y^2) = 2x + y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2 = &x * &x;
        let xy = &x * &y;
        let y2 = &y * &y;
        let poly = &(&x2 + &xy) + &y2;
        let dx = poly.partial_derivative("x");
        assert_eq!(format!("{}", dx), "2x + y");
    }

    #[test]
    fn test_partial_derivative_y() {
        // d/dy(x^2 + xy + y^2) = x + 2y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2 = &x * &x;
        let xy = &x * &y;
        let y2 = &y * &y;
        let poly = &(&x2 + &xy) + &y2;
        let dy = poly.partial_derivative("y");
        assert_eq!(format!("{}", dy), "x + 2y");
    }

    #[test]
    fn test_evaluate_at() {
        // p(x,y) = x + y, evaluate at x=3 → 3 + y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let poly = &x + &y;
        let result = poly.evaluate_at("x", &int(3));
        assert_eq!(format!("{}", result), "y + 3");
    }

    #[test]
    fn test_evaluate_fully() {
        // p(x,y) = x^2 + 2xy + y^2, evaluate at x=1, y=2 → 9
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let sum = &x + &y;
        let sq = &sum * &sum;
        let r1 = sq.evaluate_at("x", &int(1));
        let r2 = r1.evaluate_at("y", &int(2));
        assert_eq!(r2, MultiPoly::integer(9));
    }

    #[test]
    fn test_from_node_multivariate() {
        // x*y + 1
        let node = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Variable("y".to_string())),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let poly = MultiPoly::from_node(&node).unwrap();
        assert_eq!(poly.degree_in("x"), 1);
        assert_eq!(poly.degree_in("y"), 1);
        assert_eq!(poly.variables(), vec!["x", "y"]);
    }

    #[test]
    fn test_substitute() {
        // p(x,y) = x + y, substitute y → x^2, get x + x^2 = x^2 + x
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let poly = &x + &y;
        let x_sq = &x * &x;
        let result = poly.substitute("y", &x_sq);
        assert_eq!(result.degree_in("x"), 2);
        assert_eq!(result.variables(), vec!["x"]);
    }

    #[test]
    fn test_zero_handling() {
        let x = MultiPoly::variable("x");
        let neg_x = -&x;
        let zero = &x + &neg_x;
        assert!(zero.is_zero());
    }

    #[test]
    fn test_three_variables() {
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let z = MultiPoly::variable("z");
        let poly = &(&x + &y) + &z;
        assert_eq!(poly.variables(), vec!["x", "y", "z"]);
        assert_eq!(poly.total_degree(), 1);
    }

    #[test]
    fn test_scalar_multiplication() {
        let x = MultiPoly::variable("x");
        let three_x = x.scalar_mul(&int(3));
        assert_eq!(format!("{}", three_x), "3x");
    }

    #[test]
    fn test_negation() {
        let x = MultiPoly::variable("x");
        let neg = -&x;
        assert_eq!(format!("{}", neg), "-x");
    }

    #[test]
    fn test_degree_in_absent_variable() {
        let x = MultiPoly::variable("x");
        assert_eq!(x.degree_in("y"), 0);
    }

    #[test]
    fn test_leading_coeff() {
        // 3x^2 + 2x + 1 → leading coeff = 3
        let x = MultiPoly::variable("x");
        let x2 = &x * &x;
        let poly = &(&x2.scalar_mul(&int(3)) + &x.scalar_mul(&int(2))) + &MultiPoly::integer(1);
        assert_eq!(poly.leading_coeff(), MultiPoly::integer(3));
    }

    #[test]
    fn test_bivariate_expansion() {
        // (x + y)(x - y) = x^2 - y^2
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let sum = &x + &y;
        let diff = &x - &y;
        let prod = &sum * &diff;
        assert_eq!(prod.degree_in("x"), 2);
        assert_eq!(prod.degree_in("y"), 2);
        assert_eq!(prod.total_degree(), 2);
        // Verify: evaluate at x=3, y=2 → 9-4=5
        let r = prod.evaluate_at("x", &int(3)).evaluate_at("y", &int(2));
        assert_eq!(r, MultiPoly::integer(5));
    }

    #[test]
    fn test_x_plus_y_plus_z_squared() {
        // (x + y + z)^2 = x^2 + y^2 + z^2 + 2xy + 2xz + 2yz
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let z = MultiPoly::variable("z");
        let sum = &(&x + &y) + &z;
        let sq = &sum * &sum;
        assert_eq!(sq.total_degree(), 2);
        assert_eq!(sq.degree_in("x"), 2);
        assert_eq!(sq.degree_in("y"), 2);
        assert_eq!(sq.degree_in("z"), 2);
        // Verify: (1+2+3)^2 = 36
        let r = sq
            .evaluate_at("x", &int(1))
            .evaluate_at("y", &int(2))
            .evaluate_at("z", &int(3));
        assert_eq!(r, MultiPoly::integer(36));
    }

    #[test]
    fn test_mixed_degree_polynomial() {
        // x^2*y + x*y^2 + 1
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2y = &(&x * &x) * &y;
        let xy2 = &(&x * &y) * &y;
        let poly = &(&x2y + &xy2) + &MultiPoly::one();
        assert_eq!(poly.total_degree(), 3);
        assert_eq!(poly.degree_in("x"), 2);
        assert_eq!(poly.degree_in("y"), 2);
        // Verify: at x=2, y=3 → 4*3 + 2*9 + 1 = 12 + 18 + 1 = 31
        let r = poly.evaluate_at("x", &int(2)).evaluate_at("y", &int(3));
        assert_eq!(r, MultiPoly::integer(31));
    }

    #[test]
    fn test_partial_derivative_mixed() {
        // d/dx(x^2*y) = 2xy
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2y = &(&x * &x) * &y;
        let dx = x2y.partial_derivative("x");
        // Verify: 2xy at x=3, y=5 → 30
        let r = dx.evaluate_at("x", &int(3)).evaluate_at("y", &int(5));
        assert_eq!(r, MultiPoly::integer(30));
    }

    #[test]
    fn test_second_partial_derivative() {
        // d²/dxdy(x^2*y + x*y^2) = 2x + 2y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2y = &(&x * &x) * &y;
        let xy2 = &(&x * &y) * &y;
        let poly = &x2y + &xy2;
        let dxy = poly.partial_derivative("x").partial_derivative("y");
        // At x=1, y=1 → 4
        let r = dxy.evaluate_at("x", &int(1)).evaluate_at("y", &int(1));
        assert_eq!(r, MultiPoly::integer(4));
    }

    #[test]
    fn test_from_univariate_conversion() {
        let p = Polynomial::from_coeffs(
            vec![int(1), int(-2), int(3)],
            "x",
        );
        let mp = MultiPoly::from_univariate(&p);
        assert_eq!(mp.degree_in("x"), 2);
        assert!(mp.variables() == vec!["x"]);
        // Verify roundtrip: evaluate at x=2 → 3*4 - 2*2 + 1 = 9
        let r = mp.evaluate_at("x", &int(2));
        assert_eq!(r, MultiPoly::integer(9));
    }

    #[test]
    fn test_to_univariate_conversion() {
        let x = MultiPoly::variable("x");
        let x2 = &x * &x;
        let three_x2 = x2.scalar_mul(&int(3));
        let poly = &three_x2 + &MultiPoly::integer(1);
        let uni = poly.to_univariate().unwrap();
        assert_eq!(format!("{}", uni), "3x^2 + 1");
    }

    #[test]
    fn test_to_univariate_fails_multivariate() {
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let poly = &x + &y;
        assert!(poly.to_univariate().is_err());
    }

    #[test]
    fn test_substitute_cross_variable() {
        // p(x,y) = x^2 + y, substitute y → 2x + 1, get x^2 + 2x + 1 = (x+1)^2
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let poly = &(&x * &x) + &y;
        let two_x = x.scalar_mul(&int(2));
        let replacement = &two_x + &MultiPoly::one();
        let result = poly.substitute("y", &replacement);
        assert_eq!(result.degree_in("x"), 2);
        assert_eq!(result.variables(), vec!["x"]);
        // (x+1)^2 at x=2 → 9
        let r = result.evaluate_at("x", &int(2));
        assert_eq!(r, MultiPoly::integer(9));
    }

    #[test]
    fn test_display_compound_coefficient() {
        // x*(y+1) should display with parens: (y + 1)x
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let y_plus_1 = &y + &MultiPoly::one();
        let poly = &x * &y_plus_1;
        assert_eq!(format!("{}", poly), "(y + 1)x");
    }

    // --- GCD tests ---

    #[test]
    fn test_gcd_constants() {
        let a = MultiPoly::integer(6);
        let b = MultiPoly::integer(4);
        let g = MultiPoly::gcd(&a, &b);
        assert_eq!(g, MultiPoly::integer(2));
    }

    #[test]
    fn test_gcd_constant_zero() {
        let a = MultiPoly::zero();
        let b = MultiPoly::integer(5);
        let g = MultiPoly::gcd(&a, &b);
        assert_eq!(g, MultiPoly::integer(5));
    }

    #[test]
    fn test_gcd_univariate_coprime() {
        // gcd(x + 1, x + 2) = 1
        let x = MultiPoly::variable("x");
        let f = &x + &MultiPoly::one();
        let g = &x + &MultiPoly::integer(2);
        let result = MultiPoly::gcd(&f, &g);
        assert_eq!(result, MultiPoly::one());
    }

    #[test]
    fn test_gcd_univariate_common_factor() {
        // gcd(x^2 - 1, x^2 - 2x + 1) = gcd((x-1)(x+1), (x-1)^2) = x - 1
        let x = MultiPoly::variable("x");
        let one = MultiPoly::one();
        let f = &(&x * &x) - &one; // x^2 - 1
        let x_minus_1 = &x - &one;
        let g = &x_minus_1 * &x_minus_1; // (x-1)^2
        let result = MultiPoly::gcd(&f, &g);
        assert_eq!(format!("{}", result), "x - 1");
    }

    #[test]
    fn test_gcd_bivariate_common_factor() {
        // f = x(y+1), g = (y+1) → gcd = y+1
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let y1 = &y + &MultiPoly::one();
        let f = &x * &y1; // x(y+1)
        let g = y1.clone(); // y+1
        let result = MultiPoly::gcd(&f, &g);
        assert_eq!(format!("{}", result), "y + 1");
    }

    #[test]
    fn test_gcd_bivariate_xy() {
        // f = x^2*y + x*y = xy(x+1), g = x*y + y = y(x+1) → gcd = y(x+1) = xy + y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let x2y = &(&x * &x) * &y;
        let xy = &x * &y;
        let f = &x2y + &xy; // x²y + xy
        let g = &xy + &y; // xy + y
        let result = MultiPoly::gcd(&f, &g);
        // gcd = y(x+1) = xy + y
        let r_val = result
            .evaluate_at("x", &int(3))
            .evaluate_at("y", &int(5));
        // y(x+1) at x=3,y=5 → 5*4 = 20
        assert_eq!(r_val, MultiPoly::integer(20));
    }

    #[test]
    fn test_gcd_bivariate_coprime() {
        // gcd(x + 1, y + 1) = 1 (no shared variable in a common factor)
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let f = &x + &MultiPoly::one();
        let g = &y + &MultiPoly::one();
        let result = MultiPoly::gcd(&f, &g);
        assert_eq!(result, MultiPoly::one());
    }

    #[test]
    fn test_content_constant_coefficients() {
        // 6x^2 + 4x + 2 → content = 2
        let x = MultiPoly::variable("x");
        let x2 = &x * &x;
        let poly = &(&x2.scalar_mul(&int(6)) + &x.scalar_mul(&int(4)))
            + &MultiPoly::integer(2);
        let c = poly.content();
        assert_eq!(c, MultiPoly::integer(2));
    }

    #[test]
    fn test_primitive_part() {
        // 6x^2 + 4x + 2 → primitive part = 3x^2 + 2x + 1
        let x = MultiPoly::variable("x");
        let x2 = &x * &x;
        let poly = &(&x2.scalar_mul(&int(6)) + &x.scalar_mul(&int(4)))
            + &MultiPoly::integer(2);
        let pp = poly.primitive_part();
        assert_eq!(format!("{}", pp), "3x^2 + 2x + 1");
    }

    #[test]
    fn test_content_polynomial_coefficients() {
        // (2y+2)x + (4y+4) → content = 2(y+1) = 2y+2
        let _x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let two_y2 = &y.scalar_mul(&int(2)) + &MultiPoly::integer(2); // 2y+2
        let four_y4 = &y.scalar_mul(&int(4)) + &MultiPoly::integer(4); // 4y+4
        let poly = &MultiPoly::monomial(two_y2, "x", 1) + &four_y4;
        let c = poly.content();
        // content should be 2y+2 = 2(y+1)
        assert_eq!(
            c.evaluate_at("y", &int(3)),
            MultiPoly::integer(8) // 2*3+2 = 8
        );
    }

    #[test]
    fn test_pseudo_remainder() {
        // f = x^2 + x + 1, g = yx + 1 (in Q[y][x])
        // prem(f, g) = y^2 - y + 1 (worked by hand above)
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let one = MultiPoly::one();
        let f = &(&(&x * &x) + &x) + &one; // x^2 + x + 1
        let g = &(&y * &x) + &one; // yx + 1
        let r = f.pseudo_remainder(&g);
        // r should be y^2 - y + 1
        let r_at_3 = r.evaluate_at("y", &int(3));
        assert_eq!(r_at_3, MultiPoly::integer(7)); // 9 - 3 + 1 = 7
    }

    #[test]
    fn test_exact_div_scalar() {
        // (6x + 4) / 2 = 3x + 2
        let x = MultiPoly::variable("x");
        let poly = &x.scalar_mul(&int(6)) + &MultiPoly::integer(4);
        let result = poly.exact_div(&MultiPoly::integer(2));
        assert_eq!(format!("{}", result), "3x + 2");
    }

    #[test]
    fn test_exact_div_polynomial() {
        // (x^2 - 1) / (x + 1) = x - 1
        let x = MultiPoly::variable("x");
        let f = &(&x * &x) - &MultiPoly::one(); // x^2 - 1
        let d = &x + &MultiPoly::one(); // x + 1
        let result = f.exact_div(&d);
        assert_eq!(format!("{}", result), "x - 1");
    }

    #[test]
    fn test_exact_div_by_content() {
        // (y+1)x + (y^2-1) divided by (y+1)
        // = x + (y-1)
        let y = MultiPoly::variable("y");
        let y1 = &y + &MultiPoly::one();
        let y2m1 = &(&y * &y) - &MultiPoly::one(); // y^2 - 1 = (y+1)(y-1)
        let poly = &MultiPoly::monomial(y1.clone(), "x", 1) + &y2m1;
        let result = poly.exact_div(&y1);
        // x + (y - 1)
        let val = result
            .evaluate_at("x", &int(5))
            .evaluate_at("y", &int(3));
        assert_eq!(val, MultiPoly::integer(7)); // 5 + (3-1) = 7
    }

    #[test]
    fn test_gcd_with_rational_content() {
        // gcd(6x + 6, 4x + 4) = 2(x + 1) = 2x + 2
        let x = MultiPoly::variable("x");
        let f = &x.scalar_mul(&int(6)) + &MultiPoly::integer(6); // 6(x+1)
        let g = &x.scalar_mul(&int(4)) + &MultiPoly::integer(4); // 4(x+1)
        let result = MultiPoly::gcd(&f, &g);
        // gcd = 2(x+1) = 2x+2
        let val = result.evaluate_at("x", &int(3));
        assert_eq!(val, MultiPoly::integer(8)); // 2*4 = 8
    }

    #[test]
    fn test_gcd_self() {
        // gcd(f, f) = f (normalized)
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let f = &(&x * &y) + &MultiPoly::one(); // xy + 1
        let result = MultiPoly::gcd(&f, &f);
        assert_eq!(result, f);
    }

    #[test]
    fn test_gcd_difference_of_squares() {
        // gcd(x^2 - y^2, x + y) = x + y since x^2-y^2 = (x+y)(x-y)
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let f = &(&x * &x) - &(&y * &y); // x^2 - y^2
        let g = &x + &y; // x + y
        let result = MultiPoly::gcd(&f, &g);
        // Verify at x=5, y=3: (5+3) = 8
        let val = result
            .evaluate_at("x", &int(5))
            .evaluate_at("y", &int(3));
        assert_eq!(val, MultiPoly::integer(8));
    }

    #[test]
    fn test_gcd_quadratic_common_factor() {
        // f = (x+y)^2 * (x-1) = (x^2+2xy+y^2)(x-1)
        // g = (x+y) * (x+2)
        // gcd = x + y
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let one = MultiPoly::one();
        let xpy = &x + &y;
        let xpy_sq = &xpy * &xpy;
        let xm1 = &x - &one;
        let xp2 = &x + &MultiPoly::integer(2);
        let f = &xpy_sq * &xm1;
        let g = &xpy * &xp2;
        let result = MultiPoly::gcd(&f, &g);
        // x+y at x=3,y=7 → 10
        let val = result
            .evaluate_at("x", &int(3))
            .evaluate_at("y", &int(7));
        assert_eq!(val, MultiPoly::integer(10));
    }

    #[test]
    fn test_gcd_three_variables() {
        // f = xz + yz = z(x+y)
        // g = xz + z = z(x+1)
        // gcd = z
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let z = MultiPoly::variable("z");
        let f = &(&x * &z) + &(&y * &z); // z(x+y)
        let g = &(&x * &z) + &z; // z(x+1)
        let result = MultiPoly::gcd(&f, &g);
        // gcd should be z
        assert_eq!(result.variables(), vec!["z"]);
        assert_eq!(result.degree_in("z"), 1);
        let val = result.evaluate_at("z", &int(7));
        assert_eq!(val, MultiPoly::integer(7));
    }

    #[test]
    fn test_gcd_higher_degree() {
        // f = x^3 - x = x(x-1)(x+1)
        // g = x^2 - 1 = (x-1)(x+1)
        // gcd = x^2 - 1
        let x = MultiPoly::variable("x");
        let one = MultiPoly::one();
        let x3 = &(&x * &x) * &x;
        let f = &x3 - &x; // x^3 - x
        let g = &(&x * &x) - &one; // x^2 - 1
        let result = MultiPoly::gcd(&f, &g);
        // x^2 - 1 at x=3 → 8
        let val = result.evaluate_at("x", &int(3));
        assert_eq!(val, MultiPoly::integer(8));
        assert_eq!(result.main_degree(), 2);
    }

    #[test]
    fn test_gcd_bivariate_quadratic() {
        // f = x^2*y^2 - 1 = (xy-1)(xy+1)
        // g = x*y - 1
        // gcd = xy - 1
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let one = MultiPoly::one();
        let xy = &x * &y;
        let xy_sq = &xy * &xy; // x^2*y^2
        let f = &xy_sq - &one;
        let g = &xy - &one;
        let result = MultiPoly::gcd(&f, &g);
        // xy - 1 at x=3, y=4 → 11
        let val = result
            .evaluate_at("x", &int(3))
            .evaluate_at("y", &int(4));
        assert_eq!(val, MultiPoly::integer(11));
    }

    #[test]
    fn test_gcd_content_with_polynomial_gcd() {
        // f = 2(y+1)x + 2(y+1) = 2(y+1)(x+1)
        // g = 3(y+1)x + 6(y+1) = 3(y+1)(x+2)
        // gcd = (y+1)(x+1) ... wait, gcd of (x+1) and (x+2) = 1
        // so gcd = (y+1)
        let x = MultiPoly::variable("x");
        let y = MultiPoly::variable("y");
        let y1 = &y + &MultiPoly::one();
        let f = &(&(&x + &MultiPoly::one()) * &y1).scalar_mul(&int(2));
        let g = &(&(&x + &MultiPoly::integer(2)) * &y1).scalar_mul(&int(3));
        let result = MultiPoly::gcd(&f, &g);
        assert_eq!(
            result.evaluate_at("y", &int(4)),
            MultiPoly::integer(5) // y+1 at y=4 → 5
        );
    }
}
