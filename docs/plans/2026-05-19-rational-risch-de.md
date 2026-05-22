# Rational-Coefficient Risch DE Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend the Risch differential equation solver to handle rational function RHS, enabling integration of expressions like `(1/x²)·(1-x)·exp(x)`.

**Architecture:** Replace `solve_risch_de_poly` with a generalized `solve_risch_de` that solves `s·p' + F·p = G` (polynomial ODE). Add `solve_risch_de_rational` wrapper that computes the denominator bound from the squarefree factorization and transforms to the polynomial ODE. Wire into `integrate_poly_exp` by removing the polynomial-coefficient restriction.

**Tech Stack:** Rust, exact arithmetic (BigRational), existing Polynomial/RationalFunction types.

---

### Task 1: Generalize solve_risch_de_poly to handle s·p' + F·p = G

Replace the existing `solve_risch_de_poly(f, g, var) -> Option<Polynomial>` with
`solve_risch_de(s, f_poly, g_poly, var) -> Option<Polynomial>` that solves the polynomial ODE `s·p' + f_poly·p = g_poly`.

When `s = Polynomial::one(var)`, this must behave identically to the current function.

**Files:**
- Modify: `src/risch.rs:450-559` (replace `solve_risch_de_poly`)
- Modify: `src/risch.rs:1069` (call site in `integrate_poly_exp`)

**Step 1: Write failing tests for the generalized solver**

Add these tests inside the existing `mod tests` block in `src/risch.rs` (after line ~1354). Use the existing test helpers `int()`, `poly()`.

```rust
#[test]
fn test_solve_risch_de_s1_identity() {
    // s=1 case: p' + f·p = g should match old solve_risch_de_poly behavior
    // p' + x·p = x  → p = 1 (since 0 + x·1 = x ✓)
    let s = Polynomial::one("x");
    let f = poly(&[0, 1], "x"); // x
    let g = poly(&[0, 1], "x"); // x
    let result = solve_risch_de(&s, &f, &g, "x");
    assert_eq!(result, Some(poly(&[1], "x"))); // p = 1
}

#[test]
fn test_solve_risch_de_s_x() {
    // x·p' + (x-1)·p = 1-x → p = -1
    // Verify: x·0 + (x-1)·(-1) = -x+1 = 1-x ✓
    let s = poly(&[0, 1], "x");       // x
    let f = poly(&[-1, 1], "x");      // x - 1
    let g = poly(&[1, -1], "x");      // 1 - x
    let result = solve_risch_de(&s, &f, &g, "x");
    assert_eq!(result, Some(poly(&[-1], "x"))); // p = -1
}

#[test]
fn test_solve_risch_de_s_x_no_solution() {
    // x·p' + (x-1)·p = 1 → no polynomial solution
    // If p = c: c(x-1) = 1 → cx - c = 1 → c=0, -c=1 → contradiction
    let s = poly(&[0, 1], "x");
    let f = poly(&[-1, 1], "x");
    let g = poly(&[1], "x");
    let result = solve_risch_de(&s, &f, &g, "x");
    assert_eq!(result, None);
}

#[test]
fn test_solve_risch_de_s_x_sq() {
    // x²·p' + (x²-2x)·p = x-1
    // If p = c: c(x²-2x) should equal x-1.
    //   cx²-2cx = x-1 → can't match with constant. Try p = ax+b:
    //   x²·a + (x²-2x)·(ax+b) = ax² + ax³-2ax²+bx²-2bx
    //     = ax³ + (b-a)x² - 2bx
    //   Set equal to x-1: a=0, b=0, -2b=1 → no solution
    let s = poly(&[0, 0, 1], "x");    // x²
    let f = poly(&[0, -2, 1], "x");   // x² - 2x
    let g = poly(&[-1, 1], "x");      // x - 1
    let result = solve_risch_de(&s, &f, &g, "x");
    assert_eq!(result, None);
}

#[test]
fn test_solve_risch_de_g_zero() {
    // s·p' + F·p = 0 → p = 0 is always a solution
    let s = poly(&[0, 1], "x");
    let f = poly(&[1, 1], "x");
    let g = Polynomial::zero("x");
    let result = solve_risch_de(&s, &f, &g, "x");
    assert_eq!(result, Some(Polynomial::zero("x")));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib risch::tests::test_solve_risch_de -- --nocapture 2>&1 | head -20`
Expected: compilation error — `solve_risch_de` not defined.

**Step 3: Implement the generalized solver**

Replace `solve_risch_de_poly` at `src/risch.rs:450-559` with the new function. The key changes from the old code:

1. New signature: `pub fn solve_risch_de(s: &Polynomial, f_poly: &Polynomial, g_poly: &Polynomial, var: &str) -> Option<Polynomial>`
2. The degree bound accounts for both `s·p'` and `f_poly·p` terms
3. The coefficient extraction at each degree uses the full convolution with s coefficients
4. The target index and divisor depend on which term (derivative or f_poly) dominates

```rust
/// Solve the polynomial ODE  s·p' + F·p = G  for polynomial p.
///
/// This generalizes the standard Risch DE (s=1 case: p' + f·p = g).
/// Returns None if no polynomial solution exists.
///
/// ## Algorithm
///
/// At degree r, the equation gives:
///   Σ_j s_j·(r+1-j)·b_{r+1-j}  +  Σ_j F_j·b_{r-j}  =  G_r
///
/// where b_i are the coefficients of p. Processing from highest degree
/// down, the "target" coefficient (the one being solved) is determined
/// by which term dominates:
///   - deg(F) ≥ deg(s): target is b_{r-deg(F)}, divisor is lc(F)
///   - deg(F) < deg(s): target is b_{r+1-deg(s)}, divisor uses lc(s)·(r+1-deg(s))
pub fn solve_risch_de(
    s: &Polynomial,
    f_poly: &Polynomial,
    g_poly: &Polynomial,
    var: &str,
) -> Option<Polynomial> {
    // g = 0 → p = 0
    if g_poly.is_zero() {
        return Some(Polynomial::zero(var));
    }

    // s must be nonzero
    if s.is_zero() {
        return None;
    }

    let n_g = g_poly.degree().unwrap(); // G is nonzero

    // Special case: F = 0 → s·p' = G
    if f_poly.is_zero() {
        if *s == Polynomial::one(var) {
            // p' = G → p = ∫G
            return Some(g_poly.integral());
        }
        // General s·p' = G: need G divisible by s for polynomial solution
        // Actually more nuanced — fall through to general solver
    }

    let deg_s = s.degree().unwrap_or(0);
    let deg_f = f_poly.degree().unwrap_or(0);

    // Degree bound for p
    let k: usize = if f_poly.is_zero() {
        // s·p' = G → deg(s) + deg(p) - 1 = deg(G)
        if n_g + 1 < deg_s {
            return None;
        }
        n_g + 1 - deg_s
    } else if deg_f >= deg_s {
        // Leading term from F·p at degree deg_f + deg_p
        if n_g < deg_f {
            return None;
        }
        n_g - deg_f
    } else {
        // Leading term from s·p' at degree deg_s + deg_p - 1
        if n_g + 1 < deg_s {
            return None;
        }
        n_g + 1 - deg_s
    };

    let mut b = vec![BigRational::zero(); k + 1];

    // Maximum degree we need to process: the highest degree in the equation
    let max_deg = if f_poly.is_zero() {
        deg_s + k - 1  // from s·p' only; p has degree k
    } else if deg_f >= deg_s {
        deg_f + k       // from F·p
    } else {
        deg_s + k - 1   // from s·p'
    };
    // But also need to process up to n_g (the RHS)
    let process_up_to = max_deg.max(n_g);

    for r in (0..=process_up_to).rev() {
        let g_r = g_poly.coeff(r);

        // Derivative contribution: Σ_j s_j · (r+1-j) · b_{r+1-j}
        let mut known = BigRational::zero();
        for j in 0..=deg_s {
            let idx = r + 1;
            if idx < j {
                continue;
            }
            let bi = idx - j; // b index = r+1-j
            if bi > k {
                continue;
            }
            let s_j = s.coeff(j);
            if !s_j.is_zero() && bi > 0 {
                // The derivative of b_bi · x^bi is bi · b_bi · x^{bi-1}
                // s_j · x^j · bi · b_bi · x^{bi-1} contributes at degree j + bi - 1 = r
                // Wait — we need bi (the derivative factor), not r+1-j
                // Actually bi = r+1-j, so the factor is bi = r+1-j
                known += &s_j * &BigRational::from_integer(BigInt::from(bi as i64)) * &b[bi];
            }
        }

        // F·p contribution: Σ_j F_j · b_{r-j}
        // Determine target: which b index is being solved at this degree
        let target_j: Option<usize> = if f_poly.is_zero() {
            // Only derivative term: target from s·p' is b_{r+1-deg_s}
            let idx = r + 1;
            if idx >= deg_s && idx - deg_s <= k {
                Some(idx - deg_s)
            } else {
                None
            }
        } else if deg_f >= deg_s {
            // Target from F·p term: b_{r - deg_f}
            if r >= deg_f && r - deg_f <= k {
                Some(r - deg_f)
            } else {
                None
            }
        } else {
            // Target from s·p' term: b_{r+1-deg_s}
            let idx = r + 1;
            if idx >= deg_s && idx - deg_s <= k {
                Some(idx - deg_s)
            } else {
                None
            }
        };

        // Subtract the target's contribution from known (we computed it above)
        // For derivative part: if target = b_t, its contribution is s_{r+1-t} · t · b_t
        //   but only if r+1-t ≤ deg_s
        // For F·p part: if target = b_t, its contribution is F_{r-t} · b_t
        //   but only if r-t ≤ deg_f

        // Actually, let's recompute known WITHOUT the target, then compute divisor.
        // Restart: accumulate all known contributions, skipping the target.

        known = BigRational::zero();

        // Derivative contribution, skipping target
        for j in 0..=deg_s {
            if r + 1 < j { continue; }
            let bi = r + 1 - j;
            if bi > k || bi == 0 { continue; }
            if Some(bi) == target_j { continue; }
            let s_j = s.coeff(j);
            if !s_j.is_zero() {
                known += &s_j * &BigRational::from_integer(BigInt::from(bi as i64)) * &b[bi];
            }
        }

        // F·p contribution, skipping target
        let f_deg = if f_poly.is_zero() { 0 } else { deg_f };
        for j in 0..=f_deg.min(r) {
            if r < j { continue; }
            let bi = r - j;
            if bi > k { continue; }
            if Some(bi) == target_j { continue; }
            let f_j = f_poly.coeff(j);
            if !f_j.is_zero() {
                known += &f_j * &b[bi];
            }
        }

        let residual = &g_r - &known;

        match target_j {
            Some(t) => {
                // Compute divisor: sum of coefficients of b_t from both terms
                let mut divisor = BigRational::zero();

                // From derivative: s_{r+1-t} · t (if r+1-t ≤ deg_s and t > 0)
                if t > 0 && r + 1 >= t && r + 1 - t <= deg_s {
                    let s_idx = r + 1 - t;
                    divisor += &s.coeff(s_idx) * &BigRational::from_integer(BigInt::from(t as i64));
                }

                // From F·p: F_{r-t} (if r ≥ t and r-t ≤ deg_f)
                if r >= t && r - t <= f_deg {
                    divisor += f_poly.coeff(r - t);
                }

                if divisor.is_zero() {
                    if !residual.is_zero() {
                        return None;
                    }
                    // b_t is free; set to 0
                } else {
                    b[t] = &residual / &divisor;
                }
            }
            None => {
                if !residual.is_zero() {
                    return None;
                }
            }
        }
    }

    // Build p and verify: s·p' + f_poly·p must equal g_poly
    let p = Polynomial::from_coeffs(b, var);
    let check = &(s * &p.derivative()) + &(f_poly * &p);
    if check == *g_poly {
        Some(p)
    } else {
        None
    }
}
```

**Step 4: Add backward-compatible wrapper**

Keep a thin wrapper so existing call sites don't all need updating immediately:

```rust
/// Backward-compatible wrapper: solve p' + f·p = g (the s=1 case).
pub fn solve_risch_de_poly(f: &Polynomial, g: &Polynomial, var: &str) -> Option<Polynomial> {
    solve_risch_de(&Polynomial::one(var), f, g, var)
}
```

**Step 5: Run all tests**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 747 passed, 0 failed (all existing tests pass through the wrapper).

Run: `cargo test --lib risch::tests::test_solve_risch_de 2>&1 | grep -E "test |FAILED"`
Expected: all 5 new tests pass.

**Step 6: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1 | tail -5`
Expected: clean.

**Step 7: Commit**

```bash
git add src/risch.rs
git commit -m "Generalize Risch DE solver to handle s·p' + F·p = G"
```

---

### Task 2: Implement solve_risch_de_rational

The wrapper that takes `f ∈ K[x]` (polynomial) and `g ∈ K(x)` (rational function), computes the denominator bound from squarefree factorization, transforms to the polynomial ODE, and calls `solve_risch_de`.

**Files:**
- Modify: `src/risch.rs` (add function after `solve_risch_de`)

**Step 1: Write failing tests**

```rust
#[test]
fn test_solve_risch_de_rational_poly_rhs() {
    // When g is actually a polynomial, should behave like solve_risch_de_poly
    // q' + x·q = x → q = 1
    let f = poly(&[0, 1], "x");
    let g = RationalFunction::from_poly(poly(&[0, 1], "x"));
    let result = solve_risch_de_rational(&f, &g, "x");
    let expected = RationalFunction::from_poly(poly(&[1], "x"));
    assert_eq!(result, Some(expected));
}

#[test]
fn test_solve_risch_de_rational_simple_pole_rejection() {
    // q' + q = 1/x → no rational solution (denominator has simple pole)
    let f = poly(&[1], "x");
    let g = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")); // 1/x
    let result = solve_risch_de_rational(&f, &g, "x");
    assert_eq!(result, None);
}

#[test]
fn test_solve_risch_de_rational_double_pole_success() {
    // q' + q = (1-x)/x² → q = -1/x
    // Verify: (-1/x)' + (-1/x) = 1/x² - 1/x = (1-x)/x² ✓
    let f = poly(&[1], "x"); // f = 1
    let g = RationalFunction::new(
        poly(&[1, -1], "x"),   // 1 - x
        poly(&[0, 0, 1], "x"), // x²
    );
    let result = solve_risch_de_rational(&f, &g, "x");
    let expected = RationalFunction::new(
        poly(&[-1], "x"),      // -1
        poly(&[0, 1], "x"),    // x
    );
    assert_eq!(result, Some(expected));
}

#[test]
fn test_solve_risch_de_rational_double_pole_no_solution() {
    // q' + q = 1/x² → no polynomial solution in transformed DE
    // Transform: s=x, x·p' + (x-1)·p = 1
    // p=c: c(x-1) = 1 → contradiction
    let f = poly(&[1], "x");
    let g = RationalFunction::new(
        poly(&[1], "x"),       // 1
        poly(&[0, 0, 1], "x"), // x²
    );
    let result = solve_risch_de_rational(&f, &g, "x");
    assert_eq!(result, None);
}

#[test]
fn test_solve_risch_de_rational_f_zero() {
    // q' = (2x)/x² = 2/x → has simple pole → None
    let f = Polynomial::zero("x");
    let g = RationalFunction::new(
        poly(&[0, 2], "x"),    // 2x
        poly(&[0, 0, 1], "x"), // x²
    );
    // After RationalFunction::new normalizes: 2x/x² = 2/x (den = x, simple pole)
    let result = solve_risch_de_rational(&f, &g, "x");
    assert_eq!(result, None);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib risch::tests::test_solve_risch_de_rational 2>&1 | head -10`
Expected: compilation error — `solve_risch_de_rational` not defined.

**Step 3: Implement solve_risch_de_rational**

Add after `solve_risch_de_poly` in `src/risch.rs`:

```rust
/// Solve the Risch DE  q' + f·q = g  where f is a polynomial and g is a
/// rational function. Returns the rational function q, or None if no
/// rational solution exists.
///
/// ## Algorithm
///
/// 1. If g is polynomial, delegate to solve_risch_de (s=1 case).
/// 2. Squarefree-reject: if den(g) has any factor with multiplicity 1,
///    no rational solution exists (pole analysis).
/// 3. Compute denominator bound s = ∏ dⱼ^{j-1}.
/// 4. Transform: set q = p/s, derive polynomial ODE s·p' + F·p = G
///    where F = f·s - s', G = num(g) · (s²/den(g)).
/// 5. Solve for polynomial p via solve_risch_de.
/// 6. Verify q' + f·q = g.
pub fn solve_risch_de_rational(
    f: &Polynomial,
    g: &RationalFunction,
    var: &str,
) -> Option<RationalFunction> {
    let den = g.denominator();

    // If g is a polynomial, use the standard solver
    if *den == Polynomial::one(var) {
        let p = solve_risch_de(&Polynomial::one(var), f, g.numerator(), var)?;
        return Some(RationalFunction::from_poly(p));
    }

    // Squarefree factorization of the denominator
    let sfd = den.square_free_decomposition();

    // Check: if any factor has multiplicity 1, no rational solution exists
    for (factor, mult) in &sfd {
        if *mult == 1 && !factor.is_constant() {
            return None;
        }
    }

    // Compute denominator bound: s = ∏ factor^{mult-1}
    let mut s = Polynomial::one(var);
    for (factor, mult) in &sfd {
        if *mult >= 2 {
            for _ in 0..mult - 1 {
                s = &s * factor;
            }
        }
    }

    // Compute s²/den (polynomial since all multiplicities ≥ 2)
    let s_sq = &s * &s;
    let (r_poly, rem) = s_sq.div_rem(den).unwrap();
    debug_assert!(rem.is_zero(), "s²/den must be exact");

    // Transform: s·p' + (f·s - s')·p = num(g) · (s²/den)
    let s_prime = s.derivative();
    let f_transformed = &(f * &s) - &s_prime;    // F = f·s - s'
    let g_transformed = &(g.numerator().clone()) * &r_poly; // G = num · (s²/den)

    // Solve the polynomial ODE
    let p = solve_risch_de(&s, &f_transformed, &g_transformed, var)?;

    // Build q = p/s and verify
    let q = RationalFunction::new(p, s);
    let check = &q.derivative() + &RationalFunction::from_poly(f.clone()).rf_mul(&q);
    if check == *g {
        Some(q)
    } else {
        None
    }
}
```

Note: the verification step uses `RationalFunction` arithmetic. Check that `rf_mul` or `Mul` is available — if not, compute the check via numerator/denominator arithmetic.

**Step 4: Check RationalFunction arithmetic**

Before running, verify the needed operations exist:
- `RationalFunction::derivative()` — exists at line 115
- Addition of RationalFunctions — check for `impl Add` or `rf_add`
- Multiplication — check for `impl Mul` or `rf_mul`

Run: `grep -n "impl.*Add.*RationalFunction\|impl.*Mul.*RationalFunction\|fn rf_mul\|fn rf_add" src/rational_function.rs`

If these don't exist, implement the verification using `Polynomial` operations:
```rust
// Manual verification: q' + f·q = g
// q = p/s, q' = (p's - ps')/(s²)
// q' + f·q = (p's - ps')/(s²) + fp/s = (p's - ps' + fps)/(s²)
// This should equal num_g / den_g
let p_prime = p.derivative();
let check_num = &(&(&p_prime * &s) - &(&p * &s_prime)) + &(&(f * &p) * &s);
let check_den = &s * &s; // s²
// Normalize: check_num/check_den should equal num_g/den_g
// i.e., check_num · den_g == num_g · check_den
let lhs = &check_num * den;
let rhs = &(g.numerator().clone()) * &check_den;
if lhs == rhs { Some(RationalFunction::new(p, s)) } else { None }
```

**Step 5: Run tests**

Run: `cargo test --lib risch::tests::test_solve_risch_de_rational 2>&1 | grep -E "test |FAILED"`
Expected: all 5 tests pass.

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 752 passed (747 + 5 new), 0 failed.

**Step 6: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1 | tail -5`
Expected: clean.

**Step 7: Commit**

```bash
git add src/risch.rs
git commit -m "Add solve_risch_de_rational for Risch DE with rational function RHS"
```

---

### Task 3: Wire into integrate_poly_exp

Remove the polynomial-coefficient restriction in `integrate_poly_exp` and use the new solver for rational coefficients.

**Files:**
- Modify: `src/risch.rs:1041-1117` (`integrate_poly_exp`)

**Step 1: Write failing end-to-end tests**

Add to the `mod tests` block in `src/risch.rs`:

```rust
#[test]
fn test_integrate_poly_exp_rational_coeff_elementary() {
    // ∫((1-x)/x²)·exp(x)dx = -exp(x)/x
    // ExtPoly: (1-x)/x² · θ  (degree 1, coefficient (1-x)/x²)
    let coeff = RationalFunction::new(
        poly(&[1, -1], "x"),    // 1 - x
        poly(&[0, 0, 1], "x"), // x²
    );
    let num = ExtPoly::from_rf(coeff); // (1-x)/x² · θ⁰... wait, need θ¹
    // Actually: need ExtPoly with coeff at degree 1
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),     // coeff of θ⁰
        RationalFunction::new(           // coeff of θ¹
            poly(&[1, -1], "x"),
            poly(&[0, 0, 1], "x"),
        ),
    ], "x");
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), // g = x (so θ = exp(x))
        "x",
    );
    let result = integrate_poly_exp(&num, &ext, "x");
    assert!(matches!(result, Some(RischResult::Elementary(_))));
}

#[test]
fn test_integrate_poly_exp_rational_coeff_non_elementary() {
    // ∫(1/x)·exp(x)dx is non-elementary (Ei function)
    // ExtPoly: (1/x) · θ
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")), // 1/x
    ], "x");
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let result = integrate_poly_exp(&num, &ext, "x");
    assert!(matches!(result, Some(RischResult::NonElementary(_))));
}

#[test]
fn test_integrate_poly_exp_rational_coeff_inv_x_sq() {
    // ∫exp(x)/x² dx is non-elementary
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        RationalFunction::new(
            poly(&[1], "x"),
            poly(&[0, 0, 1], "x"), // 1/x²
        ),
    ], "x");
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let result = integrate_poly_exp(&num, &ext, "x");
    assert!(matches!(result, Some(RischResult::NonElementary(_))));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib risch::tests::test_integrate_poly_exp_rational 2>&1 | grep -E "test |FAILED"`
Expected: the elementary test fails (returns None instead of Elementary), and the non-elementary tests may return None instead of NonElementary.

**Step 3: Modify integrate_poly_exp**

In `src/risch.rs`, the function `integrate_poly_exp` (starting around line 1041). The current code at each degree:

```rust
// CURRENT CODE (lines ~1056-1078):
let a_i_rf = num.coeff(i);
if a_i_rf.is_zero() { continue; }
if *a_i_rf.denominator() != Polynomial::one(var) {
    return None; // ← THIS IS THE RESTRICTION TO REMOVE
}
let a_i = a_i_rf.numerator().clone();
// ... solve_risch_de_poly(&f, &a_i, var) ...
```

Replace with:

```rust
let a_i_rf = num.coeff(i);
if a_i_rf.is_zero() {
    continue;
}

if i == 0 {
    // Degree 0: q_0 = ∫a_0. For polynomial a_0, use polynomial integral.
    // For rational a_0, need rational function integration (not yet supported).
    if *a_i_rf.denominator() != Polynomial::one(var) {
        return None; // TODO: rational function integration for degree-0 term
    }
    q[0] = a_i_rf.numerator().clone().integral();
} else {
    let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));

    if *a_i_rf.denominator() == Polynomial::one(var) {
        // Polynomial coefficient: use existing polynomial solver
        match solve_risch_de_poly(&f, a_i_rf.numerator(), var) {
            Some(qi) => q_rf[i] = RationalFunction::from_poly(qi),
            None => {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative exists. \
                     The differential equation q' + ({})·q = {} has no polynomial solution.",
                    f, a_i_rf.numerator()
                )));
            }
        }
    } else {
        // Rational coefficient: use the new rational solver
        match solve_risch_de_rational(&f, &a_i_rf, var) {
            Some(qi) => q_rf[i] = qi,
            None => {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative exists. \
                     The differential equation q' + ({})·q = {} has no rational solution.",
                    f, a_i_rf
                )));
            }
        }
    }
}
```

**Important structural change:** The result coefficients `q[i]` must change from `Vec<Polynomial>` to `Vec<RationalFunction>` since the Risch DE with rational RHS returns RationalFunction solutions. Update the result-building code at the end of the function accordingly — use `rf_to_node` instead of `qi.to_node()`.

The full modified function structure:

```rust
fn integrate_poly_exp(
    num: &ExtPoly,
    ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let g_prime_rf = ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();

    let mut q: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];

    for i in 0..=deg {
        let a_i_rf = num.coeff(i);
        if a_i_rf.is_zero() {
            continue;
        }

        if i == 0 {
            if *a_i_rf.denominator() != Polynomial::one(var) {
                return None; // degree-0 rational integration not yet supported
            }
            q[0] = RationalFunction::from_poly(a_i_rf.numerator().clone().integral());
        } else {
            let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_rational(&f, &a_i_rf, var) {
                Some(qi) => q[i] = qi,
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         The differential equation q' + ({})·q = {} has no rational solution.",
                        f, a_i_rf
                    )));
                }
            }
        }
    }

    // Build result node: Σ q_i(x) · exp(g(x))^i
    let g_node = ext.argument().numerator().to_node();
    let mut terms: Vec<Node> = Vec::new();
    for (i, qi) in q.iter().enumerate() {
        if qi.is_zero() {
            continue;
        }
        let q_node = rf_to_node(qi, var);
        let term = if i == 0 {
            q_node
        } else {
            let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
            let exp_part = if i == 1 {
                exp_g
            } else {
                Node::Power(
                    Box::new(exp_g),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            if *qi == RationalFunction::one(var) {
                exp_part
            } else {
                Node::Multiply(Box::new(q_node), Box::new(exp_part))
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}
```

**Step 4: Run all tests**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 755+ passed (747 + new tests), 0 failed.

**Step 5: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1 | tail -5`
Expected: clean.

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Wire rational-coefficient Risch DE into integrate_poly_exp"
```

---

### Task 4: End-to-end integration tests via parser

Test the full pipeline: LaTeX parse → tower build → integration → result.

**Files:**
- Modify: `src/integration.rs` (add tests at end of test module)

**Step 1: Write end-to-end tests**

Add to the test module in `src/integration.rs`:

```rust
#[test]
fn test_integrate_1_minus_x_over_x_sq_exp_x() {
    // ∫((1-x)/x²)·exp(x)dx = -exp(x)/x
    let expr = parse_expression("\\frac{1-x}{x^2} \\cdot \\exp(x)").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_ok(), "Expected elementary result, got: {:?}", result);
}

#[test]
fn test_integrate_exp_x_over_x_non_elementary() {
    // ∫exp(x)/x dx is non-elementary (exponential integral Ei)
    let expr = parse_expression("\\frac{\\exp(x)}{x}").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.starts_with("NON_ELEMENTARY:"),
        "Expected NON_ELEMENTARY, got: {}",
        err
    );
}

#[test]
fn test_integrate_exp_x_over_x_sq_non_elementary() {
    // ∫exp(x)/x² dx is non-elementary
    let expr = parse_expression("\\frac{\\exp(x)}{x^2}").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.starts_with("NON_ELEMENTARY:"),
        "Expected NON_ELEMENTARY, got: {}",
        err
    );
}
```

**Step 2: Run end-to-end tests**

Run: `cargo test --lib integration::tests::test_integrate_1_minus_x 2>&1 | grep -E "test |FAILED"`
Run: `cargo test --lib integration::tests::test_integrate_exp_x_over 2>&1 | grep -E "test |FAILED"`

If the LaTeX parsing produces unexpected trees, debug with:
`cargo run -- evaluate "\\frac{1-x}{x^2} \\cdot \\exp(x)" x=1 2>&1`

**Step 3: If tests fail, debug the tower builder**

The most likely failure mode: `build_tower` may not correctly identify the expression as an exponential tower when the expression has a `Divide` at the top level with exp in the numerator and a polynomial in the denominator. Check that `node_to_extpoly_general` handles `Divide(exp_expr, x_poly)` correctly (it should: line 953-961 of risch.rs handles Divide by converting den to a scalar).

If `build_tower` can't build the tower for `\frac{1-x}{x^2} \cdot \exp(x)`, the issue is in how the expression tree is structured after parsing. Add a debug print of the parsed AST and trace through `build_tower_inner`.

**Step 4: Verify correctness numerically**

For the elementary result `∫((1-x)/x²)·exp(x)dx`:

```bash
# If the result is -exp(x)/x, verify: d/dx[-exp(x)/x] at x=2
# d/dx[-exp(x)/x] = -exp(x)/x + exp(x)/x² = exp(x)(1-x)/x²
# At x=2: exp(2)(1-2)/4 = exp(2)·(-1/4) ≈ -1.847
cargo run -- differentiate "<result_expression>" x=2
cargo run -- evaluate "\\frac{1-x}{x^2} \\cdot \\exp(2)" x=2
# Both should give the same value
```

**Step 5: Run full test suite**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 758+ passed, 0 failed.

Run: `cargo clippy --tests -- -D warnings 2>&1 | tail -5`
Expected: clean.

**Step 6: Commit**

```bash
git add src/integration.rs src/risch.rs
git commit -m "Add end-to-end tests for rational-coefficient exp integration"
```

---

### Task 5: MCP server test + README update

Verify the new capability works through the MCP server interface and update the README.

**Files:**
- Modify: `README.md`

**Step 1: Test via CLI**

```bash
cargo run -- integrate "\\frac{1-x}{x^2} \\cdot \\exp(x)"
# Expected: -exp(x)/x  or equivalent form

cargo run -- integrate "\\frac{\\exp(x)}{x}"
# Expected: NON_ELEMENTARY message
```

**Step 2: Update README**

Add to the integration capabilities section:
- Rational-coefficient exponential integration (e.g., `∫((1-x)/x²)·exp(x)dx`)
- Non-elementary detection for exponential integral type (Ei-related)
- Update the test count

**Step 3: Final full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`
Expected: clean formatting, 0 warnings, all tests pass.

**Step 4: Commit**

```bash
git add README.md
git commit -m "Update README: rational-coefficient Risch DE, N tests"
```

---

## Implementation Notes

**Squarefree factorization:** Already exists at `Polynomial::square_free_decomposition()` (src/polynomial.rs:337). Returns `Vec<(Polynomial, usize)>` — pairs of (squarefree factor, multiplicity).

**RationalFunction arithmetic:** The verification step in `solve_risch_de_rational` needs RF addition and multiplication. Check `src/rational_function.rs` for `impl Add` / `impl Mul`. If missing, use the polynomial-level verification shown in the fallback code in Task 2.

**ExtPoly::from_coeffs:** Used in tests. Check that this constructor exists; if not, build the ExtPoly manually using the available API.

**Scope boundary:** The log extension case (`integrate_poly_log` with rational coefficients) is deliberately OUT of scope. It requires rational function integration in the base field (partial fractions → logs), which is a different problem. Left as a TODO comment and future backlog item.
