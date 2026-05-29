# Rational Function GCD Simplification & Factored Display

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the simplifier cancel common rational coefficient factors in fractions and display results in factored form.

**Architecture:** Two changes to `try_univariate_divide` and `try_multivariate_divide` in `src/simplify.rs`: (1) when the polynomial GCD is trivial (degree 0), fall through to content simplification instead of returning `None`, and (2) after any simplification, try factoring numerator/denominator via `factor_over_q` for display. A helper `rational_gcd` computes GCD of two `BigRational` values. A helper `try_factored_display` converts a polynomial to factored Node form with integer-coefficient factors.

**Tech Stack:** Rust, `num-bigint`/`num-rational` crates, existing `Polynomial`/`MultiPoly` types, existing `factor_over_q` in `src/mod_poly.rs`.

**Key files:**
- `src/simplify.rs:795-882` — `try_polynomial_divide`, `try_univariate_divide`, `try_multivariate_divide`
- `src/polynomial.rs:140-166` — `content()`, `primitive_part()`, `scalar_mul()`
- `src/polynomial.rs:619-636` — `gcd_bigint`, `lcm_bigint`
- `src/mod_poly.rs:688-727` — `factor_over_q`
- `tests/simplify.rs` — simplifier unit tests

---

### Task 1: Failing tests for content simplification

**Files:**
- Modify: `tests/simplify.rs`

**Step 1: Write the failing tests**

Add these tests at the end of `mod test_simplify` in `tests/simplify.rs`:

```rust
#[test]
fn test_content_gcd_simplification() {
    // (-32a+32)/(16a+8) → (-4a+4)/(2a+1) — Alex's primary example
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{-32a+32}{16a+8}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("a", 0.435);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    let expected = (-32.0 * 0.435 + 32.0) / (16.0 * 0.435 + 8.0);
    assert!((val - expected).abs() < 1e-10, "Value mismatch: {} vs {}", val, expected);
    // Check that coefficients were reduced
    let s = format!("{}", simplified);
    assert!(!s.contains("32"), "Should have reduced coefficients, got: {}", s);
    assert!(!s.contains("16"), "Should have reduced coefficients, got: {}", s);
}

#[test]
fn test_content_gcd_constant_numerator() {
    // 48/(16a³+24a²+12a+2) — content GCD with polynomial denominator
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{48}{16a^3+24a^2+12a+2}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("a", 0.5);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    let expected = 48.0 / (16.0 * 0.125 + 24.0 * 0.25 + 12.0 * 0.5 + 2.0);
    assert!((val - expected).abs() < 1e-10, "Value mismatch: {} vs {}", val, expected);
    let s = format!("{}", simplified);
    assert!(!s.contains("48"), "Should have reduced 48, got: {}", s);
}

#[test]
fn test_content_gcd_constant_denominator() {
    // (6a+3)/3 → 2a+1
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{6a+3}{3}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    assert!(!s.contains("\\frac"), "Should reduce to polynomial, got: {}", s);
    let mut test_env = Environment::new();
    test_env.set("a", 2.0);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    assert_eq!(val, 5.0, "(6*2+3)/3 = 5");
}

#[test]
fn test_content_gcd_both_reduce() {
    // (6a²-6)/(4a+4) → content + poly GCD → 3(a-1)/2
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{6a^2-6}{4a+4}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("a", 3.0);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    let expected = (6.0 * 9.0 - 6.0) / (4.0 * 3.0 + 4.0);
    assert!((val - expected).abs() < 1e-10, "Value: {} vs {}", val, expected);
    let s = format!("{}", simplified);
    assert!(!s.contains("6"), "Should have simplified coefficients, got: {}", s);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test simplify -- test_content_gcd 2>&1`
Expected: 4 failures (content GCD not implemented yet)

**Step 3: Commit**

```bash
git add tests/simplify.rs
git commit -m "test: failing tests for rational content GCD simplification"
```

---

### Task 2: Implement content simplification

**Files:**
- Modify: `src/simplify.rs:795-850`
- Modify: `src/polynomial.rs` (add `rational_gcd` helper)

**Step 1: Add `rational_gcd` helper to `src/polynomial.rs`**

Add after `lcm_bigint` (after line 636):

```rust
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
```

**Step 2: Rewrite `try_univariate_divide` in `src/simplify.rs`**

Replace the entire `try_univariate_divide` function (lines 812-850) with:

```rust
fn try_univariate_divide(numer: &Node, denom: &Node, var: &str) -> Option<Node> {
    use crate::polynomial::rational_gcd;

    let n = Polynomial::from_node(numer, var).ok()?;
    let d = Polynomial::from_node(denom, var).ok()?;

    if d.is_zero() {
        return None;
    }

    // Step 1: Polynomial GCD cancellation
    let g = n.gcd(&d);
    let (n_red, d_red) = if g.degree().unwrap_or(0) > 0 {
        let (nr, nr_rem) = n.div_rem(&g).ok()?;
        let (dr, dr_rem) = d.div_rem(&g).ok()?;
        if !nr_rem.is_zero() || !dr_rem.is_zero() {
            return None;
        }
        (nr, dr)
    } else {
        (n.clone(), d.clone())
    };

    // Step 2: Content simplification — cancel GCD of rational coefficients
    let c_n = n_red.content();
    let c_d = d_red.content();
    let c_gcd = rational_gcd(&c_n, &c_d);

    let (n_final, d_final) = if !c_gcd.is_one() {
        let inv = BigRational::one() / &c_gcd;
        (n_red.scalar_mul(&inv), d_red.scalar_mul(&inv))
    } else {
        (n_red, d_red)
    };

    // Step 3: Check if anything changed
    if n_final == n && d_final == d {
        return None;
    }

    // Step 4: Build result
    if d_final.is_constant() {
        let d_val = d_final.coeff(0);
        if d_val == BigRational::from_integer(BigInt::from(1)) {
            return Some(n_final.to_node());
        }
        return Some(
            n_final
                .scalar_mul(
                    &(BigRational::from_integer(BigInt::from(1)) / d_val),
                )
                .to_node(),
        );
    }

    Some(Node::Divide(
        Box::new(n_final.to_node()),
        Box::new(d_final.to_node()),
    ))
}
```

**Step 3: Run tests**

Run: `cargo test --test simplify -- test_content_gcd 2>&1`
Expected: `test_content_gcd_simplification`, `test_content_gcd_constant_denominator`, and `test_content_gcd_both_reduce` should PASS. `test_content_gcd_constant_numerator` may pass or may need factored display (Task 4).

**Step 4: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: All 877+ tests pass, no regressions.

**Step 5: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1`
Expected: 0 warnings.

**Step 6: Commit**

```bash
git add src/simplify.rs src/polynomial.rs
git commit -m "feat: rational content GCD simplification in fractions"
```

---

### Task 3: Failing tests for factored display

**Files:**
- Modify: `tests/simplify.rs`

**Step 1: Write failing tests**

Add to `mod test_simplify`:

```rust
#[test]
fn test_factored_display_cubed() {
    // 48/(16a³+24a²+12a+2) → 24/(2a+1)³
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{48}{16a^3+24a^2+12a+2}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    assert!(s.contains("(2a + 1)") || s.contains("(1 + 2a)"),
        "Should have factored denominator as (2a+1), got: {}", s);
}

#[test]
fn test_factored_display_squared() {
    // 1/(a²+2a+1) → 1/(a+1)²
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{1}{a^2+2a+1}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    assert!(s.contains("(a + 1)") || s.contains("(1 + a)"),
        "Should have factored denominator as (a+1)², got: {}", s);
}

#[test]
fn test_factored_display_two_distinct_factors() {
    // 1/(a²-1) → 1/((a-1)(a+1))
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{1}{a^2-1}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    // Should contain factored form
    let mut test_env = Environment::new();
    test_env.set("a", 3.0);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    assert!((val - 0.125).abs() < 1e-10, "1/(9-1) = 0.125, got {}", val);
}

#[test]
fn test_factored_display_irreducible_unchanged() {
    // 1/(a²+a+1) — irreducible over Q, should stay expanded
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{1}{a^2+a+1}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("a", 2.0);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    let expected = 1.0 / 7.0;
    assert!((val - expected).abs() < 1e-10);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test simplify -- test_factored_display 2>&1`
Expected: `test_factored_display_cubed` and `test_factored_display_squared` should fail. The others may pass (correctness, not display).

**Step 3: Commit**

```bash
git add tests/simplify.rs
git commit -m "test: failing tests for factored display of simplified fractions"
```

---

### Task 4: Implement factored display

**Files:**
- Modify: `src/simplify.rs` (add `try_factored_display` helper, wire into `try_univariate_divide`)

**Step 1: Add `try_factored_display` function**

Add before `try_polynomial_divide` in `src/simplify.rs` (around line 785):

```rust
/// Try to display a polynomial in factored form with integer coefficients.
/// Returns None if the polynomial is degree ≤ 1 or irreducible (factoring doesn't help).
fn try_factored_display(poly: &Polynomial) -> Option<Node> {
    use crate::mod_poly::factor_over_q;
    use crate::polynomial::{gcd_bigint, lcm_bigint};

    let deg = poly.degree()?;
    if deg <= 1 {
        return None;
    }

    let (content, factors) = factor_over_q(poly);

    // Only use factored form if there are multiple factors (distinct or repeated)
    if factors.len() <= 1 {
        return None;
    }

    // Convert monic factors to integer-coefficient form, grouping by equality
    let mut grouped: Vec<(Polynomial, usize)> = Vec::new();
    let mut adjusted_content = content;

    for f in &factors {
        // Find LCM of coefficient denominators to clear fractions
        let mut lcm = BigInt::one();
        for i in 0..=f.degree().unwrap_or(0) {
            let c = f.coeff(i);
            lcm = lcm_bigint(&lcm, c.denom());
        }
        let scale = BigRational::from_integer(lcm.clone());
        let f_int = f.scalar_mul(&scale);
        adjusted_content = adjusted_content / &scale;

        if let Some(entry) = grouped.iter_mut().find(|(p, _)| {
            p.degree() == f_int.degree()
                && (0..=p.degree().unwrap_or(0)).all(|i| p.coeff(i) == f_int.coeff(i))
        }) {
            entry.1 += 1;
        } else {
            grouped.push((f_int, 1));
        }
    }

    // Build the factored node
    let mut parts: Vec<Node> = Vec::new();

    // Add content coefficient if not 1
    if !adjusted_content.is_one() {
        parts.push(rational_to_node(&adjusted_content));
    }

    for (factor, mult) in &grouped {
        let factor_node = if factor.degree().unwrap_or(0) >= 2 {
            // Wrap multi-term factors in parentheses (achieved by the display layer)
            factor.to_node()
        } else {
            factor.to_node()
        };
        let term = if *mult > 1 {
            Node::Power(
                Box::new(factor_node),
                Box::new(Node::Num(ExactNum::integer(*mult as i64))),
            )
        } else {
            factor_node
        };
        parts.push(term);
    }

    if parts.is_empty() {
        return Some(Node::Num(ExactNum::one()));
    }

    let mut result = parts.remove(0);
    for part in parts {
        result = Node::Multiply(Box::new(result), Box::new(part));
    }

    Some(result)
}
```

**Step 2: Wire factored display into `try_univariate_divide`**

In the result-building section of `try_univariate_divide` (Step 4 from Task 2), replace the final `Some(Node::Divide(...))` with:

```rust
    // Step 4: Build result with factored display
    if d_final.is_constant() {
        let d_val = d_final.coeff(0);
        if d_val == BigRational::from_integer(BigInt::from(1)) {
            return Some(n_final.to_node());
        }
        return Some(
            n_final
                .scalar_mul(
                    &(BigRational::from_integer(BigInt::from(1)) / d_val),
                )
                .to_node(),
        );
    }

    let denom_node = try_factored_display(&d_final)
        .unwrap_or_else(|| d_final.to_node());
    let numer_node = try_factored_display(&n_final)
        .unwrap_or_else(|| n_final.to_node());

    Some(Node::Divide(
        Box::new(numer_node),
        Box::new(denom_node),
    ))
```

**Step 3: Run tests**

Run: `cargo test --test simplify -- test_factored_display 2>&1`
Expected: All factored display tests pass.

**Step 4: Run full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass, no regressions.

**Step 5: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1`
Expected: 0 warnings.

**Step 6: Commit**

```bash
git add src/simplify.rs
git commit -m "feat: factored display for simplified rational expressions"
```

---

### Task 5: Content simplification for multivariate

**Files:**
- Modify: `src/multipoly.rs` (add `rational_content` method)
- Modify: `src/simplify.rs:852-882` (`try_multivariate_divide`)

**Step 1: Write failing test**

Add to `tests/simplify.rs`:

```rust
#[test]
fn test_multivariate_content_simplification() {
    // (6xy + 6x) / (3y + 3) → 2x (content GCD = 3, then poly GCD = y+1)
    let env = Environment::new();
    let x = Node::Variable("x".to_string());
    let y = Node::Variable("y".to_string());
    // numerator: 6xy + 6x
    let numer = Node::Add(
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(6))),
            Box::new(Node::Multiply(Box::new(x.clone()), Box::new(y.clone()))),
        )),
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(6))),
            Box::new(x.clone()),
        )),
    );
    // denominator: 3y + 3
    let denom = Node::Add(
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(3))),
            Box::new(y.clone()),
        )),
        Box::new(Node::Num(ExactNum::integer(3))),
    );
    let expr = Node::Divide(Box::new(numer), Box::new(denom));
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    assert!(s.contains("2x") || s.contains("2 \\cdot x"),
        "Should simplify to 2x, got: {}", s);
}
```

**Step 2: Add `rational_content` to `MultiPoly`**

Add to `impl MultiPoly` in `src/multipoly.rs`:

```rust
/// Compute the rational (leaf-level) content: the GCD of all
/// BigRational constants at the leaves of this MultiPoly.
pub fn rational_content(&self) -> BigRational {
    use crate::polynomial::rational_gcd;
    match self {
        MultiPoly::Constant(c) => c.abs(),
        MultiPoly::Poly { coeffs, .. } => {
            let mut g = BigRational::zero();
            for c in coeffs {
                if c.is_zero() {
                    continue;
                }
                g = rational_gcd(&g, &c.rational_content());
                if g.is_one() {
                    return g;
                }
            }
            g
        }
    }
}

/// Divide all leaf-level constants by a rational scalar.
pub fn scalar_div_rational(&self, s: &BigRational) -> MultiPoly {
    if s.is_one() {
        return self.clone();
    }
    let inv = BigRational::one() / s;
    self.scalar_mul(&inv)
}
```

**Step 3: Update `try_multivariate_divide`**

Replace the `try_multivariate_divide` function in `src/simplify.rs` (lines 852-882):

```rust
fn try_multivariate_divide(numer: &Node, denom: &Node) -> Option<Node> {
    let n = MultiPoly::from_node(numer).ok()?;
    let d = MultiPoly::from_node(denom).ok()?;

    if d.is_zero() {
        return None;
    }

    // Step 1: Polynomial GCD cancellation
    let g = MultiPoly::gcd(&n, &d);
    let (n_red, d_red) = if !g.is_constant() {
        (n.exact_div(&g), d.exact_div(&g))
    } else {
        (n.clone(), d.clone())
    };

    // Step 2: Rational content simplification
    use crate::polynomial::rational_gcd;
    let c_n = n_red.rational_content();
    let c_d = d_red.rational_content();
    let c_gcd = rational_gcd(&c_n, &c_d);

    let (n_final, d_final) = if !c_gcd.is_one() {
        (n_red.scalar_div_rational(&c_gcd), d_red.scalar_div_rational(&c_gcd))
    } else {
        (n_red, d_red)
    };

    // Step 3: Check if anything changed
    if n_final == n && d_final == d {
        return None;
    }

    if d_final.is_one() {
        return Some(n_final.to_node());
    }
    if let Some(d_val) = d_final.as_constant() {
        if !num_traits::Zero::is_zero(d_val) {
            let inv = BigRational::from_integer(BigInt::from(1)) / d_val;
            return Some(n_final.scalar_mul(&inv).to_node());
        }
    }

    Some(Node::Divide(
        Box::new(n_final.to_node()),
        Box::new(d_final.to_node()),
    ))
}
```

**Step 4: Run tests**

Run: `cargo test --test simplify -- test_multivariate_content 2>&1`
Expected: PASS

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

**Step 5: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1`

**Step 6: Commit**

```bash
git add src/multipoly.rs src/simplify.rs tests/simplify.rs
git commit -m "feat: rational content simplification for multivariate fractions"
```

---

### Task 6: End-to-end integration tests

**Files:**
- Modify: `tests/simplify.rs`

**Step 1: Add E2E tests through the LaTeX interface**

```rust
#[test]
fn test_e2e_alex_eigenvalue_ratio() {
    // The R = 4(1-α)/(2α+1) formula — Alex's primary use case
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{-32\\alpha+32}{16\\alpha+8}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    // Should not contain original large coefficients
    assert!(!s.contains("32"), "Coefficients should be reduced: {}", s);
    assert!(!s.contains("16"), "Coefficients should be reduced: {}", s);
}

#[test]
fn test_e2e_factored_cubic_denominator() {
    // 48/(2a+1)³ expressed as expanded denominator
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{48}{16\\alpha^3+24\\alpha^2+12\\alpha+2}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let s = format!("{}", simplified);
    // Should contain 24 (not 48) and factored denominator
    assert!(!s.contains("48"), "Should reduce to 24/..., got: {}", s);
}

#[test]
fn test_already_simplified_unchanged() {
    // (a+1)/(a+2) — coprime, content 1 — should not change
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{a+1}{a+2}", &env).unwrap();
    let simplified = expr.simplify(&env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("a", 5.0);
    let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
    assert!((val - 6.0 / 7.0).abs() < 1e-10);
}
```

**Step 2: Run all tests**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4} END {print "Total tests:", sum}'`
Expected: 877 + new tests, all passing.

**Step 3: Run full CI check**

```bash
cargo fmt -- --check && cargo clippy --tests -- -D warnings && cargo test
```
Expected: All clean.

**Step 4: Commit**

```bash
git add tests/simplify.rs
git commit -m "test: e2e tests for rational function simplification"
```

---

### Task 7: Update documentation

**Files:**
- Modify: `KNUTH-PLAN.md` (update feature list and test count)

**Step 1: Update the "Current State" section**

Add after the symbolic eigenvalues mention:
- Simplifier performs rational content GCD cancellation on fractions: `(-32α+32)/(16α+8)` → `(-4α+4)/(2α+1)`.
- Factored display for denominators with repeated or multiple factors: `48/(16α³+24α²+12α+2)` → `24/(2α+1)³`.

Update test count to reflect new total.

**Step 2: Commit**

```bash
git add KNUTH-PLAN.md
git commit -m "doc: update feature list for rational GCD simplification"
```
