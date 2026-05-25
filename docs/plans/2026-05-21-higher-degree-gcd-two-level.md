# Higher-Degree Denominator GCD for Two-Level Rational Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the degree-1 shortcut + "not yet supported" fallback in `integrate_rational_two_level` with a general GCD computation that handles any denominator degree, and fix the residual computation to use per-root log arguments instead of assuming v = h_den.

**Architecture:** Since θ₁ is transcendental over Q(x), factorization of d ∈ Q(x)[θ₂] is the same over Q(x) as over Q(x)(θ₁). Therefore gcd(d, g_c) where g_c has θ₁ coefficients equals gcd(d, r₀, r₁, ..., rₘ) where rⱼ is the θ₁-degree-j component of g_c — all standard ExtPolys. This decomposition reduces the two-level GCD to iterative calls to the existing `ExtPoly::gcd`. The residual computation mirrors the single-level pattern: Σ cᵢ · (h_den/vᵢ) · D(vᵢ), lifted to two-level arithmetic.

**Tech Stack:** Rust, existing `ExtPoly`, `RationalFunction`, `ExtPoly::gcd`, `div_rem_two_level_by_extpoly`, two-level arithmetic helpers.

**Reference:** Bronstein, *Symbolic Integration I*, §5.6 (Rothstein-Trager with GCD computation).

---

## Mathematical Foundation

### The GCD problem

After Rothstein-Trager finds constant root c, we compute g_c = a − c·D(d) and need v = gcd(d, g_c).

- d ∈ Q(x)[θ₂] (standard ExtPoly, squarefree)
- g_c ∈ Q(x)[θ₁][θ₂] (two-level: Vec<ExtPoly>)
- v must be in Q(x)[θ₂] (since it divides d)

### θ₁-component decomposition

Write g_c = Σⱼ θ₁ʲ · rⱼ(θ₂) where each rⱼ ∈ Q(x)[θ₂].

**Theorem:** gcd(d, g_c) in Q(x)(θ₁)[θ₂] = gcd(d, r₀, r₁, ..., rₘ) in Q(x)[θ₂].

**Proof:** θ₁ is transcendental over Q(x), so d factors identically over Q(x) and Q(x)(θ₁). Any common divisor h of d and g_c divides d (so h ∈ Q(x)[θ₂]) and divides each rⱼ (by transcendence of θ₁). Conversely, any common divisor of d and all rⱼ divides g_c.

### Extracting θ₁-components

The two-level polynomial g_c is `Vec<ExtPoly>` where g_c[i] is the coefficient of θ₂ⁱ (an ExtPoly in θ₁). The θ₁-component rⱼ is the ExtPoly with θ₂-coefficient i equal to g_c[i].coeff(j):

```
rⱼ = ExtPoly::from_coeffs([g_c[0].coeff(j), g_c[1].coeff(j), ...], var)
```

### Residual computation (generalized)

The single-level pattern (lines 1564-1596 of risch.rs) computes:

```
log_deriv_num = Σᵢ cᵢ · (h_den/vᵢ) · D(vᵢ)
residual_num = h_num - log_deriv_num
polynomial_residual = residual_num / h_den
```

For two-level, we need the same but with two-level numerators. The key operations:
- h_den/vᵢ: both in Q(x)[θ₂], standard ExtPoly division
- D(vᵢ): standard ExtPoly differentiation
- (h_den/vᵢ) · D(vᵢ): standard ExtPoly multiplication
- cᵢ · result: scalar multiplication, produces single-θ₁-degree term → lift to two-level

---

## Tasks

### Task 1: Add `gcd_extpoly_with_two_level` function and unit tests

**Files:**
- Modify: `src/risch.rs` — add function after `sub_two_level` (~line 2061)

**Step 1: Write failing tests**

Add these tests at the end of the test module (before the closing `}`):

```rust
// === Two-level GCD tests ===

#[test]
fn test_gcd_two_level_full_divisor() {
    // d = θ₂² - 1, g_c = (θ₁-1)(1-θ₂²) = -(θ₁-1)(θ₂²-1)
    // g_c as Vec<ExtPoly>: [(1-θ₁), 0, (θ₁-1)]
    // gcd should be θ₂²-1 (d divides g_c up to θ₁-scalar)
    let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
    let one_minus_theta1 = {
        let mut ep = ExtPoly::theta("x");
        ep = -&ep;
        &ep + &ExtPoly::from_rf(rf_const(1))
    };
    let theta1_minus_one = -&one_minus_theta1;
    let g_c = vec![one_minus_theta1, ExtPoly::zero("x"), theta1_minus_one];
    let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
    let d_monic = d.make_monic();
    let v_monic = v.make_monic();
    assert_eq!(v_monic, d_monic, "GCD should be θ₂²-1 (monic)");
}

#[test]
fn test_gcd_two_level_partial_factor() {
    // d = θ₂²-1 = (θ₂-1)(θ₂+1)
    // g_c = θ₁·θ₂·(1+θ₂) = [0, θ₁, θ₁] (coefficients of θ₂⁰, θ₂¹, θ₂²)
    // θ₁-component j=0: [0, 0, 0] → 0
    // θ₁-component j=1: [0, 1, 1] → θ₂ + θ₂²
    // gcd(θ₂²-1, 0, θ₂+θ₂²) = gcd(θ₂²-1, θ₂(θ₂+1)) = θ₂+1
    let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
    let g_c = vec![
        ExtPoly::zero("x"),
        ExtPoly::theta("x"),
        ExtPoly::theta("x"),
    ];
    let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
    let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // θ₂+1
    assert_eq!(v.make_monic(), expected.make_monic());
}

#[test]
fn test_gcd_two_level_coprime() {
    // d = θ₂²+1, g_c = [θ₁, 1] (= θ₁ + θ₂)
    // θ₁-component j=0: [0, 1] → θ₂
    // θ₁-component j=1: [1, 0] → 1
    // gcd(θ₂²+1, θ₂, 1) = 1
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x");
    let g_c = vec![ExtPoly::theta("x"), ExtPoly::from_rf(rf_const(1))];
    let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
    assert!(v.is_constant(), "GCD should be 1 (constant), got degree {:?}", v.degree());
}

#[test]
fn test_gcd_two_level_no_theta1() {
    // Pure Q(x) coefficients — should match single-level GCD.
    // d = θ₂²-1, g_c = [1, 0, -1] = 1-θ₂² = -(θ₂²-1)
    // gcd = θ₂²-1
    let d = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
    let g_c = vec![
        ExtPoly::from_rf(rf_const(1)),
        ExtPoly::zero("x"),
        ExtPoly::from_rf(rf_const(-1)),
    ];
    let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
    assert_eq!(v.make_monic(), d.make_monic());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_gcd_two_level -- --nocapture 2>&1 | head -20`

Expected: compilation error — `gcd_extpoly_with_two_level` not defined.

**Step 3: Implement `gcd_extpoly_with_two_level`**

Add after the `sub_two_level` function (~line 2061):

```rust
/// Compute gcd(d, g_c) where d ∈ Q(x)[θ₂] and g_c ∈ Q(x)[θ₁][θ₂].
///
/// Since θ₁ is transcendental over Q(x), the GCD equals gcd(d, r₀, r₁, ...)
/// where rⱼ is the coefficient of θ₁ʲ in g_c, extracted as a standard ExtPoly.
fn gcd_extpoly_with_two_level(d: &ExtPoly, g_c: &[ExtPoly], var: &str) -> ExtPoly {
    let max_theta1_deg = g_c
        .iter()
        .filter_map(|ep| ep.degree())
        .max()
        .unwrap_or(0);

    let mut result = d.clone();
    for j in 0..=max_theta1_deg {
        let rj_coeffs: Vec<RationalFunction> = g_c.iter().map(|ep| ep.coeff(j)).collect();
        let rj = ExtPoly::from_coeffs(rj_coeffs, var);
        if !rj.is_zero() {
            result = result.gcd(&rj);
            if result.is_constant() {
                break;
            }
        }
    }
    result
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_gcd_two_level -- --nocapture`

Expected: all 4 tests pass.

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add gcd_extpoly_with_two_level: θ₁-component decomposition for two-level GCD"
```

---

### Task 2: Replace degree-1 shortcut with general GCD in `integrate_rational_two_level`

**Files:**
- Modify: `src/risch.rs:2554-2607` — the root loop and GCD section

Replace the current root loop (lines 2554-2607) with code that mirrors the single-level pattern: compute GCD per root, collect (c, v) pairs, check degree sum.

**Step 1: Write failing test for degree-2 non-elementary**

Add to test module:

```rust
#[test]
fn test_integrate_rational_two_level_degree2_non_elementary() {
    // ∫ln(x)/(1+exp(2x)) dx → non-elementary
    // d = 1 + θ₂² (degree 2 in θ₂=exp(x)), a = [θ₁]
    // R(z) = (2z + θ₁)² → no constant roots → non-elementary
    let num = vec![ExtPoly::theta("x")]; // θ₁
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(0), rf_const(1)], "x"); // 1+θ₂²
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected non-elementary, got {:?}", other),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_integrate_rational_two_level_degree2 -- --nocapture 2>&1 | head -10`

Expected: returns `None` (the "not yet supported" path).

**Step 3: Replace the root loop**

Replace lines 2554–2607 (from `// Build log terms for degree-1 denominators` through the `gcd_deg_sum != h_den_deg` check) with:

```rust
                // Compute GCD and build log terms for each root
                let h_den_deg = hr.h_den.degree().unwrap_or(0);
                let mut gcd_deg_sum = 0;
                let mut log_terms: Vec<(BigRational, ExtPoly)> = Vec::new();

                for c in &roots {
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    // g_c = h_num − c·D(d) as two-level
                    let mut g_c = hr.h_num.clone();
                    let dd_len = dd.degree().map_or(0, |d| d + 1);
                    while g_c.len() < dd_len {
                        g_c.push(ExtPoly::zero(var));
                    }
                    for (i, g_c_i) in g_c.iter_mut().enumerate() {
                        let dd_coeff = dd.coeff(i);
                        if !dd_coeff.is_zero() {
                            let sub = ExtPoly::from_rf(&dd_coeff * &c_rf);
                            *g_c_i = &*g_c_i - &sub;
                        }
                    }

                    let v = gcd_extpoly_with_two_level(&hr.h_den, &g_c, var);
                    let v_deg = v.degree().unwrap_or(0);
                    gcd_deg_sum += v_deg;
                    if v_deg > 0 {
                        log_terms.push((c.clone(), v));
                    }
                }

                if gcd_deg_sum != h_den_deg {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         Rational residues cover degree {} but denominator has degree {}.",
                        gcd_deg_sum, h_den_deg
                    )));
                }

                // Build log terms: Σ cᵢ·ln(vᵢ)
                for (c, v) in &log_terms {
                    let v_node = extpoly_to_node(v, &exp_g, var);
                    let ln_v = Node::Function("ln".to_string(), vec![v_node]);
                    let term = if *c == BigRational::one() {
                        ln_v
                    } else {
                        Node::Multiply(Box::new(bigrat_to_node(c)), Box::new(ln_v))
                    };
                    result_terms.push(term);
                }
```

**Step 4: Run tests**

Run: `cargo test test_integrate_rational_two_level -- --nocapture`

Expected: all pipeline tests pass, including the new degree-2 test.

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Replace degree-1 GCD shortcut with general θ₁-component GCD in two-level RT"
```

---

### Task 3: Fix residual computation to use per-root log arguments

**Files:**
- Modify: `src/risch.rs:2609-2643` — the residual computation section

The current residual computation (lines 2609-2643) assumes v = h_den for every root and computes Σ cᵢ · D(h_den)/h_den as the log derivative sum. This is wrong when roots have different GCD factors. Replace with the single-level pattern: Σ cᵢ · (h_den/vᵢ) · D(vᵢ), using the `log_terms` collected in Task 2.

**Step 1: Write failing test for degree-2 elementary (no θ₁)**

This test routes constant-coefficient degree-2 data through the two-level pipeline to exercise the residual path.

```rust
#[test]
fn test_integrate_rational_two_level_degree2_no_theta1_elementary() {
    // Route 1/(θ₂²-1) through two-level pipeline (no θ₁).
    // ∫1/(exp(2x)-1)dx. d=θ₂²-1, D(d)=2θ₂².
    // R(z) = (1-2z)² → root z=1/2.
    // g_c = 1-(1/2)·2θ₂² = 1-θ₂² = -(θ₂²-1)
    // gcd(θ₂²-1, 1-θ₂²) = θ₂²-1 → v = d.
    // Result: (1/2)·ln(exp(2x)-1) + residual integration
    let num = vec![ExtPoly::from_rf(rf_const(1))]; // 1
    let den = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x"); // θ₂²-1
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_rational_two_level(&num, &den, &inner_ext, &outer_ext, "x") {
        Some(RischResult::Elementary(_)) => {}
        Some(RischResult::NonElementary(msg)) => {
            panic!("Expected elementary, got non-elementary: {}", msg)
        }
        None => panic!("Expected elementary, got None"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_integrate_rational_two_level_degree2_no_theta1 -- --nocapture 2>&1 | head -15`

Expected: likely panics or returns wrong result because residual computation assumes v = h_den globally.

**Step 3: Replace residual computation**

Replace lines 2609–2643 (from `// For exp extensions: compute and integrate residual` to the end of the residual block) with:

```rust
                // For exp extensions: compute and integrate residual
                // residual_num = h_num - Σ cᵢ · (h_den/vᵢ) · D(vᵢ)  (over common den h_den)
                let max_len = hr
                    .h_num
                    .len()
                    .max(dd.degree().map_or(1, |d| d + 1));
                let mut log_deriv_num = vec![ExtPoly::zero(var); max_len];

                for (c, v) in &log_terms {
                    let (w, rem) = hr.h_den.div_rem(v).unwrap();
                    debug_assert!(rem.is_zero(), "v should divide h_den");
                    let dv = outer_ext.differentiate(v);
                    let w_dv = &w * &dv;
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    let scaled = w_dv.scalar_mul(&c_rf);
                    // Lift to two-level: scalar ExtPoly → Vec<ExtPoly> at θ₁-degree 0
                    for i in 0..=scaled.degree().unwrap_or(0) {
                        let coeff = scaled.coeff(i);
                        if !coeff.is_zero() {
                            if i >= log_deriv_num.len() {
                                log_deriv_num
                                    .resize(i + 1, ExtPoly::zero(var));
                            }
                            let term = ExtPoly::from_rf(coeff);
                            log_deriv_num[i] = &log_deriv_num[i] + &term;
                        }
                    }
                }

                let residual = sub_two_level(&hr.h_num, &log_deriv_num, var);

                if !residual.iter().all(|ep| ep.is_zero()) {
                    let (poly_residual, rem) =
                        div_rem_two_level_by_extpoly(&residual, &hr.h_den, var)?;
                    if !rem.iter().all(|ep| ep.is_zero()) {
                        return Some(RischResult::NonElementary(
                            "No elementary antiderivative. \
                             Residual after two-level Rothstein-Trager is not polynomial."
                                .into(),
                        ));
                    }
                    match integrate_two_level_exp_log(
                        &poly_residual,
                        inner_ext,
                        outer_ext,
                        var,
                    ) {
                        Some(RischResult::Elementary(n)) => result_terms.push(n),
                        Some(RischResult::NonElementary(r)) => {
                            return Some(RischResult::NonElementary(r))
                        }
                        None => return None,
                    }
                }
```

**Step 4: Run tests**

Run: `cargo test test_integrate_rational_two_level -- --nocapture`

Expected: all pipeline tests pass, including the new elementary degree-2 test.

**Step 5: Run full suite + clippy**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Fix two-level residual computation: per-root log arguments instead of assuming v=h_den"
```

---

### Task 4: End-to-end tests for degree-2 denominators

**Files:**
- Modify: `tests/integration.rs` — add e2e tests
- Modify: `src/risch.rs` — add more unit tests

**Step 1: Add end-to-end non-elementary test**

Add to `tests/integration.rs` before the closing `}`:

```rust
#[test]
fn test_integrate_ln_x_over_1_plus_exp_2x_non_elementary() {
    // ∫ln(x)/(1+exp(2x)) dx → non-elementary (degree-2 denominator)
    let result = integrate_latex("\\frac{\\ln(x)}{1 + \\exp(2x)}", "x");
    assert!(
        result.is_err(),
        "∫ln(x)/(1+exp(2x))dx should be non-elementary: {:?}",
        result,
    );
    assert!(
        result.unwrap_err().starts_with("NON_ELEMENTARY:"),
        "Should be NON_ELEMENTARY"
    );
}
```

**Step 2: Add unit test for degree-2 with split roots**

Add to the risch.rs test module:

```rust
#[test]
fn test_gcd_two_level_with_x_coefficients() {
    // d = θ₂² - x²  (using RationalFunction coefficients)
    // g_c = [x, 0, -x] = x(1 - θ₂²) = -x(θ₂² - x²)... no wait.
    // d = θ₂² - x², g_c = x - x·θ₂² = x(1-θ₂²). These share no factor
    // since d = θ₂² - x² = (θ₂-x)(θ₂+x) and g_c = -x(θ₂²-1) = -x(θ₂-1)(θ₂+1).
    // gcd = 1 (coprime).
    let x_rf = RationalFunction::from_poly(poly(&[0, 1], "x"));
    let neg_x_sq = -&(&x_rf * &x_rf);
    let d = ExtPoly::from_coeffs(vec![neg_x_sq, rf_const(0), rf_const(1)], "x");
    let g_c = vec![
        ExtPoly::from_rf(x_rf.clone()),
        ExtPoly::zero("x"),
        ExtPoly::from_rf(-&x_rf),
    ];
    let v = gcd_extpoly_with_two_level(&d, &g_c, "x");
    assert!(
        v.is_constant(),
        "GCD should be 1, got degree {:?}",
        v.degree()
    );
}
```

**Step 3: Run all tests**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 4: Commit**

```bash
git add src/risch.rs tests/integration.rs
git commit -m "Add end-to-end tests for two-level rational with degree-2 denominators"
```

---

### Task 5: Update documentation

**Files:**
- Modify: `KNUTH-PLAN.md`
- Modify: `README.md` (if test count changed)

**Step 1: Update KNUTH-PLAN.md**

- Update test count
- In the "Current State" paragraph, change "degree-1 denominator GCD" references to "general denominator GCD via θ₁-component decomposition"
- Update Phase 9 Remaining items: remove "Higher-degree denominators in two-level rational"

**Step 2: Update README.md**

- Update test count

**Step 3: Commit**

```bash
git add KNUTH-PLAN.md README.md
git commit -m "doc updates"
```

---

## Non-goals for this session

1. **θ₁ in denominator** — requires mixed-coefficient Euclidean algorithm (separate backlog item)
2. **Log-on-top-of-exp tower ordering** — different tower construction (separate backlog item)
3. **Algebraic extensions** — Q(α) arithmetic (separate backlog item)

## Test matrix

| Test | Type | Denom deg | θ₁ | Expected |
|------|------|-----------|-----|----------|
| ln(x)/(1+exp(x)) | Unit + E2E | 1 | yes | Non-elementary (regression) |
| exp(x)·ln(x)/(1+exp(x)) | Unit + E2E | 1 | yes | Non-elementary (regression) |
| ln(x)/(1+exp(2x)) | Unit + E2E | 2 | yes | Non-elementary (new) |
| 1/(exp(2x)-1) via two-level | Unit | 2 | no | Elementary (new) |
| GCD full divisor | Unit | 2 | yes | v = d |
| GCD partial factor | Unit | 2 | yes | v = θ₂+1 |
| GCD coprime | Unit | 2 | yes | v = 1 |
| GCD no θ₁ | Unit | 2 | no | v = d |
| GCD with x coefficients | Unit | 2 | no | v = 1 |
