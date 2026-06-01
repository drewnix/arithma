# Radical Simplification and Parser Fixes

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix two root causes from dogfooding: (1) parser leaks LaTeX operators into variable namespace, (2) radical simplifier has no square-factor extraction, causing float fallback that violates the exactness invariant.

**Architecture:** Fix 1 is a tokenizer change — add reserved LaTeX operators to `is_variable_token` exclusion list and strip spacing commands. Fix 2 adds `extract_square_factors(n)` for integers that returns `(outside, inside)` such that `√n = outside · √inside`, then hooks this into both the `Node::Sqrt` and `Node::Function("sqrt")` simplification paths. The key insight: `ExactNum::sqrt()` currently falls back to `Float(self.to_f64().sqrt())` for non-perfect squares — this is where the exactness invariant breaks. Instead of touching ExactNum, we fix the simplifier to extract square factors before reaching that path.

**Tech Stack:** Rust, existing tokenizer/simplifier, trial division for integer factorization.

---

### Task 1: Parser — Reserve LaTeX Operators

**Files:**
- Modify: `src/tokenizer.rs`

**Step 1: Write the failing tests**

Add to the existing test section at the bottom of `src/tokenizer.rs`:

```rust
#[test]
fn test_latex_operators_not_variables() {
    // \int should not become a variable token
    assert!(!is_variable_token("int"));
    assert!(!is_variable_token("prod"));
    assert!(!is_variable_token("oint"));
    // These should still be variables
    assert!(is_variable_token("x"));
    assert!(is_variable_token("a"));
}

#[test]
fn test_latex_spacing_stripped() {
    // \, and \; should be silently ignored
    let mut t1 = Tokenizer::new("x \\, y");
    let tokens1 = t1.tokenize();
    // Should have x * y (with implicit multiplication), no empty token
    assert!(!tokens1.contains(&String::new()), "Empty token from \\,: {:?}", tokens1);
    assert!(tokens1.contains(&"x".to_string()));
    assert!(tokens1.contains(&"y".to_string()));

    let mut t2 = Tokenizer::new("x \\; y");
    let tokens2 = t2.tokenize();
    assert!(!tokens2.contains(&String::new()), "Empty token from \\;: {:?}", tokens2);

    let mut t3 = Tokenizer::new("x \\quad y");
    let tokens3 = t3.tokenize();
    assert!(!tokens3.contains(&String::new()), "Empty token from \\quad: {:?}", tokens3);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib test_latex_operators_not_variables test_latex_spacing_stripped`
Expected: At least the operator test fails (`int` is currently treated as a variable)

**Step 3: Implement fixes**

In `src/tokenizer.rs`:

**3a.** In `is_variable_token` (line 6), add LaTeX operator names to the exclusion list:

```rust
fn is_variable_token(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| c.is_alphabetic())
        && FUNCTION_REGISTRY.get(token).is_none()
        && token != "NEG"
        && token != "sum"
        && !matches!(token, "int" | "prod" | "oint" | "iint" | "iiint" | "lim" | "nabla")
}
```

**3b.** In `tokenize_latex_commands` (line 209), in the `match stripped_token.as_str()` block (line 311), add a case for spacing commands BEFORE the wildcard `_` arm:

```rust
            // LaTeX spacing commands — silently strip
            "," | ";" | "!" | ":" | "quad" | "qquad" | "enspace" | "thinspace" => {
                current_token.clear();
            }
```

**Step 4: Run tests**

Run: `cargo test --lib test_latex_operators test_latex_spacing`
Expected: PASS

**Step 5: Run full suite + clippy**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`
Expected: All clean

**Step 6: Commit**

```
git commit -m "fix: reserve LaTeX operators in tokenizer, strip spacing commands"
```

---

### Task 2: Integer Square-Factor Extraction

**Files:**
- Modify: `src/simplify.rs`

This is the mathematical core. For a positive integer n, find the largest perfect square factor and return `(outside, inside)` such that `√n = outside · √inside` with `inside` square-free.

**Step 1: Write the failing test**

Add to the test section in `src/simplify.rs` (or create a new test in `tests/simplify.rs` — check which location has the existing simplify tests):

```rust
#[test]
fn test_extract_square_factors() {
    // √12 = 2√3
    assert_eq!(extract_square_factors(12), (2, 3));
    // √8 = 2√2
    assert_eq!(extract_square_factors(8), (2, 2));
    // √18 = 3√2
    assert_eq!(extract_square_factors(18), (3, 2));
    // √7 = 1·√7 (prime, no square factors)
    assert_eq!(extract_square_factors(7), (1, 7));
    // √4 = 2·√1
    assert_eq!(extract_square_factors(4), (2, 1));
    // √1 = 1·√1
    assert_eq!(extract_square_factors(1), (1, 1));
    // √72 = 6√2
    assert_eq!(extract_square_factors(72), (6, 2));
    // √100 = 10·√1
    assert_eq!(extract_square_factors(100), (10, 1));
    // √50 = 5√2
    assert_eq!(extract_square_factors(50), (5, 2));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_extract_square_factors`
Expected: FAIL — function not found

**Step 3: Implement `extract_square_factors`**

Add to `src/simplify.rs`:

```rust
/// For a positive integer n, find (outside, inside) such that
/// √n = outside · √inside, with inside square-free.
/// Uses trial division up to √n.
fn extract_square_factors(mut n: u64) -> (u64, u64) {
    if n == 0 {
        return (0, 0);
    }
    let mut outside = 1u64;
    let mut d = 2u64;
    while d * d <= n {
        while n % (d * d) == 0 {
            outside *= d;
            n /= d * d;
        }
        // If d still divides n once, move past it
        if n % d == 0 {
            // d appears with odd exponent — one factor stays inside
            d += 1;
        } else {
            d += 1;
        }
    }
    (outside, n)
}
```

Actually, the loop logic can be simpler — just repeatedly check if d² divides n:

```rust
fn extract_square_factors(mut n: u64) -> (u64, u64) {
    if n == 0 {
        return (0, 0);
    }
    let mut outside = 1u64;
    let mut d = 2u64;
    while d * d <= n {
        while n % (d * d) == 0 {
            outside *= d;
            n /= d * d;
        }
        d += 1;
    }
    (outside, n)
}
```

This is correct because: for each prime p, we extract as many p² factors as possible. What remains is at most one factor of p (square-free remainder). The loop runs up to √n which is fast for any integer that fits in u64.

**Step 4: Run test**

Run: `cargo test test_extract_square_factors`
Expected: PASS

**Step 5: Commit**

```
git commit -m "feat: extract_square_factors for radical simplification"
```

---

### Task 3: Hook Square-Factor Extraction Into Simplifier

**Files:**
- Modify: `src/simplify.rs`

This is the critical wiring. When the simplifier encounters `√n` where n is a positive integer that's not a perfect square, instead of falling through to float, we extract square factors and return `outside · √inside`.

**Step 1: Write the failing tests**

In `tests/simplify.rs`:

```rust
#[test]
fn test_simplify_sqrt_12() {
    let env = Environment::new();
    let expr = parse_latex("\\sqrt{12}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    // Should be 2√3, not 3.464...
    assert!(s.contains("\\sqrt"), "Should preserve symbolic sqrt: {}", s);
    assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
    assert!(s.contains('2') && s.contains('3'), "Should be 2√3: {}", s);
}

#[test]
fn test_simplify_sqrt_8() {
    let env = Environment::new();
    let expr = parse_latex("\\sqrt{8}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
}

#[test]
fn test_simplify_sqrt_perfect_square() {
    let env = Environment::new();
    // √4 = 2 (no radical remaining)
    let expr = parse_latex("\\sqrt{4}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert_eq!(s, "2", "√4 should simplify to 2: {}", s);
}

#[test]
fn test_simplify_sqrt_prime() {
    let env = Environment::new();
    // √7 stays as √7, not float
    let expr = parse_latex("\\sqrt{7}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert!(s.contains("\\sqrt"), "√7 should stay symbolic: {}", s);
    assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
}

#[test]
fn test_simplify_sqrt_fraction() {
    let env = Environment::new();
    // √(1/4) = 1/2
    let expr = parse_latex("\\sqrt{\\frac{1}{4}}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert!(!s.contains("sqrt"), "√(1/4) should simplify to 1/2: {}", s);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test simplify test_simplify_sqrt`
Expected: FAIL — `√12` returns a float

**Step 3: Modify the simplifier**

There are TWO sqrt simplification paths that need fixing:

**Path A: `Node::Sqrt(operand)` at line ~541 of simplify.rs**

Replace the current block:
```rust
Node::Sqrt(operand) => {
    let simplified = operand.simplify(env)?;
    if let Node::Num(ref n) = simplified {
        let s = n.sqrt();
        if matches!(s, ExactNum::Rational(_)) {
            return Ok(Node::Num(s));
        }
        // Non-perfect-square: preserve symbolic sqrt
    }
    // sqrt(x²) → x when x positive, |x| otherwise
    ...
```

With:
```rust
Node::Sqrt(operand) => {
    let simplified = operand.simplify(env)?;
    if let Node::Num(ref n) = simplified {
        let s = n.sqrt();
        if matches!(s, ExactNum::Rational(_)) {
            return Ok(Node::Num(s));
        }
        // Non-perfect-square integer: extract square factors
        if let Some(val) = n.to_i64() {
            if val > 0 {
                let (outside, inside) = extract_square_factors(val as u64);
                if inside == 1 {
                    return Ok(Node::Num(ExactNum::integer(outside as i64)));
                }
                let sqrt_inside = Node::Sqrt(Box::new(Node::Num(ExactNum::integer(inside as i64))));
                if outside == 1 {
                    return Ok(sqrt_inside);
                }
                return Ok(Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(outside as i64))),
                    Box::new(sqrt_inside),
                ));
            }
        }
        // Non-perfect-square rational: try numerator and denominator separately
        if let ExactNum::Rational(ref r) = n {
            if !r.is_negative() {
                if let (Some(num), Some(den)) = (r.numer().to_i64(), r.denom().to_i64()) {
                    if num > 0 && den > 0 {
                        let (num_out, num_in) = extract_square_factors(num as u64);
                        let (den_out, den_in) = extract_square_factors(den as u64);
                        if num_in == 1 && den_in == 1 {
                            // Perfect square rational
                            return Ok(Node::Num(ExactNum::rational(num_out as i64, den_out as i64)));
                        }
                        // Build (num_out/den_out) · √(num_in/den_in)
                        let sqrt_part = Node::Sqrt(Box::new(Node::Num(ExactNum::rational(num_in as i64, den_in as i64))));
                        if num_out == 1 && den_out == 1 {
                            return Ok(sqrt_part);
                        }
                        let coeff = Node::Num(ExactNum::rational(num_out as i64, den_out as i64));
                        return Ok(Node::Multiply(Box::new(coeff), Box::new(sqrt_part)));
                    }
                }
            }
        }
        // Fallback: preserve symbolic form (do NOT evaluate to float)
        return Ok(Node::Sqrt(Box::new(simplified)));
    }
    // sqrt(x²) → x when x positive, |x| otherwise
    if let Node::Power(ref base, ref exp) = simplified {
        if let Node::Num(ref e) = **exp {
            if e == &ExactNum::two() {
                if let Node::Variable(ref v) = **base {
                    if env.assumptions().is_nonneg(v) {
                        return Ok(*base.clone());
                    }
                }
                return Ok(Node::Abs(base.clone()));
            }
        }
    }
    Ok(Node::Sqrt(Box::new(simplified)))
}
```

**Path B: `Node::Function("sqrt", ...)` at line ~616 of simplify.rs**

The `"sqrt"` arm currently only handles `sqrt(x²) → x/|x|`. Add the same numeric square-factor extraction BEFORE the x² check:

```rust
"sqrt" => {
    // Numeric square-factor extraction
    if let Node::Num(ref n) = arg {
        let s = n.sqrt();
        if matches!(s, ExactNum::Rational(_)) {
            return Ok(Node::Num(s));
        }
        if let Some(val) = n.to_i64() {
            if val > 0 {
                let (outside, inside) = extract_square_factors(val as u64);
                if inside == 1 {
                    return Ok(Node::Num(ExactNum::integer(outside as i64)));
                }
                let sqrt_inside = Node::Function("sqrt".to_string(), vec![Node::Num(ExactNum::integer(inside as i64))]);
                if outside == 1 {
                    return Ok(sqrt_inside);
                }
                return Ok(Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(outside as i64))),
                    Box::new(sqrt_inside),
                ));
            }
        }
        if let ExactNum::Rational(ref r) = n {
            if !r.is_negative() {
                if let (Some(num), Some(den)) = (r.numer().to_i64(), r.denom().to_i64()) {
                    if num > 0 && den > 0 {
                        let (num_out, num_in) = extract_square_factors(num as u64);
                        let (den_out, den_in) = extract_square_factors(den as u64);
                        if num_in == 1 && den_in == 1 {
                            return Ok(Node::Num(ExactNum::rational(num_out as i64, den_out as i64)));
                        }
                        let sqrt_part = Node::Function("sqrt".to_string(), vec![Node::Num(ExactNum::rational(num_in as i64, den_in as i64))]);
                        if num_out == 1 && den_out == 1 {
                            return Ok(sqrt_part);
                        }
                        let coeff = Node::Num(ExactNum::rational(num_out as i64, den_out as i64));
                        return Ok(Node::Multiply(Box::new(coeff), Box::new(sqrt_part)));
                    }
                }
            }
        }
        // Don't fall through to float — return symbolic form
        return Ok(Node::Function("sqrt".to_string(), vec![arg.clone()]));
    }
    // sqrt(x²) → x when x nonneg, |x| otherwise
    if let Node::Power(base, exp) = arg {
        ...existing code...
    }
}
```

**Important:** Both paths must use the SAME node type for their sqrt output as they received. Path A (Node::Sqrt) returns Node::Sqrt. Path B (Node::Function("sqrt")) returns Node::Function("sqrt"). This preserves display consistency.

**Step 4: Run tests**

Run: `cargo test --test simplify test_simplify_sqrt`
Expected: All PASS

**Step 5: Run full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`
Expected: All clean. **Watch for regressions** — any test that previously expected a float value from √n will now get a symbolic form. This is the correct behavior, but existing tests may need updating.

**Step 6: Commit**

```
git commit -m "feat: radical square-factor extraction — √12 → 2√3, no float fallback"
```

---

### Task 4: Fix ExactNum::sqrt to Not Fall Back to Float

**Files:**
- Modify: `src/exact.rs`

The simplifier changes in Task 3 handle the cases where the simplifier is the entry point. But `ExactNum::sqrt()` is called directly in a few places. Currently line 137 says `ExactNum::Float(self.to_f64().sqrt())` — this is the root of the float contamination. However, `ExactNum` doesn't have a way to represent "symbolic sqrt" — it's either Rational or Float. So we can't fix ExactNum itself to return a symbolic form.

**The correct fix:** leave `ExactNum::sqrt()` as-is for now (it's used for numeric evaluation in `definite_integral`), but ensure the **simplifier paths** (Task 3) never call it on non-perfect-square values. Instead they go through `extract_square_factors`. The simplifier changes in Task 3 already do this — they check `n.sqrt()` for perfect squares and handle the non-perfect-square case via `extract_square_factors` before any float fallback.

**Verification:** grep for all call sites of `.sqrt()` on ExactNum/Node to make sure none of them can produce float in the simplification path.

```
grep -rn "\.sqrt()" src/simplify.rs src/exact.rs src/integration.rs
```

If any simplifier paths still call `n.sqrt()` and use the float result, they need the same treatment.

**This task may be just verification — no code changes if Task 3 covered all paths.**

Run: `cargo test`
Commit only if changes were needed.

---

### Task 5: Regression Check and Edge Cases

**Files:**
- Modify: `tests/simplify.rs`

**Step 1: Add edge case tests**

```rust
#[test]
fn test_simplify_sqrt_large_square_factor() {
    let env = Environment::new();
    // √72 = 6√2
    let expr = parse_latex("\\sqrt{72}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert!(!s.contains('.'), "Should NOT be float: {}", s);
    assert!(s.contains("\\sqrt"), "Should have radical: {}", s);
}

#[test]
fn test_simplify_sqrt_50() {
    let env = Environment::new();
    // √50 = 5√2
    let expr = parse_latex("\\sqrt{50}", &env).unwrap();
    let result = arithma::simplify(&expr, &env).unwrap();
    let s = format!("{}", result);
    assert!(!s.contains('.'), "Should NOT be float: {}", s);
}
```

**Step 2: Run all tests**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test`

**Step 3: Commit**

```
git commit -m "test: edge cases for radical simplification"
```

---

## Regression Risks

1. **Definite integrals.** `definite_integral` evaluates Node expressions at numeric points. If the simplifier now returns `2·√3` instead of `3.464...`, the evaluation path needs to handle Multiply(Num, Sqrt(Num)) → f64 correctly. The `Node::eval` or `ExactNum::to_f64()` should still work since `Node::Sqrt(Num(3))` → `Num(Float(1.732...))` at evaluation time. But verify with the biquadratic numerical test.

2. **Existing tests expecting float output.** Any test that asserts `√n ≈ some_float` will now get a symbolic form. These tests should be UPDATED to expect the symbolic form — the old float behavior was the bug.

3. **Parser tests.** The `\int` reservation means `\int x dx` will now tokenize differently. If any test expects `int` as a variable, it needs updating. This is unlikely but check.

## What This Does NOT Handle

- `√(4a²) → 2|a|` — requires symbolic radicand factoring, not just numeric. This is important but significantly harder (need to factor symbolic products, detect even exponents). Future work.
- `√8 + √2 → 3√2` — requires like-radical-term combination in the Add/Subtract simplifier. Future work (depends on this task's output).
- Cube root simplification — similar pattern but for ∛ instead of √.
