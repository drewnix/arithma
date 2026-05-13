use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExactNum {
    Rational(BigRational),
    Float(f64),
}

impl ExactNum {
    pub fn integer(n: i64) -> Self {
        ExactNum::Rational(BigRational::from_integer(BigInt::from(n)))
    }

    pub fn rational(numer: i64, denom: i64) -> Self {
        if denom == 0 {
            return ExactNum::Float(f64::NAN);
        }
        ExactNum::Rational(BigRational::new(BigInt::from(numer), BigInt::from(denom)))
    }

    pub fn from_f64(v: f64) -> Self {
        if v.fract() == 0.0 && v.is_finite() && v.abs() < i64::MAX as f64 {
            ExactNum::integer(v as i64)
        } else {
            ExactNum::Float(v)
        }
    }

    pub fn zero() -> Self {
        ExactNum::integer(0)
    }

    pub fn one() -> Self {
        ExactNum::integer(1)
    }

    pub fn two() -> Self {
        ExactNum::integer(2)
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            ExactNum::Rational(r) => r.to_f64().unwrap_or(f64::NAN),
            ExactNum::Float(f) => *f,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            ExactNum::Rational(r) => r.is_zero(),
            ExactNum::Float(f) => *f == 0.0,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            ExactNum::Rational(r) => r.is_one(),
            ExactNum::Float(f) => *f == 1.0,
        }
    }

    pub fn is_integer(&self) -> bool {
        match self {
            ExactNum::Rational(r) => r.is_integer(),
            ExactNum::Float(f) => f.fract() == 0.0 && f.is_finite(),
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            ExactNum::Rational(r) => r.is_negative(),
            ExactNum::Float(f) => *f < 0.0,
        }
    }

    pub fn abs(&self) -> Self {
        match self {
            ExactNum::Rational(r) => ExactNum::Rational(r.abs()),
            ExactNum::Float(f) => ExactNum::Float(f.abs()),
        }
    }

    pub fn pow_f64(&self, exp: f64) -> Self {
        ExactNum::Float(self.to_f64().powf(exp))
    }

    pub fn powf(&self, exp: &ExactNum) -> Self {
        if let (ExactNum::Rational(base), Some(e)) = (self, exp.to_i64()) {
            if e >= 0 {
                return ExactNum::Rational(num_traits::pow::pow(base.clone(), e as usize));
            } else if !base.is_zero() {
                let inv = BigRational::one() / base;
                return ExactNum::Rational(num_traits::pow::pow(inv, (-e) as usize));
            }
        }
        ExactNum::Float(self.to_f64().powf(exp.to_f64()))
    }

    pub fn sqrt(&self) -> Self {
        if let ExactNum::Rational(r) = self {
            if !r.is_negative() {
                let n = r.numer().to_i64();
                let d = r.denom().to_i64();
                if let (Some(n), Some(d)) = (n, d) {
                    let nu = n.unsigned_abs();
                    let du = d.unsigned_abs();
                    let sn = (nu as f64).sqrt() as u64;
                    let sd = (du as f64).sqrt() as u64;
                    if sn * sn == nu && sd * sd == du {
                        return ExactNum::Rational(BigRational::new(
                            BigInt::from(sn),
                            BigInt::from(sd),
                        ));
                    }
                }
            }
        }
        ExactNum::Float(self.to_f64().sqrt())
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self {
            ExactNum::Rational(r) if r.is_integer() => r.numer().to_i64(),
            ExactNum::Float(f) if f.fract() == 0.0 && f.is_finite() => Some(*f as i64),
            _ => None,
        }
    }
}

impl Add for ExactNum {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => ExactNum::Rational(a + b),
            _ => ExactNum::Float(self.to_f64() + rhs.to_f64()),
        }
    }
}

impl Sub for ExactNum {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => ExactNum::Rational(a - b),
            _ => ExactNum::Float(self.to_f64() - rhs.to_f64()),
        }
    }
}

impl Mul for ExactNum {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => ExactNum::Rational(a * b),
            _ => ExactNum::Float(self.to_f64() * rhs.to_f64()),
        }
    }
}

impl Div for ExactNum {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        if rhs.is_zero() {
            return ExactNum::Float(f64::NAN);
        }
        match (&self, &rhs) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => ExactNum::Rational(a / b),
            _ => ExactNum::Float(self.to_f64() / rhs.to_f64()),
        }
    }
}

impl Neg for ExactNum {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            ExactNum::Rational(r) => ExactNum::Rational(-r),
            ExactNum::Float(f) => ExactNum::Float(-f),
        }
    }
}

impl Add for &ExactNum {
    type Output = ExactNum;
    fn add(self, rhs: Self) -> ExactNum {
        self.clone() + rhs.clone()
    }
}

impl Sub for &ExactNum {
    type Output = ExactNum;
    fn sub(self, rhs: Self) -> ExactNum {
        self.clone() - rhs.clone()
    }
}

impl Mul for &ExactNum {
    type Output = ExactNum;
    fn mul(self, rhs: Self) -> ExactNum {
        self.clone() * rhs.clone()
    }
}

impl Div for &ExactNum {
    type Output = ExactNum;
    fn div(self, rhs: Self) -> ExactNum {
        self.clone() / rhs.clone()
    }
}

impl PartialEq for ExactNum {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => a == b,
            _ => self.to_f64() == other.to_f64(),
        }
    }
}

impl PartialEq<f64> for ExactNum {
    fn eq(&self, other: &f64) -> bool {
        self.to_f64() == *other
    }
}

impl PartialOrd for ExactNum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (ExactNum::Rational(a), ExactNum::Rational(b)) => a.partial_cmp(b),
            _ => self.to_f64().partial_cmp(&other.to_f64()),
        }
    }
}

impl fmt::Display for ExactNum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExactNum::Rational(r) => {
                if r.is_integer() {
                    write!(f, "{}", r.numer())
                } else {
                    write!(f, "\\frac{{{}}}{{{}}}", r.numer(), r.denom())
                }
            }
            ExactNum::Float(v) => write!(f, "{}", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_rational_arithmetic() {
        let a = ExactNum::rational(1, 3);
        let b = ExactNum::rational(1, 3);
        let c = ExactNum::rational(1, 3);
        let sum = a + b + c;
        assert!(
            sum.is_one(),
            "1/3 + 1/3 + 1/3 should be exactly 1, got {}",
            sum
        );
    }

    #[test]
    fn test_exact_where_f64_fails() {
        let a = ExactNum::rational(1, 10);
        let b = ExactNum::rational(2, 10);
        let sum = a + b;
        assert_eq!(sum, ExactNum::rational(3, 10), "1/10 + 2/10 = 3/10 exactly");

        let f_sum: f64 = 0.1 + 0.2;
        assert_ne!(f_sum, 0.3, "f64 0.1 + 0.2 != 0.3 (it's {})", f_sum);
    }

    #[test]
    fn test_exact_rational_chain() {
        let seventh = ExactNum::rational(1, 7);
        let mut sum = ExactNum::zero();
        for _ in 0..7 {
            sum = sum + seventh.clone();
        }
        assert!(sum.is_one(), "7 * (1/7) should be exactly 1, got {}", sum);
    }

    #[test]
    fn test_integer_promotion() {
        let five = ExactNum::from_f64(5.0);
        assert!(
            matches!(five, ExactNum::Rational(_)),
            "5.0 should be promoted to exact rational"
        );
        assert_eq!(five.to_f64(), 5.0);
    }

    #[test]
    fn test_irrational_stays_float() {
        let pi = ExactNum::Float(std::f64::consts::PI);
        assert!(matches!(pi, ExactNum::Float(_)), "pi should stay as float");
    }

    #[test]
    fn test_display_rational() {
        assert_eq!(format!("{}", ExactNum::integer(42)), "42");
        assert_eq!(format!("{}", ExactNum::rational(1, 3)), "\\frac{1}{3}");
        assert_eq!(format!("{}", ExactNum::rational(2, 4)), "\\frac{1}{2}");
    }

    #[test]
    fn test_frac_parsed_exactly() {
        let env = crate::environment::Environment::new();
        let expr = crate::parser::parse_latex("\\frac{1}{3}", &env).unwrap();
        match &expr {
            crate::node::Node::Num(n) => {
                assert!(
                    matches!(n, ExactNum::Rational(_)),
                    "\\frac{{1}}{{3}} should be exact rational"
                );
                assert_eq!(format!("{}", n), "\\frac{1}{3}");
            }
            other => panic!("Expected Node::Num, got {:?}", other),
        }
    }

    #[test]
    fn test_frac_addition_exact() {
        let mut tokenizer = crate::tokenizer::Tokenizer::new("1 + \\frac{1}{3}");
        let tokens = tokenizer.tokenize();
        let expr = crate::parser::build_expression_tree(tokens).unwrap();
        let env = crate::environment::Environment::new();
        let result = crate::evaluator::Evaluator::evaluate(&expr, &env).unwrap();
        let expected = 4.0 / 3.0;
        assert!(
            (result - expected).abs() < 1e-15,
            "1 + 1/3 should be 4/3 = {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn test_evaluate_exact_preserves_rationals() {
        let env = crate::environment::Environment::new();

        // 1/3 + 1/6 = 1/2 exactly (via evaluate_exact)
        let expr = crate::node::Node::Add(
            Box::new(crate::node::Node::Num(ExactNum::rational(1, 3))),
            Box::new(crate::node::Node::Num(ExactNum::rational(1, 6))),
        );
        let result = crate::evaluator::Evaluator::evaluate_exact(&expr, &env).unwrap();
        assert_eq!(
            result,
            ExactNum::rational(1, 2),
            "1/3 + 1/6 should be exactly 1/2, got {}",
            result
        );
        assert!(
            matches!(result, ExactNum::Rational(_)),
            "result should stay rational"
        );
    }

    #[test]
    fn test_evaluate_exact_multiplication() {
        let env = crate::environment::Environment::new();

        // 2/3 * 3/4 = 1/2 exactly
        let expr = crate::node::Node::Multiply(
            Box::new(crate::node::Node::Num(ExactNum::rational(2, 3))),
            Box::new(crate::node::Node::Num(ExactNum::rational(3, 4))),
        );
        let result = crate::evaluator::Evaluator::evaluate_exact(&expr, &env).unwrap();
        assert_eq!(
            result,
            ExactNum::rational(1, 2),
            "2/3 * 3/4 should be exactly 1/2, got {}",
            result
        );
    }

    #[test]
    fn test_sqrt_exact_perfect_square() {
        let result = ExactNum::integer(9).sqrt();
        assert_eq!(result, ExactNum::integer(3), "sqrt(9) = 3 exactly");
        assert!(matches!(result, ExactNum::Rational(_)));
    }

    #[test]
    fn test_sqrt_exact_rational_perfect_square() {
        let result = ExactNum::rational(9, 4).sqrt();
        assert_eq!(result, ExactNum::rational(3, 2), "sqrt(9/4) = 3/2 exactly");
        assert!(matches!(result, ExactNum::Rational(_)));
    }

    #[test]
    fn test_sqrt_non_perfect_stays_float() {
        let result = ExactNum::integer(2).sqrt();
        assert!(matches!(result, ExactNum::Float(_)), "sqrt(2) should be float");
    }

    #[test]
    fn test_powf_exact_integer_exponent() {
        let base = ExactNum::rational(2, 3);
        let exp = ExactNum::integer(2);
        let result = base.powf(&exp);
        assert_eq!(result, ExactNum::rational(4, 9), "(2/3)^2 = 4/9 exactly");
        assert!(matches!(result, ExactNum::Rational(_)));
    }

    #[test]
    fn test_powf_exact_negative_exponent() {
        let base = ExactNum::rational(2, 3);
        let exp = ExactNum::integer(-2);
        let result = base.powf(&exp);
        assert_eq!(result, ExactNum::rational(9, 4), "(2/3)^(-2) = 9/4 exactly");
        assert!(matches!(result, ExactNum::Rational(_)));
    }

    #[test]
    fn test_powf_exact_integer_base() {
        let result = ExactNum::integer(3).powf(&ExactNum::integer(4));
        assert_eq!(result, ExactNum::integer(81), "3^4 = 81 exactly");
        assert!(matches!(result, ExactNum::Rational(_)));
    }

    #[test]
    fn test_evaluate_exact_summation() {
        let env = crate::environment::Environment::new();

        // sum_{i=1}^{4} i = 10 exactly
        let expr = crate::node::Node::Summation(
            "i".to_string(),
            Box::new(crate::node::Node::Num(ExactNum::one())),
            Box::new(crate::node::Node::Num(ExactNum::integer(4))),
            Box::new(crate::node::Node::Variable("i".to_string())),
        );
        let result = crate::evaluator::Evaluator::evaluate_exact(&expr, &env).unwrap();
        assert_eq!(
            result,
            ExactNum::integer(10),
            "sum 1..4 of i should be 10, got {}",
            result
        );
        assert!(
            matches!(result, ExactNum::Rational(_)),
            "integer sum should stay rational"
        );
    }

    #[test]
    fn test_environment_exact_variable() {
        let mut env = crate::environment::Environment::new();
        env.set_exact("x", ExactNum::rational(1, 3));

        let expr = crate::node::Node::Add(
            Box::new(crate::node::Node::Variable("x".to_string())),
            Box::new(crate::node::Node::Variable("x".to_string())),
        );
        let result = crate::evaluator::Evaluator::evaluate_exact(&expr, &env).unwrap();
        assert_eq!(
            result,
            ExactNum::rational(2, 3),
            "1/3 + 1/3 should be exactly 2/3 via exact env, got {}",
            result
        );
        assert!(matches!(result, ExactNum::Rational(_)));
    }
}
