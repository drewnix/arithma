# θ₁-in-Denominator: Content Extraction for Two-Level Rational Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Handle integrands where the denominator contains θ₁=ln(x) terms in the exp-over-log two-level tower, enabling non-elementarity detection for integrals like ∫exp(x)/(ln(x)·(1+exp(x))) dx.

**Architecture:** Factor the θ₁-content from the two-level denominator (GCD of all θ₂-coefficients as ExtPolys), yielding a primitive denominator in Q(x)[θ₂]. Thread the content factor through the Rothstein-Trager resultant as a scaling on the z-coefficient entries. Non-elementarity detection works because roots of the scaled resultant are the same as the unscaled (content is nonzero since θ₁ is transcendental). For elementary results with zero residual, assemble the answer with content in the rational-part denominator.

**Tech Stack:** Rust, existing `ExtPoly`, `ExtPoly::gcd`, `hermite_reduce_two_level`, `rothstein_trager_two_level`, `integrate_rational_two_level`.

**Reference:** Bronstein, *Symbolic Integration I*, §5.2 (integration in transcendental extensions — coefficient field structure).

---

## Mathematical Foundation

### The problem

Tower: Q(x) ⊂ Q(x, θ₁=ln(x)) ⊂ Q(x, θ₁, θ₂=exp(g(x)))

Current `integrate_rational_two_level` requires: numerator ∈ Q(x)[θ₁][θ₂], denominator ∈ Q(x)[θ₂].

We want: denominator ∈ Q(x)[θ₁][θ₂] — i.e., θ₁ allowed in denominator.

### Content extraction

The denominator D = Σᵢ dᵢ(θ₁)·θ₂ⁱ where each dᵢ ∈ Q(x)[θ₁].

**θ₁-content:** cont(D) = gcd(d₀, d₁, ..., dₘ) ∈ Q(x)[θ₁], computed via iterative `ExtPoly::gcd`.

**Primitive part:** pp(D) = D/cont(D). Each coefficient dᵢ/cont(D) ∈ Q(x)[θ₁].

**Key constraint:** For this implementation, we require pp(D) ∈ Q(x)[θ₂] — i.e., after factoring out the content, the primitive part has only Q(x) coefficients (degree 0 in θ₁). This handles the separable case D = D₁(θ₁)·D₂(θ₂).

### Example: ∫1/(ln(x)·(1+exp(x))) dx

- D as two-level: [θ₁, θ₁] (= θ₁ + θ₁·θ₂)
- cont = gcd(θ₁, θ₁) = θ₁
- pp = [1, 1] = 1 + θ₂ ∈ Q(x)[θ₂] ✓
- Integrand becomes: (1/θ₁) / (1+θ₂)

### Scaled Rothstein-Trager

For the squarefree part ∫(h_num/content)/h_den:

R(z) = res(h_den, h_num/content − z·D(h_den))

Scale by content (nonzero in C = Q(x)(θ₁)):

R_scaled(z) = res(h_den, h_num − z·content·D(h_den)) = content^m · R(z)

Roots are identical. In the Sylvester matrix, the z-coefficient at θ₂-degree j becomes:
- Old: ExtPoly::from_rf(-dd.coeff(j)) — a Q(x) scalar
- New: -content.scalar_mul(&dd.coeff(j)) — a general ExtPoly

### Non-elementarity criterion

If R_scaled(z) has no constant roots (verified by specializing x → x₀, then checking as ExtPoly identity), the integral is non-elementary. Since content^m ≠ 0, the scaled and unscaled resultants have the same root set.

### Elementary case (when roots exist)

GCD computation: gcd(h_den, h_num − c·content·D(h_den)) — uses existing `gcd_extpoly_with_two_level` since g_c is Vec<ExtPoly> with the content-scaled dd subtracted.

Log terms: c·ln(v) — correct as-is (content is a unit in the coefficient field, doesn't affect GCD in θ₂).

Residual: if zero after log-derivative subtraction → result is g_num/(content·g_den) + Σ cᵢ·ln(vᵢ). If non-zero → return None (requires C-valued polynomial integrator, future work).

---

## Tasks

### Task 1: Add `compute_theta1_content` function and tests

**Files:**
- Modify: `src/risch.rs` — add function after `gcd_extpoly_with_two_level` (~line 2317)

**Step 1: Write failing tests**

Add to the risch test module:

```rust
#[test]
fn test_theta1_content_uniform() {
    // [θ₁, θ₁] → content = θ₁
    let tl = vec![ExtPoly::theta("x"), ExtPoly::theta("x")];
    let content = compute_theta1_content(&tl, "x");
    assert_eq!(content.make_monic(), ExtPoly::theta("x").make_monic());
}

#[test]
fn test_theta1_content_no_common_factor() {
    // [1, θ₁] → content = 1 (coprime)
    let tl = vec![ExtPoly::one("x"), ExtPoly::theta("x")];
    let content = compute_theta1_content(&tl, "x");
    assert!(content.is_constant());
}

#[test]
fn test_theta1_content_power() {
    // [θ₁², θ₁²] → content = θ₁²
    let theta1_sq = {
        let t = ExtPoly::theta("x");
        &t * &t
    };
    let tl = vec![theta1_sq.clone(), theta1_sq.clone()];
    let content = compute_theta1_content(&tl, "x");
    assert_eq!(content.degree(), Some(2));
}

#[test]
fn test_theta1_content_with_zero() {
    // [0, θ₁] → content = θ₁ (skip zeros)
    let tl = vec![ExtPoly::zero("x"), ExtPoly::theta("x")];
    let content = compute_theta1_content(&tl, "x");
    assert_eq!(content.make_monic(), ExtPoly::theta("x").make_monic());
}

#[test]
fn test_theta1_content_all_constant() {
    // [rf(1), rf(2)] → content = 1 (Q(x) only, no θ₁)
    let tl = vec![ExtPoly::from_rf(rf_const(1)), ExtPoly::from_rf(rf_const(2))];
    let content = compute_theta1_content(&tl, "x");
    assert!(content.is_constant());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_theta1_content -- --nocapture 2>&1 | head -10`

Expected: compilation error — `compute_theta1_content` not defined.

**Step 3: Implement `compute_theta1_content`**

```rust
/// Compute the θ₁-content of a two-level polynomial: gcd of all θ₂-coefficients
/// as ExtPolys in θ₁.
///
/// Returns the content (an ExtPoly in θ₁). Returns a constant (degree 0) ExtPoly
/// if the coefficients have no common θ₁ factor.
fn compute_theta1_content(tl: &[ExtPoly], var: &str) -> ExtPoly {
    let mut result: Option<ExtPoly> = None;
    for ep in tl {
        if ep.is_zero() {
            continue;
        }
        result = Some(match result {
            None => ep.clone(),
            Some(acc) => acc.gcd(ep),
        });
    }
    result.unwrap_or_else(|| ExtPoly::one(var))
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_theta1_content -- --nocapture`

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add compute_theta1_content: GCD of θ₂-coefficients for two-level denominator factoring"
```

---

### Task 2: Modify `rothstein_trager_two_level` to accept content parameter

**Files:**
- Modify: `src/risch.rs` — change signature and logic of `rothstein_trager_two_level` (~line 2447), update all callers

**Step 1: Write failing test for content-scaled RT**

```rust
#[test]
fn test_rt_two_level_with_content_no_roots() {
    // ∫1/(θ₁·(1+θ₂)) where θ₁=ln(x), θ₂=exp(x)
    // After content extraction: content=θ₁, d=1+θ₂, a=[1], D(d)=θ₂
    // R_scaled(z) = res(1+θ₂, 1 − z·θ₁·θ₂)
    // Sylvester: [[1,1],[−z·θ₁, 1]] → det = 1 + z·θ₁ → no constant roots
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let a = vec![ExtPoly::from_rf(rf_const(1))]; // [1]
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let dd = ext.differentiate(&d); // D(1+θ₂) = θ₂
    let content = ExtPoly::theta("x"); // θ₁ = ln(x)
    let rz = rothstein_trager_two_level(&d, &a, &dd, Some(&content), "x");
    let roots = find_constant_roots_two_level(&rz, "x");
    assert!(roots.is_empty(), "Should have no constant roots, got {:?}", roots);
}

#[test]
fn test_rt_two_level_with_content_none_same_as_before() {
    // Regression: content=None gives same result as before
    // ∫1/(1+exp(x)): d=1+θ₂, a=[1], D(d)=θ₂. Root at z=-1.
    let d = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let a = vec![ExtPoly::from_rf(rf_const(1))];
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let dd = ext.differentiate(&d);
    let rz = rothstein_trager_two_level(&d, &a, &dd, None, "x");
    let roots = find_constant_roots_two_level(&rz, "x");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0], BigRational::from_integer(BigInt::from(-1)));
}
```

**Step 2: Modify `rothstein_trager_two_level` signature and logic**

Change signature from:
```rust
fn rothstein_trager_two_level(d: &ExtPoly, a: &[ExtPoly], dd: &ExtPoly, var: &str) -> Vec<ExtPoly>
```
to:
```rust
fn rothstein_trager_two_level(
    d: &ExtPoly,
    a: &[ExtPoly],
    dd: &ExtPoly,
    content: Option<&ExtPoly>,
    var: &str,
) -> Vec<ExtPoly>
```

In the m==0 && n==0 special case, change:
```rust
let c1 = ExtPoly::from_rf(-&dd.coeff(0));
```
to:
```rust
let dd_c0 = dd.coeff(0);
let c1 = match content {
    Some(c) => {
        let scaled = c.scalar_mul(&dd_c0);
        -&scaled
    }
    None => ExtPoly::from_rf(-&dd_c0),
};
```

In the matrix construction (last m rows), change:
```rust
let dd_coeff = dd.coeff(n - k);
if dd_coeff.is_zero() {
    row[col] = vec![a_coeff];
} else {
    row[col] = vec![a_coeff, ExtPoly::from_rf(-&dd_coeff)];
}
```
to:
```rust
let dd_coeff = dd.coeff(n - k);
if dd_coeff.is_zero() {
    row[col] = vec![a_coeff];
} else {
    let z_coeff = match content {
        Some(c) => {
            let scaled = c.scalar_mul(&dd_coeff);
            -&scaled
        }
        None => ExtPoly::from_rf(-&dd_coeff),
    };
    row[col] = vec![a_coeff, z_coeff];
}
```

**Step 3: Update all callers to pass `None`**

Find callers of `rothstein_trager_two_level` and add `None` as the content argument. There should be one caller in `integrate_rational_two_level` (~line 2798).

**Step 4: Run tests**

Run: `cargo test test_rt_two_level -- --nocapture`

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Extend rothstein_trager_two_level with content parameter for θ₁-scaled z-coefficients"
```

---

### Task 3: Modify `integrate_rational_two_level` to accept and propagate content

**Files:**
- Modify: `src/risch.rs` — add `content` parameter to `integrate_rational_two_level`, thread through RT and GCD

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_rational_two_level_with_content_non_elementary() {
    // ∫1/(ln(x)·(1+exp(x))) dx → non-elementary
    // num=[1], den=1+θ₂, content=θ₁
    let num = vec![ExtPoly::from_rf(rf_const(1))];
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let content = ExtPoly::theta("x");
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_rational_two_level(&num, &den, Some(&content), &inner_ext, &outer_ext, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected non-elementary, got {:?}", other),
    }
}

#[test]
fn test_integrate_rational_two_level_content_none_regression() {
    // Regression: content=None behaves as before
    // ∫ln(x)/(1+exp(x)) dx → non-elementary
    let num = vec![ExtPoly::theta("x")];
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let inner_ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    let outer_ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_rational_two_level(&num, &den, None, &inner_ext, &outer_ext, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected non-elementary, got {:?}", other),
    }
}
```

**Step 2: Modify `integrate_rational_two_level` signature**

From:
```rust
fn integrate_rational_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult>
```
To:
```rust
fn integrate_rational_two_level(
    num: &[ExtPoly],
    den: &ExtPoly,
    content: Option<&ExtPoly>,
    inner_ext: &DifferentialExtension,
    outer_ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult>
```

**Step 3: Thread content through the function**

Changes needed inside `integrate_rational_two_level`:

a) **Polynomial quotient integration** (~line 2764): If content is Some and quotient is non-zero, return None (C-valued polynomial integrator needed):
```rust
if !quotient.iter().all(|ep| ep.is_zero()) {
    if content.is_some() {
        // Quotient/content requires C-valued polynomial integrator — not yet supported.
        // But if RT later proves non-elementary, we'll return that instead.
        // For now, attempt RT first and only return None if RT doesn't resolve it.
    } else {
        match integrate_two_level_exp_log(&quotient, inner_ext, outer_ext, var) {
            Some(RischResult::Elementary(n)) => result_terms.push(n),
            Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
            None => return None,
        }
    }
}
```

Actually, a cleaner approach: always attempt RT first. If non-elementary, return that. If elementary but quotient needs content-integration, return None.

Let me restructure: process the proper fraction first, then the quotient.

b) **RT call** (~line 2798): Pass content:
```rust
let rz = rothstein_trager_two_level(&hr.h_den, &hr.h_num, &dd, content, var);
```

c) **GCD computation** (~line 2823): Scale dd subtraction by content:
```rust
for (i, g_c_i) in g_c.iter_mut().enumerate() {
    let dd_coeff = dd.coeff(i);
    if !dd_coeff.is_zero() {
        let sub = match content {
            Some(c) => c.scalar_mul(&(&dd_coeff * &c_rf)),
            None => ExtPoly::from_rf(&dd_coeff * &c_rf),
        };
        *g_c_i = &*g_c_i - &sub;
    }
}
```

d) **Rational part output** (~line 2778): Include content in denominator:
```rust
if !hr.g_num.iter().all(|ep| ep.is_zero()) {
    let g_num_node = two_level_to_node(&hr.g_num, &ln_x, &exp_g, var);
    let g_den_base = extpoly_to_node(&hr.g_den, &exp_g, var);
    let g_den_node = match content {
        Some(c) => {
            let c_node = extpoly_to_node(c, &ln_x, var);
            Node::Multiply(Box::new(c_node), Box::new(g_den_base))
        }
        None => g_den_base,
    };
    result_terms.push(Node::Divide(Box::new(g_num_node), Box::new(g_den_node)));
}
```

e) **Residual integration** (~line 2885): If content is Some and residual is non-zero, return None:
```rust
if !residual.iter().all(|ep| ep.is_zero()) {
    if content.is_some() {
        return None; // Residual/content requires C-valued polynomial integrator
    }
    // ... existing residual integration code ...
}
```

f) **Quotient integration** (moved after RT): If content is Some and we reach here (RT didn't prove non-elementary), check if quotient needs integration:
```rust
if content.is_some() && !quotient.iter().all(|ep| ep.is_zero()) {
    return None; // Quotient/content requires C-valued polynomial integrator
}
```

**Step 4: Update callers of `integrate_rational_two_level`**

All existing callers (~lines 2204, 2223) pass `None`:
```rust
return integrate_rational_two_level(&num_tl, &den_ep, None, &inner_ext, &outer_ext, var);
```

**Step 5: Run tests**

Run: `cargo test test_integrate_rational_two_level -- --nocapture`

**Step 6: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 7: Commit**

```bash
git add src/risch.rs
git commit -m "Thread content parameter through integrate_rational_two_level for θ₁-in-denominator"
```

---

### Task 4: Modify `try_risch_two_level` dispatch for θ₁-in-denominator

**Files:**
- Modify: `src/risch.rs` — extend the dispatch logic in `try_risch_two_level` (~line 2194)

**Step 1: Write failing end-to-end test**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_integrate_1_over_ln_x_times_1_plus_exp_x_non_elementary() {
    // ∫1/(ln(x)·(1+exp(x))) dx → non-elementary
    let result = integrate_latex("\\frac{1}{\\ln(x) \\cdot (1 + \\exp(x))}", "x");
    assert!(
        result.is_err(),
        "∫1/(ln(x)·(1+exp(x)))dx should be non-elementary: {:?}",
        result,
    );
    assert!(
        result.unwrap_err().starts_with("NON_ELEMENTARY:"),
        "Should be NON_ELEMENTARY"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_integrate_1_over_ln_x_times -- --nocapture 2>&1 | head -10`

Expected: returns Ok (parsed but not integrated — currently falls through to other methods or returns a wrong result).

**Step 3: Modify `try_risch_two_level` dispatch**

After the existing rational dispatch (line 2194-2225), when `two_level_to_extpoly` fails, add content-extraction fallback:

```rust
// Try rational case first: expression has Divide with θ₂ in denominator
if let Some((num_tl, den_tl)) = extract_two_level_rational(expr, var, &exp_poly) {
    if let Some(den_ep) = two_level_to_extpoly(&den_tl, var) {
        // Existing path: no θ₁ in denominator
        let inner_ext = DifferentialExtension::logarithmic(
            RationalFunction::from_poly(Polynomial::x(var)),
            var,
        );
        let outer_ext = DifferentialExtension::exponential(
            RationalFunction::from_poly(exp_poly.clone()),
            var,
        );
        return integrate_rational_two_level(&num_tl, &den_ep, None, &inner_ext, &outer_ext, var);
    }
    // θ₁ in denominator: try content extraction
    let content = compute_theta1_content(&den_tl, var);
    if !content.is_constant() {
        // Factor out content and check if primitive part is in Q(x)[θ₂]
        let prim_tl: Vec<ExtPoly> = den_tl
            .iter()
            .map(|ep| {
                if ep.is_zero() {
                    ExtPoly::zero(var)
                } else {
                    let (q, _) = ep.div_rem(&content).unwrap();
                    q
                }
            })
            .collect();
        if let Some(prim_den) = two_level_to_extpoly(&prim_tl, var) {
            let inner_ext = DifferentialExtension::logarithmic(
                RationalFunction::from_poly(Polynomial::x(var)),
                var,
            );
            let outer_ext = DifferentialExtension::exponential(
                RationalFunction::from_poly(exp_poly.clone()),
                var,
            );
            return integrate_rational_two_level(
                &num_tl, &prim_den, Some(&content), &inner_ext, &outer_ext, var,
            );
        }
    }
}
```

And the same for the simplified-expression fallback (lines 2208-2225):

```rust
// Try after simplification for rational case
{
    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
    if let Some((num_tl, den_tl)) = extract_two_level_rational(&simplified, var, &exp_poly) {
        if let Some(den_ep) = two_level_to_extpoly(&den_tl, var) {
            let inner_ext = DifferentialExtension::logarithmic(
                RationalFunction::from_poly(Polynomial::x(var)),
                var,
            );
            let outer_ext = DifferentialExtension::exponential(
                RationalFunction::from_poly(exp_poly.clone()),
                var,
            );
            return integrate_rational_two_level(&num_tl, &den_ep, None, &inner_ext, &outer_ext, var);
        }
        // θ₁ in denominator: try content extraction
        let content = compute_theta1_content(&den_tl, var);
        if !content.is_constant() {
            let prim_tl: Vec<ExtPoly> = den_tl
                .iter()
                .map(|ep| {
                    if ep.is_zero() {
                        ExtPoly::zero(var)
                    } else {
                        let (q, _) = ep.div_rem(&content).unwrap();
                        q
                    }
                })
                .collect();
            if let Some(prim_den) = two_level_to_extpoly(&prim_tl, var) {
                let inner_ext = DifferentialExtension::logarithmic(
                    RationalFunction::from_poly(Polynomial::x(var)),
                    var,
                );
                let outer_ext = DifferentialExtension::exponential(
                    RationalFunction::from_poly(exp_poly.clone()),
                    var,
                );
                return integrate_rational_two_level(
                    &num_tl, &prim_den, Some(&content), &inner_ext, &outer_ext, var,
                );
            }
        }
    }
}
```

**Step 4: Run test**

Run: `cargo test test_integrate_1_over_ln_x_times -- --nocapture`

Expected: non-elementary detected.

**Step 5: Run clippy + full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 6: Commit**

```bash
git add src/risch.rs tests/integration.rs
git commit -m "Dispatch θ₁-in-denominator via content extraction in try_risch_two_level"
```

---

### Task 5: Additional end-to-end tests and documentation

**Files:**
- Modify: `tests/integration.rs` — add more e2e tests
- Modify: `src/risch.rs` — add unit tests
- Modify: `KNUTH-PLAN.md`, `README.md`

**Step 1: Add end-to-end tests**

```rust
#[test]
fn test_integrate_exp_over_ln_x_times_1_plus_exp_x_non_elementary() {
    // ∫exp(x)/(ln(x)·(1+exp(x))) dx → non-elementary
    let result = integrate_latex("\\frac{\\exp(x)}{\\ln(x) \\cdot (1 + \\exp(x))}", "x");
    assert!(
        result.is_err(),
        "∫exp(x)/(ln(x)·(1+exp(x)))dx should be non-elementary: {:?}",
        result,
    );
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}

#[test]
fn test_integrate_1_over_ln_x_sq_times_1_plus_exp_x_non_elementary() {
    // ∫1/(ln(x)²·(1+exp(x))) dx → non-elementary (content = θ₁²)
    let result = integrate_latex("\\frac{1}{\\ln(x)^2 \\cdot (1 + \\exp(x))}", "x");
    assert!(
        result.is_err(),
        "∫1/(ln²(x)·(1+exp(x)))dx should be non-elementary: {:?}",
        result,
    );
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Step 2: Add regression tests to ensure existing behavior unchanged**

```rust
#[test]
fn test_integrate_ln_x_over_1_plus_exp_x_still_works() {
    // Regression: ∫ln(x)/(1+exp(x)) dx → non-elementary (no content needed)
    let result = integrate_latex("\\frac{\\ln(x)}{1 + \\exp(x)}", "x");
    assert!(result.is_err());
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Step 3: Run all tests**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 4: Update KNUTH-PLAN.md**

- Update test count
- Add to Current State: "θ₁-in-denominator via content extraction (separable case)"
- Update Phase 9 Remaining items: mark θ₁-in-denominator as done
- Add key results
- Add to Completed Work section

**Step 5: Update README.md**

- Update test count

**Step 6: Commit**

```bash
git add src/risch.rs tests/integration.rs KNUTH-PLAN.md README.md
git commit -m "Add e2e tests for θ₁-in-denominator, doc updates"
```

---

## Non-goals for this session

1. **Non-separable denominators** — e.g., θ₁² + θ₂ where content=1 but primitive part still has θ₁. Requires full Q(x)(θ₁)[θ₂] arithmetic.
2. **C-valued polynomial integration** — integrating polynomials in θ₂ with Q(x)(θ₁) coefficients (needed for quotient/content when quotient ≠ 0). Requires solving Risch DE over C = Q(x)(θ₁).
3. **Pure θ₁ denominators without θ₂** — e.g., ∫exp(x)/ln(x) dx. The denominator has no θ₂ terms, so `extract_two_level_rational` returns None. Requires a different dispatch path (polynomial-in-θ₂ with C-valued coefficients).
4. **Elementary results with non-zero residual** — when RT finds roots but the residual after log-derivative subtraction is non-zero AND content ≠ 1. Returns None.

## Test matrix

| Test | Content | Denom | Expected |
|------|---------|-------|----------|
| 1/(ln(x)·(1+exp(x))) | θ₁ | 1+θ₂ | Non-elementary (RT: 1+z·θ₁) |
| exp(x)/(ln(x)·(1+exp(x))) | θ₁ | 1+θ₂ | Non-elementary |
| 1/(ln(x)²·(1+exp(x))) | θ₁² | 1+θ₂ | Non-elementary (RT: 1+z·θ₁²) |
| ln(x)/(1+exp(x)) | 1 (none) | 1+θ₂ | Non-elementary (regression) |
| exp(x)·ln(x)/(1+exp(x)) | 1 (none) | 1+θ₂ | Non-elementary (regression) |
| Content uniform [θ₁,θ₁] | — | — | content = θ₁ |
| Content coprime [1,θ₁] | — | — | content = 1 |
| Content power [θ₁²,θ₁²] | — | — | content = θ₁² |
| RT with content, no roots | θ₁ | — | roots = [] |
| RT without content, has root | None | — | roots = [-1] (regression) |
