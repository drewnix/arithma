# Rothstein-Trager Resultant Method — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate rational functions of ln(x) via Rothstein-Trager, or prove they have no elementary antiderivative.

**Architecture:** After Hermite reduction produces ∫a/d with d squarefree (both ExtPolys in θ = ln(x)), Rothstein-Trager computes R(z) = res_θ(d, a − z·D(d)) via the Sylvester matrix determinant, finds rational roots of R(z), and builds Σ cᵢ·ln(vᵢ) from GCDs. The resultant R(z) ∈ Q(x)[z] — roots in Q are found by specializing x to a constant, using existing rational_roots(), then verifying.

**Tech Stack:** Rust, existing ExtPoly/RationalFunction/Polynomial types, cofactor expansion for determinant.

**Reference:** Bronstein, *Symbolic Integration I: Transcendental Functions*, Chapters 5 and 12.

---

### Task 1: Determinant of ExtPoly matrix (cofactor expansion)

**Files:**
- Modify: `src/risch.rs` (append new function)

**Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block at the bottom of `src/risch.rs`:

```rust
#[test]
fn test_extpoly_det_1x1() {
    // det([[3]]) = 3
    let m = vec![vec![ExtPoly::from_rf(rf_const(3))]];
    let result = extpoly_matrix_det(&m, "x");
    assert_eq!(result, ExtPoly::from_rf(rf_const(3)));
}

#[test]
fn test_extpoly_det_2x2() {
    // det([[1, 2], [3, 4]]) = 1*4 - 2*3 = -2
    let m = vec![
        vec![ExtPoly::from_rf(rf_const(1)), ExtPoly::from_rf(rf_const(2))],
        vec![ExtPoly::from_rf(rf_const(3)), ExtPoly::from_rf(rf_const(4))],
    ];
    let result = extpoly_matrix_det(&m, "x");
    assert_eq!(result, ExtPoly::from_rf(rf_const(-2)));
}

#[test]
fn test_extpoly_det_2x2_with_theta() {
    // det([[θ, 1], [1, θ]]) = θ² - 1
    let theta = ExtPoly::theta("x");
    let one = ExtPoly::from_rf(rf_const(1));
    let m = vec![
        vec![theta.clone(), one.clone()],
        vec![one.clone(), theta.clone()],
    ];
    let result = extpoly_matrix_det(&m, "x");
    // θ² - 1: coeffs [-1, 0, 1]
    let expected = ExtPoly::from_coeffs(vec![rf_const(-1), rf_const(0), rf_const(1)], "x");
    assert_eq!(result, expected);
}

#[test]
fn test_extpoly_det_3x3() {
    // det([[1,0,0],[0,1,0],[0,0,1]]) = 1
    let one = ExtPoly::from_rf(rf_const(1));
    let zero = ExtPoly::zero("x");
    let m = vec![
        vec![one.clone(), zero.clone(), zero.clone()],
        vec![zero.clone(), one.clone(), zero.clone()],
        vec![zero.clone(), zero.clone(), one.clone()],
    ];
    assert_eq!(extpoly_matrix_det(&m, "x"), one);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_extpoly_det -- --nocapture 2>&1 | head -20`
Expected: compilation error — `extpoly_matrix_det` not defined.

**Step 3: Write the implementation**

Add before the `#[cfg(test)]` block in `src/risch.rs`:

```rust
/// Determinant of a square matrix of ExtPolys via cofactor expansion.
/// For small matrices (size ≤ 5, covering Risch degrees in practice).
fn extpoly_matrix_det(m: &[Vec<ExtPoly>], var: &str) -> ExtPoly {
    let n = m.len();
    if n == 0 {
        return ExtPoly::one(var);
    }
    if n == 1 {
        return m[0][0].clone();
    }
    if n == 2 {
        return &(&m[0][0] * &m[1][1]) - &(&m[0][1] * &m[1][0]);
    }
    let mut result = ExtPoly::zero(var);
    for j in 0..n {
        if m[0][j].is_zero() {
            continue;
        }
        let minor: Vec<Vec<ExtPoly>> = (1..n)
            .map(|row| {
                (0..n)
                    .filter(|&col| col != j)
                    .map(|col| m[row][col].clone())
                    .collect()
            })
            .collect();
        let cofactor = extpoly_matrix_det(&minor, var);
        let term = &m[0][j] * &cofactor;
        if j % 2 == 0 {
            result = &result + &term;
        } else {
            result = &result - &term;
        }
    }
    result
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_extpoly_det -- --nocapture`
Expected: all 4 tests PASS.

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add ExtPoly matrix determinant via cofactor expansion"
```

---

### Task 2: Sylvester matrix construction and resultant R(z)

**Files:**
- Modify: `src/risch.rs` (append new function)

The Sylvester matrix for res_θ(f, g) where f has degree m, g has degree n is (m+n)×(m+n). But here g = a − z·D(d), so its coefficients are linear in z. We represent z-dependence using ExtPoly with variable "z" — each matrix entry is an ExtPoly in z with RF(x) coefficients.

**Step 1: Write the failing test**

```rust
#[test]
fn test_resultant_z_simple() {
    // res_θ(θ, a₀ - z·b₀) = a₀ - z·b₀ (degree 1, degree 0 → just the constant)
    // For ∫1/(x·ln(x))dx: d=θ, a=1/x, D(d)=1/x
    // R(z) = 1/x - z/x = (1-z)/x
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let d = ExtPoly::theta("x"); // θ
    let a = ExtPoly::from_rf(one_over_x.clone()); // 1/x
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let dd = ext.differentiate(&d); // D(θ) = 1/x
    let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
    // R(z) should be (1-z)/x as an ExtPoly in z
    // coeff of z^0: 1/x, coeff of z^1: -1/x
    assert_eq!(rz.degree(), Some(1));
    let neg_one_over_x = -&one_over_x;
    assert_eq!(rz.coeff(0), one_over_x);
    assert_eq!(rz.coeff(1), neg_one_over_x);
}

#[test]
fn test_resultant_z_non_elementary() {
    // For ∫1/ln(x)dx: d=θ, a=1, D(d)=1/x
    // R(z) = 1 - z/x = (x-z)/x
    // coeff of z^0: 1, coeff of z^1: -1/x
    let d = ExtPoly::theta("x");
    let a = ExtPoly::from_rf(rf_const(1));
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let dd = ext.differentiate(&d);
    let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
    assert_eq!(rz.degree(), Some(1));
    assert_eq!(rz.coeff(0), rf_const(1));
    let neg_one_over_x = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
    assert_eq!(rz.coeff(1), neg_one_over_x);
}

#[test]
fn test_resultant_z_degree2() {
    // d = θ²+θ, a = (2θ+1)/x, D(d) = (2θ+1)/x
    // a - z·D(d) = (1-z)(2θ+1)/x
    // R(z) = -(1-z)²/x²
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let two_over_x = RationalFunction::new(poly(&[2], "x"), poly(&[0, 1], "x"));
    let d = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1), rf_const(1)], "x"); // θ²+θ
    let a = ExtPoly::from_coeffs(vec![one_over_x.clone(), two_over_x.clone()], "x"); // (2θ+1)/x
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let dd = ext.differentiate(&d);
    let rz = rothstein_trager_resultant(&d, &a, &dd, "x");
    // R(z) = -(1-z)²/x² = (-1 + 2z - z²)/x²
    assert_eq!(rz.degree(), Some(2));
    // Verify R(1) = 0: all coefficients should sum to zero RF
    let r1 = &(&rz.coeff(0) + &rz.coeff(1)) + &rz.coeff(2);
    assert!(r1.is_zero(), "R(1) should be 0, got {}", r1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_resultant_z -- --nocapture 2>&1 | head -10`
Expected: compilation error — `rothstein_trager_resultant` not defined.

**Step 3: Write the implementation**

```rust
/// Build the Sylvester matrix and compute R(z) = res_θ(d, a − z·D(d)).
///
/// d and a are ExtPolys in θ (tower variable) with RF(x) coefficients.
/// dd = D(d) is the full derivative of d.
/// Returns R(z) as an ExtPoly in z with RF(x) coefficients.
///
/// The Sylvester matrix entries from d are constant in z (degree 0 ExtPoly in z).
/// The entries from (a − z·dd) are linear in z (degree ≤ 1 ExtPoly in z).
fn rothstein_trager_resultant(
    d: &ExtPoly,
    a: &ExtPoly,
    dd: &ExtPoly,
    var: &str,
) -> ExtPoly {
    let m = d.degree().unwrap_or(0);
    let n = {
        // degree of g = a - z·dd in θ: max(deg(a), deg(dd))
        let da = a.degree().unwrap_or(0);
        let ddd = dd.degree().unwrap_or(0);
        da.max(ddd)
    };

    if m == 0 && n == 0 {
        // Both constant in θ: R(z) = a₀ - z·dd₀
        let c0 = a.coeff(0);
        let c1 = -&dd.coeff(0);
        return ExtPoly::from_coeffs(vec![c0, c1], var);
    }

    let size = m + n;
    if size == 0 {
        return ExtPoly::one(var);
    }

    // Build Sylvester matrix of size (m+n) × (m+n).
    // "z" is the variable name for the result polynomial.
    //
    // First n rows: coefficients of d (degree m), shifted.
    // Row i (0 ≤ i < n): column j gets d_{m-(j-i)} if 0 ≤ j-i ≤ m, else 0.
    //
    // Last m rows: coefficients of g = a - z·dd (degree n), shifted.
    // Row (n+i) (0 ≤ i < m): column j gets g_{n-(j-i)} if 0 ≤ j-i ≤ n, else 0.
    //
    // All entries are ExtPolys in z: d-entries are degree 0,
    // g-entries are a_k - z·dd_k (degree ≤ 1).

    let zero_z = ExtPoly::zero(var);

    let mut matrix: Vec<Vec<ExtPoly>> = Vec::with_capacity(size);

    // First n rows from d (constant in z)
    for i in 0..n {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=m {
            let col = i + k;
            if col < size {
                // d coefficient at degree (m - k), stored as constant-in-z ExtPoly
                let d_coeff = d.coeff(m - k);
                row[col] = ExtPoly::from_rf(d_coeff);
            }
        }
        matrix.push(row);
    }

    // Last m rows from g = a - z·dd (linear in z)
    for i in 0..m {
        let mut row = vec![zero_z.clone(); size];
        for k in 0..=n {
            let col = i + k;
            if col < size {
                // g coefficient at degree (n - k) in θ: a_{n-k} - z·dd_{n-k}
                let a_coeff = a.coeff(n - k);
                let dd_coeff = dd.coeff(n - k);
                // As ExtPoly in z: [a_coeff, -dd_coeff]
                if dd_coeff.is_zero() {
                    row[col] = ExtPoly::from_rf(a_coeff);
                } else {
                    row[col] = ExtPoly::from_coeffs(vec![a_coeff, -&dd_coeff], var);
                }
            }
        }
        matrix.push(row);
    }

    extpoly_matrix_det(&matrix, var)
}
```

**Step 4: Run tests**

Run: `cargo test test_resultant_z -- --nocapture`
Expected: all 3 tests PASS.

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add Rothstein-Trager resultant R(z) via Sylvester matrix"
```

---

### Task 3: Find constant roots of R(z) ∈ Q(x)[z]

**Files:**
- Modify: `src/rational_function.rs` (add `evaluate` method)
- Modify: `src/risch.rs` (add root-finding function)

**Step 1: Write the failing test for RF::evaluate**

Add to `src/rational_function.rs` tests:

```rust
#[test]
fn test_rf_evaluate() {
    // (x+1)/x at x=2 → 3/2
    let rf = RationalFunction::new(
        Polynomial::from_coeffs(vec![int(1), int(1)], "x"),
        Polynomial::from_coeffs(vec![int(0), int(1)], "x"),
    );
    assert_eq!(rf.evaluate(&int(2)), Some(rat(3, 2)));
}

#[test]
fn test_rf_evaluate_zero_denom() {
    // 1/x at x=0 → None
    let rf = RationalFunction::new(
        Polynomial::one("x"),
        Polynomial::from_coeffs(vec![int(0), int(1)], "x"),
    );
    assert_eq!(rf.evaluate(&int(0)), None);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_rf_evaluate -- --nocapture 2>&1 | head -10`
Expected: compilation error — `evaluate` not defined on RationalFunction.

**Step 3: Implement RF::evaluate**

Add to the `impl RationalFunction` block in `src/rational_function.rs`:

```rust
/// Evaluate the rational function at a specific value.
/// Returns None if the denominator is zero at that point.
pub fn evaluate(&self, x: &BigRational) -> Option<BigRational> {
    let den_val = self.den.evaluate(x);
    if den_val.is_zero() {
        return None;
    }
    Some(self.num.evaluate(x) / den_val)
}
```

**Step 4: Run RF evaluate tests**

Run: `cargo test test_rf_evaluate -- --nocapture`
Expected: PASS.

**Step 5: Write the failing test for root finding**

Add to `src/risch.rs` tests:

```rust
#[test]
fn test_find_constant_roots_simple() {
    // R(z) = (1-z)/x → root at z=1
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let neg_one_over_x = -&one_over_x;
    let rz = ExtPoly::from_coeffs(vec![one_over_x, neg_one_over_x], "x");
    let roots = find_constant_roots(&rz, "x");
    assert_eq!(roots, vec![int(1)]);
}

#[test]
fn test_find_constant_roots_none() {
    // R(z) = 1 - z/x = (x-z)/x → z=x is not constant, no roots
    let neg_one_over_x = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x"));
    let rz = ExtPoly::from_coeffs(vec![rf_const(1), neg_one_over_x], "x");
    let roots = find_constant_roots(&rz, "x");
    assert!(roots.is_empty());
}

#[test]
fn test_find_constant_roots_repeated() {
    // R(z) = -(1-z)²/x² = (-1 + 2z - z²)/x²
    let one_over_x2 = RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x"));
    let rz = ExtPoly::from_coeffs(
        vec![
            -&one_over_x2,                                             // -1/x²
            RationalFunction::new(poly(&[2], "x"), poly(&[0, 0, 1], "x")), // 2/x²
            -&one_over_x2,                                             // -1/x²
        ],
        "x",
    );
    let roots = find_constant_roots(&rz, "x");
    assert_eq!(roots, vec![int(1)]);
}
```

**Step 6: Run tests to verify they fail**

Run: `cargo test test_find_constant_roots -- --nocapture 2>&1 | head -10`
Expected: compilation error — `find_constant_roots` not defined.

**Step 7: Implement find_constant_roots**

```rust
/// Find all c ∈ Q such that R(c) = 0, where R(z) is a polynomial in z
/// with RationalFunction(x) coefficients.
///
/// Strategy: specialize x to a concrete value x₀, find rational roots of
/// the resulting Q[z] polynomial, then verify each candidate against the
/// full R(z).
fn find_constant_roots(rz: &ExtPoly, var: &str) -> Vec<BigRational> {
    let deg = match rz.degree() {
        Some(d) => d,
        None => return vec![], // R(z) = 0 — degenerate
    };

    if deg == 0 {
        // Constant polynomial: either always zero or never zero
        return vec![];
    }

    // Try specialization at x = 2, 3, 5, 7 (avoid small values that might
    // be roots of denominators).
    let candidates_x = [2i64, 3, 5, 7];
    let mut candidate_roots: Option<Vec<BigRational>> = None;

    for &x_val in &candidates_x {
        let x_br = BigRational::from_integer(BigInt::from(x_val));

        // Evaluate each RF coefficient at x = x_val
        let mut specialized_coeffs = Vec::with_capacity(deg + 1);
        let mut valid = true;
        for i in 0..=deg {
            match rz.coeff(i).evaluate(&x_br) {
                Some(val) => specialized_coeffs.push(val),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }

        let spec_poly = Polynomial::from_coeffs(specialized_coeffs, "z");
        if spec_poly.is_zero() {
            continue; // Degenerate specialization, try another
        }

        let roots = spec_poly.rational_roots();
        candidate_roots = Some(roots);
        break;
    }

    let candidates = match candidate_roots {
        Some(c) => c,
        None => return vec![],
    };

    // Verify each candidate: R(c) must be the zero RF
    let mut verified = Vec::new();
    for c in candidates {
        let mut sum = RationalFunction::zero(var);
        let mut c_power = BigRational::one();
        for i in 0..=deg {
            let term = &rz.coeff(i)
                * &RationalFunction::from_constant(c_power.clone(), var);
            sum = &sum + &term;
            c_power = &c_power * &c;
        }
        if sum.is_zero() {
            if !verified.contains(&c) {
                verified.push(c);
            }
        }
    }

    verified
}
```

**Step 8: Run tests**

Run: `cargo test test_find_constant_roots -- --nocapture`
Expected: all 3 tests PASS.

**Step 9: Commit**

```bash
git add src/rational_function.rs src/risch.rs
git commit -m "Add RF::evaluate and constant root finder for Rothstein-Trager"
```

---

### Task 4: ExtPoly-to-Node and RF-to-Node conversion

**Files:**
- Modify: `src/risch.rs` (add conversion functions)

These convert the integration result (ExtPolys in θ = ln(x)) back to Node expressions.

**Step 1: Write the failing test**

```rust
#[test]
fn test_extpoly_to_node_constant() {
    // ExtPoly [3] → Node::Num(3)
    let ep = ExtPoly::from_rf(rf_const(3));
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let result = extpoly_to_node(&ep, &ln_x, "x");
    assert_eq!(format!("{}", result), "3");
}

#[test]
fn test_extpoly_to_node_theta() {
    // ExtPoly [0, 1] → ln(x)
    let ep = ExtPoly::theta("x");
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let result = extpoly_to_node(&ep, &ln_x, "x");
    assert_eq!(format!("{}", result), "\\ln(x)");
}

#[test]
fn test_extpoly_to_node_theta_plus_one() {
    // ExtPoly [1, 1] → 1 + ln(x)
    let ep = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    let result = extpoly_to_node(&ep, &ln_x, "x");
    // Should contain both terms
    let s = format!("{}", result);
    assert!(s.contains("\\ln(x)"), "Expected ln(x) in {}", s);
}

#[test]
fn test_rf_to_node_constant() {
    let rf = rf_const(5);
    let result = rf_to_node(&rf, "x");
    assert_eq!(format!("{}", result), "5");
}

#[test]
fn test_rf_to_node_polynomial() {
    // x + 1
    let rf = rf_poly(&[1, 1]);
    let result = rf_to_node(&rf, "x");
    let s = format!("{}", result);
    assert!(s.contains("x"), "Expected x in {}", s);
}

#[test]
fn test_rf_to_node_fraction() {
    // 1/x
    let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let result = rf_to_node(&rf, "x");
    let s = format!("{}", result);
    assert!(s.contains("x"), "Expected x in {}", s);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_extpoly_to_node -- --nocapture 2>&1 | head -10`
Expected: compilation error.

**Step 3: Implement conversion functions**

```rust
/// Convert a RationalFunction p(x)/q(x) to a Node expression.
fn rf_to_node(rf: &RationalFunction, var: &str) -> Node {
    let num_node = rf.numerator().to_node();
    if *rf.denominator() == Polynomial::one(var) {
        num_node
    } else {
        Node::Divide(
            Box::new(num_node),
            Box::new(rf.denominator().to_node()),
        )
    }
}

/// Convert an ExtPoly Σ aᵢ(x)·θⁱ to a Node expression,
/// where θ is replaced by `theta_node` (e.g., ln(x)).
fn extpoly_to_node(ep: &ExtPoly, theta_node: &Node, var: &str) -> Node {
    let deg = match ep.degree() {
        Some(d) => d,
        None => return Node::Num(ExactNum::zero()),
    };

    let mut terms: Vec<Node> = Vec::new();
    for i in 0..=deg {
        let coeff = ep.coeff(i);
        if coeff.is_zero() {
            continue;
        }
        let coeff_node = rf_to_node(&coeff, var);
        let term = if i == 0 {
            coeff_node
        } else {
            let theta_power = if i == 1 {
                theta_node.clone()
            } else {
                Node::Power(
                    Box::new(theta_node.clone()),
                    Box::new(Node::Num(ExactNum::integer(i as i64))),
                )
            };
            if coeff == RationalFunction::one(var) {
                theta_power
            } else {
                Node::Multiply(Box::new(coeff_node), Box::new(theta_power))
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Node::Num(ExactNum::zero());
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }
    result
}
```

**Step 4: Run tests**

Run: `cargo test test_extpoly_to_node test_rf_to_node -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add ExtPoly-to-Node and RF-to-Node conversion for Risch results"
```

---

### Task 5: Pattern detector for rational-in-log expressions

**Files:**
- Modify: `src/risch.rs` (add pattern detection function)

Detects expressions that are rational functions of ln(x) and converts them to (numerator, denominator) ExtPoly pairs.

**Step 1: Write the failing test**

```rust
#[test]
fn test_extract_log_rational_inv_x_ln_x() {
    // 1/(x·ln(x)) → num = [1/x], den = [0, 1] (θ)
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        )),
    );
    let result = extract_log_rational_pattern(&expr, "x");
    assert!(result.is_some(), "Should detect 1/(x·ln(x))");
    let (num, den, _ext) = result.unwrap();
    // num should be constant 1/x
    assert!(num.is_constant());
    // den should be θ (degree 1)
    assert_eq!(den.degree(), Some(1));
}

#[test]
fn test_extract_log_rational_inv_ln_x() {
    // 1/ln(x) → num = [1], den = [0, 1] (θ)
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let result = extract_log_rational_pattern(&expr, "x");
    assert!(result.is_some(), "Should detect 1/ln(x)");
    let (num, den, _ext) = result.unwrap();
    assert_eq!(num, ExtPoly::from_rf(rf_const(1)));
    assert_eq!(den, ExtPoly::theta("x"));
}

#[test]
fn test_extract_log_rational_not_applicable() {
    // sin(x) → no log pattern
    let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
    assert!(extract_log_rational_pattern(&expr, "x").is_none());
}
```

**Step 2: Run tests to verify they fail**

Expected: compilation error.

**Step 3: Implement pattern detector**

```rust
/// Try to express a Node as a rational function of ln(x): A(x,θ)/D(x,θ)
/// where θ = ln(x), and A, D are polynomials in θ with Q(x) coefficients.
///
/// Returns (numerator, denominator, extension) or None.
pub fn extract_log_rational_pattern(
    expr: &Node,
    var: &str,
) -> Option<(ExtPoly, ExtPoly, DifferentialExtension)> {
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(Polynomial::x(var)),
        var,
    );

    // Try unsimplified first, then simplified
    if let Some((num, den)) = extract_log_rational_inner(expr, var) {
        return Some((num, den, ext));
    }
    let env = crate::environment::Environment::new();
    let simplified =
        crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
    let (num, den) = extract_log_rational_inner(&simplified, var)?;
    Some((num, den, ext))
}

/// Convert a Node sub-expression to an ExtPoly in θ = ln(x).
/// Returns None if the expression contains structures not representable
/// as a polynomial in θ with Q(x) coefficients.
fn node_to_extpoly(expr: &Node, var: &str) -> Option<ExtPoly> {
    match expr {
        Node::Num(n) => {
            let val = n.to_bigrat()?;
            Some(ExtPoly::from_rf(RationalFunction::from_constant(val, var)))
        }

        Node::Variable(v) if v == var => {
            Some(ExtPoly::from_rf(RationalFunction::from_poly(Polynomial::x(var))))
        }

        Node::Variable(_) => None, // different variable

        // ln(x) → θ
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            if let Node::Variable(v) = &args[0] {
                if v == var {
                    return Some(ExtPoly::theta(var));
                }
            }
            None
        }

        // ln(x)^n → θ^n
        Node::Power(base, exp) => {
            if let Node::Function(name, args) = base.as_ref() {
                if name == "ln" && args.len() == 1 {
                    if let Node::Variable(v) = &args[0] {
                        if v == var {
                            if let Node::Num(n) = exp.as_ref() {
                                let e = n.to_i64()?;
                                if e >= 1 {
                                    let mut result = ExtPoly::theta(var);
                                    for _ in 1..e {
                                        let theta = ExtPoly::theta(var);
                                        result = &result * &theta;
                                    }
                                    return Some(result);
                                }
                            }
                        }
                    }
                }
            }
            // x^n where n is a positive integer
            if let Node::Variable(v) = base.as_ref() {
                if v == var {
                    if let Node::Num(n) = exp.as_ref() {
                        if let Some(e) = n.to_i64() {
                            if e >= 1 {
                                let p = Polynomial::monomial(BigRational::one(), e as usize, var);
                                return Some(ExtPoly::from_rf(RationalFunction::from_poly(p)));
                            }
                        }
                    }
                }
            }
            None
        }

        Node::Add(left, right) => {
            let l = node_to_extpoly(left, var)?;
            let r = node_to_extpoly(right, var)?;
            Some(&l + &r)
        }

        Node::Subtract(left, right) => {
            let l = node_to_extpoly(left, var)?;
            let r = node_to_extpoly(right, var)?;
            Some(&l - &r)
        }

        Node::Negate(inner) => {
            let ep = node_to_extpoly(inner, var)?;
            Some(-&ep)
        }

        Node::Multiply(left, right) => {
            let l = node_to_extpoly(left, var)?;
            let r = node_to_extpoly(right, var)?;
            Some(&l * &r)
        }

        // Division by a polynomial in x only (no θ terms in denominator)
        Node::Divide(num, den) => {
            let n = node_to_extpoly(num, var)?;
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() {
                return None;
            }
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            Some(n.scalar_mul(&inv))
        }

        _ => None,
    }
}

/// Inner helper: extract (numerator_extpoly, denominator_extpoly) from a Node.
fn extract_log_rational_inner(expr: &Node, var: &str) -> Option<(ExtPoly, ExtPoly)> {
    match expr {
        // Top-level division: A / B where both are polynomial-in-θ
        Node::Divide(num, den) => {
            let num_ep = node_to_extpoly(num, var)?;
            let den_ep = node_to_extpoly(den, var)?;
            if den_ep.is_zero() {
                return None;
            }
            // Only use this path if the denominator actually involves θ
            if den_ep.is_constant() {
                return None; // Plain rational function in x, not our domain
            }
            Some((num_ep, den_ep))
        }

        // Multiply: one factor might be 1/(...with θ...)
        // e.g., (1/x) * (1/ln(x)) → but the parser rarely produces this
        // Handle: a * (b/c) → (a*b)/c and (a/b) * c → (a*c)/b
        Node::Multiply(left, right) => {
            if let Node::Divide(n, d) = right.as_ref() {
                let d_ep = node_to_extpoly(d, var)?;
                if !d_ep.is_constant() {
                    let n_ep = node_to_extpoly(n, var)?;
                    let l_ep = node_to_extpoly(left, var)?;
                    return Some((&l_ep * &n_ep, d_ep));
                }
            }
            if let Node::Divide(n, d) = left.as_ref() {
                let d_ep = node_to_extpoly(d, var)?;
                if !d_ep.is_constant() {
                    let n_ep = node_to_extpoly(n, var)?;
                    let r_ep = node_to_extpoly(right, var)?;
                    return Some((&r_ep * &n_ep, d_ep));
                }
            }
            None
        }

        _ => None,
    }
}
```

**Step 4: Run tests**

Run: `cargo test test_extract_log_rational -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add pattern detector for rational-in-log expressions"
```

---

### Task 6: Rothstein-Trager core — full pipeline

**Files:**
- Modify: `src/risch.rs` (add main integration function)

**Step 1: Write the failing tests**

```rust
#[test]
fn test_risch_log_rational_elementary() {
    // ∫1/(x·ln(x))dx = ln(ln(x))
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        )),
    );
    let result = try_risch_log_rational(&expr, "x");
    assert!(result.is_some(), "Should recognize log rational pattern");
    match result.unwrap() {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
        }
        RischResult::NonElementary(reason) => {
            panic!("Expected elementary, got non-elementary: {}", reason);
        }
    }
}

#[test]
fn test_risch_log_rational_non_elementary() {
    // ∫1/ln(x)dx — non-elementary (logarithmic integral)
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let result = try_risch_log_rational(&expr, "x");
    assert!(result.is_some(), "Should recognize 1/ln(x)");
    match result.unwrap() {
        RischResult::NonElementary(_) => {} // expected
        RischResult::Elementary(node) => {
            panic!("Expected non-elementary, got: {}", node);
        }
    }
}

#[test]
fn test_risch_log_rational_ln_x_minus_one() {
    // ∫1/(x·(ln(x)-1))dx = ln(ln(x)-1)
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Subtract(
                Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
                Box::new(Node::Num(ExactNum::one())),
            )),
        )),
    );
    let result = try_risch_log_rational(&expr, "x");
    assert!(result.is_some());
    match result.unwrap() {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
        }
        RischResult::NonElementary(reason) => {
            panic!("Expected elementary, got non-elementary: {}", reason);
        }
    }
}

#[test]
fn test_risch_log_rational_one_plus_ln_x() {
    // ∫1/(1+ln(x))dx — non-elementary (gives Ei)
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::one())),
        Box::new(Node::Add(
            Box::new(Node::Num(ExactNum::one())),
            Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        )),
    );
    let result = try_risch_log_rational(&expr, "x");
    assert!(result.is_some());
    match result.unwrap() {
        RischResult::NonElementary(_) => {} // expected
        RischResult::Elementary(node) => {
            panic!("Expected non-elementary, got: {}", node);
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Expected: compilation error — `try_risch_log_rational` not defined.

**Step 3: Implement the core pipeline**

```rust
/// Try to integrate a rational function of ln(x) using the Rothstein-Trager method.
///
/// Full pipeline:
/// 1. Extract (numerator, denominator, extension) from Node
/// 2. Hermite reduce to get rational part + squarefree remainder
/// 3. Apply Rothstein-Trager to the squarefree remainder
/// 4. Build result Node
pub fn try_risch_log_rational(expr: &Node, var: &str) -> Option<RischResult> {
    let (num, den, ext) = extract_log_rational_pattern(expr, var)?;

    // Hermite reduce: ∫num/den = g_num/g_den + ∫h_num/h_den
    let hr = hermite_reduce(&num, &den, var).ok()?;

    let theta_node = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);

    // Start building the result
    let mut result_terms: Vec<Node> = Vec::new();

    // Rational part from Hermite reduction
    let g_is_zero = hr.g_num.is_zero();
    if !g_is_zero {
        let g_node = Node::Divide(
            Box::new(extpoly_to_node(&hr.g_num, &theta_node, var)),
            Box::new(extpoly_to_node(&hr.g_den, &theta_node, var)),
        );
        result_terms.push(g_node);
    }

    // Remaining integral: ∫h_num/h_den with h_den squarefree
    if hr.h_num.is_zero() {
        // No remaining integral — all handled by Hermite reduction
    } else if hr.h_den.is_constant() {
        // Remaining is polynomial in θ — defer to existing polynomial-log method
        // (Hermite reduction reduced it to a polynomial integral)
        return None; // Let the existing try_risch_logarithmic handle it
    } else {
        // Rothstein-Trager on h_num/h_den
        let dd = ext.differentiate(&hr.h_den);
        let rz = rothstein_trager_resultant(&hr.h_den, &hr.h_num, &dd, var);

        let roots = find_constant_roots(&rz, var);

        if roots.is_empty() {
            // No constant roots → non-elementary
            let reason = format!(
                "No elementary antiderivative exists. \
                 The Rothstein-Trager resultant R(z) = res(d, a - z·D(d)) has no rational roots, \
                 so the integral cannot be expressed as a sum of logarithms."
            );
            return Some(RischResult::NonElementary(reason));
        }

        // Verify: sum of degrees of GCDs should equal deg(h_den)
        let h_den_deg = hr.h_den.degree().unwrap_or(0);
        let mut gcd_deg_sum = 0;
        let mut log_terms: Vec<(BigRational, ExtPoly)> = Vec::new();

        for c in &roots {
            // Compute a - c·D(d)
            let c_rf = RationalFunction::from_constant(c.clone(), var);
            let c_dd = dd.scalar_mul(&c_rf);
            let g_c = &hr.h_num - &c_dd;

            let v = hr.h_den.gcd(&g_c);
            let v_deg = v.degree().unwrap_or(0);
            gcd_deg_sum += v_deg;

            if v_deg > 0 {
                log_terms.push((c.clone(), v));
            }
        }

        if gcd_deg_sum != h_den_deg {
            // Degree mismatch — not all roots found or algebraic roots exist
            let reason = format!(
                "No elementary antiderivative exists. \
                 The Rothstein-Trager method found rational residues summing to degree {} \
                 but the denominator has degree {}; algebraic residues would be needed.",
                gcd_deg_sum, h_den_deg
            );
            return Some(RischResult::NonElementary(reason));
        }

        // Build Σ cᵢ · ln(vᵢ)
        for (c, v) in &log_terms {
            let v_node = extpoly_to_node(v, &theta_node, var);
            let ln_v = Node::Function("ln".to_string(), vec![v_node]);

            let term = if c == &BigRational::one() {
                ln_v
            } else {
                let c_node = Node::Num(ExactNum::from_bigrat(c.clone()));
                Node::Multiply(Box::new(c_node), Box::new(ln_v))
            };
            result_terms.push(term);
        }
    }

    if result_terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }

    let mut result = result_terms.remove(0);
    for term in result_terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    Some(RischResult::Elementary(result))
}
```

**Step 4: Run tests**

Run: `cargo test test_risch_log_rational -- --nocapture`
Expected: all 4 tests PASS.

**Step 5: Commit**

```bash
git add src/risch.rs
git commit -m "Add Rothstein-Trager logarithmic integration core"
```

---

### Task 7: Engine wiring and lib.rs export

**Files:**
- Modify: `src/integration.rs` (add to fallback chain)
- Modify: `src/lib.rs` (export new function)

**Step 1: Wire into try_risch_fallback**

In `src/integration.rs`, update the import and fallback function:

```rust
// Update import at top:
use crate::risch::{try_risch_exponential, try_risch_logarithmic, try_risch_log_rational, RischResult};

// Update try_risch_fallback:
fn try_risch_fallback(expr: &Node, var_name: &str) -> Option<Result<Node, String>> {
    if let Some(result) = try_risch_exponential(expr, var_name) {
        return Some(match result {
            RischResult::Elementary(node) => Ok(node),
            RischResult::NonElementary(reason) => Err(format!("NON_ELEMENTARY: {}", reason)),
        });
    }
    if let Some(result) = try_risch_logarithmic(expr, var_name) {
        return Some(match result {
            RischResult::Elementary(node) => Ok(node),
            RischResult::NonElementary(reason) => Err(format!("NON_ELEMENTARY: {}", reason)),
        });
    }
    if let Some(result) = try_risch_log_rational(expr, var_name) {
        return Some(match result {
            RischResult::Elementary(node) => Ok(node),
            RischResult::NonElementary(reason) => Err(format!("NON_ELEMENTARY: {}", reason)),
        });
    }
    None
}
```

Update `src/lib.rs` export:

```rust
pub use crate::risch::{
    hermite_reduce, try_risch_exponential, try_risch_logarithmic, try_risch_log_rational,
    DifferentialExtension, HermiteResult, RischResult,
};
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: no errors.

**Step 3: Commit**

```bash
git add src/integration.rs src/lib.rs
git commit -m "Wire Rothstein-Trager into integration engine fallback chain"
```

---

### Task 8: End-to-end integration tests

**Files:**
- Modify: `tests/integration.rs` (add new test cases)

**Step 1: Write end-to-end tests**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_integrate_1_over_x_ln_x() {
    // ∫1/(x·ln(x))dx = ln(ln(x)) + C
    let result = integrate_latex("\\frac{1}{x \\cdot \\ln(x)}", "x");
    assert!(result.is_ok(), "∫1/(x·ln(x))dx should succeed: {:?}", result);
    let s = result.unwrap();
    assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
    assert!(s.contains("+ C"), "Result should contain + C: {}", s);
}

#[test]
fn test_integrate_1_over_ln_x_non_elementary() {
    // ∫1/ln(x)dx — non-elementary
    let result = integrate_latex("\\frac{1}{\\ln(x)}", "x");
    assert!(result.is_err(), "∫1/ln(x)dx should be non-elementary");
    let err = result.unwrap_err();
    assert!(err.starts_with("NON_ELEMENTARY:"), "Expected NON_ELEMENTARY, got: {}", err);
}

#[test]
fn test_integrate_1_over_x_ln_x_minus_1() {
    // ∫1/(x·(ln(x)-1))dx = ln(ln(x)-1) + C
    let result = integrate_latex("\\frac{1}{x \\cdot (\\ln(x) - 1)}", "x");
    assert!(result.is_ok(), "∫1/(x·(ln(x)-1))dx should succeed: {:?}", result);
    let s = result.unwrap();
    assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
}

#[test]
fn test_integrate_1_over_1_plus_ln_x_non_elementary() {
    // ∫1/(1+ln(x))dx — non-elementary (gives Ei)
    let result = integrate_latex("\\frac{1}{1 + \\ln(x)}", "x");
    assert!(result.is_err(), "∫1/(1+ln(x))dx should be non-elementary");
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Step 2: Run all tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass, total count increases by ~20+.

**Step 3: Run clippy**

Run: `cargo clippy --tests -- -D warnings`
Expected: no warnings.

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "Add end-to-end tests for Rothstein-Trager logarithmic integration"
```

---

### Task 9: Numerical verification tests

**Files:**
- Modify: `tests/integration.rs` (add verification)

For elementary results, verify by differentiating the result and checking it matches the integrand at specific x values.

**Step 1: Write verification tests**

```rust
#[test]
fn test_integrate_1_over_x_ln_x_numerical() {
    // Verify: d/dx[ln(ln(x))] = 1/(x·ln(x))
    // At x = e² ≈ 7.389: ln(x) = 2, so 1/(x·ln(x)) ≈ 1/(7.389·2) ≈ 0.0677
    // d/dx[ln(ln(x))] = 1/(x·ln(x)) ✓
    let result = integrate_latex("\\frac{1}{x \\cdot \\ln(x)}", "x").unwrap();
    let integral_expr = result.replace(" + C", "");

    let mut env = Environment::new();
    let x_val = std::f64::consts::E * std::f64::consts::E; // e²
    env.set("x", x_val);

    let integral_val = evaluate_expression(&integral_expr, &env).unwrap();
    let expected = (x_val.ln()).ln(); // ln(ln(e²)) = ln(2)
    assert!(
        approx_eq(integral_val, expected, 0.01),
        "ln(ln(e²)) should be ln(2) ≈ {:.4}, got {:.4}",
        expected,
        integral_val
    );
}

#[test]
fn test_integrate_1_over_x_ln_x_minus_1_numerical() {
    // Verify at x = e³ ≈ 20.09: ln(x) = 3, ln(x)-1 = 2
    // ∫ = ln(ln(x)-1), so at x=e³: ln(3-1) = ln(2) ≈ 0.693
    let result = integrate_latex("\\frac{1}{x \\cdot (\\ln(x) - 1)}", "x").unwrap();
    let integral_expr = result.replace(" + C", "");

    let mut env = Environment::new();
    let x_val = std::f64::consts::E.powi(3);
    env.set("x", x_val);

    let integral_val = evaluate_expression(&integral_expr, &env).unwrap();
    let expected = (x_val.ln() - 1.0).ln(); // ln(3-1) = ln(2)
    assert!(
        approx_eq(integral_val, expected, 0.01),
        "Expected {:.4}, got {:.4}",
        expected,
        integral_val
    );
}
```

**Step 2: Run all tests**

Run: `cargo test 2>&1 | tail -5`
Expected: all pass.

**Step 3: Final clippy + fmt**

Run: `cargo fmt && cargo clippy --tests -- -D warnings`
Expected: clean.

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "Add numerical verification for Rothstein-Trager integration results"
```

---

## Notes for the Implementer

1. **ExactNum::from_bigrat** — may not exist yet. If not, add a constructor that wraps a BigRational. Check `src/exact.rs` for existing constructors.

2. **ExactNum::to_bigrat** — used in `node_to_extpoly` for Num nodes. Check if this exists. If not, check for `to_rational()` or similar. The existing `to_i64()` is too narrow; you may need to handle rational constants from the parser.

3. **Polynomial::to_node** — already exists. Returns the polynomial as a Node AST.

4. **The parser** — `\frac{1}{x \cdot \ln(x)}` must parse correctly as `Divide(1, Multiply(x, ln(x)))`. If the parser produces a different tree shape, adjust the pattern detector.

5. **Hermite reduction edge case** — when deg(num) ≥ deg(den), Hermite reduction does polynomial division first. The polynomial quotient must be integrated separately by the existing polynomial-log method. If this case arises, `try_risch_log_rational` returns None to let the existing handler work, or integrates the polynomial part itself.

6. **Clippy discipline** — run clippy after each task. Zero warnings policy.
