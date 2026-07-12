#[cfg(test)]
mod verify_tests {
    use arithma::assumptions::{Assumption, Assumptions};
    use arithma::{build_expression_tree, Tokenizer};

    fn parse(input: &str) -> arithma::Node {
        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        build_expression_tree(tokens).unwrap()
    }

    fn verify(a: &str, b: &str, vars: &[&str]) -> arithma::verify::VerifyResult {
        let a_expr = parse(a);
        let b_expr = parse(b);
        let variables: Vec<String> = vars.iter().map(|s| s.to_string()).collect();
        arithma::verify_identity(&a_expr, &b_expr, &variables, &Assumptions::new())
    }

    fn verify_with_assumptions(
        a: &str,
        b: &str,
        vars: &[&str],
        assumptions: &Assumptions,
    ) -> arithma::verify::VerifyResult {
        let a_expr = parse(a);
        let b_expr = parse(b);
        let variables: Vec<String> = vars.iter().map(|s| s.to_string()).collect();
        arithma::verify_identity(&a_expr, &b_expr, &variables, assumptions)
    }

    // ── Passing verifications ────────────────────────────────

    #[test]
    fn verify_difference_of_squares() {
        let result = verify("x^2 - 1", "(x-1)(x+1)", &["x"]);
        assert!(result.passed, "x²-1 = (x-1)(x+1) should pass");
        assert!(result.points_tested >= 5);
    }

    #[test]
    fn verify_derivative() {
        let result = verify("3x^2", "3x^2", &["x"]);
        assert!(result.passed, "d/dx[x³] = 3x² should pass");
    }

    #[test]
    fn verify_trig_identity() {
        let result = verify("\\sin(x)^2 + \\cos(x)^2", "1", &["x"]);
        assert!(result.passed, "sin²+cos²=1 should pass");
    }

    #[test]
    fn verify_multivar() {
        let result = verify("(x+y)^2", "x^2 + 2 \\cdot x \\cdot y + y^2", &["x", "y"]);
        assert!(result.passed, "(x+y)² = x²+2xy+y² should pass");
    }

    // ── Failing verifications ────────────────────────────────

    #[test]
    fn verify_not_equal() {
        let result = verify("x^2", "x^3", &["x"]);
        assert!(!result.passed, "x² ≠ x³");
        assert!(result.counterexample.is_some());
        let cx = result.counterexample.unwrap();
        assert!((cx.lhs_value - cx.rhs_value).abs() > 1e-8);
    }

    #[test]
    fn verify_close_but_wrong() {
        let result = verify("x^2 + 1", "x^2 + 2", &["x"]);
        assert!(!result.passed, "x²+1 ≠ x²+2");
    }

    // ── Display format ───────────────────────────────────────

    #[test]
    fn display_pass() {
        let result = verify("x", "x", &["x"]);
        let s = format!("{}", result);
        assert!(s.contains("PASS"), "Should say PASS, got: {}", s);
        assert!(s.contains("points"), "Should mention points, got: {}", s);
    }

    #[test]
    fn display_fail() {
        let result = verify("x", "2x", &["x"]);
        let s = format!("{}", result);
        assert!(s.contains("FAIL"), "Should say FAIL, got: {}", s);
        assert!(s.contains("LHS"), "Should show LHS value, got: {}", s);
        assert!(s.contains("RHS"), "Should show RHS value, got: {}", s);
    }

    // ── Bug #1 fixes ────────────────────────────────────────

    #[test]
    fn greek_variable_names_normalized() {
        let result = verify("\\frac{1}{1+\\alpha}", "\\frac{1}{1+\\alpha}", &["\\alpha"]);
        assert!(
            result.passed,
            "Greek var should be normalized, got: {}",
            result
        );
        assert!(result.points_tested >= 5, "Should test at multiple points");
    }

    #[test]
    fn zero_points_is_not_pass() {
        let result = verify("x", "x", &["nonexistent_var_zzzz"]);
        assert!(!result.passed, "0 points tested should not be PASS");
        assert!(result.insufficient_points);
        let s = format!("{}", result);
        assert!(
            s.contains("INCONCLUSIVE"),
            "Should say INCONCLUSIVE, got: {}",
            s
        );
    }

    // ── Assumption-aware verification ────────────────────────

    #[test]
    fn verify_sqrt_x_squared_fails_without_assumptions() {
        let result = verify("\\sqrt{x^2}", "x", &["x"]);
        assert!(
            !result.passed,
            "√(x²) = x should fail without assumptions (negative x is a counterexample)"
        );
    }

    #[test]
    fn verify_sqrt_x_squared_passes_with_positive() {
        let mut assumptions = Assumptions::new();
        assumptions.assume("x", Assumption::Positive);
        let result = verify_with_assumptions("\\sqrt{x^2}", "x", &["x"], &assumptions);
        assert!(
            result.passed,
            "√(x²) = x should pass with x > 0, got: {}",
            result
        );
    }

    #[test]
    fn verify_sqrt_x_squared_passes_with_nonneg() {
        let mut assumptions = Assumptions::new();
        assumptions.assume("x", Assumption::NonNegative);
        let result = verify_with_assumptions("\\sqrt{x^2}", "x", &["x"], &assumptions);
        assert!(
            result.passed,
            "√(x²) = x should pass with x ≥ 0, got: {}",
            result
        );
    }

    #[test]
    fn verify_enough_points_after_filtering() {
        let mut assumptions = Assumptions::new();
        assumptions.assume("x", Assumption::Positive);
        let result = verify_with_assumptions("x", "x", &["x"], &assumptions);
        assert!(result.passed);
        assert!(
            result.points_tested >= 3,
            "Should have ≥3 positive test points, got: {}",
            result.points_tested
        );
    }
}

#[test]
fn finding4_one_sided_undefinedness_is_a_counterexample() {
    // √x and √|x| differ on the entire negative axis: √x is undefined
    // there, √|x| is not. One-sided undefinedness is a domain violation —
    // a refutation, not a skippable sample point.
    let lhs = arithma::parse_latex_raw("\\sqrt{x}").unwrap();
    let rhs = arithma::parse_latex_raw("\\sqrt{|x|}").unwrap();
    let result = arithma::verify_identity(
        &lhs,
        &rhs,
        &["x".to_string()],
        &arithma::assumptions::Assumptions::default(),
    );
    assert!(!result.passed);
    assert!(result.counterexample.is_some());
}

#[test]
fn summation_bound_variable_sampled_at_integers() {
    // Σ_{k=1}^{n} (1/k − 1/(k+1)) = 1 − 1/(n+1): true at every integer
    // n ≥ 1. The sampler must give a Σ-bound variable integer values in
    // range — not n = 0.5, where the sum has no value and a truncated
    // evaluation invents one.
    let lhs = arithma::parse_latex_raw("\\sum_{k=1}^{n} {\\frac{1}{k} - \\frac{1}{k+1}}").unwrap();
    let rhs = arithma::parse_latex_raw("1 - \\frac{1}{n+1}").unwrap();
    let result = arithma::verify_identity(
        &lhs,
        &rhs,
        &["n".to_string()],
        &arithma::assumptions::Assumptions::new(),
    );
    assert!(
        result.passed,
        "telescoping identity must verify on integer-sampled n; counterexample at {:?}",
        result.counterexample.map(|c| c.point)
    );
}

// ── Both-bounds-symbolic ranges: the sampler owes coverage of the range
// length L = n − m + 1. If every sampled range has the same length, any
// claim that is correct for that one length verifies — Σ_{k=m}^{n} f(k)
// = f(m) holds whenever L ≡ 1, for every f. Refuting these claims
// requires sampling at least two distinct lengths; that coverage is the
// invariant these tests pin.

fn verify_mn(lhs: &str, rhs: &str) -> arithma::verify::VerifyResult {
    let lhs = arithma::parse_latex_raw(lhs).unwrap();
    let rhs = arithma::parse_latex_raw(rhs).unwrap();
    arithma::verify_identity(
        &lhs,
        &rhs,
        &["m".to_string(), "n".to_string()],
        &arithma::assumptions::Assumptions::default(),
    )
}

#[test]
fn symbolic_range_sum_is_not_its_first_term() {
    let result = verify_mn("\\sum_{k=m}^{n} k^2", "m^2");
    assert!(
        !result.passed,
        "Σ_(k=m)^(n) k² = m² is false for any range longer than one term; \
         a sampler that passes it never varied the range length"
    );
}

#[test]
fn symbolic_range_constant_sum_counts_its_terms() {
    // Σ_{k=m}^{n} 1 = L exactly. The claim "= 1" is the purest length
    // probe: it holds iff L = 1 at every sampled point.
    let result = verify_mn("\\sum_{k=m}^{n} 1", "1");
    assert!(
        !result.passed,
        "Σ_(k=m)^(n) 1 = 1 must be refuted — the sum counts its terms"
    );
}

#[test]
fn symbolic_range_product_is_not_its_first_factor() {
    // Π_{k=m}^{n} 2 = 2^L; the claim "= 2" holds only at L = 1.
    let result = verify_mn("\\prod_{k=m}^{n} 2", "2");
    assert!(
        !result.passed,
        "Π_(k=m)^(n) 2 = 2 must be refuted — the product has L factors"
    );
}

// A pattern-matched pair detector names the syntax (plain variables);
// the bug is about a property (range-length coverage). A bound written
// as an expression escapes the matcher and falls back to the collapsing
// stream: Σ_{k=m}^{n+1} samples L ≡ 2 everywhere, so the two-term
// analogue of the first-term claim verifies for any body. The defense
// is asserting realized coverage, not recognizing more shapes: a Σ/Π
// whose range length is not structurally constant must realize ≥3
// distinct lengths, or the evidence is insufficient.

#[test]
fn compound_bound_sum_is_not_its_first_two_terms() {
    // Whether by refutation (a length ≠ 2 was sampled) or by refusal
    // (coverage starved), this must never PASS.
    let result = verify_mn("\\sum_{k=m}^{n+1} k^2", "m^2 + (m+1)^2");
    assert!(
        !result.passed,
        "Σ_(k=m)^(n+1) k² = m² + (m+1)² is false beyond two-term ranges; \
         a sampler that passes it never varied the range length"
    );
}

#[test]
fn starved_coverage_refusal_states_its_reason() {
    // A refusal that misstates its own reason is a check lying about
    // what it checked. The starved-coverage message must name length
    // coverage, not the (satisfied) point count.
    let result = arithma::verify::VerifyResult {
        passed: false,
        points_tested: 7,
        counterexample: None,
        insufficient_points: true,
        domain_mismatches: 0,
        starved_range_lengths: Some(1),
    };
    let rendered = format!("{}", result);
    assert!(
        rendered.contains("length") && !rendered.contains("only 7"),
        "the refusal must state its actual reason (length coverage), got: {}",
        rendered
    );
}

// The coverage assertion certifies a MARGINAL — the set of realized
// lengths. The claim is JOINT — over the bound variables. If the
// sampled (m, n) all lie on a line, a bound like 2n makes L vary along
// that line (marginal green) while the joint never leaves it, and any
// claim true exactly on the line passes. The stream must walk the
// joint: co-bound variables must realize varying differences.

#[test]
fn claim_true_only_on_the_sampled_line_is_refuted() {
    // Σ_{k=m}^{2n} 1 = (2n−m+1) + (n−m)(n−m−1): the extra term vanishes
    // exactly on n − m ∈ {0, 1}. False at n = m + 2, so a sampler that
    // walks the joint refutes it.
    let result = verify_mn("\\sum_{k=m}^{2n} 1", "(2n - m + 1) + (n-m)(n-m-1)");
    assert!(
        !result.passed,
        "a claim true only on the line n − m ∈ {{0,1}} must not verify; \
         the sampler never left the diagonal"
    );
}

#[test]
fn true_bound_expression_identity_verifies() {
    // Σ_{k=m}^{n+1} 1 = n − m + 2 is true wherever the range is defined.
    // Refusing it means the assertion is doing the constructor's job;
    // a joint-walking stream realizes ≥3 lengths and verifies it.
    let result = verify_mn("\\sum_{k=m}^{n+1} 1", "n - m + 2");
    assert!(
        result.passed,
        "true bound-expression identity must verify; got {:?} / insufficient={}",
        result.counterexample.map(|c| c.point),
        result.insufficient_points
    );
}

#[test]
fn compound_bound_product_is_not_its_first_two_factors() {
    let result = verify_mn("\\prod_{k=m}^{n+1} k", "m(m+1)");
    assert!(
        !result.passed,
        "Π_(k=m)^(n+1) k = m(m+1) must not verify on single-length sampling"
    );
}

#[test]
fn structurally_constant_length_owes_no_coverage() {
    // Σ_{k=m}^{m+2} has L ≡ 3 by construction — the author pinned the
    // length, so one length IS the whole domain and the identity is
    // verifiable. Coverage is owed only where L actually varies.
    let result = verify_mn("\\sum_{k=m}^{m+2} k", "3m + 3");
    assert!(
        result.passed,
        "fixed-length range identity must verify; counterexample at {:?}",
        result.counterexample.map(|c| c.point)
    );
}

#[test]
fn true_symbolic_range_identity_still_verifies() {
    // Σ_{k=m}^{n} k = (n(n+1) − (m−1)m)/2 at every integer pair m ≤ n + 1
    // (including the empty range, where both sides are 0). Refusing this
    // would trade the false PASS for a false refusal-of-evidence.
    let result = verify_mn("\\sum_{k=m}^{n} k", "\\frac{n(n+1) - (m-1)m}{2}");
    assert!(
        result.passed,
        "true symbolic-range identity must verify; counterexample at {:?}",
        result.counterexample.map(|c| c.point)
    );
    assert!(
        result.points_tested >= 3,
        "expected real evidence, got {} points",
        result.points_tested
    );
}
