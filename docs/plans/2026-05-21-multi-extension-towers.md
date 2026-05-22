# Multi-Extension Tower Integration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate expressions containing both exp(g(x)) and ln(x) via a two-level Risch tower, enabling both elementary antiderivative computation and rigorous non-elementarity proofs for mixed transcendental integrands.

**Architecture:** Build a two-level tower Q(x) ⊂ Q(x, θ₁=ln(x)) ⊂ Q(x, θ₁, θ₂=exp(g(x))) with exp on top. The integrand is expressed as a polynomial in θ₂ whose coefficients are ExtPolys in θ₁. Each θ₂-degree decouples into a Risch DE that, when expanded in the θ₁ basis, reduces to a triangular system of standard Risch DEs over Q(x) — all solvable by existing infrastructure. The θ₂-degree-0 coefficient (pure ln(x) polynomial) integrates via the existing `integrate_poly_log`. No new types needed: the outer polynomial is a `Vec<ExtPoly>`, reusing `ExtPoly` for inner coefficients.

**Tech Stack:** Rust, existing `ExtPoly`, `RationalFunction`, `Polynomial`, `DifferentialExtension`, `solve_risch_de_rational`, `integrate_poly_log`.

**Reference:** Manuel Bronstein, *Symbolic Integration I: Transcendental Functions*, Chapter 5 (integration in towers of transcendental extensions).

---

## Mathematical Foundation

### Tower structure

For an integrand f(x, ln(x), exp(g(x))), the tower is:

```
K₀ = Q(x)
K₁ = Q(x, θ₁)     where θ₁ = ln(x),    θ₁' = 1/x
K₂ = Q(x, θ₁, θ₂)  where θ₂ = exp(g(x)), θ₂' = g'(x)·θ₂
```

The integrand is Σ aᵢ(x, θ₁)·θ₂ⁱ where each aᵢ ∈ K₁ = Q(x)[θ₁].

### Integration algorithm

Each θ₂-degree decouples:

- **Degree 0:** ∫a₀ dx in K₁ — handled by `integrate_poly_log`.
- **Degree i ≥ 1:** Solve Risch DE: qᵢ' + i·g'·qᵢ = aᵢ where qᵢ ∈ K₁.

### Inner Risch DE solver

For qᵢ = Σ bⱼ(x)·θ₁ʲ, the full derivative in the tower is:

```
qᵢ' = Σ [bⱼ'·θ₁ʲ + j·bⱼ·(1/x)·θ₁ʲ⁻¹]
```

Collecting by θ₁-degree k: coefficient of θ₁ᵏ in qᵢ' is bₖ' + (k+1)·b_{k+1}/x.

The Risch DE qᵢ' + f·qᵢ = aᵢ (where f = i·g' is a polynomial in x) at θ₁-degree k:

```
bₖ' + f·bₖ = cₖ − (k+1)·b_{k+1}/x
```

where cₖ = aᵢ.coeff(k). Top-down from degree n:

- **Degree n:** bₙ' + f·bₙ = cₙ → standard Risch DE, solve with `solve_risch_de_rational`
- **Degree k < n:** bₖ' + f·bₖ = cₖ − (k+1)·b_{k+1}/x → standard Risch DE with known RHS

If any level has no rational solution, the integral is non-elementary.

### Degree bound

The degree of qᵢ in θ₁ is bounded by deg(aᵢ) in θ₁, since f ∈ Q[x] doesn't introduce θ₁ terms and the derivative in the tower shifts θ₁-degree down by at most 1.

### Verification examples

**Elementary:** ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x)

Tower: a₁ = θ₁ + 1/x. Risch DE: q₁' + q₁ = θ₁ + 1/x.
- θ₁¹: b₁' + b₁ = 1 → b₁ = 1
- θ₁⁰: b₀' + b₀ = 1/x − 1/x = 0 → b₀ = 0
- Result: q₁ = θ₁ = ln(x), antiderivative = ln(x)·exp(x) ✓

**Non-elementary:** ∫exp(x)·ln(x) dx

Tower: a₁ = θ₁. Risch DE: q₁' + q₁ = θ₁.
- θ₁¹: b₁' + b₁ = 1 → b₁ = 1
- θ₁⁰: b₀' + b₀ = −1/x → no rational solution (simple pole at x=0)
- Non-elementary ✓ (reduces to Ei(x))

---

## Tasks

### Task 1: Node-to-two-level conversion

**Files:**
- Modify: `src/risch.rs` (add before `#[cfg(test)]` block)

Add a function that converts a Node expression containing both exp(g(x)) and ln(x) into a two-level polynomial representation: `Vec<ExtPoly>` indexed by θ₂-degree, where each `ExtPoly` is a polynomial in θ₁ = ln(x) with Q(x) coefficients.

**Step 1: Write failing tests**

Add these tests in `src/risch.rs` inside `mod tests`:

```rust
// === Two-level tower tests ===

#[test]
fn test_two_level_exp_times_ln() {
    // exp(x)·ln(x) → [0, θ₁] (degree 1 in θ₂, coeff is θ₁)
    let expr = Node::Multiply(
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    assert_eq!(result[1], ExtPoly::theta("x")); // θ₁
}

#[test]
fn test_two_level_exp_times_ln_plus_exp_over_x() {
    // exp(x)·ln(x) + exp(x)/x → [0, θ₁ + 1/x]
    let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let exp_ln = Node::Multiply(Box::new(exp_x.clone()), Box::new(ln_x));
    let exp_over_x = Node::Divide(
        Box::new(exp_x),
        Box::new(Node::Variable("x".to_string())),
    );
    let expr = Node::Add(Box::new(exp_ln), Box::new(exp_over_x));
    let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    // coeff of θ₂ should be θ₁ + 1/x
    let expected_coeff = ExtPoly::from_coeffs(vec![
        RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")),  // 1/x
        RationalFunction::one("x"),  // 1 · θ₁
    ], "x");
    assert_eq!(result[1], expected_coeff);
}

#[test]
fn test_two_level_exp_times_ln_squared() {
    // exp(x)·ln(x)² → [0, θ₁²]
    let exp_x = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let ln_x_sq = Node::Power(
        Box::new(ln_x),
        Box::new(Node::Num(ExactNum::integer(2))),
    );
    let expr = Node::Multiply(Box::new(exp_x), Box::new(ln_x_sq));
    let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    // coeff of θ₂ should be θ₁²
    let theta = ExtPoly::theta("x");
    let theta_sq = &theta * &theta;
    assert_eq!(result[1], theta_sq);
}

#[test]
fn test_two_level_just_exp() {
    // exp(x) alone → [0, 1] (no ln, inner coeffs are constants)
    let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    assert_eq!(result[1], ExtPoly::from_rf(RationalFunction::one("x")));
}

#[test]
fn test_two_level_constant() {
    // 3 → [3] (constant, no θ₂ or θ₁)
    let expr = Node::Num(ExactNum::integer(3));
    let result = node_to_two_level(&expr, "x", &poly(&[0, 1], "x")).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], ExtPoly::from_rf(rf_const(3)));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_two_level -- --nocapture 2>&1 | head -20`
Expected: compilation errors (function doesn't exist yet)

**Step 3: Implement `node_to_two_level`**

The function converts a Node to `Vec<ExtPoly>` — a polynomial in θ₂ = exp(g(x)) where each coefficient is an ExtPoly in θ₁ = ln(x) with Q(x) coefficients.

```rust
/// Convert a Node containing both exp(g(x)) and ln(x) into a two-level
/// polynomial: Vec<ExtPoly> indexed by θ₂-degree, where each ExtPoly
/// is a polynomial in θ₁ = ln(x) with Q(x) coefficients.
fn node_to_two_level(expr: &Node, var: &str, exp_arg: &Polynomial) -> Option<Vec<ExtPoly>> {
    match expr {
        Node::Num(n) => {
            if let ExactNum::Rational(val) = n {
                let rf = RationalFunction::from_constant(val.clone(), var);
                Some(vec![ExtPoly::from_rf(rf)])
            } else {
                None
            }
        }
        Node::Variable(v) if v == var => {
            let rf = RationalFunction::from_poly(Polynomial::x(var));
            Some(vec![ExtPoly::from_rf(rf)])
        }
        Node::Variable(_) => None,
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                if arg_poly == *exp_arg {
                    return Some(vec![ExtPoly::zero(var), ExtPoly::one(var)]);
                }
            }
            None
        }
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            if let Node::Variable(v) = &args[0] {
                if v == var {
                    return Some(vec![ExtPoly::theta(var)]);
                }
            }
            None
        }
        Node::Power(base, exp) => {
            // Handle ln(x)^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "ln" && args.len() == 1 {
                    if let Node::Variable(v) = &args[0] {
                        if v == var {
                            if let Node::Num(n) = exp.as_ref() {
                                if let Some(e) = n.to_i64() {
                                    if e >= 1 {
                                        let mut r = ExtPoly::theta(var);
                                        for _ in 1..e {
                                            r = &r * &ExtPoly::theta(var);
                                        }
                                        return Some(vec![r]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Handle x^n
            if let Node::Variable(v) = base.as_ref() {
                if v == var {
                    if let Node::Num(n) = exp.as_ref() {
                        if let Some(e) = n.to_i64() {
                            if e >= 1 {
                                let p = Polynomial::monomial(
                                    BigRational::one(), e as usize, var,
                                );
                                return Some(vec![ExtPoly::from_rf(
                                    RationalFunction::from_poly(p),
                                )]);
                            }
                        }
                    }
                }
            }
            None
        }
        Node::Negate(inner) => {
            let v = node_to_two_level(inner, var, exp_arg)?;
            Some(negate_two_level(&v, var))
        }
        Node::Add(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            Some(add_two_level(&left, &right, var))
        }
        Node::Subtract(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            let neg_right = negate_two_level(&right, var);
            Some(add_two_level(&left, &neg_right, var))
        }
        Node::Multiply(l, r) => {
            let left = node_to_two_level(l, var, exp_arg)?;
            let right = node_to_two_level(r, var, exp_arg)?;
            Some(mul_two_level(&left, &right, var))
        }
        Node::Divide(num, den) => {
            // Only handle division by x-polynomials (fold into coefficients)
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let num_v = node_to_two_level(num, var, exp_arg)?;
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            let inv_ep = ExtPoly::from_rf(inv);
            Some(scalar_mul_two_level(&num_v, &inv_ep, var))
        }
        _ => None,
    }
}

/// Negate a two-level polynomial.
fn negate_two_level(v: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    v.iter().map(|c| -c).collect()
}

/// Add two two-level polynomials (pad the shorter one with zeros).
fn add_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    let len = a.len().max(b.len());
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let ai = a.get(i).cloned().unwrap_or_else(|| ExtPoly::zero(var));
        let bi = b.get(i).cloned().unwrap_or_else(|| ExtPoly::zero(var));
        result.push(&ai + &bi);
    }
    // Strip trailing zeros
    while result.last().is_some_and(|c| c.is_zero()) {
        result.pop();
    }
    if result.is_empty() {
        result.push(ExtPoly::zero(var));
    }
    result
}

/// Multiply two two-level polynomials (convolution on θ₂-degrees,
/// ExtPoly multiplication on θ₁ coefficients).
fn mul_two_level(a: &[ExtPoly], b: &[ExtPoly], var: &str) -> Vec<ExtPoly> {
    if a.is_empty() || b.is_empty() {
        return vec![ExtPoly::zero(var)];
    }
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![ExtPoly::zero(var); result_len];
    for (i, ai) in a.iter().enumerate() {
        if ai.is_zero() { continue; }
        for (j, bj) in b.iter().enumerate() {
            if bj.is_zero() { continue; }
            let product = ai * bj;
            result[i + j] = &result[i + j] + &product;
        }
    }
    // Strip trailing zeros
    while result.last().is_some_and(|c| c.is_zero()) {
        result.pop();
    }
    if result.is_empty() {
        result.push(ExtPoly::zero(var));
    }
    result
}

/// Multiply a two-level polynomial by an ExtPoly scalar (θ₁-level only).
fn scalar_mul_two_level(v: &[ExtPoly], s: &ExtPoly, var: &str) -> Vec<ExtPoly> {
    let result: Vec<ExtPoly> = v.iter().map(|c| c * s).collect();
    // Strip trailing zeros
    let mut r = result;
    while r.last().is_some_and(|c| c.is_zero()) {
        r.pop();
    }
    if r.is_empty() {
        r.push(ExtPoly::zero(var));
    }
    r
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_two_level -- --nocapture`
Expected: all 5 tests pass

**Step 5: Run clippy + full suite**

Run: `cargo clippy --tests -- -D warnings && cargo test`
Expected: 0 warnings, 772+ tests pass

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add node_to_two_level: parse mixed exp+ln into two-level polynomial"
```

---

### Task 2: Inner Risch DE solver for the log extension

**Files:**
- Modify: `src/risch.rs` (add before `#[cfg(test)]`)

Add `solve_risch_de_in_log_ext`: given f ∈ Q[x] and g ∈ Q(x)[θ₁] (an ExtPoly in θ₁ = ln(x)), find q ∈ Q(x)[θ₁] satisfying q' + f·q = g, or return None if no solution exists. The derivative q' is taken in the tower using θ₁' = 1/x.

The algorithm: expand q = Σ bₖ(x)·θ₁ᵏ and solve top-down. At each θ₁-degree k, the equation bₖ' + f·bₖ = cₖ − (k+1)·b_{k+1}/x is a standard Risch DE over Q(x), solved by `solve_risch_de_rational`.

**Step 1: Write failing tests**

```rust
#[test]
fn test_inner_de_constant_rhs() {
    // q' + q = 1 → q = 1 (ExtPoly with just constant term)
    let f = poly(&[1], "x");
    let g = ExtPoly::from_rf(rf_const(1));
    let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
    assert_eq!(result, ExtPoly::from_rf(rf_const(1)));
}

#[test]
fn test_inner_de_theta1_rhs_elementary() {
    // q' + q = θ₁ + 1/x
    // Solution: q = θ₁ (b₁ = 1, b₀ = 0)
    // Check: q' = 1/x, q' + q = 1/x + θ₁ = θ₁ + 1/x ✓
    let f = poly(&[1], "x");
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let g = ExtPoly::from_coeffs(vec![one_over_x, RationalFunction::one("x")], "x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
    assert_eq!(result, ExtPoly::theta("x"));
}

#[test]
fn test_inner_de_theta1_rhs_non_elementary() {
    // q' + q = θ₁ (just ln(x), no 1/x correction)
    // b₁' + b₁ = 1 → b₁ = 1
    // b₀' + b₀ = -1/x → no rational solution (simple pole)
    let f = poly(&[1], "x");
    let g = ExtPoly::from_coeffs(vec![RationalFunction::zero("x"), RationalFunction::one("x")], "x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x");
    assert!(result.is_none());
}

#[test]
fn test_inner_de_theta1_squared_rhs() {
    // q' + q = θ₁² + 2θ₁/x
    // b₂' + b₂ = 1 → b₂ = 1
    // b₁' + b₁ = 2/x − 2·1/x = 0 → b₁ = 0
    // b₀' + b₀ = 0 − 1·0/x = 0 → b₀ = 0
    // Solution: q = θ₁²
    let f = poly(&[1], "x");
    let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
    let g = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        two_over_x,
        RationalFunction::one("x"),
    ], "x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
    let theta = ExtPoly::theta("x");
    assert_eq!(result, &theta * &theta);
}

#[test]
fn test_inner_de_zero_rhs() {
    // q' + q = 0 → q = 0
    let f = poly(&[1], "x");
    let g = ExtPoly::zero("x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
    assert!(result.is_zero());
}

#[test]
fn test_inner_de_2x_coefficient() {
    // q' + 2x·q = 2x (from exp(x²) integration)
    // q = 1 (b₀ = 1)
    // Check: 0 + 2x·1 = 2x ✓
    let f = poly(&[0, 2], "x");
    let g = ExtPoly::from_rf(rf_poly(&[0, 2]));
    let result = solve_risch_de_in_log_ext(&f, &g, "x").unwrap();
    assert_eq!(result, ExtPoly::from_rf(rf_const(1)));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_inner_de -- --nocapture 2>&1 | head -20`
Expected: compilation errors

**Step 3: Implement `solve_risch_de_in_log_ext`**

```rust
/// Solve the Risch DE q' + f·q = g in the logarithmic extension field Q(x)[θ₁],
/// where θ₁ = ln(x) and f ∈ Q[x].
///
/// Returns q ∈ Q(x)[θ₁] as an ExtPoly, or None if no solution exists.
///
/// The derivative q' uses the tower rule: d/dx[θ₁] = 1/x, so
/// d/dx[Σ bₖ·θ₁ᵏ] = Σ (bₖ' + (k+1)·b_{k+1}/x)·θ₁ᵏ.
///
/// At each θ₁-degree k (top-down from n):
///   bₖ' + f·bₖ = gₖ − (k+1)·b_{k+1}/x
///
/// Each is a standard Risch DE over Q(x), solved by `solve_risch_de_rational`.
fn solve_risch_de_in_log_ext(
    f: &Polynomial,
    g: &ExtPoly,
    var: &str,
) -> Option<ExtPoly> {
    if g.is_zero() {
        return Some(ExtPoly::zero(var));
    }

    let n = g.degree().unwrap_or(0);
    let mut b: Vec<RationalFunction> = vec![RationalFunction::zero(var); n + 1];
    let x_rf = RationalFunction::from_poly(Polynomial::x(var));

    for k in (0..=n).rev() {
        let g_k = g.coeff(k);

        // Correction from higher degree: (k+1)·b_{k+1}/x
        let correction = if k < n {
            let scale = BigRational::from_integer(BigInt::from(k as i64 + 1));
            let scaled_b = &b[k + 1]
                * &RationalFunction::from_constant(scale, var);
            match scaled_b.checked_div(&x_rf) {
                Ok(result) => result,
                Err(_) => return None,
            }
        } else {
            RationalFunction::zero(var)
        };

        let rhs = &g_k - &correction;

        if rhs.is_zero() && f.is_zero() {
            // 0·q = 0, b_k is free — take b_k = 0
            continue;
        }

        // Solve bₖ' + f·bₖ = rhs
        match solve_risch_de_rational(f, &rhs, var) {
            Some(bk) => b[k] = bk,
            None => return None,
        }
    }

    Some(ExtPoly::from_coeffs(b, var))
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_inner_de -- --nocapture`
Expected: all 6 tests pass

**Step 5: Run clippy + full suite**

Run: `cargo clippy --tests -- -D warnings && cargo test`
Expected: 0 warnings, all tests pass

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add solve_risch_de_in_log_ext: Risch DE solver over Q(x)[ln(x)]"
```

---

### Task 3: Two-level integration function

**Files:**
- Modify: `src/risch.rs` (add before `#[cfg(test)]`)

Add `integrate_two_level_exp_log`: given the outer coefficients (polynomial in θ₂ with ExtPoly-in-θ₁ coefficients), integrate using the two-level algorithm. Returns a `RischResult`.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_two_level_exp_ln_non_elementary() {
    // ∫exp(x)·ln(x) dx → non-elementary
    // Coeffs: [0, θ₁]
    let coeffs = vec![ExtPoly::zero("x"), ExtPoly::theta("x")];
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::NonElementary(_) => {}
        r => panic!("Expected non-elementary, got {:?}", r),
    }
}

#[test]
fn test_integrate_two_level_exp_ln_plus_exp_over_x() {
    // ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x)
    // Coeffs: [0, θ₁ + 1/x]
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let coeff1 = ExtPoly::from_coeffs(vec![one_over_x, RationalFunction::one("x")], "x");
    let coeffs = vec![ExtPoly::zero("x"), coeff1];
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Expected ln in {}", s);
            assert!(s.contains("exp"), "Expected exp in {}", s);
        }
        r => panic!("Expected elementary, got {:?}", r),
    }
}

#[test]
fn test_integrate_two_level_exp_ln_sq_plus_correction() {
    // ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)²
    // Coeffs: [0, θ₁² + 2θ₁/x]
    let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
    let coeff1 = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        two_over_x,
        RationalFunction::one("x"),
    ], "x");
    let coeffs = vec![ExtPoly::zero("x"), coeff1];
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Expected ln in {}", s);
            assert!(s.contains("exp"), "Expected exp in {}", s);
        }
        r => panic!("Expected elementary, got {:?}", r),
    }
}

#[test]
fn test_integrate_two_level_pure_exp() {
    // ∫exp(x) dx = exp(x) — should still work through two-level path
    // Coeffs: [0, 1]
    let coeffs = vec![ExtPoly::zero("x"), ExtPoly::from_rf(rf_const(1))];
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    match integrate_two_level_exp_log(&coeffs, &inner_ext, &outer_ext, "x").unwrap() {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("exp"), "Expected exp in {}", s);
        }
        r => panic!("Expected elementary, got {:?}", r),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_integrate_two_level -- --nocapture 2>&1 | head -20`
Expected: compilation errors

**Step 3: Implement `integrate_two_level_exp_log`**

```rust
/// Integrate a polynomial in θ₂ = exp(g(x)) whose coefficients are
/// ExtPolys in θ₁ = ln(x) with Q(x) coefficients.
///
/// For each θ₂-degree i:
/// - i = 0: integrate in Q(x, θ₁) via `integrate_poly_log`
/// - i ≥ 1: solve Risch DE qᵢ' + i·g'·qᵢ = aᵢ via `solve_risch_de_in_log_ext`
///
/// Returns the antiderivative as a Node, or proves non-elementarity.
fn integrate_two_level_exp_log(
    outer_coeffs: &[ExtPoly],
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let deg = match outer_coeffs.len().checked_sub(1) {
        Some(d) => d,
        None => return Some(RischResult::Elementary(Node::Num(ExactNum::zero()))),
    };

    let g_prime_rf = outer_ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();
    let g_node = outer_ext.argument().numerator().to_node();

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let mut result_terms: Vec<Node> = Vec::new();

    for i in 0..=deg {
        let a_i = &outer_coeffs[i];
        if a_i.is_zero() {
            continue;
        }

        if i == 0 {
            // Integrate a₀ in Q(x, θ₁) via integrate_poly_log
            match integrate_poly_log(a_i, inner_ext, var) {
                Some(RischResult::Elementary(node)) => {
                    result_terms.push(node);
                }
                Some(RischResult::NonElementary(reason)) => {
                    return Some(RischResult::NonElementary(reason));
                }
                None => return None,
            }
        } else {
            // Solve: qᵢ' + i·g'·qᵢ = aᵢ
            let f_scaled = g_prime.scalar_mul(
                &BigRational::from_integer(BigInt::from(i as i64)),
            );
            match solve_risch_de_in_log_ext(&f_scaled, a_i, var) {
                Some(qi) => {
                    // Build node: qi_node · exp(g(x))^i
                    let qi_node = extpoly_to_node(&qi, &ln_x, var);
                    let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
                    let exp_part = if i == 1 {
                        exp_g
                    } else {
                        Node::Power(
                            Box::new(exp_g),
                            Box::new(Node::Num(ExactNum::integer(i as i64))),
                        )
                    };
                    let term = Node::Multiply(
                        Box::new(qi_node),
                        Box::new(exp_part),
                    );
                    result_terms.push(term);
                }
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         The Risch DE q' + ({})·q = {} has no solution in Q(x, ln(x)), \
                         so the integral cannot be expressed in terms of elementary functions.",
                        f_scaled, a_i
                    )));
                }
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

**Step 4: Run tests to verify they pass**

Run: `cargo test test_integrate_two_level -- --nocapture`
Expected: all 4 tests pass

**Step 5: Run clippy + full suite**

Run: `cargo clippy --tests -- -D warnings && cargo test`
Expected: 0 warnings, all tests pass

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add integrate_two_level_exp_log: two-level Risch integration for exp+ln"
```

---

### Task 4: Wire into `try_risch_tower` and build_tower

**Files:**
- Modify: `src/risch.rs` (`try_risch_tower` and `build_tower_inner`)

Extend `try_risch_tower` to try the two-level path when the single-extension tower returns None due to mixed exp+ln. Add a new `try_risch_two_level` dispatcher that uses `build_tower` scanning + `node_to_two_level` + `integrate_two_level_exp_log`.

**Step 1: Write failing integration tests**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_integrate_exp_x_times_ln_x_non_elementary() {
    // ∫exp(x)·ln(x) dx → non-elementary
    let result = integrate_latex("\\exp(x) \\cdot \\ln(x)", "x");
    assert!(result.is_err(), "∫exp(x)·ln(x)dx should be non-elementary");
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}

#[test]
fn test_integrate_exp_x_ln_x_plus_exp_x_over_x() {
    // ∫(exp(x)·ln(x) + exp(x)/x) dx = exp(x)·ln(x) + C
    let result = integrate_latex("\\exp(x) \\cdot \\ln(x) + \\frac{\\exp(x)}{x}", "x");
    assert!(
        result.is_ok(),
        "∫(exp(x)·ln(x) + exp(x)/x) dx should succeed: {:?}", result,
    );
    let s = result.unwrap();
    assert!(s.contains("\\ln"), "Expected ln in {}", s);
    assert!(s.contains("exp"), "Expected exp in {}", s);
    assert!(s.contains("+ C"), "Expected + C in {}", s);
}

#[test]
fn test_integrate_exp_x_ln_x_plus_exp_x_over_x_numerical() {
    // Verify: d/dx[exp(x)·ln(x)] = exp(x)·ln(x) + exp(x)/x
    let result = integrate_latex(
        "\\exp(x) \\cdot \\ln(x) + \\frac{\\exp(x)}{x}", "x",
    ).unwrap();
    let integral_expr = result.replace(" + C", "");
    let mut env = Environment::new();
    env.set("x", 2.0);
    let val = evaluate_expression(&integral_expr, &env).unwrap();
    let expected = 2.0_f64.exp() * 2.0_f64.ln();
    assert!(
        approx_eq(val, expected, 0.01),
        "Expected {:.4}, got {:.4}", expected, val,
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_integrate_exp_x -- --nocapture 2>&1 | tail -20`
Expected: FAIL (the tests exist but the Risch tower doesn't handle mixed exp+ln yet)

**Step 3: Implement `try_risch_two_level`**

```rust
/// Try to integrate via a two-level tower (exp on top of ln).
///
/// Called when `build_tower` returns None because both exp and ln are present.
fn try_risch_two_level(expr: &Node, var: &str) -> Option<RischResult> {
    let has_ln = contains_ln(expr, var);
    let exp_arg = find_exp_argument(expr, var);

    // Need both extensions present
    let exp_poly = match (has_ln, exp_arg) {
        (true, Some(g)) => g,
        _ => return None,
    };

    // Build two-level representation
    let build_result = node_to_two_level(expr, var, &exp_poly);

    // Try simplifying first if direct conversion fails
    let outer_coeffs = match build_result {
        Some(c) => c,
        None => {
            let env = crate::environment::Environment::new();
            let simplified = crate::simplify::Simplifiable::simplify(expr, &env)
                .unwrap_or_else(|_| expr.clone());
            node_to_two_level(&simplified, var, &exp_poly)?
        }
    };

    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(Polynomial::x(var)),
        var,
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(exp_poly),
        var,
    );

    integrate_two_level_exp_log(&outer_coeffs, &inner_ext, &outer_ext, var)
}
```

**Step 4: Modify `try_risch_tower` to call the two-level path**

In `try_risch_tower`, add the two-level fallback after the single-tower attempt:

Change `try_risch_tower` from:

```rust
pub fn try_risch_tower(expr: &Node, var: &str) -> Option<RischResult> {
    let (num, den, ext) = build_tower(expr, var)?;
    // ... existing dispatch ...
}
```

To:

```rust
pub fn try_risch_tower(expr: &Node, var: &str) -> Option<RischResult> {
    // Try single-extension tower first
    if let Some((num, den, ext)) = build_tower(expr, var) {
        if den == ExtPoly::one(var) {
            // ... existing polynomial dispatch ...
        } else if den.is_constant() {
            // ... existing constant-denominator dispatch ...
        } else {
            return integrate_rational_ext(&num, &den, &ext, var);
        }
    }

    // Try two-level tower (exp over ln)
    try_risch_two_level(expr, var)
}
```

This requires restructuring the existing `try_risch_tower` from a single-return-path function into one that tries the single-extension first, then falls through to two-level.

**Step 5: Run tests to verify they pass**

Run: `cargo test test_integrate_exp_x -- --nocapture`
Expected: all 3 new integration tests pass

**Step 6: Run clippy + full suite**

Run: `cargo clippy --tests -- -D warnings && cargo test`
Expected: 0 warnings, all tests pass (772 existing + new tests)

**Step 7: Commit**

```bash
git add src/risch.rs tests/integration.rs
git commit -m "Wire two-level tower into try_risch_tower: exp+ln integration and non-elementarity"
```

---

### Task 5: Additional test coverage and edge cases

**Files:**
- Modify: `tests/integration.rs`
- Modify: `src/risch.rs` (test module)

Add tests for edge cases and verify numerical correctness.

**Step 1: Add unit tests for inner DE edge cases**

In `src/risch.rs` test module:

```rust
#[test]
fn test_inner_de_rational_coeff_success() {
    // q' + q = 1/x + θ₁/x  →  solve in Q(x, θ₁)
    // b₁' + b₁ = 1/x → no rational solution (simple pole)
    // → None (non-elementary)
    let f = poly(&[1], "x");
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let g = ExtPoly::from_coeffs(vec![
        one_over_x.clone(),
        one_over_x,
    ], "x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x");
    assert!(result.is_none());
}

#[test]
fn test_inner_de_higher_f_coefficient() {
    // q' + 2x·q = 2x·θ₁ + 2/x
    // b₁' + 2x·b₁ = 2x → b₁ = 1
    // b₀' + 2x·b₀ = 2/x − 1·1/x = 1/x → no rational solution (simple pole)
    // → None
    let f = poly(&[0, 2], "x");
    let two_x_rf = rf_poly(&[0, 2]);
    let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
    let g = ExtPoly::from_coeffs(vec![two_over_x, two_x_rf], "x");
    let result = solve_risch_de_in_log_ext(&f, &g, "x");
    assert!(result.is_none());
}
```

**Step 2: Add end-to-end integration tests**

In `tests/integration.rs`:

```rust
#[test]
fn test_integrate_exp_x_ln_x_sq_correction_numerical() {
    // ∫(exp(x)·ln(x)² + 2·exp(x)·ln(x)/x) dx = exp(x)·ln(x)² + C
    // d/dx[exp(x)·ln(x)²] = exp(x)·ln(x)² + 2·exp(x)·ln(x)/x
    let result = integrate_latex(
        "\\exp(x) \\cdot \\ln(x)^2 + \\frac{2 \\cdot \\exp(x) \\cdot \\ln(x)}{x}", "x",
    );
    assert!(
        result.is_ok(),
        "Should succeed: {:?}", result,
    );
    let s = result.unwrap();
    let integral_expr = s.replace(" + C", "");
    let mut env = Environment::new();
    env.set("x", 2.0);
    let val = evaluate_expression(&integral_expr, &env).unwrap();
    let expected = 2.0_f64.exp() * 2.0_f64.ln().powi(2);
    assert!(
        approx_eq(val, expected, 0.01),
        "Expected {:.4}, got {:.4}", expected, val,
    );
}

#[test]
fn test_integrate_exp_x_sq_times_ln_x_non_elementary() {
    // ∫exp(x²)·ln(x) dx → non-elementary
    // Even exp(x²) alone is non-elementary
    let result = integrate_latex("\\exp(x^2) \\cdot \\ln(x)", "x");
    assert!(
        result.is_err(),
        "∫exp(x²)·ln(x)dx should be non-elementary",
    );
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Step 3: Run all tests**

Run: `cargo clippy --tests -- -D warnings && cargo test`
Expected: 0 warnings, all tests pass

**Step 4: Commit**

```bash
git add src/risch.rs tests/integration.rs
git commit -m "Add edge case and numerical verification tests for two-level towers"
```

---

### Task 6: Update KNUTH-PLAN.md and documentation

**Files:**
- Modify: `KNUTH-PLAN.md`
- Modify: `README.md`

**Step 1: Update KNUTH-PLAN.md**

Update the "Current State" section to reflect multi-extension tower capability. Update the Phase 9 progress to note that two-extension towers (exp+ln) are now handled. Update the "Remaining" section.

**Step 2: Update README.md**

Add multi-extension examples to the CLI output section and the integration description.

**Step 3: Commit**

```bash
git add KNUTH-PLAN.md README.md
git commit -m "doc updates"
```

---

## Non-goals for this session

These are explicitly out of scope and tracked in the backlog for future work:

1. **Rational functions in θ₂ with θ₁ coefficients** — would need two-level Hermite reduction + Rothstein-Trager. The current implementation handles polynomial-in-θ₂ integrands only.

2. **Log-on-top-of-exp towers** — different tower order, requires integrating inner exp coefficients at each log level. Less common in practice.

3. **Three or more extensions** — recursive tower integration. The architecture generalizes but the implementation effort is significant.

4. **exp(f(x, ln(x))) arguments** — e.g., exp(x·ln(x)). The outer extension's argument could involve the inner extension. This requires care in the derivative computation.

---

## Test matrix summary

| Test | Type | Expected |
|------|------|----------|
| exp(x)·ln(x) | Non-elementary | Proven by simple-pole rejection in inner DE |
| exp(x)·ln(x) + exp(x)/x | Elementary | exp(x)·ln(x), verified numerically |
| exp(x)·ln(x)² + 2·exp(x)·ln(x)/x | Elementary | exp(x)·ln(x)², verified numerically |
| exp(x²)·ln(x) | Non-elementary | Proven by inner DE failure |
| exp(x) (through two-level path) | Elementary | exp(x), regression check |
| Pure constant | Elementary | Trivial |
