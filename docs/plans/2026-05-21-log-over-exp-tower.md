# Log-Over-Exp Two-Level Tower Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate polynomial expressions in θ₂=ln(h(x, exp(g(x)))) with exp(g(x)) coefficients — e.g., ∫ln(1+exp(x))dx — via top-down logarithmic polynomial integration, with non-elementarity proofs.

**Architecture:** Extend the two-level tower to handle log-over-exp: Q(x) ⊂ Q(x, θ₁=exp(g(x))) ⊂ Q(x, θ₁, θ₂=ln(h(x,θ₁))). Reuse the Vec<ExtPoly> representation (index i = coefficient of θ₂ⁱ, each ExtPoly is polynomial in θ₁). The logarithmic polynomial integration works top-down: degree n gives D(bₙ) = aₙ (integrate in inner exp extension). Lower degrees: D(bₖ) = aₖ − (k+1)·bₖ₊₁·h'/h, where h'/h is rational in θ₁, making the RHS rational — dispatched to the existing `integrate_rational_ext`. Non-elementarity is detected when inner integration fails at any degree.

**Tech Stack:** Rust, existing `ExtPoly`, `RationalFunction`, `DifferentialExtension`, `solve_risch_de_rational`, `integrate_rational_ext`, two-level helpers.

**Reference:** Bronstein, *Symbolic Integration I*, §5.3 (polynomial part — logarithmic case).

---

## Mathematical Foundation

### Tower

Q(x) ⊂ Q(x, θ₁=exp(g(x))) ⊂ Q(x, θ₁, θ₂=ln(h(x, θ₁)))

where h ∈ Q(x)[θ₁] (polynomial in θ₁ with Q(x) coefficients).

### Derivative of the outer extension

D(θ₂) = D(h)/h = h'/h ∈ Q(x)(θ₁)

where h' = ∂h/∂x + g'·θ₁·∂h/∂θ₁ is computed using the exp extension rules. Since h ∈ Q(x)[θ₁], h' ∈ Q(x)[θ₁], and D(θ₂) = h'/h is a rational function in θ₁.

### Logarithmic polynomial integration (top-down)

Given ∫p(θ₂)dx where p = Σᵢ aᵢ·θ₂ⁱ, aᵢ ∈ Q(x)[θ₁]:

Look for q = Σᵢ bᵢ·θ₂ⁱ such that D(q) = p.

D(q) = Σᵢ [D(bᵢ)·θ₂ⁱ + i·bᵢ·D(θ₂)·θ₂ⁱ⁻¹]

Matching coefficients from top down:
- **Degree n:** D(bₙ) = aₙ. Integrate aₙ in Q(x, θ₁=exp(g)).
- **Degree k < n:** D(bₖ) = aₖ − (k+1)·bₖ₊₁·D(θ₂) = aₖ − (k+1)·bₖ₊₁·h'/h = (aₖ·h − (k+1)·bₖ₊₁·h') / h

The RHS at degree k < n is rational in θ₁ (because h'/h is rational). Integrate via `integrate_rational_ext` for rational RHS, or `integrate_poly_exp` for polynomial RHS.

### Structured inner integration

To continue the top-down descent, we need bₖ₊₁ as an ExtPoly (structured), not just a Node. The function `integrate_poly_in_exp_structured` integrates an ExtPoly in the exp extension by solving the Risch DE per θ₁-degree, returning the result as an ExtPoly. When the inner integration produces non-polynomial results (log terms from RT), we fall back to `integrate_rational_ext` which returns a Node — at that point we can still detect non-elementarity but can't continue structured descent.

### Worked example: ∫ln(1+exp(x)) dx

Tower: θ₁ = exp(x), θ₂ = ln(1+θ₁). h = 1+θ₁, h' = θ₁ (since D(1+θ₁) = θ₁).

Integrand: θ₂. So a₁ = 1, a₀ = 0.

Degree 1: D(b₁) = 1. In the exp extension, ∫1 dx = x. So b₁ = x (ExtPoly: [x, 0]).

Degree 0: D(b₀) = 0 − 1·x·θ₁/(1+θ₁) = −x·θ₁/(1+θ₁).
This is rational in θ₁: numerator = −x·θ₁, denominator = 1+θ₁.
Call `integrate_rational_ext` → Rothstein-Trager finds no constant roots → **non-elementary**.

---

## Tasks

### Task 1: Detection — `find_ln_of_exp_argument`

**Files:**
- Modify: `src/risch.rs` — add detection function near `find_exp_argument` (~line 1115)

Detect `ln(f)` in the AST where `f` contains `exp(g(x))` but is NOT just `x` (that's handled by the existing exp-over-log tower).

**Step 1: Write failing tests**

```rust
#[test]
fn test_find_ln_of_exp_basic() {
    // ln(1+exp(x)) → Some((poly [0,1], ExtPoly [1,1]))
    let expr = Node::Function(
        "ln".to_string(),
        vec![Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        )],
    );
    let (g, h) = find_ln_of_exp_argument(&expr, "x").unwrap();
    assert_eq!(g, poly(&[0, 1], "x")); // g(x) = x
    // h = 1 + θ₁
    assert_eq!(h, ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"));
}

#[test]
fn test_find_ln_of_exp_nested() {
    // exp(x) * ln(1+exp(x)) → Some((poly [0,1], ExtPoly [1,1]))
    let ln_part = Node::Function(
        "ln".to_string(),
        vec![Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        )],
    );
    let exp_part = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let expr = Node::Multiply(Box::new(exp_part), Box::new(ln_part));
    let (g, h) = find_ln_of_exp_argument(&expr, "x").unwrap();
    assert_eq!(g, poly(&[0, 1], "x"));
    assert_eq!(h, ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"));
}

#[test]
fn test_find_ln_of_exp_none_for_ln_x() {
    // ln(x) → None (not ln-of-exp)
    let expr = Node::Function(
        "ln".to_string(),
        vec![Node::Variable("x".to_string())],
    );
    assert!(find_ln_of_exp_argument(&expr, "x").is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_find_ln_of_exp -- --nocapture 2>&1 | head -10`

**Step 3: Implement `find_ln_of_exp_argument`**

```rust
/// Find ln(f(exp(g(x)))) in the expression tree.
/// Returns (g, h) where g is the exp argument polynomial and h is the ln argument
/// parsed as an ExtPoly in θ₁ = exp(g(x)).
/// Returns None if no ln-of-exp pattern is found.
fn find_ln_of_exp_argument(expr: &Node, var: &str) -> Option<(Polynomial, ExtPoly)> {
    match expr {
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            // Check if argument contains exp
            let exp_arg = find_exp_argument(&args[0], var)?;
            // Parse the ln argument as ExtPoly in θ₁ = exp(g)
            let kind = ExtensionKind::Exponential(exp_arg.clone());
            let h = node_to_extpoly_general(&args[0], var, &kind)?;
            // Must have θ₁ terms (otherwise it's just ln(f(x)), not ln-of-exp)
            if h.degree().unwrap_or(0) == 0 {
                return None;
            }
            Some((exp_arg, h))
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            find_ln_of_exp_argument(l, var).or_else(|| find_ln_of_exp_argument(r, var))
        }
        Node::Negate(inner) => find_ln_of_exp_argument(inner, var),
        Node::Power(base, _) => find_ln_of_exp_argument(base, var),
        _ => None,
    }
}
```

**Step 4: Run tests, clippy, full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add find_ln_of_exp_argument: detect ln(f(exp(g))) patterns in AST"
```

---

### Task 2: Parser — `node_to_two_level_log_over_exp`

**Files:**
- Modify: `src/risch.rs` — add parser near `node_to_two_level` (~line 1745)

Parse an expression as a polynomial in θ₂=ln(h(x,θ₁)) with ExtPoly (in θ₁=exp(g)) coefficients. Similar to `node_to_two_level` but with roles swapped: exp is inner (recognized as ExtPoly coefficients), ln(h) is outer (recognized as θ₂).

**Step 1: Write failing tests**

```rust
#[test]
fn test_log_over_exp_parse_bare_ln() {
    // ln(1+exp(x)) → [0, 1] (= θ₂)
    let expr = Node::Function(
        "ln".to_string(),
        vec![Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        )],
    );
    let exp_arg = poly(&[0, 1], "x");
    let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    assert_eq!(result[1], ExtPoly::one("x"));
}

#[test]
fn test_log_over_exp_parse_exp_times_ln() {
    // exp(x) * ln(1+exp(x)) → [0, θ₁] (= θ₁·θ₂)
    let ln_part = Node::Function(
        "ln".to_string(),
        vec![Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        )],
    );
    let exp_part = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let expr = Node::Multiply(Box::new(exp_part), Box::new(ln_part));
    let exp_arg = poly(&[0, 1], "x");
    let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].is_zero());
    // θ₁ = ExtPoly with coefficients [0, 1]
    let theta1 = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    assert_eq!(result[1], theta1);
}

#[test]
fn test_log_over_exp_parse_constant() {
    // 3 → [3] (constant)
    let expr = Node::Num(ExactNum::integer(3));
    let exp_arg = poly(&[0, 1], "x");
    let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = node_to_two_level_log_over_exp(&expr, "x", &exp_arg, &h).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], ExtPoly::from_rf(rf_const(3)));
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement `node_to_two_level_log_over_exp`**

```rust
/// Parse an expression as polynomial in θ₂ = ln(h(x, θ₁)) with ExtPoly (θ₁=exp(g)) coefficients.
/// Returns Vec<ExtPoly> where index i = coefficient of θ₂ⁱ.
fn node_to_two_level_log_over_exp(
    expr: &Node,
    var: &str,
    exp_arg: &Polynomial,
    h: &ExtPoly,
) -> Option<Vec<ExtPoly>> {
    let kind = ExtensionKind::Exponential(exp_arg.clone());
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
            // exp(g(x)) → θ₁ as coefficient (degree 0 in θ₂)
            if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                if arg_poly == *exp_arg {
                    let theta1 = ExtPoly::from_coeffs(vec![rf_const_var(0, var), rf_const_var(1, var)], var);
                    return Some(vec![theta1]);
                }
            }
            None
        }
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            // Check if this is ln(h) — our θ₂
            let arg_ep = node_to_extpoly_general(&args[0], var, &kind)?;
            if arg_ep == *h {
                return Some(vec![ExtPoly::zero(var), ExtPoly::one(var)]);
            }
            None
        }
        Node::Power(base, exp_node) => {
            // Handle ln(h)^n = θ₂^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "ln" && args.len() == 1 {
                    let arg_ep = node_to_extpoly_general(&args[0], var, &kind)?;
                    if arg_ep == *h {
                        if let Node::Num(n) = exp_node.as_ref() {
                            if let Some(e) = n.to_i64() {
                                if e >= 1 {
                                    let mut result = vec![ExtPoly::zero(var); e as usize + 1];
                                    result[e as usize] = ExtPoly::one(var);
                                    return Some(result);
                                }
                            }
                        }
                    }
                }
            }
            // Handle exp(g)^n = θ₁^n
            if let Node::Function(name, args) = base.as_ref() {
                if name == "exp" && args.len() == 1 {
                    if let Ok(arg_poly) = Polynomial::from_node(&args[0], var) {
                        if arg_poly == *exp_arg {
                            if let Node::Num(n) = exp_node.as_ref() {
                                if let Some(e) = n.to_i64() {
                                    if e >= 1 {
                                        let mut coeffs = vec![RationalFunction::zero(var); e as usize + 1];
                                        coeffs[e as usize] = RationalFunction::one(var);
                                        return Some(vec![ExtPoly::from_coeffs(coeffs, var)]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        Node::Negate(inner) => {
            let v = node_to_two_level_log_over_exp(inner, var, exp_arg, h)?;
            Some(negate_two_level(&v))
        }
        Node::Add(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(add_two_level(&left, &right, var))
        }
        Node::Subtract(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(sub_two_level(&left, &right, var))
        }
        Node::Multiply(l, r) => {
            let left = node_to_two_level_log_over_exp(l, var, exp_arg, h)?;
            let right = node_to_two_level_log_over_exp(r, var, exp_arg, h)?;
            Some(mul_two_level(&left, &right, var))
        }
        Node::Divide(num, den) => {
            // Only handle x-polynomial denominators (no θ₁ or θ₂ in denominator)
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let num_v = node_to_two_level_log_over_exp(num, var, exp_arg, h)?;
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            let inv_ep = ExtPoly::from_rf(inv);
            Some(scalar_mul_two_level(&num_v, &inv_ep))
        }
        _ => None,
    }
}
```

Note: `rf_const_var` is needed because the existing `rf_const` helper in tests always uses "x". In the implementation, use `RationalFunction::from_constant(BigRational::from_integer(BigInt::from(n)), var)` directly, or since all calls are with var="x", `rf_const` works in tests.

**Step 4: Run tests, clippy, full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add node_to_two_level_log_over_exp: parse polynomial-in-ln(h) with exp coefficients"
```

---

### Task 3: Structured inner integration — `integrate_in_exp_ext_structured`

**Files:**
- Modify: `src/risch.rs` — add function near `integrate_poly_exp` (~line 1375)

This function integrates an ExtPoly (polynomial in θ₁=exp(g)) and returns the result as an ExtPoly. It is the structured analogue of `integrate_poly_exp` — same algorithm, but returns ExtPoly instead of Node.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_in_exp_structured_constant() {
    // ∫1 dx in exp(x) extension → x
    let p = ExtPoly::from_rf(rf_const(1));
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let result = integrate_in_exp_ext_structured(&p, &ext, "x").unwrap();
    // x as ExtPoly: coefficient of θ₁⁰ is x
    assert_eq!(result.coeff(0), rf_poly(&[0, 1]));
    assert_eq!(result.degree(), Some(0));
}

#[test]
fn test_integrate_in_exp_structured_theta1() {
    // ∫θ₁ dx where θ₁ = exp(x), so D(b)=θ₁ means b₁'+b₁=0 (degree 1) and b₀'=0 (degree 0)
    // Wait: D(b₁·θ₁) = (b₁'+g'·b₁)·θ₁ = θ₁ → b₁'+b₁=1 → b₁=1 (constant solution)
    // So ∫exp(x) dx = exp(x) → ExtPoly [0, 1]
    let p = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let result = integrate_in_exp_ext_structured(&p, &ext, "x").unwrap();
    assert_eq!(result.coeff(1), rf_const(1));
}

#[test]
fn test_integrate_in_exp_structured_non_elementary() {
    // ∫exp(x²) dx is non-elementary. θ₁=exp(x²), integrand=θ₁.
    // DE: q'+2x·q=0 at degree 0, q'+2x·q=1 at degree 1.
    // degree 1: q'+2x·q=1 has no polynomial solution → non-elementary.
    let p = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 0, 1], "x")), "x", // g = x²
    );
    assert!(integrate_in_exp_ext_structured(&p, &ext, "x").is_none());
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement `integrate_in_exp_ext_structured`**

```rust
/// Integrate an ExtPoly in the exponential extension, returning the result as an ExtPoly.
///
/// For p = Σ aᵢ·θ₁ⁱ, each degree decouples:
///   i=0: b₀ = ∫a₀ dx (must be polynomial in x)
///   i≥1: solve b_i' + i·g'·b_i = a_i (Risch DE, rational solution)
///
/// Returns None if any degree has no rational solution (non-elementary).
fn integrate_in_exp_ext_structured(
    p: &ExtPoly,
    ext: &DifferentialExtension,
    var: &str,
) -> Option<ExtPoly> {
    let deg = p.degree().unwrap_or(0);
    let g_prime_rf = ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None;
    }
    let g_prime = g_prime_rf.numerator().clone();

    let mut result_coeffs: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];

    for i in 0..=deg {
        let a_i = p.coeff(i);
        if a_i.is_zero() {
            continue;
        }

        if i == 0 {
            if *a_i.denominator() != Polynomial::one(var) {
                return None;
            }
            result_coeffs[0] = RationalFunction::from_poly(a_i.numerator().clone().integral());
        } else {
            let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_rational(&f, &a_i, var) {
                Some(qi) => result_coeffs[i] = qi,
                None => return None,
            }
        }
    }

    Some(ExtPoly::from_coeffs(result_coeffs, var))
}
```

**Step 4: Run tests, clippy, full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add integrate_in_exp_ext_structured: inner exp integration returning ExtPoly"
```

---

### Task 4: Log-over-exp polynomial integration + wiring

**Files:**
- Modify: `src/risch.rs` — add integration function, extend `try_risch_two_level`

**Step 1: Write failing test**

```rust
#[test]
fn test_integrate_two_level_log_over_exp_non_elementary() {
    // ∫ln(1+exp(x)) dx → non-elementary
    // Degree 1: D(b₁) = 1 → b₁ = x ✓
    // Degree 0: D(b₀) = −x·θ₁/(1+θ₁) → RT has no constant roots → non-elementary
    let outer_coeffs = vec![ExtPoly::zero("x"), ExtPoly::one("x")]; // θ₂
    let inner_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ₁
    match integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected non-elementary, got {:?}", other),
    }
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement `integrate_two_level_log_over_exp`**

```rust
/// Integrate a polynomial in θ₂ = ln(h(x, θ₁)) with ExtPoly coefficients.
///
/// Uses top-down logarithmic polynomial integration:
///   Degree n: D(bₙ) = aₙ (integrate in inner exp extension)
///   Degree k < n: D(bₖ) = aₖ − (k+1)·bₖ₊₁·h'/h
fn integrate_two_level_log_over_exp(
    outer_coeffs: &[ExtPoly],
    inner_ext: &DifferentialExtension,
    h: &ExtPoly,
    var: &str,
) -> Option<RischResult> {
    if outer_coeffs.is_empty() || outer_coeffs.iter().all(|c| c.is_zero()) {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }

    // Effective degree
    let n = {
        let mut d = outer_coeffs.len() - 1;
        while d > 0 && outer_coeffs[d].is_zero() {
            d -= 1;
        }
        d
    };

    // Compute h' = D(h) in the exp extension
    let h_prime = inner_ext.differentiate(h);

    let g_node = inner_ext.argument().numerator().to_node();
    let exp_g = Node::Function("exp".to_string(), vec![g_node]);
    let h_node = extpoly_to_node(h, &exp_g, var);
    let ln_h = Node::Function("ln".to_string(), vec![h_node]);

    let mut result_terms: Vec<Node> = Vec::new();
    let mut b_prev: Option<ExtPoly> = None;

    for k in (0..=n).rev() {
        let a_k = &outer_coeffs[k];

        if k == n {
            // Top degree: D(bₙ) = aₙ — integrate in inner exp extension
            match integrate_in_exp_ext_structured(a_k, inner_ext, var) {
                Some(b_k) => {
                    if !b_k.is_zero() {
                        let b_node = extpoly_to_node(&b_k, &exp_g, var);
                        let theta2_pow = make_ln_power(&ln_h, k);
                        result_terms.push(Node::Multiply(Box::new(b_node), Box::new(theta2_pow)));
                    }
                    b_prev = Some(b_k);
                }
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         Cannot integrate the degree-{} coefficient in the inner exp extension.",
                        k
                    )));
                }
            }
        } else {
            // Lower degree: D(bₖ) = aₖ − (k+1)·bₖ₊₁·h'/h
            // = (aₖ·h − (k+1)·bₖ₊₁·h') / h
            let b_prev_ref = b_prev.as_ref().unwrap();
            let scale_int = BigRational::from_integer(BigInt::from((k + 1) as i64));
            let scale_rf = RationalFunction::from_constant(scale_int, var);
            let correction = b_prev_ref.scalar_mul(&scale_rf);
            let correction_h_prime = &correction * &h_prime;

            let a_k_times_h = a_k * h;
            let rhs_num = &a_k_times_h - &correction_h_prime;

            // Try to simplify rhs_num / h
            let g = rhs_num.gcd(h);
            let (rhs_num_reduced, _) = rhs_num.div_rem(&g).unwrap();
            let (rhs_den_reduced, _) = h.div_rem(&g).unwrap();

            if rhs_den_reduced.is_constant() {
                // Polynomial RHS — integrate structurally
                let rhs_den_scalar = rhs_den_reduced.coeff(0);
                let rhs_poly = if rhs_den_scalar == RationalFunction::one(var) {
                    rhs_num_reduced
                } else {
                    let inv = RationalFunction::one(var).checked_div(&rhs_den_scalar).ok()?;
                    rhs_num_reduced.scalar_mul(&inv)
                };

                match integrate_in_exp_ext_structured(&rhs_poly, inner_ext, var) {
                    Some(b_k_ep) => {
                        if !b_k_ep.is_zero() {
                            let b_node = extpoly_to_node(&b_k_ep, &exp_g, var);
                            if k == 0 {
                                result_terms.push(b_node);
                            } else {
                                let theta2_pow = make_ln_power(&ln_h, k);
                                result_terms.push(Node::Multiply(
                                    Box::new(b_node),
                                    Box::new(theta2_pow),
                                ));
                            }
                        }
                        b_prev = Some(b_k_ep);
                    }
                    None => {
                        return Some(RischResult::NonElementary(format!(
                            "No elementary antiderivative exists. \
                             Cannot integrate the degree-{} correction in the inner exp extension.",
                            k
                        )));
                    }
                }
            } else {
                // Rational RHS — use integrate_rational_ext
                match integrate_rational_ext(&rhs_num_reduced, &rhs_den_reduced, inner_ext, var) {
                    Some(RischResult::NonElementary(reason)) => {
                        return Some(RischResult::NonElementary(reason));
                    }
                    Some(RischResult::Elementary(node)) => {
                        if k > 0 {
                            // Need structured b_k for next step — can't extract from Node
                            return None;
                        }
                        result_terms.push(node);
                    }
                    None => return None,
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

fn make_ln_power(ln_node: &Node, k: usize) -> Node {
    if k == 0 {
        Node::Num(ExactNum::integer(1))
    } else if k == 1 {
        ln_node.clone()
    } else {
        Node::Power(
            Box::new(ln_node.clone()),
            Box::new(Node::Num(ExactNum::integer(k as i64))),
        )
    }
}
```

**Step 4: Wire into `try_risch_two_level`**

At the top of `try_risch_two_level`, before the existing exp-over-log code, add the log-over-exp detection:

```rust
// Try log-over-exp tower: ln(h(x, exp(g(x))))
if let Some((exp_poly, h)) = find_ln_of_exp_argument(expr, var) {
    if let Some(outer_coeffs) = node_to_two_level_log_over_exp(expr, var, &exp_poly, &h) {
        let inner_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(exp_poly.clone()),
            var,
        );
        if let Some(result) = integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h, var) {
            return Some(result);
        }
    }
    // Try after simplification
    let env = crate::environment::Environment::new();
    let simplified = crate::simplify::Simplifiable::simplify(expr, &env)
        .unwrap_or_else(|_| expr.clone());
    if let Some((exp_poly2, h2)) = find_ln_of_exp_argument(&simplified, var) {
        if let Some(outer_coeffs) = node_to_two_level_log_over_exp(&simplified, var, &exp_poly2, &h2) {
            let inner_ext = DifferentialExtension::exponential(
                RationalFunction::from_poly(exp_poly2),
                var,
            );
            if let Some(result) = integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h2, var) {
                return Some(result);
            }
        }
    }
}
```

**Step 5: Run tests, clippy, full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add integrate_two_level_log_over_exp: top-down logarithmic polynomial integration"
```

---

### Task 5: End-to-end tests and documentation

**Files:**
- Modify: `tests/integration.rs` — add e2e tests
- Modify: `src/risch.rs` — add more unit tests
- Modify: `KNUTH-PLAN.md`, `README.md`

**Step 1: Add end-to-end tests**

```rust
#[test]
fn test_integrate_ln_1_plus_exp_x_non_elementary() {
    // ∫ln(1+exp(x)) dx → non-elementary (involves Li₂)
    let result = integrate_latex("\\ln(1 + \\exp(x))", "x");
    assert!(
        result.is_err(),
        "∫ln(1+exp(x))dx should be non-elementary: {:?}",
        result,
    );
    assert!(
        result.unwrap_err().starts_with("NON_ELEMENTARY:"),
        "Should be NON_ELEMENTARY"
    );
}
```

**Step 2: Add unit test for exp(x)·ln(1+exp(x))**

```rust
#[test]
fn test_integrate_two_level_log_over_exp_exp_times_ln_non_elementary() {
    // ∫exp(x)·ln(1+exp(x)) dx → non-elementary
    // Degree 1: D(b₁) = θ₁, solve DE per θ₁-degree: b₁=θ₁ ✓
    // Degree 0: D(b₀) = −1·θ₁·θ₁/(1+θ₁) = −θ₁²/(1+θ₁) → rational → RT non-elementary
    let theta1 = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let outer_coeffs = vec![ExtPoly::zero("x"), theta1]; // θ₁·θ₂
    let inner_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x",
    );
    let h = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    match integrate_two_level_log_over_exp(&outer_coeffs, &inner_ext, &h, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected non-elementary, got {:?}", other),
    }
}
```

**Step 3: Run all tests**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 4: Update documentation**

In `KNUTH-PLAN.md`:
- Update test count
- Add "log-over-exp polynomial tower integration" to the multi-extension towers description
- Add key results: ∫ln(1+exp(x)) dx → non-elementary ✓
- Update Phase 9 Remaining items

In `README.md`:
- Update test count

**Step 5: Commit**

```bash
git add src/risch.rs tests/integration.rs KNUTH-PLAN.md README.md
git commit -m "Add end-to-end tests for log-over-exp tower, doc updates"
```

---

## Non-goals for this session

1. **Rational-in-θ₂ integrands** — e.g., 1/ln(1+exp(x)). Requires Hermite + RT for the log extension.
2. **Rational-in-θ₁ coefficients** — e.g., exp(x)/(1+exp(x))·ln(1+exp(x)). Requires richer coefficient representation.
3. **Elementary results with non-polynomial inner antiderivatives** — when the degree-0 correction step produces an elementary result with log terms, we can't continue structured descent. Returns None.

## Test matrix

| Test | Type | Expected |
|------|------|----------|
| ln(1+exp(x)) | Unit + E2E | Non-elementary (degree-0 RT fails) |
| exp(x)·ln(1+exp(x)) | Unit | Non-elementary (degree-0 RT fails) |
| find_ln_of_exp ln(1+exp(x)) | Unit | Detects (g=[0,1], h=[1,1]) |
| find_ln_of_exp exp(x)·ln(...) | Unit | Detects in subexpression |
| find_ln_of_exp ln(x) | Unit | None (not ln-of-exp) |
| Parser: ln(1+exp(x)) | Unit | [0, 1] = θ₂ |
| Parser: exp(x)·ln(1+exp(x)) | Unit | [0, θ₁] = θ₁·θ₂ |
| Parser: constant 3 | Unit | [3] |
| Structured exp integral: ∫1 | Unit | x |
| Structured exp integral: ∫θ₁ | Unit | θ₁ |
| Structured exp integral: ∫exp(x²) | Unit | None (non-elem) |
