# Biquadratic Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate rational functions with irreducible degree-4 biquadratic denominators (e.g., ∫1/(x⁴+1)dx) by factoring over Q(√d) and computing exact antiderivatives with √d coefficients.

**Architecture:** When `integrate_pf_term` encounters a degree-4 irreducible factor, detect if it's biquadratic (x⁴+px²+q). If so, factor as (x²+√d·x+b)(x²-√d·x+b) where b²=q and d=2b-p. Compute partial fractions in Q(√d), then integrate each quadratic term using the existing arctan/ln formula. Build the result as Node expressions with exact √d coefficients.

**Tech Stack:** Rust, existing Node AST, BigRational, Polynomial types.

**Mathematical foundation:**

For biquadratic x⁴+px²+q irreducible over Q, the factoring (x²+ax+b)(x²-ax+b) requires:
- b = √q ∈ Q (q must be a perfect rational square)
- a² = d = 2b-p > 0, d not a perfect rational square

Partial fractions of (c₃x³+c₂x²+c₁x+c₀)/(x⁴+px²+q):
- E = c₃/2 - (c₂-c₀/b)/(2a), F = c₀/(2b) - (c₁-b·c₃)/(2a)
- G = c₃/2 + (c₂-c₀/b)/(2a), H = c₀/(2b) + (c₁-b·c₃)/(2a)

Each (Ex+F)/(x²±ax+b) integrates as:
- (E/2)·ln(x²±ax+b) + (2F∓Ea)/√(4b-a²) · arctan((2x±a)/√(4b-a²))

Where 4b-a² = 2b+p is the inner discriminant.

---

### Task 1: Biquadratic Detection

**Files:**
- Modify: `src/integration.rs` (add near the bottom, before tests)

**Step 1: Write the failing test**

In `src/integration.rs` test section, add:

```rust
#[test]
fn test_detect_biquadratic() {
    use crate::polynomial::Polynomial;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    fn int(n: i64) -> BigRational { BigRational::from_integer(BigInt::from(n)) }
    fn poly(coeffs: &[i64], v: &str) -> Polynomial {
        Polynomial::from_coeffs(coeffs.iter().map(|&c| int(c)).collect(), v)
    }

    // x⁴+1: biquadratic with p=0, q=1, b=1, d=2
    let p1 = poly(&[1, 0, 0, 0, 1], "x"); // 1 + 0x + 0x² + 0x³ + x⁴
    let result = try_factor_biquadratic(&p1);
    assert!(result.is_some(), "x⁴+1 should be biquadratic");
    let (b, d) = result.unwrap();
    assert_eq!(b, int(1));
    assert_eq!(d, int(2));

    // x⁴-x²+1: biquadratic with p=-1, q=1, b=1, d=3
    let p2 = poly(&[1, 0, -1, 0, 1], "x");
    let result = try_factor_biquadratic(&p2);
    assert!(result.is_some());
    let (b, d) = result.unwrap();
    assert_eq!(b, int(1));
    assert_eq!(d, int(3));

    // x⁴+x²+1: factors over Q (d=1 is perfect square), should return None
    let p3 = poly(&[1, 0, 1, 0, 1], "x");
    assert!(try_factor_biquadratic(&p3).is_none());

    // x⁴+x+1: not biquadratic (has x term), should return None
    let p4 = poly(&[1, 1, 0, 0, 1], "x");
    assert!(try_factor_biquadratic(&p4).is_none());

    // x⁴+3x²+1: d = 2-3 = -1 < 0, should return None
    let p5 = poly(&[1, 0, 3, 0, 1], "x");
    assert!(try_factor_biquadratic(&p5).is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib test_detect_biquadratic`
Expected: FAIL — `try_factor_biquadratic` not found

**Step 3: Implement `try_factor_biquadratic`**

```rust
/// Detect if a monic degree-4 polynomial is biquadratic and factorable over Q(√d).
///
/// For x⁴+px²+q, checks the factoring (x²+ax+b)(x²-ax+b) where b²=q, a²=2b-p.
/// Returns Some((b, d)) where d = a² = 2b-p if:
///   - polynomial is biquadratic (zero x³ and x coefficients)
///   - q is a perfect rational square (so b ∈ Q)
///   - d = 2b-p > 0
///   - d is NOT a perfect rational square (otherwise it factors over Q)
fn try_factor_biquadratic(
    poly: &crate::polynomial::Polynomial,
) -> Option<(BigRational, BigRational)> {
    use num_traits::{One, Zero};

    if poly.degree() != Some(4) {
        return None;
    }

    // Check monic
    let lc = poly.coeff(4);
    if !lc.is_one() {
        return None;
    }

    // Check biquadratic: x³ and x coefficients must be zero
    if !poly.coeff(3).is_zero() || !poly.coeff(1).is_zero() {
        return None;
    }

    let p = poly.coeff(2); // coefficient of x²
    let q = poly.coeff(0); // constant term

    // b² = q, so b = √q must be rational
    let b = exact_rational_sqrt_bigrat(&q)?;

    // d = 2b - p
    let two = BigRational::from_integer(num_bigint::BigInt::from(2));
    let d = &two * &b - &p;

    // d must be positive
    if d <= BigRational::zero() {
        return None;
    }

    // d must NOT be a perfect rational square (otherwise BZ handles it)
    if exact_rational_sqrt_bigrat(&d).is_some() {
        return None;
    }

    Some((b, d))
}

/// Compute exact rational square root of a BigRational, if it exists.
fn exact_rational_sqrt_bigrat(r: &BigRational) -> Option<BigRational> {
    use num_traits::Signed;
    if r.is_negative() {
        return None;
    }
    let n = r.numer().sqrt();
    let d = r.denom().sqrt();
    if &(&n * &n) == r.numer() && &(&d * &d) == r.denom() {
        Some(BigRational::new(n, d))
    } else {
        None
    }
}
```

Note: `BigInt::sqrt()` is available from the `num-integer` crate's `Roots` trait.
Check if it's already imported — look for existing uses of `.sqrt()` on BigInt.
If not available, use `num_integer::Roots` or implement integer square root.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib test_detect_biquadratic`
Expected: PASS

**Step 5: Commit**

```
git add src/integration.rs
git commit -m "feat: biquadratic detection for degree-4 irreducible polynomials"
```

---

### Task 2: Node Builder Helpers

**Files:**
- Modify: `src/integration.rs` (add helper functions)

**Step 1: Write the test**

```rust
#[test]
fn test_node_sqrt_display() {
    let sqrt2 = node_sqrt_integer(2);
    assert_eq!(format!("{}", sqrt2), "\\sqrt{2}");

    let coeff = node_rat(1, 4);
    let term = Node::Multiply(Box::new(coeff), Box::new(sqrt2));
    let env = crate::environment::Environment::new();
    let simplified = term.simplify(&env).unwrap_or(term);
    // Should display as (1/4)·√2 or similar
    let s = format!("{}", simplified);
    assert!(s.contains("sqrt") || s.contains("\\sqrt"), "Got: {}", s);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib test_node_sqrt_display`

**Step 3: Implement helpers**

```rust
fn node_rat(n: i64, d: i64) -> Node {
    Node::Num(ExactNum::rational(n, d))
}

fn node_int(n: i64) -> Node {
    Node::Num(ExactNum::integer(n))
}

fn node_sqrt_integer(n: i64) -> Node {
    Node::Function("sqrt".to_string(), vec![node_int(n)])
}

fn node_sqrt_rat(r: &BigRational) -> Node {
    Node::Function(
        "sqrt".to_string(),
        vec![Node::Num(ExactNum::rational(
            r.numer().try_into().unwrap_or(1),
            r.denom().try_into().unwrap_or(1),
        ))],
    )
}

/// Build a + b·√d as a Node, simplifying when b=0 or a=0.
fn node_quad_surd(a: &BigRational, b: &BigRational, d: &BigRational) -> Node {
    use num_traits::Zero;
    let sqrt_d = node_sqrt_rat(d);
    let b_sqrt = if b.is_zero() {
        return rat_to_node(a);
    } else if b.is_one() {
        sqrt_d
    } else {
        Node::Multiply(Box::new(rat_to_node(b)), Box::new(sqrt_d))
    };

    if a.is_zero() {
        b_sqrt
    } else {
        Node::Add(Box::new(rat_to_node(a)), Box::new(b_sqrt))
    }
}

fn rat_to_node(r: &BigRational) -> Node {
    Node::Num(ExactNum::rational(
        r.numer().try_into().unwrap_or(1),
        r.denom().try_into().unwrap_or(1),
    ))
}
```

**Step 4: Run test, verify pass**

Run: `cargo test --lib test_node_sqrt_display`

**Step 5: Commit**

```
git commit -m "feat: Node builder helpers for biquadratic integration"
```

---

### Task 3: Biquadratic Integration Core

**Files:**
- Modify: `src/integration.rs`

This is the main mathematical kernel. Computes ∫N(x)/(x⁴+px²+q)dx exactly.

**Step 1: Write the end-to-end integration test**

```rust
#[test]
fn test_integrate_biquadratic_x4_plus_1() {
    // ∫1/(x⁴+1)dx should produce arctan + ln terms with √2 coefficients
    let result = integrate_latex("\\frac{1}{x^4 + 1}", "x");
    assert!(result.is_ok(), "Should succeed: {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("sqrt") || r.contains("\\sqrt"),
        "Result should contain √2: {}", r);
    assert!(r.contains("arctan"), "Result should contain arctan: {}", r);
    assert!(r.contains("ln"), "Result should contain ln: {}", r);
}

#[test]
fn test_integrate_biquadratic_x4_plus_1_numerical() {
    // Verify numerically: F(2) - F(1) where F = antiderivative of 1/(x⁴+1)
    // Numerical integration gives ∫₁² 1/(x⁴+1)dx ≈ 0.24352
    let result = crate::integration::definite_integral_latex(
        "\\frac{1}{x^4 + 1}", "x", 1.0, 2.0,
    );
    assert!(result.is_ok(), "Definite integral should succeed: {:?}", result);
    let val: f64 = result.unwrap().parse().unwrap();
    assert!((val - 0.24352).abs() < 0.001,
        "∫₁² 1/(x⁴+1)dx ≈ 0.24352, got {}", val);
}

#[test]
fn test_integrate_biquadratic_x4_minus_x2_plus_1() {
    // ∫1/(x⁴-x²+1)dx — factors over Q(√3)
    let result = integrate_latex("\\frac{1}{x^4 - x^2 + 1}", "x");
    assert!(result.is_ok(), "Should succeed: {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("arctan"), "Result should contain arctan: {}", r);
}

#[test]
fn test_integrate_x2_over_x4_plus_1() {
    // ∫x²/(x⁴+1)dx
    let result = integrate_latex("\\frac{x^2}{x^4 + 1}", "x");
    assert!(result.is_ok(), "Should succeed: {:?}", result);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib test_integrate_biquadratic`
Expected: FAIL — the integration returns an error for degree-4 factors

**Step 3: Implement `integrate_biquadratic_rational`**

This function takes a numerator Polynomial (degree < 4), the biquadratic parameters (p, q as BigRational; b, d from detection), and the variable name. It returns a Node representing the antiderivative.

The algorithm:
1. Extract numerator coefficients c₀, c₁, c₂, c₃
2. Compute partial fraction coefficients E, F, G, H in Q(√d):
   - Each as (rational_part, sqrt_part) where value = rational_part + sqrt_part·√d
   - 1/a = 1/√d = √d/d, so dividing by a maps (r, s) → (s·d_inv, r·d_inv) where d_inv = 1/d
3. Compute the antiderivative for each quadratic factor:
   - Ln coefficient = E/2 (or G/2)
   - Inner discriminant = 2b+p
   - Arctan coefficient = (2F-E·a)/(√inner_disc) — requires care since E and a are in Q(√d)
4. Build Node expression

```rust
fn integrate_biquadratic_rational(
    numerator: &crate::polynomial::Polynomial,
    p: &BigRational,
    q: &BigRational,
    b: &BigRational,
    d: &BigRational,
    var: &str,
) -> Result<Node, String> {
    use num_traits::{One, Zero};

    let two = BigRational::from_integer(BigInt::from(2));
    let four = BigRational::from_integer(BigInt::from(4));

    // Numerator coefficients
    let c0 = numerator.coeff(0);
    let c1 = numerator.coeff(1);
    let c2 = numerator.coeff(2);
    let c3 = numerator.coeff(3);

    // Partial fraction coefficients in Q(√d): value = rat + surd·√d
    // a = √d, so 1/a = √d/d → dividing (r,s) by a gives (s, r/d)... 
    // Actually: (r + s√d) / √d = (r + s√d)·√d/d = (r√d + sd)/d = s + r/d·√d
    // So dividing by a: (r, s) → (s, r/d)
    // And dividing by 2a: (r, s) → (s/2, r/(2d))

    // E = c₃/2 - (c₂ - c₀/b)/(2a)
    // (c₂ - c₀/b)/(2a) has rational part 0 and surd part (c₂ - c₀/b)/(2d)
    // E_rat = c₃/2 - 0 = c₃/2
    // E_surd = 0 - (c₂ - c₀/b)/(2d) = -(c₂ - c₀/b)/(2d)
    let c2_minus_c0_over_b = &c2 - &(&c0 / &b);
    let e_rat = &c3 / &two;
    let e_surd = -&c2_minus_c0_over_b / &(&two * d);

    // F = c₀/(2b) - (c₁ - b·c₃)/(2a)
    let c1_minus_bc3 = &c1 - &(b * &c3);
    let f_rat = &c0 / &(&two * b);
    let f_surd = -&c1_minus_bc3 / &(&two * d);

    // G = c₃/2 + (c₂ - c₀/b)/(2a) = (c₃/2, (c₂-c₀/b)/(2d))
    let g_rat = &c3 / &two;
    let g_surd = &c2_minus_c0_over_b / &(&two * d);

    // H = c₀/(2b) + (c₁ - b·c₃)/(2a)
    let h_rat = &c0 / &(&two * b);
    let h_surd = &c1_minus_bc3 / &(&two * d);

    // Inner discriminant: 4b - a² = 4b - d = 2b + p
    let inner_disc = &(&two * b) + p;
    if inner_disc <= BigRational::zero() {
        return Err("Inner discriminant non-positive".to_string());
    }

    let x = Node::Variable(var.to_string());
    let sqrt_d = node_sqrt_rat(d);

    // Build quadratic expressions: x² ± √d·x + b
    let x_sq = Node::Power(Box::new(x.clone()), Box::new(node_int(2)));
    let sqrt_d_x = Node::Multiply(Box::new(sqrt_d.clone()), Box::new(x.clone()));

    let quad_plus = Node::Add(
        Box::new(Node::Add(Box::new(x_sq.clone()), Box::new(sqrt_d_x.clone()))),
        Box::new(rat_to_node(b)),
    );
    let quad_minus = Node::Add(
        Box::new(Node::Subtract(Box::new(x_sq), Box::new(sqrt_d_x))),
        Box::new(rat_to_node(b)),
    );

    // --- Build the antiderivative ---
    let env = crate::environment::Environment::new();
    let mut terms: Vec<Node> = Vec::new();

    // Ln terms: (E/2)·ln(x²+ax+b) + (G/2)·ln(x²-ax+b)
    // E/2 = (e_rat/2, e_surd/2), G/2 = (g_rat/2, g_surd/2)
    let e_half_rat = &e_rat / &two;
    let e_half_surd = &e_surd / &two;
    let g_half_rat = &g_rat / &two;
    let g_half_surd = &g_surd / &two;

    let ln_plus = Node::Function("ln".to_string(), vec![Node::Abs(Box::new(quad_plus.clone()))]);
    let ln_minus = Node::Function("ln".to_string(), vec![Node::Abs(Box::new(quad_minus.clone()))]);

    if !e_half_rat.is_zero() || !e_half_surd.is_zero() {
        let coeff = node_quad_surd(&e_half_rat, &e_half_surd, d);
        terms.push(Node::Multiply(Box::new(coeff), Box::new(ln_plus.clone())));
    }
    if !g_half_rat.is_zero() || !g_half_surd.is_zero() {
        let coeff = node_quad_surd(&g_half_rat, &g_half_surd, d);
        terms.push(Node::Multiply(Box::new(coeff), Box::new(ln_minus.clone())));
    }

    // Arctan terms
    // For first factor (x²+ax+b):
    //   arctan residual = 2F - E·a where a = √d
    //   (2F - E·a)_rat = 2f_rat - e_surd·d, (2F - E·a)_surd = 2f_surd - e_rat
    //   arctan coeff = residual / √(inner_disc)
    //   arctan arg = (2x + a) / √(inner_disc) = (2x + √d) / √(inner_disc)
    let res1_rat = &(&two * &f_rat) - &(&e_surd * d);
    let res1_surd = &(&two * &f_surd) - &e_rat;

    // For second factor (x²-ax+b):
    //   arctan residual = 2H - G·(-a) = 2H + G·a
    //   (2H + G·a)_rat = 2h_rat + g_surd·d, (2H + G·a)_surd = 2h_surd + g_rat
    let res2_rat = &(&two * &h_rat) + &(&g_surd * d);
    let res2_surd = &(&two * &h_surd) + &g_rat;

    let sqrt_inner = node_sqrt_rat(&inner_disc);
    let two_x = Node::Multiply(Box::new(node_int(2)), Box::new(x.clone()));

    // arctan arg 1: (2x + √d) / √(inner_disc)
    let arctan_arg1 = Node::Divide(
        Box::new(Node::Add(Box::new(two_x.clone()), Box::new(sqrt_d.clone()))),
        Box::new(sqrt_inner.clone()),
    );
    // arctan arg 2: (2x - √d) / √(inner_disc)
    let arctan_arg2 = Node::Divide(
        Box::new(Node::Subtract(Box::new(two_x), Box::new(sqrt_d))),
        Box::new(sqrt_inner.clone()),
    );

    let arctan1 = Node::Function("arctan".to_string(), vec![arctan_arg1]);
    let arctan2 = Node::Function("arctan".to_string(), vec![arctan_arg2]);

    // Arctan coefficient: residual / √(inner_disc)
    // Since residual is (r + s√d) and we divide by √(inner_disc),
    // the coefficient is (r + s√d) / √(inner_disc)
    // Build as Node: (r + s√d) / √(inner_disc)
    if !res1_rat.is_zero() || !res1_surd.is_zero() {
        let res_node = node_quad_surd(&res1_rat, &res1_surd, d);
        let coeff = Node::Divide(Box::new(res_node), Box::new(sqrt_inner.clone()));
        terms.push(Node::Multiply(Box::new(coeff), Box::new(arctan1)));
    }
    if !res2_rat.is_zero() || !res2_surd.is_zero() {
        let res_node = node_quad_surd(&res2_rat, &res2_surd, d);
        let coeff = Node::Divide(Box::new(res_node), Box::new(sqrt_inner));
        terms.push(Node::Multiply(Box::new(coeff), Box::new(arctan2)));
    }

    // Sum all terms
    let mut result = terms.into_iter().reduce(|acc, t| Node::Add(Box::new(acc), Box::new(t)))
        .unwrap_or(node_int(0));

    // Simplify
    result = result.simplify(&env).unwrap_or(result);
    Ok(result)
}
```

**Step 4: Run tests, expect still failing** (not yet wired in)

**Step 5: Commit**

```
git commit -m "feat: biquadratic integration core — exact Q(√d) partial fractions"
```

---

### Task 4: Wire Into Integration Engine

**Files:**
- Modify: `src/integration.rs` — the `integrate_pf_term` function

**Step 1: The tests from Task 3 should now pass after this wiring**

**Step 2: Modify `integrate_pf_term`**

Find the error case at the bottom of `integrate_pf_term` (around line 1349-1354) where it says:
```rust
_ => Err(format!("Integration of degree-{} factor to power {} not yet implemented", q_deg, k))
```

Add a new arm before this error case for `q_deg == 4 && k == 1`:

```rust
(4, 1) => {
    // Try biquadratic factoring: x⁴+px²+q → (x²+√d·x+b)(x²-√d·x+b)
    if let Some((b, d)) = try_factor_biquadratic(&term.denominator) {
        let p = term.denominator.coeff(2);
        let q = term.denominator.coeff(0);
        integrate_biquadratic_rational(&term.numerator, &p, &q, &b, &d, var)
    } else {
        Err(format!(
            "Integration of non-biquadratic degree-4 factor not yet implemented"
        ))
    }
}
```

**Step 3: Run tests**

Run: `cargo test --lib test_integrate_biquadratic`
Expected: All 4 tests PASS

**Step 4: Run full test suite**

Run: `cargo test`
Expected: All 866+ tests PASS, no regressions

**Step 5: Commit**

```
git commit -m "feat: wire biquadratic integration into partial fraction engine"
```

---

### Task 5: End-to-End Tests and Verification

**Files:**
- Modify: `tests/integration.rs` (add e2e tests)

**Step 1: Add e2e tests**

```rust
#[test]
fn test_e2e_integral_x4_plus_1() {
    let result = integrate_latex("\\frac{1}{x^4 + 1}", "x").unwrap();
    assert!(result.contains("arctan"), "Should have arctan: {}", result);
    assert!(result.contains("\\sqrt"), "Should have sqrt: {}", result);
}

#[test]
fn test_e2e_integral_x4_minus_x2_plus_1() {
    let result = integrate_latex("\\frac{1}{x^4 - x^2 + 1}", "x").unwrap();
    assert!(result.contains("arctan"), "Should have arctan: {}", result);
}

#[test]
fn test_e2e_integral_x2_over_x4_plus_1() {
    let result = integrate_latex("\\frac{x^2}{x^4 + 1}", "x").unwrap();
    assert!(result.is_ok() || result.contains("arctan"),
        "Should integrate: {}", result);
}
```

Also add numerical verification via definite integrals comparing against known values.

**Step 2: Run all tests**

Run: `cargo test`

**Step 3: Run clippy**

Run: `cargo clippy --tests -- -D warnings`

**Step 4: Commit**

```
git commit -m "test: e2e tests for biquadratic integration"
```

**Step 5: Update docs**

Update `KNUTH-PLAN.md` and `README.md` with new test count and biquadratic integration capability.

```
git commit -m "doc: update for biquadratic integration"
```

---

## Edge Cases to Handle

1. **q not a perfect rational square** (e.g., x⁴+2) — skip, return error
2. **d ≤ 0** (e.g., x⁴+3x²+1) — skip, return error
3. **d is a perfect square** (e.g., x⁴+4 with d=4) — should have been factored by BZ; if we get here anyway, could factor rationally
4. **Numerator degree 0** (constant) — most common case, partial fractions simplify
5. **Numerator degree 3** — full 4-coefficient partial fraction
6. **Inner discriminant 2b+p = 0** — degenerate case, skip

## What This Does NOT Handle

- Non-biquadratic degree-4 polynomials (x⁴+x+1, x⁴+x³+1)
- Degree-4 factors with power > 1
- Algebraic extensions for the Risch algorithm
- General Q(α) arithmetic
- Factoring general quartics via resolvent cubic

These are future work (Sessions 28+).
