# Sprint 8: Correctness Hardening & Modular Crate Split

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix three correctness bugs in the simplifier and verify tool, then split Arithma into a Cargo workspace with separate `arithma-core`, `arithma-cli`, and `arithma-mcp` crates.

**Architecture:** Tasks 1–3 are independent bug fixes in `src/simplify.rs` and `src/verify.rs`. Task 4 restructures the repo into a Cargo workspace without changing any behavior — the lib stays at the root as `arithma` (the core), and the two binaries become workspace members in `crates/cli/` and `crates/mcp/`. All existing tests must continue to pass after each task.

**Tech Stack:** Rust, Cargo workspaces, BigRational exact arithmetic.

**CI discipline:** Run `cargo fmt`, `cargo clippy --tests -- -D warnings`, and `cargo test` before every commit.

---

## Task 1: Fix `sin(2)` evaluating to float during simplify (GitHub #42)

**Problem:** When `simplify` encounters `sin(2)`, `try_exact_function_value` doesn't match (2 is not a π-multiple), so control falls through to the generic `all_numeric` branch at `simplify.rs:1350-1367`, which calls the f64 function registry and returns `0.909...`. This violates the exactness principle — `sin(2)` has no closed form and should stay symbolic.

**Files:**
- Modify: `src/simplify.rs:1350-1367`
- Test: `tests/simplify.rs`

**Step 1: Write failing tests**

Add to `tests/simplify.rs` inside `mod test_simplify`:

```rust
#[test]
fn test_sin_integer_stays_symbolic() {
    let env = Environment::new();
    let expr = arithma::parse_latex("\\sin{2}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert!(
        matches!(&result, Node::Function(name, _) if name == "sin"),
        "sin(2) should stay symbolic, got: {result}"
    );
}

#[test]
fn test_cos_integer_stays_symbolic() {
    let env = Environment::new();
    let expr = arithma::parse_latex("\\cos{3}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert!(
        matches!(&result, Node::Function(name, _) if name == "cos"),
        "cos(3) should stay symbolic, got: {result}"
    );
}

#[test]
fn test_tan_integer_stays_symbolic() {
    let env = Environment::new();
    let expr = arithma::parse_latex("\\tan{1}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert!(
        matches!(&result, Node::Function(name, _) if name == "tan"),
        "tan(1) should stay symbolic, got: {result}"
    );
}

#[test]
fn test_trig_special_values_still_evaluate() {
    let env = Environment::new();
    // sin(0) → 0
    let expr = arithma::parse_latex("\\sin{0}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert_eq!(result, Node::Num(ExactNum::integer(0)));

    // cos(0) → 1
    let expr = arithma::parse_latex("\\cos{0}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert_eq!(result, Node::Num(ExactNum::integer(1)));

    // sin(π) → 0
    let expr = arithma::parse_latex("\\sin{\\pi}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    assert_eq!(result, Node::Num(ExactNum::integer(0)));
}
```

**Step 2: Run tests, confirm the first three fail**

```bash
cargo test -- test_sin_integer_stays_symbolic test_cos_integer_stays_symbolic test_tan_integer_stays_symbolic test_trig_special_values_still_evaluate 2>&1 | tail -20
```

Expected: first three FAIL (sin/cos/tan return `Node::Num(Float(...))`), fourth PASSES.

**Step 3: Implement the fix**

In `src/simplify.rs`, find the `all_numeric` fallthrough block (around line 1350). The block currently reads:

```rust
let all_numeric = simplified_args.iter().all(|a| matches!(a, Node::Num(_)));
if all_numeric {
    let f64_args: Vec<f64> = simplified_args
        .iter()
        .map(|a| { ... })
        .collect();
    if let Ok(result) = crate::functions::call_function(name, f64_args) {
        if result.is_finite() {
            return Ok(Node::Num(ExactNum::from_f64(result)));
        }
    }
}
```

Add a guard that keeps trig/hyperbolic/inverse-trig functions symbolic when their arguments are numeric but not special values. Insert this check **before** the `all_numeric` block:

```rust
// Trig, inverse trig, and hyperbolic functions with non-special numeric args
// should stay symbolic rather than evaluating to float.
// try_exact_function_value already handled special values (sin(kπ), etc.)
// so anything reaching here has no closed form.
if matches!(
    name.as_str(),
    "sin" | "cos" | "tan"
        | "csc" | "sec" | "cot"
        | "arcsin" | "arccos" | "arctan" | "asin" | "acos" | "atan"
        | "arccsc" | "arcsec" | "arccot"
        | "sinh" | "cosh" | "tanh"
        | "csch" | "sech" | "coth"
        | "arcsinh" | "arccosh" | "arctanh"
        | "arccsch" | "arcsech" | "arccoth"
) {
    return Ok(Node::Function(name.clone(), simplified_args));
}
```

This goes right after the `ln` integer guard (around line 1339-1348) and before the `let all_numeric = ...` line.

**Step 4: Run all tests**

```bash
cargo fmt && cargo clippy --tests -- -D warnings && cargo test
```

Expected: ALL pass, including the new tests and all existing special-value tests.

**Step 5: Commit**

```bash
git add src/simplify.rs tests/simplify.rs
git commit -m "fix: keep trig functions with non-special numeric args symbolic

sin(2), cos(3), tan(1) etc. now stay as symbolic expressions instead of
evaluating to float. Special values (sin(0), sin(π), cos(π/2), etc.)
still evaluate exactly. Fixes #42."
```

---

## Task 2: Fix `1 + √2 + √2` not simplifying to `1 + 2√2` (GitHub #41)

**Problem:** `try_combine_like_radicals` works pairwise on adjacent `Add` operands. For `1 + √2 + √2`, the AST is `Add(Add(1, √2), √2)`. The inner `Add(1, √2)` doesn't combine (1 is not a radical), and the outer `Add(1+√2, √2)` doesn't match either because `extract_radical_parts` can't extract a radical from `1+√2`. The existing `collect_terms` fallback only handles `Variable` and `Multiply(Num, Variable)` — it returns `Err` on `Sqrt` nodes, so radical collection never happens through that path.

**Fix:** Extend `collect_terms_inner` to handle `Sqrt` and `Multiply(Num, Sqrt)` nodes, using the display form of the radical as the term key (same approach as `try_combine_like_radicals`). Extend `rebuild_expression` to reconstruct radical terms.

**Files:**
- Modify: `src/simplify.rs` — `collect_terms_inner` and `rebuild_expression`
- Test: `tests/simplify.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn test_like_radical_collection_three_terms() {
    let env = Environment::new();
    let expr = arithma::parse_latex("1 + \\sqrt{2} + \\sqrt{2}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    let s = format!("{result}");
    assert!(
        s.contains("2\\sqrt{2}") || s.contains("2√2"),
        "1 + √2 + √2 should simplify to 1 + 2√2, got: {s}"
    );
}

#[test]
fn test_like_radical_collection_subtract() {
    let env = Environment::new();
    let expr = arithma::parse_latex("3\\sqrt{3} + \\sqrt{3} - \\sqrt{3}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    let s = format!("{result}");
    assert!(
        s.contains("3\\sqrt{3}") || s.contains("3√3"),
        "3√3 + √3 - √3 should simplify to 3√3, got: {s}"
    );
}

#[test]
fn test_like_radical_collection_with_multiple_types() {
    let env = Environment::new();
    let expr = arithma::parse_latex("\\sqrt{2} + \\sqrt{3} + \\sqrt{2}", &env).unwrap();
    let result = Evaluator::simplify(&expr, &env).unwrap();
    let s = format!("{result}");
    assert!(
        s.contains("2\\sqrt{2}") || s.contains("2√2"),
        "√2 + √3 + √2 should have 2√2 term, got: {s}"
    );
    assert!(
        s.contains("\\sqrt{3}") || s.contains("√3"),
        "√2 + √3 + √2 should still have √3 term, got: {s}"
    );
}
```

**Step 2: Run tests, confirm they fail**

```bash
cargo test -- test_like_radical_collection 2>&1 | tail -15
```

**Step 3: Implement the fix**

In `collect_terms_inner` (around line 1396), add two new match arms before the `_ => Err(...)` fallback:

```rust
Node::Sqrt(_) | Node::Function(_, _) => {
    if extract_sqrt_radicand(node).is_some() {
        let key = format!("√{}", node);
        let entry = term_map.entry(key).or_insert_with(ExactNum::zero);
        *entry = entry.clone() + sign.clone();
        Ok(())
    } else {
        Err("Unsupported node type in collect_terms".to_string())
    }
}
```

And extend the `Node::Multiply` arm to handle `Multiply(Num, Sqrt)`:

In the existing `Node::Multiply` arm, after the `(Num, Variable)` case, add:

```rust
if let Node::Num(ref coef) = **left {
    if extract_sqrt_radicand(right).is_some() {
        let key = format!("√{}", right);
        let entry = term_map.entry(key).or_insert_with(ExactNum::zero);
        *entry = entry.clone() + coef.clone() * sign.clone();
        return Ok(());
    }
}
```

In `rebuild_expression` (around line 1446), update the term-to-node conversion. Currently it only builds `Node::Variable(var)` for non-constant terms. Add handling for radical keys (those starting with `√`):

In the loop body where terms are converted to nodes, replace the variable construction with:

```rust
let node = if var.is_empty() {
    Node::Num(abs_coef)
} else if var.starts_with("√") {
    // Radical term — parse the radical from the original format
    let radical_str = &var[("√".len())..];
    // The key is format!("√{}", sqrt_node), so radical_str is the Display of the sqrt node
    // We need to reconstruct the node — parse it from the stored representation
    if let Ok(radical_node) = parse_radical_key(radical_str) {
        if abs_coef.is_one() {
            radical_node
        } else {
            Node::Multiply(Box::new(Node::Num(abs_coef)), Box::new(radical_node))
        }
    } else {
        // Fallback: treat as variable
        if abs_coef.is_one() {
            Node::Variable(var)
        } else {
            Node::Multiply(Box::new(Node::Num(abs_coef)), Box::new(Node::Variable(var)))
        }
    }
} else if abs_coef.is_one() {
    Node::Variable(var)
} else {
    Node::Multiply(Box::new(Node::Num(abs_coef)), Box::new(Node::Variable(var)))
};
```

However, parsing from Display is fragile. A cleaner approach: change `collect_terms` to use a `TermKey` enum instead of `String`, or store the radical `Node` alongside the key. The simplest correct approach:

**Alternative (recommended):** Instead of modifying `collect_terms` (which has a string-keyed design that doesn't fit radicals well), add a dedicated **multi-term radical collection pass** that runs after pairwise combination fails. This pass flattens the sum into a list of terms, groups by radical identity, and combines coefficients.

Add a new function `collect_and_combine_radicals`:

```rust
fn collect_and_combine_radicals(node: &Node, env: &Environment) -> Option<Node> {
    let mut terms: Vec<(ExactNum, Option<Node>)> = Vec::new();  // (coeff, radical_or_none)
    if !flatten_additive_terms(node, &ExactNum::one(), &mut terms) {
        return None;
    }

    // Group by radical identity (Display string for matching)
    let mut radical_groups: HashMap<String, (ExactNum, Node)> = HashMap::new();
    let mut constant = ExactNum::zero();
    let mut non_radical_terms: Vec<Node> = Vec::new();
    let mut had_radicals = false;

    for (coeff, radical) in terms {
        match radical {
            Some(rad) => {
                had_radicals = true;
                let key = format!("{}", rad);
                let entry = radical_groups.entry(key).or_insert_with(|| (ExactNum::zero(), rad));
                entry.0 = entry.0.clone() + coeff;
            }
            None => {
                constant = constant + coeff;
            }
        }
    }

    if !had_radicals {
        return None;
    }

    // Rebuild: constants first, then sorted radical terms
    let mut result_terms: Vec<Node> = Vec::new();
    if !constant.is_zero() {
        result_terms.push(Node::Num(constant));
    }
    let mut sorted: Vec<_> = radical_groups.into_values().collect();
    sorted.sort_by(|a, b| format!("{}", a.1).cmp(&format!("{}", b.1)));
    for (coeff, radical) in sorted {
        if coeff.is_zero() { continue; }
        if coeff.is_one() {
            result_terms.push(radical);
        } else if coeff == ExactNum::integer(-1) {
            result_terms.push(Node::Negate(Box::new(radical)));
        } else if coeff.is_negative() {
            let abs_coeff = -coeff;
            result_terms.push(Node::Negate(Box::new(
                Node::Multiply(Box::new(Node::Num(abs_coeff)), Box::new(radical))
            )));
        } else {
            result_terms.push(Node::Multiply(Box::new(Node::Num(coeff)), Box::new(radical)));
        }
    }

    if result_terms.is_empty() {
        return Some(Node::Num(ExactNum::zero()));
    }
    let mut iter = result_terms.into_iter();
    let mut result = iter.next().unwrap();
    for term in iter {
        match term {
            Node::Negate(inner) => {
                result = Node::Subtract(Box::new(result), inner);
            }
            _ => {
                result = Node::Add(Box::new(result), Box::new(term));
            }
        }
    }
    Some(result)
}

fn flatten_additive_terms(
    node: &Node,
    sign: &ExactNum,
    terms: &mut Vec<(ExactNum, Option<Node>)>,
) -> bool {
    match node {
        Node::Add(left, right) => {
            flatten_additive_terms(left, sign, terms) &&
            flatten_additive_terms(right, sign, terms)
        }
        Node::Subtract(left, right) => {
            let neg = sign.clone() * ExactNum::integer(-1);
            flatten_additive_terms(left, sign, terms) &&
            flatten_additive_terms(right, &neg, terms)
        }
        Node::Negate(inner) => {
            let neg = sign.clone() * ExactNum::integer(-1);
            flatten_additive_terms(inner, &neg, terms)
        }
        Node::Num(n) => {
            terms.push((n.clone() * sign.clone(), None));
            true
        }
        _ => {
            if let Some((coeff, radical)) = extract_radical_parts(node) {
                if extract_sqrt_radicand(&radical).is_some() {
                    terms.push((coeff * sign.clone(), Some(radical)));
                    return true;
                }
            }
            // Non-radical, non-numeric term — bail out
            false
        }
    }
}
```

Then call it in the `Node::Add` simplification path, after the pairwise `try_combine_like_radicals` and before the `collect_terms` fallback:

```rust
// Multi-term radical collection: 1 + √2 + √2 → 1 + 2√2
if let Some(combined) = collect_and_combine_radicals(&result, env) {
    return Ok(combined);
}
```

Also add the same call in the `Node::Subtract` path at the corresponding location.

**Step 4: Run all tests**

```bash
cargo fmt && cargo clippy --tests -- -D warnings && cargo test
```

**Step 5: Commit**

```bash
git add src/simplify.rs tests/simplify.rs
git commit -m "fix: collect like radicals across multi-term sums

1 + √2 + √2 now simplifies to 1 + 2√2. Adds a multi-term radical
collection pass that flattens additive chains and groups by radical
identity. The pairwise combination was insufficient for non-adjacent
like radicals. Fixes #41."
```

---

## Task 3: Fix verify tool assumption gap

**Problem:** `verify_identity` in `src/verify.rs` evaluates at hardcoded test points (`[0.5, -0.5, 1.5, -1.5, ...]`) without consulting the assumption system. When verifying `√(x²) = x` with `{x: positive}`, it tests negative x values and produces spurious counterexamples.

**Fix:** Accept an `Assumptions` parameter and filter test points to satisfy stated constraints. The function signature changes, so update the call site in `arithma-mcp.rs` as well.

**Files:**
- Modify: `src/verify.rs`
- Modify: `src/bin/arithma-mcp.rs` (call site)
- Test: `tests/verify.rs`

**Step 1: Write failing tests**

In `tests/verify.rs`:

```rust
#[test]
fn test_verify_with_positive_assumption() {
    use arithma::assumptions::{Assumption, Assumptions};
    let env = Environment::new();
    let lhs = arithma::parse_latex("\\sqrt{x^2}", &env).unwrap();
    let rhs = arithma::parse_latex("x", &env).unwrap();

    // Without assumptions — should fail (negative x is a counterexample)
    let result_no_assume = arithma::verify::verify_identity(
        &lhs, &rhs, &["x".to_string()], &Assumptions::new(),
    );
    assert!(!result_no_assume.passed, "√(x²) = x should fail without assumptions");

    // With x: positive — should pass
    let mut assumptions = Assumptions::new();
    assumptions.assume("x", Assumption::Positive);
    let result_positive = arithma::verify::verify_identity(
        &lhs, &rhs, &["x".to_string()], &assumptions,
    );
    assert!(result_positive.passed, "√(x²) = x should pass with x > 0");
}

#[test]
fn test_verify_with_nonneg_assumption() {
    use arithma::assumptions::{Assumption, Assumptions};
    let env = Environment::new();
    let lhs = arithma::parse_latex("\\sqrt{x}", &env).unwrap();
    let rhs_squared = arithma::parse_latex("\\sqrt{x}", &env).unwrap();

    // NonNegative: no negative test points
    let mut assumptions = Assumptions::new();
    assumptions.assume("x", Assumption::NonNegative);
    let result = arithma::verify::verify_identity(
        &lhs, &rhs_squared, &["x".to_string()], &assumptions,
    );
    assert!(result.passed);
    assert!(result.points_tested >= 3, "Should have enough non-negative test points");
}
```

**Step 2: Run tests, confirm they fail**

The tests won't compile because `verify_identity` doesn't accept an `Assumptions` parameter yet.

**Step 3: Implement the fix**

In `src/verify.rs`:

1. Add `use crate::assumptions::{Assumption, Assumptions};`

2. Add more test points that include positive-only values:
```rust
const TEST_POINTS: &[f64] = &[0.5, -0.5, 1.5, -1.5, 0.3, -0.7, 2.1, 0.1, -2.3, 3.0, 0.8, 4.5];
```

3. Change `verify_identity` signature:
```rust
pub fn verify_identity(lhs: &Node, rhs: &Node, variables: &[String], assumptions: &Assumptions) -> VerifyResult {
```

4. Add a point-filtering function:
```rust
fn point_satisfies_assumptions(var: &str, val: f64, assumptions: &Assumptions) -> bool {
    if assumptions.is_positive(var) && val <= 0.0 {
        return false;
    }
    if assumptions.is_nonneg(var) && val < 0.0 {
        return false;
    }
    if assumptions.is_negative(var) && val >= 0.0 {
        return false;
    }
    if assumptions.is_nonzero(var) && val == 0.0 {
        return false;
    }
    if assumptions.is_integer(var) && val.fract() != 0.0 {
        return false;
    }
    true
}
```

5. In the evaluation loop, after computing the test point value for each variable, check if it satisfies assumptions. If any variable's value violates its assumption, skip this test point:
```rust
let mut skip_point = false;
for (j, var) in normalized.iter().enumerate() {
    let val = base_point + 0.3 * j as f64 + 0.1 * i as f64;
    if !point_satisfies_assumptions(var, val, assumptions) {
        skip_point = true;
        break;
    }
    env.set(var, val);
    point_values.push((var.clone(), val));
}
if skip_point {
    continue;
}
```

6. Update the call site in `src/bin/arithma-mcp.rs`. Find the `verify_identity` call (search for `verify_identity`) and pass the assumptions from the environment:
```rust
// Before: verify_identity(&lhs_tree, &rhs_tree, &variables)
// After:  verify_identity(&lhs_tree, &rhs_tree, &variables, env.assumptions())
```

7. Update existing tests in `tests/verify.rs` to pass `&Assumptions::new()` as the fourth argument to `verify_identity`.

**Step 4: Run all tests**

```bash
cargo fmt && cargo clippy --tests -- -D warnings && cargo test
```

**Step 5: Commit**

```bash
git add src/verify.rs src/bin/arithma-mcp.rs tests/verify.rs
git commit -m "fix: verify tool filters test points by stated assumptions

verify_identity now accepts an Assumptions parameter and skips test
points that violate constraints (positive, nonneg, negative, nonzero,
integer). Prevents spurious counterexamples like √(x²) ≠ x when
x is assumed positive."
```

---

## Task 4: Modular crate split — Cargo workspace

**Problem:** Arithma is a single crate with the math engine, CLI, and MCP server bundled together. Someone who wants just the computation library gets the MCP server's dependencies. The CLI and MCP binaries can't be versioned or released independently.

**Goal:** Convert to a Cargo workspace with three members:
- `arithma` (root) — the math engine library. No binary targets. This is what downstream Rust projects depend on.
- `crates/cli/` — the `arithma` CLI binary. Depends on `arithma`.
- `crates/mcp/` — the `arithma-mcp` binary. Depends on `arithma`.

**Constraint:** All 1275+ tests must pass after the split. The public API of `arithma` (the lib) does not change. Binary names stay the same.

**Files:**
- Modify: `Cargo.toml` (root — becomes workspace root + lib crate)
- Create: `crates/cli/Cargo.toml`
- Create: `crates/mcp/Cargo.toml`
- Move: `src/main.rs` → `crates/cli/src/main.rs`
- Move: `src/bin/arithma-mcp.rs` → `crates/mcp/src/main.rs`
- Modify: `src/lib.rs` — remove `wasm_bindings` (move to feature gate or keep)

**Step 1: Create workspace directory structure**

```bash
mkdir -p crates/cli/src crates/mcp/src
```

**Step 2: Move binary sources**

```bash
cp src/main.rs crates/cli/src/main.rs
cp src/bin/arithma-mcp.rs crates/mcp/src/main.rs
```

Use `cp` first, not `mv` — keep originals until we confirm the workspace builds.

**Step 3: Create `crates/cli/Cargo.toml`**

```toml
[package]
name = "arithma-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "arithma"
path = "src/main.rs"

[dependencies]
arithma = { path = "../.." }
```

**Step 4: Create `crates/mcp/Cargo.toml`**

```toml
[package]
name = "arithma-mcp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "arithma-mcp"
path = "src/main.rs"

[dependencies]
arithma = { path = "../.." }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
env_logger = "0.9"
```

**Step 5: Update root `Cargo.toml`**

Add workspace definition at the top. Remove the `[[bin]]` sections and `default-run`. Keep the `[lib]` section and all library dependencies. Remove `serde_json`, `log`, `env_logger` if they're only used by the MCP binary (check with `grep` first — `serde_json` is used in `assumptions.rs` so it stays in the lib).

```toml
[workspace]
members = [".", "crates/cli", "crates/mcp"]

[package]
name = "arithma"
version = "0.1.0"
edition = "2021"

[lib]
name = "arithma"
path = "src/lib.rs"
crate-type = ["cdylib", "rlib"]

[features]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
lazy_static = "1.5.0"
regex = "1.9.5"
num-bigint = { version = "0.4", features = ["serde"] }
num-rational = { version = "0.4", features = ["serde"] }
num-traits = "0.2"
num-integer = "0.1"
```

Note: `log` and `env_logger` should only stay if they're used by the lib. Check with:
```bash
grep -rn 'use log\|use env_logger\|log::' src/ --include='*.rs' | grep -v 'src/main.rs\|src/bin/'
```

**Step 6: Remove old binary sources from root**

```bash
rm src/main.rs
rm -r src/bin/
```

**Step 7: Build and test the workspace**

```bash
cargo fmt --all
cargo clippy --workspace --tests -- -D warnings
cargo test --workspace
```

All 1275+ tests should pass. The test crates in `tests/` are part of the root package, so they run with `cargo test` on the root or with `--workspace`.

**Step 8: Verify binary names are correct**

```bash
cargo build --workspace
ls target/debug/arithma target/debug/arithma-mcp
```

Both binaries should exist with the original names.

**Step 9: Commit**

```bash
git add -A
git commit -m "refactor: split into cargo workspace with arithma-core, cli, and mcp crates

The math engine library stays at the root as 'arithma'. The CLI moves to
crates/cli/ and the MCP server to crates/mcp/. Binary names unchanged.
All tests pass. Downstream Rust projects can now depend on 'arithma'
without pulling in the MCP server's dependencies."
```

---

## Sprint Summary

| Task | Issue | Type | Effort |
|------|-------|------|--------|
| 1. sin(2) → float | #42 | Bug fix | < 30 min |
| 2. 1+√2+√2 collection | #41 | Bug fix | ~1 hour |
| 3. Verify assumptions | — | Bug fix | ~30 min |
| 4. Crate split | — | Refactor | ~1 hour |

**Total estimated effort:** One focused session.

**Dependencies:** None between tasks. All four are independent. Task 4 should go last since it restructures the repo.
