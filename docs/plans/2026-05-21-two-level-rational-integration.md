# Two-Level Rational Integration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate rational functions of exp(g(x)) with ln(x) coefficients — e.g., ln(x)/(1+exp(x)) — via two-level Rothstein-Trager, with rigorous non-elementarity proofs.

**Architecture:** Extend the two-level tower to handle rational-in-θ₂ integrands. The denominator is restricted to Q(x)[θ₂] (no θ₁ in denominator). Hermite reduction exploits linearity: run the existing single-level `hermite_reduce` on each θ₁-degree of the numerator independently (the denominator is the same for all). For the squarefree remainder, a two-level Rothstein-Trager computes the resultant with θ₁-structured entries in the Sylvester matrix, then checks for constant roots by specializing x (collapsing θ₁ to a number) and verifying candidates as ExtPoly identities. When roots exist, compute GCD for degree-1 denominators via polynomial evaluation. Falls back to existing polynomial two-level integration for the quotient and exponential residual.

**Tech Stack:** Rust, existing `ExtPoly`, `RationalFunction`, `hermite_reduce`, `DifferentialExtension`, `extpoly_to_node`, two-level helpers from Session 24.

**Reference:** Bronstein, *Symbolic Integration I*, §5.4 (Rothstein-Trager in exponential extensions).

---

## Mathematical Foundation

### Setup

Tower: Q(x) ⊂ Q(x, θ₁=ln(x)) ⊂ Q(x, θ₁, θ₂=exp(g(x)))

Integrand: A(θ₁, θ₂)/D(θ₂) where A has θ₁ coefficients, D ∈ Q(x)[θ₂].

### Hermite reduction (linearity)

Hermite reduction is **linear in the numerator** for fixed denominator. The squarefree decomposition, extended GCD, and all denominator-side operations depend only on D. So for A = Σⱼ θ₁ʲ·Aⱼ(θ₂) where each Aⱼ is a standard ExtPoly:

```
hermite_reduce(A/D) = Σⱼ θ₁ʲ · hermite_reduce(Aⱼ/D)
```

The h_den (squarefree denominator) is identical for all j since it depends only on D.

### Rothstein-Trager with θ₁ coefficients

For squarefree d ∈ Q(x)[θ₂] and numerator a with θ₁ coefficients:

R(z) = res_θ₂(d, a − z·D(d))

where D(d) = d/dx[d] in the tower. Since d has no θ₁, D(d) also has no θ₁ (the derivative d/dx of Q(x) stays in Q(x), and the exp chain rule adds g'·d_i which is Q(x)).

The Sylvester matrix has:
- Rows from d: entries are Q(x) (no θ₁, no z)
- Rows from a−z·D(d): entries are Q(x)[θ₁] + z·Q(x) (θ₁ from a, z from −z·D(d))

The determinant R(z) is polynomial in z with Q(x)[θ₁] coefficients.

### Constant root check

For c ∈ Q to be a root: R(c) must be zero as an element of Q(x)[θ₁]. This means every θ₁-coefficient (which is a RationalFunction in x) must be identically zero.

Finding candidates: specialize x → x₀, evaluate θ₁-degree-0 coefficients, find rational roots of the resulting Q[z]. Verify each candidate c by computing R(c) as an ExtPoly and checking all coefficients are zero.

### Verification example

∫ ln(x)/(1+exp(x)) dx:
- d = 1+θ₂, a = θ₁ (constant in θ₂), D(d) = θ₂
- R(z) = res(1+θ₂, θ₁ − z·θ₂) = det [[1, -z], [1, θ₁]] = θ₁ + z
- For constant root c: θ₁ + c = 0 requires θ₁ = -c (impossible, θ₁ is transcendental)
- Non-elementary ✓

---

## Tasks

### Task 1: Two-level parsing for rational expressions

**Files:**
- Modify: `src/risch.rs` — extend `try_risch_two_level`, add `extract_two_level_rational`

Currently `try_risch_two_level` converts the entire expression to `Vec<ExtPoly>` (polynomial in θ₂). This fails for rational expressions like `ln(x)/(1+exp(x))` because `node_to_two_level` can't handle a denominator containing exp.

Add a function that extracts the numerator and denominator as separate `Vec<ExtPoly>` values, then modify `try_risch_two_level` to detect and dispatch rational integrands.

**Step 1: Write failing tests**

```rust
// === Two-level rational parsing tests ===

#[test]
fn test_two_level_rational_ln_over_1_plus_exp() {
    // ln(x)/(1+exp(x)) → num=[θ₁], den=[1,1]
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let one = Node::Num(ExactNum::integer(1));
    let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let den = Node::Add(Box::new(one), Box::new(exp_x));
    let expr = Node::Divide(Box::new(ln_x), Box::new(den));
    let exp_arg = poly(&[0, 1], "x");
    let (num, denom) = extract_two_level_rational(&expr, "x", &exp_arg).unwrap();
    assert_eq!(num.len(), 1);
    assert_eq!(num[0], ExtPoly::theta("x")); // θ₁ = ln(x)
    assert_eq!(denom.len(), 2);
    assert_eq!(denom[0], ExtPoly::from_rf(rf_const(1)));
    assert_eq!(denom[1], ExtPoly::from_rf(rf_const(1)));
}

#[test]
fn test_two_level_rational_exp_ln_over_1_plus_exp() {
    // exp(x)*ln(x)/(1+exp(x)) → num=[0, θ₁], den=[1,1]
    let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let num_node = Node::Multiply(Box::new(exp_x.clone()), Box::new(ln_x));
    let one = Node::Num(ExactNum::integer(1));
    let den_node = Node::Add(Box::new(one), Box::new(exp_x));
    let expr = Node::Divide(Box::new(num_node), Box::new(den_node));
    let exp_arg = poly(&[0, 1], "x");
    let (num, denom) = extract_two_level_rational(&expr, "x", &exp_arg).unwrap();
    assert_eq!(num.len(), 2);
    assert!(num[0].is_zero());
    assert_eq!(num[1], ExtPoly::theta("x"));
    assert_eq!(denom.len(), 2);
}

#[test]
fn test_two_level_rational_polynomial_returns_none() {
    // exp(x)*ln(x) has no denominator with θ₂ → None
    let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let expr = Node::Multiply(Box::new(exp_x), Box::new(ln_x));
    let exp_arg = poly(&[0, 1], "x");
    assert!(extract_two_level_rational(&expr, "x", &exp_arg).is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_two_level_rational -- --nocapture 2>&1 | head -10`

**Step 3: Implement `extract_two_level_rational`**

```rust
/// Extract numerator and denominator as two-level polynomials from a
/// Node that is a rational function in θ₂ = exp(g(x)) with θ₁ = ln(x) coefficients.
///
/// Returns Some((num, den)) where both are Vec<ExtPoly>, or None if the
/// expression is not a recognizable rational function in θ₂.
fn extract_two_level_rational(
    expr: &Node,
    var: &str,
    exp_arg: &Polynomial,
) -> Option<(Vec<ExtPoly>, Vec<ExtPoly>)> {
    match expr {
        Node::Divide(num_node, den_node) => {
            let num = node_to_two_level(num_node, var, exp_arg)?;
            let den = node_to_two_level(den_node, var, exp_arg)?;
            // Only rational if denominator has θ₂ terms
            if den.len() <= 1 {
                return None;
            }
            Some((num, den))
        }
        Node::Multiply(left, right) => {
            // Check for a * (b/c) pattern
            if let Node::Divide(n, d) = right.as_ref() {
                let d_tl = node_to_two_level(d, var, exp_arg)?;
                if d_tl.len() > 1 {
                    let l_tl = node_to_two_level(left, var, exp_arg)?;
                    let n_tl = node_to_two_level(n, var, exp_arg)?;
                    return Some((mul_two_level(&l_tl, &n_tl, var), d_tl));
                }
            }
            // Check for (a/b) * c pattern
            if let Node::Divide(n, d) = left.as_ref() {
                let d_tl = node_to_two_level(d, var, exp_arg)?;
                if d_tl.len() > 1 {
                    let r_tl = node_to_two_level(right, var, exp_arg)?;
                    let n_tl = node_to_two_level(n, var, exp_arg)?;
                    return Some((mul_two_level(&r_tl, &n_tl, var), d_tl));
                }
            }
            None
        }
        _ => None,
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_two_level_rational -- --nocapture`

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add extract_two_level_rational: parse rational-in-exp with ln coefficients"
```

---

### Task 2: Per-θ₁-degree Hermite reduction + two-level Rothstein-Trager

**Files:**
- Modify: `src/risch.rs` — add `hermite_reduce_two_level`, `rothstein_trager_two_level`, `find_constant_roots_two_level`

This task implements the core mathematical machinery.

**Hermite reduction** exploits linearity: for each θ₁-degree j of the numerator, run the existing `hermite_reduce(Aⱼ, D)` and collect the results. The h_den is the same for all j.

**Rothstein-Trager** builds a Sylvester matrix where some entries have θ₁ coefficients (from the numerator), computes the determinant as a polynomial in z with ExtPoly coefficients, then finds constant roots.

**Step 1: Write failing tests**

```rust
#[test]
fn test_hermite_reduce_two_level_squarefree() {
    // Squarefree denominator: no reduction needed.
    // num = [θ₁], den = [1, 1] (= 1 + θ₂)
    let num = vec![ExtPoly::theta("x")];
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = hermite_reduce_two_level(&num, &den, "x").unwrap();
    // No rational part (all g_num should be zero)
    assert!(result.g_num.iter().all(|ep| ep.is_zero()));
    // Integrand unchanged: h_num = [θ₁], h_den = 1 + θ₂
    assert_eq!(result.h_num.len(), 1);
    assert_eq!(result.h_num[0], ExtPoly::theta("x"));
    assert_eq!(result.h_den, den);
}

#[test]
fn test_hermite_reduce_two_level_non_squarefree() {
    // Non-squarefree: den = (1+θ₂)² = 1 + 2θ₂ + θ₂²
    // num = [θ₁] (just θ₁, degree 0 in θ₂)
    let num = vec![ExtPoly::theta("x")];
    let t_plus_1 = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let den = &t_plus_1 * &t_plus_1;
    let result = hermite_reduce_two_level(&num, &den, "x").unwrap();
    // Rational part should be nonzero
    assert!(!result.g_num.iter().all(|ep| ep.is_zero()));
    // h_den should be squarefree
    let sfd = result.h_den.square_free_decomposition();
    assert!(sfd.iter().all(|(_, m)| *m <= 1));
}

#[test]
fn test_rt_two_level_ln_over_1_plus_exp() {
    // R(z) for ∫ln(x)/(1+exp(x)): d=1+θ₂, a=θ₁, D(d)=θ₂
    // R(z) = θ₁ + z → no constant roots (θ₁ is transcendental)
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let a = vec![ExtPoly::theta("x")];  // [θ₁] degree 0 in θ₂
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let dd = ext.differentiate(&d);
    let rz = rothstein_trager_two_level(&d, &a, &dd, "x");
    let roots = find_constant_roots_two_level(&rz, "x");
    assert!(roots.is_empty(), "Should have no constant roots");
}

#[test]
fn test_rt_two_level_constant_coeff_has_root() {
    // If the numerator has no θ₁ (just Q(x) coefficients), should behave
    // like single-level RT. Test: ∫1/(x·(1+exp(x)))·x = 1/(1+exp(x))
    // d=1+θ₂, a=1, D(d)=θ₂. R(z) = 1+z → root at z=-1
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let a = vec![ExtPoly::from_rf(rf_const(1))];  // [1]
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let dd = ext.differentiate(&d);
    let rz = rothstein_trager_two_level(&d, &a, &dd, "x");
    let roots = find_constant_roots_two_level(&rz, "x");
    assert_eq!(roots, vec![int(-1)]);
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement**

#### Hermite reduction (two-level)

```rust
/// Result of two-level Hermite reduction.
struct HermiteResultTwoLevel {
    g_num: Vec<ExtPoly>,   // numerator of rational part (θ₁-structured)
    g_den: ExtPoly,        // denominator of rational part
    h_num: Vec<ExtPoly>,   // numerator of integrand (θ₁-structured)
    h_den: ExtPoly,        // squarefree denominator
}

/// Hermite reduction for a two-level integrand A/D where A has θ₁ coefficients
/// and D ∈ Q(x)[θ₂].
///
/// Exploits linearity: runs existing `hermite_reduce` on each θ₁-degree of A
/// independently, then combines results. The h_den is the same for all θ₁-degrees.
fn hermite_reduce_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    var: &str,
) -> Result<HermiteResultTwoLevel, String> {
    // Find max θ₁-degree across all θ₂-coefficients in the numerator
    let max_theta1_deg = num.iter()
        .filter_map(|ep| ep.degree())
        .max()
        .unwrap_or(0);

    // For each θ₁-degree j, extract the standard ExtPoly (θ₂-polynomial
    // with Q(x) coefficients) and run Hermite reduction.
    let mut all_results: Vec<HermiteResult> = Vec::new();

    for j in 0..=max_theta1_deg {
        // Build Aⱼ: the coefficient of θ₁ʲ in the numerator.
        // num[i] is the coefficient of θ₂ⁱ, an ExtPoly in θ₁.
        // Aⱼ[i] = num[i].coeff(j) — the Q(x) coefficient of θ₁ʲ at θ₂ⁱ.
        let a_j_coeffs: Vec<RationalFunction> = (0..num.len())
            .map(|i| num[i].coeff(j))
            .collect();
        let a_j = ExtPoly::from_coeffs(a_j_coeffs, var);

        let hr = hermite_reduce(&a_j, den, var)?;
        all_results.push(hr);
    }

    // Combine results.
    // h_den is the same for all j (depends only on D).
    let h_den = all_results[0].h_den.clone();

    // g_den: may differ between j due to GCD simplification.
    // Combine using LCM denominator.
    let mut g_den = ExtPoly::one(var);
    for hr in &all_results {
        if !hr.g_num.is_zero() {
            // g_den = lcm(g_den, hr.g_den) = g_den * hr.g_den / gcd(g_den, hr.g_den)
            let g = g_den.gcd(&hr.g_den);
            let (factor, _) = hr.g_den.div_rem(&g).unwrap();
            g_den = &g_den * &factor;
        }
    }

    // Compute g_num and h_num for each θ₁-degree j
    // g_num[i] is the coefficient of θ₂ⁱ, an ExtPoly in θ₁
    // For θ₁-degree j: g_num_j = hr_j.g_num * (g_den / hr_j.g_den)
    let max_g_theta2 = all_results.iter()
        .filter_map(|hr| hr.g_num.degree())
        .max()
        .unwrap_or(0);
    let max_h_theta2 = all_results.iter()
        .filter_map(|hr| hr.h_num.degree())
        .max()
        .unwrap_or(0);

    let mut g_num_out = vec![ExtPoly::zero(var); max_g_theta2 + 1];
    let mut h_num_out = vec![ExtPoly::zero(var); max_h_theta2 + 1];

    for (j, hr) in all_results.iter().enumerate() {
        // Scale g_num by common denominator factor
        let scale = if hr.g_num.is_zero() {
            ExtPoly::one(var)
        } else {
            let (s, _) = g_den.div_rem(&hr.g_den).unwrap();
            s
        };
        let scaled_g = &hr.g_num * &scale;

        // Distribute θ₁ʲ coefficient into the output
        for i in 0..=max_g_theta2 {
            let coeff = scaled_g.coeff(i); // RationalFunction
            if !coeff.is_zero() {
                // Add coeff · θ₁ʲ to g_num_out[i]
                let mut new_coeffs = vec![RationalFunction::zero(var); j + 1];
                new_coeffs[j] = coeff;
                let term = ExtPoly::from_coeffs(new_coeffs, var);
                g_num_out[i] = &g_num_out[i] + &term;
            }
        }

        // h_num: h_den is the same for all j, so no scaling needed
        for i in 0..=max_h_theta2 {
            let coeff = hr.h_num.coeff(i);
            if !coeff.is_zero() {
                let mut new_coeffs = vec![RationalFunction::zero(var); j + 1];
                new_coeffs[j] = coeff;
                let term = ExtPoly::from_coeffs(new_coeffs, var);
                h_num_out[i] = &h_num_out[i] + &term;
            }
        }
    }

    // Strip trailing zeros
    while g_num_out.last().is_some_and(|c| c.is_zero()) { g_num_out.pop(); }
    while h_num_out.last().is_some_and(|c| c.is_zero()) { h_num_out.pop(); }
    if g_num_out.is_empty() { g_num_out.push(ExtPoly::zero(var)); }
    if h_num_out.is_empty() { h_num_out.push(ExtPoly::zero(var)); }

    Ok(HermiteResultTwoLevel { g_num: g_num_out, g_den, h_num: h_num_out, h_den })
}
```

#### Rothstein-Trager (two-level)

```rust
/// Two-level Rothstein-Trager resultant: R(z) = res_θ₂(d, a − z·D(d)).
///
/// d ∈ Q(x)[θ₂] (standard ExtPoly), a is Vec<ExtPoly> (two-level, θ₁ coefficients).
/// D(d) ∈ Q(x)[θ₂] (derivative in the tower).
///
/// Returns R(z) as Vec<ExtPoly> — polynomial in z with ExtPoly-in-θ₁ coefficients.
fn rothstein_trager_two_level(
    d: &ExtPoly,
    a: &[ExtPoly],
    dd: &ExtPoly,
    var: &str,
) -> Vec<ExtPoly> {
    let m = d.degree().unwrap_or(0);
    let n = {
        let da = if a.is_empty() { 0 } else { a.len() - 1 };
        let ddd = dd.degree().unwrap_or(0);
        da.max(ddd)
    };

    if m == 0 && n == 0 {
        // Both constant in θ₂: R(z) = a₀ − z·dd₀
        let c0 = a.first().cloned().unwrap_or_else(|| ExtPoly::zero(var));
        let c1_rf = -&dd.coeff(0);
        let c1 = ExtPoly::from_rf(c1_rf);
        return vec![c0, c1];
    }

    let size = m + n;
    if size == 0 {
        return vec![ExtPoly::one(var)];
    }

    // Build Sylvester matrix with Vec<ExtPoly> entries (polynomial in z).
    // Each entry is a Vec<ExtPoly>: [constant_term, z_coefficient]
    let zero_z: Vec<ExtPoly> = vec![ExtPoly::zero(var)];
    let mut matrix: Vec<Vec<Vec<ExtPoly>>> = Vec::with_capacity(size);

    // First n rows from d (no z, no θ₁ — entries are constant-in-z ExtPolys)
    for i in 0..n {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=m {
            let col = i + k;
            if col < size {
                row[col] = vec![ExtPoly::from_rf(d.coeff(m - k))];
            }
        }
        matrix.push(row);
    }

    // Last m rows from g = a − z·dd (linear in z, θ₁ in constant term)
    for i in 0..m {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=n {
            let col = i + k;
            if col < size {
                let a_coeff = a.get(n - k).cloned().unwrap_or_else(|| ExtPoly::zero(var));
                let dd_coeff = dd.coeff(n - k);
                if dd_coeff.is_zero() {
                    row[col] = vec![a_coeff];
                } else {
                    // Entry = a_coeff − z·dd_coeff
                    row[col] = vec![a_coeff, ExtPoly::from_rf(-&dd_coeff)];
                }
            }
        }
        matrix.push(row);
    }

    two_level_det(&matrix, var)
}

/// Determinant of a square matrix whose entries are Vec<ExtPoly>
/// (polynomials in z with ExtPoly-in-θ₁ coefficients).
/// Uses cofactor expansion for small matrices (Risch degrees ≤ 5).
fn two_level_det(m: &[Vec<Vec<ExtPoly>>], var: &str) -> Vec<ExtPoly> {
    let n = m.len();
    if n == 0 {
        return vec![ExtPoly::one(var)];
    }
    if n == 1 {
        return m[0][0].clone();
    }
    if n == 2 {
        let a = mul_two_level(&m[0][0], &m[1][1], var);
        let b = mul_two_level(&m[0][1], &m[1][0], var);
        return sub_two_level(&a, &b, var);
    }
    let mut result = vec![ExtPoly::zero(var)];
    for j in 0..n {
        if m[0][j].iter().all(|ep| ep.is_zero()) {
            continue;
        }
        let minor: Vec<Vec<Vec<ExtPoly>>> = (1..n)
            .map(|row| {
                (0..n)
                    .filter(|&col| col != j)
                    .map(|col| m[row][col].clone())
                    .collect()
            })
            .collect();
        let cofactor = two_level_det(&minor, var);
        let term = mul_two_level(&m[0][j], &cofactor, var);
        if j % 2 == 0 {
            result = add_two_level(&result, &term, var);
        } else {
            result = sub_two_level(&result, &term, var);
        }
    }
    result
}

/// Subtract two Vec<ExtPoly> polynomials.
fn sub_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    let neg_b = negate_two_level(b);
    add_two_level(a, &neg_b, var)
}
```

#### Constant root finding (two-level)

```rust
/// Find constant roots c ∈ Q of R(z) where R has ExtPoly (θ₁) coefficients.
///
/// Strategy: specialize x → x₀ and θ₁ → 0 to get Q[z], find rational roots,
/// verify each candidate as an ExtPoly identity.
fn find_constant_roots_two_level(rz: &[ExtPoly], var: &str) -> Vec<BigRational> {
    let deg = match rz.len().checked_sub(1) {
        Some(d) if d > 0 => d,
        _ => return vec![],
    };

    // Check if R(z) is just the zero polynomial
    if rz.iter().all(|ep| ep.is_zero()) {
        return vec![];
    }

    let candidates_x = [2i64, 3, 5, 7, 11];
    let mut candidate_roots: Option<Vec<BigRational>> = None;

    for &x_val in &candidates_x {
        let x_br = BigRational::from_integer(BigInt::from(x_val));
        let mut spec_coeffs = Vec::with_capacity(deg + 1);
        let mut valid = true;
        for k in 0..=deg {
            // rz[k] is an ExtPoly in θ₁. Evaluate at θ₁=0 means take the degree-0 coefficient.
            // Then specialize x → x₀ in that RationalFunction.
            let rf_at_0 = rz[k].coeff(0);
            match rf_at_0.evaluate(&x_br) {
                Some(val) => spec_coeffs.push(val),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }

        let spec_poly = Polynomial::from_coeffs(spec_coeffs, "z");
        if spec_poly.is_zero() {
            continue;
        }

        candidate_roots = Some(spec_poly.rational_roots());
        break;
    }

    let candidates = match candidate_roots {
        Some(c) => c,
        None => return vec![],
    };

    // Verify each candidate: R(c) must be zero as an ExtPoly
    let mut verified = Vec::new();
    for c in candidates {
        let mut sum = ExtPoly::zero(var);
        let mut c_power = BigRational::one();
        for k in 0..=deg {
            let scaled = rz[k].scalar_mul(&RationalFunction::from_constant(c_power.clone(), var));
            sum = &sum + &scaled;
            c_power = &c_power * &c;
        }
        if sum.is_zero() && !verified.contains(&c) {
            verified.push(c);
        }
    }

    verified
}
```

**Step 4: Run tests to verify they pass**

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add two-level Hermite reduction and Rothstein-Trager with θ₁ coefficients"
```

---

### Task 3: Two-level rational integration pipeline

**Files:**
- Modify: `src/risch.rs` — add `integrate_rational_two_level`, modify `try_risch_two_level`

Implement the full integration pipeline for rational-in-θ₂ integrands with θ₁ coefficients: Hermite reduce → polynomial quotient integration → Rothstein-Trager for squarefree remainder → residual integration → result assembly.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_rational_two_level_ln_over_1_plus_exp_non_elementary() {
    // ∫ln(x)/(1+exp(x)) dx → non-elementary
    let num = vec![ExtPoly::theta("x")];  // θ₁
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ₂
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::NonElementary(_) => {}
        r => panic!("Expected non-elementary, got {:?}", r),
    }
}

#[test]
fn test_integrate_rational_two_level_exp_ln_over_1_plus_exp_non_elementary() {
    // ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary
    let num = vec![ExtPoly::zero("x"), ExtPoly::theta("x")];  // θ₁·θ₂
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::NonElementary(_) => {}
        r => panic!("Expected non-elementary, got {:?}", r),
    }
}
```

**Step 2: Implement `integrate_rational_two_level`**

```rust
/// Integrate a rational function in θ₂ = exp(g(x)) with θ₁ = ln(x) coefficients.
///
/// num is a Vec<ExtPoly> (polynomial in θ₂ with θ₁ coefficients).
/// den is an ExtPoly ∈ Q(x)[θ₂] (no θ₁ in denominator).
///
/// Pipeline: polynomial division → Hermite reduce → RT → residual.
fn integrate_rational_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    if den.is_zero() {
        return None;
    }

    // Polynomial long division: separate quotient and proper fraction
    let (quotient, remainder) = div_rem_two_level_by_extpoly(num, den, var)?;

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let g_node = outer_ext.argument().numerator().to_node();
    let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
    let mut result_terms: Vec<Node> = Vec::new();

    // Integrate the polynomial quotient
    if !quotient.iter().all(|ep| ep.is_zero()) {
        match integrate_two_level_exp_log(&quotient, inner_ext, outer_ext, var) {
            Some(RischResult::Elementary(n)) => result_terms.push(n),
            Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
            None => return None,
        }
    }

    // Handle the proper rational part remainder/den
    if !remainder.iter().all(|ep| ep.is_zero()) {
        // Hermite reduce
        let hr = hermite_reduce_two_level(&remainder, den, var).ok()?;

        // Rational part from Hermite reduction
        if !hr.g_num.iter().all(|ep| ep.is_zero()) {
            let g_num_node = two_level_to_node(&hr.g_num, &ln_x, &exp_g, var);
            let g_den_node = extpoly_to_node(&hr.g_den, &exp_g, var);
            result_terms.push(Node::Divide(Box::new(g_num_node), Box::new(g_den_node)));
        }

        // Squarefree remainder
        if !hr.h_num.iter().all(|ep| ep.is_zero()) {
            if hr.h_den.is_constant() {
                // Polynomial remainder — integrate via two-level polynomial path
                match integrate_two_level_exp_log(&hr.h_num, inner_ext, outer_ext, var) {
                    Some(RischResult::Elementary(n)) => result_terms.push(n),
                    Some(RischResult::NonElementary(r)) => {
                        return Some(RischResult::NonElementary(r))
                    }
                    None => return None,
                }
            } else {
                // Rothstein-Trager on squarefree remainder
                let dd = outer_ext.differentiate(&hr.h_den);
                let rz = rothstein_trager_two_level(&hr.h_den, &hr.h_num, &dd, var);
                let roots = find_constant_roots_two_level(&rz, var);

                if roots.is_empty() {
                    return Some(RischResult::NonElementary(
                        "No elementary antiderivative exists. \
                         The two-level Rothstein-Trager resultant has no constant roots."
                            .into(),
                    ));
                }

                // Build log terms and compute GCD for each root
                let h_den_deg = hr.h_den.degree().unwrap_or(0);
                let mut gcd_deg_sum = 0;

                for c in &roots {
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    // g_c = h_num − c·D(d), as two-level
                    let dd_scaled: Vec<ExtPoly> = dd.coeffs_iter()
                        .map(|coeff| ExtPoly::from_rf(&coeff * &c_rf))
                        .collect();
                    // Wait — dd is standard ExtPoly. Convert to two-level and subtract.
                    let mut g_c = hr.h_num.clone();
                    for (i, ep) in dd_scaled.iter().enumerate() {
                        if i < g_c.len() {
                            g_c[i] = &g_c[i] - ep;
                        }
                        // If i >= g_c.len(), we'd need to extend, but
                        // practically deg(dd) <= deg(d) = deg(h_den) > deg(h_num)
                    }

                    // For degree-1 denominators: check if d divides g_c by evaluation
                    if h_den_deg == 1 {
                        // d = d₀ + d₁·θ₂, evaluate g_c at θ₂ = −d₀/d₁
                        let d0 = hr.h_den.coeff(0);
                        let d1 = hr.h_den.coeff(1);
                        let eval_point = -&(&d0 / &d1);  // scalar: RationalFunction

                        // Evaluate g_c at θ₂ = eval_point
                        let mut val = ExtPoly::zero(var);
                        let mut pt_power = RationalFunction::one(var);
                        for gc_i in &g_c {
                            val = &val + &gc_i.scalar_mul(&pt_power);
                            pt_power = &pt_power * &eval_point;
                        }

                        if val.is_zero() {
                            // d divides g_c — full denominator is a factor
                            gcd_deg_sum += h_den_deg;
                            let v_node = extpoly_to_node(&hr.h_den, &exp_g, var);
                            let ln_v = Node::Function("ln".to_string(), vec![v_node]);
                            let term = if *c == BigRational::one() {
                                ln_v
                            } else {
                                Node::Multiply(Box::new(bigrat_to_node(c)), Box::new(ln_v))
                            };
                            result_terms.push(term);
                        }
                    } else {
                        // Higher-degree denominators: not yet supported
                        return None;
                    }
                }

                if gcd_deg_sum != h_den_deg {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         Rational residues cover degree {} but denominator has degree {}.",
                        gcd_deg_sum, h_den_deg
                    )));
                }

                // For exponential extensions: compute residual
                // (same logic as single-level integrate_rational_ext)
                // ... handle residual if needed
            }
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = result_terms.remove(0);
    for t in result_terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}
```

Also need helper functions:

```rust
/// Polynomial long division: two-level numerator / ExtPoly denominator.
/// Returns (quotient, remainder) as two-level polys.
/// Requires den's leading coefficient to be invertible (nonzero Q(x)).
fn div_rem_two_level_by_extpoly(
    num: &[ExtPoly],
    den: &ExtPoly,
    var: &str,
) -> Option<(Vec<ExtPoly>, Vec<ExtPoly>)> {
    if den.is_zero() {
        return None;
    }
    let den_deg = den.degree().unwrap();
    let num_deg = num.len().checked_sub(1)?;

    if num_deg < den_deg {
        return Some((vec![ExtPoly::zero(var)], num.to_vec()));
    }

    let den_lc = den.leading_coeff().unwrap();
    let den_lc_inv = RationalFunction::one(var).checked_div(den_lc).ok()?;

    let mut remainder = num.to_vec();
    let mut quotient = vec![ExtPoly::zero(var); num_deg - den_deg + 1];

    while remainder.len() > den_deg {
        let rem_deg = remainder.len() - 1;
        let rem_lc = remainder.last().unwrap().clone();
        if rem_lc.is_zero() {
            remainder.pop();
            continue;
        }

        let q_coeff = rem_lc.scalar_mul(&den_lc_inv);
        let deg_diff = rem_deg - den_deg;
        quotient[deg_diff] = q_coeff.clone();

        // Subtract q_coeff * den (shifted by deg_diff) from remainder
        for k in 0..=den_deg {
            let den_k = den.coeff(k);
            if den_k.is_zero() {
                continue;
            }
            let sub = q_coeff.scalar_mul(&den_k);
            let idx = deg_diff + k;
            remainder[idx] = &remainder[idx] - &sub;
        }
        remainder.pop(); // remove leading term (should be zero now)
    }

    // Strip trailing zeros from remainder
    while remainder.last().is_some_and(|c| c.is_zero()) {
        remainder.pop();
    }
    if remainder.is_empty() {
        remainder.push(ExtPoly::zero(var));
    }

    Some((quotient, remainder))
}

/// Convert a two-level polynomial to a Node.
/// For Σᵢ cᵢ(θ₁)·θ₂ⁱ, produce the sum of cᵢ_node * exp_node^i.
fn two_level_to_node(
    coeffs: &[ExtPoly],
    theta1_node: &Node,
    theta2_node: &Node,
    var: &str,
) -> Node {
    let mut terms: Vec<Node> = Vec::new();
    for (i, ci) in coeffs.iter().enumerate() {
        if ci.is_zero() {
            continue;
        }
        let ci_node = extpoly_to_node(ci, theta1_node, var);
        let term = if i == 0 {
            ci_node
        } else {
            let theta2_power = if i == 1 {
                theta2_node.clone()
            } else {
                Node::Power(
                    Box::new(theta2_node.clone()),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            Node::Multiply(Box::new(ci_node), Box::new(theta2_power))
        };
        terms.push(term);
    }
    if terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    result
}
```

**Step 3: Modify `try_risch_two_level` to dispatch rational case**

After the existing polynomial path, add:

```rust
// Try rational case: expression has Divide with θ₂ in denominator
if let Some((num_tl, den_tl)) = extract_two_level_rational(expr, var, &exp_poly) {
    // Convert den_tl to standard ExtPoly (verify no θ₁ in denominator)
    let den_ep = two_level_to_extpoly(&den_tl, var)?;
    return integrate_rational_two_level(&num_tl, &den_ep, &inner_ext, &outer_ext, var);
}

// Also try after simplification
let env = crate::environment::Environment::new();
let simplified = crate::simplify::Simplifiable::simplify(expr, &env)
    .unwrap_or_else(|_| expr.clone());
if let Some((num_tl, den_tl)) = extract_two_level_rational(&simplified, var, &exp_poly) {
    let den_ep = two_level_to_extpoly(&den_tl, var)?;
    return integrate_rational_two_level(&num_tl, &den_ep, &inner_ext, &outer_ext, var);
}
```

Where `two_level_to_extpoly` converts a Vec<ExtPoly> to a standard ExtPoly by checking that all coefficients have no θ₁ terms (degree 0 in θ₁):

```rust
fn two_level_to_extpoly(tl: &[ExtPoly], var: &str) -> Option<ExtPoly> {
    let coeffs: Vec<RationalFunction> = tl.iter()
        .map(|ep| {
            if ep.degree().unwrap_or(0) > 0 {
                None
            } else {
                Some(ep.coeff(0))
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ExtPoly::from_coeffs(coeffs, var))
}
```

**Step 4: Run tests, clippy, full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add integrate_rational_two_level: rational-in-exp with ln coefficients"
```

---

### Task 4: End-to-end tests and wiring

**Files:**
- Modify: `tests/integration.rs` — add end-to-end tests with LaTeX input

**Step 1: Add tests**

```rust
// ===== Two-level tower: rational exp + ln integration =====

#[test]
fn test_integrate_ln_x_over_1_plus_exp_non_elementary() {
    // ∫ln(x)/(1+exp(x)) dx → non-elementary
    let result = integrate_latex("\\frac{\\ln(x)}{1 + \\exp(x)}", "x");
    assert!(
        result.is_err(),
        "∫ln(x)/(1+exp(x))dx should be non-elementary: {:?}",
        result,
    );
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}

#[test]
fn test_integrate_exp_ln_over_1_plus_exp_non_elementary() {
    // ∫exp(x)·ln(x)/(1+exp(x)) dx → non-elementary
    let result = integrate_latex("\\frac{\\exp(x) \\cdot \\ln(x)}{1 + \\exp(x)}", "x");
    assert!(
        result.is_err(),
        "∫exp(x)·ln(x)/(1+exp(x))dx should be non-elementary: {:?}",
        result,
    );
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Step 2: Run tests, verify they pass**

**Step 3: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 4: Commit**

```bash
git add tests/integration.rs src/risch.rs
git commit -m "Wire two-level rational integration: ln(x)/(1+exp(x)) non-elementarity"
```

---

### Task 5: Update documentation

**Files:**
- Modify: `KNUTH-PLAN.md`
- Modify: `README.md`

Update test count, add two-level rational key results, update Phase 9 remaining items.

**Step 1: Update KNUTH-PLAN.md**

- Update test count
- Add Session 24b entry to Completed Work
- Update Phase 9 Remaining items (remove rational-in-θ₂ with θ₁ coefficients)
- Add key results

**Step 2: Update README.md**

- Update test count
- Add CLI example for `ln(x)/(1+exp(x))` non-elementary

**Step 3: Commit**

```bash
git add KNUTH-PLAN.md README.md
git commit -m "doc updates"
```

---

## Non-goals for this session

1. **θ₁ in denominator** — e.g., `exp(x)/(ln(x) + exp(x))`. Requires mixed-coefficient GCD.
2. **Higher-degree GCD verification** — for denominators of degree ≥ 2 in θ₂, the GCD computation after finding RT roots is more complex. Currently returns None for these.
3. **Elementary rational results** — most two-level rational integrals are non-elementary. Full elementary result construction (log terms + residual integration) is partially implemented but the common case is non-elementarity detection.

## Test matrix

| Test | Type | Expected |
|------|------|----------|
| ln(x)/(1+exp(x)) | Non-elementary | RT resultant has θ₁ term → no constant roots |
| exp(x)·ln(x)/(1+exp(x)) | Non-elementary | After poly division: quotient non-elem + remainder non-elem |
| 1/(1+exp(x)) (through two-level, no θ₁) | Elementary | Regression: RT root at z=−1, same as single-level |
| ln(x)/(1+exp(x))² | Non-elementary | Hermite reduces, then RT on squarefree remainder |
