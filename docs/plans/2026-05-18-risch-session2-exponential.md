# Risch Session 2: Exponential Integration & Non-Elementary Detection

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable arithma to integrate r(x)·exp(g(x)) and PROVE when no elementary antiderivative exists (e.g., ∫e^{-x²}dx).

**Architecture:** For ∫p(x)·exp(g(x))dx, Liouville's theorem says the antiderivative (if elementary) has the form q(x)·exp(g(x)). This reduces to the Risch differential equation: q' + g'·q = p. We solve this by coefficient matching — if no solution exists, the integral is provably non-elementary. The result is wired into the existing integration engine as a last-resort method, with non-elementary results surfaced through MCP and CLI.

**Tech Stack:** Rust, existing `Polynomial` and `RationalFunction` types, `num-bigint`/`num-rational`.

---

### Task 1: Risch Differential Equation Solver (Polynomial Case)

**Files:**
- Modify: `src/risch.rs`

This is the mathematical core. Given polynomials f, g ∈ Q[x], find polynomial q ∈ Q[x] satisfying q' + f·q = g, or prove no such q exists.

**Algorithm:**

For q' + f·q = g where f has degree m, g has degree n:
1. **Degree bound:** If m ≥ 1, then deg(q) = n − m. If n < m and g ≠ 0, no solution exists immediately.
   If m = 0 (f constant), deg(q) = n.
2. **Coefficient matching:** Write q = Σ bᵢxⁱ with the bounded degree. Expand q' + f·q, match coefficients of each power of x against g. Solve top-down (highest degree first). Each coefficient bᵢ is determined by previously-computed higher-degree coefficients. If any step yields a contradiction, no solution exists.
3. **Verification:** After computing q, verify q' + f·q = g exactly.

**Concrete coefficient recurrence:**
At degree r: `(r+1)·b_{r+1} + Σ_{i+j=r} f_i·b_j = g_r`

Solving for b_r (the lowest-index unknown at each step):
`b_r = (g_r - (r+1)·b_{r+1} - Σ_{i=1}^{min(m,r)} f_i·b_{r-i}) / f_0`

When f_0 ≠ 0, this is a straightforward top-down solve. When f_0 = 0, check consistency (the numerator must also be 0, giving a free variable or contradiction).

**Step 1: Write failing tests**

```rust
// In src/risch.rs, add to the #[cfg(test)] module:

#[test]
fn test_risch_de_trivial() {
    // q' + 0·q = 2x → q = x² (just integration)
    let f = Polynomial::zero("x");
    let g = poly(&[0, 2], "x"); // 2x
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_some());
    let q = result.unwrap();
    assert_eq!(q, poly(&[0, 0, 1], "x")); // x²
}

#[test]
fn test_risch_de_exp_x() {
    // ∫e^x dx = e^x: q' + 1·q = 1 → q = 1
    // f = 1, g = 1
    let f = poly(&[1], "x"); // constant 1
    let g = poly(&[1], "x");
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_some());
    let q = result.unwrap();
    assert_eq!(q, poly(&[1], "x")); // q = 1
}

#[test]
fn test_risch_de_x_exp_neg_x_sq() {
    // ∫x·e^(-x²) dx: f = g' = -2x, p = x
    // q' - 2xq = x → q = -1/2
    let f = poly(&[0, -2], "x"); // -2x
    let g = poly(&[0, 1], "x");  // x
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_some());
    let q = result.unwrap();
    assert_eq!(q, Polynomial::constant(rat(-1, 2), "x")); // -1/2
}

#[test]
fn test_risch_de_exp_neg_x_sq_non_elementary() {
    // ∫e^(-x²) dx: f = -2x, p = 1
    // q' - 2xq = 1 → deg bound = 0 - 1 = -1 < 0 → no solution
    let f = poly(&[0, -2], "x"); // -2x
    let g = poly(&[1], "x");     // 1
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_none(), "∫e^(-x²)dx should be non-elementary");
}

#[test]
fn test_risch_de_exp_x_cubed_non_elementary() {
    // ∫e^(x³) dx: f = 3x², p = 1
    // deg bound = 0 - 2 = -2 < 0 → no solution
    let f = poly(&[0, 0, 3], "x"); // 3x²
    let g = poly(&[1], "x");
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_none(), "∫e^(x³)dx should be non-elementary");
}

#[test]
fn test_risch_de_x_sq_exp_neg_x_sq_non_elementary() {
    // ∫x²·e^(-x²) dx: f = -2x, p = x²
    // deg bound = 2 - 1 = 1, q = b₁x + b₀
    // Coefficient matching leads to contradiction
    let f = poly(&[0, -2], "x");
    let g = poly(&[0, 0, 1], "x"); // x²
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_none(), "∫x²·e^(-x²)dx should be non-elementary");
}

#[test]
fn test_risch_de_2x_exp_x_sq() {
    // ∫2x·e^(x²) dx = e^(x²): q' + 2xq = 2x → q = 1
    let f = poly(&[0, 2], "x"); // 2x
    let g = poly(&[0, 2], "x"); // 2x
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), poly(&[1], "x")); // q = 1
}

#[test]
fn test_risch_de_with_constant_f() {
    // q' + 3q = 6x + 3 → q = 2x - 1 (since q' = 2, 2 + 3(2x-1) = 2 + 6x - 3 = 6x - 1... hmm that's wrong)
    // Let me recalculate: q = 2x + 1/3. q' = 2. 2 + 3(2x + 1/3) = 2 + 6x + 1 = 6x + 3. ✓
    let f = poly(&[3], "x");
    let g = poly(&[3, 6], "x"); // 6x + 3
    let result = solve_risch_de_poly(&f, &g, "x");
    assert!(result.is_some());
    let q = result.unwrap();
    // q' + 3q should equal 6x + 3
    let qp = q.derivative();
    let check = &qp + &(&f * &q);
    assert_eq!(check, g);
}
```

**Step 2: Implement `solve_risch_de_poly`**

```rust
/// Solve the Risch differential equation q' + f·q = g
/// where f, g are polynomials in Q[x].
///
/// Returns Some(q) if a polynomial solution exists, None otherwise.
/// When None is returned, it is a PROOF that no polynomial solution exists
/// — for exponential integration, this means the integral is non-elementary.
pub fn solve_risch_de_poly(
    f: &Polynomial,
    g: &Polynomial,
    var: &str,
) -> Option<Polynomial> {
    // ...implementation using degree bound + coefficient matching...
}
```

**Step 3: Run tests, clippy, commit**

```
cargo fmt && cargo clippy --tests -- -D warnings && cargo test
git add src/risch.rs
git commit -m "Add Risch DE solver: q' + fq = g with non-elementary detection"
```

---

### Task 2: Exponential Pattern Detector

**Files:**
- Modify: `src/risch.rs`

Detect the pattern `r(x) · exp(g(x))` in a Node AST and extract `r` (coefficient) and `g` (exponent) as Polynomials.

**Function signature:**

```rust
/// Try to decompose a Node into r(x) · exp(g(x)) where r and g are polynomials.
/// Returns (r, g) if the pattern matches.
pub fn extract_exp_pattern(expr: &Node, var: &str) -> Option<(Polynomial, Polynomial)>
```

**Patterns to match:**
- `Function("exp", [arg])` → r = 1, g = arg-as-polynomial
- `Multiply(coeff, Function("exp", [arg]))` → r = coeff-as-polynomial, g = arg-as-polynomial
- `Multiply(Function("exp", [arg]), coeff)` → same, reversed order
- `Negate(Function("exp", [arg]))` → r = -1, g = arg-as-polynomial
- `Num(k) · exp(...)`, `Variable · exp(...)`, polynomial · exp(...)

For each: attempt `Polynomial::from_node(coeff, var)` and `Polynomial::from_node(arg, var)`. If either fails, return None.

**Tests:**

```rust
#[test]
fn test_extract_exp_simple() {
    // exp(-x^2) → r=1, g=-x²
    let expr = Node::Function("exp".to_string(),
        vec![Node::Negate(Box::new(Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::integer(2))),
        )))]);
    let (r, g) = extract_exp_pattern(&expr, "x").unwrap();
    assert_eq!(r, Polynomial::one("x"));
    // g should be -x²
}

#[test]
fn test_extract_exp_with_coeff() {
    // x * exp(-x^2) → r=x, g=-x²
    // Parse "x \\cdot \\exp(-x^2)" or construct Node directly
}

#[test]
fn test_extract_exp_no_match() {
    // sin(x) — not an exponential pattern
    let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
    assert!(extract_exp_pattern(&expr, "x").is_none());
}
```

**Commit:**
```
git add src/risch.rs
git commit -m "Add exponential pattern detector: extract r(x)·exp(g(x)) from Node AST"
```

---

### Task 3: `try_risch_exponential` — Full Integration Path

**Files:**
- Modify: `src/risch.rs`

The main entry point that ties everything together:

```rust
/// Result of Risch integration attempt.
pub enum RischResult {
    /// Found an elementary antiderivative.
    Elementary(Node),
    /// Proved that no elementary antiderivative exists.
    NonElementary(String),
}

/// Try to integrate expr using the Risch algorithm for exponential extensions.
///
/// Handles integrands of the form r(x)·exp(g(x)) where r, g are polynomials.
/// Returns:
/// - Some(Elementary(node)) if the integral is q(x)·exp(g(x))
/// - Some(NonElementary(reason)) if provably non-elementary
/// - None if this method doesn't apply (not an exp pattern)
pub fn try_risch_exponential(expr: &Node, var: &str) -> Option<RischResult> {
    // 1. Extract r(x) and g(x) from the pattern
    let (r, g) = extract_exp_pattern(expr, var)?;
    
    // 2. Compute g'(x)
    let g_prime = g.derivative();
    
    // 3. Solve Risch DE: q' + g'·q = r
    match solve_risch_de_poly(&g_prime, &r, var) {
        Some(q) => {
            // 4a. Build result: q(x) · exp(g(x))
            let q_node = q.to_node();
            let g_node = g.to_node();
            let exp_g = Node::Function("exp".to_string(), vec![g_node]);
            let result = if q == Polynomial::one(var) {
                exp_g
            } else {
                Node::Multiply(Box::new(q_node), Box::new(exp_g))
            };
            Some(RischResult::Elementary(result))
        }
        None => {
            // 4b. Non-elementary: construct explanation
            let reason = format!(
                "The integral of {} · exp({}) has no elementary antiderivative (Risch algorithm: \
                 the differential equation q' + ({}')·q = {} has no polynomial solution)",
                r, g, g, r
            );
            Some(RischResult::NonElementary(reason))
        }
    }
}
```

**Tests:**

```rust
#[test]
fn test_risch_exp_elementary() {
    // ∫e^x dx = e^x
    let expr = Node::Function("exp".to_string(), vec![Node::Variable("x".to_string())]);
    let result = try_risch_exponential(&expr, "x");
    assert!(matches!(result, Some(RischResult::Elementary(_))));
}

#[test]
fn test_risch_exp_non_elementary_gaussian() {
    // ∫e^(-x²) dx — non-elementary
    let expr = Node::Function("exp".to_string(),
        vec![Node::Negate(Box::new(Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::integer(2))),
        )))]);
    let result = try_risch_exponential(&expr, "x");
    assert!(matches!(result, Some(RischResult::NonElementary(_))));
}

#[test]
fn test_risch_exp_x_exp_neg_x_sq() {
    // ∫x·e^(-x²) dx = -1/2 · e^(-x²)
    // Build Node for x * exp(-x^2) and verify elementary result
}

#[test]
fn test_risch_exp_non_applicable() {
    // sin(x) — not an exp pattern, should return None
    let expr = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
    assert!(try_risch_exponential(&expr, "x").is_none());
}
```

**Commit:**
```
git add src/risch.rs
git commit -m "Add try_risch_exponential: integrate r(x)·exp(g(x)) or prove non-elementary"
```

---

### Task 4: Wire Into Integration Engine

**Files:**
- Modify: `src/integration.rs`
- Modify: `src/risch.rs` (make functions pub)

Add the Risch algorithm as a last-resort method in the `integrate()` function. When all heuristic methods fail, try `try_risch_exponential` before returning an error.

**Changes to `src/integration.rs`:**

1. Add `use crate::risch::{try_risch_exponential, RischResult};` at the top.

2. In the `Node::Function(name, args)` match arm (around line 294-324), before the final `Err(...)`, add:

```rust
// Last resort: try Risch algorithm for exponential patterns
if let Some(risch_result) = try_risch_exponential(&full_expr, var_name) {
    match risch_result {
        RischResult::Elementary(node) => return Ok(node),
        RischResult::NonElementary(reason) => return Err(format!("NON_ELEMENTARY: {}", reason)),
    }
}
```

3. Also add the Risch check in the `Node::Multiply` branch (around line 195) and the general fallback (line 326) for cases like `x * exp(-x^2)`:

```rust
// Before the final Err in Multiply:
if let Some(risch_result) = try_risch_exponential(expr, var_name) {
    match risch_result {
        RischResult::Elementary(node) => return Ok(node),
        RischResult::NonElementary(reason) => return Err(format!("NON_ELEMENTARY: {}", reason)),
    }
}
```

**Tests (in `tests/integration.rs` or inline):**

```rust
#[test]
fn test_integrate_exp_x() {
    // ∫e^x dx = e^x + C (already works, but verify it still does)
    let result = integrate_latex("\\exp(x)", "x").unwrap();
    assert!(result.contains("+ C"));
}

#[test]
fn test_integrate_x_exp_neg_x_sq() {
    // ∫x·e^(-x²) dx = -1/2 · e^(-x²) + C
    let result = integrate_latex("x \\cdot \\exp(-x^2)", "x");
    assert!(result.is_ok());
}

#[test]
fn test_integrate_exp_neg_x_sq_non_elementary() {
    // ∫e^(-x²) dx should report non-elementary
    let result = integrate_latex("\\exp(-x^2)", "x");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.starts_with("NON_ELEMENTARY:"), "Expected non-elementary, got: {}", err);
}

#[test]
fn test_integrate_exp_x_cubed_non_elementary() {
    // ∫e^(x³) dx — non-elementary
    let result = integrate_latex("\\exp(x^3)", "x");
    assert!(result.is_err());
    assert!(result.unwrap_err().starts_with("NON_ELEMENTARY:"));
}
```

**Commit:**
```
git add src/integration.rs src/risch.rs
git commit -m "Wire Risch exponential integration into engine with non-elementary detection"
```

---

### Task 5: MCP and CLI Reporting

**Files:**
- Modify: `src/bin/arithma-mcp.rs`
- Modify: `src/main.rs`

Surface non-elementary results cleanly to agents and human users.

**MCP changes (`src/bin/arithma-mcp.rs`):**

In `tool_integrate` (around line 503), handle the `NON_ELEMENTARY:` prefix:

```rust
fn tool_integrate(args: &Value) -> Result<String, String> {
    // ... existing code ...
    let result = match (has_lower, has_upper) {
        (Some(lower), Some(upper)) => definite_integral_latex(expr, var, lower, upper),
        _ => integrate_latex(expr, var),
    };
    
    match result {
        Ok(r) => {
            let env = env_from_args(args)?;
            parse_and_simplify_with_env(&r, &env)
        }
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            // Return the non-elementary explanation as a success message
            // (it IS a successful analysis — the agent got an answer)
            Ok(e.replacen("NON_ELEMENTARY: ", "", 1))
        }
        Err(e) => Err(e),
    }
}
```

**CLI changes (`src/main.rs`):**

In `cmd_integrate` (around line 120):

```rust
fn cmd_integrate(args: &[String]) {
    // ... existing code ...
    match arithma::integration::integrate_latex(expr, var) {
        Ok(result) => println!("{}", result),
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            println!("{}", e.replacen("NON_ELEMENTARY: ", "", 1));
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Tests:**

```rust
// Integration test for CLI (in tests/ directory)
#[test]
fn test_mcp_integrate_non_elementary() {
    // Verify the MCP tool_integrate function handles non-elementary correctly
    // (This would be tested via the full MCP flow or unit test of tool_integrate)
}
```

Also add a test in `tests/integration.rs`:

```rust
#[test]
fn test_non_elementary_message_format() {
    let result = integrate_latex("\\exp(-x^2)", "x");
    let err = result.unwrap_err();
    assert!(err.contains("NON_ELEMENTARY"));
    assert!(err.contains("no elementary antiderivative"));
}
```

**Commit:**
```
git add src/bin/arithma-mcp.rs src/main.rs
git commit -m "Surface non-elementary results in MCP and CLI"
```

---

## Session Summary

After this session, arithma will be able to:

1. **Integrate** r(x)·exp(g(x)) when an elementary antiderivative exists (via Risch DE)
2. **Prove non-elementary** when no elementary antiderivative exists (via Risch DE degree bound + coefficient matching)
3. **Report clearly** to agents ("no elementary antiderivative exists") and humans

**Key integrals that work after this session:**
- ∫e^x dx = e^x ✓ (already worked, still works)
- ∫x·e^(-x²) dx = -½·e^(-x²) ✓ (NEW — Risch DE)
- ∫2x·e^(x²) dx = e^(x²) ✓ (NEW — Risch DE)
- ∫e^(-x²) dx → "non-elementary" ✓ (NEW — the killer feature)
- ∫e^(x³) dx → "non-elementary" ✓ (NEW)
- ∫x²·e^(-x²) dx → "non-elementary" ✓ (NEW)

**Estimated new tests:** ~20-25
