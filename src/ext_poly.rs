use crate::rational_function::RationalFunction;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

/// Polynomial in a tower variable θ, with coefficients in Q(x).
/// coeffs[i] is the coefficient of θ^i. Trailing zeros are stripped.
#[derive(Debug, Clone)]
pub struct ExtPoly {
    coeffs: Vec<RationalFunction>,
    var: String, // base variable for the RationalFunction coefficients
}

impl ExtPoly {
    /// The zero extended polynomial (empty coefficient vector).
    pub fn zero(var: &str) -> Self {
        ExtPoly {
            coeffs: vec![],
            var: var.to_string(),
        }
    }

    /// The constant polynomial 1 (degree 0).
    pub fn one(var: &str) -> Self {
        ExtPoly {
            coeffs: vec![RationalFunction::one(var)],
            var: var.to_string(),
        }
    }

    /// Wrap a rational function as a constant polynomial (degree 0 if nonzero).
    pub fn from_rf(rf: RationalFunction) -> Self {
        let var = rf.variable().to_string();
        if rf.is_zero() {
            Self::zero(&var)
        } else {
            ExtPoly {
                coeffs: vec![rf],
                var,
            }
        }
    }

    /// The identity polynomial θ: coeffs = [0, 1].
    pub fn theta(var: &str) -> Self {
        ExtPoly {
            coeffs: vec![RationalFunction::zero(var), RationalFunction::one(var)],
            var: var.to_string(),
        }
    }

    /// Build from a vector of coefficients, stripping trailing zero rational functions.
    pub fn from_coeffs(mut coeffs: Vec<RationalFunction>, var: &str) -> Self {
        while coeffs.last().is_some_and(|c| c.is_zero()) {
            coeffs.pop();
        }
        ExtPoly {
            coeffs,
            var: var.to_string(),
        }
    }

    /// Degree of the polynomial. `None` for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        if self.coeffs.is_empty() {
            None
        } else {
            Some(self.coeffs.len() - 1)
        }
    }

    /// True if this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// The leading (highest-degree) coefficient, or `None` for the zero polynomial.
    pub fn leading_coeff(&self) -> Option<&RationalFunction> {
        self.coeffs.last()
    }

    /// Get the coefficient of θ^i. Returns zero if `i` is out of range.
    pub fn coeff(&self, i: usize) -> RationalFunction {
        self.coeffs
            .get(i)
            .cloned()
            .unwrap_or_else(|| RationalFunction::zero(&self.var))
    }

    /// The base variable name used by the rational function coefficients.
    pub fn variable(&self) -> &str {
        &self.var
    }

    /// Multiply every coefficient by a rational function scalar.
    pub fn scalar_mul(&self, s: &RationalFunction) -> Self {
        if s.is_zero() {
            return Self::zero(&self.var);
        }
        let coeffs = self.coeffs.iter().map(|c| c * s).collect();
        ExtPoly::from_coeffs(coeffs, &self.var)
    }
}

// --- Operator implementations ---

impl<'a> Add for &'a ExtPoly {
    type Output = ExtPoly;
    fn add(self, rhs: &'a ExtPoly) -> ExtPoly {
        let len = self.coeffs.len().max(rhs.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = rhs.coeff(i);
            coeffs.push(&a + &b);
        }
        ExtPoly::from_coeffs(coeffs, &self.var)
    }
}

impl<'a> Sub for &'a ExtPoly {
    type Output = ExtPoly;
    fn sub(self, rhs: &'a ExtPoly) -> ExtPoly {
        let len = self.coeffs.len().max(rhs.coeffs.len());
        let mut coeffs = Vec::with_capacity(len);
        for i in 0..len {
            let a = self.coeff(i);
            let b = rhs.coeff(i);
            coeffs.push(&a - &b);
        }
        ExtPoly::from_coeffs(coeffs, &self.var)
    }
}

impl<'a> Mul for &'a ExtPoly {
    type Output = ExtPoly;
    fn mul(self, rhs: &'a ExtPoly) -> ExtPoly {
        if self.is_zero() || rhs.is_zero() {
            return ExtPoly::zero(&self.var);
        }
        let len = self.coeffs.len() + rhs.coeffs.len() - 1;
        let mut coeffs: Vec<RationalFunction> = (0..len)
            .map(|_| RationalFunction::zero(&self.var))
            .collect();
        for (i, a) in self.coeffs.iter().enumerate() {
            if a.is_zero() {
                continue;
            }
            for (j, b) in rhs.coeffs.iter().enumerate() {
                coeffs[i + j] = &coeffs[i + j] + &(a * b);
            }
        }
        ExtPoly::from_coeffs(coeffs, &self.var)
    }
}

impl Neg for &ExtPoly {
    type Output = ExtPoly;
    fn neg(self) -> ExtPoly {
        let coeffs = self.coeffs.iter().map(|c| -c).collect();
        ExtPoly::from_coeffs(coeffs, &self.var)
    }
}

impl PartialEq for ExtPoly {
    fn eq(&self, other: &Self) -> bool {
        self.coeffs == other.coeffs
    }
}

impl fmt::Display for ExtPoly {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }

        let one = RationalFunction::one(&self.var);
        let mut first = true;

        for (i, coeff) in self.coeffs.iter().enumerate().rev() {
            if coeff.is_zero() {
                continue;
            }

            // Determine if the coefficient is "negative" — we check if the
            // negation equals a positive-looking form. For rational functions
            // where the numerator has a negative leading coefficient, we treat
            // it as negative.
            let neg_coeff = -coeff;
            let is_negative = is_negative_rf(coeff);

            let abs_coeff = if is_negative {
                neg_coeff.clone()
            } else {
                coeff.clone()
            };

            // Write sign
            if !first {
                if is_negative {
                    write!(f, " - ")?;
                } else {
                    write!(f, " + ")?;
                }
            } else if is_negative {
                write!(f, "-")?;
            }

            if i == 0 {
                // Constant term: always show the coefficient
                write!(f, "{abs_coeff}")?;
            } else {
                // Non-constant term: omit coefficient if it's 1
                let is_one = abs_coeff == one;

                if !is_one {
                    let coeff_str = format!("{abs_coeff}");
                    // If the coefficient contains '+' or '-' (i.e. it's a sum), wrap in parens
                    let needs_parens = coeff_str.contains(" + ") || coeff_str.contains(" - ");
                    if needs_parens {
                        write!(f, "({coeff_str})")?;
                    } else {
                        write!(f, "{coeff_str}")?;
                    }
                }

                // Write the θ part
                write!(f, "θ")?;
                if i > 1 {
                    write!(f, "^{i}")?;
                }
            }

            first = false;
        }

        Ok(())
    }
}

/// Heuristic: a rational function is "negative" if its displayed form
/// starts with '-'.
fn is_negative_rf(rf: &RationalFunction) -> bool {
    let s = format!("{rf}");
    s.starts_with('-') || s.starts_with("(-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polynomial::Polynomial;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn poly(coeffs: &[i64], var: &str) -> Polynomial {
        Polynomial::from_coeffs(coeffs.iter().map(|&c| int(c)).collect(), var)
    }

    fn rf_const(n: i64) -> RationalFunction {
        RationalFunction::from_constant(int(n), "x")
    }

    fn rf_poly(coeffs: &[i64]) -> RationalFunction {
        RationalFunction::from_poly(poly(coeffs, "x"))
    }

    #[test]
    fn test_ext_poly_zero() {
        let z = ExtPoly::zero("x");
        assert!(z.is_zero());
        assert_eq!(z.degree(), None);
        assert_eq!(format!("{z}"), "0");
    }

    #[test]
    fn test_ext_poly_one() {
        let one = ExtPoly::one("x");
        assert!(!one.is_zero());
        assert_eq!(one.degree(), Some(0));
        assert_eq!(one.coeff(0), RationalFunction::one("x"));
    }

    #[test]
    fn test_ext_poly_constant() {
        let c = ExtPoly::from_rf(rf_const(5));
        assert!(!c.is_zero());
        assert_eq!(c.degree(), Some(0));
        assert_eq!(c.coeff(0), rf_const(5));
        assert_eq!(c.coeff(1), RationalFunction::zero("x"));
    }

    #[test]
    fn test_ext_poly_theta() {
        let th = ExtPoly::theta("x");
        assert!(!th.is_zero());
        assert_eq!(th.degree(), Some(1));
        assert_eq!(th.coeff(0), RationalFunction::zero("x"));
        assert_eq!(th.coeff(1), RationalFunction::one("x"));
    }

    #[test]
    fn test_ext_poly_from_coeffs() {
        let ep = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2), rf_const(3)], "x");
        assert_eq!(ep.degree(), Some(2));
        assert_eq!(ep.coeff(0), rf_const(1));
        assert_eq!(ep.coeff(1), rf_const(2));
        assert_eq!(ep.coeff(2), rf_const(3));
        assert_eq!(ep.leading_coeff(), Some(&rf_const(3)));
        assert_eq!(ep.variable(), "x");
    }

    #[test]
    fn test_ext_poly_from_coeffs_strips_trailing() {
        let ep = ExtPoly::from_coeffs(
            vec![
                rf_const(1),
                RationalFunction::zero("x"),
                RationalFunction::zero("x"),
            ],
            "x",
        );
        assert_eq!(ep.degree(), Some(0));
        assert_eq!(ep.coeff(0), rf_const(1));
    }

    #[test]
    fn test_ext_poly_display() {
        // θ alone
        let th = ExtPoly::theta("x");
        assert_eq!(format!("{th}"), "θ");

        // constant
        let c = ExtPoly::from_rf(rf_const(3));
        assert_eq!(format!("{c}"), "3");

        // 2θ^2 + θ + 5
        let ep = ExtPoly::from_coeffs(vec![rf_const(5), rf_const(1), rf_const(2)], "x");
        assert_eq!(format!("{ep}"), "2θ^2 + θ + 5");

        // Polynomial coefficient: (x + 1)θ
        let ep2 = ExtPoly::from_coeffs(vec![RationalFunction::zero("x"), rf_poly(&[1, 1])], "x");
        assert_eq!(format!("{ep2}"), "(x + 1)θ");

        // Negative coefficient: -3θ + 1
        let ep3 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(-3)], "x");
        assert_eq!(format!("{ep3}"), "-3θ + 1");
    }

    #[test]
    fn test_ext_poly_equality() {
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let c = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(3)], "x");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_ext_poly_from_rf_zero() {
        let z = ExtPoly::from_rf(RationalFunction::zero("x"));
        assert!(z.is_zero());
        assert_eq!(z.degree(), None);
    }

    #[test]
    fn test_ext_poly_add() {
        // (2θ + 1) + (3θ + 4) = 5θ + 5
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(4), rf_const(3)], "x");
        let sum = &a + &b;
        assert_eq!(sum.coeff(0), rf_const(5));
        assert_eq!(sum.coeff(1), rf_const(5));
    }

    #[test]
    fn test_ext_poly_add_cancels() {
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(2), rf_const(-1)], "x");
        let sum = &a + &b;
        assert_eq!(sum.degree(), Some(0));
    }

    #[test]
    fn test_ext_poly_sub() {
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let diff = &a - &b;
        assert!(diff.coeff(0).is_zero());
        assert_eq!(diff.coeff(1), rf_const(1));
    }

    #[test]
    fn test_ext_poly_mul() {
        // (θ + 1)(θ - 1) = θ^2 - 1
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(1)], "x");
        let prod = &a * &b;
        assert_eq!(prod.degree(), Some(2));
        assert_eq!(prod.coeff(0), rf_const(-1));
        assert!(prod.coeff(1).is_zero());
        assert_eq!(prod.coeff(2), rf_const(1));
    }

    #[test]
    fn test_ext_poly_mul_with_rf_coeffs() {
        // (xθ)(θ + 1) = xθ^2 + xθ
        let x_rf = rf_poly(&[0, 1]); // x as a rational function
        let a = ExtPoly::from_coeffs(vec![RationalFunction::zero("x"), x_rf.clone()], "x");
        let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let prod = &a * &b;
        assert_eq!(prod.degree(), Some(2));
        assert!(prod.coeff(0).is_zero());
        assert_eq!(prod.coeff(1), x_rf);
        assert_eq!(prod.coeff(2), x_rf);
    }

    #[test]
    fn test_ext_poly_neg() {
        let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let neg_a = -&a;
        assert_eq!(neg_a.coeff(0), rf_const(-1));
        assert_eq!(neg_a.coeff(1), rf_const(-2));
    }

    #[test]
    fn test_ext_poly_scalar_mul() {
        let inv_x = RationalFunction::new(
            Polynomial::from_coeffs(vec![int(1)], "x"),
            Polynomial::from_coeffs(vec![int(0), int(1)], "x"),
        );
        let p = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
        let scaled = p.scalar_mul(&inv_x);
        assert_eq!(scaled.coeff(0), inv_x);
        assert_eq!(scaled.coeff(1), inv_x);
    }
}
