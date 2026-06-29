use std::cell::RefCell;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::rc::Rc;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::polynomial::Polynomial;

#[derive(Clone)]
pub struct FormalPowerSeries {
    inner: Rc<RefCell<FpsInner>>,
}

struct FpsInner {
    cache: Vec<BigRational>,
    gen: FpsGen,
}

enum FpsGen {
    Explicit,
    Closure(Box<dyn Fn(usize) -> BigRational>),
    Sum(FormalPowerSeries, FormalPowerSeries),
    Diff(FormalPowerSeries, FormalPowerSeries),
    Neg(FormalPowerSeries),
    ScalarMul(BigRational, FormalPowerSeries),
    Product(FormalPowerSeries, FormalPowerSeries),
    Inverse(FormalPowerSeries),
    Quotient(FormalPowerSeries, FormalPowerSeries),
}

enum CoeffAction {
    Resolved(BigRational),
    Sum(FormalPowerSeries, FormalPowerSeries),
    Diff(FormalPowerSeries, FormalPowerSeries),
    Neg(FormalPowerSeries),
    ScalarMul(BigRational, FormalPowerSeries),
    Product(FormalPowerSeries, FormalPowerSeries),
    Inverse(FormalPowerSeries),
    Quotient(FormalPowerSeries, FormalPowerSeries),
}

impl FormalPowerSeries {
    pub fn from_coeffs(coeffs: Vec<BigRational>) -> Self {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: coeffs,
                gen: FpsGen::Explicit,
            })),
        }
    }

    pub fn from_fn(f: impl Fn(usize) -> BigRational + 'static) -> Self {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Closure(Box::new(f)),
            })),
        }
    }

    pub fn from_polynomial(p: &Polynomial) -> Self {
        let n = p.degree().map_or(0, |d| d + 1);
        let coeffs: Vec<BigRational> = (0..n).map(|i| p.coeff(i)).collect();
        Self::from_coeffs(coeffs)
    }

    pub fn zero() -> Self {
        Self::from_coeffs(vec![])
    }

    pub fn one() -> Self {
        Self::constant(BigRational::one())
    }

    pub fn constant(c: BigRational) -> Self {
        if c.is_zero() {
            Self::zero()
        } else {
            Self::from_coeffs(vec![c])
        }
    }

    pub fn x() -> Self {
        Self::from_coeffs(vec![BigRational::zero(), BigRational::one()])
    }

    /// e^x = Σ x^n / n!
    pub fn exp() -> Self {
        Self::from_fn(|n| {
            let mut fact = BigRational::one();
            for i in 2..=n {
                fact *= BigRational::from_integer(BigInt::from(i));
            }
            BigRational::one() / fact
        })
    }

    /// sin(x) = Σ (-1)^k x^{2k+1} / (2k+1)!
    pub fn sin() -> Self {
        Self::from_fn(|n| {
            if n % 2 == 0 {
                return BigRational::zero();
            }
            let k = n / 2;
            let mut fact = BigRational::one();
            for i in 2..=n {
                fact *= BigRational::from_integer(BigInt::from(i));
            }
            if k % 2 == 0 {
                BigRational::one() / fact
            } else {
                -BigRational::one() / fact
            }
        })
    }

    /// cos(x) = Σ (-1)^k x^{2k} / (2k)!
    pub fn cos() -> Self {
        Self::from_fn(|n| {
            if n % 2 == 1 {
                return BigRational::zero();
            }
            let k = n / 2;
            let mut fact = BigRational::one();
            for i in 2..=n {
                fact *= BigRational::from_integer(BigInt::from(i));
            }
            if k % 2 == 0 {
                BigRational::one() / fact
            } else {
                -BigRational::one() / fact
            }
        })
    }

    /// 1/(1-x) = Σ x^n
    pub fn geometric() -> Self {
        Self::from_fn(|_| BigRational::one())
    }

    /// ln(1+x) = Σ_{n≥1} (-1)^{n+1} x^n / n
    pub fn ln_1_plus_x() -> Self {
        Self::from_fn(|n| {
            if n == 0 {
                BigRational::zero()
            } else {
                let sign = if n % 2 == 1 {
                    BigRational::one()
                } else {
                    -BigRational::one()
                };
                sign / BigRational::from_integer(BigInt::from(n))
            }
        })
    }

    pub fn coeff(&self, n: usize) -> BigRational {
        {
            let inner = self.inner.borrow();
            if n < inner.cache.len() {
                return inner.cache[n].clone();
            }
            if matches!(inner.gen, FpsGen::Explicit) {
                return BigRational::zero();
            }
        }

        loop {
            let current_len = self.inner.borrow().cache.len();
            if current_len > n {
                break;
            }
            let val = self.compute_coeff(current_len);
            self.inner.borrow_mut().cache.push(val);
        }

        self.inner.borrow().cache[n].clone()
    }

    pub fn coeffs(&self, n: usize) -> Vec<BigRational> {
        (0..=n).map(|i| self.coeff(i)).collect()
    }

    pub fn truncate(&self, n: usize, var: &str) -> Polynomial {
        Polynomial::from_coeffs(self.coeffs(n), var)
    }

    pub fn scale(&self, c: &BigRational) -> Self {
        if c.is_zero() {
            return Self::zero();
        }
        if c.is_one() {
            return self.clone();
        }
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::ScalarMul(c.clone(), self.clone()),
            })),
        }
    }

    /// Composition f(g(x)) where g(0) = 0.
    pub fn compose(&self, g: &FormalPowerSeries) -> Result<Self, String> {
        if !g.coeff(0).is_zero() {
            return Err("Composition f(g(x)) requires g(0) = 0".to_string());
        }

        let f = self.clone();
        let g = g.clone();
        // g_pow_cache[k] holds coefficients of g^k computed so far
        let g_pow_cache: Rc<RefCell<Vec<Vec<BigRational>>>> =
            Rc::new(RefCell::new(vec![vec![BigRational::one()]]));

        let cache = g_pow_cache;
        Ok(Self::from_fn(move |n| {
            let mut gpc = cache.borrow_mut();

            // Extend existing g-powers to have coefficients through degree n
            for k in 0..gpc.len() {
                while gpc[k].len() <= n {
                    let j = gpc[k].len();
                    let val = if k == 0 {
                        BigRational::zero()
                    } else {
                        let mut v = BigRational::zero();
                        for i in 0..=j {
                            v += &gpc[k - 1][i] * &g.coeff(j - i);
                        }
                        v
                    };
                    gpc[k].push(val);
                }
            }

            // Add new g-powers g^k for k up to n
            while gpc.len() <= n {
                let k = gpc.len();
                let mut gk = Vec::with_capacity(n + 1);
                for j in 0..=n {
                    let mut val = BigRational::zero();
                    for i in 0..=j {
                        val += &gpc[k - 1][i] * &g.coeff(j - i);
                    }
                    gk.push(val);
                }
                gpc.push(gk);
            }

            // [f(g(x))]_n = Σ_{k=0}^{n} f_k · [g^k]_n
            let mut result = BigRational::zero();
            for k in 0..=n {
                let fk = f.coeff(k);
                if !fk.is_zero() {
                    result += fk * &gpc[k][n];
                }
            }
            result
        }))
    }

    /// Multiplicative inverse 1/f(x) where f(0) ≠ 0.
    pub fn inverse(&self) -> Result<Self, String> {
        if self.coeff(0).is_zero() {
            return Err("Multiplicative inverse requires f(0) ≠ 0".to_string());
        }
        Ok(FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Inverse(self.clone()),
            })),
        })
    }

    /// Compositional inverse (reversion): given f with f(0)=0, f'(0)≠0,
    /// compute g such that f(g(x)) = x via Lagrange inversion.
    pub fn revert(&self) -> Result<Self, String> {
        if !self.coeff(0).is_zero() {
            return Err("Reversion requires f(0) = 0".to_string());
        }
        if self.coeff(1).is_zero() {
            return Err("Reversion requires f'(0) ≠ 0".to_string());
        }

        let f = self.clone();
        let h = FormalPowerSeries::from_fn(move |n| f.coeff(n + 1));
        let phi = h.inverse().unwrap();

        let phi_pow_cache: Rc<RefCell<Vec<Vec<BigRational>>>> =
            Rc::new(RefCell::new(vec![vec![BigRational::one()]]));

        Ok(Self::from_fn(move |n| {
            if n == 0 {
                return BigRational::zero();
            }

            let target_deg = n - 1;
            let mut ppc = phi_pow_cache.borrow_mut();

            for k in 0..ppc.len() {
                while ppc[k].len() <= target_deg {
                    let j = ppc[k].len();
                    let val = if k == 0 {
                        BigRational::zero()
                    } else {
                        let mut v = BigRational::zero();
                        for i in 0..=j {
                            v += &ppc[k - 1][i] * &phi.coeff(j - i);
                        }
                        v
                    };
                    ppc[k].push(val);
                }
            }

            while ppc.len() <= n {
                let k = ppc.len();
                let mut pk = Vec::with_capacity(target_deg + 1);
                for j in 0..=target_deg {
                    let mut val = BigRational::zero();
                    for i in 0..=j {
                        val += &ppc[k - 1][i] * &phi.coeff(j - i);
                    }
                    pk.push(val);
                }
                ppc.push(pk);
            }

            ppc[n][target_deg].clone() / BigRational::from_integer(BigInt::from(n))
        }))
    }

    /// Quotient f(x)/g(x) where g(0) ≠ 0.
    pub fn quotient(&self, g: &FormalPowerSeries) -> Result<Self, String> {
        if g.coeff(0).is_zero() {
            return Err("Quotient requires g(0) ≠ 0".to_string());
        }
        Ok(FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Quotient(self.clone(), g.clone()),
            })),
        })
    }

    /// Formal derivative: [f'(x)]_n = (n+1) · f_{n+1}.
    pub fn formal_derivative(&self) -> Self {
        let f = self.clone();
        Self::from_fn(move |n| BigRational::from_integer(BigInt::from(n + 1)) * f.coeff(n + 1))
    }

    /// Formal integral: [∫f(x)dx]_n = f_{n-1}/n for n ≥ 1, 0 for n = 0.
    pub fn formal_integral(&self) -> Self {
        let f = self.clone();
        Self::from_fn(move |n| {
            if n == 0 {
                BigRational::zero()
            } else {
                f.coeff(n - 1) / BigRational::from_integer(BigInt::from(n))
            }
        })
    }

    fn compute_coeff(&self, n: usize) -> BigRational {
        let action = {
            let inner = self.inner.borrow();
            match &inner.gen {
                FpsGen::Explicit => CoeffAction::Resolved(BigRational::zero()),
                FpsGen::Closure(f) => CoeffAction::Resolved(f(n)),
                FpsGen::Sum(a, b) => CoeffAction::Sum(a.clone(), b.clone()),
                FpsGen::Diff(a, b) => CoeffAction::Diff(a.clone(), b.clone()),
                FpsGen::Neg(a) => CoeffAction::Neg(a.clone()),
                FpsGen::ScalarMul(c, a) => CoeffAction::ScalarMul(c.clone(), a.clone()),
                FpsGen::Product(a, b) => CoeffAction::Product(a.clone(), b.clone()),
                FpsGen::Inverse(f) => CoeffAction::Inverse(f.clone()),
                FpsGen::Quotient(num, den) => CoeffAction::Quotient(num.clone(), den.clone()),
            }
        };

        match action {
            CoeffAction::Resolved(v) => v,
            CoeffAction::Sum(a, b) => a.coeff(n) + b.coeff(n),
            CoeffAction::Diff(a, b) => a.coeff(n) - b.coeff(n),
            CoeffAction::Neg(a) => -a.coeff(n),
            CoeffAction::ScalarMul(c, a) => c * a.coeff(n),
            CoeffAction::Product(a, b) => {
                let mut sum = BigRational::zero();
                for k in 0..=n {
                    sum += a.coeff(k) * b.coeff(n - k);
                }
                sum
            }
            CoeffAction::Inverse(f) => {
                let f0 = f.coeff(0);
                if n == 0 {
                    BigRational::one() / f0
                } else {
                    let mut sum = BigRational::zero();
                    for k in 1..=n {
                        sum += f.coeff(k) * self.coeff(n - k);
                    }
                    -(BigRational::one() / &f0) * sum
                }
            }
            CoeffAction::Quotient(num, den) => {
                let den0 = den.coeff(0);
                if n == 0 {
                    num.coeff(0) / den0
                } else {
                    let mut sum = BigRational::zero();
                    for k in 1..=n {
                        sum += den.coeff(k) * self.coeff(n - k);
                    }
                    (num.coeff(n) - sum) / den0
                }
            }
        }
    }
}

impl Add for &FormalPowerSeries {
    type Output = FormalPowerSeries;
    fn add(self, rhs: Self) -> FormalPowerSeries {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Sum(self.clone(), rhs.clone()),
            })),
        }
    }
}

impl Sub for &FormalPowerSeries {
    type Output = FormalPowerSeries;
    fn sub(self, rhs: Self) -> FormalPowerSeries {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Diff(self.clone(), rhs.clone()),
            })),
        }
    }
}

impl Mul for &FormalPowerSeries {
    type Output = FormalPowerSeries;
    fn mul(self, rhs: Self) -> FormalPowerSeries {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Product(self.clone(), rhs.clone()),
            })),
        }
    }
}

impl Div for &FormalPowerSeries {
    type Output = FormalPowerSeries;
    fn div(self, rhs: Self) -> FormalPowerSeries {
        self.quotient(rhs).expect("Division requires g(0) ≠ 0")
    }
}

impl Neg for &FormalPowerSeries {
    type Output = FormalPowerSeries;
    fn neg(self) -> FormalPowerSeries {
        FormalPowerSeries {
            inner: Rc::new(RefCell::new(FpsInner {
                cache: Vec::new(),
                gen: FpsGen::Neg(self.clone()),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn br(n: i64, d: i64) -> BigRational {
        BigRational::new(BigInt::from(n), BigInt::from(d))
    }

    fn bri(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    #[test]
    fn test_explicit_coefficients() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(2), bri(3)]);
        assert_eq!(f.coeff(0), bri(1));
        assert_eq!(f.coeff(1), bri(2));
        assert_eq!(f.coeff(2), bri(3));
        assert_eq!(f.coeff(3), bri(0));
        assert_eq!(f.coeff(10), bri(0));
    }

    #[test]
    fn test_from_fn_geometric() {
        let geo = FormalPowerSeries::geometric();
        for i in 0..10 {
            assert_eq!(geo.coeff(i), bri(1));
        }
    }

    #[test]
    fn test_exp_coefficients() {
        let e = FormalPowerSeries::exp();
        assert_eq!(e.coeff(0), bri(1));
        assert_eq!(e.coeff(1), bri(1));
        assert_eq!(e.coeff(2), br(1, 2));
        assert_eq!(e.coeff(3), br(1, 6));
        assert_eq!(e.coeff(4), br(1, 24));
    }

    #[test]
    fn test_sin_coefficients() {
        let s = FormalPowerSeries::sin();
        assert_eq!(s.coeff(0), bri(0));
        assert_eq!(s.coeff(1), bri(1));
        assert_eq!(s.coeff(2), bri(0));
        assert_eq!(s.coeff(3), br(-1, 6));
        assert_eq!(s.coeff(4), bri(0));
        assert_eq!(s.coeff(5), br(1, 120));
    }

    #[test]
    fn test_cos_coefficients() {
        let c = FormalPowerSeries::cos();
        assert_eq!(c.coeff(0), bri(1));
        assert_eq!(c.coeff(1), bri(0));
        assert_eq!(c.coeff(2), br(-1, 2));
        assert_eq!(c.coeff(3), bri(0));
        assert_eq!(c.coeff(4), br(1, 24));
    }

    #[test]
    fn test_ln_1_plus_x() {
        let ln = FormalPowerSeries::ln_1_plus_x();
        assert_eq!(ln.coeff(0), bri(0));
        assert_eq!(ln.coeff(1), bri(1));
        assert_eq!(ln.coeff(2), br(-1, 2));
        assert_eq!(ln.coeff(3), br(1, 3));
        assert_eq!(ln.coeff(4), br(-1, 4));
    }

    #[test]
    fn test_zero_and_one() {
        let z = FormalPowerSeries::zero();
        for n in 0..5 {
            assert!(z.coeff(n).is_zero());
        }
        let o = FormalPowerSeries::one();
        assert_eq!(o.coeff(0), bri(1));
        for n in 1..5 {
            assert!(o.coeff(n).is_zero());
        }
    }

    #[test]
    fn test_x() {
        let x = FormalPowerSeries::x();
        assert!(x.coeff(0).is_zero());
        assert_eq!(x.coeff(1), bri(1));
        assert!(x.coeff(2).is_zero());
    }

    #[test]
    fn test_addition() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(2), bri(3)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(4), bri(5)]);
        let h = &f + &g;
        assert_eq!(h.coeff(0), bri(5));
        assert_eq!(h.coeff(1), bri(7));
        assert_eq!(h.coeff(2), bri(3));
        assert_eq!(h.coeff(3), bri(0));
    }

    #[test]
    fn test_subtraction() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(5), bri(3)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(2), bri(1)]);
        let h = &f - &g;
        assert_eq!(h.coeff(0), bri(3));
        assert_eq!(h.coeff(1), bri(2));
    }

    #[test]
    fn test_negation() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(-2), bri(3)]);
        let g = -&f;
        assert_eq!(g.coeff(0), bri(-1));
        assert_eq!(g.coeff(1), bri(2));
        assert_eq!(g.coeff(2), bri(-3));
    }

    #[test]
    fn test_scalar_mul() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(2), bri(3)]);
        let g = f.scale(&bri(3));
        assert_eq!(g.coeff(0), bri(3));
        assert_eq!(g.coeff(1), bri(6));
        assert_eq!(g.coeff(2), bri(9));
    }

    #[test]
    fn test_cauchy_product_binomial_square() {
        // (1 + x)^2 = 1 + 2x + x^2
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        let g = &f * &f;
        assert_eq!(g.coeff(0), bri(1));
        assert_eq!(g.coeff(1), bri(2));
        assert_eq!(g.coeff(2), bri(1));
        assert_eq!(g.coeff(3), bri(0));
    }

    #[test]
    fn test_cauchy_product_geometric_squared() {
        // (1/(1-x))^2 = Σ (n+1) x^n
        let geo = FormalPowerSeries::geometric();
        let geo2 = &geo * &geo;
        for n in 0..8 {
            assert_eq!(geo2.coeff(n), bri(n as i64 + 1));
        }
    }

    #[test]
    fn test_self_multiply() {
        // (1 + x + x^2)^2 = 1 + 2x + 3x^2 + 2x^3 + x^4
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1), bri(1)]);
        let ff = &f * &f;
        assert_eq!(ff.coeff(0), bri(1));
        assert_eq!(ff.coeff(1), bri(2));
        assert_eq!(ff.coeff(2), bri(3));
        assert_eq!(ff.coeff(3), bri(2));
        assert_eq!(ff.coeff(4), bri(1));
        assert_eq!(ff.coeff(5), bri(0));
    }

    #[test]
    fn test_inverse_geometric_from_1_minus_x() {
        // 1/(1-x) is the inverse of (1-x)
        let one_minus_x = FormalPowerSeries::from_coeffs(vec![bri(1), bri(-1)]);
        let inv = one_minus_x.inverse().unwrap();
        for n in 0..8 {
            assert_eq!(inv.coeff(n), bri(1), "coeff({}) should be 1", n);
        }
    }

    #[test]
    fn test_inverse_2_plus_x() {
        // 1/(2+x) = Σ (-1)^n / 2^{n+1} x^n
        let f = FormalPowerSeries::from_coeffs(vec![bri(2), bri(1)]);
        let inv = f.inverse().unwrap();
        assert_eq!(inv.coeff(0), br(1, 2));
        assert_eq!(inv.coeff(1), br(-1, 4));
        assert_eq!(inv.coeff(2), br(1, 8));
        assert_eq!(inv.coeff(3), br(-1, 16));
    }

    #[test]
    fn test_inverse_roundtrip() {
        // (1+x) · 1/(1+x) = 1
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        let f_inv = f.inverse().unwrap();
        let product = &f * &f_inv;
        assert_eq!(product.coeff(0), bri(1));
        for n in 1..8 {
            assert!(
                product.coeff(n).is_zero(),
                "coeff({}) should be 0, got {}",
                n,
                product.coeff(n)
            );
        }
    }

    #[test]
    fn test_inverse_error_zero_constant() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(0), bri(1)]);
        assert!(f.inverse().is_err());
    }

    #[test]
    fn test_exp_times_exp_neg_is_one() {
        // e^x · e^{-x} = 1
        let ex = FormalPowerSeries::exp();
        let emx = FormalPowerSeries::from_fn(|n| {
            let mut fact = BigRational::one();
            for i in 2..=n {
                fact *= BigRational::from_integer(BigInt::from(i));
            }
            let sign = if n % 2 == 0 {
                BigRational::one()
            } else {
                -BigRational::one()
            };
            sign / fact
        });
        let product = &ex * &emx;
        assert_eq!(product.coeff(0), bri(1));
        for n in 1..8 {
            assert!(
                product.coeff(n).is_zero(),
                "e^x·e^(-x) coeff({}) should be 0, got {}",
                n,
                product.coeff(n)
            );
        }
    }

    #[test]
    fn test_sin_squared_plus_cos_squared() {
        let s = FormalPowerSeries::sin();
        let c = FormalPowerSeries::cos();
        let s2 = &s * &s;
        let c2 = &c * &c;
        let sum = &s2 + &c2;
        assert_eq!(sum.coeff(0), bri(1));
        for n in 1..10 {
            assert!(
                sum.coeff(n).is_zero(),
                "sin²+cos² coeff({}) should be 0, got {}",
                n,
                sum.coeff(n)
            );
        }
    }

    #[test]
    fn test_compose_linear() {
        // f(x) = 1 + x, g(x) = 2x → f(g(x)) = 1 + 2x
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(0), bri(2)]);
        let fg = f.compose(&g).unwrap();
        assert_eq!(fg.coeff(0), bri(1));
        assert_eq!(fg.coeff(1), bri(2));
        assert_eq!(fg.coeff(2), bri(0));
    }

    #[test]
    fn test_compose_quadratic_in_2x() {
        // f(x) = 1 + x + x^2, g(x) = 2x → f(g(x)) = 1 + 2x + 4x^2
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1), bri(1)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(0), bri(2)]);
        let fg = f.compose(&g).unwrap();
        assert_eq!(fg.coeff(0), bri(1));
        assert_eq!(fg.coeff(1), bri(2));
        assert_eq!(fg.coeff(2), bri(4));
        assert_eq!(fg.coeff(3), bri(0));
    }

    #[test]
    fn test_compose_exp_ln_is_identity() {
        // exp(ln(1+x)) = 1 + x
        let e = FormalPowerSeries::exp();
        let ln = FormalPowerSeries::ln_1_plus_x();
        let composed = e.compose(&ln).unwrap();
        assert_eq!(composed.coeff(0), bri(1));
        assert_eq!(composed.coeff(1), bri(1));
        for n in 2..8 {
            assert!(
                composed.coeff(n).is_zero(),
                "exp(ln(1+x)) coeff({}) should be 0, got {}",
                n,
                composed.coeff(n)
            );
        }
    }

    #[test]
    fn test_compose_requires_g0_zero() {
        let f = FormalPowerSeries::exp();
        let g = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        assert!(f.compose(&g).is_err());
    }

    #[test]
    fn test_truncate_to_polynomial() {
        let e = FormalPowerSeries::exp();
        let poly = e.truncate(4, "x");
        assert_eq!(poly.coeff(0), bri(1));
        assert_eq!(poly.coeff(1), bri(1));
        assert_eq!(poly.coeff(2), br(1, 2));
        assert_eq!(poly.coeff(3), br(1, 6));
        assert_eq!(poly.coeff(4), br(1, 24));
    }

    #[test]
    fn test_from_polynomial_roundtrip() {
        let poly = Polynomial::from_coeffs(vec![bri(1), bri(2), bri(3)], "x");
        let fps = FormalPowerSeries::from_polynomial(&poly);
        assert_eq!(fps.coeff(0), bri(1));
        assert_eq!(fps.coeff(1), bri(2));
        assert_eq!(fps.coeff(2), bri(3));
        assert_eq!(fps.coeff(3), bri(0));
        let poly2 = fps.truncate(2, "x");
        assert_eq!(poly, poly2);
    }

    #[test]
    fn test_formal_derivative_of_exp() {
        // d/dx(e^x) = e^x
        let e = FormalPowerSeries::exp();
        let de = e.formal_derivative();
        for n in 0..8 {
            assert_eq!(de.coeff(n), e.coeff(n), "d/dx(e^x) coeff({}) mismatch", n);
        }
    }

    #[test]
    fn test_formal_derivative_sin_is_cos() {
        let s = FormalPowerSeries::sin();
        let ds = s.formal_derivative();
        let c = FormalPowerSeries::cos();
        for n in 0..8 {
            assert_eq!(
                ds.coeff(n),
                c.coeff(n),
                "d/dx(sin(x)) coeff({}) mismatch",
                n
            );
        }
    }

    #[test]
    fn test_formal_integral_of_cos_is_sin() {
        let c = FormalPowerSeries::cos();
        let ic = c.formal_integral();
        let s = FormalPowerSeries::sin();
        for n in 0..8 {
            assert_eq!(ic.coeff(n), s.coeff(n), "∫cos(x)dx coeff({}) mismatch", n);
        }
    }

    #[test]
    fn test_derivative_integral_roundtrip() {
        // d/dx(∫f dx) = f
        let f = FormalPowerSeries::from_coeffs(vec![bri(3), bri(-1), bri(7), bri(2)]);
        let dif = f.formal_integral().formal_derivative();
        for n in 0..4 {
            assert_eq!(dif.coeff(n), f.coeff(n), "d/dx(∫f) coeff({}) mismatch", n);
        }
    }

    #[test]
    fn test_coeffs_vec() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(2), bri(3)]);
        let c = f.coeffs(4);
        assert_eq!(c, vec![bri(1), bri(2), bri(3), bri(0), bri(0)]);
    }

    #[test]
    fn test_inverse_via_composition() {
        // Verify: 1/exp(x) = exp(-x) via inverse
        let e = FormalPowerSeries::exp();
        let e_inv = e.inverse().unwrap();
        for n in 0..8 {
            let mut fact = BigRational::one();
            for i in 2..=n {
                fact *= BigRational::from_integer(BigInt::from(i));
            }
            let expected = if n % 2 == 0 {
                BigRational::one() / &fact
            } else {
                -BigRational::one() / &fact
            };
            assert_eq!(e_inv.coeff(n), expected, "1/e^x coeff({}) mismatch", n);
        }
    }

    #[test]
    fn test_compose_with_self_square() {
        // f(x) = x, g(x) = x → f(g(x)) = x
        let x = FormalPowerSeries::x();
        let composed = x.compose(&x).unwrap();
        assert!(composed.coeff(0).is_zero());
        assert_eq!(composed.coeff(1), bri(1));
        for n in 2..5 {
            assert!(composed.coeff(n).is_zero());
        }
    }

    #[test]
    fn test_revert_sin_is_arcsin() {
        // arcsin(x) = x + x³/6 + 3x⁵/40 + 5x⁷/112
        let s = FormalPowerSeries::sin();
        let asin = s.revert().unwrap();
        assert_eq!(asin.coeff(0), bri(0));
        assert_eq!(asin.coeff(1), bri(1));
        assert_eq!(asin.coeff(2), bri(0));
        assert_eq!(asin.coeff(3), br(1, 6));
        assert_eq!(asin.coeff(4), bri(0));
        assert_eq!(asin.coeff(5), br(3, 40));
        assert_eq!(asin.coeff(6), bri(0));
        assert_eq!(asin.coeff(7), br(5, 112));
    }

    #[test]
    fn test_revert_exp_minus_1_is_ln_1_plus_x() {
        // f(x) = e^x - 1 = x + x²/2 + x³/6 + ...
        // f^{-1}(x) = ln(1+x) = x - x²/2 + x³/3 - ...
        let exp_m1 = FormalPowerSeries::from_fn(|n| {
            if n == 0 {
                BigRational::zero()
            } else {
                let mut fact = BigRational::one();
                for i in 2..=n {
                    fact *= BigRational::from_integer(BigInt::from(i));
                }
                BigRational::one() / fact
            }
        });
        let rev = exp_m1.revert().unwrap();
        let ln = FormalPowerSeries::ln_1_plus_x();
        for n in 0..8 {
            assert_eq!(
                rev.coeff(n),
                ln.coeff(n),
                "revert(e^x-1) coeff({}) mismatch",
                n
            );
        }
    }

    #[test]
    fn test_revert_roundtrip() {
        // f(g(x)) = x where g = revert(f)
        let s = FormalPowerSeries::sin();
        let asin = s.revert().unwrap();
        let composed = s.compose(&asin).unwrap();
        assert_eq!(composed.coeff(0), bri(0));
        assert_eq!(composed.coeff(1), bri(1));
        for n in 2..8 {
            assert!(
                composed.coeff(n).is_zero(),
                "sin(arcsin(x)) coeff({}) should be 0, got {}",
                n,
                composed.coeff(n)
            );
        }
    }

    #[test]
    fn test_revert_error_nonzero_constant() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        assert!(f.revert().is_err());
    }

    #[test]
    fn test_revert_error_zero_linear() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(0), bri(0), bri(1)]);
        assert!(f.revert().is_err());
    }

    #[test]
    fn test_quotient_sin_over_cos_is_tan() {
        // tan(x) = sin(x)/cos(x) = x + x³/3 + 2x⁵/15 + ...
        let s = FormalPowerSeries::sin();
        let c = FormalPowerSeries::cos();
        let tan = s.quotient(&c).unwrap();
        assert_eq!(tan.coeff(0), bri(0));
        assert_eq!(tan.coeff(1), bri(1));
        assert_eq!(tan.coeff(2), bri(0));
        assert_eq!(tan.coeff(3), br(1, 3));
        assert_eq!(tan.coeff(4), bri(0));
        assert_eq!(tan.coeff(5), br(2, 15));
        assert_eq!(tan.coeff(6), bri(0));
        assert_eq!(tan.coeff(7), br(17, 315));
    }

    #[test]
    fn test_quotient_matches_mul_inverse() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(3), bri(-1), bri(7), bri(2)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(2), bri(5), bri(-3)]);
        let q1 = f.quotient(&g).unwrap();
        let g_inv = g.inverse().unwrap();
        let q2 = &f * &g_inv;
        for n in 0..8 {
            assert_eq!(
                q1.coeff(n),
                q2.coeff(n),
                "quotient vs mul·inverse coeff({}) mismatch",
                n
            );
        }
    }

    #[test]
    fn test_quotient_polynomial_exact() {
        // (1 + 3x + 2x²) / (1 + x) = (1+x)(1+2x) / (1+x) = 1 + 2x
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(3), bri(2)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        let q = &f / &g;
        assert_eq!(q.coeff(0), bri(1));
        assert_eq!(q.coeff(1), bri(2));
        for n in 2..6 {
            assert!(
                q.coeff(n).is_zero(),
                "exact division coeff({}) should be 0",
                n
            );
        }
    }

    #[test]
    fn test_quotient_error_zero_constant() {
        let f = FormalPowerSeries::from_coeffs(vec![bri(1), bri(1)]);
        let g = FormalPowerSeries::from_coeffs(vec![bri(0), bri(1)]);
        assert!(f.quotient(&g).is_err());
    }

    #[test]
    fn test_catalan_via_quadratic() {
        // The generating function for Catalan numbers satisfies C(x) = 1 + x·C(x)²
        // C(x) = (1 - √(1-4x)) / (2x) = Σ C_n x^n where C_n = (2n)! / ((n+1)!·n!)
        // Equivalently: x·C²(x) - C(x) + 1 = 0, or C = 1/(1 - x·C)
        // We can verify: 1/(1-x) composed with x·C(x) gives C(x)
        // But let's just verify the first few Catalan numbers via the inverse:
        // C(x) = 1 + x + 2x² + 5x³ + 14x⁴ + ...
        // Check: (1-x·C(x)) · C(x) = 1? No, the relation is C = 1 + x·C²
        // Use the recurrence: C_0 = 1, C_{n+1} = Σ_{i=0}^{n} C_i · C_{n-i}
        let catalan = FormalPowerSeries::from_fn({
            let cache: Rc<RefCell<Vec<BigRational>>> =
                Rc::new(RefCell::new(vec![BigRational::one()]));
            move |n| {
                let mut c = cache.borrow_mut();
                while c.len() <= n {
                    let m = c.len() - 1;
                    let mut val = BigRational::zero();
                    for i in 0..=m {
                        val += &c[i] * &c[m - i];
                    }
                    c.push(val);
                }
                c[n].clone()
            }
        });
        assert_eq!(catalan.coeff(0), bri(1));
        assert_eq!(catalan.coeff(1), bri(1));
        assert_eq!(catalan.coeff(2), bri(2));
        assert_eq!(catalan.coeff(3), bri(5));
        assert_eq!(catalan.coeff(4), bri(14));
        assert_eq!(catalan.coeff(5), bri(42));
    }
}
