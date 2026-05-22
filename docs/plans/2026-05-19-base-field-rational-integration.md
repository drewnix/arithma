# Base-Field Rational Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate rational functions of x into structured results (rational part + ln(x) coefficient), enabling `integrate_poly_log` to handle rational coefficients in the logarithmic Risch extension.

**Architecture:** Add `integrate_rational_base()` that decomposes a RationalFunction via partial fractions and integrates each term, separating the rational part from the ln(x) coefficient. Modify `integrate_poly_log` to use RationalFunction coefficients with a Δ accumulator for ln(x) contributions across degrees.

**Tech Stack:** Rust, existing `partial_fraction_decomposition`, `RationalFunction`, `BigRational`.

---

### Task 1: Implement integrate_rational_base

The core new function: integrate a rational function of x, returning a structured result.

**Files:**
- Modify: `src/risch.rs` (add struct + function after `solve_risch_de_rational`, around line 732)

**Step 1: Write failing tests**

Add to the `mod tests` block in `src/risch.rs`. Use existing helpers `int()`, `poly()`:

```rust
#[test]
fn test_integrate_rational_base_polynomial() {
    // ∫x² dx = x³/3 (polynomial case, no log)
    let rf = RationalFunction::from_poly(poly(&[0, 0, 1], "x")); // x²
    let result = integrate_rational_base(&rf, "x").unwrap();
    // x³/3 as RF
    let expected_num = Polynomial::from_coeffs(
        vec![int(0), int(0), int(0), BigRational::new(int(1).numer().clone(), BigInt::from(3))],
        "x",
    );
    assert_eq!(*result.rational_part.denominator(), Polynomial::one("x"));
    assert!(result.ln_x_coeff.is_zero());
}

#[test]
fn test_integrate_rational_base_inv_x_sq() {
    // ∫1/x² dx = -1/x (rational, no log)
    let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x")); // 1/x²
    let result = integrate_rational_base(&rf, "x").unwrap();
    let expected = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x")); // -1/x
    assert_eq!(result.rational_part, expected);
    assert!(result.ln_x_coeff.is_zero());
}

#[test]
fn test_integrate_rational_base_inv_x() {
    // ∫1/x dx = ln(x) → rational_part = 0, ln_x_coeff = 1
    let rf = RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")); // 1/x
    let result = integrate_rational_base(&rf, "x").unwrap();
    assert!(result.rational_part.is_zero());
    assert_eq!(result.ln_x_coeff, int(1));
}

#[test]
fn test_integrate_rational_base_inv_x_plus_1() {
    // ∫1/(x+1) dx = ln(x+1) → non-elementary in single ln(x) tower
    let rf = RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x")); // 1/(x+1)
    let result = integrate_rational_base(&rf, "x");
    assert!(result.is_err());
}

#[test]
fn test_integrate_rational_base_mixed() {
    // ∫(x+1)/x² dx = ∫1/x + 1/x² dx = ln(x) - 1/x
    // → rational_part = -1/x, ln_x_coeff = 1
    let rf = RationalFunction::new(
        poly(&[1, 1], "x"),    // x + 1
        poly(&[0, 0, 1], "x"), // x²
    );
    let result = integrate_rational_base(&rf, "x").unwrap();
    let expected_rat = RationalFunction::new(poly(&[-1], "x"), poly(&[0, 1], "x")); // -1/x
    assert_eq!(result.rational_part, expected_rat);
    assert_eq!(result.ln_x_coeff, int(1));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib risch::tests::test_integrate_rational_base 2>&1 | head -5`
Expected: compilation error — `integrate_rational_base` not defined.

**Step 3: Implement**

Add the struct and function in `src/risch.rs` after `solve_risch_de_rational` (around line 732):

```rust
/// Result of integrating a rational function in the base field K(x).
///
/// The antiderivative is: rational_part + ln_x_coeff · ln(x).
#[derive(Debug)]
pub struct BaseFieldIntegral {
    pub rational_part: RationalFunction,
    pub ln_x_coeff: BigRational,
}

/// Integrate a rational function of x, returning the rational part and
/// the coefficient of ln(x) separately.
///
/// Returns `Err` if the integral requires logarithms other than ln(x)
/// (e.g., ln(x+1)) or inverse trig functions — these are non-elementary
/// in the single ln(x) extension tower.
///
/// ## Algorithm
///
/// 1. Partial fraction decomposition
/// 2. Integrate polynomial part → polynomial
/// 3. For each PF term c/(x-a)^k:
///    - k > 1: rational function c/((1-k)·(x-a)^{k-1})
///    - k = 1, a = 0: contributes c to ln_x_coeff
///    - k = 1, a ≠ 0: non-elementary (ln(x-a) outside tower)
///    - irreducible quadratic denominator: non-elementary
pub fn integrate_rational_base(
    rf: &RationalFunction,
    var: &str,
) -> Result<BaseFieldIntegral, String> {
    // Handle zero
    if rf.is_zero() {
        return Ok(BaseFieldIntegral {
            rational_part: RationalFunction::zero(var),
            ln_x_coeff: BigRational::zero(),
        });
    }

    let decomp = crate::partial_fractions::partial_fraction_decomposition(
        rf.numerator(),
        rf.denominator(),
    )?;

    let mut rational_part = RationalFunction::from_poly(decomp.polynomial_part.integral());
    let mut ln_x_coeff = BigRational::zero();

    for term in &decomp.terms {
        let q_deg = term.denominator.degree().unwrap_or(0);

        if q_deg >= 2 {
            return Err(format!(
                "Non-elementary: irreducible factor {} of degree {} requires \
                 algebraic extension",
                term.denominator, q_deg
            ));
        }

        // Linear factor: denominator is monic (x + a), so a = coeff(0)
        let a = term.denominator.coeff(0);
        let c = term.numerator.coeff(0); // numerator is constant for linear factor

        if term.power == 1 {
            if a.is_zero() {
                // c/x → c·ln(x)
                ln_x_coeff += c;
            } else {
                // c/(x+a) → c·ln(x+a), non-elementary in single ln(x) tower
                return Err(format!(
                    "Non-elementary: integral requires ln({}), which is \
                     outside the single ln(x) extension tower",
                    term.denominator
                ));
            }
        } else {
            // c/(x+a)^k, k > 1 → c/((1-k)·(x+a)^{k-1})
            let exp = 1i64 - term.power as i64;
            let scale = &c / &BigRational::from_integer(BigInt::from(exp));
            // Build (x+a)^{k-1} as a polynomial
            let mut den_power = Polynomial::one(var);
            for _ in 0..term.power - 1 {
                den_power = &den_power * &term.denominator;
            }
            let term_rf = RationalFunction::new(
                Polynomial::constant(scale, var),
                den_power,
            );
            rational_part = &rational_part + &term_rf;
        }
    }

    Ok(BaseFieldIntegral {
        rational_part,
        ln_x_coeff,
    })
}
```

**Step 4: Run tests**

Run: `cargo test --lib risch::tests::test_integrate_rational_base 2>&1 | grep -E "test |FAILED"`
Expected: all 5 new tests pass.

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 767 (762 + 5 new), 0 failed.

**Step 5: Run clippy + fmt**

Run: `cargo fmt && cargo clippy --tests -- -D warnings 2>&1 | tail -3`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Add integrate_rational_base for structured rational function integration"
```

---

### Task 2: Modify integrate_poly_log for rational coefficients

Change `integrate_poly_log` to handle rational function coefficients using `integrate_rational_base` and the Δ accumulator.

**Files:**
- Modify: `src/risch.rs` (`integrate_poly_log`, starts around line 1293)

**Step 1: Write failing tests**

Add to `mod tests` in `src/risch.rs`:

```rust
#[test]
fn test_integrate_poly_log_rational_coeff() {
    // ∫(1/x²)·ln(x) dx = -(ln(x)+1)/x
    // ExtPoly: (1/x²)·θ where θ = ln(x)
    // Recurrence: q_1' = 1/x², so q_1 = -1/x
    //   q_0' = 0 - 1·q_1/x = 0 - (-1/x)/x = 1/x²
    //   q_0 = -1/x
    // Answer: -1/x + (-1/x)·θ = -(1+ln(x))/x
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        RationalFunction::new(poly(&[1], "x"), poly(&[0, 0, 1], "x")), // 1/x²
    ], "x");
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_poly_log(&num, &ext, "x") {
        Some(RischResult::Elementary(_node)) => {
            // Success — should produce -(1+ln(x))/x or equivalent
        }
        other => panic!("Expected Elementary, got: {:?}", other),
    }
}

#[test]
fn test_integrate_poly_log_rational_with_ln_x_absorption() {
    // ∫(1/x + ln(x)) dx = (x+1)·ln(x) - x
    // ExtPoly: a_0 = 1/x, a_1 = 1
    // Recurrence: q_1' = 1 → q_1 = x (+ constant C)
    //   q_0' = 1/x - (q_1 + Δ)/x
    //   At degree 1: ∫1 = x, no ln term, Δ stays 0
    //   But q_1 = x has no constant term, so q_1/x = 1
    //   q_0' = 1/x - 1 → ∫(1/x - 1) = ln(x) - x
    //   The ln(x) part: Δ += 1
    //   q_0 = -x (rational part only)
    //   Final: q_0 + (q_1 + Δ)·θ = -x + (x+1)·ln(x)
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::new(poly(&[1], "x"), poly(&[0, 1], "x")), // 1/x
        RationalFunction::from_poly(poly(&[1], "x")),                // 1
    ], "x");
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_poly_log(&num, &ext, "x") {
        Some(RischResult::Elementary(_node)) => {
            // Should produce (x+1)·ln(x) - x or equivalent
        }
        other => panic!("Expected Elementary, got: {:?}", other),
    }
}

#[test]
fn test_integrate_poly_log_rational_non_elementary() {
    // ∫(1/(x+1))·ln(x) dx is non-elementary in single ln(x) tower
    // because ∫1/(x+1) = ln(x+1), which requires a second extension
    let num = ExtPoly::from_coeffs(vec![
        RationalFunction::zero("x"),
        RationalFunction::new(poly(&[1], "x"), poly(&[1, 1], "x")), // 1/(x+1)
    ], "x");
    let ext = DifferentialExtension::logarithmic(
        RationalFunction::from_poly(poly(&[0, 1], "x")),
        "x",
    );
    match integrate_poly_log(&num, &ext, "x") {
        Some(RischResult::NonElementary(_)) => {}
        other => panic!("Expected NonElementary, got: {:?}", other),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib risch::tests::test_integrate_poly_log_rational 2>&1 | grep -E "test |FAILED"`
Expected: failures (currently returns None for rational coefficients).

**Step 3: Rewrite integrate_poly_log**

Replace the function at `src/risch.rs` (around line 1293-1366) with:

```rust
/// Integrate a polynomial in θ = ln(x): Σ aᵢ(x)·θⁱ.
///
/// Top-down recurrence: qₙ' = aₙ, then qₖ' = aₖ - (k+1)·q_{k+1}/x.
/// When coefficients are rational functions, integration may produce
/// ln(x) = θ terms, which are absorbed via a Δ accumulator.
fn integrate_poly_log(
    num: &ExtPoly,
    _ext: &DifferentialExtension,
    var: &str,
) -> Option<RischResult> {
    let deg = num.degree().unwrap_or(0);
    let mut q: Vec<RationalFunction> = vec![RationalFunction::zero(var); deg + 1];
    let mut ln_x_accum = BigRational::zero();

    let x_rf = RationalFunction::from_poly(Polynomial::x(var));

    for k in (0..=deg).rev() {
        let a_k_rf = num.coeff(k);

        let rhs: RationalFunction = if k == deg {
            // Top degree: RHS = a_k directly
            a_k_rf
        } else {
            // RHS = a_k - (k+1) · q_{k+1} / x
            // But at degree 0, use (q_1 + Δ) instead of q_1
            let q_kp1 = if k == 0 && deg >= 1 {
                // Effective q_1 includes the ln(x) accumulator
                let delta_rf = RationalFunction::from_constant(ln_x_accum.clone(), var);
                &q[1] + &delta_rf
            } else {
                q[k + 1].clone()
            };

            let scalar = BigRational::from_integer(BigInt::from(k as i64 + 1));
            let scalar_rf = RationalFunction::from_constant(scalar, var);
            let correction = &(&scalar_rf * &q_kp1).checked_div(&x_rf).ok()?;
            &a_k_rf - correction
        };

        if rhs.is_zero() {
            continue;
        }

        match integrate_rational_base(&rhs, var) {
            Ok(result) => {
                q[k] = result.rational_part;
                if k == 0 {
                    // At degree 0, any ln(x) is non-elementary — no lower degree to absorb
                    if !result.ln_x_coeff.is_zero() {
                        return Some(RischResult::NonElementary(
                            "No elementary antiderivative exists. \
                             The degree-0 integration produces a ln(x) term \
                             with no lower degree to absorb it."
                                .into(),
                        ));
                    }
                } else {
                    ln_x_accum += result.ln_x_coeff;
                }
            }
            Err(reason) => {
                return Some(RischResult::NonElementary(format!(
                    "No elementary antiderivative exists. {}",
                    reason
                )));
            }
        }
    }

    // Build result: Σ q_k · θ^k, with q_1 adjusted by ln_x_accum
    let ln_x = Node::Function("ln".to_string(), vec![Node::Variable(var.to_string())]);
    let mut terms: Vec<Node> = Vec::new();

    for k in 0..=deg {
        let qk = if k == 1 {
            // Add the ln(x) accumulator to the θ coefficient
            let delta_rf = RationalFunction::from_constant(ln_x_accum.clone(), var);
            &q[1] + &delta_rf
        } else {
            q[k].clone()
        };

        if qk.is_zero() {
            continue;
        }

        let q_node = rf_to_node(&qk, var);
        let term = if k == 0 {
            q_node
        } else if k == 1 {
            Node::Multiply(Box::new(q_node), Box::new(ln_x.clone()))
        } else {
            Node::Multiply(
                Box::new(q_node),
                Box::new(Node::Power(
                    Box::new(ln_x.clone()),
                    Box::new(Node::Num(ExactNum::integer(k as i64))),
                )),
            )
        };
        terms.push(term);
    }

    // Handle the case where deg == 0 but ln_x_accum is nonzero
    // (This shouldn't happen since we check at degree 0, but safety net)
    if deg == 0 && !ln_x_accum.is_zero() {
        return Some(RischResult::NonElementary(
            "No elementary antiderivative exists.".into(),
        ));
    }

    if terms.is_empty() {
        return Some(RischResult::Elementary(Node::Num(ExactNum::zero())));
    }
    let mut result = terms.remove(0);
    for t in terms {
        result = Node::Add(Box::new(result), Box::new(t));
    }
    Some(RischResult::Elementary(result))
}
```

**Important implementation notes:**

- The `checked_div` for `q_{k+1} / x` creates `RF(q_num, q_den * x)`. The RationalFunction::new normalizer will cancel if x divides the numerator. This replaces the old `q_kp1.div_rem(&x_poly)` check.
- The old code's check `if !q_kp1.coeff(0).is_zero()` (line 1316) was detecting a nonzero constant term in q_{k+1}, which would make q_{k+1}/x have a 1/x pole. With the new code, this case is handled naturally: q_{k+1}/x produces a RationalFunction with a 1/x term, and `integrate_rational_base` separates the ln(x) coefficient.
- The `RationalFunction::one` comparison for output node building becomes `qk == RationalFunction::one(var)`.

**Step 4: Run all tests**

Run: `cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`
Expected: 770 (767 + 3 new), 0 failed.

**Step 5: Run clippy + fmt**

Run: `cargo fmt && cargo clippy --tests -- -D warnings 2>&1 | tail -3`

**Step 6: Commit**

```bash
git add src/risch.rs
git commit -m "Upgrade integrate_poly_log for rational coefficients with ln(x) absorption"
```

---

### Task 3: End-to-end tests and README update

**Files:**
- Modify: `src/integration.rs` (add end-to-end tests)
- Modify: `README.md`

**Step 1: Write end-to-end tests**

Add to the test module in `src/integration.rs`:

```rust
#[test]
fn test_integrate_ln_x_over_x_sq() {
    // ∫ln(x)/x² dx = -(ln(x)+1)/x
    let expr = parse_expression("\\frac{\\ln(x)}{x^2}").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_ok(), "Expected elementary result, got: {:?}", result);
}

#[test]
fn test_integrate_inv_x_plus_ln_x() {
    // ∫(1/x + ln(x)) dx = (x+1)·ln(x) - x
    let expr = parse_expression("\\frac{1}{x} + \\ln(x)").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_ok(), "Expected elementary result, got: {:?}", result);
}

#[test]
fn test_integrate_ln_x_over_x_plus_1_non_elementary() {
    // ∫ln(x)/(x+1) dx is non-elementary in single tower
    let expr = parse_expression("\\frac{\\ln(x)}{x + 1}").unwrap();
    let result = integrate(&expr, "x");
    assert!(result.is_err(), "Expected non-elementary, got: {:?}", result);
}
```

**Step 2: Test via CLI**

```bash
cargo run -- integrate "\\frac{\\ln(x)}{x^2}"
# Expected: -(ln(x)+1)/x or equivalent

cargo run -- integrate "\\frac{1}{x} + \\ln(x)"
# Expected: (x+1)·ln(x) - x or equivalent

cargo run -- integrate "\\frac{\\ln(x)}{x + 1}"
# Expected: NON_ELEMENTARY message
```

**Step 3: Debug if needed**

If end-to-end tests fail, the most likely issue is the tower builder not correctly representing the expression. Check that `build_tower` produces the right ExtPoly for `ln(x)/x²` — it should fold the 1/x² into the ExtPoly coefficient via the `den.is_constant()` branch in `try_risch_tower` (fixed earlier this session).

**Step 4: Update README**

- Update test count
- Add CLI examples for the new logarithmic rational integration

**Step 5: Run full suite**

Run: `cargo fmt && cargo clippy --tests -- -D warnings && cargo test 2>&1 | grep "^test result:" | awk '{sum += $4; fail += $6} END {print "Passed:", sum, "Failed:", fail}'`

**Step 6: Commit**

```bash
git add src/integration.rs README.md
git commit -m "Add end-to-end tests for log-extension rational integration, update README"
```

---

## Implementation Notes

**RationalFunction arithmetic:** Add (`&a + &b`), Sub (`&a - &b`), Mul (`&a * &b`), Neg (`-&a`), and `checked_div` are all available. Division by x is `rf.checked_div(&RF::from_poly(Polynomial::x(var)))`.

**Partial fractions returns monic denominators:** The `PartialFractionTerm.denominator` is always monic (leading coeff = 1). For linear factors, `denominator = x + a` where `a = coeff(0)`. Check `a.is_zero()` to detect the x=0 pole (produces ln(x) = θ).

**The Δ accumulator only modifies θ¹:** All ln(x) contributions from any degree get absorbed into the coefficient of θ = ln(x). The accumulator is applied at degree 0 (to adjust the correction) and in the final output (to adjust q[1]).

**Scope boundary:** Irreducible quadratic denominators in partial fractions → non-elementary. This is correct: arctan and algebraic logarithms are outside the single ln(x) tower. Multi-extension towers would handle some of these cases.
