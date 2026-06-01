# Parametric Quadratic Denominator Integration

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate rational functions with symbolic quadratic denominators: ∫(px+q)/(ax²+bx+c)dx where a,b,c,p,q are arbitrary symbolic expressions (not necessarily numeric).

**Architecture:** Add `try_decompose_quadratic(expr, var)` to extract (a, b, c) from ax²+bx+c at the Node level — the quadratic analogue of the existing `try_decompose_linear`. Then add a handler in the Divide branch of `integrate()` that detects parametric quadratic denominators and applies the closed-form formula: `(p/2a)·ln|ax²+bx+c| + ((2aq-bp)/√(4ac-b²))·arctan((2ax+b)/√(4ac-b²))`. Insert after the existing parametric linear handler (line ~278) and before `try_inverse_trig_integral` (line ~280).

**Tech Stack:** Rust, Node AST, existing simplifier, existing `contains_var` and `try_decompose_linear` as patterns.

**Mathematical foundation:**

For ∫(px+q)/(ax²+bx+c)dx, split the numerator into derivative-of-denominator + constant:
- d/dx(ax²+bx+c) = 2ax+b, so px+q = (p/2a)(2ax+b) + (q - pb/2a)
- First piece: ∫(p/2a)(2ax+b)/(ax²+bx+c)dx = (p/2a)·ln|ax²+bx+c|
- Second piece: ∫(q-pb/2a)/(ax²+bx+c)dx = ((2aq-bp)/(2a))·∫1/(ax²+bx+c)dx

For ∫1/(ax²+bx+c)dx, complete the square:
- ax²+bx+c = a[(x+b/2a)² + (4ac-b²)/4a²]
- ∫dx/[a((x+b/2a)² + (4ac-b²)/4a²)] = (2/√(4ac-b²))·arctan((2ax+b)/√(4ac-b²))

Combined result:
```
∫(px+q)/(ax²+bx+c)dx = (p/2a)·ln|ax²+bx+c| + ((2aq-bp)/√(4ac-b²))·arctan((2ax+b)/√(4ac-b²))
```

Special cases:
- p=0: pure constant numerator, only arctan term
- b=0: no completing the square needed, arctan simplifies to arctan(x·√(a/c)/something)
- q-pb/2a=0: numerator is exact multiple of derivative, only ln term

---

### Task 1: `try_decompose_quadratic`

**Files:**
- Modify: `src/integration.rs` (add after `try_decompose_linear`, around line 608)

**Step 1: Write the failing test**

In `src/integration.rs`, in the `#[cfg(test)]` section at the bottom, add:

```rust
#[test]
fn test_decompose_quadratic() {
    use crate::node::Node;
    use crate::exact::ExactNum;

    // x² + a → (1, 0, a)
    let x2_plus_a = Node::Add(
        Box::new(Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::two())),
        )),
        Box::new(Node::Variable("a".to_string())),
    );
    let result = try_decompose_quadratic(&x2_plus_a, "x");
    assert!(result.is_some(), "x²+a should decompose");
    let (a_coeff, b_coeff, c_coeff) = result.unwrap();
    assert_eq!(format!("{}", a_coeff), "1");
    assert_eq!(format!("{}", b_coeff), "0");
    assert_eq!(format!("{}", c_coeff), "a");

    // 2x² + 3x + a → (2, 3, a)
    let expr = Node::Add(
        Box::new(Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::two())),
                Box::new(Node::Power(
                    Box::new(Node::Variable("x".to_string())),
                    Box::new(Node::Num(ExactNum::two())),
                )),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(3))),
                Box::new(Node::Variable("x".to_string())),
            )),
        )),
        Box::new(Node::Variable("a".to_string())),
    );
    let result = try_decompose_quadratic(&expr, "x");
    assert!(result.is_some(), "2x²+3x+a should decompose");

    // Pure constant — not quadratic
    let constant = Node::Variable("a".to_string());
    assert!(try_decompose_quadratic(&constant, "x").is_none());

    // Contains x³ — not quadratic (would need cubic decomposition)
    let cubic = Node::Add(
        Box::new(Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::integer(3))),
        )),
        Box::new(Node::Variable("a".to_string())),
    );
    assert!(try_decompose_quadratic(&cubic, "x").is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib test_decompose_quadratic`
Expected: FAIL — `try_decompose_quadratic` not found

**Step 3: Implement `try_decompose_quadratic`**

Add after `try_decompose_linear` (after line 608):

```rust
/// Decompose an expression into ax² + bx + c form at the Node level.
/// Returns Some((a, b, c)) where a, b, c are Node expressions not containing var.
/// Returns None if the expression doesn't have this form.
fn try_decompose_quadratic(expr: &Node, var: &str) -> Option<(Node, Node, Node)> {
    let mut a_coeff = Node::Num(ExactNum::zero());
    let mut b_coeff = Node::Num(ExactNum::zero());
    let mut c_coeff = Node::Num(ExactNum::zero());

    if !collect_quadratic_terms(expr, var, false, &mut a_coeff, &mut b_coeff, &mut c_coeff) {
        return None;
    }

    // Must actually be quadratic (a ≠ 0) and a must not contain var
    let env = crate::environment::Environment::new();
    a_coeff = crate::simplify::Simplifiable::simplify(&a_coeff, &env).unwrap_or(a_coeff);
    b_coeff = crate::simplify::Simplifiable::simplify(&b_coeff, &env).unwrap_or(b_coeff);
    c_coeff = crate::simplify::Simplifiable::simplify(&c_coeff, &env).unwrap_or(c_coeff);

    if matches!(&a_coeff, Node::Num(n) if n.is_zero()) {
        return None;
    }

    Some((a_coeff, b_coeff, c_coeff))
}

/// Recursively collect terms of a polynomial in var into a, b, c accumulators.
/// `negated` tracks whether we're inside a subtraction/negation.
/// Returns false if a term with degree > 2 is found.
fn collect_quadratic_terms(
    expr: &Node,
    var: &str,
    negated: bool,
    a: &mut Node,
    b: &mut Node,
    c: &mut Node,
) -> bool {
    match expr {
        // x² term (bare)
        Node::Power(base, exp) => {
            if matches!(&**base, Node::Variable(v) if v == var)
                && matches!(&**exp, Node::Num(n) if *n == ExactNum::two())
            {
                let term = Node::Num(ExactNum::one());
                add_to_accumulator(a, &term, negated);
                return true;
            }
            if matches!(&**base, Node::Variable(v) if v == var) {
                if let Node::Num(n) = &**exp {
                    if n.to_f64() > 2.0 {
                        return false;
                    }
                }
            }
            if !contains_var(expr, var) {
                add_to_accumulator(c, expr, negated);
                return true;
            }
            false
        }
        // x term (bare variable)
        Node::Variable(v) if v == var => {
            let term = Node::Num(ExactNum::one());
            add_to_accumulator(b, &term, negated);
            true
        }
        // Constant or non-var expression
        _ if !contains_var(expr, var) => {
            add_to_accumulator(c, expr, negated);
            true
        }
        // Multiply: could be coeff*x², coeff*x, or coeff*x^n
        Node::Multiply(left, right) => {
            // Try coeff * x²
            if let Node::Power(base, exp) = &**right {
                if matches!(&**base, Node::Variable(v) if v == var)
                    && matches!(&**exp, Node::Num(n) if *n == ExactNum::two())
                    && !contains_var(left, var)
                {
                    add_to_accumulator(a, left, negated);
                    return true;
                }
                if matches!(&**base, Node::Variable(v) if v == var) {
                    if let Node::Num(n) = &**exp {
                        if n.to_f64() > 2.0 {
                            return false;
                        }
                    }
                }
            }
            if let Node::Power(base, exp) = &**left {
                if matches!(&**base, Node::Variable(v) if v == var)
                    && matches!(&**exp, Node::Num(n) if *n == ExactNum::two())
                    && !contains_var(right, var)
                {
                    add_to_accumulator(a, right, negated);
                    return true;
                }
                if matches!(&**base, Node::Variable(v) if v == var) {
                    if let Node::Num(n) = &**exp {
                        if n.to_f64() > 2.0 {
                            return false;
                        }
                    }
                }
            }
            // Try coeff * x (linear term)
            if let Node::Variable(v) = &**right {
                if v == var && !contains_var(left, var) {
                    add_to_accumulator(b, left, negated);
                    return true;
                }
            }
            if let Node::Variable(v) = &**left {
                if v == var && !contains_var(right, var) {
                    add_to_accumulator(b, right, negated);
                    return true;
                }
            }
            // If neither factor contains var, it's a constant
            if !contains_var(expr, var) {
                add_to_accumulator(c, expr, negated);
                return true;
            }
            false
        }
        // Add: recurse into both sides
        Node::Add(left, right) => {
            collect_quadratic_terms(left, var, negated, a, b, c)
                && collect_quadratic_terms(right, var, negated, a, b, c)
        }
        // Subtract: recurse with flipped negation on right
        Node::Subtract(left, right) => {
            collect_quadratic_terms(left, var, negated, a, b, c)
                && collect_quadratic_terms(right, var, !negated, a, b, c)
        }
        // Negate: flip negation flag
        Node::Negate(inner) => collect_quadratic_terms(inner, var, !negated, a, b, c),
        _ => false,
    }
}

/// Add a term to an accumulator, respecting negation.
fn add_to_accumulator(acc: &mut Node, term: &Node, negated: bool) {
    let effective = if negated {
        Node::Negate(Box::new(term.clone()))
    } else {
        term.clone()
    };
    if matches!(acc, Node::Num(n) if n.is_zero()) {
        *acc = effective;
    } else {
        *acc = Node::Add(Box::new(acc.clone()), Box::new(effective));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib test_decompose_quadratic`
Expected: PASS

**Step 5: Commit**

```
git add src/integration.rs
git commit -m "feat: try_decompose_quadratic for Node-level quadratic extraction"
```

---

### Task 2: `try_parametric_quadratic_integral`

**Files:**
- Modify: `src/integration.rs` (add after `try_decompose_quadratic`)

**Step 1: Write the failing test**

In the `#[cfg(test)]` section:

```rust
#[test]
fn test_parametric_quadratic_simple() {
    // ∫1/(x²+a) dx = (1/√a)·arctan(x/√a)
    let result = integrate_latex("\\frac{1}{x^2 + a}", "x");
    assert!(result.is_ok(), "Should integrate 1/(x²+a): {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}

#[test]
fn test_parametric_quadratic_full() {
    // ∫1/(ax²+bx+c) dx
    let result = integrate_latex("\\frac{1}{a x^2 + b x + c}", "x");
    assert!(result.is_ok(), "Should integrate 1/(ax²+bx+c): {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}

#[test]
fn test_parametric_quadratic_linear_numerator() {
    // ∫x/(x²+a) dx = (1/2)·ln|x²+a|
    let result = integrate_latex("\\frac{x}{x^2 + a}", "x");
    assert!(result.is_ok(), "Should integrate x/(x²+a): {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("ln"), "Should contain ln: {}", r);
}

#[test]
fn test_parametric_quadratic_scaled() {
    // ∫3/(2x²+c) dx — scaled constant numerator
    let result = integrate_latex("\\frac{3}{2 x^2 + c}", "x");
    assert!(result.is_ok(), "Should integrate 3/(2x²+c): {:?}", result);
    let r = result.unwrap();
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib test_parametric_quadratic`
Expected: FAIL — the integration falls through to error

**Step 3: Implement `try_parametric_quadratic_integral`**

Add after `try_decompose_quadratic` and its helpers:

```rust
/// Integrate (px+q)/(ax²+bx+c) dx where a,b,c,p,q are symbolic Node expressions.
///
/// Formula: (p/2a)·ln|ax²+bx+c| + ((2aq-bp)/√(4ac-b²))·arctan((2ax+b)/√(4ac-b²))
///
/// The numerator (px+q) is extracted via try_decompose_linear on the numerator expression.
/// If the numerator doesn't contain var, it's treated as q with p=0.
fn try_parametric_quadratic_integral(
    numerator: &Node,
    denominator: &Node,
    var: &str,
) -> Option<Result<Node, String>> {
    let (a, b_node, c) = try_decompose_quadratic(denominator, var)?;

    // Extract numerator as px + q
    let (p, q) = if !contains_var(numerator, var) {
        // Constant numerator: p=0, q=numerator
        (Node::Num(ExactNum::zero()), numerator.clone())
    } else if let Some((p_coeff, q_const)) = try_decompose_linear(numerator, var) {
        (p_coeff, q_const)
    } else {
        return None;
    };

    let env = crate::environment::Environment::new();
    let x = Node::Variable(var.to_string());
    let two = Node::Num(ExactNum::two());
    let four = Node::Num(ExactNum::integer(4));

    let mut terms: Vec<Node> = Vec::new();

    // --- Ln term: (p/2a)·ln|ax²+bx+c| ---
    let p_is_zero = matches!(&p, Node::Num(n) if n.is_zero());
    if !p_is_zero {
        // p / (2a)
        let ln_coeff = Node::Divide(
            Box::new(p.clone()),
            Box::new(Node::Multiply(Box::new(two.clone()), Box::new(a.clone()))),
        );
        let ln_coeff =
            crate::simplify::Simplifiable::simplify(&ln_coeff, &env).unwrap_or(ln_coeff);

        let ln_arg = Node::Function(
            "ln".to_string(),
            vec![Node::Abs(Box::new(denominator.clone()))],
        );
        terms.push(Node::Multiply(Box::new(ln_coeff), Box::new(ln_arg)));
    }

    // --- Arctan term: ((2aq - bp) / √(4ac - b²)) · arctan((2ax + b) / √(4ac - b²)) ---
    // Compute arctan_num = 2aq - bp
    let two_aq = Node::Multiply(
        Box::new(two.clone()),
        Box::new(Node::Multiply(
            Box::new(a.clone()),
            Box::new(q.clone()),
        )),
    );
    let bp = Node::Multiply(Box::new(b_node.clone()), Box::new(p.clone()));
    let arctan_num = Node::Subtract(Box::new(two_aq), Box::new(bp));
    let arctan_num =
        crate::simplify::Simplifiable::simplify(&arctan_num, &env).unwrap_or(arctan_num);

    let arctan_num_is_zero = matches!(&arctan_num, Node::Num(n) if n.is_zero());

    if !arctan_num_is_zero {
        // discriminant = 4ac - b²
        let four_ac = Node::Multiply(
            Box::new(four.clone()),
            Box::new(Node::Multiply(
                Box::new(a.clone()),
                Box::new(c.clone()),
            )),
        );
        let b_sq = Node::Multiply(Box::new(b_node.clone()), Box::new(b_node.clone()));
        let disc = Node::Subtract(Box::new(four_ac), Box::new(b_sq));
        let disc = crate::simplify::Simplifiable::simplify(&disc, &env).unwrap_or(disc);

        let sqrt_disc = Node::Function("sqrt".to_string(), vec![disc]);

        // arctan argument: (2ax + b) / √(4ac - b²)
        let two_ax = Node::Multiply(
            Box::new(two.clone()),
            Box::new(Node::Multiply(Box::new(a.clone()), Box::new(x.clone()))),
        );
        let arctan_inner = Node::Add(Box::new(two_ax), Box::new(b_node.clone()));
        let arctan_arg = Node::Divide(Box::new(arctan_inner), Box::new(sqrt_disc.clone()));

        let arctan_node = Node::Function("arctan".to_string(), vec![arctan_arg]);

        // coefficient: (2aq - bp) / √(4ac - b²)
        let arctan_coeff = Node::Divide(Box::new(arctan_num), Box::new(sqrt_disc));
        let arctan_coeff =
            crate::simplify::Simplifiable::simplify(&arctan_coeff, &env).unwrap_or(arctan_coeff);

        terms.push(Node::Multiply(
            Box::new(arctan_coeff),
            Box::new(arctan_node),
        ));
    }

    if terms.is_empty() {
        return Some(Ok(Node::Num(ExactNum::zero())));
    }

    let mut result = terms
        .into_iter()
        .reduce(|acc, t| Node::Add(Box::new(acc), Box::new(t)))
        .unwrap();
    result = crate::simplify::Simplifiable::simplify(&result, &env).unwrap_or(result);
    Some(Ok(result))
}
```

**Step 4: Run tests — still failing** (not wired in yet)

Run: `cargo test --lib test_parametric_quadratic`
Expected: FAIL — handler not wired into `integrate()`

**Step 5: Commit**

```
git add src/integration.rs
git commit -m "feat: parametric quadratic integration formula — (px+q)/(ax²+bx+c)"
```

---

### Task 3: Wire Into Integration Engine

**Files:**
- Modify: `src/integration.rs` — the `integrate()` function, Divide branch (around line 278)

**Step 1: Add the handler**

After the parametric linear handler (the block ending at line ~278 with `}`) and before the `try_inverse_trig_integral` call (line ~280), insert:

```rust
            // ∫(px+q)/(ax²+bx+c) dx with symbolic coefficients — parametric quadratic denominator
            if let Some(result) = try_parametric_quadratic_integral(left, right, var_name) {
                return result;
            }
```

The guard logic is inside `try_parametric_quadratic_integral` — it calls `try_decompose_quadratic` on the denominator and returns `None` if the denominator isn't quadratic in `var`.

**Step 2: Run tests to verify they pass**

Run: `cargo test --lib test_parametric_quadratic`
Expected: All 4 tests PASS

**Step 3: Run full test suite for regressions**

Run: `cargo test`
Expected: All 928+ tests PASS

**Step 4: Run clippy**

Run: `cargo clippy --tests -- -D warnings`
Expected: 0 warnings

**Step 5: Commit**

```
git add src/integration.rs
git commit -m "feat: wire parametric quadratic integration into engine"
```

---

### Task 4: E2E Tests and Edge Cases

**Files:**
- Modify: `tests/parser_hardening.rs` (add after existing parametric tests, line ~188)

**Step 1: Add e2e tests**

```rust
#[test]
fn test_integrate_parametric_quadratic_simple() {
    // ∫1/(x²+a) dx = (1/√a)·arctan(x/√a)
    let env = Environment::new();
    let expr = parse_latex("\\frac{1}{x^2 + a}", &env).unwrap();
    let result = arithma::integrate(&expr, "x");
    assert!(result.is_ok(), "Should integrate 1/(x²+a): {:?}", result);
    let r = format!("{}", result.unwrap());
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}

#[test]
fn test_integrate_parametric_quadratic_full_abc() {
    // ∫1/(ax²+bx+c) dx — full general case
    let env = Environment::new();
    let expr = parse_latex("\\frac{1}{a x^2 + b x + c}", &env).unwrap();
    let result = arithma::integrate(&expr, "x");
    assert!(result.is_ok(), "Should integrate 1/(ax²+bx+c): {:?}", result);
    let r = format!("{}", result.unwrap());
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}

#[test]
fn test_integrate_parametric_quadratic_linear_num() {
    // ∫x/(x²+a) dx = (1/2)·ln|x²+a| — pure log result
    let env = Environment::new();
    let expr = parse_latex("\\frac{x}{x^2 + a}", &env).unwrap();
    let result = arithma::integrate(&expr, "x");
    assert!(result.is_ok(), "Should integrate x/(x²+a): {:?}", result);
    let r = format!("{}", result.unwrap());
    assert!(r.contains("ln"), "Should contain ln: {}", r);
}

#[test]
fn test_integrate_parametric_quadratic_both_terms() {
    // ∫(x+1)/(x²+a) dx — both ln and arctan
    let env = Environment::new();
    let expr = parse_latex("\\frac{x + 1}{x^2 + a}", &env).unwrap();
    let result = arithma::integrate(&expr, "x");
    assert!(result.is_ok(), "Should integrate (x+1)/(x²+a): {:?}", result);
    let r = format!("{}", result.unwrap());
    assert!(r.contains("ln"), "Should contain ln: {}", r);
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}

#[test]
fn test_integrate_parametric_quadratic_no_linear() {
    // ∫1/(x²+2x+a) dx — quadratic with linear term, no x in numerator
    let env = Environment::new();
    let expr = parse_latex("\\frac{1}{x^2 + 2 x + a}", &env).unwrap();
    let result = arithma::integrate(&expr, "x");
    assert!(result.is_ok(), "Should integrate 1/(x²+2x+a): {:?}", result);
    let r = format!("{}", result.unwrap());
    assert!(r.contains("arctan"), "Should contain arctan: {}", r);
}
```

**Step 2: Run the e2e tests**

Run: `cargo test --test parser_hardening test_integrate_parametric_quadratic`
Expected: All 5 PASS

**Step 3: Run full suite + clippy + fmt**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`
Expected: All clean

**Step 4: Commit**

```
git add tests/parser_hardening.rs
git commit -m "test: e2e tests for parametric quadratic integration"
```

---

### Task 5: Numerical Verification

**Files:**
- Modify: `tests/parser_hardening.rs`

Verify the parametric formula gives correct results by substituting specific numeric values and checking against known integrals.

**Step 1: Add numerical verification test**

```rust
#[test]
fn test_parametric_quadratic_numerical_consistency() {
    // ∫1/(x²+4) dx should equal (1/2)·arctan(x/2) + C
    // At x=2: (1/2)·arctan(1) = (1/2)·(π/4) = π/8 ≈ 0.3927
    // At x=0: (1/2)·arctan(0) = 0
    // So definite integral from 0 to 2 ≈ 0.3927
    let result = arithma::definite_integral_latex("\\frac{1}{x^2 + 4}", "x", 0.0, 2.0);
    assert!(result.is_ok(), "Definite integral should work: {:?}", result);
    let val: f64 = result.unwrap().parse().unwrap();
    let expected = std::f64::consts::FRAC_PI_8;
    assert!(
        (val - expected).abs() < 0.001,
        "∫₀² 1/(x²+4)dx ≈ π/8 ≈ {:.4}, got {:.4}",
        expected,
        val
    );
}
```

**Step 2: Run the test**

Run: `cargo test --test parser_hardening test_parametric_quadratic_numerical`
Expected: PASS

**Step 3: If the test fails**, the issue is likely in how `definite_integral` evaluates the symbolic sqrt. The existing `definite_integral` substitutes numeric values and evaluates — this should work since substituting a=4 into `arctan(x/√a)` gives `arctan(x/2)`. Debug from there.

**Step 4: Commit**

```
git add tests/parser_hardening.rs
git commit -m "test: numerical verification for parametric quadratic integration"
```

---

## Regression Risks

1. **Existing numeric quadratic integrals.** The handler goes before `try_inverse_trig_integral` (line ~280) which handles `∫1/(a²+x²)dx` for numeric `a`. The new handler will match these cases too. Verify the results are equivalent — mathematically they should be, since the formula reduces to the same thing when a,b,c are numeric. If the simplifier produces different-looking but equivalent output, that's acceptable.

2. **Partial fractions path.** `try_partial_fraction_integration` (line ~317) handles rational functions with numeric coefficients. The new handler goes before it, so parametric quadratics are caught first. Non-quadratic denominators and higher-degree numerators still fall through correctly.

3. **Risch fallback.** The Risch tower builder requires `BigRational` coefficients. Parametric expressions won't match the Risch pattern detectors, so they'd fail there. Catching them earlier is correct.

## What This Does NOT Handle

- Parametric denominators where the discriminant is positive (factoring into real linear factors with symbolic roots — requires symbolic square root evaluation)
- Cubic or higher parametric denominators
- Numerators of degree ≥ 2 over quadratic denominators (polynomial division needed first)
- Negative discriminant detection (we always produce the arctan form; if 4ac-b² < 0 at runtime, the sqrt will be imaginary — correct for complex analysis, surprising for real-valued use)
