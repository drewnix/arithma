# Risch Algorithm Session 1: Foundation Types

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the algebraic foundation types (`RationalFunction`, `ExtPoly`, `DifferentialExtension`) and Hermite reduction needed for the Risch decision procedure.

**Architecture:** The Risch algorithm integrates functions in a tower of transcendental extensions over Q(x). Each extension adds one variable θ = exp(f) or θ = log(f). We represent elements of Q(x) as `RationalFunction` (ratio of two `Polynomial`), and elements of Q(x)(θ) as `ExtPoly` (polynomial in θ with `RationalFunction` coefficients). Hermite reduction is the workhorse subroutine that splits an integral into a known rational part plus an integral with squarefree denominator.

**Tech Stack:** Rust, `num-bigint`/`num-rational`/`num-traits` (already in Cargo.toml), existing `Polynomial` type in `src/polynomial.rs`.

---

### Task 1: RationalFunction — Type and Constructors

**Files:**
- Create: `src/rational_function.rs`
- Modify: `src/lib.rs` (add module declaration)
- Test: inline `#[cfg(test)]` module in `src/rational_function.rs`

**Step 1: Write the failing test**

```rust
// In src/rational_function.rs
#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn poly(coeffs: &[i64], var: &str) -> Polynomial {
        let cs: Vec<BigRational> = coeffs.iter().map(|&c| int(c)).collect();
        Polynomial::from_coeffs(cs, var)
    }

    #[test]
    fn test_rf_normalize_cancels_gcd() {
        // (x^2 - 1) / (x + 1) = (x - 1) / 1
        let num = poly(&[-1, 0, 1], "x"); // x^2 - 1
        let den = poly(&[1, 1], "x");     // x + 1
        let rf = RationalFunction::new(num, den);
        assert_eq!(rf.numerator(), &poly(&[-1, 1], "x")); // x - 1
        assert!(rf.denominator().is_constant());
    }

    #[test]
    fn test_rf_zero() {
        let rf = RationalFunction::zero("x");
        assert!(rf.is_zero());
    }

    #[test]
    fn test_rf_from_poly() {
        let p = poly(&[1, 2, 3], "x");
        let rf = RationalFunction::from_poly(p.clone());
        assert_eq!(rf.numerator(), &p);
        assert_eq!(rf.denominator(), &Polynomial::one("x"));
    }

    #[test]
    fn test_rf_display() {
        let rf = RationalFunction::new(poly(&[1, 1], "x"), poly(&[1, 0, 1], "x"));
        let s = format!("{}", rf);
        assert!(s.contains("x + 1"));
        assert!(s.contains("x^2 + 1"));
    }
}
```

**Step 2: Implement RationalFunction**

```rust
use crate::polynomial::Polynomial;
use num_rational::BigRational;
use num_traits::{One, Zero};
use std::fmt;

#[derive(Debug, Clone)]
pub struct RationalFunction {
    num: Polynomial,
    den: Polynomial,
}

impl RationalFunction {
    pub fn new(num: Polynomial, den: Polynomial) -> Self {
        // Cancel common factors via GCD, make denominator monic
        // Handle zero numerator and zero denominator
        ...
    }

    pub fn zero(var: &str) -> Self { ... }
    pub fn one(var: &str) -> Self { ... }
    pub fn from_poly(p: Polynomial) -> Self { ... }
    pub fn from_constant(c: BigRational, var: &str) -> Self { ... }
    pub fn numerator(&self) -> &Polynomial { &self.num }
    pub fn denominator(&self) -> &Polynomial { &self.den }
    pub fn is_zero(&self) -> bool { self.num.is_zero() }
    pub fn is_constant(&self) -> bool { ... }
    pub fn variable(&self) -> &str { self.num.variable() }
}

impl fmt::Display for RationalFunction { ... }
impl PartialEq for RationalFunction { ... }
```

Key invariants:
- `den` is never zero
- `gcd(num, den) = 1` (always reduced)
- `den` is monic (leading coefficient = 1)
- If `num` is zero, `den` is `1`

**Step 3: Run tests**

```
cargo test --lib rational_function -- --nocapture
```

**Step 4: Add module to lib.rs**

Add after the `polynomial` module:
```rust
pub mod rational_function;
pub use crate::rational_function::RationalFunction;
```

**Step 5: Run full test suite and clippy**

```
cargo clippy --tests -- -D warnings && cargo test
```

**Step 6: Commit**

```
git add src/rational_function.rs src/lib.rs
git commit -m "Add RationalFunction type: p(x)/q(x) with auto-normalization"
```

---

### Task 2: RationalFunction Arithmetic

**Files:**
- Modify: `src/rational_function.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_rf_add() {
    // 1/x + 1/(x+1) = (2x+1) / (x(x+1)) = (2x+1) / (x^2+x)
    let a = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let b = RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x"));
    let sum = &a + &b;
    assert_eq!(sum.numerator(), &poly(&[1, 2], "x"));
    assert_eq!(sum.denominator(), &poly(&[0, 1, 1], "x"));
}

#[test]
fn test_rf_add_cancels() {
    // 1/(x+1) + (-1)/(x+1) = 0
    let a = RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x"));
    let b = RationalFunction::new(poly(&[-1], "x"), poly(&[1, 1], "x"));
    assert!((&a + &b).is_zero());
}

#[test]
fn test_rf_sub() {
    // 1/x - 1/(x+1) = 1/(x^2 + x)
    let a = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let b = RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x"));
    let diff = &a - &b;
    assert_eq!(diff.numerator(), &poly(&[1], "x"));
    assert_eq!(diff.denominator(), &poly(&[0, 1, 1], "x"));
}

#[test]
fn test_rf_mul() {
    // (x+1)/x * x/(x-1) = (x+1)/(x-1)
    let a = RationalFunction::new(poly(&[1, 1], "x"), poly(&[0, 1], "x"));
    let b = RationalFunction::new(poly(&[0, 1], "x"), poly(&[-1, 1], "x"));
    let prod = &a * &b;
    assert_eq!(prod.numerator(), &poly(&[1, 1], "x"));
    assert_eq!(prod.denominator(), &poly(&[-1, 1], "x"));
}

#[test]
fn test_rf_div() {
    // (1/x) / (1/x) = 1
    let a = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let result = a.checked_div(&a).unwrap();
    assert_eq!(result, RationalFunction::one("x"));
}

#[test]
fn test_rf_neg() {
    let a = RationalFunction::new(poly(&[1, 1], "x"), poly(&[0, 1], "x"));
    let neg_a = -&a;
    assert_eq!(neg_a.numerator(), &poly(&[-1, -1], "x"));
}
```

**Step 2: Implement arithmetic operators**

Implement `Add`, `Sub`, `Mul`, `Neg` for `&RationalFunction`, plus `checked_div` method.

Formula for each:
- Add: `(a/b) + (c/d) = (a*d + b*c) / (b*d)`, then normalize
- Sub: `(a/b) - (c/d) = (a*d - b*c) / (b*d)`, then normalize
- Mul: `(a/b) * (c/d) = (a*c) / (b*d)`, then normalize
- Div: `(a/b) / (c/d) = (a*d) / (b*c)`, then normalize (error if c = 0)
- Neg: `-(a/b) = (-a) / b`

**Step 3: Run tests**

```
cargo test --lib rational_function -- --nocapture
```

**Step 4: Run full suite + clippy**

```
cargo clippy --tests -- -D warnings && cargo test
```

**Step 5: Commit**

```
git add src/rational_function.rs
git commit -m "Add RationalFunction arithmetic: add, sub, mul, div, neg"
```

---

### Task 3: RationalFunction Derivative

**Files:**
- Modify: `src/rational_function.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_rf_derivative_polynomial() {
    // d/dx[x^2 + 1] = 2x
    let rf = RationalFunction::from_poly(poly(&[1, 0, 1], "x"));
    let drf = rf.derivative();
    assert_eq!(drf, RationalFunction::from_poly(poly(&[0, 2], "x")));
}

#[test]
fn test_rf_derivative_reciprocal() {
    // d/dx[1/x] = -1/x^2
    let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let drf = rf.derivative();
    assert_eq!(drf.numerator(), &poly(&[-1], "x"));
    assert_eq!(drf.denominator(), &poly(&[0, 0, 1], "x"));
}

#[test]
fn test_rf_derivative_quotient_rule() {
    // d/dx[(x+1)/(x-1)] = -2/(x-1)^2 = -2/(x^2 - 2x + 1)
    let rf = RationalFunction::new(poly(&[1, 1], "x"), poly(&[-1, 1], "x"));
    let drf = rf.derivative();
    assert_eq!(drf.numerator(), &poly(&[-2], "x"));
    assert_eq!(drf.denominator(), &poly(&[1, -2, 1], "x"));
}
```

**Step 2: Implement derivative**

```rust
pub fn derivative(&self) -> Self {
    // Quotient rule: (p/q)' = (p'q - pq') / q^2
    let p_prime = self.num.derivative();
    let q_prime = self.den.derivative();
    let num = &(&p_prime * &self.den) - &(&self.num * &q_prime);
    let den = &self.den * &self.den;
    Self::new(num, den)
}
```

**Step 3: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/rational_function.rs
git commit -m "Add RationalFunction derivative via quotient rule"
```

---

### Task 4: ExtPoly — Polynomial in θ with RationalFunction Coefficients

**Files:**
- Create: `src/ext_poly.rs`
- Modify: `src/lib.rs`
- Test: inline `#[cfg(test)]` in `src/ext_poly.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use crate::polynomial::Polynomial;

    fn int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    fn poly(coeffs: &[i64], var: &str) -> Polynomial {
        let cs: Vec<BigRational> = coeffs.iter().map(|&c| int(c)).collect();
        Polynomial::from_coeffs(cs, var)
    }

    fn rf_const(n: i64) -> RationalFunction {
        RationalFunction::from_constant(int(n), "x")
    }

    fn rf_poly(coeffs: &[i64]) -> RationalFunction {
        RationalFunction::from_poly(poly(coeffs, "x"))
    }

    #[test]
    fn test_ext_poly_zero() {
        let p = ExtPoly::zero("x");
        assert!(p.is_zero());
        assert_eq!(p.degree(), None);
    }

    #[test]
    fn test_ext_poly_constant() {
        let p = ExtPoly::from_rf(rf_const(5));
        assert_eq!(p.degree(), Some(0));
        assert!(!p.is_zero());
    }

    #[test]
    fn test_ext_poly_theta() {
        // θ  (the identity polynomial in θ)
        let p = ExtPoly::theta("x");
        assert_eq!(p.degree(), Some(1));
    }

    #[test]
    fn test_ext_poly_from_coeffs() {
        // (x+1)θ^2 + 1/x θ + 3
        let coeffs = vec![
            rf_const(3),
            RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")),
            rf_poly(&[1, 1]),
        ];
        let p = ExtPoly::from_coeffs(coeffs, "x");
        assert_eq!(p.degree(), Some(2));
    }

    #[test]
    fn test_ext_poly_display() {
        // 2θ + 1
        let p = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
        let s = format!("{}", p);
        assert!(s.contains("θ"));
    }
}
```

**Step 2: Implement ExtPoly**

```rust
use crate::rational_function::RationalFunction;
use crate::polynomial::Polynomial;
use num_rational::BigRational;
use num_traits::Zero;
use std::fmt;

/// Polynomial in a tower variable θ, with coefficients in Q(x).
/// coeffs[i] is the coefficient of θ^i. Trailing zeros are stripped.
#[derive(Debug, Clone)]
pub struct ExtPoly {
    coeffs: Vec<RationalFunction>,
    var: String, // base variable for the RationalFunction coefficients
}

impl ExtPoly {
    pub fn zero(var: &str) -> Self { ... }
    pub fn from_rf(rf: RationalFunction) -> Self { ... }
    pub fn theta(var: &str) -> Self { ... }
    pub fn from_coeffs(mut coeffs: Vec<RationalFunction>, var: &str) -> Self { ... strip trailing zeros ... }
    pub fn degree(&self) -> Option<usize> { ... }
    pub fn is_zero(&self) -> bool { self.coeffs.is_empty() }
    pub fn leading_coeff(&self) -> Option<&RationalFunction> { self.coeffs.last() }
    pub fn coeff(&self, i: usize) -> RationalFunction { ... }
    pub fn variable(&self) -> &str { &self.var }
}
```

**Step 3: Add to lib.rs**

```rust
pub mod ext_poly;
pub use crate::ext_poly::ExtPoly;
```

**Step 4: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/ext_poly.rs src/lib.rs
git commit -m "Add ExtPoly type: polynomial in θ with Q(x) coefficients"
```

---

### Task 5: ExtPoly Arithmetic — Add, Sub, Mul, ScalarMul

**Files:**
- Modify: `src/ext_poly.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_ext_poly_add() {
    // (2θ + 1) + (3θ + 4) = 5θ + 5
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(4), rf_const(3)], "x");
    let sum = &a + &b;
    assert_eq!(sum.degree(), Some(1));
    assert_eq!(sum.coeff(0), rf_const(5));
    assert_eq!(sum.coeff(1), rf_const(5));
}

#[test]
fn test_ext_poly_add_cancels() {
    // (θ + 1) + (-θ + 2) = 3
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(2), rf_const(-1)], "x");
    let sum = &a + &b;
    assert_eq!(sum.degree(), Some(0));
    assert_eq!(sum.coeff(0), rf_const(3));
}

#[test]
fn test_ext_poly_sub() {
    // (2θ + 1) - (θ + 1) = θ
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let diff = &a - &b;
    assert_eq!(diff.degree(), Some(1));
    assert_eq!(diff.coeff(0), RationalFunction::zero("x"));
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
    let x_rf = rf_poly(&[0, 1]);
    let a = ExtPoly::from_coeffs(vec![RationalFunction::zero("x"), x_rf.clone()], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let prod = &a * &b;
    assert_eq!(prod.degree(), Some(2));
    assert_eq!(prod.coeff(1), x_rf);
    assert_eq!(prod.coeff(2), x_rf);
}

#[test]
fn test_ext_poly_scalar_mul() {
    // (1/x) * (θ + 1) = θ/x + 1/x
    let inv_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let p = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let scaled = p.scalar_mul(&inv_x);
    assert_eq!(scaled.coeff(0), inv_x);
    assert_eq!(scaled.coeff(1), inv_x);
}
```

**Step 2: Implement arithmetic**

Implement `Add`, `Sub`, `Mul` for `&ExtPoly`, and `scalar_mul(&self, &RationalFunction)`.
- Add/Sub: coefficientwise, same as Polynomial but with RationalFunction coefficients
- Mul: schoolbook O(n*m) convolution
- ScalarMul: multiply each coefficient

Also implement `Neg` for `&ExtPoly`.

**Step 3: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/ext_poly.rs
git commit -m "Add ExtPoly arithmetic: add, sub, mul, neg, scalar_mul"
```

---

### Task 6: ExtPoly Division and GCD

**Files:**
- Modify: `src/ext_poly.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_ext_poly_div_rem_exact() {
    // (θ^2 + 2θ + 1) / (θ + 1) = (θ + 1), remainder 0
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let (q, r) = a.div_rem(&b).unwrap();
    assert_eq!(q.coeff(0), rf_const(1));
    assert_eq!(q.coeff(1), rf_const(1));
    assert!(r.is_zero());
}

#[test]
fn test_ext_poly_div_rem_with_remainder() {
    // (θ^2 + 1) / (θ + 1) = (θ - 1), remainder 2
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let (q, r) = a.div_rem(&b).unwrap();
    assert_eq!(q.coeff(0), rf_const(-1));
    assert_eq!(q.coeff(1), rf_const(1));
    assert_eq!(r.coeff(0), rf_const(2));
}

#[test]
fn test_ext_poly_gcd() {
    // gcd(θ^2 - 1, θ^2 + 2θ + 1) = θ + 1
    let a = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2), rf_const(1)], "x");
    let g = a.gcd(&b);
    assert_eq!(g.degree(), Some(1));
}

#[test]
fn test_ext_poly_gcd_coprime() {
    // gcd(θ + 1, θ + 2) = 1
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(2), rf_const(1)], "x");
    let g = a.gcd(&b);
    assert!(g.degree().unwrap_or(0) == 0);
}

#[test]
fn test_ext_poly_extended_gcd() {
    // s*(θ+1) + t*(θ-1) = gcd = 1 (they're coprime)
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let b = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(1)], "x");
    let (g, s, t) = ExtPoly::extended_gcd(&a, &b);
    assert!(g.degree().unwrap_or(0) == 0);
    // Verify: s*a + t*b = g
    let check = &(&s * &a) + &(&t * &b);
    assert_eq!(check.degree(), g.degree());
}
```

**Step 2: Implement div_rem, gcd, extended_gcd**

Same algorithms as `Polynomial` but using `RationalFunction` coefficient arithmetic:
- `div_rem`: polynomial long division, dividing leading coefficients via `checked_div`
- `gcd`: Euclidean algorithm via repeated `div_rem`
- `extended_gcd`: Extended Euclidean, returns `(gcd, s, t)` with `s*a + t*b = gcd`
- `make_monic`: divide all coefficients by leading coefficient
- `square_free_decomposition`: `gcd(f, f')` approach (requires derivative — see Task 7)

**Step 3: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/ext_poly.rs
git commit -m "Add ExtPoly division, GCD, and extended GCD"
```

---

### Task 7: DifferentialExtension and ExtPoly Derivative

**Files:**
- Create: `src/risch.rs`
- Modify: `src/ext_poly.rs` (add derivative method)
- Modify: `src/lib.rs`
- Test: inline tests in both files

**Step 1: Define the extension types**

```rust
// In src/risch.rs

use crate::polynomial::Polynomial;
use crate::rational_function::RationalFunction;
use crate::ext_poly::ExtPoly;

/// Type of transcendental extension
#[derive(Debug, Clone)]
pub enum ExtensionType {
    Logarithmic,  // θ = log(argument), so θ' = argument'/argument
    Exponential,  // θ = exp(argument), so θ' = argument' · θ
}

/// A single-level differential extension of Q(x).
/// Represents the field Q(x, θ) where θ = exp(f) or θ = log(f)
/// for some f ∈ Q(x).
#[derive(Debug, Clone)]
pub struct DifferentialExtension {
    ext_type: ExtensionType,
    argument: RationalFunction,  // f(x), the argument to exp or log
    var: String,                 // base variable name (e.g., "x")
}
```

**Step 2: Write failing tests for derivative in extension**

```rust
#[test]
fn test_derivative_log_extension() {
    // θ = log(x), θ' = 1/x
    // d/dx[θ] = 1/x (as an ExtPoly: coefficient of θ^0 is 1/x)
    // d/dx[θ^2] = 2θ · θ' = 2θ/x
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), // log(x)
        "x"
    );
    let theta = ExtPoly::theta("x");
    let d_theta = ext.differentiate(&theta);
    // Should be 1/x (a constant ExtPoly)
    assert_eq!(d_theta.degree(), Some(0));
}

#[test]
fn test_derivative_exp_extension() {
    // θ = exp(x), θ' = θ
    // d/dx[θ] = θ
    // d/dx[θ^2] = 2θ · θ' = 2θ^2
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), // exp(x)
        "x"
    );
    let theta = ExtPoly::theta("x");
    let d_theta = ext.differentiate(&theta);
    // Should be θ itself
    assert_eq!(d_theta.degree(), Some(1));
    assert_eq!(d_theta.coeff(1), rf_const(1));
}

#[test]
fn test_derivative_exp_x_squared() {
    // θ = exp(x^2), θ' = 2x · θ
    // d/dx[θ] = 2xθ
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 0, 1], "x")), // exp(x^2)
        "x"
    );
    let theta = ExtPoly::theta("x");
    let d_theta = ext.differentiate(&theta);
    assert_eq!(d_theta.degree(), Some(1));
    assert_eq!(d_theta.coeff(1), rf_poly(&[0, 2])); // 2x
}
```

**Step 3: Implement DifferentialExtension::differentiate**

The derivative of an ExtPoly Σ aᵢ(x)θⁱ depends on the extension type:

**Logarithmic** (θ = log(f), θ' = f'/f):
```
d/dx[Σ aᵢ θⁱ] = Σ [aᵢ' θⁱ + i · aᵢ · (f'/f) · θⁱ⁻¹]
```

**Exponential** (θ = exp(f), θ' = f'·θ):
```
d/dx[Σ aᵢ θⁱ] = Σ [aᵢ' θⁱ + i · aᵢ · f' · θⁱ]
                = Σ [(aᵢ' + i · f' · aᵢ) θⁱ]
```

**Step 4: Add module to lib.rs, run tests, clippy, commit**

```rust
pub mod risch;
```

```
cargo clippy --tests -- -D warnings && cargo test
git add src/risch.rs src/ext_poly.rs src/lib.rs
git commit -m "Add DifferentialExtension with derivative computation in log/exp towers"
```

---

### Task 8: Hermite Reduction

**Files:**
- Modify: `src/risch.rs`

This is the key subroutine. Given ∫ A/D where A, D are ExtPolys with deg(A) < deg(D), Hermite reduction splits it into:

∫ A/D = g + ∫ A*/D*

where g is a rational element (ExtPoly quotient) and D* is squarefree.

**Step 1: Write failing tests**

```rust
#[test]
fn test_hermite_reduction_already_squarefree() {
    // ∫ 1/(θ+1) — denominator is already squarefree, nothing to reduce
    let a = ExtPoly::from_rf(rf_const(1));
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = hermite_reduce(&a, &d, &ext);
    assert!(result.rational_part.is_zero());
    assert_eq!(result.integrand_num.degree(), Some(0)); // still 1
    assert_eq!(result.integrand_den.degree(), Some(1)); // still θ+1
}

#[test]
fn test_hermite_reduction_repeated_factor() {
    // ∫ 1/(θ+1)^2 — reduce the repeated factor
    // Should give: -1/(θ+1) + ∫ 0/(θ+1) ... but the log derivative term
    // needs the extension's derivative. For the polynomial-in-θ case
    // with a log extension, this depends on θ'.
    // For simplicity, test with θ = log(x):
    // ∫ 1/(θ+1)^2 dθ = -1/(θ+1) in the pure polynomial case
    // But we're integrating w.r.t. x, not θ.
    //
    // Test: for the base rational case (no extension, θ = x):
    // ∫ 1/(x+1)^2 dx = -1/(x+1)
    // Hermite reduction should give rational part = -1/(x+1), integrand = 0
    let a = ExtPoly::from_rf(rf_const(1));
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2), rf_const(1)], "x"); // (θ+1)^2
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let result = hermite_reduce(&a, &d, &ext);
    // The rational part should be nonzero (the -1/(θ+1) piece)
    assert!(!result.rational_part_num.is_zero() || result.integrand_num.is_zero());
}

#[test]
fn test_hermite_reduction_cubic_repeated() {
    // ∫ (2θ+1)/(θ+1)^3 — needs iterated reduction
    let a = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2)], "x");
    // (θ+1)^3 = θ^3 + 3θ^2 + 3θ + 1
    let d = ExtPoly::from_coeffs(
        vec![rf_const(1), rf_const(3), rf_const(3), rf_const(1)], "x"
    );
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let result = hermite_reduce(&a, &d, &ext);
    // After reduction, integrand denominator should be squarefree (degree 1)
    let sf_deg = result.integrand_den.degree().unwrap_or(0);
    assert!(sf_deg <= 1, "Denominator should be squarefree after Hermite reduction");
}
```

**Step 2: Implement Hermite reduction**

Algorithm (Mack's linear version, adapted from Bronstein §2.3):

```
Input: A, D ∈ k(x)[θ] with deg(A) < deg(D), D ≠ 0
Output: (g_num, g_den, A*, D*) such that
        ∫ A/D = g_num/g_den + ∫ A*/D*
        and D* is squarefree

1. Compute Dm = gcd(D, D') where D' is d/dθ (formal derivative w.r.t. θ)
2. Ds = D / Dm  (Ds is the squarefree part)
3. While deg(Dm) > 0:
   a. Dm2 = gcd(Dm, Dm')
   b. Ds2 = Dm / Dm2
   c. Use extended GCD: find B, C with B·Ds + C·(-Ds'·Dm2/Dm) = A_current
      (This is the key step — solving for the rational part contribution)
   d. Update rational part, update A_current
   e. Dm = Dm2, Ds = Ds2
4. Return (rational_part, A_reduced, Ds_final)
```

The exact formulation needs care. Implement iteratively.

Define a result struct:
```rust
pub struct HermiteResult {
    pub rational_part_num: ExtPoly,
    pub rational_part_den: ExtPoly,
    pub integrand_num: ExtPoly,
    pub integrand_den: ExtPoly,
}
```

**Step 3: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/risch.rs
git commit -m "Add Hermite reduction for ExtPoly: splits integral into rational + squarefree parts"
```

---

### Task 9: Square-Free Decomposition for ExtPoly

**Files:**
- Modify: `src/ext_poly.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_ext_poly_square_free_part() {
    // (θ+1)^2 → square-free part is (θ+1)
    let f = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(2), rf_const(1)], "x");
    let sf = f.square_free_part();
    assert_eq!(sf.degree(), Some(1));
}

#[test]
fn test_ext_poly_square_free_already() {
    // (θ^2 + 1) → already square-free
    let f = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x");
    let sf = f.square_free_part();
    assert_eq!(sf.degree(), Some(2));
}

#[test]
fn test_ext_poly_sfd() {
    // (θ+1)^2 (θ-1) = θ^3 + θ^2 - θ - 1
    let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let t_minus_1 = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(1)], "x");
    let f = &(&t_plus_1 * &t_plus_1) * &t_minus_1;
    let decomp = f.square_free_decomposition();
    assert_eq!(decomp.len(), 2);
}
```

**Step 2: Implement square_free_part and square_free_decomposition**

Port the algorithms from `Polynomial` to `ExtPoly`:
- `formal_derivative`: d/dθ, NOT the full d/dx — just differentiate as a polynomial in θ
- `square_free_part`: f / gcd(f, f')
- `square_free_decomposition`: full factorization into coprime square-free factors with multiplicities

Note: `formal_derivative` is w.r.t. θ (the tower variable), not x. This is just `Σ i·aᵢ·θ^{i-1}`.

**Step 3: Run tests, clippy, commit**

```
cargo clippy --tests -- -D warnings && cargo test
git add src/ext_poly.rs
git commit -m "Add ExtPoly square-free decomposition for Hermite reduction"
```

---

## Session Summary

After this session, we will have:

1. **`RationalFunction`** — complete arithmetic and derivative over Q(x)
2. **`ExtPoly`** — complete polynomial arithmetic in Q(x)[θ], including GCD and SFD
3. **`DifferentialExtension`** — representation of log/exp extensions with derivative computation
4. **Hermite reduction** — splits ∫ A/D into rational part + squarefree integral

This is the algebraic foundation. Session 2 will build on this to implement the actual integration algorithms for logarithmic and exponential extensions, and the "no elementary antiderivative" detection.

**Estimated test count:** ~30-35 new tests across the three files.
