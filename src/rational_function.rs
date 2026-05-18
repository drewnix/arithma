use num_rational::BigRational;
use num_traits::One;
use std::fmt;

use crate::polynomial::Polynomial;

/// A rational function p(x)/q(x) over Q.
///
/// Invariants maintained by `new`:
/// 1. `den` is never zero.
/// 2. `gcd(num, den) = 1` (always fully reduced).
/// 3. `den` is monic (leading coefficient = 1).
/// 4. If `num` is zero, `den` is `Polynomial::one()`.
#[derive(Debug, Clone)]
pub struct RationalFunction {
    num: Polynomial,
    den: Polynomial,
}

impl RationalFunction {
    /// Create a new rational function, normalizing by cancelling the GCD
    /// and making the denominator monic.
    ///
    /// # Panics
    /// Panics if `den` is the zero polynomial.
    pub fn new(num: Polynomial, den: Polynomial) -> Self {
        assert!(
            !den.is_zero(),
            "RationalFunction denominator cannot be zero"
        );

        let var = num.variable().to_string();

        if num.is_zero() {
            return RationalFunction {
                num: Polynomial::zero(&var),
                den: Polynomial::one(&var),
            };
        }

        // Cancel GCD
        let g = num.gcd(&den);
        let (num, _) = num.div_rem(&g).unwrap();
        let (den, _) = den.div_rem(&g).unwrap();

        // Make denominator monic
        let lc = den.leading_coeff().unwrap().clone();
        let inv_lc = BigRational::one() / &lc;
        let num = num.scalar_mul(&inv_lc);
        let den = den.scalar_mul(&inv_lc);

        RationalFunction { num, den }
    }

    /// The zero rational function: 0/1.
    pub fn zero(var: &str) -> Self {
        RationalFunction {
            num: Polynomial::zero(var),
            den: Polynomial::one(var),
        }
    }

    /// The identity rational function: 1/1.
    pub fn one(var: &str) -> Self {
        RationalFunction {
            num: Polynomial::one(var),
            den: Polynomial::one(var),
        }
    }

    /// Wrap a polynomial as a rational function: p/1.
    pub fn from_poly(p: Polynomial) -> Self {
        let var = p.variable().to_string();
        RationalFunction {
            num: p,
            den: Polynomial::one(&var),
        }
    }

    /// A constant rational function: c/1.
    pub fn from_constant(c: BigRational, var: &str) -> Self {
        RationalFunction {
            num: Polynomial::constant(c, var),
            den: Polynomial::one(var),
        }
    }

    /// Reference to the numerator polynomial.
    pub fn numerator(&self) -> &Polynomial {
        &self.num
    }

    /// Reference to the denominator polynomial.
    pub fn denominator(&self) -> &Polynomial {
        &self.den
    }

    /// True if the numerator is zero.
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// True if both numerator and denominator are constant polynomials.
    pub fn is_constant(&self) -> bool {
        self.num.is_constant() && self.den.is_constant()
    }

    /// The variable name used by the underlying polynomials.
    pub fn variable(&self) -> &str {
        self.num.variable()
    }
}

impl PartialEq for RationalFunction {
    fn eq(&self, other: &Self) -> bool {
        self.num == other.num && self.den == other.den
    }
}

impl fmt::Display for RationalFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.den == Polynomial::one(self.num.variable()) {
            write!(f, "{}", self.num)
        } else {
            write!(f, "({})/({})", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn rat(n: i64, d: i64) -> BigRational {
        BigRational::new(BigInt::from(n), BigInt::from(d))
    }

    #[test]
    fn test_rf_normalize_cancels_gcd() {
        // (x^2 - 1) / (x + 1) = (x - 1) / 1
        let num = Polynomial::from_coeffs(vec![int(-1), int(0), int(1)], "x");
        let den = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let rf = RationalFunction::new(num, den);
        assert_eq!(
            rf.numerator(),
            &Polynomial::from_coeffs(vec![int(-1), int(1)], "x")
        );
        assert_eq!(rf.denominator(), &Polynomial::one("x"));
    }

    #[test]
    fn test_rf_zero() {
        let rf = RationalFunction::zero("x");
        assert!(rf.is_zero());
        assert_eq!(rf.numerator(), &Polynomial::zero("x"));
        assert_eq!(rf.denominator(), &Polynomial::one("x"));
    }

    #[test]
    fn test_rf_from_poly() {
        let p = Polynomial::from_coeffs(vec![int(1), int(2), int(3)], "x");
        let rf = RationalFunction::from_poly(p.clone());
        assert_eq!(rf.numerator(), &p);
        assert_eq!(rf.denominator(), &Polynomial::one("x"));
    }

    #[test]
    fn test_rf_from_constant() {
        let rf = RationalFunction::from_constant(rat(3, 4), "x");
        assert!(rf.is_constant());
        assert_eq!(rf.numerator(), &Polynomial::constant(rat(3, 4), "x"));
        assert_eq!(rf.denominator(), &Polynomial::one("x"));
    }

    #[test]
    fn test_rf_display() {
        // Denominator is 1 -> just show numerator
        let rf = RationalFunction::from_poly(Polynomial::from_coeffs(vec![int(1), int(2)], "x"));
        assert_eq!(format!("{}", rf), "2x + 1");

        // Non-trivial denominator
        let num = Polynomial::from_coeffs(vec![int(1), int(1)], "x");
        let den = Polynomial::from_coeffs(vec![int(0), int(0), int(1)], "x");
        let rf = RationalFunction::new(num, den);
        assert_eq!(format!("{}", rf), "(x + 1)/(x^2)");
    }

    #[test]
    fn test_rf_is_constant() {
        let rf = RationalFunction::from_constant(int(5), "x");
        assert!(rf.is_constant());

        let rf = RationalFunction::from_poly(Polynomial::from_coeffs(vec![int(1), int(1)], "x"));
        assert!(!rf.is_constant());
    }

    #[test]
    fn test_rf_eq() {
        // Two different constructions that should normalize to the same thing
        let num1 = Polynomial::from_coeffs(vec![int(-2), int(0), int(2)], "x"); // 2x^2 - 2
        let den1 = Polynomial::from_coeffs(vec![int(2), int(2)], "x"); // 2x + 2
        let rf1 = RationalFunction::new(num1, den1);

        let num2 = Polynomial::from_coeffs(vec![int(-1), int(1)], "x"); // x - 1
        let den2 = Polynomial::one("x");
        let rf2 = RationalFunction::new(num2, den2);

        // 2(x^2-1) / 2(x+1) = (x-1)/1
        assert_eq!(rf1, rf2);
    }
}
