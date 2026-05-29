# Symbolic-Center Taylor Expansion

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow Taylor series expansion around a symbolic center (e.g., expand `3/(1+2x)` in `x` around `a`, giving coefficients as symbolic expressions in `a`).

**Architecture:** Add `taylor_series_symbolic(expr, var, center_node, order)` that uses `substitute_variable + simplify` instead of `evaluate_exact` to compute coefficients. A new `build_taylor_node_symbolic` constructs the output with `Node` coefficients. The `taylor_series_latex` function detects whether the center parses as a number or a symbolic expression and dispatches accordingly. MCP tool's `center` parameter becomes a string (accepting both `"0"` and `"a"`). CLI center arg attempts float parse first, falls back to LaTeX parse.

**Tech Stack:** Rust, existing `substitute_variable()` from `src/substitute.rs`, existing `differentiate()` and `Simplifiable` trait.

**Key files:**
- `src/series.rs:19-73` — `taylor_series`, `taylor_series_latex`, `build_taylor_node`
- `src/substitute.rs:61` — `substitute_variable(node, var_name, value)`
- `src/bin/arithma-mcp.rs:294-321,628-636` — MCP tool schema and handler
- `src/main.rs:336-360` — CLI handler
- `src/lib.rs:80` — public exports

---

### Task 1: Failing tests for symbolic-center Taylor

**Files:**
- Modify: `src/series.rs` (add tests at end of `mod tests`)

**Step 1: Write failing tests**

Add to `mod tests` in `src/series.rs`:

```rust
#[test]
fn test_taylor_symbolic_center_linear() {
    // Taylor of x^2 around x=a, order 2: a^2 + 2a(x-a) + (x-a)^2
    let expr = Node::Power(
        Box::new(Node::Variable("x".to_string())),
        Box::new(Node::Num(ExactNum::integer(2))),
    );
    let center = Node::Variable("a".to_string());
    let result = taylor_series_symbolic(&expr, "x", &center, 2).unwrap();
    // Evaluate at x=5, a=2: should equal 25 (since it's exact for polynomials)
    let mut env = Environment::new();
    env.set("x", 5.0);
    env.set("a", 2.0);
    let val = Evaluator::evaluate(&result, &env).unwrap();
    assert!((val - 25.0).abs() < 1e-10, "x^2 Taylor around a, at x=5 a=2: got {}", val);
}

#[test]
fn test_taylor_symbolic_center_rational() {
    // Taylor of 3/(1+2x) around x=a, order 2
    // f(a) = 3/(1+2a), f'(a) = -6/(1+2a)^2, f''(a) = 24/(1+2a)^3
    // T_2 = 3/(1+2a) - 6/(1+2a)^2 * (x-a) + 12/(1+2a)^3 * (x-a)^2
    let env = Environment::new();
    let expr = arithma::parse_latex("\\frac{3}{1+2x}", &env).unwrap();
    let center = Node::Variable("a".to_string());
    let result = taylor_series_symbolic(&expr, "x", &center, 2).unwrap();
    // Evaluate near center: at x=0.6, a=0.5 → f(0.6) = 3/2.2 ≈ 1.3636
    let mut test_env = Environment::new();
    test_env.set("x", 0.6);
    test_env.set("a", 0.5);
    let approx = Evaluator::evaluate(&result, &test_env).unwrap();
    let exact = 3.0 / (1.0 + 2.0 * 0.6);
    assert!((approx - exact).abs() < 0.01, "Expected ~{}, got {}", exact, approx);
}

#[test]
fn test_taylor_symbolic_center_latex_interface() {
    // Test through the LaTeX interface with symbolic center
    let result = taylor_series_latex_symbolic("x^2", "x", "a", 2).unwrap();
    assert!(!result.is_empty(), "Should produce output");
    // Should contain 'a' as a symbol in the coefficients
    assert!(result.contains('a'), "Coefficients should contain parameter a: {}", result);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib -- tests::test_taylor_symbolic 2>&1`
Expected: Compilation error (functions don't exist yet)

**Step 3: Commit**

```bash
git add src/series.rs
git commit -m "test: failing tests for symbolic-center Taylor expansion"
```

---

### Task 2: Implement `taylor_series_symbolic`

**Files:**
- Modify: `src/series.rs`

**Step 1: Add the symbolic Taylor function**

Add after `taylor_series_latex` (after line 73) in `src/series.rs`:

```rust
/// Compute Taylor series with a symbolic center (a Node expression).
/// Coefficients are symbolic expressions obtained by substituting
/// the expansion variable with the center and simplifying.
pub fn taylor_series_symbolic(
    expr: &Node,
    var: &str,
    center: &Node,
    order: usize,
) -> Result<Node, String> {
    use crate::substitute::substitute_variable;

    let env = Environment::new();
    let mut current = expr.simplify(&env).unwrap_or_else(|_| expr.clone());
    let mut coeffs: Vec<Node> = Vec::with_capacity(order + 1);

    for k in 0..=order {
        // Evaluate the k-th derivative at the center by substitution
        let substituted = substitute_variable(&current, var, center)?;
        let value = substituted.simplify(&env).unwrap_or(substituted);
        let fact = factorial_exact(k);
        let coeff = if fact.is_one() {
            value
        } else {
            Node::Divide(Box::new(value), Box::new(Node::Num(fact)))
                .simplify(&env)
                .unwrap_or_else(|_| Node::Num(ExactNum::zero()))
        };
        coeffs.push(coeff);

        if k < order {
            current = differentiate(&current, var)?;
            current = current.simplify(&env).unwrap_or_else(|_| current.clone());
        }
    }

    build_taylor_node_symbolic(&coeffs, var, center)
}

/// Build a Node for the symbolic Taylor polynomial.
/// Terms are: coeff_k * (var - center)^k
fn build_taylor_node_symbolic(
    coeffs: &[Node],
    var: &str,
    center: &Node,
) -> Result<Node, String> {
    let shifted = Node::Subtract(
        Box::new(Node::Variable(var.to_string())),
        Box::new(center.clone()),
    );

    let env = Environment::new();
    let mut terms: Vec<Node> = Vec::new();

    for (k, coeff) in coeffs.iter().enumerate() {
        // Skip zero coefficients
        if matches!(coeff, Node::Num(n) if n.is_zero()) {
            continue;
        }

        let term = if k == 0 {
            coeff.clone()
        } else {
            let power_node = if k == 1 {
                shifted.clone()
            } else {
                Node::Power(
                    Box::new(shifted.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )
            };

            if matches!(coeff, Node::Num(n) if n.is_one()) {
                power_node
            } else {
                Node::Multiply(Box::new(coeff.clone()), Box::new(power_node))
                    .simplify(&env)
                    .unwrap_or_else(|_| {
                        Node::Multiply(Box::new(coeff.clone()), Box::new(power_node.clone()))
                    })
            }
        };

        terms.push(term);
    }

    if terms.is_empty() {
        return Ok(Node::Num(ExactNum::zero()));
    }

    let mut result = terms.remove(0);
    for term in terms {
        result = Node::Add(Box::new(result), Box::new(term));
    }

    let simplified = result.simplify(&env).unwrap_or(result);
    Ok(simplified)
}

/// Taylor series from LaTeX input with a symbolic center.
pub fn taylor_series_latex_symbolic(
    latex_expr: &str,
    var: &str,
    center_latex: &str,
    order: usize,
) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(latex_expr);
    let tokens = tokenizer.tokenize();
    let expr = build_expression_tree(tokens)?;

    let mut center_tokenizer = Tokenizer::new(center_latex);
    let center_tokens = center_tokenizer.tokenize();
    let center = build_expression_tree(center_tokens)?;

    let result = taylor_series_symbolic(&expr, var, &center, order)?;
    let env = Environment::new();
    let simplified = result.simplify(&env).unwrap_or(result);
    Ok(format!("{}", simplified))
}
```

**Step 2: Export new functions from `src/lib.rs`**

Update the exports line (line 80):
```rust
pub use crate::series::{taylor_series, taylor_series_latex, taylor_series_symbolic, taylor_series_latex_symbolic};
```

**Step 3: Run tests**

Run: `cargo test --lib -- tests::test_taylor_symbolic 2>&1`
Expected: All 3 symbolic Taylor tests pass.

Run: `cargo test --lib -- series::tests 2>&1`
Expected: All series tests pass (old + new, no regressions).

**Step 4: Run full suite + clippy**

Run: `cargo test 2>&1 | tail -5`
Run: `cargo clippy --tests -- -D warnings 2>&1`

**Step 5: Commit**

```bash
git add src/series.rs src/lib.rs
git commit -m "feat: symbolic-center Taylor expansion"
```

---

### Task 3: Update MCP tool for symbolic centers

**Files:**
- Modify: `src/bin/arithma-mcp.rs:294-321` (schema) and `628-636` (handler)

**Step 1: Write a test via the CLI first (Task 4), but update MCP now**

Update the `center` property in the tool schema (around line 308) — change from `number` type to accept strings:

```json
"center": {
    "description": "Center point of the expansion. Use a number for numeric centers (0 for Maclaurin) or a LaTeX expression for symbolic centers (e.g. \"a\" or \"\\\\alpha\").",
    "default": 0
},
```

Remove the `"type": "number"` constraint so it accepts both numbers and strings.

**Step 2: Update the handler**

Replace `tool_taylor_series` (around line 628):

```rust
fn tool_taylor_series(args: &Value) -> Result<String, String> {
    let expr = get_str(args, "expr").ok_or("Missing required parameter: expr")?;
    let var = get_var(args, "x");
    let order = args.get("order").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let env = env_from_args(args)?;

    // Check if center is numeric or symbolic
    let center_val = args.get("center");
    let is_numeric = center_val
        .map(|v| v.is_number() || v.is_null())
        .unwrap_or(true);

    if is_numeric {
        let center = center_val.and_then(|v| v.as_f64()).unwrap_or(0.0);
        let result = taylor_series_latex(expr, &var, center, order)?;
        parse_and_simplify_with_env(&result, &env)
    } else {
        let center_str = center_val
            .and_then(|v| v.as_str())
            .ok_or("center must be a number or LaTeX expression")?;
        let center_str = &normalize_var(center_str);
        let result = taylor_series_latex_symbolic(expr, &var, center_str, order)?;
        parse_and_simplify_with_env(&result, &env)
    }
}
```

Add the import at the top of the file where other series imports are:
```rust
use arithma::series::{taylor_series_latex, taylor_series_latex_symbolic};
```
(Or update existing import line to include `taylor_series_latex_symbolic`.)

**Step 3: Run clippy**

Run: `cargo clippy --tests -- -D warnings 2>&1`

**Step 4: Commit**

```bash
git add src/bin/arithma-mcp.rs
git commit -m "feat: MCP taylor_series accepts symbolic centers"
```

---

### Task 4: Update CLI for symbolic centers

**Files:**
- Modify: `src/main.rs:336-360`

**Step 1: Update `cmd_taylor`**

Replace the center parsing logic (around line 346-349). Try parsing as float first; if that fails, treat as LaTeX expression:

```rust
fn cmd_taylor(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma taylor <expr> [var] [center] [order]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    let order = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let center_str = args.get(2).map(|s| s.as_str()).unwrap_or("0");

    // Try numeric center first, fall back to symbolic
    if let Ok(center_f64) = center_str.parse::<f64>() {
        match arithma::series::taylor_series_latex(expr, &var, center_f64, order) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let center_normalized = normalize_var(center_str);
        match arithma::series::taylor_series_latex_symbolic(expr, &var, &center_normalized, order) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
```

**Step 2: Test via CLI**

Run: `cargo run -- taylor "x^2" x a 2 2>&1`
Expected: Output containing `a` as a parameter in the coefficients.

Run: `cargo run -- taylor "\\frac{3}{1+2x}" x a 2 2>&1`
Expected: Output with symbolic coefficients in `a`.

Run: `cargo run -- taylor "\\sin(x)" x 0 5 2>&1`
Expected: Same as before (numeric center still works).

**Step 3: Run full suite + clippy**

Run: `cargo fmt -- --check && cargo clippy --tests -- -D warnings && cargo test`

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: CLI taylor command accepts symbolic centers"
```

---

### Task 5: Integration tests and docs

**Files:**
- Modify: `tests/simplify.rs` or a new `tests/taylor.rs`
- Modify: `KNUTH-PLAN.md`

**Step 1: Add E2E integration tests**

Add to `tests/simplify.rs` (at end of `mod test_simplify`):

```rust
#[test]
fn test_e2e_taylor_symbolic_center() {
    // Taylor of 3/(1+2x) around x=α, order 1
    // f(α) = 3/(1+2α), f'(α) = -6/(1+2α)^2
    // T₁ = 3/(1+2α) - 6/(1+2α)² · (x-α)
    let result = arithma::series::taylor_series_latex_symbolic(
        "\\frac{3}{1+2x}", "x", "\\alpha", 1
    ).unwrap();
    assert!(result.contains("\\alpha"), "Should contain α: {}", result);
    // Parse and evaluate to verify correctness
    let env = Environment::new();
    let expr = arithma::parse_latex(&result, &env).unwrap();
    let mut test_env = Environment::new();
    test_env.set("x", 0.6);
    test_env.set("α", 0.5);
    let val = Evaluator::evaluate(&expr, &test_env).unwrap();
    let exact = 3.0 / (1.0 + 2.0 * 0.6);
    assert!((val - exact).abs() < 0.1, "Taylor approx should be close: {} vs {}", val, exact);
}

#[test]
fn test_e2e_taylor_numeric_center_unchanged() {
    // Existing numeric center should still work
    let result = arithma::series::taylor_series_latex("\\sin(x)", "x", 0.0, 3).unwrap();
    assert!(!result.is_empty());
    assert!(!result.contains("NaN"));
}
```

**Step 2: Update KNUTH-PLAN.md**

Add to the Current State section after the simplifier description:
- **Symbolic-center Taylor expansion:** `taylor_series_symbolic` expands `f(x)` around `x = a` where `a` is a symbolic expression, producing coefficients as exact symbolic expressions in `a`. MCP and CLI accept symbolic centers alongside numeric ones.

Update test count.

**Step 3: Run final CI check**

```bash
cargo fmt -- --check && cargo clippy --tests -- -D warnings && cargo test
```

**Step 4: Commit**

```bash
git add tests/simplify.rs KNUTH-PLAN.md
git commit -m "test: e2e tests for symbolic Taylor, doc update"
```
