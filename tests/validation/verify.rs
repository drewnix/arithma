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
