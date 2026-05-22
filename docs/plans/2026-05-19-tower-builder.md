# Tower Builder Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace three hand-coded pattern detectors with a unified tower builder that converts any single-extension expression to (ExtPoly num, ExtPoly den, DifferentialExtension), then dispatches to the appropriate integration algorithm. Unlocks rational-in-exp integration.

**Architecture:** The tower builder scans for transcendentals (exp/ln), classifies the extension type, converts the integrand to ExtPoly form, and routes to polynomial or rational integration. For exponential rational integrands, a residual step after Rothstein-Trager captures the polynomial part that logarithmic integration alone misses.

**Tech Stack:** Rust, existing ExtPoly/RationalFunction/Polynomial types, existing Hermite reduction and Rothstein-Trager.

---

### Task 1: Scanning helpers + generalized node_to_extpoly

**Files:**
- Modify: `src/risch.rs` (add before `#[cfg(test)]`)

Add scanning functions and a generalized expression-to-ExtPoly converter that handles both logarithmic (θ = ln(x)) and exponential (θ = exp(g(x))) extensions.

**Step 1: Write failing tests**

```rust
// === Scanning tests ===

#[test]
fn test_contains_ln_yes() {
    let expr = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
    assert!(contains_ln(&expr, "x"));
}

#[test]
fn test_contains_ln_nested() {
    // 1 + ln(x)
    let expr = Node::Add(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
    );
    assert!(contains_ln(&expr, "x"));
}

#[test]
fn test_contains_ln_no() {
    let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    assert!(!contains_ln(&expr, "x"));
}

#[test]
fn test_find_exp_arg_simple() {
    // exp(x) → argument is polynomial [0, 1] (= x)
    let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let arg = find_exp_argument(&expr, "x").unwrap();
    assert_eq!(arg, poly(&[0, 1], "x"));
}

#[test]
fn test_find_exp_arg_x_squared() {
    // exp(x^2) → argument is x^2
    let expr = Node::Function(
        "exp".to_string(),
        vec![Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::integer(2))),
        )],
    );
    let arg = find_exp_argument(&expr, "x").unwrap();
    assert_eq!(arg, poly(&[0, 0, 1], "x"));
}

#[test]
fn test_find_exp_arg_none() {
    let expr = Node::Variable("x".to_string());
    assert!(find_exp_argument(&expr, "x").is_none());
}

#[test]
fn test_find_exp_arg_in_product() {
    // x * exp(x) → finds exp(x) argument
    let expr = Node::Multiply(
        Box::new(Node::Variable("x".to_string())),
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let arg = find_exp_argument(&expr, "x").unwrap();
    assert_eq!(arg, poly(&[0, 1], "x"));
}

// === Generalized node_to_extpoly tests ===

#[test]
fn test_general_extpoly_exp_x() {
    // exp(x) with θ = exp(x) → θ
    let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
    let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
    assert_eq!(result, ExtPoly::theta("x"));
}

#[test]
fn test_general_extpoly_x_times_exp() {
    // x * exp(x) → [0, x] (x·θ)
    let expr = Node::Multiply(
        Box::new(Node::Variable("x".to_string())),
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
    let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
    assert_eq!(result.degree(), Some(1));
    assert_eq!(result.coeff(1), rf_poly(&[0, 1])); // x
    assert!(result.coeff(0).is_zero());
}

#[test]
fn test_general_extpoly_one_plus_exp() {
    // 1 + exp(x) → [1, 1] (1 + θ)
    let expr = Node::Add(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
    );
    let kind = ExtensionKind::Exponential(poly(&[0, 1], "x"));
    let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
    let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    assert_eq!(result, expected);
}

#[test]
fn test_general_extpoly_log_still_works() {
    // ln(x) + 1 with Logarithmic → [1, 1]
    let expr = Node::Add(
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        Box::new(Node::Num(ExactNum::integer(1))),
    );
    let kind = ExtensionKind::Logarithmic;
    let result = node_to_extpoly_general(&expr, "x", &kind).unwrap();
    let expected = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    assert_eq!(result, expected);
}
```

**Step 2: Run tests — expect compilation failure**

**Step 3: Implement**

```rust
/// Classification of a single transcendental extension.
enum ExtensionKind {
    Logarithmic,            // θ = ln(x)
    Exponential(Polynomial), // θ = exp(g(x))
}

/// Check if an expression contains ln(var).
fn contains_ln(expr: &Node, var: &str) -> bool {
    match expr {
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            matches!(&args[0], Node::Variable(v) if v == var)
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            contains_ln(l, var) || contains_ln(r, var)
        }
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => contains_ln(inner, var),
        Node::Power(base, exp) => contains_ln(base, var) || contains_ln(exp, var),
        Node::Function(_, args) => args.iter().any(|a| contains_ln(a, var)),
        _ => false,
    }
}

/// Find the polynomial argument of exp() subexpressions.
/// Returns Some(g) if all exp nodes share the same argument g(x), None otherwise.
fn find_exp_argument(expr: &Node, var: &str) -> Option<Polynomial> {
    match expr {
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            let fixed = fixup_negated_power(&args[0]);
            Polynomial::from_node(&fixed, var).ok()
        }
        Node::Add(l, r) | Node::Subtract(l, r) | Node::Multiply(l, r) | Node::Divide(l, r) => {
            match (find_exp_argument(l, var), find_exp_argument(r, var)) {
                (Some(a), Some(b)) if a == b => Some(a),
                (Some(a), None) | (None, Some(a)) => Some(a),
                _ => None,
            }
        }
        Node::Negate(inner) => find_exp_argument(inner, var),
        Node::Power(base, _) => find_exp_argument(base, var),
        _ => None,
    }
}

/// Convert a Node to ExtPoly in the given extension.
/// Handles both logarithmic (ln(x) → θ) and exponential (exp(g(x)) → θ) cases.
fn node_to_extpoly_general(expr: &Node, var: &str, kind: &ExtensionKind) -> Option<ExtPoly> {
    match expr {
        Node::Num(n) => {
            if let ExactNum::Rational(val) = n {
                Some(ExtPoly::from_rf(RationalFunction::from_constant(val.clone(), var)))
            } else {
                None
            }
        }
        Node::Variable(v) if v == var => {
            Some(ExtPoly::from_rf(RationalFunction::from_poly(Polynomial::x(var))))
        }
        Node::Variable(_) => None,

        // ln(x) → θ for logarithmic extensions
        Node::Function(name, args) if name == "ln" && args.len() == 1 => {
            if matches!(kind, ExtensionKind::Logarithmic) {
                if let Node::Variable(v) = &args[0] {
                    if v == var { return Some(ExtPoly::theta(var)); }
                }
            }
            None
        }

        // exp(g(x)) → θ for exponential extensions
        Node::Function(name, args) if name == "exp" && args.len() == 1 => {
            if let ExtensionKind::Exponential(ref g) = kind {
                let fixed = fixup_negated_power(&args[0]);
                if let Ok(arg_poly) = Polynomial::from_node(&fixed, var) {
                    if arg_poly == *g {
                        return Some(ExtPoly::theta(var));
                    }
                }
            }
            None
        }

        Node::Power(base, exp) => {
            // ln(x)^n for log extensions
            if matches!(kind, ExtensionKind::Logarithmic) {
                if let Node::Function(name, args) = base.as_ref() {
                    if name == "ln" && args.len() == 1 {
                        if let Node::Variable(v) = &args[0] {
                            if v == var {
                                if let Node::Num(n) = exp.as_ref() {
                                    if let Some(e) = n.to_i64() {
                                        if e >= 1 {
                                            let mut r = ExtPoly::theta(var);
                                            for _ in 1..e { r = &r * &ExtPoly::theta(var); }
                                            return Some(r);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // x^n (positive integer power)
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

        Node::Add(l, r) => Some(&node_to_extpoly_general(l, var, kind)? + &node_to_extpoly_general(r, var, kind)?),
        Node::Subtract(l, r) => Some(&node_to_extpoly_general(l, var, kind)? - &node_to_extpoly_general(r, var, kind)?),
        Node::Negate(inner) => Some(-&node_to_extpoly_general(inner, var, kind)?),
        Node::Multiply(l, r) => Some(&node_to_extpoly_general(l, var, kind)? * &node_to_extpoly_general(r, var, kind)?),
        Node::Divide(num, den) => {
            let n = node_to_extpoly_general(num, var, kind)?;
            let den_poly = Polynomial::from_node(den, var).ok()?;
            if den_poly.is_zero() { return None; }
            let inv = RationalFunction::new(Polynomial::one(var), den_poly);
            Some(n.scalar_mul(&inv))
        }
        _ => None,
    }
}
```

**Step 4:** Run tests. **Step 5:** Commit: "Add transcendental scanning and generalized ExtPoly conversion"

---

### Task 2: build_tower

**Files:**
- Modify: `src/risch.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_build_tower_log() {
    // 1/(x·ln(x)) → num=[1/x], den=[0,1], Logarithmic
    let expr = Node::Divide(
        Box::new(Node::Num(ExactNum::integer(1))),
        Box::new(Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        )),
    );
    let (num, den, ext) = build_tower(&expr, "x").unwrap();
    assert!(matches!(ext.ext_type(), ExtensionType::Logarithmic));
    assert!(num.is_constant());
    assert_eq!(den.degree(), Some(1));
}

#[test]
fn test_build_tower_exp_polynomial() {
    // 2x·exp(x²) → num=[0, 2x], den=[1], Exponential
    let expr = Node::Multiply(
        Box::new(Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(2))),
            Box::new(Node::Variable("x".to_string())),
        )),
        Box::new(Node::Function(
            "exp".to_string(),
            vec![Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )],
        )),
    );
    let (num, den, ext) = build_tower(&expr, "x").unwrap();
    assert!(matches!(ext.ext_type(), ExtensionType::Exponential));
    assert_eq!(num.degree(), Some(1));
    assert!(den.is_constant() || den == ExtPoly::one("x"));
}

#[test]
fn test_build_tower_exp_rational() {
    // exp(x)/(1+exp(x)) → num=[0,1], den=[1,1], Exponential
    let expr = Node::Divide(
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        Box::new(Node::Add(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
        )),
    );
    let (num, den, ext) = build_tower(&expr, "x").unwrap();
    assert!(matches!(ext.ext_type(), ExtensionType::Exponential));
    assert_eq!(num.degree(), Some(1));
    assert_eq!(den.degree(), Some(1));
}

#[test]
fn test_build_tower_mixed_returns_none() {
    // ln(x) * exp(x) → mixed, None
    let expr = Node::Multiply(
        Box::new(Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())])),
        Box::new(Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())])),
    );
    assert!(build_tower(&expr, "x").is_none());
}

#[test]
fn test_build_tower_no_transcendental() {
    let expr = Node::Variable("x".to_string());
    assert!(build_tower(&expr, "x").is_none());
}
```

**Step 2:** Expect compilation failure.

**Step 3: Implement**

```rust
/// Build a single-level transcendental tower from a Node expression.
///
/// Scans for exp/ln, classifies as logarithmic or exponential extension,
/// converts the integrand to (numerator, denominator) ExtPoly pair.
/// Returns None for mixed extensions, no transcendentals, or unsupported patterns.
pub fn build_tower(expr: &Node, var: &str) -> Option<(ExtPoly, ExtPoly, DifferentialExtension)> {
    if let Some(r) = build_tower_inner(expr, var) { return Some(r); }
    let env = crate::environment::Environment::new();
    let simplified = crate::simplify::Simplifiable::simplify(expr, &env).unwrap_or_else(|_| expr.clone());
    build_tower_inner(&simplified, var)
}

fn build_tower_inner(expr: &Node, var: &str) -> Option<(ExtPoly, ExtPoly, DifferentialExtension)> {
    let has_ln = contains_ln(expr, var);
    let exp_arg = find_exp_argument(expr, var);

    let (kind, ext) = match (has_ln, exp_arg) {
        (true, None) => (
            ExtensionKind::Logarithmic,
            DifferentialExtension::logarithmic(RationalFunction::from_poly(Polynomial::x(var)), var),
        ),
        (false, Some(g)) => (
            ExtensionKind::Exponential(g.clone()),
            DifferentialExtension::exponential(RationalFunction::from_poly(g), var),
        ),
        _ => return None, // mixed, both, or neither
    };

    // Try to decompose into num/den
    match expr {
        Node::Divide(num_node, den_node) => {
            let num = node_to_extpoly_general(num_node, var, &kind)?;
            let den = node_to_extpoly_general(den_node, var, &kind)?;
            if den.is_zero() { return None; }
            Some((num, den, ext))
        }
        Node::Multiply(left, right) => {
            // a * (b/c) where c involves θ
            if let Node::Divide(n, d) = right.as_ref() {
                if let Some(d_ep) = node_to_extpoly_general(d, var, &kind) {
                    if !d_ep.is_constant() {
                        let n_ep = node_to_extpoly_general(n, var, &kind)?;
                        let l_ep = node_to_extpoly_general(left, var, &kind)?;
                        return Some((&l_ep * &n_ep, d_ep, ext));
                    }
                }
            }
            if let Node::Divide(n, d) = left.as_ref() {
                if let Some(d_ep) = node_to_extpoly_general(d, var, &kind) {
                    if !d_ep.is_constant() {
                        let n_ep = node_to_extpoly_general(n, var, &kind)?;
                        let r_ep = node_to_extpoly_general(right, var, &kind)?;
                        return Some((&r_ep * &n_ep, d_ep, ext));
                    }
                }
            }
            // Polynomial in θ (den = 1)
            let num = node_to_extpoly_general(expr, var, &kind)?;
            Some((num, ExtPoly::one(var), ext))
        }
        _ => {
            let num = node_to_extpoly_general(expr, var, &kind)?;
            Some((num, ExtPoly::one(var), ext))
        }
    }
}
```

**Step 4:** Run tests. **Step 5:** Commit: "Add tower builder: scan, classify, convert"

---

### Task 3: integrate_poly_exp

**Files:**
- Modify: `src/risch.rs`

Polynomial-in-exp solver. For Σ aᵢ(x)·θⁱ where θ = exp(g(x)), each degree is an independent Risch DE: qᵢ' + i·g'·qᵢ = aᵢ.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_poly_exp_simple() {
    // ∫exp(x)dx: num = [0, 1], θ = exp(x)
    // q₁' + 1·q₁ = 1 → q₁ = 1, result = exp(x)
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let result = integrate_poly_exp(&num, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("exp"), "Expected exp in {}", s);
        }
        _ => panic!("Expected elementary"),
    }
}

#[test]
fn test_integrate_poly_exp_2x_exp_x2() {
    // ∫2x·exp(x²)dx: num = [0, 2x], θ = exp(x²), g' = 2x
    // q₁' + 2x·q₁ = 2x → q₁ = 1, result = exp(x²)
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_poly(&[0, 2])], "x");
    let result = integrate_poly_exp(&num, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(_) => {}
        _ => panic!("Expected elementary"),
    }
}

#[test]
fn test_integrate_poly_exp_non_elementary() {
    // ∫exp(-x²)dx: num = [0, 1], θ = exp(-x²), g' = -2x
    // q₁' + (-2x)·q₁ = 1 → no polynomial solution
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 0, -1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let result = integrate_poly_exp(&num, &ext, "x").unwrap();
    assert!(matches!(result, RischResult::NonElementary(_)));
}

#[test]
fn test_integrate_poly_exp_with_constant_term() {
    // ∫(1 + exp(x))dx: num = [1, 1], θ = exp(x)
    // q₀ = ∫1 = x, q₁' + q₁ = 1 → q₁ = 1
    // result = x + exp(x)
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x");
    let result = integrate_poly_exp(&num, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("x"), "Expected x in {}", s);
            assert!(s.contains("exp"), "Expected exp in {}", s);
        }
        _ => panic!("Expected elementary"),
    }
}
```

**Step 3: Implement**

```rust
/// Integrate a polynomial in θ = exp(g(x)): Σ aᵢ(x)·θⁱ.
///
/// Each degree decouples: qᵢ' + i·g'·qᵢ = aᵢ (independent Risch DE).
/// For i=0: q₀ = ∫a₀ (polynomial integration).
fn integrate_poly_exp(num: &ExtPoly, ext: &DifferentialExtension, var: &str) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let g_prime_rf = ext.argument().derivative();
    if *g_prime_rf.denominator() != Polynomial::one(var) {
        return None; // Rational-coeff Risch DE not implemented
    }
    let g_prime = g_prime_rf.numerator().clone();

    let mut q: Vec<Polynomial> = vec![Polynomial::zero(var); deg + 1];

    for i in 0..=deg {
        let a_i_rf = num.coeff(i);
        if a_i_rf.is_zero() { continue; }
        if *a_i_rf.denominator() != Polynomial::one(var) {
            return None; // Rational coefficient, need extended solver
        }
        let a_i = a_i_rf.numerator().clone();

        if i == 0 {
            q[0] = a_i.integral();
        } else {
            let f = g_prime.scalar_mul(&BigRational::from_integer(BigInt::from(i as i64)));
            match solve_risch_de_poly(&f, &a_i, var) {
                Some(qi) => q[i] = qi,
                None => {
                    return Some(RischResult::NonElementary(format!(
                        "No elementary antiderivative exists. \
                         The differential equation q' + ({})·q = {} has no polynomial solution.",
                        f, a_i
                    )));
                }
            }
        }
    }

    // Build result: Σ qᵢ · exp(g)^i
    let g_node = ext.argument().numerator().to_node();
    let mut terms: Vec<Node> = Vec::new();
    for (i, qi) in q.iter().enumerate() {
        if qi.is_zero() { continue; }
        let q_node = qi.to_node();
        let term = if i == 0 {
            q_node
        } else {
            let exp_g = Node::Function("exp".to_string(), vec![g_node.clone()]);
            let exp_part = if i == 1 { exp_g } else {
                Node::Power(Box::new(exp_g), Box::new(Node::Num(ExactNum::integer(i as i64))))
            };
            if *qi == Polynomial::one(var) { exp_part }
            else { Node::Multiply(Box::new(q_node), Box::new(exp_part)) }
        };
        terms.push(term);
    }

    if terms.is_empty() { return Some(RischResult::Elementary(Node::Num(ExactNum::zero()))); }
    let mut result = terms.remove(0);
    for t in terms { result = Node::Add(Box::new(result), Box::new(t)); }
    Some(RischResult::Elementary(result))
}
```

**Step 4:** Run tests. **Step 5:** Commit: "Add polynomial-in-exp integration via independent Risch DE"

---

### Task 4: integrate_poly_log (refactor from try_risch_logarithmic)

**Files:**
- Modify: `src/risch.rs`

Extract the polynomial-in-log integration logic from `try_risch_logarithmic` into a standalone function that takes an ExtPoly directly (instead of extracting from a Node). The algorithm is identical: top-down coefficient solving where qᵢ' + (i+1)·q_{i+1}/x = aᵢ.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_poly_log_ln_x() {
    // ∫ln(x)dx: num = [0, 1], θ = ln(x)
    // q₁ = ∫1 = x, then q₁/x = 1 (constant term 1 ≠ 0)... wait
    // Actually: q₁' = a₁ = 1 → q₁ = x
    // Then for k=0: q₁ has coeff(0) = 0 → q₁/x = 1 (degree 0)
    // rhs = a₀ - 1·(q₁/x) = 0 - 1 = -1
    // q₀ = ∫(-1) = -x
    // Result: -x + x·ln(x)
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x");
    let result = integrate_poly_log(&num, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln") || s.contains("ln"), "Expected ln in {}", s);
        }
        _ => panic!("Expected elementary"),
    }
}

#[test]
fn test_integrate_poly_log_x_ln_x() {
    // ∫x·ln(x)dx: num = [0, x]
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_poly(&[0, 1])], "x");
    let result = integrate_poly_log(&num, &ext, "x").unwrap();
    assert!(matches!(result, RischResult::Elementary(_)));
}
```

**Step 3: Implement**

```rust
/// Integrate a polynomial in θ = ln(x): Σ aᵢ(x)·θⁱ.
///
/// Top-down: qₙ' = aₙ, then qₖ' = aₖ - (k+1)·q_{k+1}/x.
/// Non-elementary if any q_{k+1} has nonzero constant term (q_{k+1}/x not polynomial).
fn integrate_poly_log(num: &ExtPoly, _ext: &DifferentialExtension, var: &str) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let mut q: Vec<Polynomial> = vec![Polynomial::zero(var); deg + 1];

    for k in (0..=deg).rev() {
        let a_k_rf = num.coeff(k);
        if *a_k_rf.denominator() != Polynomial::one(var) { return None; }
        let a_k = a_k_rf.numerator().clone();

        if k == deg {
            q[k] = a_k.integral();
        } else {
            let q_kp1 = &q[k + 1];
            if !q_kp1.coeff(0).is_zero() {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative of polynomial-in-ln(x) form exists. \
                     At degree {}, the coefficient has nonzero constant term.", k + 1
                )));
            }
            let (q_kp1_div_x, rem) = q_kp1.div_rem(&Polynomial::x(var)).unwrap();
            debug_assert!(rem.is_zero());
            let scalar = BigRational::from_integer(BigInt::from(k as i64 + 1));
            let correction = q_kp1_div_x.scalar_mul(&scalar);
            let rhs = &a_k - &correction;
            q[k] = rhs.integral();
        }
    }

    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let mut terms: Vec<Node> = Vec::new();
    for (k, qk) in q.iter().enumerate() {
        if qk.is_zero() { continue; }
        let q_node = qk.to_node();
        let term = match k {
            0 => q_node,
            1 => Node::Multiply(Box::new(q_node), Box::new(ln_x.clone())),
            _ => Node::Multiply(
                Box::new(q_node),
                Box::new(Node::Power(
                    Box::new(ln_x.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )),
            ),
        };
        terms.push(term);
    }

    if terms.is_empty() { return Some(RischResult::Elementary(Node::Num(ExactNum::zero()))); }
    let mut result = terms.remove(0);
    for t in terms { result = Node::Add(Box::new(result), Box::new(t)); }
    Some(RischResult::Elementary(result))
}
```

**Step 4:** Run tests. **Step 5:** Commit: "Add standalone polynomial-in-log integration"

---

### Task 5: integrate_rational_ext (generalized Hermite + RT + residual)

**Files:**
- Modify: `src/risch.rs`

Generalize `try_risch_log_rational` to work for any extension type. Add residual computation for exponential extensions.

**Step 1: Write failing tests**

```rust
#[test]
fn test_integrate_rational_log() {
    // ∫(1/x)/θ where θ=ln(x) → ln(ln(x))
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let one_over_x = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x"));
    let num = ExtPoly::from_rf(one_over_x);
    let den = ExtPoly::theta("x");
    let result = integrate_rational_ext(&num, &den, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Expected ln in {}", s);
        }
        _ => panic!("Expected elementary"),
    }
}

#[test]
fn test_integrate_rational_exp_elementary() {
    // ∫θ/(1+θ) where θ=exp(x) → ln(1+exp(x))
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_coeffs(vec![rf_const(0), rf_const(1)], "x"); // θ
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ
    let result = integrate_rational_ext(&num, &den, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Expected ln in {}", s);
        }
        _ => panic!("Expected elementary"),
    }
}

#[test]
fn test_integrate_rational_exp_with_residual() {
    // ∫1/(1+θ) where θ=exp(x) → x - ln(1+exp(x))
    // RT gives -ln(1+θ), residual is 1, ∫1 = x
    let ext = DifferentialExtension::exponential(
        RationalFunction::from_poly(poly(&[0, 1], "x")), "x"
    );
    let num = ExtPoly::from_rf(rf_const(1)); // 1
    let den = ExtPoly::from_coeffs(vec![rf_const(1), rf_const(1)], "x"); // 1+θ
    let result = integrate_rational_ext(&num, &den, &ext, "x").unwrap();
    match result {
        RischResult::Elementary(node) => {
            let s = format!("{}", node);
            assert!(s.contains("\\ln"), "Expected ln in {}", s);
            assert!(s.contains("x"), "Expected x (residual) in {}", s);
        }
        _ => panic!("Expected elementary, residual should integrate"),
    }
}
```

**Step 3: Implement**

```rust
/// Integrate a rational function in a single transcendental extension.
/// Uses Hermite reduction + Rothstein-Trager + residual integration (for exp).
fn integrate_rational_ext(
    num: &ExtPoly, den: &ExtPoly, ext: &DifferentialExtension, var: &str,
) -> Option<RischResult> {
    let hr = hermite_reduce(num, den, var).ok()?;

    let theta_node = match ext.ext_type() {
        ExtensionType::Logarithmic => {
            Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())])
        }
        ExtensionType::Exponential => {
            Node::Function("exp".to_string(), vec![ext.argument().numerator().to_node()])
        }
    };

    let mut result_terms: Vec<Node> = Vec::new();

    // Rational part from Hermite reduction
    if !hr.g_num.is_zero() {
        result_terms.push(Node::Divide(
            Box::new(extpoly_to_node(&hr.g_num, &theta_node, var)),
            Box::new(extpoly_to_node(&hr.g_den, &theta_node, var)),
        ));
    }

    if !hr.h_num.is_zero() {
        if hr.h_den.is_constant() {
            // Polynomial remainder
            let poly_result = match ext.ext_type() {
                ExtensionType::Logarithmic => integrate_poly_log(&hr.h_num, ext, var),
                ExtensionType::Exponential => integrate_poly_exp(&hr.h_num, ext, var),
            };
            match poly_result {
                Some(RischResult::Elementary(n)) => result_terms.push(n),
                Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
                None => return None,
            }
        } else {
            // Rothstein-Trager
            let dd = ext.differentiate(&hr.h_den);
            let rz = rothstein_trager_resultant(&hr.h_den, &hr.h_num, &dd, var);
            let roots = find_constant_roots(&rz, var);

            if roots.is_empty() {
                return Some(RischResult::NonElementary(
                    "No elementary antiderivative exists. \
                     The Rothstein-Trager resultant has no rational roots.".into()));
            }

            let h_den_deg = hr.h_den.degree().unwrap_or(0);
            let mut gcd_deg_sum = 0;
            let mut log_terms: Vec<(BigRational, ExtPoly)> = Vec::new();

            for c in &roots {
                let c_rf = RationalFunction::from_constant(c.clone(), var);
                let g_c = &hr.h_num - &dd.scalar_mul(&c_rf);
                let v = hr.h_den.gcd(&g_c);
                let v_deg = v.degree().unwrap_or(0);
                gcd_deg_sum += v_deg;
                if v_deg > 0 { log_terms.push((c.clone(), v)); }
            }

            if gcd_deg_sum != h_den_deg {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative exists. \
                     Rational residues cover degree {} but denominator has degree {}.",
                    gcd_deg_sum, h_den_deg)));
            }

            // Build log terms
            for (c, v) in &log_terms {
                let v_node = extpoly_to_node(v, &theta_node, var);
                let ln_v = Node::Function("ln".to_string(), vec![v_node]);
                let term = if *c == BigRational::one() { ln_v }
                else { Node::Multiply(Box::new(bigrat_to_node(c)), Box::new(ln_v)) };
                result_terms.push(term);
            }

            // Exp residual: integrand minus derivative of log terms
            if matches!(ext.ext_type(), ExtensionType::Exponential) {
                let mut log_deriv_num = ExtPoly::zero(var);
                for (c, v) in &log_terms {
                    let (w, rem) = hr.h_den.div_rem(v).unwrap();
                    debug_assert!(rem.is_zero(), "v should divide h_den");
                    let dv = ext.differentiate(v);
                    let c_rf = RationalFunction::from_constant(c.clone(), var);
                    log_deriv_num = &log_deriv_num + &(&w * &dv).scalar_mul(&c_rf);
                }
                let residual_num = &hr.h_num - &log_deriv_num;

                if !residual_num.is_zero() {
                    let (quotient, remainder) = residual_num.div_rem(&hr.h_den).unwrap();
                    if !remainder.is_zero() {
                        return Some(RischResult::NonElementary(
                            "No elementary antiderivative. Residual after RT is not polynomial.".into()));
                    }
                    match integrate_poly_exp(&quotient, ext, var) {
                        Some(RischResult::Elementary(n)) => result_terms.push(n),
                        Some(RischResult::NonElementary(r)) => return Some(RischResult::NonElementary(r)),
                        None => return None,
                    }
                }
            }
        }
    }

    if result_terms.is_empty() { return Some(RischResult::Elementary(Node::Num(ExactNum::zero()))); }
    let mut result = result_terms.remove(0);
    for t in result_terms { result = Node::Add(Box::new(result), Box::new(t)); }
    Some(RischResult::Elementary(result))
}
```

**Step 4:** Run tests. **Step 5:** Commit: "Add generalized rational integration with exp residual"

---

### Task 6: try_risch_tower + additive wiring

**Files:**
- Modify: `src/risch.rs` (add try_risch_tower)
- Modify: `src/integration.rs` (wire into fallback)
- Modify: `src/lib.rs` (export)

**Step 1: Implement try_risch_tower**

```rust
/// Unified Risch integration via tower builder.
///
/// Replaces try_risch_exponential, try_risch_logarithmic, and try_risch_log_rational
/// with a single entry point that handles both extension types.
pub fn try_risch_tower(expr: &Node, var: &str) -> Option<RischResult> {
    let (num, den, ext) = build_tower(expr, var)?;

    if den.is_constant() || den == ExtPoly::one(var) {
        match ext.ext_type() {
            ExtensionType::Logarithmic => integrate_poly_log(&num, &ext, var),
            ExtensionType::Exponential => integrate_poly_exp(&num, &ext, var),
        }
    } else {
        integrate_rational_ext(&num, &den, &ext, var)
    }
}
```

**Step 2: Wire into integration.rs**

Add `try_risch_tower` BEFORE the old paths in `try_risch_fallback`:

```rust
use crate::risch::{
    try_risch_exponential, try_risch_log_rational, try_risch_logarithmic,
    try_risch_tower, RischResult,
};

fn try_risch_fallback(expr: &Node, var_name: &str) -> Option<Result<Node, String>> {
    // New unified tower path
    if let Some(result) = try_risch_tower(expr, var_name) {
        return Some(match result {
            RischResult::Elementary(node) => Ok(node),
            RischResult::NonElementary(reason) => Err(format!("NON_ELEMENTARY: {}", reason)),
        });
    }
    // Old paths as fallback (to be removed once tower handles all cases)
    if let Some(result) = try_risch_exponential(expr, var_name) { ... }
    if let Some(result) = try_risch_logarithmic(expr, var_name) { ... }
    if let Some(result) = try_risch_log_rational(expr, var_name) { ... }
    None
}
```

Update `src/lib.rs` to export `try_risch_tower` and `build_tower`.

**Step 3: Run FULL test suite**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{s+=$4; f+=$6} END {print "Passed:", s, "Failed:", f}'`
Expected: 747+ passed, 0 failed. ALL existing tests must still pass.

**Step 4:** Commit: "Add unified Risch tower dispatcher"

---

### Task 7: New capability tests + remove old code

**Files:**
- Modify: `tests/integration.rs` (add new tests)
- Modify: `src/risch.rs` (remove old functions and tests)
- Modify: `src/integration.rs` (simplify fallback)
- Modify: `src/lib.rs` (update exports)

**Step 1: Add end-to-end tests for new capabilities**

```rust
#[test]
fn test_integrate_exp_over_1_plus_exp() {
    // ∫exp(x)/(1+exp(x))dx = ln(1+exp(x)) + C
    let result = integrate_latex("\\frac{\\exp(x)}{1 + \\exp(x)}", "x");
    assert!(result.is_ok(), "∫exp(x)/(1+exp(x))dx should succeed: {:?}", result);
    let s = result.unwrap();
    assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
}

#[test]
fn test_integrate_1_over_1_plus_exp() {
    // ∫1/(1+exp(x))dx = x - ln(1+exp(x)) + C
    let result = integrate_latex("\\frac{1}{1 + \\exp(x)}", "x");
    assert!(result.is_ok(), "∫1/(1+exp(x))dx should succeed: {:?}", result);
    let s = result.unwrap();
    assert!(s.contains("\\ln"), "Result should contain ln: {}", s);
    assert!(s.contains("x"), "Result should contain x (from residual): {}", s);
}

#[test]
fn test_integrate_exp_over_1_plus_exp_numerical() {
    // Verify: d/dx[ln(1+exp(x))] = exp(x)/(1+exp(x))
    let result = integrate_latex("\\frac{\\exp(x)}{1 + \\exp(x)}", "x").unwrap();
    let integral_expr = result.replace(" + C", "");
    let mut env = Environment::new();
    env.set("x", 1.0);
    let val = evaluate_expression(&integral_expr, &env).unwrap();
    let expected = (1.0 + std::f64::consts::E).ln();
    assert!(
        approx_eq(val, expected, 0.01),
        "Expected {:.4}, got {:.4}", expected, val
    );
}
```

**Step 2: Run new tests — expect PASS (tower handles these)**

**Step 3: Remove old code from src/risch.rs**

Remove these functions:
- `extract_exp_pattern`, `extract_exp_pattern_inner`, `fixup_negated_power` — keep fixup_negated_power (used by tower builder)
- `extract_log_pattern`, `extract_log_pattern_inner`, `add_coeff_vecs`, `sub_coeff_vecs`
- `extract_log_rational_pattern`, `extract_log_rational_inner`, old `node_to_extpoly`
- `try_risch_exponential`
- `try_risch_logarithmic`
- `try_risch_log_rational`

Remove corresponding tests:
- `test_extract_exp_*` (8 tests)
- `test_extract_log_*` (4 tests)
- `test_risch_log_*` (4 tests — the polynomial-log ones)
- `test_extract_log_rational_*` (3 tests)
- `test_risch_log_rational_*` (4 tests)
- Old `test_node_to_extpoly_*` (4 tests — replaced by general versions)

**Step 4: Simplify try_risch_fallback**

```rust
fn try_risch_fallback(expr: &Node, var_name: &str) -> Option<Result<Node, String>> {
    if let Some(result) = try_risch_tower(expr, var_name) {
        return Some(match result {
            RischResult::Elementary(node) => Ok(node),
            RischResult::NonElementary(reason) => Err(format!("NON_ELEMENTARY: {}", reason)),
        });
    }
    None
}
```

Update lib.rs exports: remove old function exports, keep `try_risch_tower`, `build_tower`.

**Step 5: Run full test suite + clippy**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`
Expected: All existing end-to-end tests pass, new tests pass, old unit tests removed.

**Step 6:** Commit: "Replace pattern detectors with unified tower builder"

---

## Notes for the Implementer

1. **`fixup_negated_power`** — keep this function. It's used by `find_exp_argument` and `node_to_extpoly_general` for the parser `-x^2` precedence bug.

2. **`ExactNum` matching** — the current `node_to_extpoly` matches `ExactNum::Rational(val)`. Use the same pattern in the generalized version. Integer ExactNum values may need to be handled separately depending on the ExactNum enum variants.

3. **Polynomial comparison** — `Polynomial` has `PartialEq` (line 707 of polynomial.rs). Used by `find_exp_argument` to check if all exp arguments match.

4. **The residual computation** (Task 5) is the trickiest part. Walk through ∫1/(1+exp(x))dx by hand to verify: RT gives c=-1, v=1+θ, D(v)=θ, residual_num = 1 - (-1)·θ = 1+θ, (1+θ)/(1+θ) = 1, ∫1 = x.

5. **Test count** — you'll remove ~27 old unit tests but add ~20+ new ones. The end-to-end tests in `tests/integration.rs` should not change (all existing ones must keep passing). The total test count may decrease slightly but capability increases.

6. **Order matters** — build Task 1-5 additively (don't remove old code yet). Task 6 wires in the new code BEFORE old fallbacks. Only Task 7 removes old code, after verifying everything works.
